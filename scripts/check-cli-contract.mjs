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

const npmCli = readFileSync('packages/zerct/bin/zerct.js', 'utf8')
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
  requireSnippet(pythonCli, `case "${command}":`, `PyPI command ${command}`)
  requireSnippet(cargoCli, `"${command}" =>`, `Cargo command ${command}`)
}

for (const source of [npmCli, pythonCli, cargoCli]) {
  requireSnippet(source, 'fullstack-rust-tanstack', 'fullstack template option')
  requireSnippet(source, 'tanstack-static-frontend', 'frontend template option')
  requireSnippet(source, 'rust-api', 'Rust template option')
}

requireSnippet(pythonCli, '"--template"', 'PyPI install template flag')
requireSnippet(pythonCli, 'init_project(pathlib.Path(args.path).resolve(), args.template)', 'PyPI install template behavior')
requireSnippet(npmCli, 'installProject(projectPath(cli.args[0]), cli.template)', 'npm install template behavior')
requireSnippet(cargoCli, 'init_project(&cli.project_path(), cli.template.as_deref())?', 'Cargo install template behavior')

console.log('Checked CLI command and template contract.')
