#!/usr/bin/env node
import { spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs'
import { homedir } from 'node:os'
import path from 'node:path'

const VERSION = '0.1.0'
const DEFAULT_API_URL = 'https://api.zerct.com'
const ARCHIVE_LIMIT_BYTES = 48 * 1024 * 1024
const SESSION_DIR = '.zerct'
const SESSION_FILE = 'session-token'

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
  - Keep Cargo.lock committed.
  - Keep direct unsafe out of workspace source.
  - Listen on 0.0.0.0:$PORT.
  - Return HTTP 200 from the configured health endpoint.
`

main().catch((error) => {
  if (error instanceof ZerctError) {
    printAgentError(error.payload, error.json)
    process.exitCode = error.exitCode
    return
  }

  console.error(`zerct failed: ${error.message}`)
  process.exitCode = 1
})

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
      throw agentError('unknown_command', 'Unknown Zerct command.', 'Run `npx zerct --help` and retry with a supported command.', cli.json)
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
    throw agentError('doctor_failed', 'Zerct doctor failed.', firstFailure?.agent_instruction || 'Fix the failed checks and retry `npx zerct doctor`.', json)
  }
}

function runDoctor(projectDir) {
  const checks = []
  const requiredFiles = ['Cargo.toml', 'Cargo.lock', 'zerct.toml']
  for (const file of requiredFiles) {
    const ok = existsSync(path.join(projectDir, file))
    checks.push({
      name: file,
      ok,
      message: ok ? 'found' : 'missing',
      agent_instruction: `Create and commit ${file}, then retry.`
    })
  }

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
  }

  const unsafeHits = scanUnsafe(projectDir)
  checks.push({
    name: 'unsafe',
    ok: unsafeHits.length === 0,
    message: unsafeHits.length === 0 ? 'no direct unsafe found' : unsafeHits.slice(0, 5).join(', '),
    agent_instruction: 'Remove direct unsafe usage from workspace Rust source before deploying.'
  })

  return {
    ok: checks.every((check) => check.ok),
    project: projectDir,
    config,
    checks
  }
}

async function login(cli) {
  if (cli.token) {
    writeSessionToken(process.cwd(), cli.token)
    console.log('saved Zerct session token to .zerct/session-token')
    return
  }

  const response = await apiRequest(cli, 'POST', '/v1/login/device', null, null)
  openUrl(response.login_url)
  console.log(`opened ${response.login_url}`)
  console.log('After login, retry your deploy. If the CLI cannot finish automatically yet, set ZERCT_TOKEN or run `npx zerct login --token <token>`.')
}

async function deploy(projectDir, cli) {
  const report = runDoctor(projectDir)
  if (!report.ok) {
    const firstFailure = report.checks.find((check) => !check.ok)
    throw agentError('doctor_failed', 'Zerct doctor failed.', firstFailure?.agent_instruction || 'Fix the failed checks and retry.', cli.json)
  }

  const token = readToken(projectDir, cli)
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
  console.log(`next npx zerct logs --app ${response.app.id}`)
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
    throw agentError('unknown_command', 'Unknown env command.', 'Use `npx zerct env set --app <app_id> KEY=value`.', cli.json)
  }

  const assignment = cli.args[1] || ''
  const separator = assignment.indexOf('=')
  if (separator <= 0) {
    throw agentError('invalid_env', 'Environment assignment must be KEY=value.', 'Pass one uppercase shell-safe environment assignment, for example `API_KEY=value`.', cli.json)
  }

  const name = assignment.slice(0, separator)
  const value = assignment.slice(separator + 1)
  const token = readToken(process.cwd(), cli)
  const app = requireApp(cli)
  const response = await apiRequest(cli, 'PUT', `/v1/apps/${encodeURIComponent(app)}/env`, token, { name, value })
  printJsonOrPretty(cli, response)
}

async function billing(cli) {
  const token = readToken(process.cwd(), cli)
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
  const token = readToken(process.cwd(), cli)
  const app = requireApp(cli)
  return apiRequest(cli, 'GET', `/v1/apps/${encodeURIComponent(app)}/${kind}`, token, null)
}

function requireApp(cli) {
  if (!cli.app) {
    throw agentError('missing_app', 'App id is required.', 'Pass `--app <app_id>`. Use the app id printed by `npx zerct deploy`.', cli.json)
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
  const tar = spawnSync('tar', [
    '--exclude=.git',
    '--exclude=target',
    '--exclude=node_modules',
    '--exclude=.zerct',
    '--exclude=.env',
    '--exclude=.env.*',
    '-czf',
    '-',
    '-C',
    projectDir,
    '.'
  ], {
    encoding: 'buffer',
    maxBuffer: ARCHIVE_LIMIT_BYTES + 1024 * 1024
  })

  if (tar.error) {
    throw agentError('archive_failed', 'Could not create source archive.', 'Install `tar`, remove local build outputs, then retry `npx zerct deploy`.', false)
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

function readToken(projectDir, cli) {
  if (cli.token) {
    return cli.token
  }
  if (process.env.ZERCT_TOKEN) {
    return process.env.ZERCT_TOKEN
  }

  const projectToken = path.join(projectDir, SESSION_DIR, SESSION_FILE)
  if (existsSync(projectToken)) {
    return readFileSync(projectToken, 'utf8').trim()
  }

  const homeToken = path.join(homedir(), SESSION_DIR, SESSION_FILE)
  if (existsSync(homeToken)) {
    return readFileSync(homeToken, 'utf8').trim()
  }

  throw agentError('login_required', 'Zerct login is required.', 'Run `npx zerct login`, set `ZERCT_TOKEN`, or run `npx zerct login --token <token>`, then retry.', cli.json)
}

function writeSessionToken(projectDir, token) {
  const dir = path.join(projectDir, SESSION_DIR)
  mkdirSync(dir, { recursive: true, mode: 0o700 })
  writeFileSync(path.join(dir, SESSION_FILE), `${token.trim()}\n`, { mode: 0o600 })
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

  config.build.command ||= 'cargo build --release'
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
