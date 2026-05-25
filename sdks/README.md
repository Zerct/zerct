# SDKs

Zerct SDKs and package surfaces help agents deploy free Rust backends with
managed Postgres, private logs, and live HTTPS endpoints.

- `js/`: JavaScript and TypeScript package surface
- `rust/`: Rust package surface
- `python/`: Python package surface

Start with the CLI:

```sh
npx @zerct/zerct deploy
```

The OpenAPI contract for account, deploy, logs, and dashboard endpoints lives in
`docs/openapi.json`.
