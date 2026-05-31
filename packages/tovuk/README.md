# tovuk

Deploy Rust workers, static frontends, and worker-static services to Tovuk.

```sh
npx tovuk init hello-service --template worker-static-rust-tanstack
cd hello-service/web && bun install && cd ..
npx tovuk doctor --json
npx tovuk deploy --wait --json
```

The npm package installs the native Tovuk binary for the current platform.
Node is required by npm to install the package, but the `tovuk` command itself
does not delegate to `npx`, `tsx`, or any JavaScript runtime.

Rust workers expect `Cargo.toml`, `Cargo.lock`, and `tovuk.toml`. They must
pass `cargo fmt --all --check`, locked release-mode check/test/Clippy gates,
listen on `0.0.0.0:$PORT`, and expose the configured health endpoint.

Static frontends must use TypeScript browser source, stable native type-aware
TypeScript checks, native linting such as `oxlint`, `biome check`, or
`deno lint`, and Fallow dead-code, semantic duplicate-code, and health gates.

From a worker-static repo root, the same deploy command reads one root
`tovuk.toml`, builds the worker and frontend roots, and returns one service URL
with `/api/*` routed to the Rust worker.

Preview before deploying:

```sh
npx tovuk preview
```

Agent repair loop:

```sh
npx tovuk doctor --json
npx tovuk deploy --wait --json
npx tovuk logs --build job_1 --json
```

Fix the first failed `agent_instruction`. If a build fails, inspect build logs,
fix the first actionable log error, rerun doctor, then redeploy.

Agents can create service SQLite databases, KV namespaces, queues, cron triggers,
Durable Object namespaces, service bindings, and usage caps through the CLI.

Agents can also inspect API capabilities, account identity, usage, account
activity, services, complete service overviews, deploys, builds, service/deploy/build logs,
env metadata, custom domains, domain verification, service storage files and media,
pricing, billing checkout links, billing portal links, and support ticket create,
list, and resolve actions through the same CLI.

Before high-throughput work, read pricing and set hard caps:

```sh
npx tovuk pricing --json
npx tovuk limit set worker_requests --period month --value 10000000 --json
```

Manage service files and media without dashboard access:

```sh
npx tovuk storage list --service service_1 --json
npx tovuk storage upload --service service_1 ./logo.png uploads/logo.png --public --json
npx tovuk storage download --service service_1 uploads/logo.png ./logo.png --json
npx tovuk storage delete --service service_1 uploads/logo.png --json
npx tovuk kv put --service service_1 CACHE user:1 '{"name":"Ada"}' --json
npx tovuk kv get --service service_1 CACHE user:1 --json
npx tovuk kv bulk put --service service_1 CACHE '[{"key":"feature:search","value":"enabled"}]' --json
npx tovuk kv bulk get --service service_1 CACHE feature:search user:1 --json
npx tovuk kv bulk delete --service service_1 CACHE feature:search old:key --json
npx tovuk queue send --service service_1 jobs '{"task":"sync"}' --json
npx tovuk queue send-batch --service service_1 jobs '[{"body":{"task":"sync"}},{"body":{"task":"index"}}]' --json
npx tovuk queue metrics --service service_1 jobs --json
```

When a free-tier limit blocks work, run:

```sh
npx tovuk billing checkout --json
```

When Tovuk support is needed, include enough evidence for a support agent:

```sh
npx tovuk support create "Deploy failed" "Agent retried deploy after doctor." --service service_1 --build job_1 --deploy deploy_1 --failing-command "tovuk deploy --wait --json" --first-log-line "cargo check failed in src/main.rs" --json
```

When the issue is fixed, resolve the ticket:

```sh
npx tovuk support list --json
npx tovuk support resolve ticket_0123456789abcdef0123 --json
```

On first deploy, the CLI opens browser login, waits for GitHub or Google, stores
the Tovuk session in the OS credential store when available, and continues the
deploy. Later commands reuse that session.
