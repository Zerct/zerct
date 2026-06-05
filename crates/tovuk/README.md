# tovuk

Rust CLI package for deploying Rust workers, static frontends, and full-stack
services to Tovuk.
This is the native source of truth for the Tovuk CLI. It does not require
Node.js, npm, Python, or any JavaScript runtime.

```sh
cargo install tovuk
tovuk new hello-service --template fullstack-rust-tanstack
cd hello-service/web && npm install && cd ..
tovuk check --json
tovuk account show --json
tovuk account update --handle tovuk-team --display-name "Tovuk Team" --json
tovuk deploy --dry-run --json
tovuk check
tovuk deploy --wait --json
tovuk deploy list --json
tovuk deploy show deploy_1 --json
tovuk deploy cancel deploy_1 --json
```

From a full-stack repo root, `tovuk deploy` reads one root `tovuk.toml`,
builds the worker and frontend roots, and returns one service URL with `/api/*`
routed to the Rust worker.

`tovuk new` can scaffold a starter `tovuk.toml` from existing files, but deploy
behavior is controlled only by the committed `tovuk.toml`. Review explicit
`[capabilities]` before deploy.

Package static frontend deploys require TypeScript browser source, stable native
type-aware TypeScript checks, native linting such as `oxlint` or
`biome check`, and Fallow dead-code, semantic duplicate-code, and health
gates.
Deploy archives must contain source files only. Tovuk rejects generated
directories, secret files, and compiled artifacts such as `.exe`, `.so`,
`.dylib`, `.jar`, `.node`, and `.rlib`.

The npm package is also available:

```sh
npm install -g tovuk
tovuk deploy
```

Homebrew uses the main public repository tap:

```sh
brew tap tovuk/tovuk https://github.com/tovuk/tovuk
brew install tovuk
tovuk deploy
```

The Cargo package exposes the same agent command surface as npm:

