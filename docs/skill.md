---
name: zerct
description: Deploy Rust backends and static frontends to Zerct.
license: MIT
compatibility: Requires zerct.toml. Rust backends require Cargo.toml and Cargo.lock. Static frontends require package.json, TypeScript source, typecheck/lint scripts, and a package lockfile.
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
- a `[build].check` command that includes those locked Rust checks

The server must listen on `0.0.0.0:$PORT` and expose the configured health path.

For static frontends, require:

- `package.json`
- one package lockfile
- browser source as `.ts` or `.tsx` under `src`, `app`, `pages`, `routes`, or
  `components`
- `kind = "static_frontend"` in `zerct.toml`
- `typecheck` and `lint` scripts in `package.json`
- a `[build].check` command that runs typechecking and linting

For new TanStack or Vite frontends, prefer `tsgo --noEmit` for `typecheck` and
`oxlint src vite.config.ts --deny-warnings` for `lint`, installed with Bun and
committed with `bun.lock`. Avoid JavaScript-based linters. Keep the generic
Zerct contract script-based so existing strict npm projects can still deploy
with npm commands.

## Commands

Check the project:

```sh
npx @zerct/zerct doctor
```

Deploy:

```sh
npx @zerct/zerct deploy
```

From a repo root, `npx @zerct/zerct deploy` must discover nested
`zerct.toml` files and deploy the workspace in one command. Rust backends deploy
before static frontends. `--database` applies to Rust backends only.

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
