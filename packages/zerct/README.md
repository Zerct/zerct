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

Use `ZERCT_TOKEN` or `npx @zerct/zerct login --token <token>` for authenticated
commands.
