# AGENTS.md

This directory is the Mintlify documentation source for Zerct. These rules
tighten the repository root rules.

## Rules

1. `docs.json` is the Mintlify root configuration and must stay in this
   directory.
2. The Mintlify dashboard must point at repository `Zerct/zerct`, branch `main`,
   documentation path `/docs`.
3. Zerct is on Mintlify Hobby. Let the Mintlify GitHub App deploy docs from
   `main`; do not add a duplicate deployment workflow.
4. Use sentence case headings, short paragraphs, and concrete commands.
5. Keep examples copy-pasteable and agent-readable.
6. Do not document unlaunched product behavior as available. Mark it as planned
   or omit it.
7. Do not commit secrets, API keys, customer data, private URLs, screenshots
   that expose accounts, or generated files that contain private state.
8. Do not use em dashes or double hyphen prose. Wrap CLI flags in inline code
   or fenced code blocks.
9. OpenAPI changes must keep `docs/openapi.json` valid and pass
   `scripts/check-openapi.sh` locally from the repo root with a vacuum score of
   `100/100`.
10. After editing navigation, run `node scripts/check-docs.mjs` from the repo
    root.
11. Before finishing docs work, run `./scripts/check-all.sh` from the repo root.
