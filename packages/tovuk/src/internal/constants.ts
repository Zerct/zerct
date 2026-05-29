import { readFileSync } from 'node:fs'

interface PackageVersionManifest {
  version: string
}

export const VERSION: string = packageVersion()
export const DEFAULT_API_URL: string = 'https://api.tovuk.com'
export const ARCHIVE_LIMIT_BYTES: number = 48 * 1024 * 1024
export const DEFAULT_DEPLOY_WAIT_TIMEOUT_SECONDS: number = 900
export const SESSION_DIR: string = '.tovuk'
export const SESSION_FILE: string = 'session-token'
export const SESSION_SERVICE: string = 'com.tovuk.cli'
export const SESSION_ACCOUNT: string = 'session-token'
export const SESSION_LABEL: string = 'Tovuk session'
export const DEFAULT_LOGIN_EXPIRES_SECONDS: number = 600
export const DEFAULT_LOGIN_INTERVAL_SECONDS: number = 5
export const DEFAULT_RUST_CHECK_COMMAND: string = 'cargo fmt --all --check && cargo check --locked && cargo clippy --locked --all-targets --all-features -- -D warnings'
export const DEFAULT_NPM_FRONTEND_CHECK_COMMAND: string = 'npm ci --prefer-offline --no-audit --fund=false && npm run typecheck && npm run lint'
export const DEFAULT_BUN_FRONTEND_CHECK_COMMAND: string = 'bun ci && bun run typecheck && bun run lint'
export const PROJECT_KINDS: ReadonlySet<string> = new Set(['fullstack', 'rust_backend', 'static_frontend'])
export const PROJECT_TEMPLATES: ReadonlySet<string> = new Set(['rust-api', 'tanstack-static-frontend', 'fullstack-rust-tanstack'])
export const JAVASCRIPT_LINTERS: ReadonlySet<string> = new Set(['eslint', 'eslint_d', 'jscs', 'jshint', 'prettier', 'prettierd', 'standard', 'xo'])
export const JAVASCRIPT_BACKEND_RUNTIMES: ReadonlySet<string> = new Set(['astro', 'bun', 'deno', 'next', 'node', 'npm', 'npx', 'pnpm', 'svelte-kit', 'tsx', 'ts-node', 'vite', 'yarn'])
export const FRONTEND_SOURCE_ROOTS: ReadonlySet<string> = new Set(['src', 'app', 'pages', 'routes', 'components'])
export const FRONTEND_JAVASCRIPT_EXTENSIONS: readonly string[] = ['.js', '.jsx', '.mjs', '.cjs']
export const FRONTEND_PACKAGE_MANAGERS: ReadonlySet<string> = new Set(['npm', 'bun', 'pnpm', 'yarn'])
export const FRONTEND_INSTALL_COMMANDS: ReadonlySet<string> = new Set(['npm ci', 'bun ci', 'bun install', 'pnpm install', 'yarn install'])
export const ARCHIVE_EXCLUDES: readonly string[] = [
  '.git',
  'target',
  'node_modules',
  '.tovuk',
  '.env',
  '.env.*',
  '.npmrc',
  '.pypirc',
  '.netrc',
  '.docker',
  '.gnupg',
  '.terraform',
  '.terraformrc',
  '.ssh',
  '.aws',
  '.azure',
  '.kube',
  '.pulumi',
  '.cargo/credentials',
  '.cargo/credentials.toml',
  '.config/gcloud',
  '.config/gh',
  '.config/hub',
  '.config/heroku',
  '.config/doctl',
  '*.pem',
  '*.key',
  '*.p12',
  '*.pfx',
  '*.tfstate',
  '*.tfstate.*',
  'id_rsa',
  'id_ed25519',
  '*.sqlite',
  '*.sqlite3',
  '*.db',
  '*.log',
  '._*',
  '.DS_Store'
]
export const WALK_EXCLUDED_DIRS: ReadonlySet<string> = new Set([
  '.git',
  'target',
  'node_modules',
  '.tovuk',
  '.terraform',
  '.docker',
  '.gnupg',
  '.ssh',
  '.aws',
  '.azure',
  '.kube',
  '.pulumi'
])
export const WORKSPACE_EXCLUDED_DIRS: ReadonlySet<string> = new Set([
  ...WALK_EXCLUDED_DIRS,
  '.cache',
  '.next',
  '.turbo',
  'build',
  'coverage',
  'dist',
  'vendor'
])

