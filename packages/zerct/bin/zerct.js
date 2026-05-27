#!/usr/bin/env node
import { spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs'
import { createServer } from 'node:http'
import { homedir } from 'node:os'
import path from 'node:path'

const VERSION = '0.1.14'
const DEFAULT_API_URL = 'https://api.zerct.com'
const ARCHIVE_LIMIT_BYTES = 48 * 1024 * 1024
const DEFAULT_DEPLOY_WAIT_TIMEOUT_SECONDS = 900
const SESSION_DIR = '.zerct'
const SESSION_FILE = 'session-token'
const SESSION_SERVICE = 'com.zerct.cli'
const SESSION_ACCOUNT = 'session-token'
const SESSION_LABEL = 'Zerct session'
const DEFAULT_LOGIN_EXPIRES_SECONDS = 600
const DEFAULT_LOGIN_INTERVAL_SECONDS = 5
const DEFAULT_RUST_CHECK_COMMAND = 'cargo fmt --all --check && cargo check --locked && cargo clippy --locked --all-targets --all-features -- -D warnings'
const DEFAULT_NPM_FRONTEND_CHECK_COMMAND = 'npm ci --prefer-offline --no-audit --fund=false && npm run typecheck && npm run lint'
const DEFAULT_BUN_FRONTEND_CHECK_COMMAND = 'bun ci && bun run typecheck && bun run lint'
const PROJECT_KINDS = new Set(['rust_backend', 'static_frontend'])
const PROJECT_TEMPLATES = new Set(['rust-api', 'tanstack-static-frontend', 'fullstack-rust-tanstack'])
const FRONTEND_TEMPLATE_FILES = new Set([
  'index.html',
  'package.json',
  'src/main.tsx',
  'src/styles.css',
  'src/vite-env.d.ts',
  'tsconfig.json',
  'vite.config.ts',
  'zerct.toml'
])
const ARCHIVE_EXCLUDES = [
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
const WALK_EXCLUDED_DIRS = new Set(['.git', 'target', 'node_modules', '.zerct'])
const WORKSPACE_EXCLUDED_DIRS = new Set([
  ...WALK_EXCLUDED_DIRS,
  '.cache',
  '.next',
  '.turbo',
  'build',
  'coverage',
  'dist',
  'vendor'
])

const HELP = `Zerct ${VERSION}

Usage:
  zerct init [path] [--template rust-api|tanstack-static-frontend|fullstack-rust-tanstack]
  zerct install [path]
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

async function main() {
  const cli = parseArgs(process.argv.slice(2))

  if (cli.help) {
    console.log(HELP)
    return
  }

  if (cli.version) {
    console.log(VERSION)
    return
  }

  switch (cli.command) {
    case 'init':
      initProject(projectPath(cli.args[0]), cli.template)
      break
    case 'install':
      installProject(projectPath(cli.args[0]), cli.template)
      break
    case 'doctor':
      doctorProject(projectPath(cli.args[0]), cli.json)
      break
    case 'preview':
      previewProject(projectPath(cli.args[0]), cli.port)
      break
    case 'login':
      await login(cli)
      break
    case 'deploy':
      await deploy(projectPath(cli.args[0]), cli)
      break
    case 'capabilities':
      await capabilities(cli)
      break
    case 'me':
      await me(cli)
      break
    case 'usage':
      await usage(cli)
      break
    case 'activity':
      await activity(cli)
      break
    case 'apps':
      await apps(cli)
      break
    case 'overview':
      await overview(cli)
      break
    case 'deploys':
      await deploys(cli)
      break
    case 'builds':
      await builds(cli)
      break
    case 'logs':
      await logs(cli)
      break
    case 'status':
      await status(cli)
      break
    case 'inspect':
      await inspect(cli)
      break
    case 'db':
    case 'database':
      await database(cli)
      break
    case 'env':
      await envCommand(cli)
      break
    case 'domains':
      await domainsCommand(cli)
      break
    case 'billing':
      await billing(cli)
      break
    default:
      throw agentError('unknown_command', 'Unknown Zerct command.', 'Run `npx @zerct/zerct --help` and retry with a supported command.', cli.json)
  }
}

function parseArgs(argv) {
  const cli = {
    command: 'help',
    args: [],
    apiUrl: DEFAULT_API_URL,
    app: '',
    build: '',
    deploy: '',
    limit: '',
    cursor: '',
    token: '',
    template: '',
    port: 0,
    waitTimeoutSeconds: DEFAULT_DEPLOY_WAIT_TIMEOUT_SECONDS,
    json: false,
    database: false,
    wait: false,
    help: false,
    version: false
  }

  const positional = []
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index]
    if (arg === '--help' || arg === '-h') {
      cli.help = true
    } else if (arg === '--version' || arg === '-v') {
      cli.version = true
    } else if (arg === '--json') {
      cli.json = true
    } else if (arg === '--database') {
      cli.database = true
    } else if (arg === '--no-database') {
      cli.database = false
    } else if (arg === '--wait') {
      cli.wait = true
    } else if (arg === '--wait-timeout') {
      cli.waitTimeoutSeconds = parsePositiveInteger(requireValue(argv, index, '--wait-timeout'), '--wait-timeout')
      index += 1
    } else if (arg === '--api') {
      cli.apiUrl = requireValue(argv, index, '--api')
      index += 1
    } else if (arg === '--app') {
      cli.app = requireValue(argv, index, '--app')
      index += 1
    } else if (arg === '--build') {
      cli.build = requireValue(argv, index, '--build')
      index += 1
    } else if (arg === '--deploy') {
      cli.deploy = requireValue(argv, index, '--deploy')
      index += 1
    } else if (arg === '--limit') {
      cli.limit = requireValue(argv, index, '--limit')
      index += 1
    } else if (arg === '--cursor') {
      cli.cursor = requireValue(argv, index, '--cursor')
      index += 1
    } else if (arg === '--token') {
      cli.token = requireValue(argv, index, '--token')
      index += 1
    } else if (arg === '--template') {
      cli.template = requireValue(argv, index, '--template')
      index += 1
    } else if (arg === '--port') {
      cli.port = parsePositiveInteger(requireValue(argv, index, '--port'), '--port')
      index += 1
    } else {
      positional.push(arg)
    }
  }

  if (positional.length > 0) {
    cli.command = positional[0]
    cli.args = positional.slice(1)
  }

  cli.apiUrl = trimTrailingSlash(cli.apiUrl)
  return cli
}

function parsePositiveInteger(value, name) {
  const parsed = Number.parseInt(value, 10)
  if (!Number.isInteger(parsed) || parsed <= 0) {
    throw agentError('invalid_argument', `${name} must be a positive integer.`, `Pass ${name} as seconds, for example ${name} 900.`, false)
  }
  return parsed
}

function requireValue(argv, index, name) {
  const value = argv[index + 1]
  if (!value || value.startsWith('--')) {
    throw agentError('missing_argument', `${name} requires a value.`, `Pass a value after ${name}.`, false)
  }
  return value
}

function projectPath(value) {
  return path.resolve(value || process.cwd())
}

function initProject(projectDir, template = '') {
  if (template) {
    mkdirSync(projectDir, { recursive: true, mode: 0o755 })
    createTemplate(projectDir, template)
    return
  }
  ensureDirectory(projectDir)

  const configPath = path.join(projectDir, 'zerct.toml')
  if (existsSync(configPath)) {
    console.log('zerct.toml already exists')
    return
  }

  const kind = inferProjectKind(projectDir)
  const source = kind === 'static_frontend'
    ? frontendConfig(projectDir)
    : rustBackendConfig(projectDir)

  writeFileSync(configPath, source, { mode: 0o644 })
  console.log(`created ${path.relative(process.cwd(), configPath)}`)
  console.log(`detected ${kind}`)
}

function createTemplate(projectDir, template) {
  if (!PROJECT_TEMPLATES.has(template)) {
    throw agentError('invalid_template', 'Zerct template is unknown.', `Use one of: ${[...PROJECT_TEMPLATES].join(', ')}.`, false)
  }
  if (template === 'rust-api') {
    writeRustApiTemplate(projectDir, serviceNameFromDir(projectDir))
  } else if (template === 'tanstack-static-frontend') {
    writeFrontendTemplate(projectDir, serviceNameFromDir(projectDir), '/api')
  } else {
    const apiDir = path.join(projectDir, 'api')
    const webDir = path.join(projectDir, 'web')
    writeRustApiTemplate(apiDir, 'api')
    writeFrontendTemplate(webDir, 'web', 'http://localhost:3000')
  }
  console.log(`created ${template} template`)
}

function writeRustApiTemplate(projectDir, name) {
  mkdirSync(path.join(projectDir, 'src'), { recursive: true, mode: 0o755 })
  writeNewFile(path.join(projectDir, 'Cargo.toml'), `[package]
name = "${name}"
version = "0.1.0"
edition = "2024"
publish = false

[lints.rust]
unsafe_code = "forbid"
warnings = "deny"
`)
  writeNewFile(path.join(projectDir, 'Cargo.lock'), `# This file is automatically @generated by Cargo.
version = 4

[[package]]
name = "${name}"
version = "0.1.0"
`)
  writeNewFile(path.join(projectDir, 'src', 'main.rs'), rustApiSource())
  writeNewFile(path.join(projectDir, 'zerct.toml'), rustBackendConfig(projectDir))
}

function writeFrontendTemplate(projectDir, name, apiBaseUrl) {
  mkdirSync(path.join(projectDir, 'src'), { recursive: true, mode: 0o755 })
  writeNewFile(path.join(projectDir, 'package.json'), `{
  "name": "${name}",
  "private": true,
  "type": "module",
  "scripts": {
    "typecheck": "tsgo --noEmit",
    "lint": "oxlint src vite.config.ts --deny-warnings",
    "build": "vite build",
    "preview": "vite preview --host 0.0.0.0"
  },
  "dependencies": {
    "react": "^19.2.1",
    "react-dom": "^19.2.1",
    "@tanstack/react-router": "^1.140.0"
  },
  "devDependencies": {
    "@types/react": "^19.2.7",
    "@types/react-dom": "^19.2.3",
    "@typescript/native-preview": "^7.0.0-dev.20251126.1",
    "@vitejs/plugin-react": "^5.1.1",
    "oxlint": "^1.30.0",
    "typescript": "^5.9.3",
    "vite": "^7.2.4"
  }
}
`)
  writeFrontendTemplateFile(projectDir, 'index.html', '<div id="root"></div><script type="module" src="/src/main.tsx"></script>\n')
  writeFrontendTemplateFile(projectDir, 'src/styles.css', 'body{margin:0;font-family:system-ui,sans-serif}main{min-height:100svh;display:grid;place-items:center;padding:2rem}code{font-family:ui-monospace,monospace}\n')
  writeFrontendTemplateFile(projectDir, 'src/vite-env.d.ts', '/// <reference types="vite/client" />\n')
  writeFrontendTemplateFile(projectDir, 'src/main.tsx', frontendSource(apiBaseUrl))
  writeFrontendTemplateFile(projectDir, 'tsconfig.json', '{"compilerOptions":{"strict":true,"jsx":"react-jsx","module":"ESNext","moduleResolution":"Bundler","target":"ES2022","noEmit":true,"skipLibCheck":true},"include":["src","vite.config.ts"]}\n')
  writeFrontendTemplateFile(projectDir, 'vite.config.ts', 'import react from "@vitejs/plugin-react";\nimport { defineConfig } from "vite";\n\nexport default defineConfig({ plugins: [react()] });\n')
  writeFrontendTemplateFile(projectDir, 'zerct.toml', frontendConfig(projectDir))
  console.log('run package install in the frontend directory before doctor: bun install or npm install')
}

function writeFrontendTemplateFile(projectDir, relative, source) {
  if (!FRONTEND_TEMPLATE_FILES.has(relative)) {
    throw new Error(`unexpected template file: ${relative}`)
  }
  writeNewFile(path.join(projectDir, relative), source)
}

function writeNewFile(file, source) {
  if (existsSync(file)) {
    throw agentError('file_exists', `Refusing to overwrite ${path.relative(process.cwd(), file)}.`, 'Move the existing file or choose an empty directory, then retry.', false)
  }
  writeFileSync(file, source, { mode: 0o644 })
}

function rustBackendConfig(projectDir) {
  const name = serviceNameFromCargo(projectDir) || serviceNameFromDir(projectDir)
  return `name = "${name}"

[build]
check = "${DEFAULT_RUST_CHECK_COMMAND}"
command = "cargo build --release"

[run]
command = "./target/release/${name}"
port = 3000
health = "/healthz"

[resources]
memory = "512mb"
cpu = "0.25"
idle_timeout_minutes = 15
`
}

function frontendConfig(projectDir) {
  const name = serviceNameFromPackage(projectDir) || serviceNameFromDir(projectDir)
  return `name = "${name}"
kind = "static_frontend"

[build]
check = "${frontendCheckCommand(projectDir)}"
command = "${frontendBuildCommand(projectDir)}"
output = "dist"
`
}

function installProject(projectDir, template = '') {
  initProject(projectDir, template)
  doctorProject(projectDir, false)
}

function doctorProject(projectDir, json) {
  const report = runDoctorWorkspace(projectDir)
  if (json) {
    console.log(JSON.stringify(report, null, 2))
    if (!report.ok) {
      process.exitCode = 1
    }
    return
  }

  if (Array.isArray(report.projects)) {
    for (const project of report.projects) {
      console.log(`project ${project.relative}`)
      for (const check of project.checks) {
        console.log(`${check.ok ? 'ok' : 'fail'} ${check.name}${check.message ? ` - ${check.message}` : ''}`)
      }
    }
  } else {
    for (const check of report.checks) {
      console.log(`${check.ok ? 'ok' : 'fail'} ${check.name}${check.message ? ` - ${check.message}` : ''}`)
    }
  }

  if (!report.ok) {
    const checks = Array.isArray(report.projects)
      ? report.projects.flatMap((project) => project.checks)
      : report.checks
    const firstFailure = checks.find((check) => !check.ok)
    throw agentError('doctor_failed', 'Zerct doctor failed.', firstFailure?.agent_instruction || 'Fix the failed checks and retry `npx @zerct/zerct doctor`.', json)
  }
}

function previewProject(projectDir, port) {
  const report = runDoctorWorkspace(projectDir)
  if (Array.isArray(report.projects)) {
    throw agentError('workspace_preview_unsupported', 'Preview one project at a time.', 'Run `npx @zerct/zerct preview api` or `npx @zerct/zerct preview web` from the workspace root.', false)
  }
  if (!report.ok) {
    const firstFailure = report.checks.find((check) => !check.ok)
    throw agentError('doctor_failed', 'Zerct doctor failed.', firstFailure?.agent_instruction || 'Fix the failed checks and retry `npx @zerct/zerct preview`.', false)
  }

  const config = parseZerctToml(readFileSync(path.join(projectDir, 'zerct.toml'), 'utf8'), projectDir)
  validateConfig(config)
  runShell(config.build.command, projectDir, 'Build failed before preview.')
  if (config.kind === 'static_frontend') {
    serveStatic(path.join(projectDir, config.build.output), port || 4173)
    return
  }

  const runtimePort = port || config.run.port
  console.log(`preview http://127.0.0.1:${runtimePort}`)
  const result = spawnSync(config.run.command, {
    cwd: projectDir,
    env: { ...process.env, PORT: String(runtimePort) },
    shell: true,
    stdio: 'inherit'
  })
  if (result.error) {
    throw agentError('preview_failed', 'Preview command failed.', result.error.message, false)
  }
  if (result.status !== 0) {
    throw agentError('preview_failed', 'Preview command exited with an error.', 'Fix the local runtime command and retry `npx @zerct/zerct preview`.', false)
  }
}

function runShell(command, projectDir, failureMessage) {
  console.log(command)
  const result = spawnSync(command, {
    cwd: projectDir,
    env: process.env,
    shell: true,
    stdio: 'inherit'
  })
  if (result.error) {
    throw agentError('command_failed', failureMessage, result.error.message, false)
  }
  if (result.status !== 0) {
    throw agentError('command_failed', failureMessage, 'Fix the command output above, then retry.', false)
  }
}

function serveStatic(root, port) {
  ensureDirectory(root)
  const server = createServer((request, response) => {
    const pathname = decodeURIComponent(new URL(request.url || '/', `http://127.0.0.1:${port}`).pathname)
    const target = staticTarget(root, pathname)
    if (!target) {
      response.writeHead(404, { 'content-type': 'text/plain; charset=utf-8' })
      response.end('not found')
      return
    }
    response.writeHead(200, { 'content-type': contentType(target) })
    response.end(readFileSync(target))
  })
  server.listen(port, '127.0.0.1', () => {
    console.log(`preview http://127.0.0.1:${port}`)
  })
}

function staticTarget(root, pathname) {
  const safePath = pathname.replace(/^\/+/u, '')
  const candidate = path.resolve(root, safePath || 'index.html')
  if (!candidate.startsWith(path.resolve(root) + path.sep) && candidate !== path.resolve(root)) {
    return ''
  }
  if (existsSync(candidate) && statSync(candidate).isFile()) {
    return candidate
  }
  const index = path.join(root, 'index.html')
  return existsSync(index) ? index : ''
}

function contentType(file) {
  if (file.endsWith('.html')) {
    return 'text/html; charset=utf-8'
  }
  if (file.endsWith('.css')) {
    return 'text/css; charset=utf-8'
  }
  if (file.endsWith('.js') || file.endsWith('.mjs')) {
    return 'text/javascript; charset=utf-8'
  }
  if (file.endsWith('.json')) {
    return 'application/json; charset=utf-8'
  }
  if (file.endsWith('.svg')) {
    return 'image/svg+xml'
  }
  return 'application/octet-stream'
}

function runDoctorWorkspace(projectDir) {
  if (existsSync(path.join(projectDir, 'zerct.toml'))) {
    return runDoctor(projectDir)
  }

  const projects = discoverDeployProjects(projectDir)
  if (projects.length === 0) {
    return runDoctor(projectDir)
  }

  const reports = projects.map((project) => ({
    relative: project.relative,
    ...runDoctor(project.dir)
  }))
  return {
    ok: reports.every((report) => report.ok),
    workspace: projectDir,
    projects: reports
  }
}

function runDoctor(projectDir) {
  const checks = []
  let config = null
  let configValid = false
  const configPath = path.join(projectDir, 'zerct.toml')
  if (existsSync(configPath)) {
    try {
      config = parseZerctToml(readFileSync(configPath, 'utf8'), projectDir)
      validateConfig(config)
      configValid = true
      checks.push({ name: 'zerct.toml', ok: true, message: 'valid' })
    } catch (error) {
      checks.push({
        name: 'zerct.toml',
        ok: false,
        message: error.message,
        agent_instruction: `Fix zerct.toml: ${error.message}.`
      })
    }
  } else {
    checks.push({
      name: 'zerct.toml',
      ok: false,
      message: 'missing',
      agent_instruction: 'Create and commit zerct.toml, then retry.'
    })
  }

  const kind = config?.kind || 'rust_backend'
  const requiredFiles = kind === 'static_frontend'
    ? ['package.json']
    : ['Cargo.toml', 'Cargo.lock']
  for (const file of requiredFiles) {
    const ok = existsSync(path.join(projectDir, file))
    checks.push({
      name: file,
      ok,
      message: ok ? 'found' : 'missing',
      agent_instruction: `Create and commit ${file}, then retry.`
    })
  }

  if (kind === 'static_frontend') {
    const hasLockfile = frontendLockfileExists(projectDir)
    checks.push({
      name: 'frontend lockfile',
      ok: hasLockfile,
      message: hasLockfile ? 'found' : 'missing',
      agent_instruction: 'Commit package-lock.json, pnpm-lock.yaml, yarn.lock, bun.lock, or bun.lockb, then retry.'
    })
    checks.push(...frontendSourceChecks(projectDir))
    checks.push(...frontendScriptChecks(projectDir, configValid))
  }

  const unsafeHits = scanUnsafe(projectDir)
  checks.push({
    name: 'unsafe',
    ok: unsafeHits.length === 0,
    message: unsafeHits.length === 0 ? 'no direct unsafe found' : unsafeHits.slice(0, 5).join(', '),
    agent_instruction: 'Remove direct unsafe usage from workspace Rust source before deploying.'
  })
  if (kind === 'rust_backend' && configValid) {
    checks.push(cargoFmt(projectDir))
    checks.push(cargoCheck(projectDir))
    checks.push(cargoClippy(projectDir))
  }

  return {
    ok: checks.every((check) => check.ok),
    project: projectDir,
    config,
    checks
  }
}

function cargoCheck(projectDir) {
  const cargo = spawnSync('cargo', ['check', '--locked', '--quiet'], {
    cwd: projectDir,
    encoding: 'utf8',
    env: { ...process.env, CARGO_TERM_COLOR: 'never' },
    stdio: ['ignore', 'pipe', 'pipe']
  })

  if (cargo.error) {
    return {
      name: 'cargo check',
      ok: false,
      message: cargo.error.message,
      agent_instruction: 'Install Rust and Cargo, then run `cargo check --locked` locally before deploying.'
    }
  }

  return {
    name: 'cargo check',
    ok: cargo.status === 0,
    message: cargo.status === 0 ? 'passed' : (cargo.stderr || cargo.stdout || 'cargo check failed').trim().slice(0, 240),
    agent_instruction: 'Run `cargo check --locked`, fix every compiler error and warning, then redeploy.'
  }
}

function cargoFmt(projectDir) {
  const cargo = spawnSync('cargo', ['fmt', '--all', '--check'], {
    cwd: projectDir,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe']
  })

  if (cargo.error) {
    return {
      name: 'cargo fmt',
      ok: false,
      message: cargo.error.message,
      agent_instruction: 'Install rustfmt with Rust, then run `cargo fmt --all --check` before deploying.'
    }
  }

  return {
    name: 'cargo fmt',
    ok: cargo.status === 0,
    message: cargo.status === 0 ? 'passed' : (cargo.stderr || cargo.stdout || 'cargo fmt failed').trim().slice(0, 240),
    agent_instruction: 'Run `cargo fmt --all`, then redeploy.'
  }
}

function cargoClippy(projectDir) {
  const cargo = spawnSync('cargo', ['clippy', '--locked', '--all-targets', '--all-features', '--quiet', '--', '-D', 'warnings'], {
    cwd: projectDir,
    encoding: 'utf8',
    env: { ...process.env, CARGO_TERM_COLOR: 'never' },
    stdio: ['ignore', 'pipe', 'pipe']
  })

  if (cargo.error) {
    return {
      name: 'cargo clippy',
      ok: false,
      message: cargo.error.message,
      agent_instruction: 'Install Rust clippy, then run `cargo clippy --locked --all-targets --all-features -- -D warnings` before deploying.'
    }
  }

  return {
    name: 'cargo clippy',
    ok: cargo.status === 0,
    message: cargo.status === 0 ? 'passed' : (cargo.stderr || cargo.stdout || 'cargo clippy failed').trim().slice(0, 240),
    agent_instruction: 'Run `cargo clippy --locked --all-targets --all-features -- -D warnings`, fix every warning, then redeploy.'
  }
}

async function login(cli) {
  if (cli.token) {
    writeSessionToken(cli.token)
    console.log('saved Zerct session token')
    return
  }

  await loginAndStore(cli)
}

async function deploy(projectDir, cli) {
  const projects = discoverDeployProjects(projectDir)
  if (projects.length === 0) {
    throw agentError('missing_project_contract', 'No zerct.toml was found.', 'Run `npx @zerct/zerct init` in each app directory, or pass a project path.', cli.json)
  }

  if (projects.length === 1) {
    const project = projects[0]
    if (project.kind === 'static_frontend' && cli.database) {
      throw agentError('invalid_database_target', 'Static frontends cannot attach managed Postgres directly.', 'Deploy a Rust backend with managed Postgres and call it from the frontend.', cli.json)
    }
    const token = await readOrLoginToken(project.dir, cli)
    await preflightDeployLimits([project], cli, token, cli.database)
    const result = await deployProject(project.dir, cli, token, cli.database)
    if (cli.wait) {
      result.final_build = await waitForBuild(cli, token, result.build_job.id)
    }
    printDeployResult(result, cli)
    return
  }

  const token = await readOrLoginToken(projectDir, cli)
  await preflightDeployLimits(projects, cli, token, cli.database)
  const results = []
  if (!cli.json) {
    console.log(`deploying ${projects.length} projects`)
  }

  for (const project of projects) {
    const wantsDatabase = cli.database && project.kind === 'rust_backend'
    if (!cli.json) {
      console.log(`checking ${project.relative}`)
    }
    const response = await deployProject(project.dir, cli, token, wantsDatabase)
    results.push({ project, wantsDatabase, response })
    if (!cli.json) {
      console.log(`${project.relative} queued ${response.build_job.id}`)
      console.log(`${project.relative} url ${response.app.url}`)
    }
  }

  if (cli.wait) {
    await waitForWorkspaceBuilds(cli, token, results)
  }

  printWorkspaceDeployResults(projectDir, results, cli)
}

async function preflightDeployLimits(projects, cli, token, databaseRequested) {
  const [usageResponse, appsResponse] = await Promise.all([
    apiRequest(cli, 'GET', '/v1/usage', token, null),
    apiRequest(cli, 'GET', '/v1/apps', token, null)
  ])
  const usage = usageResponse?.usage || {}
  const limits = usageResponse?.limits || {}
  const apps = Array.isArray(appsResponse?.apps) ? appsResponse.apps : []
  const existingApps = new Map(apps.map((app) => [app.name, app]))
  let newProjects = 0
  let newDatabases = 0

  for (const project of projects) {
    if (!project.name || project.kind === 'unknown') {
      continue
    }
    const existing = existingApps.get(project.name)
    if (!existing) {
      newProjects += 1
    }
    if (databaseRequested && project.kind === 'rust_backend' && !existing?.databaseStorageMib) {
      newDatabases += 1
    }
  }

  if (newProjects > 0 && Number(usage.appCount) + newProjects > Number(limits.projects)) {
    throw agentError(
      'payment_required',
      `Project limit reached: ${usage.appCount}/${limits.projects} projects are already used.`,
      'Redeploy an existing app by reusing its `name` in zerct.toml, or run `npx @zerct/zerct billing` to open Stripe Checkout before creating another project.',
      cli.json
    )
  }

  if (newDatabases > 0 && Number(usage.databaseCount) + newDatabases > Number(limits.managedDatabases)) {
    throw agentError(
      'payment_required',
      `Managed Postgres limit reached: ${usage.databaseCount}/${limits.managedDatabases} databases are already used.`,
      'Redeploy an app that already has managed Postgres, deploy without `--database`, or run `npx @zerct/zerct billing` to open Stripe Checkout.',
      cli.json
    )
  }
}

async function deployProject(projectDir, cli, token, wantsDatabase) {
  const report = runDoctor(projectDir)
  if (!report.ok) {
    const firstFailure = report.checks.find((check) => !check.ok)
    throw agentError('doctor_failed', 'Zerct doctor failed.', firstFailure?.agent_instruction || 'Fix the failed checks and retry.', cli.json)
  }

  const archive = createArchiveBase64(projectDir)
  const commitSha = gitCommitSha(projectDir)
  const body = {
    config: report.config,
    commit_sha: commitSha,
    wants_database: wantsDatabase,
    source_archive_base64: archive
  }

  return apiRequest(cli, 'POST', '/v1/deploy', token, body)
}

function printDeployResult(response, cli) {
  if (cli.json) {
    console.log(JSON.stringify(response, null, 2))
    return
  }

  console.log(`queued ${response.build_job.id}`)
  console.log(`app ${response.app.id}`)
  console.log(`url ${response.app.url}`)
  console.log(`next npx @zerct/zerct logs --app ${response.app.id}`)
}

function printWorkspaceDeployResults(projectDir, results, cli) {
  if (cli.json) {
    console.log(JSON.stringify({
      workspace: projectDir,
      deploys: results.map((result) => ({
        path: result.project.relative,
        kind: result.project.kind,
        wants_database: result.wantsDatabase,
        app: result.response.app,
        build_job: result.response.build_job,
        final_build: result.finalBuild || null
      }))
    }, null, 2))
    return
  }

  const firstApp = results[0]?.response?.app?.id
  if (firstApp) {
    console.log(`next npx @zerct/zerct logs --app ${firstApp}`)
  }
}

async function waitForWorkspaceBuilds(cli, token, results) {
  await Promise.all(results.map(async (result) => {
    result.finalBuild = await waitForBuild(cli, token, result.response.build_job.id)
  }))
}

async function waitForBuild(cli, token, buildId) {
  const deadline = Date.now() + cli.waitTimeoutSeconds * 1000
  let lastStatus = ''

  while (Date.now() <= deadline) {
    const response = await apiRequest(cli, 'GET', `/v1/builds/${encodeURIComponent(buildId)}`, token, null)
    const build = response.build
    if (!build?.status) {
      throw agentError('build_status_unavailable', 'Build status is unavailable.', `Retry with \`npx @zerct/zerct logs --build ${buildId}\`.`, cli.json)
    }

    if (build.status !== lastStatus) {
      progress(cli, `build ${build.id} ${build.status}`)
      lastStatus = build.status
    }
    if (['succeeded', 'failed', 'canceled'].includes(build.status)) {
      return build
    }
    await sleep(3000)
  }

  throw agentError(
    'build_wait_timeout',
    `Timed out waiting for build ${buildId}.`,
    `Run \`npx @zerct/zerct logs --build ${buildId}\` to continue watching.`,
    cli.json
  )
}

async function logs(cli) {
  const token = await readOrLoginToken(process.cwd(), cli)
  const page = pageQuery(cli)
  let route = ''
  if (cli.build) {
    route = `/v1/builds/${encodeURIComponent(cli.build)}/logs${page}`
  } else if (cli.deploy) {
    route = `/v1/deploys/${encodeURIComponent(cli.deploy)}/logs${page}`
  } else {
    route = `/v1/apps/${encodeURIComponent(requireApp(cli))}/logs${page}`
  }
  const response = await apiRequest(cli, 'GET', route, token, null)
  if (cli.json) {
    console.log(JSON.stringify(response, null, 2))
    return
  }
  for (const line of response.lines || []) {
    console.log(`[${line.timestamp}] ${line.stream}: ${line.message}`)
  }
  if (response.has_more && response.next_cursor) {
    const target = cli.build
      ? `--build ${cli.build}`
      : cli.deploy
        ? `--deploy ${cli.deploy}`
        : `--app ${requireApp(cli)}`
    console.log(`next npx @zerct/zerct logs ${target} --cursor ${response.next_cursor}`)
  }
}

async function apps(cli) {
  const token = await readOrLoginToken(process.cwd(), cli)
  const response = await apiRequest(cli, 'GET', '/v1/apps', token, null)
  printJsonOrPretty(cli, response)
}

async function capabilities(cli) {
  const response = await apiRequest(cli, 'GET', '/v1/capabilities', null, null)
  printJsonOrPretty(cli, response)
}

async function me(cli) {
  const token = await readOrLoginToken(process.cwd(), cli)
  const response = await apiRequest(cli, 'GET', '/v1/me', token, null)
  printJsonOrPretty(cli, response)
}

async function usage(cli) {
  const token = await readOrLoginToken(process.cwd(), cli)
  const response = await apiRequest(cli, 'GET', '/v1/usage', token, null)
  printJsonOrPretty(cli, response)
}

async function activity(cli) {
  const token = await readOrLoginToken(process.cwd(), cli)
  const response = await apiRequest(cli, 'GET', `/v1/activity${pageQuery(cli)}`, token, null)
  printJsonOrPretty(cli, response)
}

async function overview(cli) {
  const token = await readOrLoginToken(process.cwd(), cli)
  const app = requireApp(cli)
  const response = await apiRequest(cli, 'GET', `/v1/apps/${encodeURIComponent(app)}/overview${pageQuery(cli)}`, token, null)
  printJsonOrPretty(cli, response)
}

async function deploys(cli) {
  const token = await readOrLoginToken(process.cwd(), cli)
  const route = cli.app
    ? `/v1/apps/${encodeURIComponent(cli.app)}/deploys${pageQuery(cli)}`
    : `/v1/deploys${pageQuery(cli)}`
  const response = await apiRequest(cli, 'GET', route, token, null)
  printJsonOrPretty(cli, response)
}

async function builds(cli) {
  const token = await readOrLoginToken(process.cwd(), cli)
  const route = cli.app
    ? `/v1/apps/${encodeURIComponent(cli.app)}/builds${pageQuery(cli)}`
    : `/v1/builds${pageQuery(cli)}`
  const response = await apiRequest(cli, 'GET', route, token, null)
  printJsonOrPretty(cli, response)
}

async function status(cli) {
  const response = await appGet(cli, 'status')
  printJsonOrPretty(cli, response)
}

async function inspect(cli) {
  const response = await appGet(cli, 'inspect')
  printJsonOrPretty(cli, response)
}

async function database(cli) {
  const response = await appGet(cli, 'database')
  printJsonOrPretty(cli, response)
}

async function envCommand(cli) {
  if (cli.args[0] === 'list') {
    const response = await appGet(cli, 'env')
    printJsonOrPretty(cli, response)
    return
  }

  if (cli.args[0] === 'delete') {
    const name = cli.args[1] || ''
    if (!name) {
      throw agentError('invalid_env', 'Environment variable name is required.', 'Use `npx @zerct/zerct env delete --app <app> KEY`.', cli.json)
    }
    const token = await readOrLoginToken(process.cwd(), cli)
    const app = requireApp(cli)
    const response = await apiRequest(cli, 'DELETE', `/v1/apps/${encodeURIComponent(app)}/env/${encodeURIComponent(name)}`, token, null)
    printJsonOrPretty(cli, response)
    return
  }

  if (cli.args[0] !== 'set') {
    throw agentError('unknown_command', 'Unknown env command.', 'Use `npx @zerct/zerct env list`, `env set`, or `env delete`.', cli.json)
  }

  const assignment = cli.args[1] || ''
  const separator = assignment.indexOf('=')
  if (separator <= 0) {
    throw agentError('invalid_env', 'Environment assignment must be KEY=value.', 'Pass one uppercase shell-safe environment assignment, for example `API_KEY=value`.', cli.json)
  }

  const name = assignment.slice(0, separator)
  const value = assignment.slice(separator + 1)
  const token = await readOrLoginToken(process.cwd(), cli)
  const app = requireApp(cli)
  const response = await apiRequest(cli, 'PUT', `/v1/apps/${encodeURIComponent(app)}/env`, token, { name, value })
  printJsonOrPretty(cli, response)
}

async function domainsCommand(cli) {
  const action = cli.args[0] || 'list'
  if (action === 'list') {
    const response = await appGet(cli, 'domains')
    printJsonOrPretty(cli, response)
    return
  }

  const domain = cli.args[1] || ''
  if (!domain) {
    throw agentError('missing_domain', 'Domain is required.', 'Use `npx @zerct/zerct domains add --app <app> api.example.com`.', cli.json)
  }

  const token = await readOrLoginToken(process.cwd(), cli)
  const app = requireApp(cli)
  if (action === 'add') {
    const response = await apiRequest(cli, 'POST', `/v1/apps/${encodeURIComponent(app)}/domains`, token, { domain })
    printJsonOrPretty(cli, response)
    return
  }
  if (action === 'verify') {
    const response = await apiRequest(cli, 'POST', `/v1/apps/${encodeURIComponent(app)}/domains/${encodeURIComponent(domain)}/verify`, token, null)
    printJsonOrPretty(cli, response)
    return
  }
  if (action === 'delete') {
    const response = await apiRequest(cli, 'DELETE', `/v1/apps/${encodeURIComponent(app)}/domains/${encodeURIComponent(domain)}`, token, null)
    printJsonOrPretty(cli, response)
    return
  }

  throw agentError('unknown_command', 'Unknown domains command.', 'Use `domains list`, `domains add`, `domains verify`, or `domains delete`.', cli.json)
}

async function billing(cli) {
  const token = await readOrLoginToken(process.cwd(), cli)
  if (cli.args[0] === 'portal') {
    const response = await apiRequest(cli, 'POST', '/v1/billing/portal', token, null)
    if (cli.json) {
      console.log(JSON.stringify(response, null, 2))
      return
    }
    console.log(response.checkout.url)
    openUrl(response.checkout.url)
    return
  }

  const response = await apiRequest(cli, 'POST', '/v1/billing/checkout', token, {
    target_plan: 'pro',
    reason: 'Upgrade to Zerct Pro.'
  })
  if (cli.json) {
    console.log(JSON.stringify(response, null, 2))
    return
  }
  console.log(response.checkout.url)
  openUrl(response.checkout.url)
}

async function appGet(cli, kind) {
  const token = await readOrLoginToken(process.cwd(), cli)
  const app = requireApp(cli)
  return apiRequest(cli, 'GET', `/v1/apps/${encodeURIComponent(app)}/${kind}`, token, null)
}

async function readOrLoginToken(projectDir, cli) {
  const token = readStoredToken(projectDir, cli)
  if (token) {
    return token
  }

  return loginAndStore(cli)
}

async function loginAndStore(cli) {
  const start = await apiRequest(cli, 'POST', '/v1/login/device', null, null)
  const loginUrl = start.loginUrl || start.login_url
  if (!loginUrl) {
    throw agentError('login_failed', 'Zerct login did not return a browser URL.', 'Retry `npx @zerct/zerct login`. If it keeps failing, check Zerct status.', cli.json)
  }
  openUrl(loginUrl)
  progress(cli, 'opened browser login')
  progress(cli, `waiting for browser login code ${start.userCode || start.user_code || 'ZERCT'}`)

  const session = await pollLogin(cli, start)
  if (!session.token) {
    throw agentError('login_failed', 'Zerct login did not return a session token.', 'Run `npx @zerct/zerct login` again and complete the browser login.', cli.json)
  }

  writeSessionToken(session.token)
  progress(cli, `logged in as ${session.email || 'Zerct user'}`)
  return session.token
}

async function pollLogin(cli, start) {
  const deviceCode = start.deviceCode || start.device_code
  if (!deviceCode) {
    throw agentError('login_failed', 'Zerct login did not return a device code.', 'Retry `npx @zerct/zerct login`. If it keeps failing, check Zerct status.', cli.json)
  }

  const expiresMs = Number(start.expiresInSeconds || start.expires_in_seconds || DEFAULT_LOGIN_EXPIRES_SECONDS) * 1000
  const deadline = Date.now() + expiresMs
  let intervalMs = Number(start.intervalSeconds || start.interval_seconds || DEFAULT_LOGIN_INTERVAL_SECONDS) * 1000

  while (Date.now() < deadline) {
    await sleep(intervalMs)
    const response = await apiRequest(cli, 'GET', `/v1/login/device/${encodeURIComponent(deviceCode)}`, null, null)
    if (response.status === 'complete') {
      return response
    }
    if (response.status === 'expired') {
      throw agentError('login_expired', 'Zerct login expired before it completed.', 'Run `npx @zerct/zerct login` again and finish the browser login in the newly opened tab.', cli.json)
    }
    intervalMs = Math.max(
      DEFAULT_LOGIN_INTERVAL_SECONDS * 1000,
      Number(response.intervalSeconds || response.interval_seconds || DEFAULT_LOGIN_INTERVAL_SECONDS) * 1000
    )
  }

  throw agentError('login_expired', 'Zerct login expired before it completed.', 'Run `npx @zerct/zerct login` again and finish the browser login in the newly opened tab.', cli.json)
}

function requireApp(cli) {
  if (!cli.app) {
    throw agentError('missing_app', 'App is required.', 'Pass `--app <app>` using either the app name from zerct.toml or the app id printed by deploy.', cli.json)
  }
  return cli.app
}

function pageQuery(cli) {
  const params = new URLSearchParams()
  if (cli.limit) {
    params.set('limit', cli.limit)
  }
  if (cli.cursor) {
    params.set('cursor', cli.cursor)
  }
  const value = params.toString()
  return value ? `?${value}` : ''
}

async function apiRequest(cli, method, route, token, body) {
  const headers = {
    accept: 'application/json'
  }
  if (token) {
    headers.authorization = `Bearer ${token}`
  }
  if (body !== null) {
    headers['content-type'] = 'application/json'
  }

  const response = await fetch(`${cli.apiUrl}${route}`, {
    method,
    headers,
    body: body === null ? undefined : JSON.stringify(body)
  })

  const text = await response.text()
  const data = parseJson(text)
  if (!response.ok) {
    const payload = data && typeof data === 'object' ? data : {
      code: 'api_error',
      message: `Zerct API returned HTTP ${response.status}.`,
      agent_instruction: 'Retry the command. If it keeps failing, check Zerct status before changing your project.',
      docs_url: null,
      checkout_url: null
    }
    throw new ZerctError(payload, cli.json, response.status >= 500 ? 2 : 1)
  }

  return data
}

function parseJson(text) {
  if (!text.trim()) {
    return null
  }
  try {
    return JSON.parse(text)
  } catch (_error) {
    return null
  }
}

function createArchiveBase64(projectDir) {
  const excludeArgs = ARCHIVE_EXCLUDES.map((pattern) => `--exclude=${pattern}`)
  const tar = spawnSync('tar', [...excludeArgs, '-czf', '-', '-C', projectDir, '.'], {
    encoding: 'buffer',
    env: { ...process.env, COPYFILE_DISABLE: '1' },
    maxBuffer: ARCHIVE_LIMIT_BYTES + 1024 * 1024
  })

  if (tar.error) {
    throw agentError('archive_failed', 'Could not create source archive.', 'Install `tar`, remove local build outputs, then retry `npx @zerct/zerct deploy`.', false)
  }
  if (tar.status !== 0) {
    throw agentError('archive_failed', 'Could not create source archive.', String(tar.stderr || 'Check project files and retry.'), false)
  }
  if (tar.stdout.length > ARCHIVE_LIMIT_BYTES) {
    throw agentError('archive_too_large', 'Source archive is too large.', 'Remove build outputs, target directories, logs, and local caches before deploying.', false)
  }

  return tar.stdout.toString('base64')
}

function gitCommitSha(projectDir) {
  const git = spawnSync('git', ['rev-parse', 'HEAD'], {
    cwd: projectDir,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'ignore']
  })
  return git.status === 0 ? git.stdout.trim() || null : null
}

