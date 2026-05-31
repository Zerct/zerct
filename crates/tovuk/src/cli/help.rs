use super::constants::VERSION;

pub(crate) fn help_text() -> String {
    format!(
        r#"Tovuk {VERSION}

Usage:
  tovuk init [path] [--template rust-worker|tanstack-static-frontend|worker-static-rust-tanstack]
  tovuk install [path] [--template rust-worker|tanstack-static-frontend|worker-static-rust-tanstack]
  tovuk doctor [path] [--json]
  tovuk preview [path] [--port <port>]
  tovuk login [--token <token>] [--api <url>]
  tovuk deploy [path] [--wait] [--wait-timeout <seconds>] [--api <url>] [--json]
  tovuk capabilities [--api <url>] [--json]
  tovuk pricing [--api <url>] [--json]
  tovuk me [--api <url>] [--json]
  tovuk usage [--api <url>] [--json]
  tovuk activity [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  tovuk service list [--api <url>] [--json]
  tovuk service show <service> [--api <url>] [--json]
  tovuk overview --service <service> [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  tovuk deploys [--service <service>] [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  tovuk builds [--service <service>] [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  tovuk logs --service <service> [--deploy <deploy_id>] [--build <build_id>] [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  tovuk status --service <service> [--api <url>] [--json]
  tovuk inspect --service <service> [--api <url>] [--json]
  tovuk platform --service <service> [--api <url>] [--json]
  tovuk database create --service <service> DB [--api <url>] [--json]
  tovuk database query --service <service> DB "select 1" [--params <json_array>] [--api <url>] [--json]
  tovuk database delete --service <service> DB [--api <url>] [--json]
  tovuk kv create --service <service> CACHE [--api <url>] [--json]
  tovuk kv namespace delete --service <service> CACHE [--api <url>] [--json]
  tovuk kv keys --service <service> CACHE [--api <url>] [--json]
  tovuk kv get --service <service> CACHE <key> [--api <url>] [--json]
  tovuk kv put --service <service> CACHE <key> <value> [--metadata <json>] [--ttl <seconds>] [--api <url>] [--json]
  tovuk kv delete --service <service> CACHE <key> [--api <url>] [--json]
  tovuk queue create --service <service> jobs [--max-retries <n>] [--retention-seconds <seconds>] [--api <url>] [--json]
  tovuk queue messages --service <service> jobs [--api <url>] [--json]
  tovuk queue send --service <service> jobs <body> [--delay-seconds <seconds>] [--api <url>] [--json]
  tovuk queue delete --service <service> jobs [--api <url>] [--json]
  tovuk cron create --service <service> nightly "0 0 * * *" [--api <url>] [--json]
  tovuk cron delete --service <service> nightly [--api <url>] [--json]
  tovuk durable-object create --service <service> Room [--api <url>] [--json]
  tovuk durable-object delete --service <service> Room [--api <url>] [--json]
  tovuk binding create --service <service> AUTH_SERVICE --target <target_service> [--api <url>] [--json]
  tovuk binding delete --service <service> AUTH_SERVICE [--api <url>] [--json]
  tovuk limit set worker_requests --period day --value 100000 [--api <url>] [--json]
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
  - Use tovuk storage upload/list/download/delete for service files and media without dashboard access; pass --public only when a CDN URL is intended.
  - Use tovuk pricing --json before heavy work, then set usage caps before paid overages.
  - Run deploy from a worker-static repo root with one tovuk.toml to build worker and frontend together.
  - Prefer same-origin worker-static services over split services.
  - When a plan limit blocks work, run tovuk billing checkout --json and show the returned URL to the human.
  - Create support tickets only with command output, service id, build id, deploy id, and the first actionable log line.
  - Resolve support tickets after the issue is fixed so later agents do not duplicate work.
  - Keep direct unsafe out of Rust source.
  - Keep Rust worker resources small: 128mb-2gb memory, 0.05-2 CPU, and 1-60 minute idle timeout.
"#
    )
}
