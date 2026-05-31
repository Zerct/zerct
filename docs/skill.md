---
name: tovuk
description: Deploy Rust workers, static frontends, and worker-static services to Tovuk.
license: MIT
compatibility: Requires tovuk.toml. Worker-static services use one root tovuk.toml with worker and frontend roots. Rust workers require Cargo.toml and Cargo.lock. Package frontends require package.json, TypeScript source, typecheck/lint scripts, and a package lockfile. Plain static frontends require index.html.
metadata:
  author: Tovuk
  version: "0.1"
---

# Tovuk

Use this skill when a user wants to deploy a Rust worker or static frontend to
Tovuk, inspect a deployment, read logs, or prepare a project for deployment.

## Project contract

Always require `tovuk.toml`.

For Rust workers, require `Cargo.toml`, `Cargo.lock`, strict locked
release-mode Rust checks, a health endpoint, and a server that listens on
`0.0.0.0:$PORT`.

For static frontends, require `kind = "static_frontend"`, TypeScript browser
source, native type-aware typechecking, native linting, Fallow dead-code and
duplicate-code gates, and a lockfile. Plain static frontends can use
`index.html`, `check = ":"`, `command = ":"`, and `output = "."`.

For worker-static services, require one root `tovuk.toml` with:

- `kind = "worker_static"`
- `[worker].root`, `[worker].check`, `[worker].build`, `[worker].command`,
  `[worker].port`, and `[worker].health = "/api/healthz"`
- `[frontend].root`, `[frontend].check`, `[frontend].build`, and
  `[frontend].output`
- a frontend that calls same-origin `/api`

JavaScript and TypeScript are frontend-only on Tovuk. Move API routes, SSR
handlers, middleware, and server logic to Rust workers.

## Commands

```sh
tovuk doctor --json
tovuk init hello-service --template worker-static-rust-tanstack
tovuk preview
tovuk deploy --wait --json
tovuk pricing --json
tovuk logs --build <build_id> --json
```

Manage platform resources without dashboard access:

```sh
tovuk platform --service <service> --json
tovuk database create --service <service> DB --json
tovuk database query --service <service> DB "select 1" --json
tovuk database delete --service <service> DB --json
tovuk kv create --service <service> CACHE --json
tovuk kv bulk put --service <service> CACHE '[{"key":"feature:search","value":"enabled"}]' --json
tovuk kv bulk get --service <service> CACHE feature:search user:1 --json
tovuk kv bulk delete --service <service> CACHE feature:search old:key --json
tovuk kv namespace delete --service <service> CACHE --json
tovuk queue create --service <service> failed_jobs --json
tovuk queue create --service <service> jobs --max-batch-size 10 --max-batch-timeout-seconds 5 --dead-letter-queue failed_jobs --json
tovuk queue update --service <service> jobs --max-batch-size 25 --json
tovuk queue update --service <service> jobs --clear-dead-letter-queue --json
tovuk queue send --service <service> jobs '{"task":"sync"}' --json
tovuk queue send-batch --service <service> jobs '[{"body":{"task":"sync"}},{"body":{"task":"index"}}]' --json
tovuk queue metrics --service <service> jobs --json
tovuk queue delete --service <service> jobs --json
tovuk cron create --service <service> nightly "0 0 * * *" --json
tovuk cron update --service <service> nightly "*/15 * * * *" --json
tovuk cron disable --service <service> nightly --json
tovuk cron enable --service <service> nightly --json
tovuk cron delete --service <service> nightly --json
tovuk state create --service <service> Room --json
tovuk state put --service <service> Room room-1 counter 1 --json
tovuk state get --service <service> Room room-1 counter --json
tovuk state delete --service <service> Room --json
tovuk binding create --service <service> AUTH_SERVICE --target auth-service --json
tovuk binding delete --service <service> AUTH_SERVICE --json
tovuk limit set build_minutes --period month --value 6000 --json
tovuk limit set worker_requests --period day --value 100000 --json
tovuk limit set state_requests --period month --value 1000000 --json
tovuk limit set state_sqlite_rows_written --period month --value 50000000 --json
tovuk limit delete worker_requests --period day --json
```

Before high-throughput work or paid usage, read pricing and set hard caps:

```sh
tovuk pricing --json
tovuk limit set build_minutes --period month --value 6000 --json
tovuk limit set worker_requests --period month --value 10000000 --json
tovuk limit set worker_cpu_ms --period month --value 30000000 --json
tovuk limit set state_requests --period month --value 1000000 --json
tovuk limit set state_sqlite_rows_written --period month --value 50000000 --json
```

Manage service files and media without dashboard access:

```sh
tovuk storage list --service <service> --json
tovuk storage upload --service <service> ./logo.png uploads/logo.png --public --json
tovuk storage download --service <service> uploads/logo.png ./logo.png --json
tovuk storage url --service <service> uploads/logo.png --json
tovuk storage delete --service <service> uploads/logo.png --json
```

Use `tovuk capabilities --json` to inspect product choices, meters, limit
fields, and plan prices before selecting Worker, State, SQLite, KV, queues,
cron, service bindings, or object storage.

When a plan limit blocks work:

```sh
tovuk billing checkout "Plan limit reached" --json
```

When Tovuk support is needed, create a ticket only after collecting the failing
command, service id, build id, deploy id, first actionable log line, and what the
agent already tried. Resolve the ticket when the issue is fixed.
