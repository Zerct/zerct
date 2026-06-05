# tovuk

Deploy Rust workers, static frontends, and full-stack services to Tovuk.

```sh
npm install -g tovuk
```

```sh
tovuk new hello-service --template fullstack-rust-tanstack
cd hello-service/web && npm install && cd ..
tovuk check --json
tovuk account show --json
tovuk account update --handle tovuk-team --display-name "Tovuk Team" --json
tovuk deploy --dry-run --json
tovuk deploy --wait --json
tovuk deploy list --json
tovuk deploy show deploy_1 --json
tovuk deploy cancel deploy_1 --json
```

For agent sessions, set `TOVUK_OUTPUT=json` once and run the same commands
without repeating `--json`. Use `--output text` on a single command when you
want human-readable output.

The npm package installs the native Tovuk binary for the current platform.
Node is required by npm to install the package, but the installed `tovuk`
command runs as a native binary.

Homebrew uses the main public repository tap:

```sh
brew tap tovuk/tovuk https://github.com/tovuk/tovuk
brew install tovuk
tovuk deploy
```

Rust workers expect `Cargo.toml`, `Cargo.lock`, and `tovuk.toml`. They must
pass `cargo fmt --all --check`, locked release-mode check/test/Clippy gates,
listen on `0.0.0.0:$PORT`, and expose the configured health endpoint.

Package static frontends must use TypeScript browser source, stable native
type-aware TypeScript checks, native linting such as `oxlint` or
`biome check`, and Fallow dead-code, semantic duplicate-code, and health gates.
Deploy archives must contain source files only. Tovuk rejects generated
directories, secret files, and compiled artifacts such as `.exe`, `.so`,
`.dylib`, `.jar`, `.node`, and `.rlib`.

From a full-stack repo root, the same deploy command reads one root
`tovuk.toml`, reads explicit `[capabilities]`, builds the worker and frontend
roots, and returns one service URL with `/api/*` routed to the Rust worker.

`tovuk new` can scaffold a starter `tovuk.toml` from existing files, but deploy
behavior is controlled only by the committed `tovuk.toml`. Review explicit
`[capabilities]` before deploy.

Check before deploying:

```sh
tovuk check
```

Agent repair loop:

```sh
tovuk check --json
tovuk deploy --dry-run --json
tovuk deploy --wait --json
tovuk service status service_1 --json
tovuk deploy show deploy_1 --json
tovuk deploy cancel deploy_1 --json
tovuk logs --build job_1 --json
```

Fix the first failed `agent_instruction`. If a build fails, inspect build logs,
fix the first actionable log error, rerun check, then redeploy.

Agents can create service SQLite databases, KV namespaces, queues, cron triggers,
State namespaces, service bindings, and usage caps through the CLI.

Agents can also inspect pricing, usage, service details, build logs, env
metadata, custom domains, domain verification, service storage files and
media, billing checkout links, billing portal links, service deletion, and
support ticket create/list/resolve actions through the same CLI. Abuse reports,
owner-visible report lists, and owner appeals also use `tovuk abuse`.

Before high-throughput work, read pricing and set hard caps:

```sh
tovuk pricing --json
tovuk usage --json
tovuk account activity --json
tovuk deploy --dry-run --json
tovuk deploy list --json
tovuk deploy show deploy_1 --json
tovuk service show service_1 --json
tovuk limits set worker_requests --period month --value 10000000 --notify-at-percent 80 --json
```

The pricing response includes plan pricing and product meter metadata, so agents
can choose the correct product and cap the right meters in one flow.
The usage response includes `billingEstimate.lineItems` for current-month cost
estimates.
Without `--json`, `tovuk service list` prints a compact table with Service
kind, runtime status, URL, enabled and disabled capabilities, and resource counts.
`tovuk service status` prints only the live/deploy/build summary, while
`tovuk service show` prints the broader Service snapshot.
Dashboard Overview Service rows expose copyable commands for `service status`,
`service show`, logs, storage listing, worker request caps, support tickets,
and service deletion.
The deploy dry-run response combines `tovuk.toml`, explicit enabled and disabled
capabilities, quality checks, capability meters, account limits, and
`billingEstimate` before deploy, without creating a build. Each service
includes `missingConfig` for `tovuk.toml` repair, `requiredFixes` for every
failed quality check, and `meterPlan` entries for enabled service meters with
meter units, pricing fields, limit fields, and ready-to-fill `tovuk limits set`
cap commands with `--notify-at-percent`.

Manage service files and media without dashboard access:

```sh
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
tovuk queue send --service service_1 jobs '{"task":"sync"}' --json
tovuk queue send-batch --service service_1 jobs '[{"body":{"task":"sync"}},{"body":{"task":"index"}}]' --json
tovuk queue metrics --service service_1 jobs --json
tovuk state list --service service_1 --json
tovuk state objects --service service_1 Room --json
tovuk state keys --service service_1 Room room-1 --json
tovuk state put --service service_1 Room room-1 counter 1 --json
tovuk state get --service service_1 Room room-1 counter --json
tovuk state alarm set --service service_1 Room room-1 --delay-seconds 60 --json
tovuk state alarm get --service service_1 Room room-1 --json
tovuk state alarm delete --service service_1 Room room-1 --json
tovuk state delete-value --service service_1 Room room-1 counter --json
```

`tovuk storage upload` automatically switches to multipart transfer for files
larger than 100 MiB.
Public media uploads reject executable and script payloads. Store artifacts
privately, and use Static Frontend for browser-executed web assets.

When a free-tier limit blocks work, run:

```sh
tovuk billing checkout --json
tovuk billing portal
```

When Tovuk support is needed, include enough evidence for a support agent:

```sh
tovuk support create "Deploy failed" "Agent retried deploy after check." --service service_1 --build job_1 --deploy deploy_1 --failing-command "tovuk deploy --wait --json" --first-log-line "cargo check failed in src/main.rs" --json
```

Report and appeal abuse without dashboard access:

```sh
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

When the issue is fixed, resolve the ticket:

```sh
tovuk support list --json
tovuk support resolve ticket_0123456789abcdef0123 --json
```

On first deploy, the CLI opens browser login, waits for GitHub or Google, stores
the Tovuk session in the OS credential store when available, and continues the
deploy. Later commands reuse that session.