export const HELP: string = `Tovuk ${VERSION}

Usage:
  tovuk init [path] [--template rust-api|tanstack-static-frontend|fullstack-rust-tanstack]
  tovuk install [path] [--template rust-api|tanstack-static-frontend|fullstack-rust-tanstack]
  tovuk doctor [path] [--json]
  tovuk preview [path] [--port <port>]
  tovuk login [--token <token>] [--api <url>]
  tovuk deploy [path] [--database] [--wait] [--wait-timeout <seconds>] [--api <url>] [--json]
  tovuk capabilities [--api <url>] [--json]
  tovuk me [--api <url>] [--json]
  tovuk usage [--api <url>] [--json]
  tovuk activity [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  tovuk apps [--api <url>] [--json]
  tovuk overview --app <app> [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  tovuk deploys [--app <app>] [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  tovuk builds [--app <app>] [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  tovuk logs --app <app> [--deploy <deploy_id>] [--build <build_id>] [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  tovuk status --app <app> [--api <url>] [--json]
  tovuk inspect --app <app> [--api <url>] [--json]
  tovuk db --app <app> [--api <url>] [--json]
  tovuk env list --app <app> [--api <url>] [--json]
  tovuk env set --app <app> KEY=value [--api <url>] [--json]
  tovuk env delete --app <app> KEY [--api <url>] [--json]
  tovuk domains list --app <app> [--api <url>] [--json]
  tovuk domains add --app <app> <domain> [--api <url>] [--json]
  tovuk domains verify --app <app> <domain> [--api <url>] [--json]
  tovuk domains delete --app <app> <domain> [--api <url>] [--json]
  tovuk billing [checkout|portal] [reason] [--api <url>] [--json]
  tovuk support list [--limit <n>] [--api <url>] [--json]
  tovuk support create "Subject" "Details" [--app <app>] [--build <build_id>] [--deploy <deploy_id>] [--failing-command <command>] [--first-log-line <line>] [--severity low|normal|urgent] [--api <url>] [--json]
  tovuk support resolve <ticket_id> [--api <url>] [--json]

Agent contract:
  - Fullstack apps set kind = "fullstack", keep backend and frontend roots in one tovuk.toml, serve the frontend at /, and serve the Rust API under /api.
  - Rust backends keep Cargo.lock committed, pass rustfmt, listen on 0.0.0.0:$PORT, and return HTTP 200 from health.
  - Static frontends set kind = "static_frontend", keep TypeScript source, a package lockfile, stable native typecheck, native lint, and Fallow quality gates.
  - Plain static HTML/CSS/JS frontends may use kind = "static_frontend" with check = ":", command = ":", and output = ".".
  - JavaScript and TypeScript are frontend-only on Tovuk; backend build and runtime commands must be Cargo release builds and Rust release binaries.
  - Frontends call Rust backends for APIs, managed Postgres, and server-side logic.
  - Run deploy from a fullstack repo root with one tovuk.toml to build backend and frontend together.
  - When split frontend and backend apps use different hostnames, configure backend CORS or use a same-origin custom domain.
  - When a plan limit blocks work, run tovuk billing checkout --json and show the returned URL to the human.
  - Create support tickets only with command output, app id, build id, deploy id, and the first actionable log line.
  - Resolve support tickets after the issue is fixed so later agents do not duplicate work.
  - Keep direct unsafe out of Rust source.
`

function packageVersion(): string {
  const manifest: unknown = JSON.parse(readFileSync(new URL('../../package.json', import.meta.url), 'utf8'))
  if (isPackageVersionManifest(manifest)) {
    return manifest.version
  }
  throw new Error('packages/tovuk/package.json must include a version string')
}

function isPackageVersionManifest(value: unknown): value is PackageVersionManifest {
  return typeof value === 'object'
    && value !== null
    && !Array.isArray(value)
    && 'version' in value
    && typeof value.version === 'string'
}