function readStoredToken(projectDir, cli) {
  if (cli.token) {
    return cli.token
  }
  if (process.env.ZERCT_TOKEN) {
    return process.env.ZERCT_TOKEN
  }

  const keychainToken = readKeychainToken()
  if (keychainToken) {
    return keychainToken
  }

  const userToken = readTokenFile(userSessionPath())
  if (userToken) {
    return userToken
  }

  const projectToken = path.join(projectDir, SESSION_DIR, SESSION_FILE)
  const legacyProjectToken = readTokenFile(projectToken)
  if (legacyProjectToken) {
    return legacyProjectToken
  }

  const homeToken = path.join(homedir(), SESSION_DIR, SESSION_FILE)
  return readTokenFile(homeToken)
}

function writeSessionToken(token) {
  const cleanToken = token.trim()
  if (!cleanToken) {
    throw agentError('login_failed', 'Zerct session token is empty.', 'Run `npx @zerct/zerct login` again and complete the browser login.', false)
  }
  if (writeKeychainToken(cleanToken)) {
    return
  }

  writeTokenFile(userSessionPath(), cleanToken)
}

function readTokenFile(filePath) {
  if (!existsSync(filePath)) {
    return ''
  }
  return readFileSync(filePath, 'utf8').trim()
}

function writeTokenFile(filePath, token) {
  const dir = path.dirname(filePath)
  mkdirSync(dir, { recursive: true, mode: 0o700 })
  writeFileSync(filePath, `${token}\n`, { mode: 0o600 })
}

