import { agentError } from './errors.ts'
import { apiRequest, pageQuery, requireApp } from './api.ts'
import { checkoutResponseFromJson, logsResponseFromJson } from './api-models.ts'
import { readOrLoginToken } from './auth.ts'
import { openUrl, printJson } from './project.ts'
import type { ApiMethod, CliOptions, JsonObject, JsonValue } from './types.ts'

interface LogsRequest {
  route: string
  target: string
}

type SubcommandHandler = (cli: CliOptions) => Promise<void>
type SubcommandTable = Readonly<Record<string, SubcommandHandler>>
type DomainMethod = Extract<ApiMethod, 'DELETE' | 'POST'>
type DomainRoute = (app: string, domain: string) => string
type DomainBody = (domain: string) => JsonObject | null
type BillingAction = 'checkout' | 'portal'

const ENV_COMMANDS: SubcommandTable = {
  list: async (cli): Promise<void> => printJson(await appGet(cli, 'env')),
  set: envSet,
  delete: envDelete
}

const DOMAIN_COMMANDS: SubcommandTable = {
  list: async (cli): Promise<void> => printJson(await appGet(cli, 'domains')),
  add: domainMutation('POST', (app): string => `/v1/apps/${encodeURIComponent(app)}/domains`, (domain): JsonObject => ({ domain })),
  verify: domainMutation('POST', (app, domain): string => `/v1/apps/${encodeURIComponent(app)}/domains/${encodeURIComponent(domain)}/verify`, (): null => null),
  delete: domainMutation('DELETE', (app, domain): string => `/v1/apps/${encodeURIComponent(app)}/domains/${encodeURIComponent(domain)}`, (): null => null)
}

const SUPPORT_COMMANDS: SubcommandTable = {
  list: supportList,
  create: supportCreate,
  resolve: supportResolve
}

async function logs(cli: CliOptions): Promise<void> {
  const token = await readOrLoginToken(process.cwd(), cli)
  const request = logsRequest(cli)
  const response = logsResponseFromJson(await apiRequest(cli, 'GET', request.route, token, null))
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
  await runSubcommand(cli, ENV_COMMANDS, 'list', 'Unknown env command.', 'Use `npx @zerct/zerct env list`, `env set`, or `env delete`.')
}

async function envDelete(cli: CliOptions): Promise<void> {
  const name = requireCommandArg(cli, 'invalid_env', 'Environment variable name is required.', 'Use `npx @zerct/zerct env delete --app <app> KEY`.')
  const token = await readOrLoginToken(process.cwd(), cli)
  const app = requireApp(cli)
  const response = await apiRequest(cli, 'DELETE', `/v1/apps/${encodeURIComponent(app)}/env/${encodeURIComponent(name)}`, token, null)
  printJson(response)
}

async function envSet(cli: CliOptions): Promise<void> {
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
  await runSubcommand(cli, DOMAIN_COMMANDS, 'list', 'Unknown domains command.', 'Use `domains list`, `domains add`, `domains verify`, or `domains delete`.')
}

function domainMutation(method: DomainMethod, route: DomainRoute, body: DomainBody): SubcommandHandler {
  return async (cli): Promise<void> => {
    const domain = requireCommandArg(cli, 'missing_domain', 'Domain is required.', 'Use `npx @zerct/zerct domains add --app <app> api.example.com`.')
    const token = await readOrLoginToken(process.cwd(), cli)
    const app = requireApp(cli)
    const response = await apiRequest(cli, method, route(app, domain), token, body(domain))
    printJson(response)
  }
}

async function billing(cli: CliOptions): Promise<void> {
  const token = await readOrLoginToken(process.cwd(), cli)
  const action = billingAction(cli.args[0] ?? 'checkout', cli.json)
  const route = action === 'portal' ? '/v1/billing/portal' : '/v1/billing/checkout'
  const reason = cli.args.slice(1).join(' ').trim() || 'Upgrade to Zerct Pro.'
  const body: JsonObject | null = action === 'checkout'
    ? { target_plan: 'pro', reason }
    : null
  const response = checkoutResponseFromJson(await apiRequest(cli, 'POST', route, token, body))
  if (cli.json) {
    printJson(response)
    return
  }
  console.log(response.checkout.url)
  openUrl(response.checkout.url)
}

function billingAction(value: string, json: boolean): BillingAction {
  if (!value || value === 'checkout') {
    return 'checkout'
  }
  if (value === 'portal') {
    return 'portal'
  }
  throw agentError('unknown_billing_command', 'Unknown billing command.', 'Use `npx @zerct/zerct billing checkout --json` or `npx @zerct/zerct billing portal`.', json)
}

async function support(cli: CliOptions): Promise<void> {
  await runSubcommand(cli, SUPPORT_COMMANDS, 'list', 'Unknown support command.', 'Use `npx @zerct/zerct support list --json` or `support create` with subject and details.')
}

async function supportList(cli: CliOptions): Promise<void> {
  await printAuthenticated(cli, `/v1/support/tickets${pageQuery(cli)}`)
}

async function supportCreate(cli: CliOptions): Promise<void> {
  const subject = cli.args[1] ?? ''
  const details = cli.args.slice(2).join(' ').trim()
  if (!subject || !details) {
    throw agentError(
      'invalid_support_ticket',
      'Support ticket subject and details are required.',
      'Use `npx @zerct/zerct support create "Short subject" "Command, app id, build id, deploy id, and first actionable log line" --json`.',
      cli.json
    )
  }

  const token = await readOrLoginToken(process.cwd(), cli)
  const body: JsonObject = {
    details,
    severity: cli.severity || 'normal',
    subject
  }
  if (cli.app) {
    body['app_id'] = cli.app
  }
  if (cli.failingCommand) {
    body['failing_command'] = cli.failingCommand
  }
  if (cli.build) {
    body['build_id'] = cli.build
  }
  if (cli.deploy) {
    body['deploy_id'] = cli.deploy
  }
  if (cli.firstLogLine) {
    body['first_log_line'] = cli.firstLogLine
  }
  const response = await apiRequest(cli, 'POST', '/v1/support/tickets', token, body)
  printJson(response)
}

async function supportResolve(cli: CliOptions): Promise<void> {
  const ticketId = requireCommandArg(
    cli,
    'invalid_support_ticket',
    'Support ticket id is required.',
    'Use `npx @zerct/zerct support resolve <ticket_id> --json` with an id from support list.'
  )
  const token = await readOrLoginToken(process.cwd(), cli)
  const response = await apiRequest(cli, 'POST', `/v1/support/tickets/${encodeURIComponent(ticketId)}/resolve`, token, null)
  printJson(response)
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

async function runSubcommand(cli: CliOptions, commands: SubcommandTable, defaultName: string, message: string, instruction: string): Promise<void> {
  const command = commands[cli.args[0] ?? defaultName]
  if (!command) {
    throw agentError('unknown_command', message, instruction, cli.json)
  }
  await command(cli)
}

function requireCommandArg(cli: CliOptions, code: string, message: string, instruction: string): string {
  const value = cli.args[1] ?? ''
  if (!value) {
    throw agentError(code, message, instruction, cli.json)
  }
  return value
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
  billing,
  support
}
