import { ZerctError, agentError } from './errors.js'

async function appGet(cli, kind) {
  const token = await readOrLoginToken(process.cwd(), cli)
  const app = requireApp(cli)
  return apiRequest(cli, 'GET', `/v1/apps/${encodeURIComponent(app)}/${kind}`, token, null)
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

export { appGet, requireApp, pageQuery, apiRequest, parseJson }
