export const VERSION: string = '0.1.28'
export const DEFAULT_API_URL: string = 'https://api.zerct.com'
export const ARCHIVE_LIMIT_BYTES: number = 48 * 1024 * 1024
export const DEFAULT_DEPLOY_WAIT_TIMEOUT_SECONDS: number = 900
export const SESSION_DIR: string = '.zerct'
export const SESSION_FILE: string = 'session-token'
export const SESSION_SERVICE: string = 'com.zerct.cli'
export const SESSION_ACCOUNT: string = 'session-token'
export const SESSION_LABEL: string = 'Zerct session'
export const DEFAULT_LOGIN_EXPIRES_SECONDS: number = 600
export const DEFAULT_LOGIN_INTERVAL_SECONDS: number = 5
export const DEFAULT_RUST_CHECK_COMMAND: string = 'cargo fmt --all --check && cargo check --locked && cargo clippy --locked --all-targets --all-features -- -D warnings'
export const DEFAULT_NPM_FRONTEND_CHECK_COMMAND: string = 'npm ci --prefer-offline --no-audit --fund=false && npm run typecheck && npm run lint'
export const DEFAULT_BUN_FRONTEND_CHECK_COMMAND: string = 'bun ci && bun run typecheck && bun run lint'
export const PROJECT_KINDS: ReadonlySet<string> = new Set(['rust_backend', 'static_frontend'])
export const PROJECT_TEMPLATES: ReadonlySet<string> = new Set(['rust-api', 'tanstack-static-frontend', 'fullstack-rust-tanstack'])
export const JAVASCRIPT_LINTERS: ReadonlySet<string> = new Set(['eslint', 'eslint_d', 'jscs', 'jshint', 'prettier', 'prettierd', 'standard', 'xo'])
export const FRONTEND_SOURCE_ROOTS: ReadonlySet<string> = new Set(['src', 'app', 'pages', 'routes', 'components'])
export const FRONTEND_JAVASCRIPT_EXTENSIONS: readonly string[] = ['.js', '.jsx', '.mjs', '.cjs']
export const FRONTEND_PACKAGE_MANAGERS: ReadonlySet<string> = new Set(['npm', 'bun', 'pnpm', 'yarn'])
export const FRONTEND_INSTALL_COMMANDS: ReadonlySet<string> = new Set(['npm ci', 'bun ci', 'bun install', 'pnpm install', 'yarn install'])
export const FRONTEND_TEMPLATE_FILES: ReadonlySet<string> = new Set([
  'index.html',
  'package.json',
  'src/main.tsx',
  'src/styles.css',
  'src/vite-env.d.ts',
  'tsconfig.json',
  'vite.config.ts',
  'zerct.toml'
])
export const ARCHIVE_EXCLUDES: readonly string[] = [
  '.git',
  'target',
  'node_modules',
  '.zerct',
  '.env',
  '.env.*',
  '.npmrc',
  '.pypirc',
  '.netrc',
  '.ssh',
  '.aws',
  '.azure',
  '.kube',
  '.config/gcloud',
  '*.pem',
  '*.key',
  '*.p12',
  '*.pfx',
  'id_rsa',
  'id_ed25519',
  '*.sqlite',
  '*.sqlite3',
  '*.db',
  '*.log',
  '._*',
  '.DS_Store'
]
export const WALK_EXCLUDED_DIRS: ReadonlySet<string> = new Set(['.git', 'target', 'node_modules', '.zerct'])
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

export const HELP: string = `Zerct ${VERSION}

Usage:
  zerct init [path] [--template rust-api|tanstack-static-frontend|fullstack-rust-tanstack]
  zerct install [path] [--template rust-api|tanstack-static-frontend|fullstack-rust-tanstack]
  zerct doctor [path] [--json]
  zerct preview [path] [--port <port>]
  zerct login [--token <token>] [--api <url>]
  zerct deploy [path] [--database] [--wait] [--wait-timeout <seconds>] [--api <url>] [--json]
  zerct capabilities [--api <url>] [--json]
  zerct me [--api <url>] [--json]
  zerct usage [--api <url>] [--json]
  zerct activity [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  zerct apps [--api <url>] [--json]
  zerct overview --app <app> [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  zerct deploys [--app <app>] [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  zerct builds [--app <app>] [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  zerct logs --app <app> [--deploy <deploy_id>] [--build <build_id>] [--limit <n>] [--cursor <cursor>] [--api <url>] [--json]
  zerct status --app <app> [--api <url>] [--json]
  zerct inspect --app <app> [--api <url>] [--json]
  zerct db --app <app> [--api <url>] [--json]
  zerct env list --app <app> [--api <url>] [--json]
  zerct env set --app <app> KEY=value [--api <url>] [--json]
  zerct env delete --app <app> KEY [--api <url>] [--json]
  zerct domains list --app <app> [--api <url>] [--json]
  zerct domains add --app <app> <domain> [--api <url>] [--json]
  zerct domains verify --app <app> <domain> [--api <url>] [--json]
  zerct domains delete --app <app> <domain> [--api <url>] [--json]
  zerct billing [portal] [--api <url>] [--json]

Agent contract:
  - Rust backends keep Cargo.lock committed, pass rustfmt, listen on 0.0.0.0:$PORT, and return HTTP 200 from health.
  - Static frontends set kind = "static_frontend", keep TypeScript source, a package lockfile, and typecheck + lint scripts.
  - Frontends call Rust backends for APIs, managed Postgres, and server-side logic.
  - Run deploy from a repo root with nested zerct.toml files to deploy the whole workspace in one command.
  - When a frontend calls a backend on another hostname, configure backend CORS or use a same-origin custom domain.
  - Keep direct unsafe out of Rust source.
`
