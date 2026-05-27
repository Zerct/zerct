# Zerct Public Repo Agent Guide

Zerct public packages must stay usable by coding agents without dashboard-only steps. Keep changes focused on the SDKs, CLI, docs, examples, and package metadata in this repository.

Never delegate Zerct public repo work to subagents. Do the audit,
implementation, verification, and reporting yourself in the current Codex
thread.

## No-Dashboard Contract

- Any user action documented in this repo must have an API, SDK, or CLI path.
- Do not add workflows that require dashboard clicks without also adding a machine-readable endpoint and CLI command.
- Plan-limit handling must produce a subscription checkout URL through `npx @zerct/zerct billing checkout --json`.
- Support escalation must be possible through `npx @zerct/zerct support create "Subject" "Details" --json` with command output, app id, build id, deploy id, and the first actionable log line when available.
- Support resolution must be possible through `npx @zerct/zerct support resolve <ticket_id> --json` after the issue is fixed.
- Keep prompts and docs explicit about what an agent should do next.

## Package Surfaces

- `packages/zerct` is the canonical npm CLI implementation.
- `packages/zerct-py` and `crates/zerct` delegate to the npm CLI through `ZERCT_NPM_CLI`; keep delegated behavior compatible.
- `Formula/zerct.rb` is the canonical Homebrew formula for the main
  `Zerct/zerct` repository. Do not move Homebrew back to a separate tap repo.
- CLI errors must remain JSON-friendly and include `code`, `message`, `agent_instruction`, `docs_url`, and `checkout_url` when payment is required.
- Keep command help, README examples, `docs/llms.txt`, and `docs/skill.md` aligned with any command changes.

## API And Docs

- Update `docs/openapi.json` for every public API surface.
- OpenAPI must satisfy `scripts/check-openapi.sh` with a 100 score.
- Mintlify navigation lives in `docs/docs.json`; add new pages there when they are user-facing.
- Avoid dashboard-first language. Prefer concrete commands and API endpoints.
- Keep prose ASCII-only unless the surrounding file already uses another character set.

## Checks

Run the narrowest relevant checks while editing, then run the full suite before handing off when practical:

```sh
npm --prefix packages/zerct run check
node scripts/check-cli-contract.mjs
node scripts/check-docs.mjs
node scripts/check-prose-style.mjs
scripts/check-openapi.sh
ruby -c Formula/zerct.rb
brew style Formula/zerct.rb
./scripts/check-all.sh
```

For Rust wrapper changes, also run:

```sh
cargo fmt --check --manifest-path crates/zerct/Cargo.toml
cargo clippy --locked --manifest-path crates/zerct/Cargo.toml --all-targets --all-features -- -D warnings
```
