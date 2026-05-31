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
tovuk logs --build <build_id> --json
```

Manage platform resources without dashboard access:

```sh
tovuk platform --service <service> --json
tovuk database create --service <service> DB --json
tovuk kv create --service <service> CACHE --json
tovuk queue create --service <service> jobs --json
tovuk cron create --service <service> nightly "0 0 * * *" --json
tovuk durable-object create --service <service> Room --json
tovuk binding create --service <service> AUTH_SERVICE --target auth-service --json
tovuk limit set worker_requests --period day --value 100000 --json
```

Manage service files and media without dashboard access:

```sh
tovuk storage list --service <service> --json
tovuk storage upload --service <service> ./logo.png uploads/logo.png --public --json
tovuk storage download --service <service> uploads/logo.png ./logo.png --json
tovuk storage delete --service <service> uploads/logo.png --json
```

When a plan limit blocks work:

```sh
tovuk billing checkout "Plan limit reached" --json
```

When Tovuk support is needed, create a ticket only after collecting the failing
command, service id, build id, deploy id, first actionable log line, and what the
agent already tried. Resolve the ticket when the issue is fixed.
