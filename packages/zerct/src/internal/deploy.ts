import { agentError } from './errors.ts'
import { apiRequest, jsonObjectField, jsonObjectOrEmpty, numberField } from './api.ts'
import { appsResponseFromJson, buildStatusResponseFromJson, deployResponseFromJson } from './api-models.ts'
import { createArchiveBase64, gitCommitSha } from './archive.ts'
import { readOrLoginToken } from './auth.ts'
import { runDoctor } from './doctor.ts'
import { discoverDeployProjects } from './workspace.ts'
import { printJson, progress, sleep } from './project.ts'
import type {
  AppSummary,
  BuildRecord,
  CliOptions,
  DeployResponse,
  DeployProjectInfo,
  JsonObject,
  WorkspaceDeployResult
} from './types.ts'

async function deploy(projectDir: string, cli: CliOptions): Promise<void> {
  const projects = discoverDeployProjects(projectDir)
  if (projects.length === 0) {
    throw agentError('missing_project_contract', 'No zerct.toml was found.', 'Run `npx @zerct/zerct init` in each app directory, or pass a project path.', cli.json)
  }

  const singleProject = projects.length === 1 ? projects[0] : undefined
  rejectInvalidDatabaseTarget(singleProject, cli)
  const token = await readOrLoginToken(singleProject?.dir ?? projectDir, cli)
  await preflightDeployLimits(projects, cli, token, cli.database)
  const results = await deployProjects(projects, cli, token)

  if (cli.wait) {
    await waitForWorkspaceBuilds(cli, token, results)
  }

  const singleResult = results.length === 1 ? results[0] : undefined
  if (singleResult) {
    printDeployResult(singleResult.response, cli)
    return
  }

  printWorkspaceDeployResults(projectDir, results, cli)
}

function rejectInvalidDatabaseTarget(singleProject: DeployProjectInfo | undefined, cli: CliOptions): void {
  if (singleProject?.kind === 'static_frontend' && cli.database) {
    throw agentError('invalid_database_target', 'Static frontends cannot attach managed Postgres directly.', 'Deploy a Rust backend with managed Postgres and call it from the frontend.', cli.json)
  }
}

async function deployProjects(projects: DeployProjectInfo[], cli: CliOptions, token: string): Promise<WorkspaceDeployResult[]> {
  const results: WorkspaceDeployResult[] = []
  const workspaceDeploy = projects.length > 1
  if (workspaceDeploy && !cli.json) {
    console.log(`deploying ${projects.length} projects`)
  }

  for (const project of projects) {
    const wantsDatabase = projectWantsDatabase(project, cli.database)
    if (workspaceDeploy && !cli.json) {
      console.log(`checking ${project.relative}`)
    }
    const response = await deployProject(project.dir, cli, token, wantsDatabase)
    results.push({ project, wantsDatabase, response })
    if (workspaceDeploy && !cli.json) {
      console.log(`${project.relative} queued ${response.build_job.id}`)
      console.log(`${project.relative} url ${response.app.url}`)
    }
  }

  return results
}

function projectWantsDatabase(project: DeployProjectInfo, databaseRequested: boolean): boolean {
  return databaseRequested && project.kind === 'rust_backend'
}

async function preflightDeployLimits(projects: DeployProjectInfo[], cli: CliOptions, token: string, databaseRequested: boolean): Promise<void> {
  const [usageResponse, appsResponse] = await Promise.all([
    apiRequest(cli, 'GET', '/v1/usage', token, null),
    apiRequest(cli, 'GET', '/v1/apps', token, null)
  ])
  const usageRoot = jsonObjectOrEmpty(usageResponse)
  const usage = jsonObjectField(usageRoot, 'usage')
  const limits = jsonObjectField(usageRoot, 'limits')
  const apps = appsResponseFromJson(appsResponse).apps
  const existingApps = new Map<string, AppSummary>()
  for (const app of apps) {
    if (app.name) {
      existingApps.set(app.name, app)
    }
  }
  let newProjects = 0
  let newDatabases = 0

  for (const project of projects) {
    if (!project.name || project.kind === 'unknown') {
      continue
    }
    const existing = existingApps.get(project.name)
    if (!existing) {
      newProjects += 1
    }
    if (projectWantsDatabase(project, databaseRequested) && existing?.databaseStorageMib === undefined) {
      newDatabases += 1
    }
  }

  const usedProjects = numberField(usage, 'appCount')
  const projectLimit = numberField(limits, 'projects')
  const usedDatabases = numberField(usage, 'databaseCount')
  const databaseLimit = numberField(limits, 'managedDatabases')

  if (newProjects > 0 && usedProjects + newProjects > projectLimit) {
    throw agentError(
      'payment_required',
      `Project limit reached: ${usedProjects}/${projectLimit} projects are already used.`,
      'Redeploy an existing app by reusing its `name` in zerct.toml, or run `npx @zerct/zerct billing` to open Stripe Checkout before creating another project.',
      cli.json
    )
  }

  if (newDatabases > 0 && usedDatabases + newDatabases > databaseLimit) {
    throw agentError(
      'payment_required',
      `Managed Postgres limit reached: ${usedDatabases}/${databaseLimit} databases are already used.`,
      'Redeploy an app that already has managed Postgres, deploy without `--database`, or run `npx @zerct/zerct billing` to open Stripe Checkout.',
      cli.json
    )
  }
}