function userSessionPath() {
  if (process.platform === 'win32' && process.env.APPDATA) {
    return path.join(process.env.APPDATA, 'Zerct', SESSION_FILE)
  }
  const configHome = process.env.XDG_CONFIG_HOME || path.join(homedir(), '.config')
  return path.join(configHome, 'zerct', SESSION_FILE)
}

function readKeychainToken() {
  if (process.platform === 'darwin') {
    const result = spawnSync('security', ['find-generic-password', '-s', SESSION_SERVICE, '-a', SESSION_ACCOUNT, '-w'], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore']
    })
    return result.status === 0 ? result.stdout.trim() : ''
  }

  if (process.platform === 'linux' && hasCommand('secret-tool')) {
    const result = spawnSync('secret-tool', ['lookup', 'service', SESSION_SERVICE, 'account', SESSION_ACCOUNT], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore']
    })
    return result.status === 0 ? result.stdout.trim() : ''
  }

  return ''
}

function writeKeychainToken(token) {
  if (process.platform === 'darwin') {
    const result = spawnSync('security', [
      'add-generic-password',
      '-U',
      '-s',
      SESSION_SERVICE,
      '-a',
      SESSION_ACCOUNT,
      '-l',
      SESSION_LABEL,
      '-w',
      token
    ], { stdio: 'ignore' })
    return result.status === 0
  }

  if (process.platform === 'linux' && hasCommand('secret-tool')) {
    const result = spawnSync('secret-tool', [
      'store',
      '--label',
      SESSION_LABEL,
      'service',
      SESSION_SERVICE,
      'account',
      SESSION_ACCOUNT
    ], {
      input: token,
      stdio: ['pipe', 'ignore', 'ignore']
    })
    return result.status === 0
  }

  return false
}

