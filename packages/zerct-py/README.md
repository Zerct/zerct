# zerct

Python CLI package for deploying Rust backends and static frontends to Zerct.
It delegates to the npm Zerct CLI so PyPI stays aligned with the primary
agent command surface.

```sh
pipx install zerct
zerct init my-app --template fullstack-rust-tanstack
cd my-app/web && bun install && cd ..
zerct doctor --json
zerct preview
zerct deploy --wait --json
```

From a full-stack repo root, `zerct deploy` discovers nested `zerct.toml` files
and deploys the whole workspace in one command.

Rust backend deploys require `cargo fmt --all --check`, locked `cargo check`,
and locked all-target, all-feature Clippy with `-D warnings`.

Static frontend deploys require TypeScript browser source, `tsgo --noEmit` for
typecheck, and native linting such as `oxlint`, `biome check`, or `deno lint`.

The npm package remains the primary first install path:

```sh
npx @zerct/zerct deploy
```

Python installs require Node.js 18+ with `npx` available at runtime.

The Python package exposes the same agent command surface as npm:

```sh
zerct capabilities
zerct me
zerct usage
zerct activity --json
zerct apps
zerct overview --app app_1 --json
zerct deploys --app app_1
zerct builds
zerct logs --deploy deploy_1 --limit 100 --json
zerct env list --app app_1
zerct env set --app app_1 API_KEY=value
zerct env delete --app app_1 API_KEY
zerct domains add --app app_1 api.example.com
zerct domains verify --app app_1 api.example.com
zerct billing portal
```

Agent repair loop:

```sh
zerct doctor --json
zerct deploy --wait --json
zerct logs --build job_1 --json
```

Fix the first failed `agent_instruction`. If a build fails, inspect build logs,
fix the first actionable log error, rerun doctor, then redeploy.

On first deploy, the CLI opens browser login, waits for GitHub or Google, stores
the Zerct session in the OS credential store when available, and continues the
deploy. Later commands reuse that session.
