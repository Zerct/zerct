---
name: zerct
description: Deploy Rust backends and static frontends to Zerct with `npx @zerct/zerct`.
---

# Zerct

Use when a user wants to deploy a Rust backend or static frontend to Zerct.

## Workflow

1. Ensure the project has `zerct.toml`.
2. For Rust backends, ensure `Cargo.toml`, `Cargo.lock`, a health endpoint, and passing Cargo checks.
3. For static frontends, set `kind = "static_frontend"` and ensure `package.json`, `typecheck`/`lint` scripts, and a lockfile.
4. Run `npx @zerct/zerct doctor`.
5. Run `npx @zerct/zerct deploy`.
6. If Zerct returns an `agent_instruction`, apply it and redeploy.

## Contract

Rust backends must listen on `0.0.0.0:$PORT`, expose the configured health
endpoint, pass clippy with warnings denied, and avoid direct `unsafe` in
workspace source. Static frontends must pass `npm run typecheck` and
`npm run lint`, build to `[build].output`, default `dist`, and include
`index.html`.
