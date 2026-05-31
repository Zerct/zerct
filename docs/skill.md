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
npx tovuk doctor --json
npx tovuk init hello-service --template worker-static-rust-tanstack
npx tovuk preview
npx tovuk deploy --wait --json
npx tovuk pricing --json
npx tovuk logs --build <build_id> --json
```

Manage platform resources without dashboard access:

```sh
npx tovuk platform --service <service> --json
npx tovuk database create --service <service> DB --json
npx tovuk database query --service <service> DB "select 1" --json
npx tovuk database delete --service <service> DB --json
npx tovuk kv create --service <service> CACHE --json
npx tovuk kv namespace delete --service <service> CACHE --json
npx tovuk queue create --service <service> jobs --json
npx tovuk queue delete --service <service> jobs --json
npx tovuk cron create --service <service> nightly "0 0 * * *" --json
npx tovuk cron delete --service <service> nightly --json
npx tovuk durable-object create --service <service> Room --json
npx tovuk durable-object delete --service <service> Room --json
npx tovuk binding create --service <service> AUTH_SERVICE --target auth-service --json
npx tovuk binding delete --service <service> AUTH_SERVICE --json
npx tovuk limit set worker_requests --period day --value 100000 --json
npx tovuk limit delete worker_requests --period day --json
```

Before high-throughput work or paid usage, read pricing and set hard caps:

```sh
npx tovuk pricing --json
npx tovuk limit set worker_requests --period month --value 10000000 --json
npx tovuk limit set worker_cpu_ms --period month --value 30000000 --json
```

Manage service files and media without dashboard access:

```sh
npx tovuk storage list --service <service> --json
npx tovuk storage upload --service <service> ./logo.png uploads/logo.png --public --json
npx tovuk storage download --service <service> uploads/logo.png ./logo.png --json
npx tovuk storage delete --service <service> uploads/logo.png --json
```

When a plan limit blocks work:

```sh
npx tovuk billing checkout "Plan limit reached" --json
```

When Tovuk support is needed, create a ticket only after collecting the failing
command, service id, build id, deploy id, first actionable log line, and what the
agent already tried. Resolve the ticket when the issue is fixed.
