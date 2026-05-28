---
name: zerct
description: Deploy Rust backends and static frontends to Zerct with `npx @zerct/zerct`.
---

# Zerct

Use when a user wants to deploy a Rust backend or static frontend to Zerct.

## Workflow

1. Ensure the project has `zerct.toml`.
2. For Rust backends, ensure `Cargo.toml`, `Cargo.lock`, a health endpoint, `cargo fmt --all --check`, and the strict locked Cargo checks.
3. For static frontends, set `kind = "static_frontend"` and ensure `package.json`, TypeScript browser source, `tsgo --noEmit` typecheck, native linting, Fallow quality gates, a lockfile, and a strict frontend check command.
4. Prefer Bun with `bun.lock`, `tsgo --noEmit`, source-scoped `oxlint`, and Fallow for new frontend projects. Avoid JavaScript-based lint, format, dead-code, or duplicate-code tooling.
5. For a new full-stack project, run `npx @zerct/zerct init my-app --template fullstack-rust-tanstack`.
6. Run `npx @zerct/zerct doctor --json`.
7. Run `npx @zerct/zerct preview api` and `npx @zerct/zerct preview web` when local tools are available.
8. Run `npx @zerct/zerct deploy --wait --json`. From a repo root with nested `zerct.toml` files, this deploys the whole workspace in one command.
9. If Zerct returns an `agent_instruction`, apply it, rerun doctor, and redeploy.
10. If a build fails, run `npx @zerct/zerct logs --build <build_id> --json`, fix the first actionable log error, rerun doctor, and redeploy.

## Contract

Rust backends must listen on `0.0.0.0:$PORT`, expose the configured health
endpoint, pass `cargo fmt --all --check`, run locked `cargo check`, run locked
all-target/all-feature Clippy with `-D warnings`, and avoid direct `unsafe` in
workspace source. Static
frontends must use `.ts` or `.tsx` browser source under `src`, `app`, `pages`,
`routes`, or `components`; install dependencies, run `tsgo --noEmit`, run
native linting plus Fallow dead-code, semantic duplicate-code, and health gates,
build to `[build].output`, default `dist`; and include
`index.html`. Frontends call Rust
backends for APIs, managed Postgres, and server-side logic.
