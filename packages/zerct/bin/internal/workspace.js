import { existsSync, readFileSync, readdirSync } from 'node:fs'
import path from 'node:path'
import { WORKSPACE_EXCLUDED_DIRS } from './constants.js'
import { parseZerctToml } from './config.js'
import { ensureDirectory } from './project.js'

function discoverDeployProjects(rootDir) {
  ensureDirectory(rootDir)
  if (existsSync(path.join(rootDir, 'zerct.toml'))) {
    return [deployProjectInfo(rootDir, rootDir)]
  }

  const projectDirs = []
  discoverProjectDirs(rootDir, projectDirs)
  return projectDirs
    .map((dir) => deployProjectInfo(dir, rootDir))
    .sort(compareDeployProjects)
}

function discoverProjectDirs(dir, projectDirs) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (!entry.isDirectory() || WORKSPACE_EXCLUDED_DIRS.has(entry.name)) {
      continue
    }

    const child = path.join(dir, entry.name)
    if (existsSync(path.join(child, 'zerct.toml'))) {
      projectDirs.push(child)
      continue
    }
    discoverProjectDirs(child, projectDirs)
  }
}

function deployProjectInfo(dir, rootDir) {
  const relative = path.relative(rootDir, dir).replace(/\\/gu, '/') || '.'
  try {
    const config = parseZerctToml(readFileSync(path.join(dir, 'zerct.toml'), 'utf8'), dir)
    return { dir, relative, name: config.name || '', kind: config.kind }
  } catch (_error) {
    return { dir, relative, name: '', kind: 'unknown' }
  }
}

function compareDeployProjects(left, right) {
  return kindOrder(left.kind) - kindOrder(right.kind)
    || left.relative.localeCompare(right.relative)
}

function kindOrder(kind) {
  if (kind === 'rust_backend') {
    return 0
  }
  if (kind === 'static_frontend') {
    return 1
  }
  return 2
}

export { discoverDeployProjects, discoverProjectDirs, deployProjectInfo, compareDeployProjects, kindOrder }
