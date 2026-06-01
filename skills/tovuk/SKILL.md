---
name: tovuk
description: Deploy Rust workers, static frontends, and worker-static apps to Tovuk with `tovuk`.
---

# Tovuk

Use when a user wants to deploy a Rust worker, static frontend, or
worker-static app to Tovuk.

## Workflow

1. Ensure the project has `tovuk.toml`.
2. For Rust workers, ensure `Cargo.toml`, `Cargo.lock`, a health endpoint,
   `cargo fmt --all --check`, locked release-mode check/test/Clippy gates,
   strict Clippy resource lints, and small declared runtime resources.
3. For static frontends, set `kind = "static_frontend"` and ensure
   `package.json`, TypeScript browser source, stable native type-aware
   typechecking, native linting, Fallow quality gates, a lockfile, and a strict
   frontend check command.
4. For plain static frontends without a package manager, set
   `kind = "static_frontend"`, require `index.html`, use
   `[build].check = ":"`, `[build].command = ":"`, and `[build].output = "."`.
5. For worker-static apps, set `kind = "worker_static"` in one root
   `tovuk.toml`, configure `[worker].root` and `[frontend].root`, serve the
   frontend at `/`, and route API calls through same-origin `/api`.
6. Prefer Bun with `bun.lock`, source-scoped Oxlint type-aware checks, and
   Fallow for new package frontends. Avoid JavaScript-based lint, format,
   typecheck, dead-code, or duplicate-code tooling.
7. For a new worker-static project, run
   `tovuk new my-app --template worker-static-rust-tanstack`.
8. Run `tovuk check --json`.
9. Run `tovuk check` when local tools are available.
10. Run `tovuk deploy --wait --json`.
11. If Tovuk returns an `agent_instruction`, apply it, rerun check, and
    redeploy.
12. If a build fails, run `tovuk logs --build <build_id> --json`, fix the
    first actionable log error, rerun check, and redeploy.

## Platform resources

Agents can manage runtime resources without dashboard access:

```sh
tovuk service resources <service> --json
tovuk database create --service <service> DB --json
tovuk kv create --service <service> CACHE --json
tovuk kv bulk put --service <service> CACHE '[{"key":"feature:search","value":"enabled"}]' --json
tovuk kv bulk get --service <service> CACHE feature:search user:1 --json
tovuk queue create --service <service> jobs --json
tovuk queue send --service <service> jobs '{"task":"sync"}' --json
tovuk queue send-batch --service <service> jobs '[{"body":{"task":"sync"}},{"body":{"task":"index"}}]' --json
tovuk queue metrics --service <service> jobs --json
tovuk cron create --service <service> nightly "0 0 * * *" --json
tovuk cron update --service <service> nightly "*/15 * * * *" --json
tovuk cron disable --service <service> nightly --json
tovuk durable-object create --service <service> Room --json
tovuk binding create --service <service> AUTH_SERVICE --target auth-service --json
tovuk caps set worker_requests --period day --value 100000 --json
```

## Contract

Rust workers must listen on `0.0.0.0:$PORT`, expose the configured health
endpoint, pass `cargo fmt --all --check`, run locked release-mode `cargo check`
and `cargo test`, run strict all-target/all-feature Clippy with panic/unwrap
bans and resource-sensitive lints, and avoid direct `unsafe` in workspace
source.

Static frontends must use `.ts` or `.tsx` browser source under `src`, `app`,
`pages`, `routes`, or `components`; install dependencies, run stable native
type-aware TypeScript checks, run native linting plus Fallow dead-code,
semantic duplicate-code, and health gates; build to `[build].output`; and
include `index.html`.

Worker-static frontends call same-origin `/api` for APIs and server-side logic.
JavaScript and TypeScript are frontend-only on Tovuk.
