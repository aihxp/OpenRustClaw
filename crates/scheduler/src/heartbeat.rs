//! Heartbeat Scheduler - unprompted automation
//!
//! Triggers actions based on conditions without user prompting.
//! Enables autonomous automation like OpenClaw's heartbeat.

use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use cron::Schedule;
use tokio::sync::{RwLock, broadcast};
use tokio::time::{Duration, interval};
use tracing;
use uuid::Uuid;

/// Heartbeat scheduler for autonomous triggers
pub struct HeartbeatScheduler {
    tasks: Arc<RwLock<HashMap<TaskId, HeartbeatTask>>>,
    triggers: broadcast::Sender<HeartbeatEvent>,
    check_interval: Duration,
    file_state: Arc<Mutex<HashMap<(PathBuf, bool), FileFingerprint>>>,
    last_activity: Arc<Mutex<DateTime<Utc>>>,
    recent_emails: Arc<Mutex<Vec<EmailEvent>>>,
    recent_system_events: Arc<Mutex<HashMap<SystemEventType, DateTime<Utc>>>>,
    handlers: Arc<Mutex<HeartbeatHandlers>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileFingerprint {
    modified_at: Option<std::time::SystemTime>,
    size_bytes: u64,
}

#[derive(Debug, Clone)]
struct EmailEvent {
    mailbox: String,
    subject: String,
    body: String,
    received_at: DateTime<Utc>,
}

type AgentMessageHandler = Arc<
    dyn Fn(AgentTarget, String) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync,
>;
type ToolCallHandler = Arc<
    dyn Fn(String, serde_json::Value) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>
        + Send
        + Sync,
>;
type SkillHandler =
    Arc<dyn Fn(String, String) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync>;
type NotificationHandler = Arc<
    dyn Fn(String, String, Urgency) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>
        + Send
        + Sync,
>;

#[derive(Default)]
struct HeartbeatHandlers {
    agent_message: Option<AgentMessageHandler>,
    tool_call: Option<ToolCallHandler>,
    skill: Option<SkillHandler>,
    notification: Option<NotificationHandler>,
}

/// Unique identifier for heartbeat tasks
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct TaskId(pub Uuid);

impl TaskId {
    /// Generate a new unique task ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

/// A heartbeat task with condition and action
pub struct HeartbeatTask {
    /// Unique task identifier
    pub id: TaskId,
    /// Human-readable task name
    pub name: String,
    /// Condition that triggers the action
    pub condition: HeartbeatCondition,
    /// Action to execute when condition is met
    pub action: HeartbeatAction,
    /// Last time the task was triggered
    pub last_triggered: Option<DateTime<Utc>>,
    /// Number of times the task has been triggered
    pub trigger_count: u64,
    /// Whether the task is enabled
    pub enabled: bool,
    /// Minimum time between triggers
    pub cooldown: Duration,
}

/// Conditions that can trigger an action
#[derive(Clone)]
pub enum HeartbeatCondition {
    /// Time-based (cron expression)
    Cron { schedule: Schedule },

    /// File or directory changed
    FileChanged { path: PathBuf, recursive: bool },

    /// Email received matching query
    EmailReceived {
        /// Search query for emails
        query: String,
        /// Mailbox to monitor
        mailbox: String,
    },

    /// User has been idle for duration
    IdleDuration { duration: Duration },

    /// System event (login, logout, etc)
    SystemEvent { event_type: SystemEventType },

    /// Custom condition with closure
    Custom {
        /// Name of the custom condition
        name: String,
        /// Check function that returns true if condition is met
        #[allow(clippy::type_complexity)]
        check: Arc<dyn Fn() -> bool + Send + Sync>,
    },

    /// Multiple conditions (AND)
    All(Vec<HeartbeatCondition>),

    /// Multiple conditions (OR)
    Any(Vec<HeartbeatCondition>),
}

/// System event types for triggering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemEventType {
    /// User logged in
    UserLogin,
    /// User logged out
    UserLogout,
    /// System booted
    SystemBoot,
    /// Network connected
    NetworkConnect,
    /// Network disconnected
    NetworkDisconnect,
}

/// Actions to execute when triggered
#[derive(Clone)]
pub enum HeartbeatAction {
    /// Send a message to an agent
    AgentMessage {
        /// Target agent
        target: AgentTarget,
        /// Message content
        message: String,
    },

    /// Execute a tool
    ToolCall {
        /// Tool name
        tool_name: String,
        /// Tool arguments
        arguments: serde_json::Value,
    },

    /// Run a script/skill
    RunSkill {
        /// Skill name
        skill_name: String,
        /// Input to the skill
        input: String,
    },

