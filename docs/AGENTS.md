# AGENTS.md

This directory is the Mintlify documentation source for Zerct.

## Rules

1. `docs.json` is the Mintlify root configuration and must stay in this
   directory.
2. The Mintlify dashboard must point at repository `Zerct/zerct`, branch `main`,
   documentation path `/docs`.
3. Zerct is on Mintlify Hobby. Use repository-owned Blacksmith workflows for
   docs validation, deployment triggers, and score checks.
4. Use sentence case headings and short, direct paragraphs.
5. Keep examples copy-pasteable and agent-readable.
6. Do not document unlaunched product behavior as available. Mark it as planned
   or omit it.
7. Do not commit secrets, API keys, customer data, private URLs, or screenshots
   that expose accounts.
8. After editing navigation, run `node scripts/check-docs.mjs` from the repo
   root.
