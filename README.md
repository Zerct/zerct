# Zerct

Public Zerct workspace for packages, SDKs, agent skills, and examples.

Zerct hosts Rust backends and exposes them as APIs for frontends hosted
anywhere.

## Install

Use npm for the lowest-friction agent path:

```sh
npx @zerct/zerct init
npx @zerct/zerct doctor
npx @zerct/zerct deploy
```

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

Mintlify docs live in the separate public `Zerct/docs` repository.
Homebrew formulae live in the separate public `Zerct/homebrew-tap` repository.

`packages/zerct` is the CLI behavior source of truth. Other package surfaces
must stay thin or share the same contract so deploy UX does not drift.

## Example

```sh
cd examples/hello-rust
npx @zerct/zerct doctor
npx @zerct/zerct deploy
```
