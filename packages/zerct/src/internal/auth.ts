import { spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { homedir } from 'node:os'
import path from 'node:path'
import { DEFAULT_LOGIN_EXPIRES_SECONDS, DEFAULT_LOGIN_INTERVAL_SECONDS, SESSION_ACCOUNT, SESSION_DIR, SESSION_FILE, SESSION_LABEL, SESSION_SERVICE } from './constants.ts'
import { agentError } from './errors.ts'
import { apiRequest } from './api.ts'
import { jsonObjectOrEmpty, numberField, stringField } from './json.ts'
import { hasCommand, openUrl, progress, sleep } from './project.ts'
import type { CliOptions, JsonObject, LoginPollResponse, LoginStartResponse } from './types.ts'

type AliasSpec<Key extends string> = Readonly<{
  field: Key
  aliases: readonly string[]
}>

const LOGIN_START_STRING_FIELDS = [
  { field: 'deviceCode', aliases: ['deviceCode', 'device_code'] },
  { field: 'loginUrl', aliases: ['loginUrl', 'login_url'] },
  { field: 'userCode', aliases: ['userCode', 'user_code'] }
] as const

const LOGIN_START_NUMBER_FIELDS = [
  { field: 'expiresInSeconds', aliases: ['expiresInSeconds', 'expires_in_seconds'] },
  { field: 'intervalSeconds', aliases: ['intervalSeconds', 'interval_seconds'] }
] as const

const LOGIN_POLL_STRING_FIELDS = [
  { field: 'email', aliases: ['email'] },
  { field: 'status', aliases: ['status'] },
  { field: 'token', aliases: ['token'] }
] as const

const LOGIN_POLL_NUMBER_FIELDS = [
  { field: 'intervalSeconds', aliases: ['intervalSeconds', 'interval_seconds'] }
] as const

async function login(cli: CliOptions): Promise<void> {
  if (cli.token) {
    writeSessionToken(cli.token)
    console.log('saved Zerct session token')
    return
  }

  await loginAndStore(cli)
}

async function readOrLoginToken(projectDir: string, cli: CliOptions): Promise<string> {
  const token = readStoredToken(projectDir, cli)
  if (token) {
    return token
  }

  return loginAndStore(cli)
}

async function loginAndStore(cli: CliOptions): Promise<string> {
  const start = loginStartResponse(await apiRequest(cli, 'POST', '/v1/login/device', null, null))
  if (!start.loginUrl) {
    throw agentError('login_failed', 'Zerct login did not return a browser URL.', 'Retry `npx @zerct/zerct login`. If it keeps failing, check Zerct status.', cli.json)
  }
  openUrl(start.loginUrl)
  progress(cli, 'opened browser login')
  progress(cli, `waiting for browser login code ${start.userCode ?? 'ZERCT'}`)

  const session = await pollLogin(cli, start)
  if (!session.token) {
    throw agentError('login_failed', 'Zerct login did not return a session token.', 'Run `npx @zerct/zerct login` again and complete the browser login.', cli.json)
  }

  writeSessionToken(session.token)
  progress(cli, `logged in as ${session.email ?? 'Zerct user'}`)
  return session.token
}

async function pollLogin(cli: CliOptions, start: LoginStartResponse): Promise<LoginPollResponse> {
  if (!start.deviceCode) {
    throw agentError('login_failed', 'Zerct login did not return a device code.', 'Retry `npx @zerct/zerct login`. If it keeps failing, check Zerct status.', cli.json)
  }

  const expiresMs = (start.expiresInSeconds ?? DEFAULT_LOGIN_EXPIRES_SECONDS) * 1000
  const deadline = Date.now() + expiresMs
  let intervalMs = (start.intervalSeconds ?? DEFAULT_LOGIN_INTERVAL_SECONDS) * 1000

  while (Date.now() < deadline) {
    await sleep(intervalMs)
    const response = loginPollResponse(await apiRequest(cli, 'GET', `/v1/login/device/${encodeURIComponent(start.deviceCode)}`, null, null))
    if (response.status === 'complete') {
      return response
    }
    if (response.status === 'expired') {
      throw agentError('login_expired', 'Zerct login expired before it completed.', 'Run `npx @zerct/zerct login` again and finish the browser login in the newly opened tab.', cli.json)
    }
    intervalMs = Math.max(
      DEFAULT_LOGIN_INTERVAL_SECONDS * 1000,
      (response.intervalSeconds ?? DEFAULT_LOGIN_INTERVAL_SECONDS) * 1000
    )
  }

  throw agentError('login_expired', 'Zerct login expired before it completed.', 'Run `npx @zerct/zerct login` again and finish the browser login in the newly opened tab.', cli.json)
}

function readStoredToken(projectDir: string, cli: CliOptions): string {
  if (cli.token) {
    return cli.token
  }
  if (process.env['ZERCT_TOKEN']) {
    return process.env['ZERCT_TOKEN']
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

function writeSessionToken(token: string): void {
  const cleanToken = token.trim()
  if (!cleanToken) {
    throw agentError('login_failed', 'Zerct session token is empty.', 'Run `npx @zerct/zerct login` again and complete the browser login.', false)
  }
  if (writeKeychainToken(cleanToken)) {
    return
  }

  writeTokenFile(userSessionPath(), cleanToken)
}

function readTokenFile(filePath: string): string {
  if (!existsSync(filePath)) {
    return ''
  }
  return readFileSync(filePath, 'utf8').trim()
}

function writeTokenFile(filePath: string, token: string): void {
  const dir = path.dirname(filePath)
  mkdirSync(dir, { recursive: true, mode: 0o700 })
  writeFileSync(filePath, `${token}\n`, { mode: 0o600 })
}

function userSessionPath(): string {
  if (process.platform === 'win32' && process.env['APPDATA']) {
    return path.join(process.env['APPDATA'], 'Zerct', SESSION_FILE)
  }
  const configHome = process.env['XDG_CONFIG_HOME'] ?? path.join(homedir(), '.config')
  return path.join(configHome, 'zerct', SESSION_FILE)
}

function readKeychainToken(): string {
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

function writeKeychainToken(token: string): boolean {
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

function loginStartResponse(value: Awaited<ReturnType<typeof apiRequest>>): LoginStartResponse {
  const source = jsonObjectOrEmpty(value)
  return {
    ...stringAliasFields(source, LOGIN_START_STRING_FIELDS),
    ...numberAliasFields(source, LOGIN_START_NUMBER_FIELDS)
  }
}

function loginPollResponse(value: Awaited<ReturnType<typeof apiRequest>>): LoginPollResponse {
  const source = jsonObjectOrEmpty(value)
  return {
    ...stringAliasFields(source, LOGIN_POLL_STRING_FIELDS),
    ...numberAliasFields(source, LOGIN_POLL_NUMBER_FIELDS)
  }
}

function stringAliasFields<Key extends string>(
  source: JsonObject,
  specs: readonly AliasSpec<Key>[]
): Partial<Record<Key, string>> {
  const response: Partial<Record<Key, string>> = {}
  for (const spec of specs) {
    const value = firstStringAlias(source, spec.aliases)
    if (value) {
      response[spec.field] = value
    }
  }
  return response
}

function numberAliasFields<Key extends string>(
  source: JsonObject,
  specs: readonly AliasSpec<Key>[]
): Partial<Record<Key, number>> {
  const response: Partial<Record<Key, number>> = {}
  for (const spec of specs) {
    const value = firstPositiveNumberAlias(source, spec.aliases)
    if (value > 0) {
      response[spec.field] = value
    }
  }
  return response
}

function firstStringAlias(source: JsonObject, aliases: readonly string[]): string {
  for (const alias of aliases) {
    const value = stringField(source, alias)
    if (value) {
      return value
    }
  }
  return ''
}

function firstPositiveNumberAlias(source: JsonObject, aliases: readonly string[]): number {
  for (const alias of aliases) {
    const value = numberField(source, alias)
    if (value > 0) {
      return value
    }
  }
  return 0
}

export { login, readOrLoginToken }
