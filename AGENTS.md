# AGENTS.md

This is the public Zerct packages, SDKs, skills, examples, and docs
repository.

This file applies to the whole repository. Nested `AGENTS.md` files may only
tighten these rules.

## Non-Negotiables

1. Do not commit secrets, tokens, OAuth credentials, Stripe keys, customer data,
   private URLs, SSH material, cloud credentials, database dumps, or logs that
   contain sensitive data.
2. Do not use em dashes or double hyphen prose in Markdown, MDX, or text files.
   Wrap CLI flags such as `--json` in inline code or fenced code.
3. Do not add general lint, test, or CI workflows. GitHub Actions are allowed
   only for package publishing, Mintlify docs validation/score, and explicit
   CodeRabbit push review.
4. Every workflow must run on a Blacksmith runner. Use
   `blacksmith-2vcpu-ubuntu-2404` unless measured runtime, memory, or disk
   pressure proves a larger runner is needed.
5. Keep direct Rust `unsafe` out of repo-owned Rust code and examples. Do not
   weaken existing `unsafe_code = "forbid"` settings.
6. Before finishing, run `./scripts/check-all.sh`, confirm the working tree is
   clean except intentional changes, and state whether publishing happened.
   `check-all` must keep locked Cargo checks and Clippy with `-D warnings` for
   the Cargo CLI and examples.

## Change Discipline

1. Prefer the smallest change that solves the task.
2. On every change, simplify: remove stale placeholders, duplicate rules,
   unused package surface, dead docs, and redundant wrappers before adding new
   abstraction.
3. Keep behavior source of truth in one place. Do not let npm, PyPI, Cargo,
   skills, docs, and examples drift into independent deploy behavior.
4. Do not rename public commands, package names, domains, workflow names, or
   published files without updating docs, package metadata, and checks in the
   same change.
5. Preserve agent-ready errors. CLI failures should include a stable code,
   human message, and direct recovery instruction when practical.

## Package Surfaces

1. Public package names stay aligned: npm `zerct`, PyPI `zerct`, crates.io
   `zerct`, and Homebrew `Zerct/tap/zerct`.
2. Until npm approves unscoped `zerct`, the public npm command is:

   ```sh
   npx @zerct/zerct deploy
   ```

3. Keep `packages/zerct/package.json` named `zerct`; the scoped publish script
   rewrites the name to `@zerct/zerct`.
4. Keep `packages/zerct` dependency-free unless a dependency removes real
   complexity and is maintained, small, and necessary.
5. Keep `packages/zerct` as the CLI behavior source of truth. PyPI, Cargo,
   SDKs, and skills must stay thin or share generated contracts.
6. Package CLIs must not print tokens, env values, database URLs, provider
   secrets, or Stripe/Cloudflare values. Print safe presence, status, ids, and
   public URLs only.
7. Package CLIs must not shell out just to detect whether a command exists.
   Use direct PATH lookup or the host language's standard command lookup helper.
8. Source archive creation must exclude local secrets, VCS metadata, build
   outputs, databases, logs, cloud credentials, SSH material, and private key
   files.
9. Rust backend and static frontend deploy behavior must stay aligned across
   npm, PyPI, Cargo, skills, docs, and examples. Any custom Rust
   `[build].check` must include locked `cargo check` and locked all-target,
   all-feature Clippy with `-D warnings`; any custom static frontend
   `[build].check` must run both typechecking and linting. Static frontend
   browser source must be TypeScript under `src`, `app`, `pages`, `routes`, or
   `components`; reject `.js`, `.jsx`, `.mjs`, and `.cjs` browser source.
   Running deploy from a repo root with nested `zerct.toml` files must deploy
   the workspace in one command, with Rust backends before static frontends.
   Template creation, project-kind detection, and local preview behavior must
   also stay aligned across npm, PyPI, Cargo, skills, and docs.
10. Prefer native frontend tooling. New static frontend docs, examples, skills,
    and agent guidance should use Bun with committed `bun.lock`, Go-based
    `tsgo` for TypeScript checking, and Rust-based linters such as `oxlint`,
    `biome`, or `deno lint`. Do not add JavaScript-based linters such as ESLint
    to new examples.
