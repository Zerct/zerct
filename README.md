# Tovuk

Public Tovuk workspace for packages, agent skills, examples, and docs.

Tovuk hosts Rust workers, static frontends, and worker-static services. A
worker-static service uses one `tovuk.toml`, one deployment URL, static files at
`/`, and a Rust worker under `/api/*`.

## Install

Install the native CLI once, then run `tovuk` directly:

```sh
npm install -g tovuk
```

Other supported installers:

```sh
brew tap tovuk/tovuk https://github.com/tovuk/tovuk
brew install tovuk
pipx install tovuk
cargo install tovuk
```

Agent commands should use the native binary:

```sh
tovuk init
tovuk doctor
tovuk preview
tovuk deploy
```

Create a worker-static starter:

```sh
tovuk init hello-service --template worker-static-rust-tanstack
cd hello-service/web && bun install && cd ..
```

From a worker-static repo root, `tovuk deploy` reads the single root
`tovuk.toml`, builds `api` and `web`, and returns one service URL. Create
databases, KV namespaces, queues, cron triggers, State namespaces,
service bindings, object storage objects, and usage caps through CLI resource
commands. Service binding call chains can use up to 32 worker invocations per
top-level request.

Worker-static deploys use this `tovuk.toml` shape:

```toml
name = "hello-service"
kind = "worker_static"

[worker]
root = "api"
check = "cargo fmt --all --check && cargo check --locked --release --all-targets --all-features && cargo test --locked --release --all-targets --all-features && cargo clippy --locked --release --all-targets --all-features -- -D warnings -D clippy::all -D clippy::pedantic -D clippy::dbg_macro -D clippy::todo -D clippy::unimplemented -D clippy::panic -D clippy::unwrap_used -D clippy::expect_used -D clippy::large_futures -D clippy::large_include_file -D clippy::large_stack_frames -D clippy::mem_forget -D clippy::rc_buffer -D clippy::rc_mutex -D clippy::redundant_clone -D clippy::clone_on_ref_ptr"
build = "cargo build --release"
command = "./target/release/api"
port = 3000
health = "/api/healthz"

[frontend]
root = "web"
check = "bun ci && bun run typecheck && bun run lint"
build = "bun run build"
output = "dist"
```

Static frontend deploys use the same command with this `tovuk.toml`:

```toml
name = "dashboard"
kind = "static_frontend"

[build]
check = "bun ci && bun run typecheck && bun run lint"
command = "bun run build"
output = "dist"
```

For new TanStack or Vite frontends, prefer fast native checks and avoid
JavaScript-based lint, format, dead-code, or duplicate-code tooling:

```sh
bun add -d oxlint oxlint-tsgolint fallow
```

```json
{
  "scripts": {
    "typecheck": "oxlint src vite.config.ts --deny-warnings --type-aware --type-check --tsconfig tsconfig.json",
    "lint": "oxlint src vite.config.ts --deny-warnings && fallow dead-code --production --include-dupes --include-entry-exports --fail-on-issues && fallow dupes --production --mode semantic --threshold 1 --ignore-imports --fail-on-issues && fallow health --production --max-cyclomatic 10 --max-cognitive 15 --max-crap 20 --complexity",
    "build": "vite build"
  }
}
```

Rust worker checks must include `cargo fmt --all --check`, locked release-mode
`cargo check`, locked release-mode tests, and strict all-target, all-feature
Clippy with panic/unwrap bans plus resource-sensitive lints.
Frontend checks must install dependencies, run stable native type-aware
TypeScript checks, and run native linting plus Fallow dead-code, semantic
duplicate-code, and health gates before build work is queued. Frontend browser
source must be `.ts` or `.tsx` under
`src`, `app`, `pages`, `routes`, or `components`; browser `.js`, `.jsx`,
`.mjs`, and `.cjs` source is rejected. Bun projects should commit `bun.lock`
for the fastest Tovuk build path. Existing npm projects can still deploy with a
committed npm lockfile and npm-based build commands.

Plain static HTML/CSS/JS sites can deploy without a package manager by setting
`kind = "static_frontend"`, `[build].check = ":"`, `[build].command = ":"`, and
`[build].output = "."`.

Use Homebrew for a persistent developer CLI:

```sh
brew tap tovuk/tovuk https://github.com/tovuk/tovuk
brew install tovuk
tovuk deploy
```

- npm: `tovuk`
- PyPI: `tovuk`
- crates.io: `tovuk`
- Homebrew: `tovuk/tovuk/tovuk`

