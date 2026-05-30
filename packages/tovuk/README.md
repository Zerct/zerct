# tovuk

Deploy Rust workers, static frontends, and worker-static apps to Tovuk.

```sh
tovuk init my-app --template worker-static-rust-tanstack
cd my-app/web && bun install && cd ..
tovuk doctor --json
tovuk deploy --wait --json
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
`tovuk.toml`, builds the worker and frontend roots, and returns one app URL
with `/api/*` routed to the Rust worker.

Preview before deploying:

```sh
tovuk preview
```

Agent repair loop:

```sh
tovuk doctor --json
tovuk deploy --wait --json
tovuk logs --build job_1 --json
```

Fix the first failed `agent_instruction`. If a build fails, inspect build logs,
fix the first actionable log error, rerun doctor, then redeploy.

Agents can create app SQLite databases, KV namespaces, queues, cron triggers,
Durable Object namespaces, service bindings, and usage caps through the CLI.

Agents can also inspect API capabilities, account identity, usage, account
activity, apps, complete app overviews, deploys, builds, app/deploy/build logs,
env metadata, custom domains, domain verification, app storage files and media,
billing checkout links, billing portal links, and support ticket create, list,
and resolve actions
through the same CLI.

Manage app files and media without dashboard access:

```sh
tovuk storage list --app app_1 --json
tovuk storage upload --app app_1 ./logo.png uploads/logo.png --public --json
tovuk storage download --app app_1 uploads/logo.png ./logo.png --json
tovuk storage delete --app app_1 uploads/logo.png --json
```

When a free-tier limit blocks work, run:

```sh
tovuk billing checkout --json
```

When Tovuk support is needed, include enough evidence for a support agent:

```sh
tovuk support create "Deploy failed" "Agent retried deploy after doctor." --app app_1 --build job_1 --deploy deploy_1 --failing-command "tovuk deploy --wait --json" --first-log-line "cargo check failed in src/main.rs" --json
```

When the issue is fixed, resolve the ticket:

```sh
tovuk support list --json
tovuk support resolve ticket_0123456789abcdef0123 --json
```

On first deploy, the CLI opens browser login, waits for GitHub or Google, stores
the Tovuk session in the OS credential store when available, and continues the
deploy. Later commands reuse that session.
