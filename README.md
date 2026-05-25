# Zerct

Public Zerct workspace for packages, SDKs, agent skills, and examples.

Zerct hosts Rust backends and static frontends. Frontends call Rust backend
deployments for APIs and managed Postgres.

## Install

Use npm for the lowest-friction agent path:

```sh
npx @zerct/zerct init
npx @zerct/zerct doctor
npx @zerct/zerct deploy
```

Static frontend deploys use the same command with this `zerct.toml`:

```toml
name = "dashboard"
kind = "static_frontend"

[build]
check = "npm run typecheck && npm run lint"
command = "npm ci && npm run build"
output = "dist"
```

Rust backend checks must include locked `cargo check` and locked all-target,
all-feature Clippy with `-D warnings`. Static frontend checks must run both
typechecking and linting before build work is queued.

Use Homebrew for a persistent developer CLI:

```sh
brew tap Zerct/tap
brew install zerct
zerct deploy
```

The target npm package is the unscoped package `zerct`. Until npm approves that
name, `@zerct/zerct` is the public npm package.

- npm: `@zerct/zerct`, pending `zerct`
- PyPI: `zerct`
- crates.io: `zerct`
- Homebrew: `Zerct/tap/zerct`

## Repository

- `packages/zerct`: npm CLI.
- `packages/zerct-py`: PyPI CLI package.
- `crates/zerct`: Cargo CLI crate.
- `sdks/`: public SDKs.
- `skills/`: agent skill files.
- `examples/`: deployable examples.
- `docs/`: Mintlify documentation.

Homebrew formulae live in the separate public `Zerct/homebrew-tap` repository.

`packages/zerct` is the CLI behavior source of truth. Other package surfaces
must stay thin or share the same contract so deploy UX does not drift.

## Example

```sh
cd examples/hello-rust
npx @zerct/zerct doctor
npx @zerct/zerct deploy
```

On first deploy, the CLI opens browser login, waits for GitHub or Google, stores
the Zerct session in the user's credential store when available, and continues
the deploy. Later commands reuse that session.
