import { spawnSync } from 'node:child_process'
import { existsSync, readFileSync } from 'node:fs'
import path from 'node:path'
import { agentError } from './errors.ts'
import { parseZerctToml, validateConfig } from './config.ts'
import { discoverDeployProjects } from './workspace.ts'
import { printJson, scanUnsafe } from './project.ts'
import { frontendLockfileExists, frontendScriptChecks, frontendSourceChecks } from './frontend-policy.ts'
import type { DoctorCheck, DoctorReport, WorkspaceDoctorReport, ZerctConfig } from './types.ts'

interface CargoCheckSpec {
  name: string
  args: string[]
  missing: string
  failed: string
}

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
      checks.push({ name: 'zerct.toml', ok: true, message: 'valid', agent_instruction: 'Config is valid.' })
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
      agent_instruction: `Create and commit ${file}, then retry.`
    })
  }

  if (kind === 'static_frontend') {
    const hasLockfile = frontendLockfileExists(projectDir)
    checks.push({
      name: 'frontend lockfile',
      ok: hasLockfile,
      message: hasLockfile ? 'found' : 'missing',
      agent_instruction: 'Commit package-lock.json, pnpm-lock.yaml, yarn.lock, bun.lock, or bun.lockb, then retry.'
    })
    checks.push(...frontendSourceChecks(projectDir))
    checks.push(...frontendScriptChecks(projectDir, configValid))
  } else {
    checks.push(cargoLints(projectDir))
  }

  const unsafeHits = scanUnsafe(projectDir)
  checks.push({
    name: 'unsafe',
    ok: unsafeHits.length === 0,
    message: unsafeHits.length === 0 ? 'no direct unsafe found' : unsafeHits.slice(0, 5).join(', '),
    agent_instruction: 'Remove direct unsafe usage from workspace Rust source before deploying.'
  })
  if (kind === 'rust_backend' && configValid) {
    checks.push(cargoFmt(projectDir))
    checks.push(cargoCheck(projectDir))
    checks.push(cargoClippy(projectDir))
  }

  return {
    ok: checks.every((check) => check.ok),
    project: projectDir,
    config,
    checks
  }
}

function cargoCheck(projectDir: string): DoctorCheck {
  return cargoCommandCheck(projectDir, {
    name: 'cargo check',
    args: ['check', '--locked', '--quiet'],
    missing: 'Install Rust and Cargo, then run `cargo check --locked` locally before deploying.',
    failed: 'Run `cargo check --locked`, fix every compiler error and warning, then redeploy.'
  })
}

function cargoFmt(projectDir: string): DoctorCheck {
  return cargoCommandCheck(projectDir, {
    name: 'cargo fmt',
    args: ['fmt', '--all', '--check'],
    missing: 'Install rustfmt with Rust, then run `cargo fmt --all --check` before deploying.',
    failed: 'Run `cargo fmt --all`, then redeploy.'
  })
}

function cargoClippy(projectDir: string): DoctorCheck {
  return cargoCommandCheck(projectDir, {
    name: 'cargo clippy',
    args: ['clippy', '--locked', '--all-targets', '--all-features', '--quiet', '--', '-D', 'warnings'],
    missing: 'Install Rust clippy, then run `cargo clippy --locked --all-targets --all-features -- -D warnings` before deploying.',
    failed: 'Run `cargo clippy --locked --all-targets --all-features -- -D warnings`, fix every warning, then redeploy.'
  })
}

function cargoCommandCheck(projectDir: string, check: CargoCheckSpec): DoctorCheck {
  const cargo = spawnSync('cargo', check.args, {
    cwd: projectDir,
    encoding: 'utf8',
    env: { ...process.env, CARGO_TERM_COLOR: 'never' },
    stdio: ['ignore', 'pipe', 'pipe']
  })

  if (cargo.error) {
    return {
      name: check.name,
      ok: false,
      message: cargo.error.message,
      agent_instruction: check.missing
    }
  }

  return {
    name: check.name,
    ok: cargo.status === 0,
    message: cargo.status === 0 ? 'passed' : (cargo.stderr || cargo.stdout || `${check.name} failed`).trim().slice(0, 240),
    agent_instruction: check.failed
  }
}

function cargoLints(projectDir: string): DoctorCheck {
  const cargoToml = path.join(projectDir, 'Cargo.toml')
  let source = ''
  try {
    source = readFileSync(cargoToml, 'utf8')
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : String(error)
    return {
      name: 'cargo lints',
      ok: false,
      message,
      agent_instruction: 'Create Cargo.toml with strict Rust lints, then retry.'
    }
  }

  const ok = cargoLintLevel(source, 'unsafe_code') === 'forbid' &&
    cargoLintLevel(source, 'warnings') === 'deny'
  return {
    name: 'cargo lints',
    ok,
    message: ok ? 'strict' : 'missing unsafe_code=forbid or warnings=deny',
    agent_instruction: 'Add `[lints.rust]` with `unsafe_code = "forbid"` and `warnings = "deny"`, then retry.'
  }
}

function cargoLintLevel(source: string, lintName: string): string {
  let section = ''
  for (const rawLine of source.split(/\r?\n/u)) {
    const line = rawLine.replace(/#.*/u, '').trim()
    const sectionMatch = line.match(/^\[([^\]]+)\]$/u)
    if (sectionMatch) {
      section = sectionMatch[1] ?? ''
      continue
    }
    if (section !== 'lints.rust' && section !== 'workspace.lints.rust') {
      continue
    }
    const assignment = line.match(/^([A-Za-z0-9_]+)\s*=\s*(?:"([^"]+)"|\{[^}]*level\s*=\s*"([^"]+)")/u)
    if (assignment?.[1] === lintName) {
      return assignment[2] ?? assignment[3] ?? ''
    }
  }

  return ''
}

function isWorkspaceDoctorReport(report: DoctorReport | WorkspaceDoctorReport): report is WorkspaceDoctorReport {
  return 'projects' in report
}

export { doctorProject, runDoctor, runDoctorWorkspace }
