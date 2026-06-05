# Tovuk

Public Tovuk workspace for packages, agent skills, examples, and docs.

Tovuk hosts Rust workers, static frontends, and full-stack services. A
full-stack service uses one `tovuk.toml`, one deployment URL, static files at
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
tovuk new
tovuk check
tovuk account show
tovuk account update --handle your-handle --display-name "Your Team"
tovuk deploy --dry-run
tovuk deploy --wait
tovuk deploy list
tovuk deploy show <deploy_id>
tovuk deploy cancel <deploy_id>
```

`tovuk new` can scaffold a starter `tovuk.toml` from existing files, but deploy
behavior is controlled only by the committed `tovuk.toml`. Agents must review
`[capabilities]` before deploy instead of relying on automatic project
detection.

Create a full-stack starter:

```sh
tovuk new hello-service --template fullstack-rust-tanstack
cd hello-service/web && npm install && cd ..
```

From a full-stack repo root, `tovuk deploy` reads the single root
`tovuk.toml`, builds `api` and `web`, and returns one service URL. Create
databases, KV namespaces, queues, cron triggers, State namespaces,
service bindings, object storage objects, and usage caps through CLI resource
commands. Service binding call chains can use up to 32 worker invocations per
top-level request.

Full-stack deploys use this `tovuk.toml` shape:

```toml
name = "hello-service"
kind = "fullstack"

[capabilities]
static_frontend = true
worker = true
sqlite = false
object_storage = false
kv = false
state = false
queue = false
cron = false
service_bindings = false
secrets = false
custom_domains = false
logs = true
builds = true
usage_caps = true
billing = true
support = true
abuse = true

[worker]
root = "api"
check = "cargo fmt --all --check && cargo check --locked --release --all-targets --all-features && cargo test --locked --release --all-targets --all-features && cargo clippy --locked --release --all-targets --all-features -- -D warnings -D clippy::all -D clippy::pedantic -D clippy::dbg_macro -D clippy::todo -D clippy::unimplemented -D clippy::panic -D clippy::unwrap_used -D clippy::expect_used -D clippy::large_futures -D clippy::large_include_file -D clippy::large_stack_frames -D clippy::mem_forget -D clippy::rc_buffer -D clippy::rc_mutex -D clippy::redundant_clone -D clippy::clone_on_ref_ptr"
build = "cargo build --release"
command = "./target/release/api"
port = 3000
health = "/api/healthz"

[frontend]
root = "web"
check = "npm ci --prefer-offline --no-audit --fund=false && npm run typecheck && npm run lint"
build = "npm run build"
output = "dist"
```

Static frontend deploys use the same command with this `tovuk.toml`:

```toml
name = "marketing-site"
kind = "static_frontend"

[capabilities]
static_frontend = true
worker = false
sqlite = false
object_storage = false
kv = false
state = false
queue = false
cron = false
service_bindings = false
secrets = false
custom_domains = false
logs = true
builds = true
usage_caps = true
billing = true
support = true
abuse = true

[build]
check = "npm ci --prefer-offline --no-audit --fund=false && npm run typecheck && npm run lint"
command = "npm run build"
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
Deploy archives must contain source files only. Tovuk rejects generated
directories, secret files, and compiled artifacts such as `.exe`, `.so`,
`.dylib`, `.jar`, `.node`, and `.rlib`.
Package frontend checks must install dependencies, run stable native type-aware
TypeScript checks, and run native linting plus Fallow dead-code, semantic
duplicate-code, and health gates before build work is queued. Package frontend
browser source must be `.ts` or `.tsx` under
`src`, `app`, `pages`, `routes`, or `components`; browser `.js`, `.jsx`,
`.mjs`, and `.cjs` source is rejected. Bun projects should commit `bun.lock`
for the fastest Tovuk build path. Existing npm projects can still deploy with a
committed npm lockfile and npm-based build commands.

