import { spawnSync } from 'node:child_process'
import { ARCHIVE_EXCLUDES, ARCHIVE_LIMIT_BYTES } from './constants.js'
import { agentError } from './errors.js'

function createArchiveBase64(projectDir) {
  const excludeArgs = ARCHIVE_EXCLUDES.map((pattern) => `--exclude=${pattern}`)
  const tar = spawnSync('tar', [...excludeArgs, '-czf', '-', '-C', projectDir, '.'], {
    encoding: 'buffer',
    env: { ...process.env, COPYFILE_DISABLE: '1' },
    maxBuffer: ARCHIVE_LIMIT_BYTES + 1024 * 1024
  })

  if (tar.error) {
    throw agentError('archive_failed', 'Could not create source archive.', 'Install `tar`, remove local build outputs, then retry `npx @zerct/zerct deploy`.', false)
  }
  if (tar.status !== 0) {
    throw agentError('archive_failed', 'Could not create source archive.', String(tar.stderr || 'Check project files and retry.'), false)
  }
  if (tar.stdout.length > ARCHIVE_LIMIT_BYTES) {
    throw agentError('archive_too_large', 'Source archive is too large.', 'Remove build outputs, target directories, logs, and local caches before deploying.', false)
  }

  return tar.stdout.toString('base64')
}

function gitCommitSha(projectDir) {
  const git = spawnSync('git', ['rev-parse', 'HEAD'], {
    cwd: projectDir,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'ignore']
  })
  return git.status === 0 ? git.stdout.trim() || null : null
}

export { createArchiveBase64, gitCommitSha }
