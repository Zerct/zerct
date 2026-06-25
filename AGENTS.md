# Tovuk Public Repo Agent Guide

This repository is the public package and documentation surface for Tovuk. Tovuk
is a paid scraping API service: users authenticate with API keys, create scraper
requests, and read stored public-data results. Do not reintroduce customer
website deploys, backends, databases, workers, object storage buckets, queues,
cron jobs, custom domains, secrets, runtime services, or other cloud-service
products.

Keep this file compact and durable. Put directory-specific rules in a closer
`AGENTS.md` if a subtree needs different commands or ownership. More deeply
nested files override this one. When editing agent instructions, first remove
stale or duplicated guidance; add rules only for Tovuk-specific invariants,
commands, or verification gates that remain true across coding-tool upgrades.

Never delegate Tovuk public repo work to subagents. Do the audit,
implementation, verification, and reporting yourself in the current Codex
thread.

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
  billing. Plans are account-level and balance-first: `$20/month` includes
  `$20`, `$100/month` includes `$125`, and `$200/month` includes `$300`.
- Billing is per successful stored scraper result. Keep pricing exact and
  synchronized across docs, OpenAPI examples, package READMEs, CLI help, and
  contract checks.
- Support escalation must be possible through both
  `tovuk support create "Subject" "Details" --json` and
  `POST /v1/support/tickets` with command output, request id when available,
  and the first actionable error line.

## Rust-Native Boundary

- The native CLI source of truth is `crates/tovuk`.
- `packages/tovuk` ships the native Tovuk binary through npm and must not add
  runtime JavaScript dependencies.
- `packages/tovuk-py` launches or downloads the same native Tovuk binary and
  must keep `TOVUK_NATIVE_BINARY` override support.
- JavaScript and TypeScript are allowed only for static documentation/frontend
  assets. Do not add API routes, SSR handlers, middleware, Node/Bun/Deno
  servers, or TypeScript runtime commands.
- Prefer Rust-native verification. Do not add Go checks.
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
cargo clippy --locked --release --manifest-path crates/tovuk/Cargo.toml --all-targets --all-features -- -D warnings -D clippy::all -D clippy::pedantic
npm --prefix packages/tovuk run check
scripts/check-public-contracts.sh package-versions
scripts/check-public-contracts.sh cli-contract
scripts/check-public-contracts.sh docs
./scripts/check-prose-style.sh
scripts/check-openapi.sh
ruby -c Formula/tovuk.rb
./scripts/check-all.sh
```