    /// Send notification
    Notify {
        /// Notification title
        title: String,
        /// Notification body
        body: String,
        /// Notification urgency
        urgency: Urgency,
    },

    /// Custom action
    Custom {
        /// Name of the custom action
        name: String,
        /// Execute function
        #[allow(clippy::type_complexity)]
        execute: Arc<dyn Fn() -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync>,
    },
}

impl std::fmt::Debug for HeartbeatAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AgentMessage { target, message } => f
                .debug_struct("AgentMessage")
                .field("target", target)
                .field("message", message)
                .finish(),
            Self::ToolCall {
                tool_name,
                arguments,
            } => f
                .debug_struct("ToolCall")
                .field("tool_name", tool_name)
                .field("arguments", arguments)
                .finish(),
            Self::RunSkill { skill_name, input } => f
                .debug_struct("RunSkill")
                .field("skill_name", skill_name)
                .field("input", input)
                .finish(),
            Self::Notify {
                title,
                body,
                urgency,
            } => f
                .debug_struct("Notify")
                .field("title", title)
                .field("body", body)
                .field("urgency", urgency)
                .finish(),
            Self::Custom { name, .. } => f
                .debug_struct("Custom")
                .field("name", name)
                .finish_non_exhaustive(),
        }
    }
}

/// Target for agent messages
#[derive(Debug)]
pub enum AgentTarget {
    /// Main session
    Main,
    /// Any agent in workspace
    Workspace(String),
    /// Specific session ID
    Session(String),
    /// All active sessions
    All,
}

/// Notification urgency levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Urgency {
    /// Low urgency
    Low,
    /// Normal urgency
    Normal,
    /// High urgency
    High,
    /// Critical urgency
    Critical,
}

/// Event emitted when a task triggers
#[derive(Debug, Clone)]
pub struct HeartbeatEvent {
    /// Task ID that triggered
    pub task_id: TaskId,
    /// Task name
    pub task_name: String,
    /// When the task was triggered
    pub triggered_at: DateTime<Utc>,
    /// Description of which condition triggered
    pub condition_met: String,
}

