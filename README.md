# Zerct

Public Zerct workspace for packages, SDKs, agent skills, and examples.

Zerct hosts Rust backends and exposes them as APIs for frontends hosted
anywhere.

## Install

The target npm package is the unscoped package `zerct`, so agents and humans can
run:

```sh
npx @zerct/zerct init
npx @zerct/zerct doctor
npx @zerct/zerct deploy
```

Until npm approves the unscoped name, use the scoped fallback:

```sh
npx @zerct/zerct deploy
```

Other package surfaces use the same public name:

- npm: `zerct`
- PyPI: `zerct`
- crates.io: `zerct`

## Repository

- `packages/zerct`: npm CLI.
- `packages/zerct-py`: PyPI CLI package.
- `crates/zerct`: Cargo CLI crate.
- `sdks/`: public SDKs.
- `skills/`: agent skill files.
- `examples/`: deployable examples.

Mintlify docs live in the separate public `Zerct/docs` repository.

`packages/zerct` is the CLI behavior source of truth. Other package surfaces
must stay thin or share the same contract so deploy UX does not drift.

## Example

```sh
cd examples/hello-rust
npx @zerct/zerct doctor
npx @zerct/zerct deploy
```
