-- Webhooks table: external integration webhooks
CREATE TABLE IF NOT EXISTS webhooks (
    id TEXT PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    source TEXT NOT NULL CHECK(source IN ('generic', 'github', 'gitlab', 'stripe', 'slack', 'discord', 'gmail', 'telegram') OR source LIKE 'custom:%'),
    secret TEXT,  -- Secret for signature verification
    action_type TEXT NOT NULL CHECK(action_type IN ('agent_message', 'trigger_skill', 'emit_event', 'custom')),
    template TEXT,  -- Message template (for agent_message action)
    skill_name TEXT,  -- Skill to trigger (for trigger_skill action)
    event_type TEXT,  -- Event type to emit (for emit_event action)
    rate_limit_max INTEGER,  -- Max requests per window
    rate_limit_window INTEGER,  -- Window duration in seconds
    enabled INTEGER DEFAULT 1,
    metadata TEXT DEFAULT '{}',  -- JSON metadata
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_webhooks_enabled ON webhooks(enabled);
CREATE INDEX IF NOT EXISTS idx_webhooks_source ON webhooks(source);

-- Webhook delivery logs: track webhook delivery attempts
CREATE TABLE IF NOT EXISTS webhook_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    webhook_id TEXT NOT NULL REFERENCES webhooks(id),
    request_id TEXT,  -- Unique request identifier
    source_ip TEXT,
    user_agent TEXT,
    payload_size INTEGER,
    status_code INTEGER,
    response_body TEXT,
    processing_time_ms INTEGER,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_webhook_logs_webhook ON webhook_logs(webhook_id, created_at);
CREATE INDEX IF NOT EXISTS idx_webhook_logs_time ON webhook_logs(created_at);
