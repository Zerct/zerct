use super::VERSION;

pub(crate) fn help_text() -> String {
    format!(
        r#"Tovuk {VERSION}

Usage:
  tovuk init [path] [--template rust-api|tanstack-static-frontend|fullstack-rust-tanstack]
  tovuk install [path] [--template rust-api|tanstack-static-frontend|fullstack-rust-tanstack]
  tovuk doctor [path] [--json]
  tovuk preview [path] [--port <port>]
  tovuk login [--token <token>] [--api <url>]
  tovuk deploy [path] [--database] [--wait] [--wait-timeout <seconds>] [--api <url>] [--json]
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
  tovuk db --app <app> [--api <url>] [--json]
  tovuk env list --app <app> [--api <url>] [--json]
  tovuk env set --app <app> KEY=value [--api <url>] [--json]
  tovuk env delete --app <app> KEY [--api <url>] [--json]
  tovuk domains list --app <app> [--api <url>] [--json]
  tovuk domains add --app <app> <domain> [--api <url>] [--json]
  tovuk domains verify --app <app> <domain> [--api <url>] [--json]
  tovuk domains delete --app <app> <domain> [--api <url>] [--json]
  tovuk billing [checkout|portal] [reason] [--api <url>] [--json]
  tovuk support list [--limit <n>] [--api <url>] [--json]
  tovuk support create "Subject" "Details" [--app <app>] [--build <build_id>] [--deploy <deploy_id>] [--failing-command <command>] [--first-log-line <line>] [--severity low|normal|urgent] [--api <url>] [--json]
  tovuk support resolve <ticket_id> [--api <url>] [--json]

Agent contract:
  - Fullstack apps set kind = "fullstack", keep backend and frontend roots in one tovuk.toml, serve the frontend at /, and serve the Rust API under /api.
  - Rust backends keep Cargo.lock committed, pass rustfmt plus locked release-mode check/test/Clippy gates, listen on 0.0.0.0:$PORT, and return HTTP 200 from health.
  - Static frontends set kind = "static_frontend", keep TypeScript source, a package lockfile, stable native typecheck, native lint, and Fallow quality gates.
  - Plain static HTML/CSS/JS frontends may use kind = "static_frontend" with check = ":", command = ":", and output = ".".
  - JavaScript and TypeScript are frontend-only on Tovuk; backend build and runtime commands must be Cargo release builds and Rust release binaries.
  - Frontends call Rust backends for APIs, managed Postgres, and server-side logic.
  - Run deploy from a fullstack repo root with one tovuk.toml to build backend and frontend together.
  - When split frontend and backend apps use different hostnames, configure backend CORS or use a same-origin custom domain.
  - When a plan limit blocks work, run tovuk billing checkout --json and show the returned URL to the human.
  - Create support tickets only with command output, app id, build id, deploy id, and the first actionable log line.
  - Resolve support tickets after the issue is fixed so later agents do not duplicate work.
  - Keep direct unsafe out of Rust source.
  - Keep Rust backend resources small: 128mb-2gb memory, 0.05-2 CPU, and 1-60 minute idle timeout.
"#
    )
}
