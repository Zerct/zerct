# Publishing

Package publishing is manual and environment-gated. Local checks remain local.
GitHub Actions are limited to release publishing and explicit Mintlify docs
automation. Run `./scripts/check-all.sh` before dispatching any publish
workflow.

All publish workflows use Blacksmith runners. Keep lightweight publish jobs on
`blacksmith-2vcpu-ubuntu-2404`; move to a larger Blacksmith runner only after a
measured publish run needs more CPU, memory, or disk. The Blacksmith GitHub App
must stay installed on every Zerct repository that uses a `blacksmith-*`
runner.

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

The workflow publishes with npm provenance. For the first publish, npm may
require either an existing package trusted publisher configuration or a valid
`NPM_TOKEN` repository environment secret. After trusted publishing works,
remove long-lived publish tokens.

Temporary scoped fallback:

- Package: `@zerct/zerct`
- Workflow: `.github/workflows/publish-npm-scoped.yml`
- Local command: `./scripts/publish-npm-scoped.sh`
- Verified command: `npx @zerct/zerct deploy`

Scoped trusted publisher values:

- Organization or user: `Zerct`
- Repository: `zerct`
- Workflow filename: `publish-npm-scoped.yml`
- Environment name: `npm`
- Allowed action: `npm publish`

The scoped package is generated from `packages/zerct` during publish so the CLI
source stays single-owner.

Unscoped package request:

- Requested package: `zerct`
- npm/GitHub Community request: https://github.com/orgs/community/discussions/196901
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

## Homebrew

Tap repository: `Zerct/homebrew-tap`

Formula: `Formula/zerct.rb`

User command:

```sh
brew tap Zerct/tap
brew install zerct
```

The formula should point at the published crates.io archive for the matching
CLI version. Before pushing a formula change, run:

```sh
brew style Formula/zerct.rb
brew install --build-from-source Zerct/tap/zerct
brew test Zerct/tap/zerct
brew audit --strict --online --new Zerct/tap/zerct
```

## Mintlify docs

Docs source: `docs/`

Domain: `docs.zerct.com`

Mintlify dashboard settings:

- Repository: `Zerct/zerct`
- Branch: `main`
- Documentation path: `/docs`

Zerct uses Mintlify Hobby, so do not rely on Mintlify's paid CI checks. The
repository owns validation through Blacksmith workflows and lets the Mintlify
GitHub App deploy from `main`:

- `.github/workflows/docs-validate.yml`: validates PR docs changes.
- `.github/workflows/docs-deploy.yml`: validates, then optionally triggers a
  Mintlify deploy through the Admin API when configured.
- `.github/workflows/docs-score.yml`: checks public agent-readiness endpoints
  and optionally runs authenticated `mint score` with an A-grade floor.

While `docs.zerct.com` is still in TXT verification, Mintlify can update and
index the docs but fail domain revalidation. The deploy script treats that as a
non-fatal pending-domain state only when the update and indexing already
succeeded.

Optional GitHub configuration:

- Secret: `MINTLIFY_ADMIN_API_KEY`
- Variable: `MINTLIFY_PROJECT_ID`
- Optional secret: `MINTLIFY_CLI_CONFIG_JSON`
- Optional variable: `MINTLIFY_SCORE_MIN` (defaults to `90`)

Mintlify deploys from the configured branch after the GitHub App is installed
on this repository. The deploy workflow can also trigger deployment through the
Admin API. The old `Zerct/docs` repository should be left read-only or archived
after the dashboard is switched.

As of May 25, 2026, `mint score https://docs.zerct.com` reports `92/100`.
Hosted Mintlify pages currently lose points on platform-level HTML size/parity
checks; `mint score https://www.mintlify.com/docs` reports a similar `91/100`.
