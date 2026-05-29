---
name: tovuk
description: Deploy Rust backends and static frontends to Tovuk with `npx tovuk`.
---

# Tovuk

Use when a user wants to deploy a Rust backend or static frontend to Tovuk.

## Workflow

1. Ensure the project has `tovuk.toml`.
2. For Rust backends, ensure `Cargo.toml`, `Cargo.lock`, a health endpoint, `cargo fmt --all --check`, and the strict locked Cargo checks.
3. For static frontends, set `kind = "static_frontend"` and ensure `package.json`, TypeScript browser source, stable native type-aware typechecking, native linting, Fallow quality gates, a lockfile, and a strict frontend check command.
4. For plain static frontends without a package manager, set `kind = "static_frontend"`, require `index.html`, use `[build].check = ":"`, `[build].command = ":"`, and `[build].output = "."`.
5. For fullstack apps, set `kind = "fullstack"` in one root `tovuk.toml`, configure `[backend].root` and `[frontend].root`, serve the frontend at `/`, and route API calls through same-origin `/api`.
6. Prefer Bun with `bun.lock`, source-scoped Oxlint type-aware checks, and Fallow for new package frontends. Avoid JavaScript-based lint, format, typecheck, dead-code, or duplicate-code tooling.
7. For a new fullstack project, run `npx tovuk init my-app --template fullstack-rust-tanstack`.
8. Run `npx tovuk doctor --json`.
9. Run `npx tovuk preview` when local tools are available.
10. Run `npx tovuk deploy --wait --json`.
11. If Tovuk returns an `agent_instruction`, apply it, rerun doctor, and redeploy.
12. If a build fails, run `npx tovuk logs --build <build_id> --json`, fix the first actionable log error, rerun doctor, and redeploy.

## Contract

Rust backends must listen on `0.0.0.0:$PORT`, expose the configured health
endpoint, pass `cargo fmt --all --check`, run locked `cargo check`, run locked
all-target/all-feature Clippy with `-D warnings`, and avoid direct `unsafe` in
workspace source. Static
frontends must use `.ts` or `.tsx` browser source under `src`, `app`, `pages`,
`routes`, or `components`; install dependencies, run stable native type-aware
TypeScript checks, run native linting plus Fallow dead-code, semantic
duplicate-code, and health gates, build to `[build].output`, default `dist`;
and include `index.html`. Fullstack frontends call same-origin `/api` for
APIs, managed Postgres, and server-side logic.
