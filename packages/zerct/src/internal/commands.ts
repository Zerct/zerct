import { agentError } from './errors.ts'
import { apiRequest, isJsonObject, jsonArrayField, jsonObjectField, jsonObjectOrEmpty, pageQuery, requireApp, stringField } from './api.ts'
import { readOrLoginToken } from './auth.ts'
import { openUrl, printJson } from './project.ts'
import type { CheckoutResponse, CliOptions, JsonObject, JsonValue, LogLine, LogsResponse } from './types.ts'

interface LogsRequest {
  route: string
  target: string
}

async function logs(cli: CliOptions): Promise<void> {
  const token = await readOrLoginToken(process.cwd(), cli)
  const request = logsRequest(cli)
  const response = logsResponse(await apiRequest(cli, 'GET', request.route, token, null))
  if (cli.json) {
    printJson(response)
    return
  }
  for (const line of response.lines) {
    console.log(`[${line.timestamp}] ${line.stream}: ${line.message}`)
  }
  if (response.has_more && response.next_cursor) {
    console.log(`next npx @zerct/zerct logs ${request.target} --cursor ${response.next_cursor}`)
  }
}

function logsRequest(cli: CliOptions): LogsRequest {
  const page = pageQuery(cli)
  if (cli.build) {
    return {
      route: `/v1/builds/${encodeURIComponent(cli.build)}/logs${page}`,
      target: `--build ${cli.build}`
    }
  }
  if (cli.deploy) {
    return {
      route: `/v1/deploys/${encodeURIComponent(cli.deploy)}/logs${page}`,
      target: `--deploy ${cli.deploy}`
    }
  }

  const app = requireApp(cli)
  return {
    route: `/v1/apps/${encodeURIComponent(app)}/logs${page}`,
    target: `--app ${app}`
  }
}

async function apps(cli: CliOptions): Promise<void> {
  await printAuthenticated(cli, '/v1/apps')
}

async function capabilities(cli: CliOptions): Promise<void> {
  const response = await apiRequest(cli, 'GET', '/v1/capabilities', null, null)
  printJson(response)
}

async function me(cli: CliOptions): Promise<void> {
  await printAuthenticated(cli, '/v1/me')
}

async function usage(cli: CliOptions): Promise<void> {
  await printAuthenticated(cli, '/v1/usage')
}

async function activity(cli: CliOptions): Promise<void> {
  await printAuthenticated(cli, `/v1/activity${pageQuery(cli)}`)
}

async function overview(cli: CliOptions): Promise<void> {
  const app = requireApp(cli)
  await printAuthenticated(cli, `/v1/apps/${encodeURIComponent(app)}/overview${pageQuery(cli)}`)
}

async function deploys(cli: CliOptions): Promise<void> {
  const route = cli.app
    ? `/v1/apps/${encodeURIComponent(cli.app)}/deploys${pageQuery(cli)}`
    : `/v1/deploys${pageQuery(cli)}`
  await printAuthenticated(cli, route)
}

async function builds(cli: CliOptions): Promise<void> {
  const route = cli.app
    ? `/v1/apps/${encodeURIComponent(cli.app)}/builds${pageQuery(cli)}`
    : `/v1/builds${pageQuery(cli)}`
  await printAuthenticated(cli, route)
}

async function status(cli: CliOptions): Promise<void> {
  printJson(await appGet(cli, 'status'))
}

async function inspect(cli: CliOptions): Promise<void> {
  printJson(await appGet(cli, 'inspect'))
}

async function database(cli: CliOptions): Promise<void> {
  printJson(await appGet(cli, 'database'))
}

async function envCommand(cli: CliOptions): Promise<void> {
  if (cli.args[0] === 'list') {
    printJson(await appGet(cli, 'env'))
    return
  }

  if (cli.args[0] === 'delete') {
    const name = cli.args[1] ?? ''
    if (!name) {
      throw agentError('invalid_env', 'Environment variable name is required.', 'Use `npx @zerct/zerct env delete --app <app> KEY`.', cli.json)
    }
    const token = await readOrLoginToken(process.cwd(), cli)
    const app = requireApp(cli)
    const response = await apiRequest(cli, 'DELETE', `/v1/apps/${encodeURIComponent(app)}/env/${encodeURIComponent(name)}`, token, null)
    printJson(response)
    return
  }

  if (cli.args[0] !== 'set') {
    throw agentError('unknown_command', 'Unknown env command.', 'Use `npx @zerct/zerct env list`, `env set`, or `env delete`.', cli.json)
  }

  const assignment = cli.args[1] ?? ''
  const separator = assignment.indexOf('=')
  if (separator <= 0) {
    throw agentError('invalid_env', 'Environment assignment must be KEY=value.', 'Pass one uppercase shell-safe environment assignment, for example `API_KEY=value`.', cli.json)
  }

  const name = assignment.slice(0, separator)
  const value = assignment.slice(separator + 1)
  const token = await readOrLoginToken(process.cwd(), cli)
  const app = requireApp(cli)
  const response = await apiRequest(cli, 'PUT', `/v1/apps/${encodeURIComponent(app)}/env`, token, { name, value })
  printJson(response)
}

