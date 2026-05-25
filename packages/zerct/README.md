# zerct

Deploy Rust backends to Zerct.

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

On first deploy, the CLI opens browser login, waits for GitHub or Google, stores
the Zerct session in the OS credential store when available, and continues the
deploy. Later commands reuse that session.
