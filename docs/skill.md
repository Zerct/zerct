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
- passing `cargo fmt --all --check`
- passing `cargo check --locked`
- passing `cargo clippy --locked --all-targets --all-features -- -D warnings`
- a `[build].check` command that includes formatting plus those locked Rust checks

The server must listen on `0.0.0.0:$PORT` and expose the configured health path.

For static frontends, require:

- `package.json`
- one package lockfile
- browser source as `.ts` or `.tsx` under `src`, `app`, `pages`, `routes`, or
  `components`
- `kind = "static_frontend"` in `zerct.toml`
- `typecheck` and `lint` scripts in `package.json`
- `typecheck` runs `tsgo --noEmit` with `@typescript/native-preview`
- `lint` runs native tooling such as `oxlint`, `biome check`, or `deno lint`
- a `[build].check` command that installs dependencies and runs typechecking
  plus linting

For new TanStack or Vite frontends, prefer `tsgo --noEmit` for `typecheck` and
`oxlint src vite.config.ts --deny-warnings` for `lint`, installed with Bun and
committed with `bun.lock`. Avoid JavaScript-based lint or format tooling. Keep the generic
Zerct contract script-based so existing strict npm projects can still deploy
with npm commands.

When a browser frontend calls a Rust backend on another hostname, configure
backend CORS for that frontend origin or put both apps behind a same-origin
custom domain. Zerct keeps frontend and backend deployments as separate HTTPS
origins by default.

## Commands

Check the project:

```sh
npx @zerct/zerct doctor --json
```

Create a full-stack starter:

```sh
npx @zerct/zerct init my-app --template fullstack-rust-tanstack
```

Preview one project:

```sh
npx @zerct/zerct preview api
npx @zerct/zerct preview web
```

Deploy:

```sh
npx @zerct/zerct deploy --wait --json
```

From a repo root, `npx @zerct/zerct deploy` must discover nested
`zerct.toml` files and deploy the workspace in one command. Rust backends deploy
before static frontends. `--database` applies to Rust backends only.

Use a persistent CLI:

```sh
brew tap Zerct/zerct https://github.com/Zerct/zerct
brew install zerct
zerct deploy
```

Create a Stripe Checkout URL when a plan limit blocks work:

```sh
npx @zerct/zerct billing checkout "Plan limit reached" --json
```

Create a support ticket after collecting command output and ids:

```sh
npx @zerct/zerct support create "Deploy failed" "Command, app id, build id, deploy id, and first actionable log line." --json
```

Resolve a support ticket after the agent fixes the issue:

```sh
npx @zerct/zerct support resolve ticket_0123456789abcdef0123 --json
```

## Agent behavior

When a Zerct command returns JSON with `agent_instruction`, follow that field
first, change the project, and retry the command. Do not guess around a failed
guardrail.

After a failed deploy, inspect build logs first:

```sh
npx @zerct/zerct logs --build <build_id> --json
```

Fix the first actionable log error, rerun doctor, then redeploy.

If the remaining blocker is a Zerct platform issue, create a support ticket
with the failing command, app id, build id, deploy id, first actionable log
line, and what the agent already tried. Do not open duplicate tickets before
running `npx @zerct/zerct support list --json`. Resolve the ticket after the
issue is fixed.
