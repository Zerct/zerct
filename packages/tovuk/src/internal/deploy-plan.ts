import { agentError } from './errors.ts'
import { paymentRequiredAgentError } from './agent-error-enrichment.ts'
import { apiRequest } from './api.ts'
import { appsResponseFromJson } from './api-models.ts'
import { jsonObjectField, jsonObjectOrEmpty, numberField } from './json.ts'
import type { AppSummary, CliOptions, DeployPlanProject, DeployProjectInfo } from './types.ts'

async function createDeployPlan(projects: DeployProjectInfo[], cli: CliOptions, token: string): Promise<DeployPlanProject[]> {
  const plan = projects.map((project) => ({
    project,
    wantsDatabase: cli.database && project.kind === 'rust_backend'
  }))
  rejectInvalidDatabaseTargets(plan, cli)
  await preflightDeployLimits(plan, cli, token)
  return plan
}

function rejectInvalidDatabaseTargets(plan: DeployPlanProject[], cli: CliOptions): void {
  if (cli.database && plan.length === 1 && plan[0]?.project.kind === 'static_frontend') {
    throw agentError('invalid_database_target', 'Static frontends cannot attach managed Postgres directly.', 'Deploy a Rust backend with managed Postgres and call it from the frontend.', cli.json)
  }
}

async function preflightDeployLimits(plan: DeployPlanProject[], cli: CliOptions, token: string): Promise<void> {
  const [usageResponse, appsResponse] = await Promise.all([
    apiRequest(cli, 'GET', '/v1/usage', token, null),
    apiRequest(cli, 'GET', '/v1/apps', token, null)
  ])
  const usageRoot = jsonObjectOrEmpty(usageResponse)
  const usage = jsonObjectField(usageRoot, 'usage')
  const limits = jsonObjectField(usageRoot, 'limits')
  const existingApps = appNameMap(appsResponseFromJson(appsResponse).apps)
  const requested = requestedNewResources(plan, existingApps)

  const usedProjects = numberField(usage, 'appCount')
  const projectLimit = numberField(limits, 'projects')
  const usedDatabases = numberField(usage, 'databaseCount')
  const databaseLimit = numberField(limits, 'managedDatabases')

  if (requested.projects > 0 && usedProjects + requested.projects > projectLimit) {
    throw await paymentRequiredAgentError(
      cli,
      token,
      `Project limit reached: ${usedProjects}/${projectLimit} projects are already used.`,
      'Redeploy an existing app by reusing its `name` in tovuk.toml, or open the returned Stripe Checkout URL before creating another project.'
    )
  }

  if (requested.databases > 0 && usedDatabases + requested.databases > databaseLimit) {
    throw await paymentRequiredAgentError(
      cli,
      token,
      `Managed Postgres limit reached: ${usedDatabases}/${databaseLimit} databases are already used.`,
      'Redeploy an app that already has managed Postgres, deploy without `--database`, or open the returned Stripe Checkout URL.'
    )
  }
}

function appNameMap(apps: AppSummary[]): Map<string, AppSummary> {
  const existingApps = new Map<string, AppSummary>()
  for (const app of apps) {
    if (app.name) {
      existingApps.set(app.name, app)
    }
  }
  return existingApps
}

function requestedNewResources(plan: DeployPlanProject[], existingApps: Map<string, AppSummary>): { projects: number; databases: number } {
  let projects = 0
  let databases = 0

  for (const target of plan) {
    if (!target.project.name || target.project.kind === 'unknown') {
      continue
    }
    const existing = existingApps.get(target.project.name)
    if (!existing) {
      projects += 1
    }
    if (target.wantsDatabase && existing?.databaseStorageMib === undefined) {
      databases += 1
    }
  }

  return { projects, databases }
}

export { createDeployPlan }
