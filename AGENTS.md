# AGENTS.md

This is the public Zerct packages, SDKs, skills, examples, and docs
repository.

This file applies to the whole repository. Nested `AGENTS.md` files may only
tighten these rules.

## Non-Negotiables

1. Do not commit secrets, tokens, OAuth credentials, Stripe keys, customer data,
   private URLs, SSH material, cloud credentials, database dumps, or logs that
   contain sensitive data.
2. Do not use em dashes in any tracked text file. Do not use double hyphen
   prose in Markdown, MDX, or text files. Wrap CLI flags such as `--json` in
   inline code or fenced code.
3. Do not add general lint, test, or CI workflows. GitHub Actions are allowed
   only for package publishing and Mintlify docs validation/score.
4. Every workflow must run on a Blacksmith runner. Use
   `blacksmith-2vcpu-ubuntu-2404` unless measured runtime, memory, or disk
   pressure proves a larger runner is needed.
5. Keep direct Rust `unsafe` out of repo-owned Rust code and examples. Do not
   weaken existing `unsafe_code = "forbid"` settings.
6. Before every commit, run the
   `thermo-nuclear-code-quality-review` skill against the current diff. Treat
   structural regressions, needless abstractions, duplicated helpers, and
   spaghetti branching as blockers unless there is a documented reason.
7. Before finishing, run `./scripts/check-all.sh`, confirm the working tree is
   clean except intentional changes, commit, push, and verify the matching
   publish or docs deploy path. State whether publishing, docs deployment, or
   no-runtime deployment verification happened.
   `check-all` must keep locked Cargo checks and Clippy with `-D warnings` for
   the Cargo CLI and examples. The Cargo CLI must keep extra deny lints for
   debug macros, TODOs, unimplemented code, forgotten memory, large includes,
   path overwrite pushes, panics in Result-returning functions, and unwraps.

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
4. Keep `packages/zerct` TypeScript-only. No `.js`, `.jsx`, `.mjs`, or `.cjs`
   files are allowed inside the npm package. The only runtime dependency
   currently allowed is `tsx`, because the published npm bin is TypeScript
   source and must run on supported Node versions.
   Keep `packages/zerct/tsconfig.json` at the strictest practical compiler
   boundary: `strict`, exact optional properties, unchecked indexed access,
   unchecked side-effect imports, type-only import behavior, isolated modules
   and declarations, erasable syntax, no JavaScript input, and no skipped lib
   checking. Type coverage must remain 100%. The npm check stack must include
   `tsc`, `tsgo`, `oxlint` type-aware linting, `publint`, dependency tree
   validation, vulnerability audit, registry signature audit, and the
   repository npm SDK policy script.
5. Keep `packages/zerct` as the CLI behavior source of truth. PyPI and Cargo
   CLIs are thin delegates to the npm CLI so they expose the same agent-facing
   command surface without reimplementing login, init, doctor, preview, deploy,
   wait, capabilities, identity, usage, activity, apps, overview, deploys,
   builds, logs, status, inspect, database, env, domains, and billing portal
   behavior. SDKs and skills must document the same contract.
   The npm `src/zerct.ts` file must stay a small dispatcher; feature logic
   belongs in focused `src/internal/` modules with explicit types.
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
   backend must declare `[lints.rust]` or `[workspace.lints.rust]` with
   `unsafe_code = "forbid"` and `warnings = "deny"`. Any custom Rust
   `[build].check` must include `cargo fmt --all --check`, locked
   `cargo check`, and locked all-target, all-feature Clippy with
   `-D warnings`; any custom static frontend `[build].check` must install
   dependencies and run both typechecking and linting. Static frontend
   `typecheck` scripts must run `tsgo --noEmit`, and `lint` scripts must run
   native linting such as `oxlint`, `biome check`, or `deno lint`. Static
   frontend browser source must be TypeScript under `src`, `app`, `pages`,
   `routes`, or `components`; reject `.js`, `.jsx`, `.mjs`, and `.cjs` browser
   source.
   Running deploy from a repo root with nested `zerct.toml` files must deploy
   the workspace in one command, with Rust backends before static frontends.
   Template creation, project-kind detection, and local preview behavior must
   also stay aligned across npm, PyPI, Cargo, skills, and docs.
10. Prefer native frontend tooling. New static frontend docs, examples, skills,
    and agent guidance should use Bun with committed `bun.lock`, Go-based
    `tsgo` for TypeScript checking, and Rust-based linters such as `oxlint`,
    `biome`, or `deno lint`. Do not add JavaScript-based lint or format tools
    such as ESLint or Prettier to new examples.
11. Static frontend SDK behavior must be lockfile-aware: use Bun defaults when
    `bun.lock` or `bun.lockb` exists, otherwise keep npm compatibility for
    existing projects. The doctor command must reject weak typecheck scripts
    and JavaScript-based or fake frontend lint/format scripts before upload.
    Keep frontend policy constants centralized inside each CLI implementation
    and update npm, PyPI, and Cargo together.
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
8. Keep the agent repair loop visible in docs, package READMEs, and skills:
   run doctor in JSON mode, apply `agent_instruction`, deploy with wait in JSON
   mode, inspect build logs, rerun doctor, then redeploy.

## Examples And Skills

1. Examples must be minimal, buildable, and safe to copy.
2. Rust examples must include `Cargo.lock`, `zerct.toml`, a health endpoint, and
   pass `cargo fmt --all --check`, `cargo check --locked`, plus
   `cargo clippy --locked --all-targets --all-features -- -D warnings`, with
   no secrets.
3. Static frontend examples must include `zerct.toml` with
   `kind = "static_frontend"`, `package.json`, `tsgo --noEmit` typecheck,
   native linting, TypeScript browser source, a package lockfile, strict
   `[build].check`, and no secrets.
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

After commit and push, verify the pushed change reached the right distribution
path before calling work complete:

- package version changes must be watched through the Blacksmith publish
  workflow and verified against the target registry;
- docs changes under `docs/` must rely on the Mintlify GitHub App for deploy,
  while local validation and score checks still run through `check-all`;
- workflow changes must be inspected with `gh run` or the relevant provider
  status when they affect publishing or docs validation;
- instruction-only changes with no package or docs deploy target must be
  reported as pushed with no runtime publish path.

Do not silently skip publish or deploy verification after pushing. If the
verification cannot run, say exactly why and what remains unverified.

## Required Local Check

Run this before calling work complete:

```sh
./scripts/check-all.sh
```

If any check cannot run, say exactly why and what remains unverified.
