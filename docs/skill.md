---
name: tovuk
description: Deploy Rust workers, static frontends, and full-stack services to Tovuk.
license: MIT
compatibility: Requires tovuk.toml. Full-stack services use one root tovuk.toml with worker and frontend roots. Rust workers require Cargo.toml and Cargo.lock. Package frontends require package.json, TypeScript source, typecheck/lint scripts, and a package lockfile. Plain static frontends require index.html.
metadata:
  author: Tovuk
  version: "0.1"
---

# Tovuk

Use this skill when a user wants to deploy a Rust worker, static frontend, or
full-stack service to Tovuk, inspect a deployment, read logs, or prepare a
project for deployment.

## Project contract

Always require `tovuk.toml` with explicit `[capabilities]` booleans.

For Rust workers, require `Cargo.toml`, `Cargo.lock`, strict locked
release-mode Rust checks, a health endpoint, and a server that listens on
`0.0.0.0:$PORT`.

For package static frontends, require `kind = "static_frontend"`,
TypeScript browser source, native type-aware typechecking, native linting,
Fallow dead-code and duplicate-code gates, and a lockfile. Plain static
frontends can use `index.html`, `check = ":"`, `command = ":"`, and
`output = "."`.

For full-stack services, require one root `tovuk.toml` with:

- `kind = "fullstack"`
- `[capabilities].static_frontend = true` and `[capabilities].worker = true`
- `[worker].root`, `[worker].check`, `[worker].build`, `[worker].command`,
  `[worker].port`, and `[worker].health = "/api/healthz"`
- `[frontend].root`, `[frontend].check`, `[frontend].build`, and
  `[frontend].output`
- a frontend that calls same-origin `/api`

JavaScript and TypeScript are frontend-only on Tovuk. Move API routes, SSR
handlers, middleware, and server logic to Rust workers.

## Commands

```sh
tovuk new hello-service --template fullstack-rust-tanstack
cd hello-service/web
npm install
cd ..
tovuk check --json
tovuk account show --json
tovuk account update --handle <handle> --display-name <name> --json
tovuk dev --json
tovuk deploy --dry-run --build-artifact --json
tovuk deploy --wait --json
tovuk service show <service> --json
tovuk service status <service> --json
tovuk deploy list --json
tovuk deploy show <deploy_id> --json
tovuk deploy cancel <deploy_id> --json
tovuk pricing --json
tovuk logs --build <build_id> --json
```

Manage service resources without dashboard access:

```sh
tovuk service status <service> --json
tovuk service show <service> --json
tovuk sqlite create --service <service> DB --json
tovuk sqlite query --service <service> DB "select 1" --json
tovuk sqlite batch --service <service> DB '[{"sql":"create table users (id integer primary key)"},{"sql":"insert into users (id) values (?)","params":[1]}]' --json
tovuk sqlite delete --service <service> DB --json
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
tovuk state list --service <service> --json
tovuk state create --service <service> Room --json
tovuk state put --service <service> Room room-1 counter 1 --json
tovuk state get --service <service> Room room-1 counter --json
tovuk state alarm set --service <service> Room room-1 --delay-seconds 60 --json
tovuk state alarm get --service <service> Room room-1 --json
tovuk state alarm delete --service <service> Room room-1 --json
tovuk state delete --service <service> Room --json
tovuk binding create --service <service> AUTH_SERVICE --target auth-service --json
tovuk binding delete --service <service> AUTH_SERVICE --json
tovuk limits set build_minutes --period month --value 6000 --notify-at-percent 80 --json
tovuk limits set worker_requests --period day --value 100000 --notify-at-percent 80 --json
tovuk limits set state_requests --period month --value 1000000 --notify-at-percent 80 --json
tovuk limits set state_sqlite_rows_written --period month --value 50000000 --notify-at-percent 80 --json
tovuk limits delete worker_requests --period day --json
```

Before high-throughput work or paid usage, read pricing and set hard caps:

```sh
tovuk pricing --json
tovuk usage --json
tovuk limits set build_minutes --period month --value 6000 --notify-at-percent 80 --json
tovuk limits set worker_requests --period month --value 10000000 --notify-at-percent 80 --json
tovuk limits set worker_cpu_ms --period month --value 30000000 --notify-at-percent 80 --json
tovuk limits set state_requests --period month --value 1000000 --notify-at-percent 80 --json
tovuk limits set state_sqlite_rows_written --period month --value 50000000 --notify-at-percent 80 --json
```

Manage service files and media without dashboard access:

```sh
tovuk storage list --service <service> --json
tovuk storage upload --service <service> ./logo.png uploads/logo.png --public --json
tovuk storage download --service <service> uploads/logo.png ./logo.png --json
tovuk storage url --service <service> uploads/logo.png --json
tovuk storage delete --service <service> uploads/logo.png --json
tovuk abuse report https://demo.tovuk.app "Phishing page" "Credential collection form" --category phishing --reporter-email reporter@example.com --evidence "Screenshot URL and request id" --json
tovuk abuse list --json
tovuk abuse list --operator --json
tovuk abuse appeal abuse_0123456789abcdef0123 "Removed the reported file and rotated credentials." --evidence "deploy_1 remediation log" --json
tovuk abuse triage abuse_0123456789abcdef0123 "Reviewed reporter evidence and target service metadata." --json
tovuk abuse notify-owner abuse_0123456789abcdef0123 "Owner-visible report recorded with evidence summary." --json
tovuk abuse quarantine abuse_0123456789abcdef0123 "Confirmed malware object and preserved scanner evidence." --json
tovuk abuse resolve abuse_0123456789abcdef0123 "Reporter issue remediated and clean deploy verified." --json
tovuk abuse reject abuse_0123456789abcdef0123 "Evidence did not match the reported target." --json
tovuk abuse release abuse_0123456789abcdef0123 "Owner removed object and redeployed clean build." --json
```

`tovuk storage upload` automatically switches to multipart transfer for files
larger than 100 MiB.

Use `tovuk pricing --json` to inspect product choices, meters, limit
fields, and plan prices before selecting Worker, Static Frontend, SQLite,
Object Storage, State, KV, Queues, Cron, Service Bindings, Secrets, Custom
Domains, Logs, Builds, or Usage Caps.
Use `tovuk usage --json` to inspect `billingEstimate.lineItems` before load
tests or paid usage.

When a plan limit blocks work:

```sh
tovuk billing checkout "Plan limit reached" --json
tovuk billing portal
```

When Tovuk support is needed, create a ticket only after collecting the failing
command, service id, build id, deploy id, first actionable log line, and what the
agent already tried. Resolve the ticket when the issue is fixed.

Report abuse with a target URL, category, reporter email, and preserved
evidence. Service owners should run `tovuk abuse list --json`, preserve
remediation evidence, and appeal with `tovuk abuse appeal <report_id> --json`
when the report was fixed or is incorrect. Operator agents can triage, notify
owners where lawful, quarantine, resolve, reject, or release confirmed reports
with preserved evidence.