```sh
tovuk pricing
tovuk pricing --json
tovuk account show --json
tovuk account activity --json
tovuk deploy --dry-run --json
tovuk deploy list --json
tovuk deploy show deploy_1 --json
tovuk deploy cancel deploy_1 --json
tovuk usage
tovuk usage --json
tovuk service list
tovuk service show service_1 --json
tovuk service delete service_1 --json
tovuk logs --build job_1 --limit 100 --json
tovuk env list --service service_1
tovuk env set --service service_1 API_KEY=value
tovuk env delete --service service_1 API_KEY
tovuk domains add --service service_1 api.example.com
tovuk domains verify --service service_1 api.example.com
tovuk storage list --service service_1 --json
tovuk storage upload --service service_1 ./logo.png uploads/logo.png --public --json
tovuk storage download --service service_1 uploads/logo.png ./logo.png --json
tovuk storage url --service service_1 uploads/logo.png --json
tovuk storage delete --service service_1 uploads/logo.png --json
tovuk sqlite create --service service_1 DB --json
tovuk sqlite query --service service_1 DB "select 1" --json
tovuk sqlite backup create --service service_1 DB --json
tovuk sqlite backup list --service service_1 DB --json
tovuk sqlite backup restore --service service_1 DB sqlite_backup_1 --json
tovuk sqlite delete --service service_1 DB --json
tovuk kv create --service service_1 CACHE --json
tovuk kv put --service service_1 CACHE user:1 '{"name":"Ada"}' --json
tovuk kv get --service service_1 CACHE user:1 --json
tovuk kv bulk put --service service_1 CACHE '[{"key":"feature:search","value":"enabled"}]' --json
tovuk kv bulk get --service service_1 CACHE feature:search user:1 --json
tovuk kv bulk delete --service service_1 CACHE feature:search old:key --json
tovuk kv namespace delete --service service_1 CACHE --json
tovuk queue create --service service_1 failed_jobs --json
tovuk queue create --service service_1 jobs --max-batch-size 10 --max-batch-timeout-seconds 5 --dead-letter-queue failed_jobs --json
tovuk queue update --service service_1 jobs --max-batch-size 25 --json
tovuk queue update --service service_1 jobs --clear-dead-letter-queue --json
tovuk queue send --service service_1 jobs '{"task":"sync"}' --json
tovuk queue send-batch --service service_1 jobs '[{"body":{"task":"sync"}},{"body":{"task":"index"}}]' --json
tovuk queue metrics --service service_1 jobs --json
tovuk queue delete --service service_1 jobs --json
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
tovuk cron create --service service_1 nightly "0 0 * * *" --json
tovuk cron update --service service_1 nightly "*/15 * * * *" --json
tovuk cron disable --service service_1 nightly --json
tovuk cron enable --service service_1 nightly --json
tovuk cron delete --service service_1 nightly --json
tovuk state list --service service_1 --json
tovuk state create --service service_1 Room --json
tovuk state objects --service service_1 Room --json
tovuk state keys --service service_1 Room room-1 --json
tovuk state put --service service_1 Room room-1 counter 1 --json
tovuk state get --service service_1 Room room-1 counter --json
tovuk state alarm set --service service_1 Room room-1 --delay-seconds 60 --json
tovuk state alarm get --service service_1 Room room-1 --json
tovuk state alarm delete --service service_1 Room room-1 --json
tovuk state delete-value --service service_1 Room room-1 counter --json
tovuk state delete --service service_1 Room --json
tovuk binding create --service service_1 AUTH_SERVICE --target auth-service --json
tovuk binding delete --service service_1 AUTH_SERVICE --json
tovuk limits set worker_requests --period day --value 100000 --notify-at-percent 80 --json
tovuk limits set state_requests --period month --value 1000000 --notify-at-percent 80 --json
tovuk limits set state_sqlite_rows_written --period month --value 50000000 --notify-at-percent 80 --json
tovuk limits delete worker_requests --period day --json
tovuk billing checkout --json
tovuk billing portal
tovuk support create "Deploy failed" "Agent retried deploy after check." --service service_1 --build job_1 --deploy deploy_1 --failing-command "tovuk deploy --wait --json" --first-log-line "cargo check failed in src/main.rs" --json
tovuk support list --json
tovuk support resolve ticket_0123456789abcdef0123 --json
```

`tovuk usage --json` includes `billingEstimate.lineItems` for current-month
cost estimates.
`tovuk service show <service> --json` includes `accountUsage` and
`billingEstimate` beside capabilities, service resources, recent deploys,
builds, logs, env names, domains, and next actions.
Without `--json`, `tovuk service list` prints a compact table with Service
kind, runtime status, URL, enabled and disabled capabilities, and resource
counts. `tovuk service show` prints a compact Service snapshot with
capabilities, resource counts, usage, latest deploy, latest build, and next
actions.
Dashboard Overview Service rows expose copyable commands for `service show`,
logs, storage listing, worker request caps, support tickets, and service
deletion.
`tovuk deploy --dry-run --json` combines `tovuk.toml`, quality checks,
capability meters, account limits, and `billingEstimate` before deploy, without
creating a build. Each service includes `missingConfig` for `tovuk.toml`
repair, `requiredFixes` for every failed quality check, and `meterPlan` entries
for enabled service meters with meter units, pricing fields, limit fields, and
ready-to-fill `tovuk limits set` cap commands with `--notify-at-percent`.

`tovuk storage upload` automatically switches to multipart transfer for files
larger than 100 MiB.
Public media uploads reject executable and script payloads. Store artifacts
privately, and use Static Frontend for browser-executed web assets.

Agent repair loop:

```sh
tovuk check --json
tovuk deploy --dry-run --json
tovuk deploy --wait --json
tovuk deploy show deploy_1 --json
tovuk deploy cancel deploy_1 --json
tovuk logs --build job_1 --json
```

Fix the first failed `agent_instruction`. If a build fails, inspect build logs,
fix the first actionable log error, rerun check, then redeploy.

On first deploy, the CLI opens browser login, waits for GitHub or Google, stores
the Tovuk session in the OS credential store when available, and continues the
deploy. Later commands reuse that session.
