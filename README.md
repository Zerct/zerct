# Zerct

Public Zerct workspace for packages, SDKs, agent skills, and examples.

Zerct hosts Rust backends and static frontends. Frontends call Rust backend
deploys for APIs and managed Postgres.

## Install

Use npm for the lowest-friction agent path:

```sh
npx @zerct/zerct init
npx @zerct/zerct doctor
npx @zerct/zerct deploy
```

From a full-stack repo root, `npx @zerct/zerct deploy --database` discovers
nested `zerct.toml` projects and deploys the whole workspace in one command.
Rust backends deploy first. Static frontends deploy after them. Managed Postgres
is requested only for Rust backends.

Static frontend deploys use the same command with this `zerct.toml`:

```toml
name = "dashboard"
kind = "static_frontend"

[build]
check = "bun ci && bun run typecheck && bun run lint"
command = "bun run build"
output = "dist"
```

For new TanStack or Vite frontends, prefer fast native checks and avoid
JavaScript-based linters:

```sh
bun add -d @typescript/native-preview oxlint
```

```json
{
  "scripts": {
    "typecheck": "tsgo --noEmit",
    "lint": "oxlint src vite.config.ts --deny-warnings",
    "build": "vite build"
  }
}
```

Rust backend checks must include locked `cargo check` and locked all-target,
all-feature Clippy with `-D warnings`. Static frontend checks must run both
typechecking and linting before build work is queued. Static frontend browser
source must be `.ts` or `.tsx` under `src`, `app`, `pages`, `routes`, or
`components`; browser `.js`, `.jsx`, `.mjs`, and `.cjs` source is rejected. Bun
projects should commit `bun.lock` for the fastest Zerct build path. Existing npm
projects can still deploy with a committed npm lockfile and npm-based build
commands.

Use Homebrew for a persistent developer CLI:

```sh
brew tap Zerct/tap
brew install zerct
zerct deploy
```

The target npm package is the unscoped package `zerct`. Until npm approves that
name, `@zerct/zerct` is the public npm package.

- npm: `@zerct/zerct`, pending `zerct`
- PyPI: `zerct`
- crates.io: `zerct`
- Homebrew: `Zerct/tap/zerct`

## Repository

- `packages/zerct`: npm CLI.
- `packages/zerct-py`: PyPI CLI package.
- `crates/zerct`: Cargo CLI crate.
- `sdks/`: public SDKs.
- `skills/`: agent skill files.
- `examples/`: deployable examples.
- `docs/`: Mintlify documentation.

Homebrew formulae live in the separate public `Zerct/homebrew-tap` repository.

`packages/zerct` is the CLI behavior source of truth. Other package surfaces
must stay thin or share the same contract so deploy UX does not drift.

## Example

```sh
cd examples/hello-rust
npx @zerct/zerct doctor
npx @zerct/zerct deploy
```

On first deploy, the CLI opens browser login, waits for GitHub or Google, stores
the Zerct session in the user's credential store when available, and continues
the deploy. Later commands reuse that session.

Useful agent commands:

```sh
npx @zerct/zerct capabilities
npx @zerct/zerct me
npx @zerct/zerct usage
npx @zerct/zerct apps
npx @zerct/zerct deploys
npx @zerct/zerct builds --app app_1
npx @zerct/zerct logs --app app_1 --limit 100 --json
npx @zerct/zerct logs --deploy deploy_1 --json
npx @zerct/zerct logs --build job_1 --json
npx @zerct/zerct env list --app app_1
npx @zerct/zerct domains list --app app_1
npx @zerct/zerct billing portal
```
