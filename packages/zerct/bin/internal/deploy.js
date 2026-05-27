import { agentError } from './errors.js'
import { apiRequest } from './api.js'
import { createArchiveBase64, gitCommitSha } from './archive.js'
import { readOrLoginToken } from './auth.js'
import { runDoctor } from './doctor.js'
import { discoverDeployProjects } from './workspace.js'
import { progress, sleep } from './project.js'

async function deploy(projectDir, cli) {
  const projects = discoverDeployProjects(projectDir)
  if (projects.length === 0) {
    throw agentError('missing_project_contract', 'No zerct.toml was found.', 'Run `npx @zerct/zerct init` in each app directory, or pass a project path.', cli.json)
  }

  if (projects.length === 1) {
    const project = projects[0]
    if (project.kind === 'static_frontend' && cli.database) {
      throw agentError('invalid_database_target', 'Static frontends cannot attach managed Postgres directly.', 'Deploy a Rust backend with managed Postgres and call it from the frontend.', cli.json)
    }
    const token = await readOrLoginToken(project.dir, cli)
    await preflightDeployLimits([project], cli, token, cli.database)
    const result = await deployProject(project.dir, cli, token, cli.database)
    if (cli.wait) {
      result.final_build = await waitForBuild(cli, token, result.build_job.id)
    }
    printDeployResult(result, cli)
    return
  }

  const token = await readOrLoginToken(projectDir, cli)
  await preflightDeployLimits(projects, cli, token, cli.database)
  const results = []
  if (!cli.json) {
    console.log(`deploying ${projects.length} projects`)
  }

  for (const project of projects) {
    const wantsDatabase = cli.database && project.kind === 'rust_backend'
    if (!cli.json) {
      console.log(`checking ${project.relative}`)
    }
    const response = await deployProject(project.dir, cli, token, wantsDatabase)
    results.push({ project, wantsDatabase, response })
    if (!cli.json) {
      console.log(`${project.relative} queued ${response.build_job.id}`)
      console.log(`${project.relative} url ${response.app.url}`)
    }
  }

  if (cli.wait) {
    await waitForWorkspaceBuilds(cli, token, results)
  }

  printWorkspaceDeployResults(projectDir, results, cli)
}

async function preflightDeployLimits(projects, cli, token, databaseRequested) {
  const [usageResponse, appsResponse] = await Promise.all([
    apiRequest(cli, 'GET', '/v1/usage', token, null),
    apiRequest(cli, 'GET', '/v1/apps', token, null)
  ])
  const usage = usageResponse?.usage || {}
  const limits = usageResponse?.limits || {}
  const apps = Array.isArray(appsResponse?.apps) ? appsResponse.apps : []
  const existingApps = new Map(apps.map((app) => [app.name, app]))
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
    if (databaseRequested && project.kind === 'rust_backend' && !existing?.databaseStorageMib) {
      newDatabases += 1
    }
  }

  if (newProjects > 0 && Number(usage.appCount) + newProjects > Number(limits.projects)) {
    throw agentError(
      'payment_required',
      `Project limit reached: ${usage.appCount}/${limits.projects} projects are already used.`,
      'Redeploy an existing app by reusing its `name` in zerct.toml, or run `npx @zerct/zerct billing` to open Stripe Checkout before creating another project.',
      cli.json
    )
  }

  if (newDatabases > 0 && Number(usage.databaseCount) + newDatabases > Number(limits.managedDatabases)) {
    throw agentError(
      'payment_required',
      `Managed Postgres limit reached: ${usage.databaseCount}/${limits.managedDatabases} databases are already used.`,
      'Redeploy an app that already has managed Postgres, deploy without `--database`, or run `npx @zerct/zerct billing` to open Stripe Checkout.',
      cli.json
    )
  }
}

async function deployProject(projectDir, cli, token, wantsDatabase) {
  const report = runDoctor(projectDir)
  if (!report.ok) {
    const firstFailure = report.checks.find((check) => !check.ok)
    throw agentError('doctor_failed', 'Zerct doctor failed.', firstFailure?.agent_instruction || 'Fix the failed checks and retry.', cli.json)
  }

  const archive = createArchiveBase64(projectDir)
  const commitSha = gitCommitSha(projectDir)
  const body = {
    config: report.config,
    commit_sha: commitSha,
    wants_database: wantsDatabase,
    source_archive_base64: archive
  }

  return apiRequest(cli, 'POST', '/v1/deploy', token, body)
}

function printDeployResult(response, cli) {
  if (cli.json) {
    console.log(JSON.stringify(response, null, 2))
    return
  }

  console.log(`queued ${response.build_job.id}`)
  console.log(`app ${response.app.id}`)
  console.log(`url ${response.app.url}`)
  console.log(`next npx @zerct/zerct logs --app ${response.app.id}`)
}

function printWorkspaceDeployResults(projectDir, results, cli) {
  if (cli.json) {
    console.log(JSON.stringify({
      workspace: projectDir,
      deploys: results.map((result) => ({
        path: result.project.relative,
        kind: result.project.kind,
        wants_database: result.wantsDatabase,
        app: result.response.app,
        build_job: result.response.build_job,
        final_build: result.finalBuild || null
      }))
    }, null, 2))
    return
  }

  const firstApp = results[0]?.response?.app?.id
  if (firstApp) {
    console.log(`next npx @zerct/zerct logs --app ${firstApp}`)
  }
}

async function waitForWorkspaceBuilds(cli, token, results) {
  await Promise.all(results.map(async (result) => {
    result.finalBuild = await waitForBuild(cli, token, result.response.build_job.id)
  }))
}

async function waitForBuild(cli, token, buildId) {
  const deadline = Date.now() + cli.waitTimeoutSeconds * 1000
  let lastStatus = ''

  while (Date.now() <= deadline) {
    const response = await apiRequest(cli, 'GET', `/v1/builds/${encodeURIComponent(buildId)}`, token, null)
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

export { deploy, preflightDeployLimits, deployProject, printDeployResult, printWorkspaceDeployResults, waitForWorkspaceBuilds, waitForBuild }
