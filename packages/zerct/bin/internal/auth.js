import { spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { homedir } from 'node:os'
import path from 'node:path'
import { DEFAULT_LOGIN_EXPIRES_SECONDS, DEFAULT_LOGIN_INTERVAL_SECONDS, SESSION_ACCOUNT, SESSION_DIR, SESSION_FILE, SESSION_LABEL, SESSION_SERVICE } from './constants.js'
import { agentError } from './errors.js'
import { apiRequest } from './api.js'
import { hasCommand, openUrl, progress, sleep } from './project.js'

async function login(cli) {
  if (cli.token) {
    writeSessionToken(cli.token)
    console.log('saved Zerct session token')
    return
  }

  await loginAndStore(cli)
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

export { login, readOrLoginToken, loginAndStore, pollLogin, readStoredToken, writeSessionToken, readTokenFile, writeTokenFile, userSessionPath, readKeychainToken, writeKeychainToken }
