# zerct

Deploy Rust backends and static frontends to Zerct.

```sh
npx @zerct/zerct deploy
```

Target unscoped command, pending npm approval:

```sh
npx zerct init
npx zerct doctor
npx zerct deploy
```

Zerct expects `Cargo.toml`, `Cargo.lock`, and `zerct.toml`. The app must listen
on `0.0.0.0:$PORT` and expose the configured health endpoint.

From a full-stack repo root, the same deploy command discovers nested
`zerct.toml` files and deploys the whole workspace in one command.

Agents can also inspect apps, deploys, builds, app/deploy/build logs, env
metadata, custom domains, and billing portal links through the same CLI.

On first deploy, the CLI opens browser login, waits for GitHub or Google, stores
the Zerct session in the OS credential store when available, and continues the
deploy. Later commands reuse that session.