Agent prompt:

```txt
Use Tovuk to deploy this project. Read https://docs.tovuk.com/llms.txt first.
Run `tovuk doctor --json`. Fix the first failed check by following
`agent_instruction`, then rerun doctor. Deploy with
`tovuk deploy --wait --json`. If the build fails, read
`tovuk logs --build <build_id> --json`, fix the first actionable
error, rerun doctor, and redeploy. If a plan limit blocks work, run
`tovuk billing checkout --json` and show the returned URL to the
human. If Tovuk support is needed, run `tovuk support create` with
`--failing-command`, `--service`, `--build`, `--deploy`, and `--first-log-line`.
Resolve the support ticket after the issue is fixed.
```

## Repository

- `packages/tovuk`: npm CLI.
- `packages/tovuk-py`: PyPI CLI package.
- `crates/tovuk`: Cargo CLI crate.
- `skills/`: agent skill files.
- `examples/`: deployable examples.
- `docs/`: Mintlify documentation.

The Homebrew formula lives in `Formula/tovuk.rb` in this main public repo.

`packages/tovuk` is the CLI behavior source of truth. PyPI and Cargo CLIs must
expose the same agent-facing commands, recovery text, login behavior, deploy
flow, logs, env, domains, usage, billing, and support operations so deploy UX
does not drift.

## Example

```sh
cd examples/hello-rust
tovuk doctor
tovuk deploy
```

On first deploy, the CLI opens browser login, waits for GitHub or Google, stores
the Tovuk session in the user's credential store when available, and continues
the deploy. Later commands reuse that session.

Useful agent commands:

```sh
tovuk capabilities
tovuk pricing --json
tovuk me
tovuk usage
tovuk activity --json
tovuk service list
tovuk service show service_1 --json
tovuk service delete service_1 --json
tovuk deploys
tovuk builds --service service_1
tovuk logs --service service_1 --limit 100 --json
tovuk logs --deploy deploy_1 --json
tovuk logs --build job_1 --json
tovuk env list --service service_1
tovuk domains list --service service_1
tovuk domains verify --service service_1 api.example.com
tovuk database query --service service_1 DB "select 1" --json
tovuk database backup create --service service_1 DB --json
tovuk database backup restore --service service_1 DB sqlite_backup_1 --json
tovuk kv put --service service_1 CACHE user:1 '{"name":"Ada"}' --json
tovuk kv get --service service_1 CACHE user:1 --json
tovuk kv bulk put --service service_1 CACHE '[{"key":"feature:search","value":"enabled"}]' --json
tovuk kv bulk get --service service_1 CACHE feature:search user:1 --json
tovuk kv bulk delete --service service_1 CACHE feature:search old:key --json
tovuk queue send --service service_1 jobs '{"task":"sync"}' --json
tovuk queue send-batch --service service_1 jobs '[{"body":{"task":"sync"}},{"body":{"task":"index"}}]' --json
tovuk queue metrics --service service_1 jobs --json
tovuk storage list --service service_1 --json
tovuk storage upload --service service_1 ./logo.png uploads/logo.png --public --json
tovuk storage url --service service_1 uploads/logo.png --json
tovuk storage download --service service_1 uploads/logo.png ./logo.png --json
tovuk storage delete --service service_1 uploads/logo.png --json
tovuk billing checkout --json
tovuk billing portal
tovuk support create "Deploy failed" "Agent retried deploy after doctor." --service service_1 --build job_1 --deploy deploy_1 --failing-command "tovuk deploy --wait --json" --first-log-line "cargo check failed in src/main.rs" --json
tovuk support list --json
tovuk support resolve ticket_0123456789abcdef0123 --json
```

`tovuk storage upload` automatically uses multipart transfer for files larger
than 100 MiB, so agents can upload large media through the same command.

`tovuk pricing --json` returns both plan pricing and product meter metadata,
so agents can choose Worker, Static Frontend, SQLite, Object Storage, State,
KV, Queues, Cron, Service Bindings, Secrets, Custom Domains, Logs, Builds, or
Usage Caps and set the matching usage caps before heavy work.
It also exposes object storage object, single-part upload, and multipart upload
ceilings so agents can plan large file transfers before reserving bytes.

The same commands are available through PyPI and Cargo after installation:

```sh
pipx install tovuk
cargo install tovuk
tovuk deploy --wait
tovuk logs --build job_1 --json
```
