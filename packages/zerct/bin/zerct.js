#!/usr/bin/env node
import { spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs'
import { homedir } from 'node:os'
import path from 'node:path'

const VERSION = '0.1.2'
const DEFAULT_API_URL = 'https://api.zerct.com'
const ARCHIVE_LIMIT_BYTES = 48 * 1024 * 1024
const SESSION_DIR = '.zerct'
const SESSION_FILE = 'session-token'
const SESSION_SERVICE = 'com.zerct.cli'
const SESSION_ACCOUNT = 'session-token'
const SESSION_LABEL = 'Zerct session'
const DEFAULT_LOGIN_EXPIRES_SECONDS = 600
const DEFAULT_LOGIN_INTERVAL_SECONDS = 5
const DEFAULT_RUST_CHECK_COMMAND = 'cargo check --locked && cargo clippy --locked --all-targets --all-features -- -D warnings'
const DEFAULT_FRONTEND_CHECK_COMMAND = 'npm run typecheck && npm run lint'
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

const HELP = `Zerct ${VERSION}

Usage:
  zerct init [path]
  zerct install [path]
  zerct doctor [path] [--json]
  zerct login [--token <token>] [--api <url>]
  zerct deploy [path] [--database] [--api <url>] [--json]
  zerct logs --app <app_id> [--api <url>] [--json]
  zerct status --app <app_id> [--api <url>] [--json]
  zerct inspect --app <app_id> [--api <url>] [--json]
  zerct db --app <app_id> [--api <url>] [--json]
  zerct env set --app <app_id> KEY=value [--api <url>] [--json]
  zerct billing [--api <url>] [--json]

Agent contract:
  - Rust backends keep Cargo.lock committed, listen on 0.0.0.0:$PORT, and return HTTP 200 from health.
  - Static frontends set kind = "static_frontend", keep a package lockfile, and expose typecheck + lint scripts.
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
      initProject(projectPath(cli.args[0]))
      break
    case 'install':
      installProject(projectPath(cli.args[0]))
      break
    case 'doctor':
      doctorProject(projectPath(cli.args[0]), cli.json)
      break
    case 'login':
      await login(cli)
      break
    case 'deploy':
      await deploy(projectPath(cli.args[0]), cli)
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
    token: '',
    json: false,
    database: false,
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
    } else if (arg === '--api') {
      cli.apiUrl = requireValue(argv, index, '--api')
      index += 1
    } else if (arg === '--app') {
      cli.app = requireValue(argv, index, '--app')
      index += 1
    } else if (arg === '--token') {
      cli.token = requireValue(argv, index, '--token')
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

function initProject(projectDir) {
  ensureDirectory(projectDir)
  const configPath = path.join(projectDir, 'zerct.toml')
  if (existsSync(configPath)) {
    console.log('zerct.toml already exists')
    return
  }

  const name = serviceNameFromDir(projectDir)
  const source = `name = "${name}"

[build]
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

  writeFileSync(configPath, source, { mode: 0o644 })
  console.log(`created ${path.relative(process.cwd(), configPath)}`)
}

function installProject(projectDir) {
  initProject(projectDir)
  doctorProject(projectDir, false)
}

function doctorProject(projectDir, json) {
  const report = runDoctor(projectDir)
  if (json) {
    console.log(JSON.stringify(report, null, 2))
    if (!report.ok) {
      process.exitCode = 1
    }
    return
  }

  for (const check of report.checks) {
    console.log(`${check.ok ? 'ok' : 'fail'} ${check.name}${check.message ? ` - ${check.message}` : ''}`)
  }

  if (!report.ok) {
    const firstFailure = report.checks.find((check) => !check.ok)
    throw agentError('doctor_failed', 'Zerct doctor failed.', firstFailure?.agent_instruction || 'Fix the failed checks and retry `npx @zerct/zerct doctor`.', json)
  }
}

