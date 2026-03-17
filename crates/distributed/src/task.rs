//! Distributed task management and execution.

use crate::error::{DistributedError, Result};
use crate::memory::DistributedMemory;
use crate::node::NodeId;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{Mutex, RwLock};
use tokio::time::{Duration, interval};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// A distributed task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier.
    pub id: String,
    /// Task type.
    pub task_type: String,
    /// Task payload.
    pub payload: Vec<u8>,
    /// Priority level.
    pub priority: TaskPriority,
    /// Labels for routing.
    pub labels: HashMap<String, String>,
    /// Session ID (for sticky routing).
    pub session_id: String,
    /// Assigned worker node.
    pub assigned_node: Option<NodeId>,
    /// Task state.
    pub state: TaskState,
    /// Created timestamp.
    pub created_at: DateTime<Utc>,
    /// Started timestamp.
    pub started_at: Option<DateTime<Utc>>,
    /// Completed timestamp.
    pub completed_at: Option<DateTime<Utc>>,
    /// Timeout in seconds.
    pub timeout_secs: u64,
    /// Retry count.
    pub retry_count: u32,
    /// Maximum retries.
    pub max_retries: u32,
}

/// Task priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl TaskPriority {
    /// Get numeric priority value (higher = more important).
    pub fn value(&self) -> u8 {
        *self as u8
    }
}

/// Task state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    /// Task is pending assignment.
    Pending,
    /// Task is scheduled on a worker.
    Scheduled,
    /// Task is running.
    Running,
    /// Task completed successfully.
    Completed,
    /// Task failed.
    Failed,
    /// Task was cancelled.
    Cancelled,
    /// Task timed out.
    TimedOut,
}

impl std::fmt::Display for TaskState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskState::Pending => write!(f, "pending"),
            TaskState::Scheduled => write!(f, "scheduled"),
            TaskState::Running => write!(f, "running"),
            TaskState::Completed => write!(f, "completed"),
            TaskState::Failed => write!(f, "failed"),
            TaskState::Cancelled => write!(f, "cancelled"),
            TaskState::TimedOut => write!(f, "timed_out"),
        }
    }
}

/// Task result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Whether the task succeeded.
    pub success: bool,
    /// Result data.
    pub data: Vec<u8>,
    /// Error message (if failed).
    pub error: Option<String>,
    /// Execution duration in milliseconds.
    pub duration_ms: u64,
    /// Worker that executed the task.
    pub worker_id: String,
}

impl TaskResult {
    /// Create a successful result.
    pub fn success(data: Vec<u8>, duration_ms: u64, worker_id: String) -> Self {
        Self {
            success: true,
            data,
            error: None,
            duration_ms,
            worker_id,
        }
    }

    /// Create a failed result.
    pub fn failure(error: impl Into<String>, duration_ms: u64, worker_id: String) -> Self {
        Self {
            success: false,
            data: vec![],
            error: Some(error.into()),
            duration_ms,
            worker_id,
        }
    }
}

/// Task executor trait.
#[async_trait]
pub trait TaskExecutor: Send + Sync {
    /// Execute a task.
    async fn execute(&self, task: Task) -> Result<TaskResult>;

    /// Check if this executor can handle a task type.
    fn can_handle(&self, task_type: &str) -> bool;

    /// Get supported task types.
    fn supported_types(&self) -> Vec<String>;
}

/// Task manager for a worker node.
pub struct TaskManager {
    /// Local node ID.
    node_id: NodeId,
    /// Distributed memory for task storage.
    memory: Arc<dyn DistributedMemory>,
    /// Task executor (reserved for future use).
    #[allow(dead_code)]
    executor: Arc<dyn TaskExecutor>,
    /// Local task queue.
    queue: Mutex<Vec<Task>>,
    /// Running tasks.
    running_tasks: RwLock<HashMap<String, Task>>,
    /// Completed tasks (kept for a while for result retrieval).
    completed_tasks: RwLock<HashMap<String, (Task, TaskResult)>>,
    /// Running flag.
    running: AtomicU64,
    /// Task counter for metrics.
    tasks_completed: AtomicU64,
    /// Task counter for metrics.
    tasks_failed: AtomicU64,
}

impl TaskManager {
    /// Create a new task manager.
    pub fn new(
        node_id: NodeId,
        memory: Arc<dyn DistributedMemory>,
        executor: Arc<dyn TaskExecutor>,
    ) -> Arc<Self> {
        let manager = Arc::new(Self {
            node_id,
            memory,
            executor,
            queue: Mutex::new(Vec::new()),
            running_tasks: RwLock::new(HashMap::new()),
            completed_tasks: RwLock::new(HashMap::new()),
            running: AtomicU64::new(1),
            tasks_completed: AtomicU64::new(0),
            tasks_failed: AtomicU64::new(0),
        });

        // Start cleanup task
        manager.clone().start_cleanup_task();

        manager
    }

    /// Stop the task manager.
    pub async fn stop(&self) -> Result<()> {
        self.running.store(0, Ordering::SeqCst);
        Ok(())
    }