async function deployProject(projectDir: string, cli: CliOptions, token: string, wantsDatabase: boolean): Promise<DeployResponse> {
  const report = runDoctor(projectDir)
  if (!report.ok) {
    const firstFailure = report.checks.find((check) => !check.ok)
    throw agentError('doctor_failed', 'Zerct doctor failed.', firstFailure?.agent_instruction ?? 'Fix the failed checks and retry.', cli.json)
  }

  const body: JsonObject = {
    config: report.config,
    commit_sha: gitCommitSha(projectDir),
    wants_database: wantsDatabase,
    source_archive_base64: createArchiveBase64(projectDir)
  }

  return deployResponseFromJson(await apiRequest(cli, 'POST', '/v1/deploy', token, body))
}

function printDeployResult(response: DeployResponse, cli: CliOptions): void {
  if (cli.json) {
    printJson(response)
    return
  }

  console.log(`queued ${response.build_job.id}`)
  console.log(`app ${response.app.id}`)
  console.log(`url ${response.app.url}`)
  console.log(`next npx @zerct/zerct logs --app ${response.app.id}`)
}

function printWorkspaceDeployResults(projectDir: string, results: WorkspaceDeployResult[], cli: CliOptions): void {
  if (cli.json) {
    printJson({
      workspace: projectDir,
      deploys: results.map((result) => ({
        path: result.project.relative,
        kind: result.project.kind,
        wants_database: result.wantsDatabase,
        app: result.response.app,
        build_job: result.response.build_job,
        final_build: result.finalBuild ?? null
      }))
    })
    return
  }

  const firstApp = results[0]?.response.app.id
  if (firstApp) {
    console.log(`next npx @zerct/zerct logs --app ${firstApp}`)
  }
}

async function waitForWorkspaceBuilds(cli: CliOptions, token: string, results: WorkspaceDeployResult[]): Promise<void> {
  await Promise.all(results.map(async (result): Promise<void> => {
    const finalBuild = await waitForBuild(cli, token, result.response.build_job.id)
    result.finalBuild = finalBuild
    result.response.final_build = finalBuild
  }))
}

async function waitForBuild(cli: CliOptions, token: string, buildId: string): Promise<BuildRecord> {
  const deadline = Date.now() + cli.waitTimeoutSeconds * 1000
  let lastStatus = ''

  while (Date.now() <= deadline) {
    const response = buildStatusResponseFromJson(await apiRequest(cli, 'GET', `/v1/builds/${encodeURIComponent(buildId)}`, token, null))
    const build = response.build
    if (!build?.status) {
      throw agentError('build_status_unavailable', 'Build status is unavailable.', `Retry with \`npx @zerct/zerct logs --build ${buildId}\`.`, cli.json)
    }

    if (build.status !== lastStatus) {
      progress(cli, `build ${build.id} ${build.status}`)
      lastStatus = build.status
    }
    if (['succeeded', 'failed', 'canceled'].includes(build.status)) {
      return build
    }
    await sleep(3000)
  }

  throw agentError(
    'build_wait_timeout',
    `Timed out waiting for build ${buildId}.`,
    `Run \`npx @zerct/zerct logs --build ${buildId}\` to continue watching.`,
    cli.json
  )
}

export {
  deploy
}
