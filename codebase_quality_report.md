# OpenRustClaw Codebase Quality Report

**Generated:** 2026-03-17 08:26:18 UTC

## 📊 Overall Metrics

| Metric | Value | Status |
|--------|-------|--------|
| Build | - | warning: fields `api_key`, `referer`, and `app_name` are never read
  --> crates/openrouter_api/src/client.rs:20:5
   |
18 | struct ClientInner {
   |        ----------- fields in this struct
19 |     http: reqwest::Client,
20 |     api_key: String,
   |     ^^^^^^^
21 |     base_url: String,
22 |     referer: String,
   |     ^^^^^^^
23 |     app_name: String,
   |     ^^^^^^^^
   |
   = note: `ClientInner` has a derived impl for the trait `Debug`, but this is intentionally ignored during dead code analysis
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: field `api_key` is never read
  --> crates/anthropic_rust/src/client.rs:25:5
   |
23 | struct ClientInner {
   |        ----------- field in this struct
24 |     http: reqwest::Client,
25 |     api_key: String,
   |     ^^^^^^^
   |
   = note: `ClientInner` has a derived impl for the trait `Debug`, but this is intentionally ignored during dead code analysis
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: field `organization` is never read
  --> crates/async_openai/src/client.rs:23:5
   |
19 | struct ClientInner {
   |        ----------- field in this struct
...
23 |     organization: Option<String>,
   |     ^^^^^^^^^^^^
   |
   = note: `ClientInner` has a derived impl for the trait `Debug`, but this is intentionally ignored during dead code analysis
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: methods `get` and `delete` are never used
   --> crates/async_openai/src/client.rs:142:25
    |
 28 | impl OpenAIClient {
    | ----------------- methods in this implementation
...
142 |     pub(crate) async fn get(&self, path: &str) -> Result<reqwest::Response> {
    |                         ^^^
...
175 |     pub(crate) async fn delete(&self, path: &str) -> Result<reqwest::Response> {
    |                         ^^^^^^

warning: `openrouter-api` (lib) generated 1 warning
warning: `anthropic-rust` (lib) generated 1 warning
warning: `async-openai` (lib) generated 2 warnings
warning: unused import: `crate::tool_formats::translate_tool_definition`
  --> crates/providers/src/gemini.rs:22:5
   |
22 | use crate::tool_formats::translate_tool_definition;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: unused variable: `code`
   --> crates/providers/src/gemini.rs:422:21
    |
422 |                 let code = error.get("code").and_then(|v| v.as_i64());
    |                     ^^^^ help: if this is intentional, prefix it with an underscore: `_code`
    |
    = note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default

warning: method `model_max_output_tokens` is never used
  --> crates/providers/src/gemini.rs:57:8
   |
35 | impl GeminiProvider {
   | ------------------- method in this implementation
...
57 |     fn model_max_output_tokens(&self) -> usize {
   |        ^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: `openrustclaw-providers` (lib) generated 3 warnings (run `cargo fix --lib -p openrustclaw-providers` to apply 2 suggestions)
warning: unused variable: `xml`
   --> crates/security/src/sso/saml.rs:241:13
    |
241 |         let xml = String::from_utf8(decoded)
    |             ^^^ help: if this is intentional, prefix it with an underscore: `_xml`
    |
    = note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default

warning: field `issuer` is never read
  --> crates/security/src/sso/oidc.rs:65:5
   |
64 | struct DiscoveryDocument {
   |        ----------------- field in this struct
65 |     issuer: String,
   |     ^^^^^^
   |
   = note: `DiscoveryDocument` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: method `validate_signature` is never used
   --> crates/security/src/sso/saml.rs:255:8
    |
214 | impl SamlClient {
    | --------------- method in this implementation
...
255 |     fn validate_signature(&self, _response: &str) -> Result<(), SsoError> {
    |        ^^^^^^^^^^^^^^^^^^

warning: `openrustclaw-security` (lib) generated 3 warnings (run `cargo fix --lib -p openrustclaw-security` to apply 1 suggestion)
warning: unused import: `set_active_connections`
  --> crates/gateway/src/server.rs:11:31
   |
11 |     record_websocket_message, set_active_connections, SimpleTimer,
   |                               ^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: `openrustclaw-gateway` (lib) generated 1 warning (run `cargo fix --lib -p openrustclaw-gateway` to apply 1 suggestion)
warning: unused import: `TokenData`
  --> crates/channels/src/teams.rs:30:67
   |
30 | use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, TokenData, Validation};
   |                                                                   ^^^^^^^^^
   |
   = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: fields `incoming_tx` and `message_cache` are never read
  --> crates/channels/src/discord.rs:32:5
   |
30 | pub struct DiscordChannel {
   |            -------------- fields in this struct
31 |     config: DiscordConfig,
32 |     incoming_tx: mpsc::Sender<IncomingMessage>,
   |     ^^^^^^^^^^^
...
36 |     message_cache: Arc<RwLock<HashMap<Uuid, String>>>, // Maps session_id to message_id
   |     ^^^^^^^^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: field `http_client` is never read
  --> crates/channels/src/gmail_pubsub.rs:70:5
   |
64 | pub struct GmailPubSub {
   |            ----------- field in this struct
...
70 |     http_client: reqwest::Client,
   |     ^^^^^^^^^^^

warning: field `thread_id` is never read
   --> crates/channels/src/gmail_pubsub.rs:171:9
    |
166 | struct MessageMetadata {
    |        --------------- field in this struct
...
171 |     pub thread_id: String,
    |         ^^^^^^^^^
    |
    = note: `MessageMetadata` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis

warning: struct `MessagePart` is never constructed
   --> crates/channels/src/gmail_pubsub.rs:176:8
    |
176 | struct MessagePart {
    |        ^^^^^^^^^^^

warning: struct `MessagePartBody` is never constructed
   --> crates/channels/src/gmail_pubsub.rs:194:8
    |
194 | struct MessagePartBody {
    |        ^^^^^^^^^^^^^^^

warning: struct `MessageHeader` is never constructed
   --> crates/channels/src/gmail_pubsub.rs:206:8
    |
206 | struct MessageHeader {
    |        ^^^^^^^^^^^^^

warning: struct `GmailMessage` is never constructed
   --> crates/channels/src/gmail_pubsub.rs:215:8
    |
215 | struct GmailMessage {
    |        ^^^^^^^^^^^^

warning: struct `HistoryResponse` is never constructed
   --> crates/channels/src/gmail_pubsub.rs:236:8
    |
236 | struct HistoryResponse {
    |        ^^^^^^^^^^^^^^^

warning: struct `HistoryRecord` is never constructed
   --> crates/channels/src/gmail_pubsub.rs:249:8
    |
249 | struct HistoryRecord {
    |        ^^^^^^^^^^^^^

warning: struct `MessageAdded` is never constructed
   --> crates/channels/src/gmail_pubsub.rs:257:8
    |
257 | struct MessageAdded {
    |        ^^^^^^^^^^^^

warning: struct `ParsedParts` is never constructed
   --> crates/channels/src/gmail_pubsub.rs:264:8
    |
264 | struct ParsedParts {
    |        ^^^^^^^^^^^

warning: associated items `parse_addresses`, `decode_base64`, `parse_message`, `get_parts`, and `is_sender_allowed` are never used
   --> crates/channels/src/gmail_pubsub.rs:302:8
    |
273 | impl GmailPubSub {
    | ---------------- associated items in this implementation
...
302 |     fn parse_addresses(s: &str) -> Vec<String> {
    |        ^^^^^^^^^^^^^^^
...
334 |     fn decode_base64(data: &str) -> Option<Vec<u8>> {
    |        ^^^^^^^^^^^^^
...
341 |     fn parse_message(&self, msg: GmailMessage) -> Result<EmailMessage> {
    |        ^^^^^^^^^^^^^
...
382 |     fn get_parts(&self, payload: &MessagePart) -> ParsedParts {
    |        ^^^^^^^^^
...
638 |     fn is_sender_allowed(&self, _from: &str) -> bool {
    |        ^^^^^^^^^^^^^^^^^

warning: function `decode` is never used
   --> crates/channels/src/gmail_pubsub.rs:822:12
    |
822 |     pub fn decode(data: &str) -> Result<Vec<u8>, base64::DecodeError> {
    |            ^^^^^^

warning: constant `GOOGLE_CHAT_API_BASE` is never used
  --> crates/channels/src/google_chat.rs:39:7
   |
39 | const GOOGLE_CHAT_API_BASE: &str = "https://chat.googleapis.com/v1";
   |       ^^^^^^^^^^^^^^^^^^^^

warning: fields `incoming_tx` and `http_client` are never read
  --> crates/channels/src/google_chat.rs:44:5
   |
42 | pub struct GoogleChatChannel {
   |            ----------------- fields in this struct
43 |     config: GoogleChatConfig,
44 |     incoming_tx: mpsc::Sender<IncomingMessage>,
   |     ^^^^^^^^^^^
...
51 |     http_client: reqwest::Client,
   |     ^^^^^^^^^^^

warning: variant `ButtonList` is never constructed
   --> crates/channels/src/google_chat.rs:107:5
    |
105 | enum Widget {
    |      ------ variant in this enum
106 |     TextParagraph { text: String },
107 |     ButtonList { buttons: Vec<Button> },
    |     ^^^^^^^^^^
    |
    = note: `Widget` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis

warning: variant `OpenLink` is never constructed
   --> crates/channels/src/google_chat.rs:121:5
    |
120 | enum OnClick {
    |      ------- variant in this enum
121 |     OpenLink { open_link: OpenLink },
    |     ^^^^^^^^
    |
    = note: `OnClick` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis

warning: field `event_time` is never read
   --> crates/channels/src/google_chat.rs:136:5
    |
132 | struct ChatEvent {
    |        --------- field in this struct
...
136 |     event_time: String,
    |     ^^^^^^^^^^
    |
    = note: `ChatEvent` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis

warning: fields `argument_text` and `annotations` are never read
   --> crates/channels/src/google_chat.rs:160:5
    |
155 | struct EventMessage {
    |        ------------ fields in this struct
...
160 |     argument_text: Option<String>,
    |     ^^^^^^^^^^^^^
...
163 |     annotations: Option<Vec<Annotation>>,
    |     ^^^^^^^^^^^
    |
    = note: `EventMessage` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis

warning: fields `annotation_type` and `user_mention` are never read
   --> crates/channels/src/google_chat.rs:177:5
    |
175 | struct Annotation {
    |        ---------- fields in this struct
176 |     #[serde(rename = "type")]
177 |     annotation_type: String,
    |     ^^^^^^^^^^^^^^^
178 |     #[serde(rename = "userMention")]
179 |     user_mention: Option<UserMention>,
    |     ^^^^^^^^^^^^
    |
    = note: `Annotation` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis

warning: fields `user` and `mention_type` are never read
   --> crates/channels/src/google_chat.rs:185:5
    |
184 | struct UserMention {
    |        ----------- fields in this struct
185 |     user: User,
    |     ^^^^
186 |     #[serde(rename = "type")]
187 |     mention_type: String,
    |     ^^^^^^^^^^^^
    |
    = note: `UserMention` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis

warning: associated items `is_user_allowed`, `is_space_allowed`, `should_respond`, and `build_card_message` are never used
   --> crates/channels/src/google_chat.rs:228:8
    |
199 | impl GoogleChatChannel {
    | ---------------------- associated items in this implementation
...
228 |     fn is_user_allowed(&self, email: Option<&str>, user_id: &str) -> bool {
    |        ^^^^^^^^^^^^^^^
...
243 |     fn is_space_allowed(&self, space_id: &str) -> bool {
    |        ^^^^^^^^^^^^^^^^
...
251 |     fn should_respond(&self, event: &ChatEvent) -> bool {
    |        ^^^^^^^^^^^^^^
...
287 |     fn build_card_message(
    |        ^^^^^^^^^^^^^^^^^^

warning: fields `incoming_tx`, `message_cache`, and `client` are never read
  --> crates/channels/src/matrix.rs:36:5
   |
34 | pub struct MatrixChannel {
   |            ------------- fields in this struct
35 |     config: MatrixConfig,
36 |     incoming_tx: mpsc::Sender<IncomingMessage>,
   |     ^^^^^^^^^^^
...
41 |     message_cache: Arc<RwLock<HashMap<Uuid, String>>>,
   |     ^^^^^^^^^^^^^
42 |     /// Client handle (placeholder for actual matrix-sdk Client)
43 |     client: Arc<RwLock<Option<Arc<()>>>>,
   |     ^^^^^^

warning: associated items `is_user_allowed`, `is_room_allowed`, `extract_display_name`, and `html_to_text` are never used
  --> crates/channels/src/matrix.rs:69:8
   |
46 | impl MatrixChannel {
   | ------------------ associated items in this implementation
...
69 |     fn is_user_allowed(&self, user_id: &str) -> bool {
   |        ^^^^^^^^^^^^^^^
...
77 |     fn is_room_allowed(&self, room_id: &str) -> bool {
   |        ^^^^^^^^^^^^^^^
...
86 |     fn extract_display_name(user_id: &str) -> String {
   |        ^^^^^^^^^^^^^^^^^^^^
...
96 |     fn html_to_text(html: &str) -> String {
   |        ^^^^^^^^^^^^

warning: associated function `platform_from_metadata` is never used
   --> crates/channels/src/meta.rs:447:8
    |
148 | impl MetaChannel {
    | ---------------- associated function in this implementation
...
447 |     fn platform_from_metadata(metadata: &serde_json::Value) -> Platform {
    |        ^^^^^^^^^^^^^^^^^^^^^^

warning: field `incoming_tx` is never read
  --> crates/channels/src/slack.rs:29:5
   |
27 | pub struct SlackChannel {
   |            ------------ field in this struct
28 |     config: SlackConfig,
29 |     incoming_tx: mpsc::Sender<IncomingMessage>,
   |     ^^^^^^^^^^^

warning: associated function `slack_to_markdown` is never used
  --> crates/channels/src/slack.rs:56:8
   |
35 | impl SlackChannel {
   | ----------------- associated function in this implementation
...
56 |     fn slack_to_markdown(text: &str) -> String {
   |        ^^^^^^^^^^^^^^^^^

warning: field `token_endpoint` is never read
   --> crates/channels/src/teams.rs:125:9
    |
121 | struct OpenIdConfig {
    |        ------------ field in this struct
...
125 |     pub token_endpoint: Option<String>,
    |         ^^^^^^^^^^^^^^
    |
    = note: `OpenIdConfig` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis

warning: fields `kty` and `use_` are never read
   --> crates/channels/src/teams.rs:131:9
    |
130 | struct Jwk {
    |        --- fields in this struct
131 |     pub kty: String,
    |         ^^^
...
134 |     pub use_: Option<String>,
    |         ^^^^
    |
    = note: `Jwk` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis

warning: method `bot_framework_url` is never used
   --> crates/channels/src/teams.rs:232:8
    |
192 | impl TeamsChannel {
    | ----------------- method in this implementation
...
232 |     fn bot_framework_url(&self) -> &'static str {
    |        ^^^^^^^^^^^^^^^^^

warning: field `incoming_tx` is never read
  --> crates/channels/src/telegram.rs:28:5
   |
26 | pub struct TelegramChannel {
   |            --------------- field in this struct
27 |     config: TelegramConfig,
28 |     incoming_tx: mpsc::Sender<IncomingMessage>,
   |     ^^^^^^^^^^^

warning: method `is_user_allowed` is never used
  --> crates/channels/src/telegram.rs:55:8
   |
34 | impl TelegramChannel {
   | -------------------- method in this implementation
...
55 |     fn is_user_allowed(&self, user_id: i64) -> bool {
   |        ^^^^^^^^^^^^^^^

warning: methods `is_number_allowed` and `validate_dm_access` are never used
   --> crates/channels/src/whatsapp.rs:220:8
    |
183 | impl WhatsAppChannel {
    | -------------------- methods in this implementation
...
220 |     fn is_number_allowed(&self, phone_number: &str) -> bool {
    |        ^^^^^^^^^^^^^^^^^
...
239 |     fn validate_dm_access(&self, phone_number: &str, is_group: bool) -> bool {
    |        ^^^^^^^^^^^^^^^^^^

warning: `openrustclaw-channels` (lib) generated 34 warnings (run `cargo fix --lib -p openrustclaw-channels` to apply 1 suggestion)
   Compiling openrustclaw-distributed v0.1.0 (/home/hprincivil/projects/OpenRustClaw/crates/distributed)
warning: field `config` is never read
  --> crates/cursor/src/client.rs:54:5
   |
52 | pub struct CursorClient {
   |            ------------ field in this struct
53 |     connection: ClientConnection,
54 |     config: CursorConfig,
   |     ^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: enum `ConnectionHandle` is never used
  --> crates/cursor/src/client.rs:59:6
   |
59 | enum ConnectionHandle {
   |      ^^^^^^^^^^^^^^^^

warning: fields `id`, `initialized`, and `capabilities` are never read
  --> crates/cursor/src/server.rs:66:5
   |
65 | struct ConnectionState {
   |        --------------- fields in this struct
66 |     id: String,
   |     ^^
67 |     initialized: bool,
   |     ^^^^^^^^^^^
68 |     capabilities: AcpCapabilities,
   |     ^^^^^^^^^^^^
   |
   = note: `ConnectionState` has a derived impl for the trait `Debug`, but this is intentionally ignored during dead code analysis

warning: missing documentation for a struct field
  --> crates/cursor/src/acp.rs:50:20
   |
50 |     FileModified { path: std::path::PathBuf },
   |                    ^^^^^^^^^^^^^^^^^^^^^^^^
   |
note: the lint level is defined here
  --> crates/cursor/src/lib.rs:85:9
   |
85 | #![warn(missing_docs)]
   |         ^^^^^^^^^^^^

warning: missing documentation for a struct field
  --> crates/cursor/src/acp.rs:52:22
   |
52 |     TerminalOutput { terminal_id: String, output: String },
   |                      ^^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
  --> crates/cursor/src/acp.rs:52:43
   |
52 |     TerminalOutput { terminal_id: String, output: String },
   |                                           ^^^^^^^^^^^^^^

warning: missing documentation for a struct field
  --> crates/cursor/src/acp.rs:54:25
   |
54 |     DiagnosticUpdated { file_path: std::path::PathBuf },
   |                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
  --> crates/cursor/src/client.rs:30:11
   |
30 |     Tcp { host: String, port: u16 },
   |           ^^^^^^^^^^^^

warning: missing documentation for a struct field
  --> crates/cursor/src/client.rs:30:25
   |
30 |     Tcp { host: String, port: u16 },
   |                         ^^^^^^^^^

warning: missing documentation for a struct field
  --> crates/cursor/src/client.rs:32:17
   |
32 |     WebSocket { url: String },
   |                 ^^^^^^^^^^^

warning: missing documentation for a variant
 --> crates/cursor/src/error.rs:9:5
  |
9 |     Connection(String),
  |     ^^^^^^^^^^

warning: missing documentation for a variant
  --> crates/cursor/src/error.rs:12:5
   |
12 |     Protocol(String),
   |     ^^^^^^^^

warning: missing documentation for a variant
  --> crates/cursor/src/error.rs:15:5
   |
15 |     ToolNotFound(String),
   |     ^^^^^^^^^^^^

warning: missing documentation for a variant
  --> crates/cursor/src/error.rs:18:5
   |
18 |     ToolExecution { tool: String, message: String },
   |     ^^^^^^^^^^^^^

warning: missing documentation for a struct field
  --> crates/cursor/src/error.rs:18:21
   |
18 |     ToolExecution { tool: String, message: String },
   |                     ^^^^^^^^^^^^

warning: missing documentation for a struct field
  --> crates/cursor/src/error.rs:18:35
   |
18 |     ToolExecution { tool: String, message: String },
   |                                   ^^^^^^^^^^^^^^^

warning: missing documentation for a variant
  --> crates/cursor/src/error.rs:21:5
   |
21 |     FileOperation(String),
   |     ^^^^^^^^^^^^^

warning: missing documentation for a variant
  --> crates/cursor/src/error.rs:24:5
   |
24 |     GitOperation(String),
   |     ^^^^^^^^^^^^

warning: missing documentation for a variant
  --> crates/cursor/src/error.rs:27:5
   |
27 |     Terminal(String),
   |     ^^^^^^^^

warning: missing documentation for a variant
  --> crates/cursor/src/error.rs:30:5
   |
30 |     Linter(String),
   |     ^^^^^^

warning: missing documentation for a variant
  --> crates/cursor/src/error.rs:33:5
   |
33 |     Config(String),
   |     ^^^^^^

warning: missing documentation for a variant
  --> crates/cursor/src/error.rs:36:5
   |
36 |     InvalidContext(String),
   |     ^^^^^^^^^^^^^^

warning: missing documentation for a variant
  --> crates/cursor/src/error.rs:39:5
   |
39 |     ProjectRootNotFound,
   |     ^^^^^^^^^^^^^^^^^^^

warning: missing documentation for a variant
  --> crates/cursor/src/error.rs:42:5
   |
42 |     PatternError(String),
   |     ^^^^^^^^^^^^

warning: missing documentation for a variant
  --> crates/cursor/src/error.rs:45:5
   |
45 |     Timeout(String),
   |     ^^^^^^^

warning: missing documentation for a struct field
  --> crates/cursor/src/server.rs:28:12
   |
28 |     Http { port: u16 },
   |            ^^^^^^^^^

warning: missing documentation for a struct field
  --> crates/cursor/src/tools/terminal.rs:25:5
   |
25 |     pub id: String,
   |     ^^^^^^^^^^^^^^

warning: missing documentation for a struct field
  --> crates/cursor/src/tools/terminal.rs:26:5
   |
26 |     pub cwd: std::path::PathBuf,
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
  --> crates/cursor/src/tools/terminal.rs:27:5
   |
27 |     pub last_command: Option<String>,
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
  --> crates/cursor/src/tools/terminal.rs:28:5
   |
28 |     pub recent_output: String,
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
  --> crates/cursor/src/tools/terminal.rs:29:5
   |
29 |     pub is_running: bool,
   |     ^^^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
  --> crates/cursor/src/tools/terminal.rs:30:5
   |
30 |     pub last_exit_code: Option<i32>,
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
  --> crates/cursor/src/tools/terminal.rs:31:5
   |
31 |     pub history: Vec<CommandHistoryEntry>,
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
  --> crates/cursor/src/tools/terminal.rs:37:5
   |
37 |     pub command: String,
   |     ^^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
  --> crates/cursor/src/tools/terminal.rs:38:5
   |
38 |     pub stdout: String,
   |     ^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
  --> crates/cursor/src/tools/terminal.rs:39:5
   |
39 |     pub stderr: String,
   |     ^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
  --> crates/cursor/src/tools/terminal.rs:40:5
   |
40 |     pub exit_code: i32,
   |     ^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
  --> crates/cursor/src/tools/terminal.rs:41:5
   |
41 |     pub executed_at: chrono::DateTime<chrono::Utc>,
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
  --> crates/cursor/src/tools/terminal.rs:42:5
   |
42 |     pub duration_ms: u64,
   |     ^^^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/tools/terminal.rs:235:5
    |
235 |     pub stdout: String,
    |     ^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/tools/terminal.rs:236:5
    |
236 |     pub stderr: String,
    |     ^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/tools/terminal.rs:237:5
    |
237 |     pub exit_code: i32,
    |     ^^^^^^^^^^^^^^^^^^

warning: missing documentation for a variant
   --> crates/cursor/src/types.rs:159:5
    |
159 |     Error,
    |     ^^^^^

warning: missing documentation for a variant
   --> crates/cursor/src/types.rs:160:5
    |
160 |     Warning,
    |     ^^^^^^^

warning: missing documentation for a variant
   --> crates/cursor/src/types.rs:161:5
    |
161 |     Information,
    |     ^^^^^^^^^^^

warning: missing documentation for a variant
   --> crates/cursor/src/types.rs:162:5
    |
162 |     Hint,
    |     ^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:187:22
    |
187 |     ExecuteCommand { command: String, terminal_id: Option<String> },
    |                      ^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:187:39
    |
187 |     ExecuteCommand { command: String, terminal_id: Option<String> },
    |                                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:189:16
    |
189 |     ReadFile { path: PathBuf },
    |                ^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:191:17
    |
191 |     WriteFile { path: PathBuf, content: String },
    |                 ^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:191:32
    |
191 |     WriteFile { path: PathBuf, content: String },
    |                                ^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:194:9
    |
194 |         path: PathBuf,
    |         ^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:195:9
    |
195 |         old_text: String,
    |         ^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:196:9
    |
196 |         new_text: String,
    |         ^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:200:9
    |
200 |         query: String,
    |         ^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:201:9
    |
201 |         path_pattern: Option<String>,
    |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:202:9
    |
202 |         max_results: Option<usize>,
    |         ^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:205:17
    |
205 |     ListFiles { path: PathBuf, recursive: bool },
    |                 ^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:205:32
    |
205 |     ListFiles { path: PathBuf, recursive: bool },
    |                                ^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:209:15
    |
209 |     GitDiff { staged: bool },
    |               ^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:211:18
    |
211 |     GitCommand { args: Vec<String> },
    |                  ^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:213:17
    |
213 |     RunLinter { tool: String, path: Option<PathBuf> },
    |                 ^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:213:31
    |
213 |     RunLinter { tool: String, path: Option<PathBuf> },
    |                               ^^^^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:224:9
    |
224 |         stdout: String,
    |         ^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:225:9
    |
225 |         stderr: String,
    |         ^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:226:9
    |
226 |         exit_code: i32,
    |         ^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:230:9
    |
230 |         path: PathBuf,
    |         ^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:231:9
    |
231 |         content: String,
    |         ^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:235:9
    |
235 |         path: PathBuf,
    |         ^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:236:9
    |
236 |         operation: String,
    |         ^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:240:9
    |
240 |         matches: Vec<SearchMatch>,
    |         ^^^^^^^^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:241:9
    |
241 |         total: usize,
    |         ^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:245:9
    |
245 |         path: PathBuf,
    |         ^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:246:9
    |
246 |         entries: Vec<DirEntry>,
    |         ^^^^^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:254:9
    |
254 |         tool: String,
    |         ^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:255:9
    |
255 |         output: String,
    |         ^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:256:9
    |
256 |         diagnostics: Vec<Diagnostic>,
    |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:260:9
    |
260 |         message: String,
    |         ^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:261:9
    |
261 |         code: Option<String>,
    |         ^^^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:300:5
    |
300 |     pub branch: String,
    |     ^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:301:5
    |
301 |     pub modified: Vec<PathBuf>,
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:302:5
    |
302 |     pub staged: Vec<PathBuf>,
    |     ^^^^^^^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:303:5
    |
303 |     pub untracked: Vec<PathBuf>,
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:304:5
    |
304 |     pub renamed: Vec<(PathBuf, PathBuf)>,
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:305:5
    |
305 |     pub deleted: Vec<PathBuf>,
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:306:5
    |
306 |     pub ahead: usize,
    |     ^^^^^^^^^^^^^^^^

warning: missing documentation for a struct field
   --> crates/cursor/src/types.rs:307:5
    |
307 |     pub behind: usize,
    |     ^^^^^^^^^^^^^^^^^

warning: fields `client` and `server_url` are never read
  --> crates/mcp2cli/src/adapters/mcp_adapter.rs:17:5
   |
16 | pub struct McpAdapter {
   |            ---------- fields in this struct
17 |     client: McpClient,
   |     ^^^^^^
18 |     server_url: Option<String>,
   |     ^^^^^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: associated functions `mcp_tool_to_summary`, `mcp_tool_to_help`, `extract_parameters`, `json_schema_type`, `args_to_json`, and `parse_value` are never used
   --> crates/mcp2cli/src/adapters/mcp_adapter.rs:76:8
    |
 21 | impl McpAdapter {
    | --------------- associated functions in this implementation
...
 76 |     fn mcp_tool_to_summary(tool: &McpToolDef) -> ToolSummary {
    |        ^^^^^^^^^^^^^^^^^^^
...
 90 |     fn mcp_tool_to_help(tool: &McpToolDef) -> ToolHelp {
    |        ^^^^^^^^^^^^^^^^
...
119 |     fn extract_parameters(schema: &Value) -> Vec<ParamHelp> {
    |        ^^^^^^^^^^^^^^^^^^
...
169 |     fn json_schema_type(prop: &Value) -> String {
    |        ^^^^^^^^^^^^^^^^
...
187 |     fn args_to_json(args: &Value, params: &[ParamHelp]) -> Value {
    |        ^^^^^^^^^^^^
...
230 |     fn parse_value(s: &str, type_name: &str) -> Value {
    |        ^^^^^^^^^^^

warning: field `spec` is never read
  --> crates/mcp2cli/src/adapters/openapi_adapter.rs:18:5
   |
17 | pub struct OpenApiAdapter {
   |            -------------- field in this struct
18 |     spec: OpenAPI,
   |     ^^^^

warning: unused import: `VoiceError`
 --> crates/voice/src/tts.rs:5:20
  |
5 | use crate::error::{VoiceError, VoiceResult};
  |                    ^^^^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: unused import: `crate::VoiceError`
 --> crates/voice/src/types.rs:4:5
  |
4 | use crate::VoiceError;
  |     ^^^^^^^^^^^^^^^^^

warning: unused variable: `vad_state`
   --> crates/voice/src/wake.rs:155:25
    |
155 |                     let vad_state = vad.process_frame(&frame);
    |                         ^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_vad_state`
    |
    = note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default

warning: fields `stt`, `event_rx`, and `audio_rx` are never read
   --> crates/voice/src/talk_mode.rs:168:5
    |
164 | pub struct TalkMode {
    |            -------- fields in this struct
...
168 |     stt: Arc<SpeechToText>,
    |     ^^^
...
182 |     event_rx: Arc<Mutex<mpsc::Receiver<TalkEvent>>>,
    |     ^^^^^^^^
...
192 |     audio_rx: Arc<Mutex<mpsc::Receiver<Vec<f32>>>>,
    |     ^^^^^^^^
    |
    = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: method `update_activity` is never used
   --> crates/voice/src/talk_mode.rs:471:14
    |
195 | impl TalkMode {
    | ------------- method in this implementation
...
471 |     async fn update_activity(&self) {
    |              ^^^^^^^^^^^^^^^

warning: field `config` is never read
   --> crates/voice/src/wake.rs:211:5
    |
210 | pub struct PorcupineWakeDetector {
    |            --------------------- field in this struct
211 |     config: WakeWordConfig,
    |     ^^^^^^

warning: `openrustclaw-cursor` (lib) generated 87 warnings
warning: `openrustclaw-mcp2cli` (lib) generated 3 warnings
warning: `openrustclaw-voice` (lib) generated 6 warnings (run `cargo fix --lib -p openrustclaw-voice` to apply 3 suggestions)
warning: field `default_timeout` is never read
  --> tests/e2e/src/common/http_client.rs:11:5
   |
 8 | pub struct TestHttpClient {
   |            -------------- field in this struct
...
11 |     default_timeout: Duration,
   |     ^^^^^^^^^^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: methods `http`, `get`, and `delete` are never used
   --> crates/azure_openai/src/client.rs:226:19
    |
 25 | impl AzureOpenAIClient {
    | ---------------------- methods in this implementation
...
226 |     pub(crate) fn http(&self) -> &reqwest::Client {
    |                   ^^^^
...
294 |     pub(crate) async fn get(&self, path: &str) -> Result<reqwest::Response> {
    |                         ^^^
...
332 |     pub(crate) async fn delete(&self, path: &str) -> Result<reqwest::Response> {
    |                         ^^^^^^
    |
    = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: fields `expires_in` and `token_type` are never read
   --> crates/azure_openai/src/auth.rs:131:5
    |
129 | struct TokenResponse {
    |        ------------- fields in this struct
130 |     access_token: String,
131 |     expires_in: u64,
    |     ^^^^^^^^^^
132 |     token_type: String,
    |     ^^^^^^^^^^
    |
    = note: `TokenResponse` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis

warning: fields `expires_on`, `resource`, and `token_type` are never read
   --> crates/azure_openai/src/auth.rs:274:5
    |
272 | struct ManagedIdentityTokenResponse {
    |        ---------------------------- fields in this struct
273 |     access_token: String,
274 |     expires_on: Option<String>,
    |     ^^^^^^^^^^
275 |     resource: Option<String>,
    |     ^^^^^^^^
276 |     token_type: Option<String>,
    |     ^^^^^^^^^^
    |
    = note: `ManagedIdentityTokenResponse` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis

warning: `openrustclaw-e2e-tests` (lib) generated 1 warning
warning: `azure-openai` (lib) generated 3 warnings
warning: function `get_preset_handler` is never used
   --> crates/cli/src/commands/webhooks.rs:605:8
    |
605 | pub fn get_preset_handler(source: &str, secret: Option<String>) -> Option<WebhookHandler> {
    |        ^^^^^^^^^^^^^^^^^^
    |
    = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: field `session_id` is never read
  --> crates/automation/src/cdp.rs:33:5
   |
30 | pub struct CdpBackend {
   |            ---------- field in this struct
...
33 |     session_id: String,
   |     ^^^^^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: method `enable_domain` is never used
  --> crates/automation/src/cdp.rs:94:14
   |
36 | impl CdpBackend {
   | --------------- method in this implementation
...
94 |     async fn enable_domain(&self, domain: &str) -> Result<()> {
   |              ^^^^^^^^^^^^^

warning: field `context_id` is never read
   --> crates/automation/src/cdp.rs:181:5
    |
180 | pub struct CdpContext {
    |            ---------- field in this struct
181 |     context_id: String,
    |     ^^^^^^^^^^

warning: fields `node_id` and `remote_object_id` are never read
   --> crates/automation/src/cdp.rs:573:5
    |
570 | pub struct CdpElement {
    |            ---------- fields in this struct
...
573 |     node_id: Option<i64>,
    |     ^^^^^^^
574 |     remote_object_id: Option<String>,
    |     ^^^^^^^^^^^^^^^^

warning: fields `url_pattern` and `resource_type` are never read
   --> crates/automation/src/cdp.rs:936:5
    |
935 | pub struct RequestPattern {
    |            -------------- fields in this struct
936 |     url_pattern: String,
    |     ^^^^^^^^^^^
937 |     resource_type: Option<ResourceType>,
    |     ^^^^^^^^^^^^^
    |
    = note: `RequestPattern` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis

warning: field `browser_type` is never read
  --> crates/automation/src/playwright.rs:27:5
   |
24 | pub struct PlaywrightBackend {
   |            ----------------- field in this struct
...
27 |     browser_type: String,
   |     ^^^^^^^^^^^^

warning: hidden lifetime parameters in types are deprecated
   --> crates/bedrock/src/auth/mod.rs:181:54
    |
181 |         let signing_params: aws_sigv4::http_request::SigningParams = signing_params.into();
    |                             -------------------------^^^^^^^^^^^^^
    |                             |
    |                             expected lifetime parameter
    |
note: the lint level is defined here
   --> crates/bedrock/src/lib.rs:61:9
    |
 61 | #![warn(rust_2018_idioms)]
    |         ^^^^^^^^^^^^^^^^
    = note: `#[warn(elided_lifetimes_in_paths)]` implied by `#[warn(rust_2018_idioms)]`
help: indicate the anonymous lifetime
    |
181 |         let signing_params: aws_sigv4::http_request::SigningParams<'_> = signing_params.into();
    |                                                                   ++++

warning: associated items `default_config_path` and `parse_config_file` are never used
   --> crates/bedrock/src/auth/credentials.rs:213:8
    |
185 | impl ProfileCredentialProvider {
    | ------------------------------ associated items in this implementation
...
213 |     fn default_config_path() -> Option<PathBuf> {
    |        ^^^^^^^^^^^^^^^^^^^
...
233 |     fn parse_config_file(&self, path: &PathBuf) -> Result<HashMap<String, HashMap<String, String>>> {
    |        ^^^^^^^^^^^^^^^^^
    |
    = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: field `region` is never read
  --> crates/bedrock/src/converse/mod.rs:41:5
   |
39 | struct ConverseClientInner {
   |        ------------------- field in this struct
40 |     http: reqwest::Client,
41 |     region: crate::auth::Region,
   |     ^^^^^^
   |
   = note: `ConverseClientInner` has a derived impl for the trait `Debug`, but this is intentionally ignored during dead code analysis

warning: struct `ConverseStreamRequest` is never constructed
  --> crates/bedrock/src/converse/request.rs:84:12
   |
84 | pub struct ConverseStreamRequest {
   |            ^^^^^^^^^^^^^^^^^^^^^

warning: associated function `from_request` is never used
  --> crates/bedrock/src/converse/request.rs:92:12
   |
90 | impl ConverseStreamRequest {
   | -------------------------- associated function in this implementation
91 |     /// Create from a ConverseRequest.
92 |     pub fn from_request(request: ConverseRequest) -> Self {
   |            ^^^^^^^^^^^^

warning: fields `id` and `name` are never read
   --> crates/bedrock/src/streaming/mod.rs:124:9
    |
123 |     ToolUse {
    |     ------- fields in this variant
124 |         id: String,
    |         ^^
125 |         name: String,
    |         ^^^^
    |
    = note: `PartialBlock` has a derived impl for the trait `Debug`, but this is intentionally ignored during dead code analysis

warning: `openrustclaw-cli` (bin "openrustclaw") generated 1 warning
warning: `openrustclaw-automation` (lib) generated 6 warnings
warning: `aws-bedrock` (lib) generated 6 warnings
warning: method `delete` is never used
   --> crates/vllm/src/client.rs:234:25
    |
 29 | impl VllmClient {
    | --------------- method in this implementation
...
234 |     pub(crate) async fn delete(&self, path: &str) -> Result<reqwest::Response> {
    |                         ^^^^^^
    |
    = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: `vllm` (lib) generated 1 warning
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.78s
✅ PASS |
| Total unwrap() | 488 | ❌ |
| Total tests | 963 | - |
| TODO/FIXME count | 11 | ✅ |
| Lines of code | 139211 | - |

## 📦 Per-Crate Breakdown

| Crate | Lines | unwrap() | Tests | TODOs | README |
|-------|-------|----------|-------|-------|--------|
| core | 2434 | 5 | 8 | 0 | ✅ |
| security | 1782 | 14 | 31 | 0 | ✅ |
| channels | 11419 | 44 | 91 | 0 | ✅ |
| gateway | 2302 | 10 | 14 | 0 | ✅ |
| agent | 2677 | 8 | 2 | 0 | ✅ |
| distributed | 7045 | 38 | 15 | 3 | ✅ |
| mobile | 1694 | 19 | 17 | 0 | ❌ |

## 🎯 Scoring Guide

### Current vs Target (10/10)

| Category | Current | Target | Gap |
|----------|---------|--------|-----|
| Architecture | ~8/10 | 10/10 | 2 |
| Code Quality | 3/10 | 10/10 | - |
| Security | ~8/10 | 10/10 | 2 |
| Testing | 5/10 | 10/10 | - |
| Documentation | 7/10 | 10/10 | - |
| Build Status | 3/10 | 10/10 | - |

## 🔍 Detailed Findings

### Top Files by unwrap() Count

| File | Count |
|------|-------|
| crates/cursor/tests/integration_tests.rs | 40 |
| crates/cursor/src/tools/codebase.rs | 23 |
| crates/distributed/tests/integration_tests.rs | 17 |
| crates/mobile/src/android.rs | 13 |
| crates/channels/src/commands.rs | 13 |
| crates/mcp2cli/tests/integration_tests.rs | 12 |
| crates/distributed/src/memory.rs | 11 |
| crates/providers/src/gemini.rs | 10 |
| crates/ai21/tests/integration_tests.rs | 9 |
| crates/mcp2cli/src/toon.rs | 8 |
| crates/distributed/src/load_balancer.rs | 8 |
| crates/azure_openai/src/client.rs | 8 |
| crates/providers/src/anthropic.rs | 6 |
| crates/mcp2cli/src/adapters/openapi_adapter.rs | 6 |
| crates/gateway/src/webhooks.rs | 6 |
| crates/distributed/src/lib.rs | 6 |
| crates/cursor/src/tools/git.rs | 6 |
| crates/cursor/src/lib.rs | 6 |

### Security TODOs


### Architecture TODOs


---

*Report generated by OpenRustClaw Quality Tracker*
