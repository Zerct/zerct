import { agentError } from './errors.js'
import { apiRequest, appGet, pageQuery, requireApp } from './api.js'
import { readOrLoginToken } from './auth.js'
import { openUrl, printJsonOrPretty } from './project.js'

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

export { logs, apps, capabilities, me, usage, activity, overview, deploys, builds, status, inspect, database, envCommand, domainsCommand, billing }
