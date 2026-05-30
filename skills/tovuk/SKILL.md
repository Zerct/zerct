---
name: tovuk
description: Deploy Rust workers, static frontends, and worker-static apps to Tovuk with `npx tovuk`.
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
   `npx tovuk init my-app --template worker-static-rust-tanstack`.
8. Run `npx tovuk doctor --json`.
9. Run `npx tovuk preview` when local tools are available.
10. Run `npx tovuk deploy --wait --json`.
11. If Tovuk returns an `agent_instruction`, apply it, rerun doctor, and
    redeploy.
12. If a build fails, run `npx tovuk logs --build <build_id> --json`, fix the
    first actionable log error, rerun doctor, and redeploy.

## Platform resources

Agents can manage runtime resources without dashboard access:

```sh
npx tovuk platform --app <app_id> --json
npx tovuk sqlite create --app <app_id> DB --json
npx tovuk kv create --app <app_id> CACHE --json
npx tovuk queue create --app <app_id> jobs --json
npx tovuk cron create --app <app_id> nightly "0 0 * * *" --json
npx tovuk durable create --app <app_id> Room --json
npx tovuk binding create --app <app_id> AUTH_SERVICE --target auth-app --json
npx tovuk caps set worker_requests --period day --value 100000 --json
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
