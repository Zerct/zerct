import { ZerctError, agentError } from './errors.ts'
import type { AgentErrorPayload, ApiMethod, CliOptions, JsonObject, JsonValue } from './types.ts'

function requireApp(cli: CliOptions): string {
  if (!cli.app) {
    throw agentError('missing_app', 'App is required.', 'Pass `--app <app>` using either the app name from zerct.toml or the app id printed by deploy.', cli.json)
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
      message: `Zerct API returned HTTP ${response.status}.`,
      agent_instruction: 'Retry the command. If it keeps failing, check Zerct status before changing your project.',
      docs_url: null,
      checkout_url: null
    }
    throw new ZerctError(payload, cli.json, response.status >= 500 ? 2 : 1)
  }

  return data
}

function parseJson(text: string): JsonValue | null {
  if (!text.trim()) {
    return null
  }
  try {
    return JSON.parse(text)
  } catch {
    return null
  }
}

function isJsonObject(value: JsonValue | null): value is JsonObject {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
}

function isAgentErrorPayload(value: JsonValue | null): value is AgentErrorPayload {
  return isJsonObject(value)
    && typeof value['code'] === 'string'
    && typeof value['message'] === 'string'
    && (typeof value['agent_instruction'] === 'string' || value['agent_instruction'] === null)
}

function jsonObjectOrEmpty(value: JsonValue | null): JsonObject {
  return isJsonObject(value) ? value : {}
}

function jsonObjectField(source: JsonObject, key: string): JsonObject {
  return jsonObjectOrEmpty(source[key] ?? null)
}

function optionalJsonObjectField(source: JsonObject, key: string): JsonObject | null {
  const value = source[key] ?? null
  return isJsonObject(value) ? value : null
}

function jsonArrayField(source: JsonObject, key: string): JsonValue[] {
  const value = source[key]
  return Array.isArray(value) ? value : []
}

function stringField(source: JsonObject, key: string): string {
  const value = source[key]
  return typeof value === 'string' ? value : ''
}

function numberField(source: JsonObject, key: string): number {
  const value = source[key]
  return typeof value === 'number' ? value : Number(value ?? 0)
}

export {
  requireApp,
  pageQuery,
  apiRequest,
  isJsonObject,
  jsonObjectOrEmpty,
  jsonObjectField,
  optionalJsonObjectField,
  jsonArrayField,
  stringField,
  numberField
}
