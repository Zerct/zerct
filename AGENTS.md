# AGENTS.md

This is the public Zerct packages, SDKs, skills, and examples repository.

## Rules

1. Keep public UX simple: prefer `npx zerct deploy`.
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