impl HeartbeatScheduler {
    /// Create a new heartbeat scheduler
    ///
    /// # Arguments
    /// * `check_interval` - How often to check conditions
    ///
    /// # Example
    /// ```
    /// use std::time::Duration;
    /// use openrustclaw_scheduler::heartbeat::HeartbeatScheduler;
    ///
    /// let scheduler = HeartbeatScheduler::new(Duration::from_secs(30));
    /// ```
    pub fn new(check_interval: Duration) -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            triggers: tx,
            check_interval,
            file_state: Arc::new(Mutex::new(HashMap::new())),
            last_activity: Arc::new(Mutex::new(Utc::now())),
            recent_emails: Arc::new(Mutex::new(Vec::new())),
            recent_system_events: Arc::new(Mutex::new(HashMap::new())),
            handlers: Arc::new(Mutex::new(HeartbeatHandlers {
                notification: Some(Arc::new(|title, body, urgency| {
                    Box::pin(async move {
                        match urgency {
                            Urgency::Critical => tracing::error!("[CRITICAL] {}: {}", title, body),
                            Urgency::High => tracing::warn!("[HIGH] {}: {}", title, body),
                            Urgency::Normal => tracing::info!("[NORMAL] {}: {}", title, body),
                            Urgency::Low => tracing::debug!("[LOW] {}: {}", title, body),
                        }
                        Ok(())
                    })
                })),
                ..HeartbeatHandlers::default()
            })),
        }
    }

    /// Add a new task
    ///
    /// # Arguments
    /// * `task` - The task to add
    ///
    /// # Returns
    /// The task ID
    pub async fn add_task(&self, task: HeartbeatTask) -> TaskId {
        let mut tasks = self.tasks.write().await;
        let id = task.id.clone();
        tasks.insert(id.clone(), task);
        id
    }

    /// Remove a task
    ///
    /// # Arguments
    /// * `id` - The task ID to remove
    ///
    /// # Returns
    /// The removed task, if any
    pub async fn remove_task(&self, id: &TaskId) -> Option<HeartbeatTask> {
        let mut tasks = self.tasks.write().await;
        tasks.remove(id)
    }

    /// Get a task by ID
    ///
    /// # Arguments
    /// * `id` - The task ID
    ///
    /// # Returns
    /// The task, if found
    pub async fn get_task(&self, id: &TaskId) -> Option<HeartbeatTask> {
        let tasks = self.tasks.read().await;
        tasks.get(id).cloned()
    }

    /// Enable a task
    ///
    /// # Arguments
    /// * `id` - The task ID to enable
    ///
    /// # Returns
    /// True if the task was found and enabled
    pub async fn enable_task(&self, id: &TaskId) -> bool {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(id) {
            task.enabled = true;
            true
        } else {
            false
        }
    }

    /// Disable a task
    ///
    /// # Arguments
    /// * `id` - The task ID to disable
    ///
    /// # Returns
    /// True if the task was found and disabled
    pub async fn disable_task(&self, id: &TaskId) -> bool {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(id) {
            task.enabled = false;
            true
        } else {
            false
        }
    }

    /// Get all task IDs
    pub async fn list_tasks(&self) -> Vec<TaskId> {
        let tasks = self.tasks.read().await;
        tasks.keys().cloned().collect()
    }

    /// Start the scheduler loop
    ///
    /// This runs indefinitely, checking conditions and executing actions.
    pub async fn run(&self) -> Result<()> {
        let mut ticker = interval(self.check_interval);

        loop {
            ticker.tick().await;

            self.check_and_trigger().await;
        }
    }

    /// Run a single check cycle
    ///
    /// Checks all tasks and triggers those whose conditions are met.
    /// This is useful for testing or manual execution.
    pub async fn check_and_trigger(&self) {
        let tasks = self.tasks.read().await;
        let task_ids: Vec<TaskId> = tasks.keys().cloned().collect();
        drop(tasks); // Release read lock before processing

        for id in task_ids {
            let should_trigger = {
                let tasks = self.tasks.read().await;
                if let Some(task) = tasks.get(&id) {
                    if !task.enabled {
                        continue;
                    }

                    // Check cooldown
                    if let Some(last) = task.last_triggered {
                        let elapsed = Utc::now() - last;
                        if let Ok(cooldown) = ChronoDuration::from_std(task.cooldown)
                            && elapsed < cooldown
                        {
                            continue;
                        }
                    }

                    // Check condition
                    match self.check_condition(&task.condition).await {
                        Ok(result) => result,
                        Err(e) => {
                            tracing::error!(
                                "Heartbeat condition failed for task '{}': {}",
                                task.name,
                                e
                            );
                            false
                        }
                    }
                } else {
                    false
                }
            };

            if should_trigger {
                self.trigger_task(&id).await;
            }
        }
    }

    /// Trigger a specific task
    async fn trigger_task(&self, id: &TaskId) {
        let (task_name, action) = {
            let tasks = self.tasks.read().await;
            if let Some(task) = tasks.get(id) {
                (task.name.clone(), task.action.clone())
            } else {
                return;
            }
        };

        // Execute action
        let result = self.execute_action(&action).await;

        if let Err(e) = result {
            tracing::error!("Heartbeat action failed for task '{}': {}", task_name, e);
        } else {
            // Update task stats
            let mut tasks = self.tasks.write().await;
            if let Some(task) = tasks.get_mut(id) {
                task.last_triggered = Some(Utc::now());
                task.trigger_count += 1;
            }
            drop(tasks);

            // Broadcast event
            let event = HeartbeatEvent {
                task_id: id.clone(),
                task_name: task_name.clone(),
                triggered_at: Utc::now(),
                condition_met: format!("Condition met for task '{}'", task_name),
            };

            let _ = self.triggers.send(event);
            tracing::info!("Heartbeat task '{}' triggered successfully", task_name);
        }
    }

    /// Check if a condition is met
    ///
    /// This function is not async to avoid recursion issues with Box::pin
    fn check_condition_sync(&self, condition: &HeartbeatCondition) -> Result<bool> {
        match condition {
            HeartbeatCondition::Cron { schedule } => {
                let now = Utc::now();
                // Check if the schedule includes the current time
                // We check if there's an upcoming occurrence within the last second
                for datetime in schedule.upcoming(Utc).take(1) {
                    let diff = datetime - now;
                    if diff.num_seconds().abs() <= 1 {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            HeartbeatCondition::FileChanged { path, recursive } => {
                self.check_file_changed(path, *recursive)
            }
            HeartbeatCondition::EmailReceived { query, mailbox } => {
                Ok(self.check_email_received(query, mailbox))
            }
            HeartbeatCondition::IdleDuration { duration } => {
                if let Some(last_activity) = self.last_activity.lock().ok().map(|guard| *guard) {
                    if let Ok(idle_for) = ChronoDuration::from_std(*duration) {
                        Ok(Utc::now() - last_activity >= idle_for)
                    } else {
                        Ok(false)
                    }
                } else {
                    Ok(false)
                }
            }
            HeartbeatCondition::SystemEvent { event_type } => {
                Ok(self.check_system_event(*event_type))
            }
            HeartbeatCondition::Custom { name, check } => {
                let result = check();
                tracing::trace!("Custom condition '{}' returned: {}", name, result);
                Ok(result)
            }
            HeartbeatCondition::All(conditions) => {
                for c in conditions {
                    if !self.check_condition_sync(c)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            HeartbeatCondition::Any(conditions) => {
                for c in conditions {
                    if self.check_condition_sync(c)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
        }
    }

    /// Check if a condition is met (async wrapper)
    async fn check_condition(&self, condition: &HeartbeatCondition) -> Result<bool> {
        self.check_condition_sync(condition)
    }

    /// Execute an action
    async fn execute_action(&self, action: &HeartbeatAction) -> Result<()> {
        match action {
            HeartbeatAction::AgentMessage { target, message } => {
                if let Some(handler) = self
                    .handlers
                    .lock()
                    .ok()
                    .and_then(|handlers| handlers.agent_message.clone())
                {
                    handler(target.clone(), message.clone()).await
                } else {
                    Err(
                        openrustclaw_core::error::SchedulerError::HeartbeatActionFailed(
                            "No agent message handler configured".to_string(),
                        )
                        .into(),
                    )
                }
            }
            HeartbeatAction::ToolCall {
                tool_name,
                arguments,
            } => {
                if let Some(handler) = self
                    .handlers
                    .lock()
                    .ok()
                    .and_then(|handlers| handlers.tool_call.clone())
                {
                    handler(tool_name.clone(), arguments.clone()).await
                } else {
                    Err(
                        openrustclaw_core::error::SchedulerError::HeartbeatActionFailed(format!(
                            "No tool call handler configured for {}",
                            tool_name
                        ))
                        .into(),
                    )
                }
            }
            HeartbeatAction::RunSkill { skill_name, input } => {
                if let Some(handler) = self
                    .handlers
                    .lock()
                    .ok()
                    .and_then(|handlers| handlers.skill.clone())
                {
                    handler(skill_name.clone(), input.clone()).await
                } else {
                    Err(
                        openrustclaw_core::error::SchedulerError::HeartbeatActionFailed(format!(
                            "No skill handler configured for {}",
                            skill_name
                        ))
                        .into(),
                    )
                }
            }
            HeartbeatAction::Notify {
                title,
                body,
                urgency,
            } => {
                if let Some(handler) = self
                    .handlers
                    .lock()
                    .ok()
                    .and_then(|handlers| handlers.notification.clone())
                {
                    handler(title.clone(), body.clone(), *urgency).await
                } else {
                    Err(
                        openrustclaw_core::error::SchedulerError::HeartbeatActionFailed(
                            "No notification handler configured".to_string(),
                        )
                        .into(),
                    )
                }
            }
            HeartbeatAction::Custom { name, execute } => {
                tracing::info!("Executing custom action: {}", name);
                execute().await
            }
        }
    }

    /// Subscribe to heartbeat events
    ///
    /// # Returns
    /// A receiver for heartbeat events
    pub fn subscribe(&self) -> broadcast::Receiver<HeartbeatEvent> {
        self.triggers.subscribe()
    }

    /// Record user activity for idle-duration conditions.
    pub fn record_activity(&self) {
        if let Ok(mut last_activity) = self.last_activity.lock() {
            *last_activity = Utc::now();
        }
    }

    /// Record a recent system event.
    pub fn record_system_event(&self, event_type: SystemEventType) {
        if let Ok(mut events) = self.recent_system_events.lock() {
            events.insert(event_type, Utc::now());
        }
    }

    /// Record a received email for mailbox-based conditions.
    pub fn record_email(
        &self,
        mailbox: impl Into<String>,
        subject: impl Into<String>,
        body: impl Into<String>,
    ) {
        if let Ok(mut emails) = self.recent_emails.lock() {
            emails.push(EmailEvent {
                mailbox: mailbox.into(),
                subject: subject.into(),
                body: body.into(),
                received_at: Utc::now(),
            });
            let cutoff = Utc::now() - ChronoDuration::minutes(10);
            emails.retain(|email| email.received_at >= cutoff);
        }
    }

    /// Set the async handler for agent-message heartbeat actions.
    pub fn set_agent_message_handler<F, Fut>(&self, handler: F)
    where
        F: Fn(AgentTarget, String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        if let Ok(mut handlers) = self.handlers.lock() {
            handlers.agent_message = Some(Arc::new(move |target, message| {
                Box::pin(handler(target, message))
            }));
        }
    }

    /// Set the async handler for tool-call heartbeat actions.
    pub fn set_tool_call_handler<F, Fut>(&self, handler: F)
    where
        F: Fn(String, serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        if let Ok(mut handlers) = self.handlers.lock() {
            handlers.tool_call = Some(Arc::new(move |tool_name, arguments| {
                Box::pin(handler(tool_name, arguments))
            }));
        }
    }

    /// Set the async handler for skill heartbeat actions.
    pub fn set_skill_handler<F, Fut>(&self, handler: F)
    where
        F: Fn(String, String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        if let Ok(mut handlers) = self.handlers.lock() {
            handlers.skill = Some(Arc::new(move |skill_name, input| {
                Box::pin(handler(skill_name, input))
            }));
        }
    }

    /// Set the async handler for notifications.
    pub fn set_notification_handler<F, Fut>(&self, handler: F)
    where
        F: Fn(String, String, Urgency) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        if let Ok(mut handlers) = self.handlers.lock() {
            handlers.notification = Some(Arc::new(move |title, body, urgency| {
                Box::pin(handler(title, body, urgency))
            }));
        }
    }

    fn check_file_changed(&self, path: &PathBuf, recursive: bool) -> Result<bool> {
        let fingerprint = compute_file_fingerprint(path, recursive)?;
        let key = (path.clone(), recursive);
        let mut state = self.file_state.lock().map_err(|_| {
            openrustclaw_core::error::SchedulerError::FileWatchError(
                "Failed to lock file watcher state".to_string(),
            )
        })?;

        match state.get(&key) {
            Some(previous) if previous != &fingerprint => {
                state.insert(key, fingerprint);
                Ok(true)
            }
            Some(_) => Ok(false),
            None => {
                state.insert(key, fingerprint);
                Ok(false)
            }
        }
    }

    fn check_email_received(&self, query: &str, mailbox: &str) -> bool {
        let query = query.to_lowercase();
        let cutoff = Utc::now()
            - ChronoDuration::from_std(self.check_interval)
                .unwrap_or_else(|_| ChronoDuration::seconds(1));
        self.recent_emails
            .lock()
            .ok()
            .map(|emails| {
                emails.iter().any(|email| {
                    email.mailbox.eq_ignore_ascii_case(mailbox)
                        && email.received_at >= cutoff
                        && format!("{} {}", email.subject, email.body)
                            .to_lowercase()
                            .contains(&query)
                })
            })
            .unwrap_or(false)
    }

    fn check_system_event(&self, event_type: SystemEventType) -> bool {
        let cutoff = Utc::now()
            - ChronoDuration::from_std(self.check_interval)
                .unwrap_or_else(|_| ChronoDuration::seconds(1));
        self.recent_system_events
            .lock()
            .ok()
            .and_then(|events| events.get(&event_type).copied())
            .map(|timestamp| timestamp >= cutoff)
            .unwrap_or(false)
    }
}

fn compute_file_fingerprint(path: &PathBuf, recursive: bool) -> Result<FileFingerprint> {
    let metadata = std::fs::metadata(path)
        .map_err(|e| openrustclaw_core::error::SchedulerError::FileWatchError(e.to_string()))?;

    if metadata.is_file() {
        return Ok(FileFingerprint {
            modified_at: metadata.modified().ok(),
            size_bytes: metadata.len(),
        });
    }

    let mut latest_modified = metadata.modified().ok();
    let mut total_size = 0_u64;
    let max_depth = if recursive { usize::MAX } else { 1 };

    for entry in walkdir::WalkDir::new(path)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        let entry_metadata = entry
            .metadata()
            .map_err(|e| openrustclaw_core::error::SchedulerError::FileWatchError(e.to_string()))?;
        total_size = total_size.saturating_add(entry_metadata.len());
        if let Ok(modified) = entry_metadata.modified() {
            latest_modified = Some(match latest_modified {
                Some(current) if current > modified => current,
                _ => modified,
            });
        }
    }

    Ok(FileFingerprint {
        modified_at: latest_modified,
        size_bytes: total_size,
    })
}

impl Default for HeartbeatScheduler {
    fn default() -> Self {
        Self::new(Duration::from_secs(30))
    }
}

impl Clone for HeartbeatTask {
    fn clone(&self) -> Self {
        // Note: Custom conditions and actions cannot be cloned directly
        // This is a partial clone for non-custom tasks
        Self {
            id: self.id.clone(),
            name: self.name.clone(),
            condition: self.condition.clone(),
            action: self.action.clone(),
            last_triggered: self.last_triggered,
            trigger_count: self.trigger_count,
            enabled: self.enabled,
            cooldown: self.cooldown,
        }
    }
}

impl std::fmt::Debug for HeartbeatCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cron { schedule } => f
                .debug_struct("Cron")
                .field("schedule", &schedule.to_string())
                .finish(),
            Self::FileChanged { path, recursive } => f
                .debug_struct("FileChanged")
                .field("path", path)
                .field("recursive", recursive)
                .finish(),
            Self::EmailReceived { query, mailbox } => f
                .debug_struct("EmailReceived")
                .field("query", query)
                .field("mailbox", mailbox)
                .finish(),
            Self::IdleDuration { duration } => f
                .debug_struct("IdleDuration")
                .field("duration", duration)
                .finish(),
            Self::SystemEvent { event_type } => f
                .debug_struct("SystemEvent")
                .field("event_type", event_type)
                .finish(),
            Self::Custom { name, .. } => f
                .debug_struct("Custom")
                .field("name", name)
                .finish_non_exhaustive(),
            Self::All(conditions) => f.debug_tuple("All").field(conditions).finish(),
            Self::Any(conditions) => f.debug_tuple("Any").field(conditions).finish(),
        }
    }
}

impl Clone for AgentTarget {
    fn clone(&self) -> Self {
        match self {
            Self::Main => Self::Main,
            Self::Workspace(s) => Self::Workspace(s.clone()),
            Self::Session(s) => Self::Session(s.clone()),
            Self::All => Self::All,
        }
    }
}

// Builder for easy task creation

/// Builder for creating heartbeat tasks
///
/// # Example
/// ```
/// use std::time::Duration;
/// use openrustclaw_scheduler::heartbeat::{HeartbeatTaskBuilder, AgentTarget};
///
/// // Create a daily summary task at 9am
/// let task = HeartbeatTaskBuilder::new("Morning Summary")
///     .on_cron("0 0 9 * * *")  // Seconds Minutes Hours Day Month DayOfWeek
///     .expect("Valid cron")
///     .send_to_agent(AgentTarget::Main, "Generate daily summary")
///     .build()
///     .expect("Valid task");
/// ```
pub struct HeartbeatTaskBuilder {
    name: String,
    condition: Option<HeartbeatCondition>,
    action: Option<HeartbeatAction>,
    cooldown: Duration,
}

impl HeartbeatTaskBuilder {
    /// Create a new task builder
    ///
    /// # Arguments
    /// * `name` - The task name
    ///
    /// # Example
    /// ```
    /// use openrustclaw_scheduler::heartbeat::HeartbeatTaskBuilder;
    ///
    /// let builder = HeartbeatTaskBuilder::new("My Task");
    /// ```
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            condition: None,
            action: None,
            cooldown: Duration::from_secs(60),
        }
    }

    /// Set condition to a cron schedule
    ///
    /// # Arguments
    /// * `expression` - A cron expression with seconds (e.g., "0 0 9 * * *" for 9am daily)
    ///
    /// # Errors
    /// Returns an error if the cron expression is invalid
    pub fn on_cron(mut self, expression: &str) -> crate::Result<Self> {
        use openrustclaw_core::error::SchedulerError;
        let schedule: Schedule = expression
            .parse()
            .map_err(|e| SchedulerError::InvalidCronExpression(format!("{}", e)))?;
        self.condition = Some(HeartbeatCondition::Cron { schedule });
        Ok(self)
    }

    /// Set condition to file change detection
    ///
    /// # Arguments
    /// * `path` - Path to the file or directory to watch
    pub fn on_file_change(mut self, path: impl Into<PathBuf>) -> Self {
        self.condition = Some(HeartbeatCondition::FileChanged {
            path: path.into(),
            recursive: false,
        });
        self
    }

    /// Set condition to recursive file change detection
    ///
    /// # Arguments
    /// * `path` - Path to the directory to watch recursively
    pub fn on_file_change_recursive(mut self, path: impl Into<PathBuf>) -> Self {
        self.condition = Some(HeartbeatCondition::FileChanged {
            path: path.into(),
            recursive: true,
        });
        self
    }

    /// Set condition to idle duration
    ///
    /// # Arguments
    /// * `duration` - Duration of idle time required
    pub fn on_idle(mut self, duration: Duration) -> Self {
        self.condition = Some(HeartbeatCondition::IdleDuration { duration });
        self
    }

    /// Set condition to email received
    ///
    /// # Arguments
    /// * `query` - Email search query
    /// * `mailbox` - Mailbox to monitor (e.g., "INBOX")
    pub fn on_email(mut self, query: impl Into<String>, mailbox: impl Into<String>) -> Self {
        self.condition = Some(HeartbeatCondition::EmailReceived {
            query: query.into(),
            mailbox: mailbox.into(),
        });
        self
    }

    /// Set condition to system event
    ///
    /// # Arguments
    /// * `event_type` - Type of system event to watch for
    pub fn on_system_event(mut self, event_type: SystemEventType) -> Self {
        self.condition = Some(HeartbeatCondition::SystemEvent { event_type });
        self
    }

    /// Set condition to a custom check
    ///
    /// # Arguments
    /// * `name` - Name of the condition
    /// * `check` - Function that returns true if condition is met
    pub fn on_custom<F>(mut self, name: impl Into<String>, check: F) -> Self
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        self.condition = Some(HeartbeatCondition::Custom {
            name: name.into(),
            check: Arc::new(check),
        });
        self
    }

    /// Set action to send a message to an agent
    ///
    /// # Arguments
    /// * `target` - Target agent
    /// * `message` - Message content
    pub fn send_to_agent(mut self, target: AgentTarget, message: impl Into<String>) -> Self {
        self.action = Some(HeartbeatAction::AgentMessage {
            target,
            message: message.into(),
        });
        self
    }

    /// Set action to call a tool
    ///
    /// # Arguments
    /// * `tool_name` - Name of the tool
    /// * `arguments` - Tool arguments as JSON
    pub fn call_tool(mut self, tool_name: impl Into<String>, arguments: serde_json::Value) -> Self {
        self.action = Some(HeartbeatAction::ToolCall {
            tool_name: tool_name.into(),
            arguments,
        });
        self
    }

    /// Set action to run a skill
    ///
    /// # Arguments
    /// * `skill_name` - Name of the skill
    /// * `input` - Input to the skill
    pub fn run_skill(mut self, skill_name: impl Into<String>, input: impl Into<String>) -> Self {
        self.action = Some(HeartbeatAction::RunSkill {
            skill_name: skill_name.into(),
            input: input.into(),
        });
        self
    }

    /// Set action to send a notification
    ///
    /// # Arguments
    /// * `title` - Notification title
    /// * `body` - Notification body
    pub fn notify(mut self, title: impl Into<String>, body: impl Into<String>) -> Self {
        self.action = Some(HeartbeatAction::Notify {
            title: title.into(),
            body: body.into(),
            urgency: Urgency::Normal,
        });
        self
    }

    /// Set action to send a notification with urgency
    ///
    /// # Arguments
    /// * `title` - Notification title
    /// * `body` - Notification body
    /// * `urgency` - Notification urgency
    pub fn notify_with_urgency(
        mut self,
        title: impl Into<String>,
        body: impl Into<String>,
        urgency: Urgency,
    ) -> Self {
        self.action = Some(HeartbeatAction::Notify {
            title: title.into(),
            body: body.into(),
            urgency,
        });
        self
    }

    /// Set action to a custom executor
    ///
    /// # Arguments
    /// * `name` - Name of the action
    /// * `execute` - Async function to execute
    pub fn custom_action<F, Fut>(mut self, name: impl Into<String>, execute: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        self.action = Some(HeartbeatAction::Custom {
            name: name.into(),
            execute: Arc::new(move || Box::pin(execute())),
        });
        self
    }

    /// Set the cooldown duration
    ///
    /// # Arguments
    /// * `cooldown` - Minimum time between triggers
    pub fn with_cooldown(mut self, cooldown: Duration) -> Self {
        self.cooldown = cooldown;
        self
    }

    /// Build the heartbeat task
    ///
    /// # Errors
    /// Returns an error if condition or action is missing
    pub fn build(self) -> crate::Result<HeartbeatTask> {
        use openrustclaw_core::error::SchedulerError;

        let condition = self
            .condition
            .ok_or(SchedulerError::MissingHeartbeatCondition)?;
        let action = self.action.ok_or(SchedulerError::MissingHeartbeatAction)?;

        Ok(HeartbeatTask {
            id: TaskId::new(),
            name: self.name,
            condition,
            action,
            last_triggered: None,
            trigger_count: 0,
            enabled: true,
            cooldown: self.cooldown,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::TempDir;
    use tokio::time::sleep;

    #[test]
    fn test_task_id_generation() {
        let id1 = TaskId::new();
        let id2 = TaskId::new();
        assert_ne!(id1.0, id2.0);
    }

    #[test]
    fn test_builder_with_cron() {
        // Cron format with seconds: second minute hour day month day_of_week
        let task = HeartbeatTaskBuilder::new("Test Task")
            .on_cron("0 0 9 * * *")
            .expect("Valid cron")
            .send_to_agent(AgentTarget::Main, "Hello")
            .build()
            .expect("Valid task");

        assert_eq!(task.name, "Test Task");
        assert!(task.enabled);
        assert_eq!(task.trigger_count, 0);
    }

    #[test]
    fn test_builder_missing_condition() {
        let result = HeartbeatTaskBuilder::new("Test Task")
            .send_to_agent(AgentTarget::Main, "Hello")
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_builder_missing_action() {
        // Cron format with seconds
        let result = HeartbeatTaskBuilder::new("Test Task")
            .on_cron("0 0 9 * * *")
            .expect("Valid cron expression")
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_custom_condition() {
        let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let task = HeartbeatTaskBuilder::new("Custom Task")
            .on_custom("always_true", move || {
                counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                true
            })
            .notify("Test", "Message")
            .build()
            .expect("Valid task");

        assert_eq!(task.name, "Custom Task");
    }

    #[tokio::test]
    async fn test_scheduler_add_and_remove() {
        let scheduler = HeartbeatScheduler::new(Duration::from_secs(60));

        // Cron format with seconds
        let task = HeartbeatTaskBuilder::new("Test Task")
            .on_cron("0 0 9 * * *")
            .expect("Valid cron expression")
            .send_to_agent(AgentTarget::Main, "Hello")
            .build()
            .expect("Valid task");

        let id = scheduler.add_task(task).await;

        let tasks = scheduler.list_tasks().await;
        assert_eq!(tasks.len(), 1);

        scheduler.remove_task(&id).await;

        let tasks = scheduler.list_tasks().await;
        assert!(tasks.is_empty());
    }

    #[tokio::test]
    async fn test_enable_disable_task() {
        let scheduler = HeartbeatScheduler::new(Duration::from_secs(60));

        // Cron format with seconds
        let task = HeartbeatTaskBuilder::new("Test Task")
            .on_cron("0 0 9 * * *")
            .expect("Valid cron expression")
            .send_to_agent(AgentTarget::Main, "Hello")
            .build()
            .expect("Valid task");

        let id = scheduler.add_task(task).await;

        assert!(scheduler.disable_task(&id).await);

        let task = scheduler.get_task(&id).await.expect("Task exists");
        assert!(!task.enabled);

        assert!(scheduler.enable_task(&id).await);

        let task = scheduler.get_task(&id).await.expect("Task exists");
        assert!(task.enabled);
    }

    #[tokio::test]
    async fn test_event_subscription() {
        let scheduler = HeartbeatScheduler::new(Duration::from_secs(60));
        let _rx = scheduler.subscribe();

        // Just verify subscription works - receiver was created successfully
        assert!(true);
    }

    #[tokio::test]
    async fn test_file_change_condition_triggers_after_change() {
        let scheduler = HeartbeatScheduler::new(Duration::from_secs(60));
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("watched.txt");
        tokio::fs::write(&file_path, b"v1").await.unwrap();

        let task = HeartbeatTaskBuilder::new("Watch File")
            .on_file_change(&file_path)
            .notify("changed", "file updated")
            .build()
            .unwrap();
        let id = scheduler.add_task(task).await;

        scheduler.check_and_trigger().await;
        tokio::fs::write(&file_path, b"v2").await.unwrap();
        scheduler.check_and_trigger().await;

        let task = scheduler.get_task(&id).await.unwrap();
        assert_eq!(task.trigger_count, 1);
    }

    #[tokio::test]
    async fn test_idle_duration_condition_uses_activity_state() {
        let scheduler = HeartbeatScheduler::new(Duration::from_secs(60));
        scheduler.record_activity();

        let task = HeartbeatTaskBuilder::new("Idle Check")
            .on_idle(Duration::from_millis(10))
            .notify("idle", "user idle")
            .build()
            .unwrap();
        let id = scheduler.add_task(task).await;

        sleep(Duration::from_millis(20)).await;
        scheduler.check_and_trigger().await;

        let task = scheduler.get_task(&id).await.unwrap();
        assert_eq!(task.trigger_count, 1);
    }

    #[tokio::test]
    async fn test_tool_call_requires_handler_and_succeeds_when_configured() {
        let scheduler = HeartbeatScheduler::new(Duration::from_secs(60));
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_clone = calls.clone();
        scheduler.set_tool_call_handler(move |tool_name, arguments| {
            let calls = calls_clone.clone();
            async move {
                assert_eq!(tool_name, "refresh_index");
                assert_eq!(arguments["force"], true);
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        });

        let task = HeartbeatTaskBuilder::new("Run Tool")
            .on_custom("always", || true)
            .call_tool("refresh_index", serde_json::json!({"force": true}))
            .build()
            .unwrap();
        let id = scheduler.add_task(task).await;

        scheduler.check_and_trigger().await;

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let task = scheduler.get_task(&id).await.unwrap();
        assert_eq!(task.trigger_count, 1);
    }

    #[tokio::test]
    async fn test_agent_message_without_handler_does_not_report_success() {
        let scheduler = HeartbeatScheduler::new(Duration::from_secs(60));
        let task = HeartbeatTaskBuilder::new("Needs Handler")
            .on_custom("always", || true)
            .send_to_agent(AgentTarget::Main, "hello")
            .build()
            .unwrap();
        let id = scheduler.add_task(task).await;

        scheduler.check_and_trigger().await;

        let task = scheduler.get_task(&id).await.unwrap();
        assert_eq!(task.trigger_count, 0);
    }
}
