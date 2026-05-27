import { spawnSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import path from 'node:path'
import { scanUnsafe } from './project.ts'
import type { DoctorCheck } from './types.ts'

interface CargoCheckSpec {
  name: string
  args: string[]
  missing: string
  failed: string
}

function rustDoctorChecks(projectDir: string, configValid: boolean): DoctorCheck[] {
  const checks = [cargoLints(projectDir), unsafeCheck(projectDir)]
  if (configValid) {
    checks.push(cargoFmt(projectDir), cargoCheck(projectDir), cargoClippy(projectDir))
  }
  return checks
}

function unsafeCheck(projectDir: string): DoctorCheck {
  const unsafeHits = scanUnsafe(projectDir)
  return {
    name: 'unsafe',
    ok: unsafeHits.length === 0,
    message: unsafeHits.length === 0 ? 'no direct unsafe found' : unsafeHits.slice(0, 5).join(', '),
    agent_instruction: 'Remove direct unsafe usage from workspace Rust source before deploying.'
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

export { rustDoctorChecks, unsafeCheck }
