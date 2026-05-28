import { existsSync, readFileSync, readdirSync } from 'node:fs'
import path from 'node:path'
import { WORKSPACE_EXCLUDED_DIRS } from './constants.ts'
import { parseTovukToml } from './config.ts'
import { ensureDirectory } from './project.ts'
import type { DeployProjectInfo, DiscoveredProjectKind } from './types.ts'

function discoverDeployProjects(rootDir: string): DeployProjectInfo[] {
  ensureDirectory(rootDir)
  if (existsSync(path.join(rootDir, 'tovuk.toml'))) {
    return [deployProjectInfo(rootDir, rootDir)]
  }

  const projectDirs: string[] = []
  discoverProjectDirs(rootDir, projectDirs)
  return projectDirs
    .map((dir) => deployProjectInfo(dir, rootDir))
    .toSorted(compareDeployProjects)
}

function discoverProjectDirs(dir: string, projectDirs: string[]): void {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (!entry.isDirectory() || WORKSPACE_EXCLUDED_DIRS.has(entry.name)) {
      continue
    }

    const child = path.join(dir, entry.name)
    if (existsSync(path.join(child, 'tovuk.toml'))) {
      projectDirs.push(child)
      continue
    }
    discoverProjectDirs(child, projectDirs)
  }
}

function deployProjectInfo(dir: string, rootDir: string): DeployProjectInfo {
  const relative = path.relative(rootDir, dir).replace(/\\/gu, '/') || '.'
  try {
    const config = parseTovukToml(readFileSync(path.join(dir, 'tovuk.toml'), 'utf8'), dir)
    return { dir, relative, name: config.name || '', kind: config.kind }
  } catch {
    return { dir, relative, name: '', kind: 'unknown' }
  }
}

function compareDeployProjects(left: DeployProjectInfo, right: DeployProjectInfo): number {
  return kindOrder(left.kind) - kindOrder(right.kind)
    || left.relative.localeCompare(right.relative)
}

function kindOrder(kind: DiscoveredProjectKind): number {
  if (kind === 'rust_backend') {
    return 0
  }
  if (kind === 'static_frontend') {
    return 1
  }
  return 2
}

export { discoverDeployProjects }