async function domainsCommand(cli: CliOptions): Promise<void> {
  const action = cli.args[0] ?? 'list'
  if (action === 'list') {
    printJson(await appGet(cli, 'domains'))
    return
  }

  const domain = cli.args[1] ?? ''
  if (!domain) {
    throw agentError('missing_domain', 'Domain is required.', 'Use `npx @zerct/zerct domains add --app <app> api.example.com`.', cli.json)
  }

  const token = await readOrLoginToken(process.cwd(), cli)
  const app = requireApp(cli)
  if (action === 'add') {
    const response = await apiRequest(cli, 'POST', `/v1/apps/${encodeURIComponent(app)}/domains`, token, { domain })
    printJson(response)
    return
  }
  if (action === 'verify') {
    const response = await apiRequest(cli, 'POST', `/v1/apps/${encodeURIComponent(app)}/domains/${encodeURIComponent(domain)}/verify`, token, null)
    printJson(response)
    return
  }
  if (action === 'delete') {
    const response = await apiRequest(cli, 'DELETE', `/v1/apps/${encodeURIComponent(app)}/domains/${encodeURIComponent(domain)}`, token, null)
    printJson(response)
    return
  }

  throw agentError('unknown_command', 'Unknown domains command.', 'Use `domains list`, `domains add`, `domains verify`, or `domains delete`.', cli.json)
}

async function billing(cli: CliOptions): Promise<void> {
  const token = await readOrLoginToken(process.cwd(), cli)
  const route = cli.args[0] === 'portal' ? '/v1/billing/portal' : '/v1/billing/checkout'
  const body: JsonObject | null = route.endsWith('/checkout')
    ? { target_plan: 'pro', reason: 'Upgrade to Zerct Pro.' }
    : null
  const response = checkoutResponse(await apiRequest(cli, 'POST', route, token, body))
  if (cli.json) {
    printJson(response)
    return
  }
  console.log(response.checkout.url)
  openUrl(response.checkout.url)
}

async function printAuthenticated(cli: CliOptions, route: string): Promise<void> {
  const token = await readOrLoginToken(process.cwd(), cli)
  const response = await apiRequest(cli, 'GET', route, token, null)
  printJson(response)
}

async function appGet(cli: CliOptions, kind: string): Promise<JsonValue | null> {
  const token = await readOrLoginToken(process.cwd(), cli)
  const app = requireApp(cli)
  return apiRequest(cli, 'GET', `/v1/apps/${encodeURIComponent(app)}/${kind}`, token, null)
}

function logsResponse(value: JsonValue | null): LogsResponse {
  const source = jsonObjectOrEmpty(value)
  const lines = jsonArrayField(source, 'lines').map(logLine).filter((line): line is LogLine => line !== null)
  return {
    lines,
    has_more: source['has_more'] === true,
    next_cursor: stringField(source, 'next_cursor')
  }
}

function logLine(value: JsonValue): LogLine | null {
  if (!isJsonObject(value)) {
    return null
  }
  const timestamp = stringField(value, 'timestamp')
  const stream = stringField(value, 'stream')
  const message = stringField(value, 'message')
  return timestamp && stream && message ? { timestamp, stream, message } : null
}

function checkoutResponse(value: JsonValue | null): CheckoutResponse {
  const source = jsonObjectOrEmpty(value)
  return {
    checkout: {
      url: stringField(jsonObjectField(source, 'checkout'), 'url')
    }
  }
}

export {
  logs,
  apps,
  capabilities,
  me,
  usage,
  activity,
  overview,
  deploys,
  builds,
  status,
  inspect,
  database,
  envCommand,
  domainsCommand,
  billing
}