function hasCommand(command) {
  return (process.env.PATH || '')
    .split(path.delimiter)
    .filter(Boolean)
    .some((directory) => existsSync(path.join(directory, command)))
}

function parseZerctToml(source, projectDir) {
  const config = {
    build: {},
    run: {},
    resources: {}
  }
  let section = config

  for (const rawLine of source.split(/\r?\n/u)) {
    const line = rawLine.trim()
    if (!line || line.startsWith('#')) {
      continue
    }

    const sectionMatch = line.match(/^\[([a-z_]+)\]$/u)
    if (sectionMatch) {
      const name = sectionMatch[1]
      if (!['build', 'run', 'resources'].includes(name)) {
        throw new Error(`unsupported section [${name}]`)
      }
      section = config[name]
      continue
    }

    const assignment = line.match(/^([a-z_]+)\s*=\s*(.+)$/u)
    if (!assignment) {
      throw new Error(`invalid line: ${line}`)
    }

    section[assignment[1]] = parseTomlValue(assignment[2])
  }

  config.kind ||= 'rust_backend'
  config.build.check ||= config.kind === 'static_frontend' ? frontendCheckCommand(projectDir) : DEFAULT_RUST_CHECK_COMMAND
  config.build.command ||= config.kind === 'static_frontend' ? frontendBuildCommand(projectDir) : 'cargo build --release'
  if (config.kind === 'static_frontend') {
    config.build.output ||= 'dist'
  }
  config.run.port ||= 3000
  config.run.health ||= '/healthz'
  config.resources.memory ||= '512mb'
  config.resources.cpu ||= '0.25'
  config.resources.idle_timeout_minutes ||= 15
  return config
}

