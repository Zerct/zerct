import { JAVASCRIPT_BACKEND_RUNTIMES, PROJECT_KINDS, RUST_STRICT_CLIPPY_DENY_LINTS } from './constants.ts'
import { commandTokens, hasFrontendInstallCommand, hasFrontendScriptRun, usesJavascriptLinter } from './frontend-policy.ts'
import { isSafeRelativePath } from './project.ts'
import type { ProjectKind, TovukConfig } from './types.ts'

function validateConfig(config: TovukConfig): void {
  validateIdentity(config)
  if (config.kind === 'fullstack') {
    validateFullstackConfig(config)
    return
  }
  validateBuildConfig(config)
  if (config.kind === 'static_frontend') {
    validateStaticFrontendConfig(config)
    return
  }
  validateRustBackendConfig(config)
}

function validateIdentity(config: TovukConfig): void {
  if (!/^[a-z0-9](?:[a-z0-9-]{0,46}[a-z0-9])?$/u.test(config.name ?? '')) {
    throw new Error('name must be lowercase DNS-safe text up to 48 characters')
  }
  if (!PROJECT_KINDS.has(config.kind)) {
    throw new Error('kind must be fullstack, rust_backend, or static_frontend')
  }
}

function validateBuildConfig(config: TovukConfig): void {
  if (!config.build.command.trim()) {
    throw new Error('[build].command is required')
  }
  if (!config.build.check.trim()) {
    throw new Error('[build].check is required')
  }
  validateCheckCommand(config.kind, config.build.check)
  if (config.kind === 'rust_backend') {
    validateRustBuildCommand(config.build.command)
  }
}

function validateStaticFrontendConfig(config: TovukConfig): void {
  validateOutput(config.build.output, '[build].output')
}

function validateRustBackendConfig(config: TovukConfig): void {
  if (config.build.output) {
    throw new Error('[build].output is only valid for static_frontend')
  }
  requireCommand(config.run.command, '[run].command')
  validateRustRunCommand(config.run.command)
  validatePort(config.run.port, '[run].port')
  validateHealth(config.run.health, '[run].health')
  validateResourceConfig(config)
}

function validateFullstackConfig(config: TovukConfig): void {
  const backendRoot = validateRoot(config.backend.root, '[backend].root')
  const frontendRoot = validateRoot(config.frontend.root, '[frontend].root')
  if (backendRoot === frontendRoot) {
    throw new Error('[backend].root and [frontend].root must be different directories')
  }
  validateFullstackSections(config)
  validateResourceConfig(config)
}

function validateFullstackSections(config: TovukConfig): void {
  validateRustCheckCommand(requireCommand(config.backend.check, '[backend].check'))
  validateRustBuildCommand(requireCommand(config.backend.build, '[backend].build'))
  validateRustRunCommand(requireCommand(config.backend.command, '[backend].command'))
  validatePort(config.backend.port, '[backend].port')
  validateHealth(config.backend.health, '[backend].health')
  validateFrontendCheckCommand(requireCommand(config.frontend.check, '[frontend].check'))
  requireCommand(config.frontend.build, '[frontend].build')
  validateOutput(config.frontend.output, '[frontend].output')
}

function validateRoot(value: string | undefined, field: string): string {
  if (!value || !isSafeRelativePath(value)) {
    throw new Error(`${field} must be a safe relative directory such as api or web`)
  }
  return value
}

function requireCommand(value: string | undefined, field: string): string {
  if (!value?.trim()) {
    throw new Error(`${field} is required`)
  }
  return value
}

function validatePort(value: number | undefined, field: string): void {
  if (!Number.isInteger(value) || (value ?? 0) < 1 || (value ?? 0) > 65535) {
    throw new Error(`${field} must be between 1 and 65535`)
  }
}

function validateHealth(value: string | undefined, field: string): void {
  if (!value?.startsWith('/')) {
    throw new Error(`${field} must be an absolute path`)
  }
}

function validateOutput(value: string | undefined, field: string): void {
  if (typeof value !== 'string' || !isSafeRelativeDirectory(value)) {
    throw new Error(`${field} must be a safe relative directory like dist or .`)
  }
}

