# AGENTS.md

This is the public Zerct packages, SDKs, skills, and examples repository.

This file applies to the whole repository. If a nested `AGENTS.md` is added
later, the nested file may only tighten these rules, not weaken them.

## Rules

1. Keep public UX simple: use `npx @zerct/zerct deploy` until npm approves the
   unscoped `zerct` package name.
2. Do not commit secrets, npm tokens, Stripe keys, OAuth secrets, or customer
   data.
3. The source npm package manifest in `packages/zerct/package.json` stays named
   `zerct` so the repo is ready for the unscoped package. The scoped publish
   script rewrites the package name to `@zerct/zerct` until npm approves
   unscoped `zerct`.
4. Keep `packages/zerct` dependency-free unless a dependency removes real
   complexity and is maintained, small, and necessary.
5. CLI errors must include a direct `agent_instruction` when possible.
6. Public Mintlify docs live in `docs/` in this repository. Configure Mintlify
   as a monorepo with documentation path `/docs`.
7. Do not add GitHub Actions unless explicitly requested. Allowed workflows are
   package publishing and Mintlify docs validation, deployment, and score
   checks. Every workflow must run on Blacksmith runners.
8. Run local verification before publishing.
9. Public package names should stay aligned: npm `zerct`, PyPI `zerct`,
   crates.io `zerct`, and Homebrew `Zerct/tap/zerct`.
10. GitHub Actions may publish packages or run explicit docs workflows only. Do
    not add general lint, test, or CI workflows unless explicitly requested.
11. Use `blacksmith-2vcpu-ubuntu-2404` for lightweight package publishing and
    docs jobs. Increase runner size only when a measured job needs more CPU,
    memory, or disk.
12. Keep `packages/zerct` as the CLI behavior source of truth. PyPI, Cargo,
    SDKs, and skills must stay thin or share a generated contract instead of
    growing independent deploy behavior.
13. On every change, remove redundant code, duplicate instructions, stale
    placeholders, and unused package surface before adding new abstractions.
14. Never publish with a dirty working tree. Commit and push the exact package
    source before triggering a package publishing workflow.
15. Bump only the packages whose published contents changed. Do not republish an
    existing version.
16. Prefer trusted publishing/OIDC for registries. Use long-lived npm tokens
    only as a fallback for flows that cannot use trusted publishing.
17. Source archive creation must exclude local secrets, VCS metadata, build
    outputs, databases, logs, cloud credentials, SSH material, and private key
    files. Do not add another deploy archive path without equivalent excludes.
18. Package CLIs must not print tokens, env values, database URLs, provider
    tokens, or Stripe/Cloudflare values. Print presence, status, ids, and URLs
    only when they are safe for users and agents.
19. Keep examples minimal, buildable, and safe to copy. Examples must include
    `Cargo.lock`, `zerct.toml`, a health endpoint, and no secrets.
20. Keep skill files agent-ready: direct commands, expected files, deploy
    contract, and common failure fixes. Do not put marketing copy in skills.
21. Before finishing, run `./scripts/check-all.sh`, confirm no accidental
    untracked files, and state whether publishing was performed.
22. Homebrew formulae live in `Zerct/homebrew-tap`. Do not duplicate formulae in
    this repository.
23. Do not add a separate docs repository for public docs. Keep docs changes
    co-located with SDK, package, and example changes.

## Package Commands

Current public npm command:

```sh
npx @zerct/zerct deploy
```

Use unscoped `npx zerct ...` only after npm approves and the unscoped package is
published from this repository.

Current Homebrew command:

```sh
brew tap Zerct/tap
brew install zerct
```

Current Mintlify settings:

```txt
Repository: Zerct/zerct
Branch: main
Documentation path: /docs
```

## Required Local Checks

Run this before calling work complete:

```sh
./scripts/check-all.sh
```

If a check cannot be run, say exactly why. Package publishing workflows may run
the same local check, but they are not a replacement for checking locally before
committing.
