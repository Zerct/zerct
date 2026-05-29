# tovuk

Python CLI package for deploying Rust backends, static frontends, and fullstack
apps to Tovuk.
It delegates to the npm Tovuk CLI so PyPI stays aligned with the primary
agent command surface.

```sh
pipx install tovuk
tovuk init my-app --template fullstack-rust-tanstack
cd my-app/web && bun install && cd ..
tovuk doctor --json
tovuk preview
tovuk deploy --wait --json
```

From a fullstack repo root, `tovuk deploy` reads one root `tovuk.toml`, builds
the backend and frontend roots, and returns one app URL with `/api/*` routed to
the Rust backend.

Rust backend deploys require `cargo fmt --all --check`, locked release-mode
`cargo check`, locked release-mode tests, and strict all-target, all-feature
Clippy with panic/unwrap bans plus resource-sensitive lints.

Static frontend deploys require TypeScript browser source, stable native
type-aware TypeScript checks, native linting such as `oxlint`, `biome check`,
or `deno lint`, and Fallow dead-code, semantic duplicate-code, and health
gates.

The npm package remains the primary first install path:

```sh
tovuk deploy
```

Python installs require Node.js 18+ with `npx` available at runtime.

The Python package exposes the same agent command surface as npm:

```sh
tovuk capabilities
tovuk me
tovuk usage
tovuk activity --json
tovuk apps
tovuk overview --app app_1 --json
tovuk deploys --app app_1
tovuk builds
tovuk logs --deploy deploy_1 --limit 100 --json
tovuk env list --app app_1
tovuk env set --app app_1 API_KEY=value
tovuk env delete --app app_1 API_KEY
tovuk domains add --app app_1 api.example.com
tovuk domains verify --app app_1 api.example.com
tovuk billing checkout --json
tovuk billing portal
tovuk support create "Deploy failed" "Agent retried deploy after doctor." --app app_1 --build job_1 --deploy deploy_1 --failing-command "tovuk deploy --wait --json" --first-log-line "cargo check failed in src/main.rs" --json
tovuk support list --json
tovuk support resolve ticket_0123456789abcdef0123 --json
```

Agent repair loop:

```sh
tovuk doctor --json
tovuk deploy --wait --json
tovuk logs --build job_1 --json
```

Fix the first failed `agent_instruction`. If a build fails, inspect build logs,
fix the first actionable log error, rerun doctor, then redeploy.

On first deploy, the CLI opens browser login, waits for GitHub or Google, stores
the Tovuk session in the OS credential store when available, and continues the
deploy. Later commands reuse that session.
