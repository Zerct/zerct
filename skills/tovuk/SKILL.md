---
name: tovuk
description: Deploy Rust workers, static frontends, and full-stack services to Tovuk with `tovuk`.
---

# Tovuk

Use when a user wants to deploy a Rust worker, static frontend, or
full-stack service to Tovuk.

## Workflow

1. Ensure the project has `tovuk.toml`.
2. For Rust workers, ensure `Cargo.toml`, `Cargo.lock`, a health endpoint,
   `cargo fmt --all --check`, locked release-mode check/test/Clippy gates,
   strict Clippy resource lints, and small declared runtime resources.
3. For static frontends, set `kind = "static_frontend"` and ensure
   `package.json`, TypeScript browser source, stable native type-aware
   typechecking, native linting, Fallow quality gates, a lockfile, and a strict
   frontend check command.
4. For plain static frontends without a package manager, set
   `kind = "static_frontend"`, require `index.html`, use
   `[build].check = ":"`, `[build].command = ":"`, and `[build].output = "."`.
5. For full-stack services, set `kind = "fullstack"` in one root
   `tovuk.toml`, configure `[worker].root` and `[frontend].root`, serve the
   frontend at `/`, and route API calls through same-origin `/api`.
6. Prefer Bun with `bun.lock`, source-scoped Oxlint type-aware checks, and
   Fallow for new package frontends. Avoid JavaScript-based lint, format,
   typecheck, dead-code, or duplicate-code tooling.
7. For a new full-stack project, run
   `tovuk new my-app --template fullstack-rust-tanstack`.
8. Run `tovuk check --json`.
9. Run `tovuk check` when local tools are available.
10. Run `tovuk deploy --wait --json`.
11. If Tovuk returns an `agent_instruction`, apply it, rerun check, and
    redeploy.
12. If a build fails, run `tovuk logs --build <build_id> --json`, fix the
    first actionable log error, rerun check, and redeploy.

## Service resources

Agents can manage runtime resources without dashboard access:

```sh
tovuk service show <service> --json
tovuk database create --service <service> DB --json
tovuk kv create --service <service> CACHE --json
tovuk kv bulk put --service <service> CACHE '[{"key":"feature:search","value":"enabled"}]' --json
tovuk kv bulk get --service <service> CACHE feature:search user:1 --json
tovuk queue create --service <service> jobs --json
tovuk queue send --service <service> jobs '{"task":"sync"}' --json
tovuk queue send-batch --service <service> jobs '[{"body":{"task":"sync"}},{"body":{"task":"index"}}]' --json
tovuk queue metrics --service <service> jobs --json
tovuk cron create --service <service> nightly "0 0 * * *" --json
tovuk cron update --service <service> nightly "*/15 * * * *" --json
tovuk cron disable --service <service> nightly --json
tovuk state create --service <service> Room --json
tovuk binding create --service <service> AUTH_SERVICE --target auth-service --json
tovuk limits set worker_requests --period day --value 100000 --json
tovuk abuse report https://demo.tovuk.app "Phishing page" "Credential collection form" --category phishing --reporter-email reporter@example.com --evidence "Screenshot URL and request id" --json
tovuk abuse list --json
tovuk abuse appeal abuse_0123456789abcdef0123 "Removed the reported file and rotated credentials." --evidence "deploy_1 remediation log" --json
tovuk abuse list --operator --json
tovuk abuse triage abuse_0123456789abcdef0123 "Reviewed reporter evidence and target service metadata." --json
tovuk abuse notify-owner abuse_0123456789abcdef0123 "Owner-visible report recorded with evidence summary." --json
tovuk abuse quarantine abuse_0123456789abcdef0123 "Confirmed malware object and preserved scanner evidence." --json
tovuk abuse resolve abuse_0123456789abcdef0123 "Reporter issue remediated and clean deploy verified." --json
tovuk abuse reject abuse_0123456789abcdef0123 "Evidence did not match the reported target." --json
tovuk abuse release abuse_0123456789abcdef0123 "Owner removed object and redeployed clean build." --json
```

## Contract

Rust workers must listen on `0.0.0.0:$PORT`, expose the configured health
endpoint, pass `cargo fmt --all --check`, run locked release-mode `cargo check`
and `cargo test`, run strict all-target/all-feature Clippy with panic/unwrap
bans and resource-sensitive lints, and avoid direct `unsafe` in workspace
source.

Static frontends must use `.ts` or `.tsx` browser source under `src`, `app`,
`pages`, `routes`, or `components`; install dependencies, run stable native
type-aware TypeScript checks, run native linting plus Fallow dead-code,
semantic duplicate-code, and health gates; build to `[build].output`; and
include `index.html`.

Full-stack frontends call same-origin `/api` for APIs and server-side logic.
JavaScript and TypeScript are frontend-only on Tovuk.

Abuse reports are API and CLI first. Create reports with target URL, category,
reporter email, and evidence. Service owners use `tovuk abuse list --json` and
`tovuk abuse appeal <report_id> --json` with remediation evidence. Operators use
`tovuk abuse list --operator --json`, then triage, notify-owner, quarantine,
resolve, reject, or release with preserved evidence.
