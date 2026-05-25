# Zerct Docs

Mintlify docs live in this directory.

In the Mintlify dashboard, configure the docs project as a monorepo:

- Repository: `Zerct/zerct`
- Branch: `main`
- Documentation path: `/docs`

Do not include a trailing slash in the path.

Zerct is on Mintlify Hobby. Do not depend on Mintlify's paid CI checks. Use the
Blacksmith workflows in this repository for validation, deployment triggers, and
agent-readiness scoring.

Local preview:

```sh
npx mint@latest dev
```

Run the preview command from this `docs/` directory because `docs.json` lives
here.

Local checks:

```sh
npx mint@latest validate
npx mint@latest broken-links --check-anchors --check-redirects --check-snippets
npx mint@latest a11y --skip-contrast
../scripts/check-openapi.sh
```