Plain static HTML/CSS/JS sites can deploy without a package manager by setting
`kind = "static_frontend"`, `[capabilities].static_frontend = true`,
`[build].check = ":"`, `[build].command = ":"`, and `[build].output = "."`.

Use Homebrew for a persistent developer CLI:

```sh
brew tap tovuk/tovuk https://github.com/tovuk/tovuk
brew install tovuk
tovuk deploy
```

- npm: `tovuk`
- PyPI: `tovuk`
- crates.io: `tovuk`
- Homebrew: main-repo tap, then `brew install tovuk`

Agent prompt:

```txt
Use Tovuk to deploy this project. Read https://docs.tovuk.com/llms.txt first.
Set `TOVUK_OUTPUT=json` for the session, or pass `--json` on each command.
Run `tovuk check`. Fix the first failed check by following
`agent_instruction`, then rerun check. Run `tovuk deploy --dry-run` and review
warnings, meters, limits, and `billingEstimate`. Deploy with
`tovuk deploy --wait`. Inspect deploy history with
`tovuk deploy list` and `tovuk deploy show <deploy_id>`.
If the build fails, read `tovuk logs --build <build_id>`, fix the
first actionable error, rerun check, and redeploy. If a plan limit blocks work, run
`tovuk billing checkout` and show the returned URL to the
human. For invoices, payment methods, or subscription changes, run
`tovuk billing portal`. If Tovuk support is needed, run `tovuk support create` with
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

`tovuk service show <service> --json` is the canonical agent snapshot for one
service. It includes service state, capabilities, deploy history, build
history, recent logs, env names, domains, resources, `accountUsage`,
`billingEstimate`, and next actions.
Set `TOVUK_OUTPUT=json` when an agent should receive JSON by default.
Without `--json`, `tovuk service list` prints a compact table with kind,
runtime status, URL, enabled and disabled capabilities, and per-Service
resource counts. `tovuk service show` prints a compact Service snapshot with
capabilities, resource counts, current usage, latest deploy, latest build, and
next actions.
Dashboard Overview Service rows expose copyable commands for `service show`,
logs, storage listing, worker request caps, support tickets, and service
deletion.

## Example

```sh
cd examples/hello-rust
tovuk check
tovuk deploy
```

On first deploy, the CLI opens browser login, waits for GitHub or Google, stores
the Tovuk session in the user's credential store when available, and continues
the deploy. Later commands reuse that session.

Useful agent commands:

```sh
tovuk pricing
tovuk pricing --json
tovuk account show --json
tovuk account activity --json
tovuk account update --handle tovuk-team --display-name "Tovuk Team" --json
tovuk deploy --dry-run --json
tovuk deploy list --json
tovuk deploy show deploy_1 --json
tovuk deploy cancel deploy_1 --json
tovuk usage
tovuk usage --json
tovuk service list
tovuk service show service_1 --json
tovuk service delete service_1 --json
tovuk logs --service service_1 --limit 100 --json
tovuk logs --deploy deploy_1 --json
tovuk logs --build job_1 --json
tovuk env list --service service_1
tovuk domains list --service service_1
tovuk domains verify --service service_1 api.example.com
tovuk sqlite query --service service_1 DB "select 1" --json
tovuk sqlite backup create --service service_1 DB --json
tovuk sqlite backup restore --service service_1 DB sqlite_backup_1 --json
tovuk kv put --service service_1 CACHE user:1 '{"name":"Ada"}' --json
tovuk kv get --service service_1 CACHE user:1 --json
tovuk kv bulk put --service service_1 CACHE '[{"key":"feature:search","value":"enabled"}]' --json
tovuk kv bulk get --service service_1 CACHE feature:search user:1 --json
tovuk kv bulk delete --service service_1 CACHE feature:search old:key --json
tovuk queue send --service service_1 jobs '{"task":"sync"}' --json
tovuk queue send-batch --service service_1 jobs '[{"body":{"task":"sync"}},{"body":{"task":"index"}}]' --json
tovuk queue metrics --service service_1 jobs --json
tovuk state list --service service_1 --json
tovuk state objects --service service_1 Room --json
tovuk state keys --service service_1 Room room-1 --json
tovuk state alarm set --service service_1 Room room-1 --delay-seconds 60 --json
tovuk state alarm get --service service_1 Room room-1 --json
tovuk state alarm delete --service service_1 Room room-1 --json
tovuk state delete-value --service service_1 Room room-1 counter --json
tovuk storage list --service service_1 --json
tovuk storage upload --service service_1 ./logo.png uploads/logo.png --public --json
tovuk storage url --service service_1 uploads/logo.png --json
tovuk storage download --service service_1 uploads/logo.png ./logo.png --json
tovuk storage delete --service service_1 uploads/logo.png --json
tovuk billing checkout --json
tovuk billing portal
tovuk support create "Deploy failed" "Agent retried deploy after check." --service service_1 --build job_1 --deploy deploy_1 --failing-command "tovuk deploy --wait --json" --first-log-line "cargo check failed in src/main.rs" --json
tovuk support list --json
tovuk support resolve ticket_0123456789abcdef0123 --json
tovuk abuse report https://demo.tovuk.app "Phishing page" "Credential collection form" --category phishing --reporter-email reporter@example.com --evidence "Screenshot URL and request id" --json
tovuk abuse list --json
tovuk abuse list --operator --json
tovuk abuse appeal abuse_0123456789abcdef0123 "Removed the reported file and rotated credentials." --evidence "deploy_1 remediation log" --json
tovuk abuse triage abuse_0123456789abcdef0123 "Reviewed reporter evidence and target service metadata." --json
tovuk abuse notify-owner abuse_0123456789abcdef0123 "Owner-visible report recorded with evidence summary." --json
tovuk abuse quarantine abuse_0123456789abcdef0123 "Confirmed malware object and preserved scanner evidence." --json
tovuk abuse resolve abuse_0123456789abcdef0123 "Reporter issue remediated and clean deploy verified." --json
tovuk abuse reject abuse_0123456789abcdef0123 "Evidence did not match the reported target." --json
tovuk abuse release abuse_0123456789abcdef0123 "Owner removed object and redeployed clean build." --json
tovuk nodes list --token "$TOVUK_OPERATOR_TOKEN" --json
tovuk nodes drain tovuk-riesling --token "$TOVUK_OPERATOR_TOKEN" --json
tovuk nodes enable tovuk-riesling --token "$TOVUK_OPERATOR_TOKEN" --json
```

`tovuk storage upload` automatically uses multipart transfer for files larger
than 100 MiB, so agents can upload large media through the same command.
Public media uploads reject executable and script payloads. Store artifacts
privately, and use Static Frontend for browser-executed web assets.

`tovuk pricing --json` returns both plan pricing and product meter metadata,
so agents can choose Worker, Static Frontend, SQLite, Object Storage, State,
KV, Queues, Cron, Service Bindings, Secrets, Custom Domains, Logs, Builds, or
Usage Caps and set the matching usage caps before heavy work.
It also exposes object storage object, single-part upload, and multipart upload
ceilings so agents can plan large file transfers before reserving bytes.
`tovuk usage --json` returns `billingEstimate.lineItems` for priced meters and
explicitly free transfer meters, so agents can audit and cap usage before load
tests or public launches.
`tovuk deploy --dry-run --json` is read-only and combines `tovuk.toml`, explicit
enabled and disabled capabilities, quality checks, capability meters, account
limits, and billing estimates before deploy. Each service includes
`missingConfig` for `tovuk.toml` repair, `requiredFixes` for every failed
quality check, and `meterPlan` entries for enabled service meters with meter
units, pricing fields, limit fields, and ready-to-fill `tovuk limits set` cap
commands with `--notify-at-percent`.

The same commands are available through PyPI and Cargo after installation:

```sh
pipx install tovuk
cargo install tovuk
tovuk deploy --wait
tovuk deploy show deploy_1 --json
tovuk deploy cancel deploy_1 --json
tovuk logs --build job_1 --json
```
