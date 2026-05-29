---
name: tovuk
description: Deploy Rust backends, static frontends, and fullstack apps to Tovuk.
license: MIT
compatibility: Requires tovuk.toml. Fullstack apps use one root tovuk.toml with backend and frontend roots. Rust backends require Cargo.toml and Cargo.lock. Package frontends require package.json, TypeScript source, typecheck/lint scripts, and a package lockfile. Plain static frontends require index.html.
metadata:
  author: Tovuk
  version: "0.1"
---

# Tovuk

Use this skill when a user wants to deploy a Rust backend or static frontend to
Tovuk, inspect a deployment, read logs, or prepare a project for deployment.

## Project contract

Always require:

- `tovuk.toml`

For Rust backends, require:

- `Cargo.toml`
- `Cargo.lock`
- passing `cargo fmt --all --check`
- passing locked release-mode `cargo check`, `cargo test`, and all-target,
  all-feature Clippy
- strict Clippy deny lints for `all`, `pedantic`, panic/unwrap bans, and
  resource-sensitive lints such as `large_futures`, `large_stack_frames`,
  `mem_forget`, and `redundant_clone`
- a `[build].check` command that includes formatting plus those locked release
  Rust checks

The server must listen on `0.0.0.0:$PORT` and expose the configured health path.

For static frontends, require:

- `package.json`
- one package lockfile
- browser source as `.ts` or `.tsx` under `src`, `app`, `pages`, `routes`, or
  `components`
- `kind = "static_frontend"` in `tovuk.toml`
- `typecheck` and `lint` scripts in `package.json`
- `typecheck` runs stable native type-aware TypeScript checks such as
  `oxlint --type-aware --type-check`
- `lint` runs native tooling such as `oxlint`, `biome check`, or `deno lint`
- `lint` runs Fallow `dead-code`, semantic `dupes`, and `health` gates
- a `[build].check` command that installs dependencies and runs typechecking
  plus linting

For plain static frontends, require:

- `index.html`
- `kind = "static_frontend"` in `tovuk.toml`
- `[build].check = ":"`
- `[build].command = ":"`
- `[build].output = "."`

For fullstack apps, require one root `tovuk.toml` with:

- `kind = "fullstack"`
- `[backend].root`, `[backend].check`, `[backend].build`, `[backend].command`,
  `[backend].port`, and `[backend].health = "/api/healthz"`
- `[frontend].root`, `[frontend].check`, `[frontend].build`, and
  `[frontend].output`
- a frontend that calls same-origin `/api`

For new TanStack or Vite frontends, prefer Oxlint type-aware type checking plus
Fallow for `lint`, installed with Bun and committed with `bun.lock`. Avoid
JavaScript-based lint, format, typecheck, dead-code, or duplicate-code tooling.
Keep the generic Tovuk contract
script-based so existing strict npm projects can still deploy with npm
commands.

Fullstack apps deploy to one URL. Tovuk serves the frontend at `/` and proxies
`/api/*` to the Rust backend. When a browser frontend calls a separate backend
deployment on another hostname, configure backend CORS for that frontend origin
or put both apps behind a same-origin custom domain.

## Commands

Check the project:

```sh
tovuk doctor --json
```

Create a fullstack starter:

```sh
tovuk init my-app --template fullstack-rust-tanstack
```

Preview:

```sh
tovuk preview
```

Deploy:

```sh
tovuk deploy --wait --json
```

For fullstack apps, `--database` applies to the Rust backend inside the same
deployment. For workspaces with multiple `tovuk.toml` files, `tovuk deploy`
still deploys all discovered projects in one command.

Manage app files and media without dashboard access:

```sh
tovuk storage list --app <app_id> --json
tovuk storage upload --app <app_id> ./logo.png uploads/logo.png --public --json
tovuk storage download --app <app_id> uploads/logo.png ./logo.png --json
tovuk storage delete --app <app_id> uploads/logo.png --json
```

Use `--public` only when the app needs a CDN URL for the object. Use
`--content-type <mime>` when the file extension is missing or ambiguous.

Use a persistent CLI:

```sh
brew tap tovuk/tovuk https://github.com/tovuk/tovuk
brew install tovuk
tovuk deploy
```

Create a Stripe Checkout URL when a plan limit blocks work:

```sh
tovuk billing checkout "Plan limit reached" --json
```

Create a support ticket after collecting command output and ids:

```sh
tovuk support create "Deploy failed" "Agent retried deploy after doctor." --app <app_id> --build <build_id> --deploy <deploy_id> --failing-command "tovuk deploy --wait --json" --first-log-line "first actionable log line" --json
```

Resolve a support ticket after the agent fixes the issue:

```sh
tovuk support resolve ticket_0123456789abcdef0123 --json
```

## Agent behavior

When a Tovuk command returns JSON with `agent_instruction`, follow that field
first, change the project, and retry the command. Do not guess around a failed
guardrail.

After a failed deploy, inspect build logs first:

```sh
tovuk logs --build <build_id> --json
```

Fix the first actionable log error, rerun doctor, then redeploy.

If the remaining blocker is a Tovuk platform issue, create a support ticket
with the failing command, app id, build id, deploy id, first actionable log
line, and what the agent already tried. Do not open duplicate tickets before
running `tovuk support list --json`. Resolve the ticket after the
issue is fixed.
