import { readFileSync } from 'node:fs'

function fail(message) {
  console.error(`CLI contract check failed: ${message}`)
  process.exit(1)
}

function requireSnippet(source, snippet, label) {
  if (!source.includes(snippet)) {
    fail(`${label} is missing`)
  }
}

function rejectSnippet(source, snippet, label) {
  if (source.includes(snippet)) {
    fail(`${label} is present`)
  }
}

const npmCli = readFileSync('packages/tovuk/src/tovuk.ts', 'utf8')
const npmAuth = readFileSync('packages/tovuk/src/internal/auth.ts', 'utf8')
const npmConstants = readFileSync('packages/tovuk/src/internal/constants.ts', 'utf8')
const npmTemplates = readFileSync('packages/tovuk/src/internal/templates.ts', 'utf8')
const pythonCli = readFileSync('packages/tovuk-py/src/tovuk/cli.py', 'utf8')
const pythonReadme = readFileSync('packages/tovuk-py/README.md', 'utf8')
const cargoCli = readFileSync('crates/tovuk/src/main.rs', 'utf8')
const cargoReadme = readFileSync('crates/tovuk/README.md', 'utf8')
const homebrewFormula = readFileSync('Formula/tovuk.rb', 'utf8')
const npmPackage = JSON.parse(readFileSync('packages/tovuk/package.json', 'utf8'))
const npmVersion = npmPackage.version

const commands = [
  'init',
  'install',
  'doctor',
  'preview',
  'login',
  'deploy',
  'capabilities',
  'me',
  'usage',
  'activity',
  'apps',
  'overview',
  'deploys',
  'builds',
  'logs',
  'status',
  'inspect',
  'db',
  'database',
  'env',
  'domains',
  'billing',
  'support'
]

for (const command of commands) {
  requireSnippet(npmCli, `['${command}',`, `npm command ${command}`)
}

requireSnippet(npmConstants, 'fullstack-rust-tanstack', 'fullstack template option')
requireSnippet(npmConstants, 'tanstack-static-frontend', 'frontend template option')
requireSnippet(npmConstants, 'rust-api', 'Rust template option')

requireSnippet(npmCli, 'installProject(projectPath(cli.args[0]), cli.template)', 'npm install template behavior')
requireSnippet(npmTemplates, 'function installProject', 'npm install template implementation')
requireSnippet(pythonCli, 'TOVUK_NPM_CLI', 'PyPI delegates to local npm CLI for checks')
requireSnippet(pythonCli, 'NPM_PACKAGE = "tovuk"', 'PyPI delegates to public npm package')
requireSnippet(pythonCli, `NPM_PACKAGE_VERSION = "${npmVersion}"`, 'PyPI pins delegated npm package version')
requireSnippet(cargoCli, 'TOVUK_NPM_CLI', 'Cargo delegates to local npm CLI for checks')
requireSnippet(cargoCli, 'const NPM_PACKAGE: &str = "tovuk";', 'Cargo delegates to public npm package')
requireSnippet(cargoCli, `const NPM_PACKAGE_VERSION: &str = "${npmVersion}";`, 'Cargo pins delegated npm package version')
requireSnippet(homebrewFormula, `tovuk-${npmVersion}.tgz`, 'Homebrew formula pins npm package version')
requireSnippet(homebrewFormula, 'std_npm_args(prefix: libexec)', 'Homebrew formula uses standard npm install args')
requireSnippet(homebrewFormula, 'tovuk billing [checkout|portal]', 'Homebrew formula tests billing commands')
requireSnippet(homebrewFormula, 'tovuk support create', 'Homebrew formula tests support commands')
rejectSnippet(npmAuth, 'path.join(projectDir, SESSION_DIR, SESSION_FILE)', 'project-local session token fallback')

for (const source of [npmConstants, pythonReadme, cargoReadme]) {
  requireSnippet(source, 'tovuk billing checkout --json', 'agentic billing checkout command')
  requireSnippet(source, 'tovuk support create', 'agentic support create command')
  requireSnippet(source, 'tovuk support list', 'agentic support list command')
  requireSnippet(source, 'tovuk support resolve', 'agentic support resolve command')
}

console.log('Checked CLI command and template contract.')