    /// Submit a task to the queue.
    pub async fn submit_task(&self, task: Task) -> Result<()> {
        let task_id = task.id.clone();

        if task.assigned_node.as_ref() == Some(&self.node_id) {
            // Task is for us, add to local queue
            let mut queue = self.queue.lock().await;
            queue.push(task);
            // Sort by priority (higher first)
            queue.sort_by(|a, b| b.priority.value().cmp(&a.priority.value()));
            debug!("Added task {} to local queue", task_id);
        } else {
            // Store in distributed memory for the assigned node
            let key = format!("task:{}", task_id);
            let value = serde_json::to_vec(&task)?;
            self.memory.set(&key, value, Some(3600)).await?;
            debug!("Stored task {} in distributed memory", task_id);
        }

        Ok(())
    }

    /// Get the next task from the queue.
    pub async fn get_next_task(&self) -> Result<Option<Task>> {
        // First check local queue
        let mut queue = self.queue.lock().await;
        if let Some(task) = queue.pop() {
            let mut task = task;
            task.state = TaskState::Running;
            task.started_at = Some(Utc::now());
            self.running_tasks
                .write()
                .await
                .insert(task.id.clone(), task.clone());
            return Ok(Some(task));
        }

        // Check distributed memory for tasks assigned to us
        // This would scan for tasks with our node ID
        // For now, return None
        Ok(None)
    }

    /// Complete a task.
    pub async fn complete_task(&self, task_id: &str, result: TaskResult) -> Result<()> {
        // Remove from running
        let task = self.running_tasks.write().await.remove(task_id);

        if let Some(mut task) = task {
            task.state = TaskState::Completed;
            task.completed_at = Some(Utc::now());

            // Store result
            let result_key = format!("task_result:{}", task_id);
            let result_value = serde_json::to_vec(&result)?;
            self.memory
                .set(&result_key, result_value, Some(3600))
                .await?;

            // Store in completed tasks
            self.completed_tasks
                .write()
                .await
                .insert(task_id.to_string(), (task, result));

            self.tasks_completed.fetch_add(1, Ordering::SeqCst);
            info!("Task {} completed successfully", task_id);
        }

        Ok(())
    }

    /// Mark a task as failed.
    pub async fn fail_task(&self, task_id: &str, error: String) -> Result<()> {
        // Remove from running
        let task = self.running_tasks.write().await.remove(task_id);

        if let Some(mut task) = task {
            task.retry_count += 1;

            if task.retry_count >= task.max_retries {
                task.state = TaskState::Failed;
                task.completed_at = Some(Utc::now());

                let result = TaskResult::failure(error, 0, self.node_id.clone());
                let result_key = format!("task_result:{}", task_id);
                let result_value = serde_json::to_vec(&result)?;
                self.memory
                    .set(&result_key, result_value, Some(3600))
                    .await?;

                let retry_count = task.retry_count;
                self.completed_tasks
                    .write()
                    .await
                    .insert(task_id.to_string(), (task, result));

                self.tasks_failed.fetch_add(1, Ordering::SeqCst);
                error!("Task {} failed after {} retries", task_id, retry_count);
            } else {
                // Retry the task
                task.state = TaskState::Pending;
                task.started_at = None;

                let retry_count = task.retry_count;
                let mut queue = self.queue.lock().await;
                queue.push(task);
                queue.sort_by(|a, b| b.priority.value().cmp(&a.priority.value()));

                warn!(
                    "Task {} failed, retrying (attempt {})",
                    task_id, retry_count
                );
            }
        }

        Ok(())
    }

    /// Cancel a task.
    pub async fn cancel_task(&self, task_id: &str) -> Result<bool> {
        // Try to remove from queue
        let mut queue = self.queue.lock().await;
        if let Some(pos) = queue.iter().position(|t| t.id == task_id) {
            let mut task = queue.remove(pos);
            task.state = TaskState::Cancelled;
            task.completed_at = Some(Utc::now());

            let result = TaskResult::failure("Cancelled", 0, self.node_id.clone());
            self.completed_tasks
                .write()
                .await
                .insert(task_id.to_string(), (task, result));

            return Ok(true);
        }

        // If running, we can't really stop it (would need cancellation tokens)
        // Just mark as cancelled in the result
        Ok(false)
    }

    /// Get task status.
    pub async fn get_task_status(&self, task_id: &str) -> Result<TaskState> {
        // Check running
        if let Some(task) = self.running_tasks.read().await.get(task_id) {
            return Ok(task.state);
        }

        // Check completed
        if self.completed_tasks.read().await.contains_key(task_id) {
            return Ok(TaskState::Completed);
        }

        // Check queue
        let queue = self.queue.lock().await;
        if let Some(task) = queue.iter().find(|t| t.id == task_id) {
            return Ok(task.state);
        }

        // Check distributed memory
        let key = format!("task:{}", task_id);
        if let Some(value) = self.memory.get(&key).await? {
            let task: Task = serde_json::from_slice(&value)?;
            return Ok(task.state);
        }

        Err(DistributedError::TaskNotFound(task_id.to_string()))
    }

