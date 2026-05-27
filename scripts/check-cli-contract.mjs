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
const cargoCli = readFileSync('crates/zerct/src/main.rs', 'utf8')

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
  'env',
  'domains',
  'billing'
]

for (const command of commands) {
  requireSnippet(npmCli, `case '${command}':`, `npm command ${command}`)
}

requireSnippet(npmConstants, 'fullstack-rust-tanstack', 'fullstack template option')
requireSnippet(npmConstants, 'tanstack-static-frontend', 'frontend template option')
requireSnippet(npmConstants, 'rust-api', 'Rust template option')

requireSnippet(npmCli, 'installProject(projectPath(cli.args[0]), cli.template)', 'npm install template behavior')
requireSnippet(npmTemplates, 'function installProject', 'npm install template implementation')
requireSnippet(pythonCli, 'ZERCT_NPM_CLI', 'PyPI delegates to local npm CLI for checks')
requireSnippet(pythonCli, 'NPM_PACKAGE = "@zerct/zerct"', 'PyPI delegates to public npm package')
requireSnippet(cargoCli, 'ZERCT_NPM_CLI', 'Cargo delegates to local npm CLI for checks')
requireSnippet(cargoCli, 'const NPM_PACKAGE: &str = "@zerct/zerct";', 'Cargo delegates to public npm package')

console.log('Checked CLI command and template contract.')
