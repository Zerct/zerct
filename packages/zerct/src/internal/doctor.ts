import { existsSync, readFileSync } from 'node:fs'
import path from 'node:path'
import { agentError } from './errors.ts'
import { parseZerctToml, validateConfig } from './config.ts'
import { discoverDeployProjects } from './workspace.ts'
import { printJson } from './project.ts'
import { rustDoctorChecks, unsafeCheck } from './rust-doctor.ts'
import { frontendLockfileExists, frontendScriptChecks, frontendSourceChecks } from './frontend-policy.ts'
import type { DoctorCheck, DoctorReport, WorkspaceDoctorReport, ZerctConfig } from './types.ts'

function doctorProject(projectDir: string, json: boolean): void {
  const report = runDoctorWorkspace(projectDir)
  if (json) {
    printJson(report)
    if (!report.ok) {
      process.exitCode = 1
    }
    return
  }

  if (isWorkspaceDoctorReport(report)) {
    for (const project of report.projects) {
      console.log(`project ${project.relative}`)
      for (const check of project.checks) {
        console.log(`${check.ok ? 'ok' : 'fail'} ${check.name}${check.message ? ` - ${check.message}` : ''}`)
      }
    }
  } else {
    for (const check of report.checks) {
      console.log(`${check.ok ? 'ok' : 'fail'} ${check.name}${check.message ? ` - ${check.message}` : ''}`)
    }
  }

  if (!report.ok) {
    const checks = isWorkspaceDoctorReport(report)
      ? report.projects.flatMap((project) => project.checks)
      : report.checks
    const firstFailure = checks.find((check) => !check.ok)
    throw agentError('doctor_failed', 'Zerct doctor failed.', firstFailure?.agent_instruction || 'Fix the failed checks and retry `npx @zerct/zerct doctor`.', json)
  }
}

function runDoctorWorkspace(projectDir: string): DoctorReport | WorkspaceDoctorReport {
  if (existsSync(path.join(projectDir, 'zerct.toml'))) {
    return runDoctor(projectDir)
  }

  const projects = discoverDeployProjects(projectDir)
  if (projects.length === 0) {
    return runDoctor(projectDir)
  }

  const reports = projects.map((project) => {
    const report = runDoctor(project.dir)
    return {
      relative: project.relative,
      ok: report.ok,
      project: report.project,
      config: report.config,
      checks: report.checks
    }
  })
  return {
    ok: reports.every((report) => report.ok),
    workspace: projectDir,
    projects: reports
  }
}

function runDoctor(projectDir: string): DoctorReport {
  const checks: DoctorCheck[] = []
  let config: ZerctConfig | null = null
  let configValid = false
  const configPath = path.join(projectDir, 'zerct.toml')
  if (existsSync(configPath)) {
    try {
      config = parseZerctToml(readFileSync(configPath, 'utf8'), projectDir)
      validateConfig(config)
      configValid = true
      checks.push({ name: 'zerct.toml', ok: true, message: 'valid', agent_instruction: null })
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error)
      checks.push({
        name: 'zerct.toml',
        ok: false,
        message,
        agent_instruction: `Fix zerct.toml: ${message}.`
      })
    }
  } else {
    checks.push({
      name: 'zerct.toml',
      ok: false,
      message: 'missing',
      agent_instruction: 'Create and commit zerct.toml, then retry.'
    })
  }

  const kind = config?.kind || 'rust_backend'
  const requiredFiles = kind === 'static_frontend'
    ? ['package.json']
    : ['Cargo.toml', 'Cargo.lock']
  for (const file of requiredFiles) {
    const ok = existsSync(path.join(projectDir, file))
    checks.push({
      name: file,
      ok,
      message: ok ? 'found' : 'missing',
      agent_instruction: ok ? null : `Create and commit ${file}, then retry.`
    })
  }

  if (kind === 'static_frontend') {
    const hasLockfile = frontendLockfileExists(projectDir)
    checks.push({
      name: 'frontend lockfile',
      ok: hasLockfile,
      message: hasLockfile ? 'found' : 'missing',
      agent_instruction: hasLockfile ? null : 'Commit package-lock.json, pnpm-lock.yaml, yarn.lock, bun.lock, or bun.lockb, then retry.'
    })
    checks.push(...frontendSourceChecks(projectDir))
    checks.push(...frontendScriptChecks(projectDir, configValid))
  } else {
    checks.push(...rustDoctorChecks(projectDir, configValid))
  }

  if (kind === 'static_frontend') {
    checks.push(unsafeCheck(projectDir))
  }

  return {
    ok: checks.every((check) => check.ok),
    project: projectDir,
    config,
    checks
  }
}

function isWorkspaceDoctorReport(report: DoctorReport | WorkspaceDoctorReport): report is WorkspaceDoctorReport {
  return 'projects' in report
}

export { doctorProject, runDoctor, runDoctorWorkspace }
