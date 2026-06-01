use super::constants::VERSION;

const HELP_BODY: &str = r#"
Usage:
  tovuk init [path] [--template rust-worker|tanstack-static-frontend|worker-static-rust-tanstack]
  tovuk install [path] [--template rust-worker|tanstack-static-frontend|worker-static-rust-tanstack]
  tovuk check [path] [--json]
  tovuk preview [path] [--port <port>]
  tovuk login [--token <token>] [--api <url>]
  tovuk plan [path] [--api <url>] [--json]
  tovuk deploy [path] [--wait] [--wait-timeout <seconds>] [--api <url>] [--json]
  tovuk capabilities [--api <url>] [--json]
  tovuk pricing [--api <url>] [--json]
  tovuk me [--api <url>] [--json]
  tovuk usage [--api <url>] [--json]
  tovuk activity [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  tovuk service list [--api <url>] [--json]
  tovuk service show <service> [--api <url>] [--json]
  tovuk service delete <service> [--api <url>] [--json]
  tovuk overview --service <service> [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  tovuk deploys [--service <service>] [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  tovuk builds [--service <service>] [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  tovuk logs --service <service> [--deploy <deploy_id>] [--build <build_id>] [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  tovuk status --service <service> [--api <url>] [--json]
  tovuk inspect --service <service> [--api <url>] [--json]
  tovuk platform --service <service> [--api <url>] [--json]
  tovuk database create --service <service> DB [--api <url>] [--json]
  tovuk database query --service <service> DB "select 1" [--params <json_array>] [--api <url>] [--json]
  tovuk database backup [list|create|restore] --service <service> DB [backup_id] [--api <url>] [--json]
  tovuk database delete --service <service> DB [--api <url>] [--json]
  tovuk kv create --service <service> CACHE [--api <url>] [--json]
  tovuk kv namespace delete --service <service> CACHE [--api <url>] [--json]
  tovuk kv keys --service <service> CACHE [--api <url>] [--json]
  tovuk kv get --service <service> CACHE <key> [--api <url>] [--json]
  tovuk kv put --service <service> CACHE <key> <value> [--metadata <json>] [--expiration <unix_seconds>] [--ttl <seconds>] [--api <url>] [--json]
  tovuk kv delete --service <service> CACHE <key> [--api <url>] [--json]
  tovuk kv bulk put --service <service> CACHE '[{"key":"a","value":"1"}]' [--api <url>] [--json]
  tovuk kv bulk get --service <service> CACHE key-a key-b [--api <url>] [--json]
  tovuk kv bulk delete --service <service> CACHE key-a key-b [--api <url>] [--json]
  tovuk queue create --service <service> jobs [--max-retries <n>] [--retention-seconds <seconds>] [--max-batch-size <n>] [--max-batch-timeout-seconds <seconds>] [--dead-letter-queue <queue>] [--api <url>] [--json]
  tovuk queue update --service <service> jobs [--max-retries <n>] [--retention-seconds <seconds>] [--max-batch-size <n>] [--max-batch-timeout-seconds <seconds>] [--dead-letter-queue <queue>|--clear-dead-letter-queue] [--api <url>] [--json]
  tovuk queue messages --service <service> jobs [--api <url>] [--json]
  tovuk queue metrics --service <service> jobs [--api <url>] [--json]
  tovuk queue send --service <service> jobs <body> [--delay-seconds <seconds>] [--api <url>] [--json]
  tovuk queue send-batch --service <service> jobs '[{"body":{"task":"sync"}}]' [--delay-seconds <seconds>] [--api <url>] [--json]
  tovuk queue delete --service <service> jobs [--api <url>] [--json]
  tovuk cron create --service <service> nightly "0 0 * * *" [--api <url>] [--json]
  tovuk cron update --service <service> nightly "*/15 * * * *" [--api <url>] [--json]
  tovuk cron enable --service <service> nightly [--api <url>] [--json]
  tovuk cron disable --service <service> nightly [--api <url>] [--json]
  tovuk cron delete --service <service> nightly [--api <url>] [--json]
  tovuk state create --service <service> Room [--api <url>] [--json]
  tovuk state objects --service <service> Room [--api <url>] [--json]
  tovuk state keys --service <service> Room room-1 [--api <url>] [--json]
  tovuk state get --service <service> Room room-1 counter [--api <url>] [--json]
  tovuk state put --service <service> Room room-1 counter 1 [--api <url>] [--json]
  tovuk state alarm [get|delete] --service <service> Room room-1 [--api <url>] [--json]
  tovuk state alarm set --service <service> Room room-1 [--delay-seconds <seconds>|<unix_ms>] [--api <url>] [--json]
  tovuk state delete-value --service <service> Room room-1 counter [--api <url>] [--json]
  tovuk state delete --service <service> Room [--api <url>] [--json]
  tovuk binding create --service <service> AUTH_SERVICE --target <target_service> [--api <url>] [--json]
  tovuk binding delete --service <service> AUTH_SERVICE [--api <url>] [--json]
  tovuk limit set build_minutes --period month --value 6000 [--api <url>] [--json]
  tovuk limit set worker_requests --period day --value 100000 [--api <url>] [--json]
  tovuk limit set state_requests --period month --value 1000000 [--api <url>] [--json]
  tovuk limit set state_sqlite_rows_written --period month --value 50000000 [--api <url>] [--json]
  tovuk limit delete worker_requests --period day [--api <url>] [--json]
  tovuk env list --service <service> [--api <url>] [--json]
  tovuk env set --service <service> KEY=value [--api <url>] [--json]
  tovuk env delete --service <service> KEY [--api <url>] [--json]
  tovuk domains list --service <service> [--api <url>] [--json]
  tovuk domains add --service <service> <domain> [--api <url>] [--json]
  tovuk domains verify --service <service> <domain> [--api <url>] [--json]
  tovuk domains delete --service <service> <domain> [--api <url>] [--json]
  tovuk storage list --service <service> [--api <url>] [--json]
  tovuk storage upload --service <service> <local_file> [storage_path] [--content-type <mime>] [--public] [--api <url>] [--json]
  tovuk storage download --service <service> <storage_path> [local_file] [--api <url>] [--json]
  tovuk storage delete --service <service> <storage_path> [--api <url>] [--json]
  tovuk storage url --service <service> <storage_path> [--api <url>] [--json]
  tovuk billing [checkout|portal] [reason] [--api <url>] [--json]
  tovuk support list [--limit <n>] [--api <url>] [--json]
  tovuk support create "Subject" "Details" [--service <service>] [--build <build_id>] [--deploy <deploy_id>] [--failing-command <command>] [--first-log-line <line>] [--severity low|normal|urgent] [--api <url>] [--json]
  tovuk support resolve <ticket_id> [--api <url>] [--json]

Agent contract:
  - Worker-static services set kind = "worker_static", keep worker and frontend roots in one tovuk.toml, serve the frontend at /, and serve the Rust worker API under /api.
  - Rust workers keep Cargo.lock committed, pass rustfmt plus locked release-mode check/test/Clippy gates, listen on 0.0.0.0:$PORT, and return HTTP 200 from health.
  - Static frontends set kind = "static_frontend", keep TypeScript source, a package lockfile, stable native typecheck, native lint, and Fallow quality gates.
  - Plain static HTML/CSS/JS frontends may use kind = "static_frontend" with check = ":", command = ":", and output = ".".
  - JavaScript and TypeScript are frontend-only on Tovuk; worker build and runtime commands must be Cargo release builds and Rust release binaries.
  - Frontends call Rust workers for APIs, SQLite, KV, queues, objects, and server-side logic.
  - Create SQLite backups before migrations or destructive writes; restore from CLI/API without dashboard access, then verify with read queries.
  - State alarms schedule one wake-up per State object. Alarm handlers run in Rust workers, receive retry metadata, and retry up to six times with exponential backoff.
  - Use tovuk storage upload/list/download/delete for service files and media without dashboard access; upload automatically switches to multipart for large files; pass --public only when a public media URL is intended.
  - Use tovuk pricing --json and tovuk usage --json before heavy work, inspect billingEstimate.lineItems, then set usage caps for builds, worker, SQLite, KV, queue, State, and object storage meters before paid overages.
  - Run tovuk plan --json before deploy so agents can inspect explicit capabilities, missing config, meters, limits, billing estimates, and next actions.
  - Run deploy from a worker-static repo root with one tovuk.toml to build worker and frontend together.
  - Delete unused test services with tovuk service delete <service> --json after smoke tests.
  - Prefer same-origin worker-static services over split services.
  - When a plan limit blocks work, run tovuk billing checkout --json and show the returned URL to the human.
  - Create support tickets only with command output, service id, build id, deploy id, and the first actionable log line.
  - Resolve support tickets after the issue is fixed so later agents do not duplicate work.
  - Keep direct unsafe out of Rust source.
  - Keep Rust worker resources within Tovuk limits: 128mb memory, CPU allocation 1, metered worker_cpu_ms caps, and 1-60 minute idle timeout.
"#;

pub(crate) fn help_text() -> String {
    format!("Tovuk {VERSION}\n{HELP_BODY}")
}
