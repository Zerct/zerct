# AGENTS.md

This is the public Zerct packages, SDKs, skills, and examples repository.

## Rules

1. Keep public UX simple: use `npx @zerct/zerct deploy` until npm approves the
   unscoped `zerct` package name.
2. Do not commit secrets, npm tokens, Stripe keys, OAuth secrets, or customer
   data.
3. The npm package name must stay unscoped as `zerct`.
4. Keep `packages/zerct` dependency-free unless a dependency removes real
   complexity.
5. CLI errors must include a direct `agent_instruction` when possible.
6. Public docs live in `Zerct/docs`; do not add Mintlify docs here.
7. Do not add GitHub Actions unless explicitly requested.
8. Prefer local verification before publishing.
9. Public package names should stay aligned: npm `zerct`, PyPI `zerct`, and
   crates.io `zerct`.
10. GitHub Actions may publish packages only. Do not add push/PR check, lint,
    test, or CI workflows unless explicitly requested.
11. Keep `packages/zerct` as the CLI behavior source of truth. PyPI, Cargo,
    SDKs, and skills must stay thin or share a generated contract instead of
    growing independent deploy behavior.
12. On every change, remove redundant code, duplicate instructions, stale
    placeholders, and unused package surface before adding new abstractions.
