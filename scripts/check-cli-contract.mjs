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

const npmCli = readFileSync('packages/zerct/src/zerct.ts', 'utf8')
const npmConstants = readFileSync('packages/zerct/src/internal/constants.ts', 'utf8')
const npmTemplates = readFileSync('packages/zerct/src/internal/templates.ts', 'utf8')
const pythonCli = readFileSync('packages/zerct-py/src/zerct/cli.py', 'utf8')
const pythonReadme = readFileSync('packages/zerct-py/README.md', 'utf8')
const cargoCli = readFileSync('crates/zerct/src/main.rs', 'utf8')
const cargoReadme = readFileSync('crates/zerct/README.md', 'utf8')
const homebrewFormula = readFileSync('Formula/zerct.rb', 'utf8')
const npmPackage = JSON.parse(readFileSync('packages/zerct/package.json', 'utf8'))
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
requireSnippet(pythonCli, 'ZERCT_NPM_CLI', 'PyPI delegates to local npm CLI for checks')
requireSnippet(pythonCli, 'NPM_PACKAGE = "@zerct/zerct"', 'PyPI delegates to public npm package')
requireSnippet(pythonCli, `NPM_PACKAGE_VERSION = "${npmVersion}"`, 'PyPI pins delegated npm package version')
requireSnippet(cargoCli, 'ZERCT_NPM_CLI', 'Cargo delegates to local npm CLI for checks')
requireSnippet(cargoCli, 'const NPM_PACKAGE: &str = "@zerct/zerct";', 'Cargo delegates to public npm package')
requireSnippet(cargoCli, `const NPM_PACKAGE_VERSION: &str = "${npmVersion}";`, 'Cargo pins delegated npm package version')
requireSnippet(homebrewFormula, `zerct-${npmVersion}.tgz`, 'Homebrew formula pins npm package version')
requireSnippet(homebrewFormula, 'std_npm_args(prefix: libexec)', 'Homebrew formula uses standard npm install args')
requireSnippet(homebrewFormula, 'zerct billing [checkout|portal]', 'Homebrew formula tests billing commands')
requireSnippet(homebrewFormula, 'zerct support create', 'Homebrew formula tests support commands')

for (const source of [npmConstants, pythonReadme, cargoReadme]) {
  requireSnippet(source, 'zerct billing checkout --json', 'agentic billing checkout command')
  requireSnippet(source, 'zerct support create', 'agentic support create command')
  requireSnippet(source, 'zerct support list', 'agentic support list command')
  requireSnippet(source, 'zerct support resolve', 'agentic support resolve command')
}

console.log('Checked CLI command and template contract.')
