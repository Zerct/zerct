import { DEFAULT_RUST_CHECK_COMMAND, PROJECT_KINDS } from './constants.js'
import { commandTokens, frontendBuildCommand, frontendCheckCommand, hasFrontendInstallCommand, hasFrontendScriptRun, usesJavascriptLinter } from './frontend-policy.js'
import { isSafeRelativePath } from './project.js'

function parseZerctToml(source, projectDir) {
  const config = {
    build: {},
    run: {},
    resources: {}
  }
  let section = config

  for (const rawLine of source.split(/\r?\n/u)) {
    const line = rawLine.trim()
    if (!line || line.startsWith('#')) {
      continue
    }

    const sectionMatch = line.match(/^\[([a-z_]+)\]$/u)
    if (sectionMatch) {
      const name = sectionMatch[1]
      if (!['build', 'run', 'resources'].includes(name)) {
        throw new Error(`unsupported section [${name}]`)
      }
      section = config[name]
      continue
    }

    const assignment = line.match(/^([a-z_]+)\s*=\s*(.+)$/u)
    if (!assignment) {
      throw new Error(`invalid line: ${line}`)
    }

    section[assignment[1]] = parseTomlValue(assignment[2])
  }

  config.kind ||= 'rust_backend'
  config.build.check ||= config.kind === 'static_frontend' ? frontendCheckCommand(projectDir) : DEFAULT_RUST_CHECK_COMMAND
  config.build.command ||= config.kind === 'static_frontend' ? frontendBuildCommand(projectDir) : 'cargo build --release'
  if (config.kind === 'static_frontend') {
    config.build.output ||= 'dist'
  }
  config.run.port ||= 3000
  config.run.health ||= '/healthz'
  config.resources.memory ||= '512mb'
  config.resources.cpu ||= '0.25'
  config.resources.idle_timeout_minutes ||= 15
  return config
}

function parseTomlValue(raw) {
  const value = raw.trim()
  if (value.startsWith('"') && value.endsWith('"')) {
    return value.slice(1, -1).replace(/\\"/gu, '"')
  }
  if (value === 'true') {
    return true
  }
  if (value === 'false') {
    return false
  }
  if (/^\d+$/u.test(value)) {
    return Number(value)
  }
  throw new Error(`unsupported TOML value: ${value}`)
}

function validateConfig(config) {
  if (!/^[a-z0-9](?:[a-z0-9-]{0,46}[a-z0-9])?$/u.test(config.name || '')) {
    throw new Error('name must be lowercase DNS-safe text up to 48 characters')
  }
  if (!PROJECT_KINDS.has(config.kind)) {
    throw new Error('kind must be rust_backend or static_frontend')
  }
  if (typeof config.build.command !== 'string' || !config.build.command.trim()) {
    throw new Error('[build].command is required')
  }
  if (typeof config.build.check !== 'string' || !config.build.check.trim()) {
    throw new Error('[build].check is required')
  }
  validateCheckCommand(config.kind, config.build.check)
  if (config.kind === 'static_frontend') {
    if (typeof config.build.output !== 'string' || !isSafeRelativePath(config.build.output)) {
      throw new Error('[build].output must be a safe relative directory like dist')
    }
    return
  }
  if (config.build.output) {
    throw new Error('[build].output is only valid for static_frontend')
  }
  if (!config.run.command || typeof config.run.command !== 'string') {
    throw new Error('[run].command is required')
  }
  if (!Number.isInteger(config.run.port) || config.run.port < 1 || config.run.port > 65535) {
    throw new Error('[run].port must be between 1 and 65535')
  }
  if (typeof config.run.health !== 'string' || !config.run.health.startsWith('/')) {
    throw new Error('[run].health must be an absolute path')
  }
  if (!/^\d+\s*(mb|mib|gb|gib)$/iu.test(config.resources.memory)) {
    throw new Error('[resources].memory must look like 512mb or 1gb')
  }
  if (!/^\d+(?:\.\d{1,3})?$/u.test(config.resources.cpu)) {
    throw new Error('[resources].cpu must look like 0.25, 0.5, 1, or 2')
  }
}

function validateCheckCommand(kind, command) {
  if (kind === 'static_frontend') {
    validateFrontendCheckCommand(command)
    return
  }

  const required = ['cargo fmt --all --check', 'cargo check --locked', 'cargo clippy --locked', '--all-targets', '--all-features', '-D warnings']
  if (required.every((fragment) => command.includes(fragment))) {
    return
  }
  throw new Error('[build].check must include cargo fmt --all --check, cargo check --locked, and cargo clippy --locked --all-targets --all-features -- -D warnings')
}

function validateFrontendCheckCommand(command) {
  if (usesJavascriptLinter(command)) {
    throw new Error('[build].check must not run JavaScript-based lint or format tooling; use oxlint, biome, or deno lint')
  }
  const tokens = commandTokens(command)
  if (
    hasFrontendInstallCommand(tokens) &&
    hasFrontendScriptRun(tokens, 'typecheck') &&
    hasFrontendScriptRun(tokens, 'lint')
  ) {
    return
  }
  throw new Error('[build].check must install dependencies and run package scripts, for example `bun ci && bun run typecheck && bun run lint` or `npm ci --prefer-offline --no-audit --fund=false && npm run typecheck && npm run lint`')
}

export { parseZerctToml, parseTomlValue, validateConfig, validateCheckCommand, validateFrontendCheckCommand }
