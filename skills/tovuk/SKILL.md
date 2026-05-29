---
name: tovuk
description: Deploy Rust backends and static frontends to Tovuk with `tovuk`.
---

# Tovuk

Use when a user wants to deploy a Rust backend or static frontend to Tovuk.

## Workflow

1. Ensure the project has `tovuk.toml`.
2. For Rust backends, ensure `Cargo.toml`, `Cargo.lock`, a health endpoint, `cargo fmt --all --check`, and the strict locked Cargo checks.
3. For static frontends, set `kind = "static_frontend"` and ensure `package.json`, TypeScript browser source, stable native type-aware typechecking, native linting, Fallow quality gates, a lockfile, and a strict frontend check command.
4. Prefer Bun with `bun.lock`, source-scoped Oxlint type-aware checks, and Fallow for new frontend projects. Avoid JavaScript-based lint, format, typecheck, dead-code, or duplicate-code tooling.
5. For a new full-stack project, run `tovuk init my-app --template fullstack-rust-tanstack`.
6. Run `tovuk doctor --json`.
7. Run `tovuk preview api` and `tovuk preview web` when local tools are available.
8. Run `tovuk deploy --wait --json`. From a repo root with nested `tovuk.toml` files, this deploys the whole workspace in one command.
9. If Tovuk returns an `agent_instruction`, apply it, rerun doctor, and redeploy.
10. If a build fails, run `tovuk logs --build <build_id> --json`, fix the first actionable log error, rerun doctor, and redeploy.

## Contract

Rust backends must listen on `0.0.0.0:$PORT`, expose the configured health
endpoint, pass `cargo fmt --all --check`, run locked `cargo check`, run locked
all-target/all-feature Clippy with `-D warnings`, and avoid direct `unsafe` in
workspace source. Static
frontends must use `.ts` or `.tsx` browser source under `src`, `app`, `pages`,
`routes`, or `components`; install dependencies, run stable native type-aware
TypeScript checks, run native linting plus Fallow dead-code, semantic
duplicate-code, and health gates, build to `[build].output`, default `dist`;
and include
`index.html`. Frontends call Rust
backends for APIs, managed Postgres, and server-side logic.
