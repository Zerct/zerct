import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { WALK_EXCLUDED_DIRS } from './constants.ts'
import { agentError } from './errors.ts'
import type { CliOptions, FileVisitor, JsonValue, PackageManifest, PathVisitor, ProjectKind } from './types.ts'

function hasCommand(command: string): boolean {
  return (process.env['PATH'] ?? '')
    .split(path.delimiter)
    .filter(Boolean)
    .some((directory) => existsSync(path.join(directory, command)))
}

function isSafeRelativePath(value: string | undefined): value is string {
  return typeof value === 'string'
    && value.length > 0
    && !path.isAbsolute(value)
    && !value.includes('\\')
    && value.split('/').every((part) => part && part !== '.' && part !== '..')
}

function walkProjectFiles(projectDir: string, visit: FileVisitor): void {
  walk(projectDir, (file) => {
    visit(file, path.relative(projectDir, file).replace(/\\/gu, '/'))
  })
}

function walk(dir: string, visit: PathVisitor): void {
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

function ensureDirectory(dir: string): void {
  if (!existsSync(dir) || !statSync(dir).isDirectory()) {
    throw agentError('missing_project', 'Project directory does not exist.', 'Run Zerct from the root of a Rust project or pass the project path.', false)
  }
}

function serviceNameFromDir(projectDir: string): string {
  const name = serviceNameFromValue(path.basename(projectDir))
  return name || 'api'
}

function serviceNameFromCargo(projectDir: string): string {
  try {
    const source = readFileSync(path.join(projectDir, 'Cargo.toml'), 'utf8')
    return serviceNameFromValue(source.match(/^\s*name\s*=\s*"([^"]+)"/mu)?.[1] || '')
  } catch {
    return ''
  }
}

function serviceNameFromPackage(projectDir: string): string {
  const manifest = readPackageJson(projectDir)
  return serviceNameFromValue(typeof manifest?.name === 'string' ? manifest.name : '')
}

function serviceNameFromValue(value: string): string {
  return value.toLowerCase().replace(/[^a-z0-9-]+/gu, '-').replace(/^-+|-+$/gu, '').slice(0, 48)
}

function inferProjectKind(projectDir: string): ProjectKind {
  if (existsSync(path.join(projectDir, 'Cargo.toml'))) {
    return 'rust_backend'
  }
  if (existsSync(path.join(projectDir, 'package.json'))) {
    return 'static_frontend'
  }
  return 'rust_backend'
}

function readPackageJson(projectDir: string): PackageManifest | null {
  try {
    const parsed: unknown = JSON.parse(readFileSync(path.join(projectDir, 'package.json'), 'utf8'))
    return isPackageManifest(parsed) ? parsed : null
  } catch {
    return null
  }
}

function isPackageManifest(value: unknown): value is PackageManifest {
  if (!isRecord(value)) {
    return false
  }
  const name = value['name']
  const scripts = value['scripts']
  return (name === undefined || typeof name === 'string') &&
    (scripts === undefined || isStringRecord(scripts))
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function isStringRecord(value: unknown): value is Record<string, string | undefined> {
  return isRecord(value) && Object.values(value).every((entry) => entry === undefined || typeof entry === 'string')
}

function printJson(value: JsonValue | null): void {
  console.log(JSON.stringify(value, null, 2))
}

function openUrl(url: string): void {
  const command = process.platform === 'darwin' ? 'open' : process.platform === 'win32' ? 'cmd' : 'xdg-open'
  const args = process.platform === 'win32' ? ['/c', 'start', '', url] : [url]
  spawnSync(command, args, { stdio: 'ignore' })
}

function sleep(milliseconds: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, milliseconds)
  })
}

function progress(cli: CliOptions, message: string): void {
  if (cli.json) {
    console.error(message)
    return
  }
  console.log(message)
}

export { hasCommand, isSafeRelativePath, walkProjectFiles, ensureDirectory, serviceNameFromDir, serviceNameFromCargo, serviceNameFromPackage, inferProjectKind, readPackageJson, printJson, openUrl, sleep, progress }
