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
  tovuk me [--api <url>] [--json]
  tovuk usage [--api <url>] [--json]
  tovuk activity [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  tovuk apps [--api <url>] [--json]
  tovuk overview --app <app> [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  tovuk deploys [--app <app>] [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  tovuk builds [--app <app>] [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  tovuk logs --app <app> [--deploy <deploy_id>] [--build <build_id>] [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  tovuk status --app <app> [--api <url>] [--json]
  tovuk inspect --app <app> [--api <url>] [--json]
  tovuk platform --app <app> [--api <url>] [--json]
  tovuk sqlite create --app <app> DB [--api <url>] [--json]
  tovuk kv create --app <app> CACHE [--api <url>] [--json]
  tovuk queue create --app <app> jobs [--api <url>] [--json]
  tovuk cron create --app <app> nightly "0 0 * * *" [--api <url>] [--json]
  tovuk durable create --app <app> Room [--api <url>] [--json]
  tovuk binding create --app <app> AUTH_SERVICE --target <target_app> [--api <url>] [--json]
  tovuk caps set worker_requests --period day --value 100000 [--api <url>] [--json]
  tovuk env list --app <app> [--api <url>] [--json]
  tovuk env set --app <app> KEY=value [--api <url>] [--json]
  tovuk env delete --app <app> KEY [--api <url>] [--json]
  tovuk domains list --app <app> [--api <url>] [--json]
  tovuk domains add --app <app> <domain> [--api <url>] [--json]
  tovuk domains verify --app <app> <domain> [--api <url>] [--json]
  tovuk domains delete --app <app> <domain> [--api <url>] [--json]
  tovuk storage list --app <app> [--api <url>] [--json]
  tovuk storage upload --app <app> <local_file> [storage_path] [--content-type <mime>] [--public] [--api <url>] [--json]
  tovuk storage download --app <app> <storage_path> [local_file] [--api <url>] [--json]
  tovuk storage delete --app <app> <storage_path> [--api <url>] [--json]
  tovuk storage url --app <app> <storage_path> [--api <url>] [--json]
  tovuk billing [checkout|portal] [reason] [--api <url>] [--json]
  tovuk support list [--limit <n>] [--api <url>] [--json]
  tovuk support create "Subject" "Details" [--app <app>] [--build <build_id>] [--deploy <deploy_id>] [--failing-command <command>] [--first-log-line <line>] [--severity low|normal|urgent] [--api <url>] [--json]
  tovuk support resolve <ticket_id> [--api <url>] [--json]

Agent contract:
  - Worker-static apps set kind = "worker_static", keep worker and frontend roots in one tovuk.toml, serve the frontend at /, and serve the Rust worker API under /api.
  - Rust workers keep Cargo.lock committed, pass rustfmt plus locked release-mode check/test/Clippy gates, listen on 0.0.0.0:$PORT, and return HTTP 200 from health.
  - Static frontends set kind = "static_frontend", keep TypeScript source, a package lockfile, stable native typecheck, native lint, and Fallow quality gates.
  - Plain static HTML/CSS/JS frontends may use kind = "static_frontend" with check = ":", command = ":", and output = ".".
  - JavaScript and TypeScript are frontend-only on Tovuk; worker build and runtime commands must be Cargo release builds and Rust release binaries.
  - Frontends call Rust workers for APIs, SQLite, KV, queues, objects, and server-side logic.
  - Use tovuk storage upload/list/download/delete for app files and media without dashboard access; pass --public only when a CDN URL is intended.
  - Run deploy from a worker-static repo root with one tovuk.toml to build worker and frontend together.
  - Prefer same-origin worker-static apps over split apps.
  - When a plan limit blocks work, run tovuk billing checkout --json and show the returned URL to the human.
  - Create support tickets only with command output, app id, build id, deploy id, and the first actionable log line.
  - Resolve support tickets after the issue is fixed so later agents do not duplicate work.
  - Keep direct unsafe out of Rust source.
  - Keep Rust worker resources small: 128mb-2gb memory, 0.05-2 CPU, and 1-60 minute idle timeout.
"#
    )
}
