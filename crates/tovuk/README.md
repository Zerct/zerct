# tovuk

Rust CLI package for deploying Rust workers, static frontends, and worker-static
services to Tovuk.
This is the native source of truth for the Tovuk CLI. It does not require
Node.js, npm, `npx`, `tsx`, or Python at runtime.

```sh
cargo install tovuk
tovuk init hello-service --template worker-static-rust-tanstack
cd hello-service/web && bun install && cd ..
tovuk doctor --json
tovuk preview
tovuk deploy --wait --json
```

From a worker-static repo root, `tovuk deploy` reads one root `tovuk.toml`,
builds the worker and frontend roots, and returns one service URL with `/api/*`
routed to the Rust worker.

Static frontend deploys require TypeScript browser source, stable native
type-aware TypeScript checks, native linting such as `oxlint`, `biome check`,
or `deno lint`, and Fallow dead-code, semantic duplicate-code, and health
gates.

The npm package is also available:

```sh
npx tovuk deploy
```

The Cargo package exposes the same agent command surface as npm:

```sh
tovuk capabilities
tovuk me
tovuk usage
tovuk activity --json
tovuk service list
tovuk service show service_1 --json
tovuk deploys --service service_1
tovuk builds
tovuk logs --build job_1 --limit 100 --json
tovuk env list --service service_1
tovuk env set --service service_1 API_KEY=value
tovuk env delete --service service_1 API_KEY
tovuk domains add --service service_1 api.example.com
tovuk domains verify --service service_1 api.example.com
tovuk storage list --service service_1 --json
tovuk storage upload --service service_1 ./logo.png uploads/logo.png --public --json
tovuk storage download --service service_1 uploads/logo.png ./logo.png --json
tovuk storage delete --service service_1 uploads/logo.png --json
tovuk platform --service service_1 --json
tovuk database create --service service_1 DB --json
tovuk database query --service service_1 DB "select 1" --json
tovuk kv create --service service_1 CACHE --json
tovuk kv put --service service_1 CACHE user:1 '{"name":"Ada"}' --json
tovuk kv get --service service_1 CACHE user:1 --json
tovuk queue create --service service_1 jobs --json
tovuk queue send --service service_1 jobs '{"task":"sync"}' --json
tovuk cron create --service service_1 nightly "0 0 * * *" --json
tovuk durable-object create --service service_1 Room --json
tovuk binding create --service service_1 AUTH_SERVICE --target auth-service --json
tovuk limit set worker_requests --period day --value 100000 --json
tovuk billing checkout --json
tovuk billing portal
tovuk support create "Deploy failed" "Agent retried deploy after doctor." --service service_1 --build job_1 --deploy deploy_1 --failing-command "tovuk deploy --wait --json" --first-log-line "cargo check failed in src/main.rs" --json
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
