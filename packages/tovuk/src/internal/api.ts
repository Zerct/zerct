import { TovukError, agentError } from './errors.ts'
import { enrichAgentErrorPayload } from './agent-error-enrichment.ts'
import { isJsonObject, parseJson } from './json.ts'
import type { AgentErrorPayload, ApiMethod, CliOptions, JsonValue } from './types.ts'

function requireApp(cli: CliOptions): string {
  if (!cli.app) {
    throw agentError('missing_app', 'App is required.', 'Pass `--app <app>` using either the app name from tovuk.toml or the app id printed by deploy.', cli.json)
  }
  return cli.app
}

function pageQuery(cli: CliOptions): string {
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

async function apiRequest(
  cli: CliOptions,
  method: ApiMethod,
  route: string,
  token: string | null,
  body: JsonValue | null
): Promise<JsonValue | null> {
  const headers = new Headers({ accept: 'application/json' })
  if (token) {
    headers.set('authorization', `Bearer ${token}`)
  }
  if (body !== null) {
    headers.set('content-type', 'application/json')
  }

  const init: RequestInit = {
    method,
    headers
  }
  if (body !== null) {
    init.body = JSON.stringify(body)
  }
  const response = await fetch(`${cli.apiUrl}${route}`, init)

  const text = await response.text()
  const data = parseJson(text)
  if (!response.ok) {
    const payload: AgentErrorPayload = isAgentErrorPayload(data) ? data : {
      code: 'api_error',
      message: `Tovuk API returned HTTP ${response.status}.`,
      agent_instruction: 'Retry the command. If it keeps failing, check Tovuk status before changing your project.',
      docs_url: null,
      checkout_url: null
    }
    await enrichAgentErrorPayload({ cli, route, token }, payload)
    throw new TovukError(payload, cli.json, response.status >= 500 ? 2 : 1)
  }

  return data
}

function isAgentErrorPayload(value: JsonValue | null): value is AgentErrorPayload {
  return isJsonObject(value)
    && typeof value['code'] === 'string'
    && typeof value['message'] === 'string'
    && (typeof value['agent_instruction'] === 'string' || value['agent_instruction'] === null)
}

export {
  requireApp,
  pageQuery,
  apiRequest
}
