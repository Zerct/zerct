---
name: zerct
description: Deploy Rust backends and static frontends to Zerct with `npx @zerct/zerct`.
---

# Zerct

Use when a user wants to deploy a Rust backend or static frontend to Zerct.

## Workflow

1. Ensure the project has `zerct.toml`.
2. For Rust backends, ensure `Cargo.toml`, `Cargo.lock`, a health endpoint, and the strict locked Cargo checks.
3. For static frontends, set `kind = "static_frontend"` and ensure `package.json`, TypeScript browser source, `typecheck`/`lint` scripts, a lockfile, and a strict frontend check command. Prefer `tsgo --noEmit` plus source-scoped `oxlint` for new TanStack or Vite projects. Avoid JavaScript-based linters unless no native equivalent exists for a required rule.
4. Run `npx @zerct/zerct doctor`.
5. Run `npx @zerct/zerct deploy`.
6. If Zerct returns an `agent_instruction`, apply it and redeploy.

## Contract

Rust backends must listen on `0.0.0.0:$PORT`, expose the configured health
endpoint, run locked `cargo check`, run locked all-target/all-feature Clippy
with `-D warnings`, and avoid direct `unsafe` in workspace source. Static
frontends must use `.ts` or `.tsx` browser source under `src`, `app`, `pages`,
`routes`, or `components`; run typechecking and linting; build to
`[build].output`, default `dist`; and include `index.html`.
