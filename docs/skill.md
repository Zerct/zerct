---
name: zerct
description: Deploy Rust backends and static frontends to Zerct.
license: MIT
compatibility: Requires zerct.toml. Rust backends require Cargo.toml and Cargo.lock. Static frontends require package.json, typecheck/lint scripts, and a package lockfile.
metadata:
  author: Zerct
  version: "0.1"
---

# Zerct

Use this skill when a user wants to deploy a Rust backend or static frontend to
Zerct, inspect a deployment, read logs, or prepare a project for deployment.

## Project contract

Always require:

- `zerct.toml`

For Rust backends, require:

- `Cargo.toml`
- `Cargo.lock`
- passing `cargo check --locked`
- passing `cargo clippy --locked --all-targets --all-features -- -D warnings`

The server must listen on `0.0.0.0:$PORT` and expose the configured health path.

For static frontends, require:

- `package.json`
- one package lockfile
- `kind = "static_frontend"` in `zerct.toml`
- `typecheck` and `lint` scripts in `package.json`

## Commands

Check the project:

```sh
npx @zerct/zerct doctor
```

Deploy:

```sh
npx @zerct/zerct deploy
```

Use a persistent CLI:

```sh
brew tap Zerct/tap
brew install zerct
zerct deploy
```

## Agent behavior

When a Zerct command returns JSON with `agent_instruction`, follow that field
first, change the project, and retry the command. Do not guess around a failed
guardrail.