11. Static frontend SDK behavior must be lockfile-aware: use Bun defaults when
    `bun.lock` or `bun.lockb` exists, otherwise keep npm compatibility for
    existing projects. The doctor command must reject JavaScript-based
    frontend lint scripts before upload.
12. Managed Postgres guidance must be connection-efficient. Rust examples,
    docs, and skills must use a small process-wide pool, avoid
    connect-per-request code, and stay compatible with PgBouncer transaction
    pooling. For the `postgres` crate, prefer typed query APIs such as
    `query_typed_one` and `execute_typed` over prepared-statement paths. Free
    examples should keep app-side pools at 4 connections or fewer.
13. Starter templates must create missing target directories, refuse to
    overwrite existing files, include a safe CORS path through
    `FRONTEND_ORIGIN`, and keep dynamic work in the Rust backend.

## Docs And OpenAPI

1. Public Mintlify docs live in `docs/` in this repository. Do not add a
   separate docs repository for public docs.
2. Mintlify settings must remain:

   ```txt
   Repository: Zerct/zerct
   Branch: main
   Documentation path: /docs
   ```

3. Zerct is on Mintlify Hobby. Let the Mintlify GitHub App deploy docs from
   `main`; do not add a duplicate deployment workflow.
4. Do not document unlaunched behavior as available. Mark it as planned or omit
   it.
5. Docs must be short, direct, copy-pasteable, and agent-readable.
6. Every OpenAPI file must pass `scripts/check-openapi.sh` locally through
   `./scripts/check-all.sh`. Keep vacuum pinned, use the all-rules ruleset with
   hard mode, fail at hint severity, and require score `100/100`.
7. If `docs/docs.json` references an OpenAPI file, that file must exist and be
   included in the vacuum check.

## Examples And Skills

1. Examples must be minimal, buildable, and safe to copy.
2. Rust examples must include `Cargo.lock`, `zerct.toml`, a health endpoint, and
   pass `cargo check --locked` plus
   `cargo clippy --locked --all-targets --all-features -- -D warnings`, with
   no secrets.
3. Static frontend examples must include `zerct.toml` with
   `kind = "static_frontend"`, `package.json`, `typecheck` and `lint` scripts,
   TypeScript browser source, a package lockfile, strict `[build].check`, and
   no secrets.
4. New TanStack or Vite frontend examples should prefer `tsgo --noEmit` for
   `typecheck` and source-scoped `oxlint` for `lint`. Avoid JavaScript-based
   linters unless no native equivalent exists for a required rule.
5. Skills must be operational, not marketing. Include direct commands, expected
   files, deploy contract, and common failure fixes.
6. Skills that describe user deployments must keep the no-direct-unsafe policy
   visible.
7. Managed Postgres examples must show bounded queries, explicit column lists,
   and indexes for joins, filters, and ordering before adding load-test or
   analytics endpoints.

## Publishing

1. Never publish with a dirty working tree.
2. Commit and push the exact package source before triggering a publish
   workflow.
3. Bump only packages whose published contents changed. Do not republish an
   existing version.
4. Prefer trusted publishing/OIDC. Use long-lived registry tokens only as a
   fallback for flows that cannot use trusted publishing yet.
5. Package publish workflows may run on pushes that touch package version
   files, but every publish workflow must first check the target registry and
   skip if that exact version already exists.
6. Keep `scripts/check-package-versions.mjs` enforcing version consistency
   across package manifests, lockfiles, and CLI version constants.
7. Homebrew formulae live in `Zerct/homebrew-tap`. Do not duplicate formulae in
   this repository.
8. Current Homebrew command:

   ```sh
   brew tap Zerct/tap
   brew install zerct
   ```

## CodeRabbit

1. Keep `.coderabbit.yaml` strict and concise.
2. Native CodeRabbit GitHub App reviews are PR-based.
3. Push review must use Blacksmith and must never run without an Agentic API key
   stored as `CODERABBIT_API_KEY`.
4. If the key is missing, the push workflow must skip with a notice instead of
   failing.

## Required Local Check

Run this before calling work complete:

```sh
./scripts/check-all.sh
```

If any check cannot run, say exactly why and what remains unverified.
