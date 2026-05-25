# Publishing

Package publishing is manual and environment-gated. Local checks remain local;
GitHub Actions are only for release publishing. Run `./scripts/check-all.sh`
before dispatching any publish workflow.

## npm

Package: `zerct`

Workflow: `.github/workflows/publish-npm.yml`

Environment: `npm`

Recommended npm trusted publisher values:

- Organization or user: `Zerct`
- Repository: `zerct`
- Workflow filename: `publish-npm.yml`
- Environment name: `npm`
- Allowed action: `npm publish`

For the very first publish, npm may require either an existing package trusted
publisher configuration or a valid `NPM_TOKEN` repository environment secret.
After trusted publishing works, remove long-lived publish tokens.

Temporary scoped fallback:

- Package: `@zerct/zerct`
- Workflow: `.github/workflows/publish-npm-scoped.yml`
- Local command: `./scripts/publish-npm-scoped.sh`
- Verified command: `npx @zerct/zerct deploy`

The scoped package is generated from `packages/zerct` during publish so the CLI
source stays single-owner.

Unscoped package request:

- Requested package: `zerct`
- npm/GitHub Community request: https://github.com/orgs/community/discussions/196900
- Current npm block: package name is considered too similar to `react`

If npm approves the unscoped package, publish `packages/zerct` directly with
`npm publish --access public` or `.github/workflows/publish-npm.yml`. Keep
`@zerct/zerct` published as the stable fallback.

## PyPI

Project: `zerct`

Workflow: `.github/workflows/publish-pypi.yml`

Environment: `pypi`

Pending publisher values:

- PyPI Project Name: `zerct`
- Owner: `Zerct`
- Repository name: `zerct`
- Workflow name: `publish-pypi.yml`
- Environment name: `pypi`

## crates.io

Crate: `zerct`

Workflow: `.github/workflows/publish-crates.yml`

Environment: `crates`

Required secret:

- `CARGO_REGISTRY_TOKEN`

If crates.io trusted publishing is enabled for the crate later, replace the
long-lived token path with the registry-supported OIDC flow.