    /// Get task result.
    pub async fn get_task_result(&self, task_id: &str) -> Result<TaskResult> {
        // Check local completed
        if let Some((_, result)) = self.completed_tasks.read().await.get(task_id) {
            return Ok(result.clone());
        }

        // Check distributed memory
        let key = format!("task_result:{}", task_id);
        if let Some(value) = self.memory.get(&key).await? {
            let result: TaskResult = serde_json::from_slice(&value)?;
            return Ok(result);
        }

        Err(DistributedError::TaskNotFound(task_id.to_string()))
    }

    /// Get count of running tasks.
    pub async fn running_task_count(&self) -> usize {
        self.running_tasks.read().await.len()
    }

    /// Get count of queued tasks.
    pub async fn queued_task_count(&self) -> usize {
        self.queue.lock().await.len()
    }

    /// Get task metrics.
    pub fn metrics(&self) -> TaskMetrics {
        TaskMetrics {
            completed: self.tasks_completed.load(Ordering::SeqCst),
            failed: self.tasks_failed.load(Ordering::SeqCst),
        }
    }

    /// Start cleanup task.
    fn start_cleanup_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(300)); // 5 minutes

            loop {
                interval.tick().await;

                if self.running.load(Ordering::SeqCst) == 0 {
                    break;
                }

                self.cleanup_completed_tasks().await;
            }
        });
    }

    /// Clean up old completed tasks.
    async fn cleanup_completed_tasks(&self) {
        let cutoff = Utc::now() - chrono::Duration::minutes(10);

        let to_remove: Vec<String> = self
            .completed_tasks
            .read()
            .await
            .iter()
            .filter(|(_, (task, _))| task.completed_at.map(|t| t < cutoff).unwrap_or(false))
            .map(|(id, _)| id.clone())
            .collect();

        for id in to_remove {
            self.completed_tasks.write().await.remove(&id);
            debug!("Cleaned up completed task {}", id);
        }
    }
}

/// Task metrics.
#[derive(Debug, Clone, Copy, Default)]
pub struct TaskMetrics {
    pub completed: u64,
    pub failed: u64,
}

/// Task scheduler (runs on leader).
pub struct TaskScheduler {
    memory: Arc<dyn DistributedMemory>,
    pending_tasks: RwLock<Vec<Task>>,
}

impl TaskScheduler {
    /// Create a new task scheduler.
    pub fn new(memory: Arc<dyn DistributedMemory>) -> Arc<Self> {
        Arc::new(Self {
            memory,
            pending_tasks: RwLock::new(Vec::new()),
        })
    }

    /// Schedule a task.
    pub async fn schedule_task(&self, task: Task) -> Result<()> {
        let mut tasks = self.pending_tasks.write().await;
        tasks.push(task);
        // Sort by priority
        tasks.sort_by(|a, b| b.priority.value().cmp(&a.priority.value()));
        Ok(())
    }

    /// Get pending tasks for assignment.
    pub async fn get_pending_tasks(&self, limit: usize) -> Vec<Task> {
        let tasks = self.pending_tasks.read().await;
        tasks.iter().take(limit).cloned().collect()
    }

    /// Mark task as assigned.
    pub async fn assign_task(&self, task_id: &str, worker_id: NodeId) -> Result<()> {
        let mut tasks = self.pending_tasks.write().await;

        if let Some(pos) = tasks.iter().position(|t| t.id == task_id) {
            let mut task = tasks.remove(pos);
            task.assigned_node = Some(worker_id);
            task.state = TaskState::Scheduled;

            // Store in distributed memory for the worker
            let key = format!("task:{}", task.id);
            let value = serde_json::to_vec(&task)?;
            self.memory.set(&key, value, Some(3600)).await?;
        }

        Ok(())
    }

    /// Create a new task.
    pub fn create_task(
        task_type: impl Into<String>,
        payload: Vec<u8>,
        session_id: impl Into<String>,
        priority: TaskPriority,
    ) -> Task {
        Task {
            id: Uuid::new_v4().to_string(),
            task_type: task_type.into(),
            payload,
            priority,
            labels: HashMap::new(),
            session_id: session_id.into(),
            assigned_node: None,
            state: TaskState::Pending,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            timeout_secs: 300,
            retry_count: 0,
            max_retries: 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_priority_ordering() {
        assert!(TaskPriority::Critical.value() > TaskPriority::High.value());
        assert!(TaskPriority::High.value() > TaskPriority::Normal.value());
        assert!(TaskPriority::Normal.value() > TaskPriority::Low.value());
    }

    #[test]
    fn test_task_result_success() {
        let result = TaskResult::success(b"data".to_vec(), 100, "worker-1".to_string());
        assert!(result.success);
        assert_eq!(result.data, b"data");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_task_result_failure() {
        let result = TaskResult::failure("error", 100, "worker-1".to_string());
        assert!(!result.success);
        assert_eq!(result.error, Some("error".to_string()));
    }

    #[test]
    fn test_task_state_display() {
        assert_eq!(TaskState::Running.to_string(), "running");
        assert_eq!(TaskState::Completed.to_string(), "completed");
    }
}