function parseTomlValue(raw) {
  const value = raw.trim()
  if (value.startsWith('"') && value.endsWith('"')) {
    return value.slice(1, -1).replace(/\\"/gu, '"')
  }
  if (value === 'true') {
    return true
  }
  if (value === 'false') {
    return false
  }
  if (/^\d+$/u.test(value)) {
    return Number(value)
  }
  throw new Error(`unsupported TOML value: ${value}`)
}

function validateConfig(config) {
  if (!/^[a-z0-9](?:[a-z0-9-]{0,46}[a-z0-9])?$/u.test(config.name || '')) {
    throw new Error('name must be lowercase DNS-safe text up to 48 characters')
  }
  if (!PROJECT_KINDS.has(config.kind)) {
    throw new Error('kind must be rust_backend or static_frontend')
  }
  if (typeof config.build.command !== 'string' || !config.build.command.trim()) {
    throw new Error('[build].command is required')
  }
  if (typeof config.build.check !== 'string' || !config.build.check.trim()) {
    throw new Error('[build].check is required')
  }
  validateCheckCommand(config.kind, config.build.check)
  if (config.kind === 'static_frontend') {
    if (typeof config.build.output !== 'string' || !isSafeRelativePath(config.build.output)) {
      throw new Error('[build].output must be a safe relative directory like dist')
    }
    return
  }
  if (config.build.output) {
    throw new Error('[build].output is only valid for static_frontend')
  }
  if (!config.run.command || typeof config.run.command !== 'string') {
    throw new Error('[run].command is required')
  }
  if (!Number.isInteger(config.run.port) || config.run.port < 1 || config.run.port > 65535) {
    throw new Error('[run].port must be between 1 and 65535')
  }
  if (typeof config.run.health !== 'string' || !config.run.health.startsWith('/')) {
    throw new Error('[run].health must be an absolute path')
  }
  if (!/^\d+\s*(mb|mib|gb|gib)$/iu.test(config.resources.memory)) {
    throw new Error('[resources].memory must look like 512mb or 1gb')
  }
  if (!/^\d+(?:\.\d{1,3})?$/u.test(config.resources.cpu)) {
    throw new Error('[resources].cpu must look like 0.25, 0.5, 1, or 2')
  }
}

function validateCheckCommand(kind, command) {
  if (kind === 'static_frontend') {
    validateFrontendCheckCommand(command)
    return
  }

  const required = ['cargo fmt --all --check', 'cargo check --locked', 'cargo clippy --locked', '--all-targets', '--all-features', '-D warnings']
  if (required.every((fragment) => command.includes(fragment))) {
    return
  }
  throw new Error('[build].check must include cargo fmt --all --check, cargo check --locked, and cargo clippy --locked --all-targets --all-features -- -D warnings')
}

function validateFrontendCheckCommand(command) {
  if (usesJavascriptLinter(command)) {
    throw new Error('[build].check must not run JavaScript-based linters; use oxlint, biome, or deno lint')
  }
  const tokens = commandTokens(command)
  if (
    hasFrontendInstallCommand(tokens) &&
    hasFrontendScriptRun(tokens, 'typecheck') &&
    hasFrontendScriptRun(tokens, 'lint')
  ) {
    return
  }
  throw new Error('[build].check must install dependencies and run package scripts, for example `bun ci && bun run typecheck && bun run lint` or `npm ci --prefer-offline --no-audit --fund=false && npm run typecheck && npm run lint`')
}

function frontendLockfileExists(projectDir) {
  return ['package-lock.json', 'npm-shrinkwrap.json', 'pnpm-lock.yaml', 'yarn.lock', 'bun.lock', 'bun.lockb']
    .some((file) => existsSync(path.join(projectDir, file)))
}

function frontendPackageManager(projectDir) {
  return existsSync(path.join(projectDir, 'bun.lock')) || existsSync(path.join(projectDir, 'bun.lockb'))
    ? 'bun'
    : 'npm'
}

function frontendCheckCommand(projectDir) {
  return frontendPackageManager(projectDir) === 'bun'
    ? DEFAULT_BUN_FRONTEND_CHECK_COMMAND
    : DEFAULT_NPM_FRONTEND_CHECK_COMMAND
}

function frontendBuildCommand(projectDir) {
  return frontendPackageManager(projectDir) === 'bun'
    ? 'bun run build'
    : 'npm run build'
}

function frontendScriptChecks(projectDir, runScripts) {
  const manifest = readPackageJson(projectDir)
  const missing = (script) => !manifest?.scripts || typeof manifest.scripts[script] !== 'string' || !manifest.scripts[script].trim()
  const checks = ['typecheck', 'lint'].map((script) => ({
    name: `package script ${script}`,
    ok: !missing(script),
    message: missing(script) ? 'missing' : 'found',
    agent_instruction: `Add a non-empty "${script}" script to package.json, then retry.`
  }))
  const lintScript = manifest?.scripts?.lint || ''
  const nativeLint = !lintScript || !usesJavascriptLinter(lintScript)
  checks.push({
    name: 'native frontend lint',
    ok: nativeLint,
    message: nativeLint ? 'accepted' : 'JavaScript linter found',
    agent_instruction: 'Replace the lint script with native tooling such as `oxlint src vite.config.ts --deny-warnings`, `biome check .`, or `deno lint`, then retry.'
  })

  if (runScripts && checks.every((check) => check.ok)) {
    checks.push(packageScriptCheck(projectDir, 'typecheck'))
    checks.push(packageScriptCheck(projectDir, 'lint'))
  }

  return checks
}

function frontendSourceChecks(projectDir) {
  const report = frontendSourceReport(projectDir)
  return [
    {
      name: 'typescript source',
      ok: report.typescript.length > 0,
      message: report.typescript.length > 0 ? report.typescript.slice(0, 3).join(', ') : 'missing',
      agent_instruction: 'Add browser source as .ts or .tsx under src, app, pages, routes, or components, then retry.'
    },
    {
      name: 'javascript source',
      ok: report.javascript.length === 0,
      message: report.javascript.length === 0 ? 'none found' : report.javascript.slice(0, 5).join(', '),
      agent_instruction: 'Rename browser .js, .jsx, .mjs, or .cjs source files to .ts or .tsx and fix type errors before deploying.'
    }
  ]
}

function frontendSourceReport(projectDir) {
  const report = { typescript: [], javascript: [] }
  walkProjectFiles(projectDir, (file, relative) => {
    if (!isFrontendSourcePath(relative)) {
      return
    }
    if (isFrontendTypescriptSource(relative)) {
      report.typescript.push(relative)
    } else if (isFrontendJavascriptSource(relative)) {
      report.javascript.push(relative)
    }
  })
  return report
}

function isFrontendSourcePath(relative) {
  const [root] = relative.split('/')
  return ['src', 'app', 'pages', 'routes', 'components'].includes(root)
}

function isFrontendTypescriptSource(relative) {
  return !relative.endsWith('.d.ts') && (relative.endsWith('.ts') || relative.endsWith('.tsx'))
}

function isFrontendJavascriptSource(relative) {
  return ['.js', '.jsx', '.mjs', '.cjs'].some((extension) => relative.endsWith(extension))
}

function readPackageJson(projectDir) {
  try {
    return JSON.parse(readFileSync(path.join(projectDir, 'package.json'), 'utf8'))
  } catch (_error) {
    return null
  }
}

function usesJavascriptLinter(command) {
  const tokens = commandTokens(command)
  return tokens.some((token, index) => {
    const commandName = commandNameFromToken(token)
    return ['eslint', 'eslint_d', 'standard', 'xo'].includes(commandName)
      || (commandName === 'next' && tokens[index + 1] === 'lint')
  })
}

function commandTokens(command) {
  return command
    .replace(/[&|;()]/gu, ' ')
    .split(/\s+/u)
    .map((token) => token.trim().replace(/^["']|["']$/gu, ''))
    .filter(Boolean)
}

function commandNameFromToken(token) {
  return token.split('/').pop()
}

function hasFrontendInstallCommand(tokens) {
  const accepted = new Set(['npm ci', 'bun ci', 'bun install', 'pnpm install', 'yarn install'])
  return tokens.some((token, index) => accepted.has(`${commandNameFromToken(token)} ${tokens[index + 1] || ''}`))
}

function hasFrontendScriptRun(tokens, script) {
  const managers = new Set(['npm', 'bun', 'pnpm', 'yarn'])
  return tokens.some((token, index) => {
    if (!managers.has(commandNameFromToken(token)) || tokens[index + 1] !== 'run') {
      return false
    }
    return tokens[index + 2] === script || (tokens[index + 2] || '').startsWith('-') && tokens[index + 3] === script
  })
}

function packageScriptCheck(projectDir, script) {
  const manager = frontendPackageManager(projectDir)
  const args = manager === 'bun' ? ['run', script] : ['run', '--silent', script]
  const result = spawnSync(manager, args, {
    cwd: projectDir,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe']
  })
  if (result.error) {
    return {
      name: `${manager} run ${script}`,
      ok: false,
      message: result.error.message,
      agent_instruction: `Install ${manager === 'bun' ? 'Bun' : 'Node.js and npm'}, then run \`${manager} run ${script}\` before deploying.`
    }
  }

  return {
    name: `${manager} run ${script}`,
    ok: result.status === 0,
    message: result.status === 0 ? 'passed' : (result.stderr || result.stdout || `${manager} run ${script} failed`).trim().slice(0, 240),
    agent_instruction: `Run \`${manager} run ${script}\`, fix every error, then redeploy.`
  }
}

function discoverDeployProjects(rootDir) {
  ensureDirectory(rootDir)
  if (existsSync(path.join(rootDir, 'zerct.toml'))) {
    return [deployProjectInfo(rootDir, rootDir)]
  }

  const projectDirs = []
  discoverProjectDirs(rootDir, projectDirs)
  return projectDirs
    .map((dir) => deployProjectInfo(dir, rootDir))
    .sort(compareDeployProjects)
}

function discoverProjectDirs(dir, projectDirs) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (!entry.isDirectory() || WORKSPACE_EXCLUDED_DIRS.has(entry.name)) {
      continue
    }

    const child = path.join(dir, entry.name)
    if (existsSync(path.join(child, 'zerct.toml'))) {
      projectDirs.push(child)
      continue
    }
    discoverProjectDirs(child, projectDirs)
  }
}

function deployProjectInfo(dir, rootDir) {
  const relative = path.relative(rootDir, dir).replace(/\\/gu, '/') || '.'
  try {
    const config = parseZerctToml(readFileSync(path.join(dir, 'zerct.toml'), 'utf8'), dir)
    return { dir, relative, name: config.name || '', kind: config.kind }
  } catch (_error) {
    return { dir, relative, name: '', kind: 'unknown' }
  }
}

function compareDeployProjects(left, right) {
  return kindOrder(left.kind) - kindOrder(right.kind)
    || left.relative.localeCompare(right.relative)
}

function kindOrder(kind) {
  if (kind === 'rust_backend') {
    return 0
  }
  if (kind === 'static_frontend') {
    return 1
  }
  return 2
}

function isSafeRelativePath(value) {
  return value
    && !path.isAbsolute(value)
    && !value.includes('\\')
    && value.split('/').every((part) => part && part !== '.' && part !== '..')
}

function scanUnsafe(projectDir) {
  const hits = []
  walkProjectFiles(projectDir, (file, relative) => {
    if (!file.endsWith('.rs')) {
      return
    }
    const source = readFileSync(file, 'utf8')
    if (/\bunsafe\b/u.test(source)) {
      hits.push(relative)
    }
  })
  return hits
}

function walkProjectFiles(projectDir, visit) {
  walk(projectDir, (file) => {
    visit(file, path.relative(projectDir, file).replace(/\\/gu, '/'))
  })
}

function walk(dir, visit) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (WALK_EXCLUDED_DIRS.has(entry.name)) {
      continue
    }
    const fullPath = path.join(dir, entry.name)
    if (entry.isDirectory()) {
      walk(fullPath, visit)
    } else if (entry.isFile()) {
      visit(fullPath)
    }
  }
}

