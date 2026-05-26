# zerct

Python CLI package for deploying Rust backends and static frontends to Zerct.

```sh
pipx install zerct
zerct init my-app --template fullstack-rust-tanstack
cd my-app/web && bun install && cd ..
zerct doctor
zerct preview
zerct deploy
```

From a full-stack repo root, `zerct deploy` discovers nested `zerct.toml` files
and deploys the whole workspace in one command.

The npm package remains the primary first install path:

```sh
npx @zerct/zerct deploy
```

The Python package exposes the same agent command surface as npm:

```sh
zerct capabilities
zerct me
zerct usage
zerct activity --json
zerct apps
zerct overview --app app_1 --json
zerct deploys --app app_1
zerct builds
zerct logs --deploy deploy_1 --limit 100 --json
zerct env list --app app_1
zerct env set --app app_1 API_KEY=value
zerct env delete --app app_1 API_KEY
zerct domains add --app app_1 api.example.com
zerct domains verify --app app_1 api.example.com
zerct billing portal
```

On first deploy, the CLI opens browser login, waits for GitHub or Google, stores
the Zerct session in the OS credential store when available, and continues the
deploy. Later commands reuse that session.