function runDoctor(projectDir) {
  const checks = []
  let config = null
  const configPath = path.join(projectDir, 'zerct.toml')
  if (existsSync(configPath)) {
    try {
      config = parseZerctToml(readFileSync(configPath, 'utf8'))
      validateConfig(config)
      checks.push({ name: 'zerct.toml', ok: true, message: 'valid' })
    } catch (error) {
      checks.push({
        name: 'zerct.toml',
        ok: false,
        message: error.message,
        agent_instruction: 'Fix zerct.toml so it matches the Zerct deploy contract.'
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
    checks.push(...frontendScriptChecks(projectDir))
  }

  const unsafeHits = scanUnsafe(projectDir)
  checks.push({
    name: 'unsafe',
    ok: unsafeHits.length === 0,
    message: unsafeHits.length === 0 ? 'no direct unsafe found' : unsafeHits.slice(0, 5).join(', '),
    agent_instruction: 'Remove direct unsafe usage from workspace Rust source before deploying.'
  })
  if (kind === 'rust_backend') {
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
  const report = runDoctor(projectDir)
  if (!report.ok) {
    const firstFailure = report.checks.find((check) => !check.ok)
    throw agentError('doctor_failed', 'Zerct doctor failed.', firstFailure?.agent_instruction || 'Fix the failed checks and retry.', cli.json)
  }
  if (report.config?.kind === 'static_frontend' && cli.database) {
    throw agentError('invalid_database_target', 'Static frontends cannot attach managed Postgres directly.', 'Deploy a Rust backend with managed Postgres and call it from the frontend.', cli.json)
  }

  const token = await readOrLoginToken(projectDir, cli)
  const archive = createArchiveBase64(projectDir)
  const commitSha = gitCommitSha(projectDir)
  const body = {
    config: report.config,
    commit_sha: commitSha,
    wants_database: cli.database,
    source_archive_base64: archive
  }

  const response = await apiRequest(cli, 'POST', '/v1/deployments', token, body)
  if (cli.json) {
    console.log(JSON.stringify(response, null, 2))
    return
  }

  console.log(`queued ${response.build_job.id}`)
  console.log(`app ${response.app.id}`)
  console.log(`url ${response.app.url}`)
  console.log(`next npx @zerct/zerct logs --app ${response.app.id}`)
}

async function logs(cli) {
  const response = await appGet(cli, 'logs')
  if (cli.json) {
    console.log(JSON.stringify(response, null, 2))
    return
  }
  for (const line of response.lines || []) {
    console.log(`[${line.timestamp}] ${line.stream}: ${line.message}`)
  }
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
  if (cli.args[0] !== 'set') {
    throw agentError('unknown_command', 'Unknown env command.', 'Use `npx @zerct/zerct env set --app <app_id> KEY=value`.', cli.json)
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

async function billing(cli) {
  const token = await readOrLoginToken(process.cwd(), cli)
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
    throw agentError('missing_app', 'App id is required.', 'Pass `--app <app_id>`. Use the app id printed by `npx @zerct/zerct deploy`.', cli.json)
  }
  return cli.app
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
  const result = spawnSync('sh', ['-c', `command -v ${command} >/dev/null 2>&1`], {
    stdio: 'ignore'
  })
  return result.status === 0
}

function parseZerctToml(source) {
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
  config.build.check ||= config.kind === 'static_frontend' ? DEFAULT_FRONTEND_CHECK_COMMAND : DEFAULT_RUST_CHECK_COMMAND
  config.build.command ||= config.kind === 'static_frontend' ? 'npm ci && npm run build' : 'cargo build --release'
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
  if (!['rust_backend', 'static_frontend'].includes(config.kind)) {
    throw new Error('kind must be rust_backend or static_frontend')
  }
  if (typeof config.build.command !== 'string' || !config.build.command.trim()) {
    throw new Error('[build].command is required')
  }
  if (typeof config.build.check !== 'string' || !config.build.check.trim()) {
    throw new Error('[build].check is required')
  }
  if (config.kind === 'static_frontend') {
    if (typeof config.build.output !== 'string' || !isSafeRelativePath(config.build.output)) {
      throw new Error('[build].output must be a safe relative directory like dist')
    }
    return
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

function frontendLockfileExists(projectDir) {
  return ['package-lock.json', 'npm-shrinkwrap.json', 'pnpm-lock.yaml', 'yarn.lock', 'bun.lock', 'bun.lockb']
    .some((file) => existsSync(path.join(projectDir, file)))
}

function frontendScriptChecks(projectDir) {
  const manifest = readPackageJson(projectDir)
  const missing = (script) => !manifest?.scripts || typeof manifest.scripts[script] !== 'string' || !manifest.scripts[script].trim()
  const checks = ['typecheck', 'lint'].map((script) => ({
    name: `npm script ${script}`,
    ok: !missing(script),
    message: missing(script) ? 'missing' : 'found',
    agent_instruction: `Add a non-empty "${script}" script to package.json, then retry.`
  }))

  if (checks.every((check) => check.ok)) {
    checks.push(npmScriptCheck(projectDir, 'typecheck'))
    checks.push(npmScriptCheck(projectDir, 'lint'))
  }

  return checks
}

function readPackageJson(projectDir) {
  try {
    return JSON.parse(readFileSync(path.join(projectDir, 'package.json'), 'utf8'))
  } catch (_error) {
    return null
  }
}

function npmScriptCheck(projectDir, script) {
  const npm = spawnSync('npm', ['run', '--silent', script], {
    cwd: projectDir,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe']
  })
  if (npm.error) {
    return {
      name: `npm run ${script}`,
      ok: false,
      message: npm.error.message,
      agent_instruction: `Install Node.js and npm, then run \`npm run ${script}\` before deploying.`
    }
  }

  return {
    name: `npm run ${script}`,
    ok: npm.status === 0,
    message: npm.status === 0 ? 'passed' : (npm.stderr || npm.stdout || `npm run ${script} failed`).trim().slice(0, 240),
    agent_instruction: `Run \`npm run ${script}\`, fix every error, then redeploy.`
  }
}

function isSafeRelativePath(value) {
  return value
    && !path.isAbsolute(value)
    && !value.includes('\\')
    && value.split('/').every((part) => part && part !== '.' && part !== '..')
}

function scanUnsafe(projectDir) {
  const hits = []
  walk(projectDir, (file) => {
    if (!file.endsWith('.rs')) {
      return
    }
    const source = readFileSync(file, 'utf8')
    if (/\bunsafe\b/u.test(source)) {
      hits.push(path.relative(projectDir, file))
    }
  })
  return hits
}

function walk(dir, visit) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (['.git', 'target', 'node_modules', '.zerct'].includes(entry.name)) {
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
  const name = path.basename(projectDir).toLowerCase().replace(/[^a-z0-9-]+/gu, '-').replace(/^-+|-+$/gu, '')
  return name || 'api'
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
