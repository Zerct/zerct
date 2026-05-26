# zerct

Rust CLI package for deploying Rust backends and static frontends to Zerct.

```sh
cargo install zerct
zerct init
zerct doctor
zerct deploy
```

From a full-stack repo root, `zerct deploy` discovers nested `zerct.toml` files
and deploys the whole workspace in one command.

The primary first-run path remains:

```sh
npx @zerct/zerct deploy
```
