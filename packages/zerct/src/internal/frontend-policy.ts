import { existsSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { DEFAULT_BUN_FRONTEND_CHECK_COMMAND, DEFAULT_NPM_FRONTEND_CHECK_COMMAND, FRONTEND_INSTALL_COMMANDS, FRONTEND_JAVASCRIPT_EXTENSIONS, FRONTEND_PACKAGE_MANAGERS, FRONTEND_SOURCE_ROOTS, JAVASCRIPT_LINTERS } from './constants.ts'
import { readPackageJson, walkProjectFiles } from './project.ts'
import type { DoctorCheck, FrontendSourceReport, PackageManifest } from './types.ts'

const REQUIRED_FRONTEND_SCRIPTS = ['typecheck', 'lint'] as const
type FrontendScriptName = typeof REQUIRED_FRONTEND_SCRIPTS[number]

function frontendLockfileExists(projectDir: string): boolean {
  return ['package-lock.json', 'npm-shrinkwrap.json', 'pnpm-lock.yaml', 'yarn.lock', 'bun.lock', 'bun.lockb']
    .some((file) => existsSync(path.join(projectDir, file)))
}

function frontendPackageManager(projectDir: string): 'bun' | 'npm' {
  return existsSync(path.join(projectDir, 'bun.lock')) || existsSync(path.join(projectDir, 'bun.lockb'))
    ? 'bun'
    : 'npm'
}

function frontendCheckCommand(projectDir: string): string {
  return frontendPackageManager(projectDir) === 'bun'
    ? DEFAULT_BUN_FRONTEND_CHECK_COMMAND
    : DEFAULT_NPM_FRONTEND_CHECK_COMMAND
}

function frontendBuildCommand(projectDir: string): string {
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
  const checks: DoctorCheck[] = REQUIRED_FRONTEND_SCRIPTS.map((script) => {
    const exists = scripts[script] !== ''
    return {
      name: `package script ${script}`,
      ok: exists,
      message: exists ? 'found' : 'missing',
      agent_instruction: exists ? null : `Add a non-empty "${script}" script to package.json, then retry.`
    }
  })
  const strictTypecheck = usesStrictFrontendTypechecker(scripts.typecheck)
  checks.push({
    name: 'strict frontend typecheck',
    ok: strictTypecheck,
    message: strictTypecheck ? 'accepted' : 'tsgo --noEmit missing',
    agent_instruction: strictTypecheck ? null : 'Set package.json `typecheck` to `tsgo --noEmit`, install `@typescript/native-preview`, then retry.'
  })

  const nativeLint = !usesJavascriptLinter(scripts.lint) && usesNativeFrontendLinter(scripts.lint)
  checks.push({
    name: 'native frontend lint',
    ok: nativeLint,
    message: nativeLint ? 'accepted' : 'native linter missing',
    agent_instruction: nativeLint ? null : 'Replace the lint script with native tooling such as `oxlint src vite.config.ts --deny-warnings`, `biome check .`, or `deno lint`, then retry.'
  })

  if (runScripts && checks.every((check) => check.ok)) {
    checks.push(...REQUIRED_FRONTEND_SCRIPTS.map((script) => packageScriptCheck(projectDir, script)))
  }

  return checks
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
    {
      name: 'javascript source',
      ok: report.javascript.length === 0,
      message: report.javascript.length === 0 ? 'none found' : report.javascript.slice(0, 5).join(', '),
      agent_instruction: report.javascript.length === 0 ? null : 'Rename browser .js, .jsx, .mjs, or .cjs source files to .ts or .tsx and fix type errors before deploying.'
    }
  ]
}

function frontendSourceReport(projectDir: string): FrontendSourceReport {
  const report: FrontendSourceReport = { typescript: [], javascript: [] }
  walkProjectFiles(projectDir, (_file, relative) => {
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
    return (commandName === 'tsgo' && tokens.includes('--noEmit'))
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

export { frontendLockfileExists, frontendCheckCommand, frontendBuildCommand, frontendScriptChecks, frontendSourceChecks, usesJavascriptLinter, commandTokens, hasFrontendInstallCommand, hasFrontendScriptRun }
