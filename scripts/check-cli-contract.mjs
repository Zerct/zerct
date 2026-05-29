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

const cargoCli = readFileSync('crates/tovuk/src/main.rs', 'utf8')
const cargoReadme = readFileSync('crates/tovuk/README.md', 'utf8')
const npmPackage = JSON.parse(readFileSync('packages/tovuk/package.json', 'utf8'))
const npmInstall = readFileSync('packages/tovuk/install.mjs', 'utf8')
const npmReadme = readFileSync('packages/tovuk/README.md', 'utf8')
const pythonCli = readFileSync('packages/tovuk-py/src/tovuk/cli.py', 'utf8')
const pythonReadme = readFileSync('packages/tovuk-py/README.md', 'utf8')
const homebrewFormula = readFileSync('Formula/tovuk.rb', 'utf8')

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
  requireSnippet(cargoCli, `"${command}"`, `native command ${command}`)
}

for (const source of [cargoCli, cargoReadme, npmReadme, pythonReadme, homebrewFormula]) {
  requireSnippet(source, 'tovuk billing checkout --json', 'agentic billing checkout command')
  requireSnippet(source, 'tovuk support create', 'agentic support create command')
  requireSnippet(source, 'tovuk support list', 'agentic support list command')
  requireSnippet(source, 'tovuk support resolve', 'agentic support resolve command')
}

requireSnippet(cargoCli, 'fullstack-rust-tanstack', 'fullstack template option')
requireSnippet(cargoCli, 'tanstack-static-frontend', 'frontend template option')
requireSnippet(cargoCli, 'rust-api', 'Rust template option')
requireSnippet(cargoCli, 'JavaScript and TypeScript are frontend-only on Tovuk', 'Rust-only backend policy')
requireSnippet(npmInstall, 'TOVUK_NATIVE_BINARY', 'npm local native binary override')
requireSnippet(pythonCli, 'TOVUK_NATIVE_BINARY', 'PyPI local native binary override')
requireSnippet(homebrewFormula, 'depends_on "rust" => :build', 'Homebrew builds native Rust CLI')
requireSnippet(homebrewFormula, 'crates/tovuk', 'Homebrew installs Rust crate path')

if (npmPackage.bin?.tovuk !== 'bin/tovuk') {
  fail('npm package must expose bin/tovuk')
}
if (npmPackage.dependencies || npmPackage.devDependencies) {
  fail('npm package must not ship runtime JavaScript dependencies')
}

for (const source of [cargoCli, npmInstall, pythonCli, cargoReadme, npmReadme, pythonReadme, homebrewFormula]) {
  rejectSnippet(source, 'TOVUK_NPM_CLI', 'legacy npm delegation')
  rejectSnippet(source, 'NPM_PACKAGE_VERSION', 'legacy npm package pin')
  rejectSnippet(source, 'npx -y', 'legacy npx delegation')
  rejectSnippet(source, `@${'zer'}${'ct'}`, 'legacy org scope')
}

console.log('Checked native CLI command and package contract.')