function ensureDirectory(dir) {
  if (!existsSync(dir) || !statSync(dir).isDirectory()) {
    throw agentError('missing_project', 'Project directory does not exist.', 'Run Zerct from the root of a Rust project or pass the project path.', false)
  }
}

function serviceNameFromDir(projectDir) {
  const name = serviceNameFromValue(path.basename(projectDir))
  return name || 'api'
}

function serviceNameFromCargo(projectDir) {
  try {
    const source = readFileSync(path.join(projectDir, 'Cargo.toml'), 'utf8')
    return serviceNameFromValue(source.match(/^\s*name\s*=\s*"([^"]+)"/mu)?.[1] || '')
  } catch (_error) {
    return ''
  }
}

function serviceNameFromPackage(projectDir) {
  const manifest = readPackageJson(projectDir)
  return serviceNameFromValue(typeof manifest?.name === 'string' ? manifest.name : '')
}

function serviceNameFromValue(value) {
  return value.toLowerCase().replace(/[^a-z0-9-]+/gu, '-').replace(/^-+|-+$/gu, '').slice(0, 48)
}

function inferProjectKind(projectDir) {
  if (existsSync(path.join(projectDir, 'Cargo.toml'))) {
    return 'rust_backend'
  }
  if (existsSync(path.join(projectDir, 'package.json'))) {
    return 'static_frontend'
  }
  return 'rust_backend'
}

