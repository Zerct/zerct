import { DEFAULT_RUST_CHECK_COMMAND, PROJECT_KINDS } from './constants.ts'
import { frontendBuildCommand, frontendCheckCommand } from './frontend-policy.ts'
import type { BackendConfig, BuildConfig, FrontendConfig, ProjectKind, ResourceConfig, RunConfig, TovukConfig } from './types.ts'

type SectionName = 'backend' | 'build' | 'frontend' | 'resources' | 'root' | 'run'
type TomlValue = boolean | number | string
type SectionAssigner = (config: MutableTovukConfig, key: string, value: TomlValue) => void

interface MutableSection {
  [key: string]: number | string | undefined
}

interface MutableTovukConfig {
  name?: string
  kind?: ProjectKind
  build: MutableSection
  run: MutableSection
  frontend: MutableSection
  backend: MutableSection
  resources: MutableSection
}

const SECTION_FIELD_TYPES: Readonly<Record<Exclude<SectionName, 'root'>, { strings: ReadonlySet<string>; numbers: ReadonlySet<string> }>> = {
  backend: fieldTypes(['root', 'check', 'build', 'command', 'health'], ['port']),
  build: fieldTypes(['command', 'check', 'output'], []),
  frontend: fieldTypes(['root', 'check', 'build', 'output'], []),
  resources: fieldTypes(['memory', 'cpu'], ['idle_timeout_minutes']),
  run: fieldTypes(['command', 'health'], ['port'])
}

const SECTION_ASSIGNERS: Readonly<Record<SectionName, SectionAssigner>> = {
  backend: (config, key, value): void => assignSectionValue(config.backend, key, value, 'backend'),
  build: (config, key, value): void => assignSectionValue(config.build, key, value, 'build'),
  frontend: (config, key, value): void => assignSectionValue(config.frontend, key, value, 'frontend'),
  resources: (config, key, value): void => assignSectionValue(config.resources, key, value, 'resources'),
  root: assignRootValue,
  run: (config, key, value): void => assignSectionValue(config.run, key, value, 'run')
}

function parseTovukToml(source: string, projectDir: string): TovukConfig {
  const config: MutableTovukConfig = {
    build: {},
    run: {},
    frontend: {},
    backend: {},
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
    frontend: frontendConfig(config, kind, projectDir),
    backend: backendConfig(config, kind),
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
  if (value === 'backend' || value === 'build' || value === 'frontend' || value === 'run' || value === 'resources') {
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

function assignSectionValue(sectionValues: MutableSection, key: string, value: TomlValue, section: Exclude<SectionName, 'root'>): void {
  const schema = SECTION_FIELD_TYPES[section]
  if (schema.strings.has(key)) {
    sectionValues[key] = expectString(key, value)
    return
  }
  if (schema.numbers.has(key)) {
    sectionValues[key] = expectNumber(key, value)
    return
  }
  throw new Error(`unsupported [${section}] key ${key}`)
}

function fieldTypes(strings: readonly string[], numbers: readonly string[]): { strings: ReadonlySet<string>; numbers: ReadonlySet<string> } {
  return {
    strings: new Set(strings),
    numbers: new Set(numbers)
  }
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
  throw new Error('kind must be fullstack, rust_backend, or static_frontend')
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
    check: optionalString(config.build['check']) ?? defaultCheckCommand(kind, projectDir),
    command: optionalString(config.build['command']) ?? defaultBuildCommand(kind, projectDir)
  }
  const output = kind === 'static_frontend'
    ? optionalString(config.build['output']) ?? 'dist'
    : optionalString(config.build['output'])
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

function frontendConfig(config: MutableTovukConfig, kind: ProjectKind, projectDir: string): FrontendConfig {
  if (kind !== 'fullstack') {
    return {}
  }
  const root = optionalString(config.frontend['root'])
  const frontendDir = root ? pathJoin(projectDir, root) : projectDir
  const result: FrontendConfig = {
    check: optionalString(config.frontend['check']) ?? frontendCheckCommand(frontendDir),
    build: optionalString(config.frontend['build']) ?? frontendBuildCommand(frontendDir),
    output: optionalString(config.frontend['output']) ?? 'dist'
  }
  assignIfPresent(root, (value) => { result.root = value })
  return result
}

function backendConfig(config: MutableTovukConfig, kind: ProjectKind): BackendConfig {
  if (kind !== 'fullstack') {
    return {}
  }
  const result: BackendConfig = {
    check: optionalString(config.backend['check']) ?? DEFAULT_RUST_CHECK_COMMAND,
    build: optionalString(config.backend['build']) ?? 'cargo build --release',
    port: optionalNumber(config.backend['port']) ?? 3000,
    health: optionalString(config.backend['health']) ?? '/api/healthz'
  }
  assignIfPresent(optionalString(config.backend['root']), (value) => { result.root = value })
  assignIfPresent(optionalString(config.backend['command']), (value) => { result.command = value })
  return result
}

function pathJoin(root: string, relative: string): string {
  return `${root.replace(/\/+$/u, '')}/${relative.replace(/^\/+/u, '')}`
}

function runConfig(config: MutableTovukConfig): RunConfig {
  const run: RunConfig = {
    port: optionalNumber(config.run['port']) ?? 3000,
    health: optionalString(config.run['health']) ?? '/healthz'
  }
  assignIfPresent(optionalString(config.run['command']), (value) => { run.command = value })
  return run
}

function resourceConfig(config: MutableTovukConfig): ResourceConfig {
  return {
    memory: optionalString(config.resources['memory']) ?? '512mb',
    cpu: optionalString(config.resources['cpu']) ?? '0.25',
    idle_timeout_minutes: optionalNumber(config.resources['idle_timeout_minutes']) ?? 15
  }
}

function optionalString(value: number | string | undefined): string | undefined {
  return typeof value === 'string' ? value : undefined
}

function optionalNumber(value: number | string | undefined): number | undefined {
  return typeof value === 'number' ? value : undefined
}

function assignIfPresent(value: string | undefined, assign: (value: string) => void): void {
  if (value) {
    assign(value)
  }
}

export { parseTovukToml }
