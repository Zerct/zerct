import { existsSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { doctorCheck } from './checks.ts'
import { DEFAULT_BUN_FRONTEND_CHECK_COMMAND, DEFAULT_NPM_FRONTEND_CHECK_COMMAND, FRONTEND_INSTALL_COMMANDS, FRONTEND_JAVASCRIPT_EXTENSIONS, FRONTEND_PACKAGE_MANAGERS, FRONTEND_SOURCE_ROOTS, JAVASCRIPT_LINTERS } from './constants.ts'
import { readPackageJson, walkProjectFiles } from './project.ts'
import type { DoctorCheck, FrontendSourceReport, PackageManifest } from './types.ts'

const REQUIRED_FRONTEND_SCRIPTS = ['typecheck', 'lint'] as const
const FRONTEND_PAGES_API_PREFIXES: readonly (readonly string[])[] = [['pages', 'api'], ['src', 'pages', 'api']]
const FRONTEND_APP_API_PREFIXES: readonly (readonly string[])[] = [['app', 'api'], ['src', 'app', 'api']]
type FrontendScriptName = typeof REQUIRED_FRONTEND_SCRIPTS[number]
type CommandPredicate = (command: string) => boolean

function frontendLockfileExists(projectDir: string): boolean {
  return ['package-lock.json', 'npm-shrinkwrap.json', 'pnpm-lock.yaml', 'yarn.lock', 'bun.lock', 'bun.lockb']
    .some((file) => existsSync(path.join(projectDir, file)))
}

function isPlainStaticFrontend(projectDir: string): boolean {
  return !existsSync(path.join(projectDir, 'package.json')) && existsSync(path.join(projectDir, 'index.html'))
}

function frontendPackageManager(projectDir: string): 'bun' | 'npm' {
  return existsSync(path.join(projectDir, 'bun.lock')) || existsSync(path.join(projectDir, 'bun.lockb'))
    ? 'bun'
    : 'npm'
}

function frontendCheckCommand(projectDir: string): string {
  if (isPlainStaticFrontend(projectDir)) {
    return ':'
  }
  return frontendPackageManager(projectDir) === 'bun'
    ? DEFAULT_BUN_FRONTEND_CHECK_COMMAND
    : DEFAULT_NPM_FRONTEND_CHECK_COMMAND
}

function frontendBuildCommand(projectDir: string): string {
  if (isPlainStaticFrontend(projectDir)) {
    return ':'
  }
  return frontendPackageManager(projectDir) === 'bun'
    ? 'bun run build'
    : 'npm run build'
}

function frontendScriptChecks(projectDir: string, runScripts: boolean): DoctorCheck[] {
  const manifest = readPackageJson(projectDir)
  const scripts: Record<FrontendScriptName, string> = {
    typecheck: packageScriptValue(manifest, 'typecheck'),
    lint: packageScriptValue(manifest, 'lint')
  }
  const checks: DoctorCheck[] = [
    ...REQUIRED_FRONTEND_SCRIPTS.map((script) => packageScriptExistsCheck(script, scripts[script])),
    strictTypecheckCheck(scripts.typecheck),
    nativeLintCheck(manifest),
    nativeQualityGateCheck(manifest)
  ]

  if (runScripts && checks.every((check) => check.ok)) {
    checks.push(...REQUIRED_FRONTEND_SCRIPTS.map((script) => packageScriptCheck(projectDir, script)))
  }

  return checks
}

function packageScriptExistsCheck(script: FrontendScriptName, command: string): DoctorCheck {
  const ok = command !== ''
  return doctorCheck(`package script ${script}`, ok, 'found', 'missing', `Add a non-empty "${script}" script to package.json, then retry.`)
}

function strictTypecheckCheck(command: string): DoctorCheck {
  const ok = usesStrictFrontendTypechecker(command)
  return doctorCheck('strict frontend typecheck', ok, 'accepted', 'native typecheck missing', 'Set package.json `typecheck` to `oxlint src vite.config.ts --deny-warnings --type-aware --type-check --tsconfig tsconfig.json`, then retry.')
}

function nativeLintCheck(manifest: PackageManifest | null): DoctorCheck {
  const ok = !packageScriptTreeUses(manifest, 'lint', usesJavascriptLinter)
    && packageScriptTreeUses(manifest, 'lint', usesNativeFrontendLinter)
  return doctorCheck('native frontend lint', ok, 'accepted', 'native linter missing', 'Replace the lint script with native tooling such as `oxlint src vite.config.ts --deny-warnings`, `biome check .`, or `deno lint`, then retry.')
}

function nativeQualityGateCheck(manifest: PackageManifest | null): DoctorCheck {
  const ok = packageScriptTreeUses(manifest, 'lint', usesNativeDeadCodeChecker)
    && packageScriptTreeUses(manifest, 'lint', usesNativeDuplicateChecker)
    && packageScriptTreeUses(manifest, 'lint', usesNativeHealthChecker)
  return doctorCheck('native frontend quality gates', ok, 'accepted', 'dead-code, duplicate-code, or health gate missing', 'Add Fallow checks for `dead-code`, semantic `dupes`, and `health` to package.json `lint`, then retry.')
}

function frontendSourceChecks(projectDir: string): DoctorCheck[] {
  const report = frontendSourceReport(projectDir)
  return [
    {
      name: 'typescript source',
      ok: report.typescript.length > 0,
      message: report.typescript.length > 0 ? report.typescript.slice(0, 3).join(', ') : 'missing',
      agent_instruction: report.typescript.length > 0 ? null : 'Add browser source as .ts or .tsx under src, app, pages, routes, or components, then retry.'
    },
    forbiddenSourceCheck('javascript source', report.javascript, 'Rename browser .js, .jsx, .mjs, or .cjs source files to .ts or .tsx and fix type errors before deploying.'),
    forbiddenSourceCheck('frontend server routes', report.serverRoutes, 'Move API routes, SSR handlers, middleware, and server logic to the Rust backend; static frontend source may only contain browser code.')
  ]
}

function forbiddenSourceCheck(name: string, files: readonly string[], instruction: string): DoctorCheck {
  return {
    name,
    ok: files.length === 0,
    message: files.length === 0 ? 'none found' : files.slice(0, 5).join(', '),
    agent_instruction: files.length === 0 ? null : instruction
  }
}

function frontendSourceReport(projectDir: string): FrontendSourceReport {
  const report: FrontendSourceReport = { typescript: [], javascript: [], serverRoutes: [] }
  walkProjectFiles(projectDir, (_file, relative) => {
    if (isFrontendServerRoute(relative)) {
      report.serverRoutes.push(relative)
    }
    const sourceKind = frontendSourceKind(relative)
    if (sourceKind) {
      report[sourceKind].push(relative)
    }
  })
  return report
}

function frontendSourceKind(relative: string): keyof FrontendSourceReport | null {
  if (!isFrontendSourcePath(relative)) {
    return null
  }
  if (isFrontendTypescriptSource(relative)) {
    return 'typescript'
  }
  return isFrontendJavascriptSource(relative) ? 'javascript' : null
}

function isFrontendSourcePath(relative: string): boolean {
  const [root = ''] = relative.split('/')
  return FRONTEND_SOURCE_ROOTS.has(root)
}

function isFrontendTypescriptSource(relative: string): boolean {
  return !relative.endsWith('.d.ts') && (relative.endsWith('.ts') || relative.endsWith('.tsx'))
}

function isFrontendJavascriptSource(relative: string): boolean {
  return FRONTEND_JAVASCRIPT_EXTENSIONS.some((extension) => relative.endsWith(extension))
}

function isFrontendServerRoute(relative: string): boolean {
  if (!isFrontendTypescriptSource(relative) && !isFrontendJavascriptSource(relative)) {
    return false
  }
  const parts = relative.toLowerCase().split('/')
  const file = parts.at(-1) ?? ''
  return isFrontendServerHandlerFile(file) || isFrontendApiRoute(parts, file)
}

function isFrontendServerHandlerFile(file: string): boolean {
  return file.startsWith('+server.') || file.startsWith('middleware.')
}

function isFrontendApiRoute(pathParts: readonly string[], file: string): boolean {
  return FRONTEND_PAGES_API_PREFIXES.some((prefix) => pathStartsWith(pathParts, prefix))
    || (file.startsWith('route.') && FRONTEND_APP_API_PREFIXES.some((prefix) => pathStartsWith(pathParts, prefix)))
}

function pathStartsWith(pathParts: readonly string[], prefix: readonly string[]): boolean {
  return pathParts.length >= prefix.length && prefix.every((part, index) => pathParts[index] === part)
}

function packageScriptValue(manifest: PackageManifest | null, script: string): string {
  const value = manifest?.scripts?.[script]
  return typeof value === 'string' ? value.trim() : ''
}

function usesJavascriptLinter(command: string): boolean {
  const tokens = commandTokens(command)
  return tokens.some((token, index) => {
    const commandName = commandNameFromToken(token)
    return JAVASCRIPT_LINTERS.has(commandName)
      || (commandName === 'next' && tokens[index + 1] === 'lint')
  })
}

function usesStrictFrontendTypechecker(command: string): boolean {
  const tokens = commandTokens(command)
  return tokens.some((token, index) => {
    const commandName = commandNameFromToken(token)
    return (commandName === 'oxlint' && tokens.includes('--type-aware') && tokens.includes('--type-check'))
      || (commandName === 'deno' && tokens[index + 1] === 'check')
  })
}

function usesNativeFrontendLinter(command: string): boolean {
  const tokens = commandTokens(command)
  return tokens.some((token, index) => {
    const commandName = commandNameFromToken(token)
    return commandName === 'oxlint'
      || (commandName === 'biome' && ['check', 'lint'].includes(tokens[index + 1] ?? ''))
      || (commandName === 'deno' && tokens[index + 1] === 'lint')
  })
}

function usesNativeDeadCodeChecker(command: string): boolean {
  return usesFallowSubcommand(command, 'dead-code')
}

function usesNativeDuplicateChecker(command: string): boolean {
  return usesFallowSubcommand(command, 'dupes')
}

function usesNativeHealthChecker(command: string): boolean {
  return usesFallowSubcommand(command, 'health')
}

function usesFallowSubcommand(command: string, subcommand: string): boolean {
  return commandTokens(command).some((token, index, tokens) => (
    commandNameFromToken(token) === 'fallow' && tokens[index + 1] === subcommand
  ))
}

function packageScriptTreeUses(manifest: PackageManifest | null, script: string, predicate: CommandPredicate, seen = new Set<string>()): boolean {
  if (seen.has(script)) {
    return false
  }
  seen.add(script)

  const command = packageScriptValue(manifest, script)
  if (command === '') {
    return false
  }
  if (predicate(command)) {
    return true
  }

  return referencedPackageScripts(command).some((referencedScript) => packageScriptTreeUses(manifest, referencedScript, predicate, seen))
}

function referencedPackageScripts(command: string): string[] {
  const tokens = commandTokens(command)
  const scripts: string[] = []
  for (const [index, token] of tokens.entries()) {
    if (!FRONTEND_PACKAGE_MANAGERS.has(commandNameFromToken(token)) || tokens[index + 1] !== 'run') {
      continue
    }
    const script = scriptNameAfterRun(tokens, index + 2)
    if (script !== null) {
      scripts.push(script)
    }
  }
  return scripts
}

function scriptNameAfterRun(tokens: readonly string[], start: number): string | null {
  let index = start
  while (tokens[index]?.startsWith('-')) {
    index += 1
  }
  return tokens[index] ?? null
}

function commandTokens(command: string): string[] {
  return command
    .replace(/[&|;()]/gu, ' ')
    .split(/\s+/u)
    .map((token) => token.trim().replace(/^["']|["']$/gu, ''))
    .filter(Boolean)
}

function commandNameFromToken(token: string): string {
  return token.split('/').pop() ?? ''
}

function hasFrontendInstallCommand(tokens: string[]): boolean {
  return tokens.some((token, index) => FRONTEND_INSTALL_COMMANDS.has(`${commandNameFromToken(token)} ${tokens[index + 1] || ''}`))
}

function hasFrontendScriptRun(tokens: string[], script: string): boolean {
  return tokens.some((token, index) => {
    if (!FRONTEND_PACKAGE_MANAGERS.has(commandNameFromToken(token)) || tokens[index + 1] !== 'run') {
      return false
    }
    return tokens[index + 2] === script || ((tokens[index + 2] ?? '').startsWith('-') && tokens[index + 3] === script)
  })
}

function packageScriptCheck(projectDir: string, script: string): DoctorCheck {
  const manager = frontendPackageManager(projectDir)
  const args = manager === 'bun' ? ['run', script] : ['run', '--silent', script]
  const result = spawnSync(manager, args, {
    cwd: projectDir,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe']
  })
  if (result.error) {
    return {
      name: `${manager} run ${script}`,
      ok: false,
      message: result.error.message,
      agent_instruction: `Install ${manager === 'bun' ? 'Bun' : 'Node.js and npm'}, then run \`${manager} run ${script}\` before deploying.`
    }
  }

  return {
    name: `${manager} run ${script}`,
    ok: result.status === 0,
    message: result.status === 0 ? 'passed' : (result.stderr || result.stdout || `${manager} run ${script} failed`).trim().slice(0, 240),
    agent_instruction: result.status === 0 ? null : `Run \`${manager} run ${script}\`, fix every error, then redeploy.`
  }
}

export { frontendLockfileExists, isPlainStaticFrontend, frontendCheckCommand, frontendBuildCommand, frontendScriptChecks, frontendSourceChecks, usesJavascriptLinter, commandTokens, hasFrontendInstallCommand, hasFrontendScriptRun }
