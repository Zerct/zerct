import { DEFAULT_RUST_CHECK_COMMAND, PROJECT_KINDS } from './constants.ts'
import { commandTokens, frontendBuildCommand, frontendCheckCommand, hasFrontendInstallCommand, hasFrontendScriptRun, usesJavascriptLinter } from './frontend-policy.ts'
import { isSafeRelativePath } from './project.ts'
import type { BuildConfig, ProjectKind, ResourceConfig, RunConfig, ZerctConfig } from './types.ts'

type SectionName = 'build' | 'resources' | 'root' | 'run'
type TomlValue = boolean | number | string

interface MutableZerctConfig {
  name?: string
  kind?: ProjectKind
  build: Partial<BuildConfig>
  run: Partial<RunConfig>
  resources: Partial<ResourceConfig>
}

function parseZerctToml(source: string, projectDir: string): ZerctConfig {
  const config: MutableZerctConfig = {
    build: {},
    run: {},
    resources: {}
  }
  let section: SectionName = 'root'

  for (const rawLine of source.split(/\r?\n/u)) {
    const line = rawLine.trim()
    if (!line || line.startsWith('#')) {
      continue
    }

    const sectionMatch = line.match(/^\[([a-z_]+)\]$/u)
    if (sectionMatch) {
      section = parseSection(sectionMatch[1] ?? '')
      continue
    }

    const assignment = line.match(/^([a-z_]+)\s*=\s*(.+)$/u)
    if (!assignment) {
      throw new Error(`invalid line: ${line}`)
    }

    assignTomlValue(config, section, assignment[1] ?? '', parseTomlValue(assignment[2] ?? ''))
  }

  const kind = config.kind ?? 'rust_backend'
  const build: BuildConfig = {
    check: config.build.check ?? (kind === 'static_frontend' ? frontendCheckCommand(projectDir) : DEFAULT_RUST_CHECK_COMMAND),
    command: config.build.command ?? (kind === 'static_frontend' ? frontendBuildCommand(projectDir) : 'cargo build --release')
  }
  if (kind === 'static_frontend') {
    build.output = config.build.output ?? 'dist'
  } else if (typeof config.build.output === 'string') {
    build.output = config.build.output
  }

  const run: RunConfig = {
    port: config.run.port ?? 3000,
    health: config.run.health ?? '/healthz'
  }
  if (typeof config.run.command === 'string') {
    run.command = config.run.command
  }

  const resources: ResourceConfig = {
    memory: config.resources.memory ?? '512mb',
    cpu: config.resources.cpu ?? '0.25',
    idle_timeout_minutes: config.resources.idle_timeout_minutes ?? 15
  }

  const result: ZerctConfig = { kind, build, run, resources }
  if (config.name) {
    result.name = config.name
  }
  return result
}

function parseSection(value: string): SectionName {
  if (value === 'build' || value === 'run' || value === 'resources') {
    return value
  }
  throw new Error(`unsupported section [${value}]`)
}

function assignTomlValue(config: MutableZerctConfig, section: SectionName, key: string, value: TomlValue): void {
  if (section === 'root') {
    assignRootValue(config, key, value)
    return
  }
  if (section === 'build') {
    assignBuildValue(config.build, key, value)
    return
  }
  if (section === 'run') {
    assignRunValue(config.run, key, value)
    return
  }
  assignResourceValue(config.resources, key, value)
}

function assignRootValue(config: MutableZerctConfig, key: string, value: TomlValue): void {
  if (key === 'name') {
    config.name = expectString(key, value)
    return
  }
  if (key === 'kind') {
    config.kind = expectProjectKind(expectString(key, value))
    return
  }
  throw new Error(`unsupported root key ${key}`)
}

function assignBuildValue(build: Partial<BuildConfig>, key: string, value: TomlValue): void {
  if (key === 'command' || key === 'check' || key === 'output') {
    build[key] = expectString(key, value)
    return
  }
  throw new Error(`unsupported [build] key ${key}`)
}

function assignRunValue(run: Partial<RunConfig>, key: string, value: TomlValue): void {
  if (key === 'command' || key === 'health') {
    run[key] = expectString(key, value)
    return
  }
  if (key === 'port') {
    run.port = expectNumber(key, value)
    return
  }
  throw new Error(`unsupported [run] key ${key}`)
}

function assignResourceValue(resources: Partial<ResourceConfig>, key: string, value: TomlValue): void {
  if (key === 'memory' || key === 'cpu') {
    resources[key] = expectString(key, value)
    return
  }
  if (key === 'idle_timeout_minutes') {
    resources.idle_timeout_minutes = expectNumber(key, value)
    return
  }
  throw new Error(`unsupported [resources] key ${key}`)
}

function expectString(key: string, value: TomlValue): string {
  if (typeof value === 'string') {
    return value
  }
  throw new Error(`${key} must be a string`)
}

function expectNumber(key: string, value: TomlValue): number {
  if (typeof value === 'number') {
    return value
  }
  throw new Error(`${key} must be a number`)
}

function expectProjectKind(value: string): ProjectKind {
  if (isProjectKind(value)) {
    return value
  }
  throw new Error('kind must be rust_backend or static_frontend')
}

function isProjectKind(value: string): value is ProjectKind {
  return PROJECT_KINDS.has(value)
}

function parseTomlValue(raw: string): TomlValue {
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

function validateConfig(config: ZerctConfig): void {
  if (!/^[a-z0-9](?:[a-z0-9-]{0,46}[a-z0-9])?$/u.test(config.name ?? '')) {
    throw new Error('name must be lowercase DNS-safe text up to 48 characters')
  }
  if (!PROJECT_KINDS.has(config.kind)) {
    throw new Error('kind must be rust_backend or static_frontend')
  }
  if (!config.build.command.trim()) {
    throw new Error('[build].command is required')
  }
  if (!config.build.check.trim()) {
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
  if (!config.run.command?.trim()) {
    throw new Error('[run].command is required')
  }
  if (!Number.isInteger(config.run.port) || config.run.port < 1 || config.run.port > 65535) {
    throw new Error('[run].port must be between 1 and 65535')
  }
  if (!config.run.health.startsWith('/')) {
    throw new Error('[run].health must be an absolute path')
  }
  if (!/^\d+\s*(mb|mib|gb|gib)$/iu.test(config.resources.memory)) {
    throw new Error('[resources].memory must look like 512mb or 1gb')
  }
  if (!/^\d+(?:\.\d{1,3})?$/u.test(config.resources.cpu)) {
    throw new Error('[resources].cpu must look like 0.25, 0.5, 1, or 2')
  }
}

function validateCheckCommand(kind: ProjectKind, command: string): void {
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

function validateFrontendCheckCommand(command: string): void {
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

export {
  parseZerctToml,
  parseTomlValue,
  validateConfig,
  validateCheckCommand,
  validateFrontendCheckCommand,
  isProjectKind
}
