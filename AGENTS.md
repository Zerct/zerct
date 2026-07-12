# Tovuk Public Repository Guide

This repository is the public distribution and documentation surface for Tovuk:
the native Rust CLI, thin npm and PyPI launchers, Homebrew formula, public API
contract, Mintlify docs, and public agent skill. Keep private runtime,
infrastructure, and customer-service implementation out of this repository.

## Codex Instructions

Codex loads project instructions from the repo root down to the current
directory. In each directory, `AGENTS.override.md` takes precedence over
`AGENTS.md`, and Codex uses at most one instruction file there. A closer
`AGENTS.md` should contain only durable rules for its subtree and overrides
broader guidance. The default combined `project_doc_max_bytes` limit is 32
KiB; keep each file compact and remove stale or duplicated guidance.

Add nested guidance only when a subtree has genuinely different ownership,
commands, or invariants. State its scope in the first six lines. Do not repeat
this discovery policy in nested files.

## Public Boundary

- Public features are limited to login, account, API keys, pricing, scraper
  discovery, scraper requests and results, usage, billing, and support.
- Never add private services, databases, workers, queues, storage, cron jobs,
  domains, hostnames, secrets, credentials, proxy configuration, deployment
  topology, internal endpoints, private repository paths, or private product
  implementation.
- Public scraper inputs are public URLs, handles, search terms, place IDs, or
  other public identifiers. Never request cookies, passwords, session data,
  account tokens, private repository credentials, or private content.
- Reject private implementation details in docs, examples, generated files,
  tests, configuration, commit messages, and release artifacts, not only in
  production code.
- Route vulnerability reports through `SECURITY.md`. Do not publish an
  undisclosed report, credential, or customer data in an issue or fixture.
- Do not copy files or dependency policy from a private repository wholesale.
  Public configuration must be derived from this repository's own manifests,
  lockfiles, release targets, and public contracts.

## Source Of Truth

- The native CLI lives in `crates/tovuk`.
- Repository policy and verification live in `checks`; this crate is
  local-only and must never be published.
- `packages/tovuk` and `packages/tovuk-py` install or launch the same native
  binary. Keep `TOVUK_NATIVE_BINARY` support.
- Do not vendor third-party crates. Published Cargo artifacts must resolve from
  crates.io and build from their own contents without repository-only patches.
- The npm package must have zero runtime and development dependencies. Its MJS
  files are packaging adapters only; product logic belongs in Rust.
- Prefer Rust for validation, parsing, generation, checksums, release policy,
  and workflow logic. Python, JavaScript, Ruby, YAML, and shell are allowed only
  where their package manager or host interface requires them.
- This repository has no TanStack application. Do not add or copy a website
  during CLI, SDK, docs, or strictness work.
- Keep Cargo, npm, PyPI, Python module, Homebrew, native target, and CLI version
  metadata synchronized.
- `GET /v1/pricing` is the public pricing authority. Docs, OpenAPI, examples,
  package READMEs, CLI output, and contract checks are consumers; do not create
  an independent price catalog.

## API And Documentation

- Update `docs/openapi.json` for every public API change.
- Add each user-facing Mintlify page to `docs/docs.json`.
- Keep public routes, examples, limits, scraper names, billing semantics, and
  support behavior synchronized across docs, OpenAPI, packages, CLI help, and
  Rust contract checks.
- Prefer direct CLI commands and HTTP endpoints over dashboard-first wording.
- Keep tracked prose ASCII-only unless an existing external name requires
  otherwise. Do not use Unicode em dashes.
- Preserve the support path through both
  `tovuk support create "Subject" "Details" --json` and
  `POST /v1/support/tickets`.

## Engineering Rules

- Rust 1.97.0, rustfmt, rustc, rustdoc, Clippy, and dependency policy are pinned
  repository requirements.
- Do not add lint suppressions, raise thresholds, disable checks, downgrade
  warnings, or use dirty-package shortcuts. Fix findings structurally.
- The npm launcher deliberately disables only `unicorn/no-null`: its public
  agent-error JSON must emit explicit `null` fields, while `undefined` would
  silently remove those fields. Oxlint's blanket `restriction` category is not
  enabled because it contains browser-only and mutually exclusive Node bans;
  all applicable correctness, suspicious, performance, style, pedantic, and
  nursery categories are errors. Do not add another JavaScript rule exception.
- Do not use `unwrap`, `expect`, panic paths, ignored results, unsafe code,
  unchecked indexing, implicit numeric conversions, or unreviewed dependency
  features.
- Keep functions and files within the configured complexity and size limits.
  Split by domain responsibility rather than moving code mechanically.
- Preserve external JSON, CLI, API, package, and file-format contracts when
  renaming internal Rust identifiers.
- Do not discard unrelated worktree changes. Generated synchronization may
  update tracked files, so inspect the diff before and after every full gate.
- Keep `.gitignore` precise. Ignore known outputs and secrets, but keep
  source-like `build`, `dist`, and SDK paths visible to Git and contract tests.
- After staging an intentional file addition or removal, regenerate
  `public-tree.json` with the Rust `sync-public-tree-policy --write` command,
  inspect it, and stage the generated policy. Never edit its digest manually.

## Verification

Run narrow checks while editing. Before commit, run:

```sh
cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin check-pre-commit --
```

Before push or handoff, run the complete local and CI-equivalent gate:

```sh
cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin check-all --
```

After any history rewrite or leakage purge, scan every locally reachable Git
object with `check-public-contracts -- private-history` before publication.

The full gate must cover formatting, release checks, tests, maximum Clippy,
rustdoc with warnings denied, dependency feature fingerprints, cargo-deny,
cargo-audit, cargo-machete, package/runtime checks, Actions policy, docs,
OpenAPI, prose, TOML, JavaScript adapter syntax, Python, Ruby/Homebrew, and
public-leakage checks. Do not report a failing or skipped gate as passing.

## Release And Deployment

- Deployment for this repository means public CLI/package release and Mintlify
  docs synchronization. There is no public website deployment here.
- Release only a coherent synchronized version. Verify that the version is not
  already published before pushing a release-triggering change.
- Push or publish only when the active user instruction authorizes it.
- After push, monitor all CI, native asset, crates.io, npm, PyPI, Homebrew, and
  Mintlify workflows. Treat remote verification as part of completion.
