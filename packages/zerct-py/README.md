# zerct

Python CLI package for deploying Rust backends and static frontends to Zerct.

```sh
pipx install zerct
zerct init
zerct doctor
zerct deploy
```

From a full-stack repo root, `zerct deploy` discovers nested `zerct.toml` files
and deploys the whole workspace in one command.

The npm package remains the primary first install path:

```sh
npx @zerct/zerct deploy
```

On first deploy, the CLI opens browser login, waits for GitHub or Google, stores
the Zerct session in the OS credential store when available, and continues the
deploy. Later commands reuse that session.