function rustApiSource() {
  return `use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};

fn main() -> std::io::Result<()> {
    let port = std::env::var("PORT").unwrap_or_else(|_error| "3000".to_owned());
    let listener = TcpListener::bind(format!("0.0.0.0:{port}"))?;

    for stream in listener.incoming() {
        handle(stream?)?;
    }

    Ok(())
}

fn handle(mut stream: TcpStream) -> std::io::Result<()> {
    let mut buffer = [0_u8; 2048];
    let size = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..size]);
    let mut parts = request
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace();
    let method = parts.next().unwrap_or_default();
    let path = parts.next().unwrap_or("/");
    let origin = request
        .lines()
        .find_map(|line| line.strip_prefix("Origin: "))
        .unwrap_or("*");
    let cors_origin = allowed_origin(origin);

    if method == "OPTIONS" {
        return write_response(&mut stream, "204 No Content", "", &cors_origin);
    }

    let body = if path == "/healthz" {
        r#"{"ok":true}"#
    } else {
        r#"{"message":"hello from zerct","backend":"rust"}"#
    };
    write_response(&mut stream, "200 OK", body, &cors_origin)
}

fn allowed_origin(request_origin: &str) -> String {
    let configured = std::env::var("FRONTEND_ORIGIN").unwrap_or_else(|_error| request_origin.to_owned());
    if configured == "*" || configured == request_origin {
        configured
    } else {
        "null".to_owned()
    }
}

fn write_response(
    stream: &mut TcpStream,
    status: &str,
    body: &str,
    origin: &str,
) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\\r\\ncontent-type: application/json\\r\\ncontent-length: {}\\r\\naccess-control-allow-origin: {origin}\\r\\naccess-control-allow-methods: GET, OPTIONS\\r\\naccess-control-allow-headers: content-type, authorization\\r\\nconnection: close\\r\\n\\r\\n{body}",
        body.len()
    )
}
`
}

