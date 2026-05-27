import { agentError } from './errors.ts'
import { apiRequest } from './api.ts'
import { buildStatusResponseFromJson, deployResponseFromJson } from './api-models.ts'
import { createArchiveBase64, gitCommitSha } from './archive.ts'
import { readOrLoginToken } from './auth.ts'
import { createDeployPlan } from './deploy-plan.ts'
import { runDoctor } from './doctor.ts'
import { discoverDeployProjects } from './workspace.ts'
import { printJson, progress, sleep } from './project.ts'
import type {
  BuildRecord,
  CliOptions,
  DeployPlanProject,
  DeployResponse,
  JsonObject,
  WorkspaceDeployResult
} from './types.ts'

async function deploy(projectDir: string, cli: CliOptions): Promise<void> {
  const projects = discoverDeployProjects(projectDir)
  if (projects.length === 0) {
    throw agentError('missing_project_contract', 'No zerct.toml was found.', 'Run `npx @zerct/zerct init` in each app directory, or pass a project path.', cli.json)
  }

  const token = await readOrLoginToken(projects.length === 1 ? projects[0]?.dir ?? projectDir : projectDir, cli)
  const plan = await createDeployPlan(projects, cli, token)
  const results = await deployProjects(plan, cli, token)

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

async function deployProjects(plan: DeployPlanProject[], cli: CliOptions, token: string): Promise<WorkspaceDeployResult[]> {
  const results: WorkspaceDeployResult[] = []
  const workspaceDeploy = plan.length > 1
  if (workspaceDeploy && !cli.json) {
    console.log(`deploying ${plan.length} projects`)
  }

  for (const { project, wantsDatabase } of plan) {
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
