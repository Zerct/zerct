import { PROJECT_KINDS } from './constants.ts'
import { commandTokens, hasFrontendInstallCommand, hasFrontendScriptRun, usesJavascriptLinter } from './frontend-policy.ts'
import { isSafeRelativePath } from './project.ts'
import type { ProjectKind, ZerctConfig } from './types.ts'

function validateConfig(config: ZerctConfig): void {
  validateIdentity(config)
  validateBuildConfig(config)
  if (config.kind === 'static_frontend') {
    validateStaticFrontendConfig(config)
    return
  }
  validateRustBackendConfig(config)
}

function validateIdentity(config: ZerctConfig): void {
  if (!/^[a-z0-9](?:[a-z0-9-]{0,46}[a-z0-9])?$/u.test(config.name ?? '')) {
    throw new Error('name must be lowercase DNS-safe text up to 48 characters')
  }
  if (!PROJECT_KINDS.has(config.kind)) {
    throw new Error('kind must be rust_backend or static_frontend')
  }
}

function validateBuildConfig(config: ZerctConfig): void {
  if (!config.build.command.trim()) {
    throw new Error('[build].command is required')
  }
  if (!config.build.check.trim()) {
    throw new Error('[build].check is required')
  }
  validateCheckCommand(config.kind, config.build.check)
}

function validateStaticFrontendConfig(config: ZerctConfig): void {
  if (typeof config.build.output !== 'string' || !isSafeRelativePath(config.build.output)) {
    throw new Error('[build].output must be a safe relative directory like dist')
  }
}

function validateRustBackendConfig(config: ZerctConfig): void {
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
  validateResourceConfig(config)
}

function validateResourceConfig(config: ZerctConfig): void {
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

export { validateConfig }