function validateResourceConfig(config: TovukConfig): void {
  const memoryMib = memoryToMib(config.resources.memory)
  if (memoryMib < 128 || memoryMib > 2048) {
    throw new Error('[resources].memory must be between 128mb and 2gb; use the smallest working value')
  }
  const cpuMillis = cpuToMillis(config.resources.cpu)
  if (cpuMillis < 50 || cpuMillis > 2000) {
    throw new Error('[resources].cpu must be between 0.05 and 2; use the smallest working value')
  }
  if (!Number.isInteger(config.resources.idle_timeout_minutes) || config.resources.idle_timeout_minutes < 1 || config.resources.idle_timeout_minutes > 60) {
    throw new Error('[resources].idle_timeout_minutes must be between 1 and 60')
  }
}

function validateCheckCommand(kind: ProjectKind, command: string): void {
  if (kind === 'static_frontend') {
    validateFrontendCheckCommand(command)
    return
  }

  validateRustCheckCommand(command)
}

function validateRustCheckCommand(command: string): void {
  const required = [
    'cargo fmt --all --check',
    'cargo check --locked --release --all-targets --all-features',
    'cargo test --locked --release --all-targets --all-features',
    'cargo clippy --locked --release --all-targets --all-features',
    '-D warnings',
    ...RUST_STRICT_CLIPPY_DENY_LINTS.map((lint) => `-D ${lint}`)
  ]
  if (required.every((fragment) => command.includes(fragment))) {
    return
  }
  throw new Error('[build].check must run rustfmt, locked release-mode cargo check, locked release-mode tests, and strict Clippy resource lints')
}

function validateRustBuildCommand(command: string): void {
  if (usesJavascriptBackendRuntime(command)) {
    throw new Error('Rust backend build commands cannot invoke JavaScript or TypeScript runtimes; use cargo build --release')
  }
  const tokens = commandTokens(command)
  if (tokens.some((token) => commandNameFromToken(token) === 'cargo') && tokens.includes('build') && tokens.includes('--release')) {
    return
  }
  throw new Error('Rust backend build commands must run cargo build --release')
}

function validateRustRunCommand(command: string | undefined): void {
  const value = command ?? ''
  if (usesJavascriptBackendRuntime(value)) {
    throw new Error('Rust backend runtime commands cannot invoke JavaScript or TypeScript runtimes; run ./target/release/<binary> instead')
  }
  if (commandTokens(value).some((token) => token.includes('target/release/'))) {
    return
  }
  throw new Error('Rust backend runtime commands must start a binary under ./target/release/')
}

function validateFrontendCheckCommand(command: string): void {
  if (isNoopCommand(command)) {
    return
  }
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

function usesJavascriptBackendRuntime(command: string): boolean {
  return commandTokens(command)
    .some((token) => JAVASCRIPT_BACKEND_RUNTIMES.has(commandNameFromToken(token)))
}

function isSafeRelativeDirectory(value: string): boolean {
  return value === '.' || isSafeRelativePath(value)
}

function isNoopCommand(command: string): boolean {
  return command.trim() === ':' || command.trim() === 'true'
}

function commandNameFromToken(token: string): string {
  return token.split('/').pop() ?? ''
}

function memoryToMib(value: string): number {
  const match = value.trim().toLowerCase().match(/^(\d+)\s*(mb|mib|gb|gib)$/u)
  if (!match) {
    throw new Error('[resources].memory must look like 256mb, 512mb, or 1gb')
  }
  const amount = Number.parseInt(match[1] ?? '', 10)
  return amount * ((match[2] ?? '').startsWith('g') ? 1024 : 1)
}

function cpuToMillis(value: string): number {
  if (!/^\d+(?:\.\d{1,3})?$/u.test(value.trim())) {
    throw new Error('[resources].cpu must look like 0.25, 0.5, 1, or 2')
  }
  return Math.round(Number.parseFloat(value) * 1000)
}

export { validateConfig }
