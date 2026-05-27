# Zerct Docs

Mintlify docs live in this directory.

In the Mintlify dashboard, configure the docs project as a monorepo:

- Repository: `Zerct/zerct`
- Branch: `main`
- Documentation path: `/docs`

Do not include a trailing slash in the path.

Zerct is on Mintlify Hobby. The Mintlify GitHub App deploys from `main`.
Blacksmith workflows in this repository are only for validation and
agent-readiness scoring.

Local preview:

```sh
npx mint@4.2.578 dev
```

Run the preview command from this `docs/` directory because `docs.json` lives
here.

Local checks:

```sh
npx mint@4.2.578 validate
npx mint@4.2.578 broken-links --check-anchors --check-redirects --check-snippets
npx mint@4.2.578 a11y --skip-contrast
../scripts/check-openapi.sh
```
