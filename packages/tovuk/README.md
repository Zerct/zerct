# tovuk

Deploy Rust backends and static frontends to Tovuk.

```sh
tovuk init my-app --template fullstack-rust-tanstack
cd my-app/web && bun install && cd ..
tovuk doctor --json
tovuk deploy --wait --json
```

`tovuk` is the public npm command.

Rust backends expect `Cargo.toml`, `Cargo.lock`, and `tovuk.toml`. They must
pass `cargo fmt --all --check`, locked Cargo checks, listen on
`0.0.0.0:$PORT`, and expose the configured health endpoint.

Static frontends must use TypeScript browser source, stable native type-aware
TypeScript checks, native linting such as `oxlint`, `biome check`, or
`deno lint`, and Fallow dead-code, semantic duplicate-code, and health gates.

From a full-stack repo root, the same deploy command discovers nested
`tovuk.toml` files and deploys the whole workspace in one command.

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

Managed Postgres apps receive `DATABASE_URL`, `TOVUK_DATABASE_URL`, and
`TOVUK_DATABASE_CONNECTION_LIMIT`. Use that limit as the max size for your
database pool.

Agents can also inspect API capabilities, account identity, usage, account
activity, apps, complete app overviews, deploys, builds, app/deploy/build logs,
env metadata, custom domains, domain verification, billing checkout links,
billing portal links, and support ticket create, list, and resolve actions
through the same CLI.

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
tovuk support resolve ticket_0123456789abcdef0123 --json
```

On first deploy, the CLI opens browser login, waits for GitHub or Google, stores
the Tovuk session in the OS credential store when available, and continues the
deploy. Later commands reuse that session.
