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
7. Source archive creation must exclude local secrets, VCS metadata, build
   outputs, databases, logs, cloud credentials, SSH material, and private key
   files.

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
   no secrets.
3. Skills must be operational, not marketing. Include direct commands, expected
   files, deploy contract, and common failure fixes.
4. Skills that describe user deployments must keep the no-direct-unsafe policy
   visible.

## Publishing

1. Never publish with a dirty working tree.
2. Commit and push the exact package source before triggering a publish
   workflow.
3. Bump only packages whose published contents changed. Do not republish an
   existing version.
4. Prefer trusted publishing/OIDC. Use long-lived registry tokens only as a
   fallback for flows that cannot use trusted publishing yet.
5. Homebrew formulae live in `Zerct/homebrew-tap`. Do not duplicate formulae in
   this repository.
6. Current Homebrew command:

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
