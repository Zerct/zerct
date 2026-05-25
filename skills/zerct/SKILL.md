---
name: zerct
description: Deploy Rust backends to Zerct with `npx @zerct/zerct`.
---

# Zerct

Use when a user wants to deploy a Rust backend to Zerct.

## Workflow

1. Ensure the project has `Cargo.toml`, `Cargo.lock`, and `zerct.toml`.
2. Run `npx @zerct/zerct doctor`.
3. Run `npx @zerct/zerct deploy`.
4. If Zerct returns an `agent_instruction`, apply it and redeploy.

## Contract

The app must listen on `0.0.0.0:$PORT`, expose its configured health endpoint,
and avoid direct `unsafe` in workspace source.
