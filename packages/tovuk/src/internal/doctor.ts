import { existsSync, readFileSync } from 'node:fs'
import path from 'node:path'
import { doctorCheck } from './checks.ts'
import { agentError } from './errors.ts'
import { parseTovukToml, validateConfig } from './config.ts'
import { discoverDeployProjects } from './workspace.ts'
import { printJson } from './project.ts'
import { rustDoctorChecks, unsafeCheck } from './rust-doctor.ts'
import { frontendLockfileExists, frontendScriptChecks, frontendSourceChecks, isPlainStaticFrontend } from './frontend-policy.ts'
import type { DoctorCheck, DoctorReport, WorkspaceDoctorReport, TovukConfig } from './types.ts'

function doctorProject(projectDir: string, json: boolean): void {
  const report = runDoctorWorkspace(projectDir)
  if (json) {
    printJson(report)
    if (!report.ok) {
      process.exitCode = 1
    }
    return
  }

  printDoctorReport(report)

  if (!report.ok) {
    const firstFailure = doctorChecks(report).find((check) => !check.ok)
    throw agentError('doctor_failed', 'Tovuk doctor failed.', firstFailure?.agent_instruction || 'Fix the failed checks and retry `npx tovuk doctor`.', json)
  }
}

function runDoctorWorkspace(projectDir: string): DoctorReport | WorkspaceDoctorReport {
  if (existsSync(path.join(projectDir, 'tovuk.toml'))) {
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
  const configResult = readConfig(projectDir)
  const checks: DoctorCheck[] = [configResult.check]
  const kind = configResult.config?.kind || 'rust_backend'
  if (kind === 'fullstack' && configResult.config) {
    checks.push(...fullstackChecks(projectDir, configResult.config, configResult.valid))
    return doctorReport(projectDir, configResult.config, checks)
  }

  checks.push(...requiredFileChecks(projectDir, kind))
  if (kind === 'static_frontend') {
    checks.push(...staticFrontendChecks(projectDir, configResult.valid))
  } else {
    checks.push(...rustDoctorChecks(projectDir, configResult.valid))
  }

  if (kind === 'static_frontend') {
    checks.push(unsafeCheck(projectDir))
  }

  return doctorReport(projectDir, configResult.config, checks)
}

function doctorReport(projectDir: string, config: TovukConfig | null, checks: DoctorCheck[]): DoctorReport {
  return {
    ok: checks.every((check) => check.ok),
    project: projectDir,
    config,
    checks
  }
}

function printDoctorReport(report: DoctorReport | WorkspaceDoctorReport): void {
  if (isWorkspaceDoctorReport(report)) {
    report.projects.forEach(printProjectReport)
    return
  }
  printChecks(report.checks)
}

function printProjectReport(report: DoctorReport & { relative: string }): void {
  console.log(`project ${report.relative}`)
  printChecks(report.checks)
}

function printChecks(checks: readonly DoctorCheck[]): void {
  for (const check of checks) {
    console.log(`${check.ok ? 'ok' : 'fail'} ${check.name}${check.message ? ` - ${check.message}` : ''}`)
  }
}

function doctorChecks(report: DoctorReport | WorkspaceDoctorReport): DoctorCheck[] {
  return isWorkspaceDoctorReport(report)
    ? report.projects.flatMap((project) => project.checks)
    : report.checks
}

function readConfig(projectDir: string): { check: DoctorCheck; config: TovukConfig | null; valid: boolean } {
  const configPath = path.join(projectDir, 'tovuk.toml')
  if (!existsSync(configPath)) {
    return {
      check: { name: 'tovuk.toml', ok: false, message: 'missing', agent_instruction: 'Create and commit tovuk.toml, then retry.' },
      config: null,
      valid: false
    }
  }
  try {
    const config = parseTovukToml(readFileSync(configPath, 'utf8'), projectDir)
    validateConfig(config)
    return { check: { name: 'tovuk.toml', ok: true, message: 'valid', agent_instruction: null }, config, valid: true }
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : String(error)
    return { check: { name: 'tovuk.toml', ok: false, message, agent_instruction: `Fix tovuk.toml: ${message}.` }, config: null, valid: false }
  }
}

function requiredFileChecks(projectDir: string, kind: TovukConfig['kind']): DoctorCheck[] {
  return requiredFiles(projectDir, kind).map((file) => {
    const ok = existsSync(path.join(projectDir, file))
    return doctorCheck(file, ok, 'found', 'missing', `Create and commit ${file}, then retry.`)
  })
}

function requiredFiles(projectDir: string, kind: TovukConfig['kind']): string[] {
  if (kind === 'static_frontend') {
    return isPlainStaticFrontend(projectDir) ? ['index.html'] : ['package.json']
  }
  return ['Cargo.toml', 'Cargo.lock']
}

function fullstackChecks(projectDir: string, config: TovukConfig, configValid: boolean): DoctorCheck[] {
  const backendRoot = config.backend.root || ''
  const frontendRoot = config.frontend.root || ''
  const backendDir = path.join(projectDir, backendRoot)
  const frontendDir = path.join(projectDir, frontendRoot)
  return [
    ...requiredFilesAt(backendDir, backendRoot, ['Cargo.toml', 'Cargo.lock']),
    ...rustDoctorChecks(backendDir, configValid),
    ...requiredFilesAt(frontendDir, frontendRoot, isPlainStaticFrontend(frontendDir) ? ['index.html'] : ['package.json']),
    ...staticFrontendChecks(frontendDir, configValid)
  ]
}

function requiredFilesAt(projectDir: string, label: string, files: string[]): DoctorCheck[] {
  return files.map((file) => {
    const display = label ? `${label}/${file}` : file
    const ok = existsSync(path.join(projectDir, file))
    return doctorCheck(display, ok, 'found', 'missing', `Create and commit ${display}, then retry.`)
  })
}

function staticFrontendChecks(projectDir: string, configValid: boolean): DoctorCheck[] {
  if (isPlainStaticFrontend(projectDir)) {
    return []
  }
  return [
    frontendLockfileCheck(projectDir),
    ...frontendSourceChecks(projectDir),
    ...frontendScriptChecks(projectDir, configValid)
  ]
}

function frontendLockfileCheck(projectDir: string): DoctorCheck {
  const ok = frontendLockfileExists(projectDir)
  return doctorCheck('frontend lockfile', ok, 'found', 'missing', 'Commit package-lock.json, pnpm-lock.yaml, yarn.lock, bun.lock, or bun.lockb, then retry.')
}

function isWorkspaceDoctorReport(report: DoctorReport | WorkspaceDoctorReport): report is WorkspaceDoctorReport {
  return 'projects' in report
}

export { doctorProject, runDoctor, runDoctorWorkspace }
