import { existsSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { DEFAULT_BUN_FRONTEND_CHECK_COMMAND, DEFAULT_NPM_FRONTEND_CHECK_COMMAND, FRONTEND_INSTALL_COMMANDS, FRONTEND_JAVASCRIPT_EXTENSIONS, FRONTEND_PACKAGE_MANAGERS, FRONTEND_SOURCE_ROOTS, JAVASCRIPT_LINTERS } from './constants.js'
import { readPackageJson, walkProjectFiles } from './project.js'

function frontendLockfileExists(projectDir) {
  return ['package-lock.json', 'npm-shrinkwrap.json', 'pnpm-lock.yaml', 'yarn.lock', 'bun.lock', 'bun.lockb']
    .some((file) => existsSync(path.join(projectDir, file)))
}

function frontendPackageManager(projectDir) {
  return existsSync(path.join(projectDir, 'bun.lock')) || existsSync(path.join(projectDir, 'bun.lockb'))
    ? 'bun'
    : 'npm'
}

function frontendCheckCommand(projectDir) {
  return frontendPackageManager(projectDir) === 'bun'
    ? DEFAULT_BUN_FRONTEND_CHECK_COMMAND
    : DEFAULT_NPM_FRONTEND_CHECK_COMMAND
}

function frontendBuildCommand(projectDir) {
  return frontendPackageManager(projectDir) === 'bun'
    ? 'bun run build'
    : 'npm run build'
}

function frontendScriptChecks(projectDir, runScripts) {
  const manifest = readPackageJson(projectDir)
  const missing = (script) => packageScriptValue(manifest, script) === ''
  const checks = ['typecheck', 'lint'].map((script) => ({
    name: `package script ${script}`,
    ok: !missing(script),
    message: missing(script) ? 'missing' : 'found',
    agent_instruction: `Add a non-empty "${script}" script to package.json, then retry.`
  }))
  const typecheckScript = packageScriptValue(manifest, 'typecheck')
  const strictTypecheck = usesStrictFrontendTypechecker(typecheckScript)
  checks.push({
    name: 'strict frontend typecheck',
    ok: strictTypecheck,
    message: strictTypecheck ? 'accepted' : 'tsgo --noEmit missing',
    agent_instruction: 'Set package.json `typecheck` to `tsgo --noEmit`, install `@typescript/native-preview`, then retry.'
  })

  const lintScript = packageScriptValue(manifest, 'lint')
  const nativeLint = !usesJavascriptLinter(lintScript) && usesNativeFrontendLinter(lintScript)
  checks.push({
    name: 'native frontend lint',
    ok: nativeLint,
    message: nativeLint ? 'accepted' : 'native linter missing',
    agent_instruction: 'Replace the lint script with native tooling such as `oxlint src vite.config.ts --deny-warnings`, `biome check .`, or `deno lint`, then retry.'
  })

  if (runScripts && checks.every((check) => check.ok)) {
    checks.push(packageScriptCheck(projectDir, 'typecheck'))
    checks.push(packageScriptCheck(projectDir, 'lint'))
  }

  return checks
}

function frontendSourceChecks(projectDir) {
  const report = frontendSourceReport(projectDir)
  return [
    {
      name: 'typescript source',
      ok: report.typescript.length > 0,
      message: report.typescript.length > 0 ? report.typescript.slice(0, 3).join(', ') : 'missing',
      agent_instruction: 'Add browser source as .ts or .tsx under src, app, pages, routes, or components, then retry.'
    },
    {
      name: 'javascript source',
      ok: report.javascript.length === 0,
      message: report.javascript.length === 0 ? 'none found' : report.javascript.slice(0, 5).join(', '),
      agent_instruction: 'Rename browser .js, .jsx, .mjs, or .cjs source files to .ts or .tsx and fix type errors before deploying.'
    }
  ]
}

function frontendSourceReport(projectDir) {
  const report = { typescript: [], javascript: [] }
  walkProjectFiles(projectDir, (file, relative) => {
    if (!isFrontendSourcePath(relative)) {
      return
    }
    if (isFrontendTypescriptSource(relative)) {
      report.typescript.push(relative)
    } else if (isFrontendJavascriptSource(relative)) {
      report.javascript.push(relative)
    }
  })
  return report
}

function isFrontendSourcePath(relative) {
  const [root] = relative.split('/')
  return FRONTEND_SOURCE_ROOTS.has(root)
}

function isFrontendTypescriptSource(relative) {
  return !relative.endsWith('.d.ts') && (relative.endsWith('.ts') || relative.endsWith('.tsx'))
}

function isFrontendJavascriptSource(relative) {
  return FRONTEND_JAVASCRIPT_EXTENSIONS.some((extension) => relative.endsWith(extension))
}

function packageScriptValue(manifest, script) {
  const value = manifest?.scripts?.[script]
  return typeof value === 'string' ? value.trim() : ''
}

function usesJavascriptLinter(command) {
  const tokens = commandTokens(command)
  return tokens.some((token, index) => {
    const commandName = commandNameFromToken(token)
    return JAVASCRIPT_LINTERS.has(commandName)
      || (commandName === 'next' && tokens[index + 1] === 'lint')
  })
}

function usesStrictFrontendTypechecker(command) {
  const tokens = commandTokens(command)
  return tokens.some((token, index) => {
    const commandName = commandNameFromToken(token)
    return (commandName === 'tsgo' && tokens.includes('--noEmit'))
      || (commandName === 'deno' && tokens[index + 1] === 'check')
  })
}

function usesNativeFrontendLinter(command) {
  const tokens = commandTokens(command)
  return tokens.some((token, index) => {
    const commandName = commandNameFromToken(token)
    return commandName === 'oxlint'
      || (commandName === 'biome' && ['check', 'lint'].includes(tokens[index + 1]))
      || (commandName === 'deno' && tokens[index + 1] === 'lint')
  })
}

function commandTokens(command) {
  return command
    .replace(/[&|;()]/gu, ' ')
    .split(/\s+/u)
    .map((token) => token.trim().replace(/^["']|["']$/gu, ''))
    .filter(Boolean)
}

function commandNameFromToken(token) {
  return token.split('/').pop()
}

function hasFrontendInstallCommand(tokens) {
  return tokens.some((token, index) => FRONTEND_INSTALL_COMMANDS.has(`${commandNameFromToken(token)} ${tokens[index + 1] || ''}`))
}

function hasFrontendScriptRun(tokens, script) {
  return tokens.some((token, index) => {
    if (!FRONTEND_PACKAGE_MANAGERS.has(commandNameFromToken(token)) || tokens[index + 1] !== 'run') {
      return false
    }
    return tokens[index + 2] === script || (tokens[index + 2] || '').startsWith('-') && tokens[index + 3] === script
  })
}

function packageScriptCheck(projectDir, script) {
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
    agent_instruction: `Run \`${manager} run ${script}\`, fix every error, then redeploy.`
  }
}

export { frontendLockfileExists, frontendPackageManager, frontendCheckCommand, frontendBuildCommand, frontendScriptChecks, frontendSourceChecks, frontendSourceReport, isFrontendSourcePath, isFrontendTypescriptSource, isFrontendJavascriptSource, packageScriptValue, usesJavascriptLinter, usesStrictFrontendTypechecker, usesNativeFrontendLinter, commandTokens, commandNameFromToken, hasFrontendInstallCommand, hasFrontendScriptRun, packageScriptCheck }
