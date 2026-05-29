# Tovuk

Public Tovuk workspace for packages, agent skills, examples, and docs.

Tovuk hosts Rust backends and static frontends. Frontends can be fully dynamic
in the browser by calling Rust backend deploys for APIs and managed Postgres,
without keeping a Node or Bun server alive.

## Install

Use npm for the lowest-friction agent path:

```sh
npx tovuk init
npx tovuk doctor
npx tovuk preview
npx tovuk deploy
```

Create a full-stack starter:

```sh
npx tovuk init my-app --template fullstack-rust-tanstack
cd my-app/web && bun install && cd ..
```

From a full-stack repo root, `npx tovuk deploy --database` discovers
nested `tovuk.toml` projects and deploys the whole workspace in one command.
Rust backends deploy first. Static frontends deploy after them. Managed
Postgres is requested only for Rust backends.

Static frontend deploys use the same command with this `tovuk.toml`:

```toml
name = "dashboard"
kind = "static_frontend"

[build]
check = "bun ci && bun run typecheck && bun run lint"
command = "bun run build"
output = "dist"
```

For new TanStack or Vite frontends, prefer fast native checks and avoid
JavaScript-based lint, format, dead-code, or duplicate-code tooling:

```sh
bun add -d oxlint oxlint-tsgolint fallow
```

```json
{
  "scripts": {
    "typecheck": "oxlint src vite.config.ts --deny-warnings --type-aware --type-check --tsconfig tsconfig.json",
    "lint": "oxlint src vite.config.ts --deny-warnings && fallow dead-code --production --include-dupes --include-entry-exports --fail-on-issues && fallow dupes --production --mode semantic --threshold 1 --ignore-imports --fail-on-issues && fallow health --production --max-cyclomatic 10 --max-cognitive 15 --max-crap 20 --complexity",
    "build": "vite build"
  }
}
```

Rust backend checks must include `cargo fmt --all --check`, locked
`cargo check`, and locked all-target, all-feature Clippy with `-D warnings`.
Frontend checks must install dependencies, run stable native type-aware
TypeScript checks, and run native linting plus Fallow dead-code, semantic
duplicate-code, and health gates before build work is queued. Frontend browser
source must be `.ts` or `.tsx` under
`src`, `app`, `pages`, `routes`, or `components`; browser `.js`, `.jsx`,
`.mjs`, and `.cjs` source is rejected. Bun projects should commit `bun.lock`
for the fastest Tovuk build path. Existing npm projects can still deploy with a
committed npm lockfile and npm-based build commands.

Use Homebrew for a persistent developer CLI:

```sh
brew tap tovuk/tovuk https://github.com/tovuk/tovuk
brew install tovuk
tovuk deploy
```

- npm: `tovuk`
- PyPI: `tovuk`
- crates.io: `tovuk`
- Homebrew: `tovuk/tovuk/tovuk`

Agent prompt:

```txt
Use Tovuk to deploy this project. Read https://docs.tovuk.com/llms.txt first.
Run `npx tovuk doctor --json`. Fix the first failed check by following
`agent_instruction`, then rerun doctor. Deploy with
`npx tovuk deploy --wait --json`. If the build fails, read
`npx tovuk logs --build <build_id> --json`, fix the first actionable
error, rerun doctor, and redeploy. If a plan limit blocks work, run
`npx tovuk billing checkout --json` and show the returned URL to the
human. If Tovuk support is needed, run `npx tovuk support create` with
`--failing-command`, `--app`, `--build`, `--deploy`, and `--first-log-line`.
Resolve the support ticket after the issue is fixed.
```

## Repository

- `packages/tovuk`: npm CLI.
- `packages/tovuk-py`: PyPI CLI package.
- `crates/tovuk`: Cargo CLI crate.
- `skills/`: agent skill files.
- `examples/`: deployable examples.
- `docs/`: Mintlify documentation.

The Homebrew formula lives in `Formula/tovuk.rb` in this main public repo.

`packages/tovuk` is the CLI behavior source of truth. PyPI and Cargo CLIs must
expose the same agent-facing commands, recovery text, login behavior, deploy
flow, logs, env, domains, usage, billing, and support operations so deploy UX
does not drift.

## Example

```sh
cd examples/hello-rust
npx tovuk doctor
npx tovuk deploy
```

On first deploy, the CLI opens browser login, waits for GitHub or Google, stores
the Tovuk session in the user's credential store when available, and continues
the deploy. Later commands reuse that session.

Useful agent commands:

```sh
npx tovuk capabilities
npx tovuk me
npx tovuk usage
npx tovuk activity --json
npx tovuk apps
npx tovuk overview --app app_1 --json
npx tovuk deploys
npx tovuk builds --app app_1
npx tovuk logs --app app_1 --limit 100 --json
npx tovuk logs --deploy deploy_1 --json
npx tovuk logs --build job_1 --json
npx tovuk env list --app app_1
npx tovuk domains list --app app_1
npx tovuk domains verify --app app_1 api.example.com
npx tovuk billing checkout --json
npx tovuk billing portal
npx tovuk support create "Deploy failed" "Agent retried deploy after doctor." --app app_1 --build job_1 --deploy deploy_1 --failing-command "npx tovuk deploy --wait --json" --first-log-line "cargo check failed in src/main.rs" --json
npx tovuk support list --json
npx tovuk support resolve ticket_0123456789abcdef0123 --json
```

The same commands are available through PyPI and Cargo after installation:

```sh
pipx install tovuk
cargo install tovuk
tovuk deploy --wait
tovuk logs --build job_1 --json
```
