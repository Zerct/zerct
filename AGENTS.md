# Tovuk Public Repo Agent Guide

This repository is the public package and documentation surface for Tovuk. Tovuk
is a paid scraping API service: users authenticate with API keys, create scraper
requests, and read stored public-data results. Do not reintroduce customer
website deploys, backends, databases, workers, object storage buckets, queues,
cron jobs, custom domains, secrets, runtime services, or other cloud-service
products.

Keep this file compact and durable. Codex loads project instructions from the
repo root down to the current directory, and the default combined project
guidance cap is 32 KiB. Put directory-specific rules in a closer `AGENTS.md`
if a subtree needs different commands or ownership. More deeply nested files
override this one. When editing agent instructions, follow the OpenAI Codex
guidance: first remove stale or duplicated guidance, then add rules only for
Tovuk-specific invariants, commands, or verification gates that remain true
across coding-tool upgrades.

Do not delegate implementation, final judgment, verification, or reporting to
subagents. If the user explicitly requests a skill workflow that uses review
subagents, use them only for discovery and independent critique inside the
current Codex thread; they must not create user-owned chats, standalone
automation runs, or separate persistent workstreams. The current Codex thread
still owns the edits, checks, and final report.

Work locally and commit coherent increments. Push to main or master only after
30 local commits and only when the current user or automation instruction
includes that batched-push rule, unless the user explicitly asks earlier. Do
not deploy from this public repo unless the user explicitly asks for a deploy.

## Product Boundary

- Public CLI commands are limited to login, account, pricing, scraper, request,
  usage, billing, and support workflows.
- Public docs, package READMEs, examples, OpenAPI, generated artifacts, tests,
  and skills must describe scraper APIs only.
- Scraper inputs must be public URLs, public handles, public search terms, or
  public identifiers. Never ask users to provide cookies, passwords, account
  tokens, private session data, private repository credentials, proxy URLs, or
  private account content through the public API or CLI.
- There is no free scraper tier. Creating scraper requests requires paid
  billing. Plans are account-level and balance-first. Read the current plan
  catalog from the engine `Plan::pricing()` source before editing plan copy.
  The public CLI must consume `GET /v1/pricing`; public docs and contract checks
  are consumers, not pricing authorities. Do not add another public pricing
  table or static JSON catalog.
- Billing is per successful stored scraper result. Keep pricing exact and
  synchronized across docs, OpenAPI examples, package READMEs, CLI help, and
  contract checks.
- Support escalation must be possible through both
  `tovuk support create "Subject" "Details" --json` and
  `POST /v1/support/tickets` with command output, request id when available,
  and the first actionable error line.

## Rust-Native Boundary

- The native CLI source of truth is `crates/tovuk`.
- Local repository policy and docs checks live in `checks`; this crate is
  local-only and must not be published as a user package.
- `packages/tovuk` ships the native Tovuk binary through npm and must not add
  runtime JavaScript dependencies.
- `packages/tovuk-py` launches or downloads the same native Tovuk binary and
  must keep `TOVUK_NATIVE_BINARY` override support.
- JavaScript and TypeScript are allowed only for static documentation/frontend
  assets. Do not add API routes, SSR handlers, middleware, Node/Bun/Deno
  servers, or TypeScript runtime commands.
- Prefer Rust-native verification. Add a Go-native check only when it is
  stable, stricter than the Rust-native alternatives for a real quality gap,
  and does not add runtime surface or package dependencies.
- Keep Cargo, npm, PyPI, and Homebrew package versions synchronized whenever
  the native CLI changes.
- `Formula/tovuk.rb` is the Homebrew formula for the main `tovuk/tovuk`
  repository.

## API And Docs

- Update `docs/openapi.json` for every public API surface.
- OpenAPI must satisfy `scripts/check-openapi.sh` with a 100 score.
- Update README files when commands, packages, pricing, limits, API behavior, or
  supported scraper surfaces change.
- Mintlify navigation lives in `docs/docs.json`; add every user-facing page
  there so search, sitemap, assistant context, and MCP search can discover it.
- Avoid dashboard-first language. Prefer concrete CLI commands and API
  endpoints.
- Keep prose ASCII-only unless the surrounding file already uses another
  character set.
- Do not use Unicode em dashes in tracked text.

## Checks

Run narrow checks while editing, then run the full suite before handoff when
practical:

```sh
cargo fmt --check --manifest-path crates/tovuk/Cargo.toml
cargo fmt --check --manifest-path checks/Cargo.toml
cargo clippy --locked --release --manifest-path crates/tovuk/Cargo.toml --all-targets --all-features -- -D warnings -D clippy::all -D clippy::pedantic
cargo clippy --locked --release --manifest-path checks/Cargo.toml --all-targets --all-features -- -D warnings -D clippy::all -D clippy::pedantic
npm --prefix packages/tovuk run check
scripts/check-public-contracts.sh package-versions
scripts/check-public-contracts.sh cli-contract
scripts/check-public-contracts.sh docs
./scripts/check-prose-style.sh
scripts/check-openapi.sh
ruby -c Formula/tovuk.rb
./scripts/check-all.sh
```
