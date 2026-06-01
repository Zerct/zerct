pub(crate) const VERSION: &str = "0.1.83";
pub(crate) const DEFAULT_API_URL: &str = "https://api.tovuk.com";
pub(crate) const ARCHIVE_LIMIT_BYTES: usize = 48 * 1024 * 1024;
pub(crate) const DEFAULT_DEPLOY_WAIT_TIMEOUT_SECONDS: u64 = 900;
pub(crate) const SESSION_DIR: &str = ".tovuk";
pub(crate) const SESSION_FILE: &str = "session-token";
pub(crate) const SESSION_SERVICE: &str = "com.tovuk.cli";
pub(crate) const SESSION_ACCOUNT: &str = "session-token";
pub(crate) const SESSION_LABEL: &str = "Tovuk session";
pub(crate) const DEFAULT_LOGIN_EXPIRES_SECONDS: u64 = 600;
pub(crate) const DEFAULT_LOGIN_INTERVAL_SECONDS: u64 = 5;
pub(crate) const BILLING_CHECKOUT_ROUTE: &str = "/v1/billing/checkout";

pub(crate) const RUST_STRICT_CLIPPY_DENY_LINTS: &[&str] = &[
    "clippy::all",
    "clippy::pedantic",
    "clippy::dbg_macro",
    "clippy::todo",
    "clippy::unimplemented",
    "clippy::panic",
    "clippy::unwrap_used",
    "clippy::expect_used",
    "clippy::large_futures",
    "clippy::large_include_file",
    "clippy::large_stack_frames",
    "clippy::mem_forget",
    "clippy::rc_buffer",
    "clippy::rc_mutex",
    "clippy::redundant_clone",
    "clippy::clone_on_ref_ptr",
];

pub(crate) const DEFAULT_RUST_CHECK_COMMAND: &str = "cargo fmt --all --check && cargo check --locked --release --all-targets --all-features && cargo test --locked --release --all-targets --all-features && cargo clippy --locked --release --all-targets --all-features -- -D warnings -D clippy::all -D clippy::pedantic -D clippy::dbg_macro -D clippy::todo -D clippy::unimplemented -D clippy::panic -D clippy::unwrap_used -D clippy::expect_used -D clippy::large_futures -D clippy::large_include_file -D clippy::large_stack_frames -D clippy::mem_forget -D clippy::rc_buffer -D clippy::rc_mutex -D clippy::redundant_clone -D clippy::clone_on_ref_ptr";
pub(crate) const DEFAULT_NPM_FRONTEND_CHECK_COMMAND: &str =
    "npm ci --prefer-offline --no-audit --fund=false && npm run typecheck && npm run lint";
pub(crate) const DEFAULT_BUN_FRONTEND_CHECK_COMMAND: &str =
    "bun ci && bun run typecheck && bun run lint";

pub(crate) const PROJECT_TEMPLATES: &[&str] = &[
    "rust-worker",
    "tanstack-static-frontend",
    "fullstack-rust-tanstack",
];
pub(crate) const JAVASCRIPT_LINTERS: &[&str] = &[
    "eslint",
    "eslint_d",
    "jscs",
    "jshint",
    "prettier",
    "prettierd",
    "standard",
    "xo",
];
pub(crate) const JAVASCRIPT_BACKEND_RUNTIMES: &[&str] = &[
    "astro",
    "bun",
    "deno",
    "next",
    "node",
    "npm",
    "npx",
    "pnpm",
    "svelte-kit",
    "tsx",
    "ts-node",
    "vite",
    "yarn",
];
pub(crate) const FRONTEND_SOURCE_ROOTS: &[&str] = &["src", "app", "pages", "routes", "components"];
pub(crate) const FRONTEND_JAVASCRIPT_EXTENSIONS: &[&str] = &[".js", ".jsx", ".mjs", ".cjs"];
pub(crate) const FRONTEND_PACKAGE_MANAGERS: &[&str] = &["npm", "bun", "pnpm", "yarn"];
pub(crate) const FRONTEND_INSTALL_COMMANDS: &[&str] = &[
    "npm ci",
    "bun ci",
    "bun install",
    "pnpm install",
    "yarn install",
];
pub(crate) const WALK_EXCLUDED_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    ".tovuk",
    ".terraform",
    ".docker",
    ".gnupg",
    ".ssh",
    ".aws",
    ".azure",
    ".kube",
    ".pulumi",
];
pub(crate) const WORKSPACE_EXCLUDED_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    ".tovuk",
    ".terraform",
    ".docker",
    ".gnupg",
    ".ssh",
    ".aws",
    ".azure",
    ".kube",
    ".pulumi",
    ".cache",
    ".next",
    ".turbo",
    "build",
    "coverage",
    "dist",
    "vendor",
];
pub(crate) const ARCHIVE_EXCLUDES: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    ".tovuk",
    ".env",
    ".env.*",
    ".npmrc",
    ".pypirc",
    ".netrc",
    ".docker",
    ".gnupg",
    ".terraform",
    ".terraformrc",
    ".ssh",
    ".aws",
    ".azure",
    ".kube",
    ".pulumi",
    ".cargo/credentials",
    ".cargo/credentials.toml",
    ".config/gcloud",
    ".config/gh",
    ".config/hub",
    ".config/heroku",
    ".config/doctl",
    "*.pem",
    "*.key",
    "*.p12",
    "*.pfx",
    "*.tfstate",
    "*.tfstate.*",
    "id_rsa",
    "id_ed25519",
    "*.sqlite",
    "*.sqlite3",
    "*.db",
    "*.log",
    "._*",
    ".DS_Store",
];
