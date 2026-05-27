import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { FRONTEND_SOURCE_ROOTS, FRONTEND_JAVASCRIPT_EXTENSIONS, WALK_EXCLUDED_DIRS } from './constants.js'
import { agentError } from './errors.js'

function hasCommand(command) {
  return (process.env.PATH || '')
    .split(path.delimiter)
    .filter(Boolean)
    .some((directory) => existsSync(path.join(directory, command)))
}

function isSafeRelativePath(value) {
  return value
    && !path.isAbsolute(value)
    && !value.includes('\\')
    && value.split('/').every((part) => part && part !== '.' && part !== '..')
}

function scanUnsafe(projectDir) {
  const hits = []
  walkProjectFiles(projectDir, (file, relative) => {
    if (!file.endsWith('.rs')) {
      return
    }
    const source = readFileSync(file, 'utf8')
    if (/\bunsafe\b/u.test(source)) {
      hits.push(relative)
    }
  })
  return hits
}

function walkProjectFiles(projectDir, visit) {
  walk(projectDir, (file) => {
    visit(file, path.relative(projectDir, file).replace(/\\/gu, '/'))
  })
}

function walk(dir, visit) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (WALK_EXCLUDED_DIRS.has(entry.name)) {
      continue
    }
    const fullPath = path.join(dir, entry.name)
    if (entry.isDirectory()) {
      walk(fullPath, visit)
    } else if (entry.isFile()) {
      visit(fullPath)
    }
  }
}

function ensureDirectory(dir) {
  if (!existsSync(dir) || !statSync(dir).isDirectory()) {
    throw agentError('missing_project', 'Project directory does not exist.', 'Run Zerct from the root of a Rust project or pass the project path.', false)
  }
}

function serviceNameFromDir(projectDir) {
  const name = serviceNameFromValue(path.basename(projectDir))
  return name || 'api'
}

function serviceNameFromCargo(projectDir) {
  try {
    const source = readFileSync(path.join(projectDir, 'Cargo.toml'), 'utf8')
    return serviceNameFromValue(source.match(/^\s*name\s*=\s*"([^"]+)"/mu)?.[1] || '')
  } catch (_error) {
    return ''
  }
}

function serviceNameFromPackage(projectDir) {
  const manifest = readPackageJson(projectDir)
  return serviceNameFromValue(typeof manifest?.name === 'string' ? manifest.name : '')
}

function serviceNameFromValue(value) {
  return value.toLowerCase().replace(/[^a-z0-9-]+/gu, '-').replace(/^-+|-+$/gu, '').slice(0, 48)
}

function inferProjectKind(projectDir) {
  if (existsSync(path.join(projectDir, 'Cargo.toml'))) {
    return 'rust_backend'
  }
  if (existsSync(path.join(projectDir, 'package.json'))) {
    return 'static_frontend'
  }
  return 'rust_backend'
}

function readPackageJson(projectDir) {
  try {
    return JSON.parse(readFileSync(path.join(projectDir, 'package.json'), 'utf8'))
  } catch (_error) {
    return null
  }
}

function printJsonOrPretty(cli, value) {
  console.log(JSON.stringify(value, null, cli.json ? 2 : 2))
}

function openUrl(url) {
  const command = process.platform === 'darwin' ? 'open' : process.platform === 'win32' ? 'cmd' : 'xdg-open'
  const args = process.platform === 'win32' ? ['/c', 'start', '', url] : [url]
  spawnSync(command, args, { stdio: 'ignore', detached: true })
}

function sleep(milliseconds) {
  return new Promise((resolve) => {
    setTimeout(resolve, milliseconds)
  })
}

function progress(cli, message) {
  if (cli.json) {
    console.error(message)
    return
  }
  console.log(message)
}

export { hasCommand, isSafeRelativePath, scanUnsafe, walkProjectFiles, walk, ensureDirectory, serviceNameFromDir, serviceNameFromCargo, serviceNameFromPackage, serviceNameFromValue, inferProjectKind, readPackageJson, printJsonOrPretty, openUrl, sleep, progress }
