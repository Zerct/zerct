# Zerct

Public Zerct workspace for packages, SDKs, agent skills, and examples.

Zerct hosts Rust backends and exposes them as APIs for frontends hosted
anywhere.

## Install

The primary package is the unscoped npm package `zerct`, so agents and humans
can run:

```sh
npx zerct init
npx zerct doctor
npx zerct deploy
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

## Example

```sh
cd examples/hello-rust
npx zerct doctor
npx zerct deploy
```
