# zerct

Rust CLI package for deploying Rust backends and static frontends to Zerct.

```sh
cargo install zerct
zerct init my-app --template fullstack-rust-tanstack
cd my-app/web && bun install && cd ..
zerct doctor
zerct preview
zerct deploy
```

From a full-stack repo root, `zerct deploy` discovers nested `zerct.toml` files
and deploys the whole workspace in one command.

The primary first-run path remains:

```sh
npx @zerct/zerct deploy
```