function frontendSource(apiBaseUrl) {
  return `import { createRootRoute, createRouter, RouterProvider } from '@tanstack/react-router'
import { createRoot } from 'react-dom/client'
import './styles.css'

const apiBaseUrl = import.meta.env.VITE_API_URL ?? '${apiBaseUrl}'

function App() {
  return (
    <main>
      <section>
        <h1>Zerct TanStack Frontend</h1>
        <p>Static runtime, dynamic Rust backend calls.</p>
        <code>{apiBaseUrl}</code>
      </section>
    </main>
  )
}

const rootRoute = createRootRoute({ component: App })
const router = createRouter({ routeTree: rootRoute })

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}

createRoot(document.getElementById('root')!).render(<RouterProvider router={router} />)
`
}

function printJsonOrPretty(cli, value) {
  console.log(JSON.stringify(value, null, cli.json ? 2 : 2))
}

function openUrl(url) {
  const command = process.platform === 'darwin' ? 'open' : process.platform === 'win32' ? 'cmd' : 'xdg-open'
  const args = process.platform === 'win32' ? ['/c', 'start', '', url] : [url]
  spawnSync(command, args, { stdio: 'ignore', detached: true })
}

function sleep(milliseconds) {
  return new Promise((resolve) => {
    setTimeout(resolve, milliseconds)
  })
}

function progress(cli, message) {
  if (cli.json) {
    console.error(message)
    return
  }
  console.log(message)
}

function trimTrailingSlash(value) {
  return value.replace(/\/+$/u, '')
}

function agentError(code, message, agentInstruction, json) {
  return new ZerctError({
    code,
    message,
    agent_instruction: agentInstruction,
    docs_url: null,
    checkout_url: null
  }, json, 1)
}

function printAgentError(payload, json) {
  if (json) {
    console.error(JSON.stringify(payload, null, 2))
    return
  }

  console.error(payload.message || 'Zerct command failed.')
  if (payload.agent_instruction) {
    console.error(`agent_instruction: ${payload.agent_instruction}`)
  }
  if (payload.docs_url) {
    console.error(`docs: ${payload.docs_url}`)
  }
  if (payload.checkout_url) {
    console.error(`checkout: ${payload.checkout_url}`)
  }
}

class ZerctError extends Error {
  constructor(payload, json, exitCode) {
    super(payload.message || 'Zerct command failed.')
    this.payload = payload
    this.json = json
    this.exitCode = exitCode
  }
}

main().catch((error) => {
  if (error instanceof ZerctError) {
    printAgentError(error.payload, error.json)
    process.exitCode = error.exitCode
    return
  }

  console.error(`zerct failed: ${error.message}`)
  process.exitCode = 1
})
