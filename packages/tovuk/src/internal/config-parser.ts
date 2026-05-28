import { DEFAULT_RUST_CHECK_COMMAND, PROJECT_KINDS } from './constants.ts'
import { frontendBuildCommand, frontendCheckCommand } from './frontend-policy.ts'
import type { BuildConfig, ProjectKind, ResourceConfig, RunConfig, TovukConfig } from './types.ts'

type SectionName = 'build' | 'resources' | 'root' | 'run'
type TomlValue = boolean | number | string
type SectionAssigner = (config: MutableTovukConfig, key: string, value: TomlValue) => void

interface MutableTovukConfig {
  name?: string
  kind?: ProjectKind
  build: Partial<BuildConfig>
  run: Partial<RunConfig>
  resources: Partial<ResourceConfig>
}

const SECTION_ASSIGNERS: Readonly<Record<SectionName, SectionAssigner>> = {
  build: (config, key, value): void => assignBuildValue(config.build, key, value),
  resources: (config, key, value): void => assignResourceValue(config.resources, key, value),
  root: assignRootValue,
  run: (config, key, value): void => assignRunValue(config.run, key, value)
}

function parseTovukToml(source: string, projectDir: string): TovukConfig {
  const config: MutableTovukConfig = {
    build: {},
    run: {},
    resources: {}
  }
  let section: SectionName = 'root'

  for (const rawLine of source.split(/\r?\n/u)) {
    const parsedLine = parseTomlLine(rawLine)
    if (parsedLine === null) {
      continue
    }
    if (parsedLine.kind === 'section') {
      section = parsedLine.section
      continue
    }
    assignTomlValue(config, section, parsedLine.key, parsedLine.value)
  }

  return tovukConfig(config, projectDir)
}

function tovukConfig(config: MutableTovukConfig, projectDir: string): TovukConfig {
  const kind = config.kind ?? 'rust_backend'
  const result: TovukConfig = {
    kind,
    build: buildConfig(config, kind, projectDir),
    run: runConfig(config),
    resources: resourceConfig(config)
  }
  if (config.name) {
    result.name = config.name
  }
  return result
}

function parseTomlLine(rawLine: string): { kind: 'section'; section: SectionName } | { kind: 'assignment'; key: string; value: TomlValue } | null {
  const line = rawLine.trim()
  if (!line || line.startsWith('#')) {
    return null
  }
  const sectionMatch = line.match(/^\[([a-z_]+)\]$/u)
  if (sectionMatch) {
    return { kind: 'section', section: parseSection(sectionMatch[1] ?? '') }
  }
  const assignment = line.match(/^([a-z_]+)\s*=\s*(.+)$/u)
  if (assignment) {
    return { kind: 'assignment', key: assignment[1] ?? '', value: parseTomlValue(assignment[2] ?? '') }
  }
  throw new Error(`invalid line: ${line}`)
}

function parseSection(value: string): SectionName {
  if (value === 'build' || value === 'run' || value === 'resources') {
    return value
  }
  throw new Error(`unsupported section [${value}]`)
}

function assignTomlValue(config: MutableTovukConfig, section: SectionName, key: string, value: TomlValue): void {
  SECTION_ASSIGNERS[section](config, key, value)
}

function assignRootValue(config: MutableTovukConfig, key: string, value: TomlValue): void {
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

function buildConfig(config: MutableTovukConfig, kind: ProjectKind, projectDir: string): BuildConfig {
  const build: BuildConfig = {
    check: config.build.check ?? defaultCheckCommand(kind, projectDir),
    command: config.build.command ?? defaultBuildCommand(kind, projectDir)
  }
  const output = kind === 'static_frontend'
    ? config.build.output ?? 'dist'
    : config.build.output
  if (typeof output === 'string') {
    build.output = output
  }
  return build
}

function defaultCheckCommand(kind: ProjectKind, projectDir: string): string {
  return kind === 'static_frontend' ? frontendCheckCommand(projectDir) : DEFAULT_RUST_CHECK_COMMAND
}

function defaultBuildCommand(kind: ProjectKind, projectDir: string): string {
  return kind === 'static_frontend' ? frontendBuildCommand(projectDir) : 'cargo build --release'
}

function runConfig(config: MutableTovukConfig): RunConfig {
  const run: RunConfig = {
    port: config.run.port ?? 3000,
    health: config.run.health ?? '/healthz'
  }
  if (typeof config.run.command === 'string') {
    run.command = config.run.command
  }
  return run
}

function resourceConfig(config: MutableTovukConfig): ResourceConfig {
  return {
    memory: config.resources.memory ?? '512mb',
    cpu: config.resources.cpu ?? '0.25',
    idle_timeout_minutes: config.resources.idle_timeout_minutes ?? 15
  }
}

export { parseTovukToml }
