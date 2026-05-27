# zerct

Deploy Rust backends and static frontends to Zerct.

```sh
npx @zerct/zerct init my-app --template fullstack-rust-tanstack
cd my-app/web && bun install && cd ..
npx @zerct/zerct doctor --json
npx @zerct/zerct deploy --wait --json
```

`npx @zerct/zerct` is the current public npm command. The future unscoped
`npx zerct` command is pending npm approval.

Rust backends expect `Cargo.toml`, `Cargo.lock`, and `zerct.toml`. They must
pass `cargo fmt --all --check`, locked Cargo checks, listen on
`0.0.0.0:$PORT`, and expose the configured health endpoint.

Static frontends must use TypeScript browser source, `tsgo --noEmit` for
typecheck, and native linting such as `oxlint`, `biome check`, or `deno lint`.

From a full-stack repo root, the same deploy command discovers nested
`zerct.toml` files and deploys the whole workspace in one command.

Preview before deploying:

```sh
npx @zerct/zerct preview
```

Agent repair loop:

```sh
npx @zerct/zerct doctor --json
npx @zerct/zerct deploy --wait --json
npx @zerct/zerct logs --build job_1 --json
```

Fix the first failed `agent_instruction`. If a build fails, inspect build logs,
fix the first actionable log error, rerun doctor, then redeploy.

Managed Postgres apps receive `DATABASE_URL`, `ZERCT_DATABASE_URL`, and
`ZERCT_DATABASE_CONNECTION_LIMIT`. Use that limit as the max size for your
database pool.

Agents can also inspect API capabilities, account identity, usage, account
activity, apps, complete app overviews, deploys, builds, app/deploy/build logs,
env metadata, custom domains, domain verification, and billing portal links
through the same CLI.

On first deploy, the CLI opens browser login, waits for GitHub or Google, stores
the Zerct session in the OS credential store when available, and continues the
deploy. Later commands reuse that session.
