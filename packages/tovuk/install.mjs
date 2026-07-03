#!/usr/bin/env node
import { createHash } from 'node:crypto'
import { createWriteStream, chmodSync, copyFileSync, mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, statSync } from 'node:fs'
import { get } from 'node:https'
import { arch, platform, tmpdir } from 'node:os'
import { basename, dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const packageRoot = dirname(fileURLToPath(import.meta.url))
const manifest = JSON.parse(readFileSync(join(packageRoot, 'package.json'), 'utf8'))
const nativeTargets = JSON.parse(readFileSync(join(packageRoot, 'native-release-targets.json'), 'utf8')).targets
const binaryPath = join(packageRoot, 'bin', nativeBinaryName())
const REQUEST_TIMEOUT_MS = 30_000
const MAX_REDIRECTS = 5

mkdirSync(dirname(binaryPath), { recursive: true })

if (process.env.TOVUK_NATIVE_BINARY) {
  installFromLocal(process.env.TOVUK_NATIVE_BINARY)
} else {
  await installFromRelease()
}

function installFromLocal(source) {
  copyFileSync(source, binaryPath)
  chmodSync(binaryPath, 0o755)
}

async function installFromRelease() {
  const target = nativeTarget()
  const asset = `tovuk-${manifest.version}-${target.triple}${target.asset_ext}`
  const url = `https://github.com/tovuk/tovuk/releases/download/v${manifest.version}/${asset}`
  const checksumUrl = `${url}.sha256`
  const tempDir = mkdtempSync(join(tmpdir(), 'tovuk-install-'))
  const tempPath = join(tempDir, basename(asset))
  try {
    await download(url, tempPath)
    assertRegularFile(tempPath)
    const expectedSha256 = parseChecksum(await fetchText(checksumUrl), asset)
    verifySha256(tempPath, expectedSha256)
    renameSync(tempPath, binaryPath)
    chmodSync(binaryPath, 0o755)
  } catch (error) {
    throw new Error(`Could not install native Tovuk binary from ${url}: ${error instanceof Error ? error.message : String(error)}`)
  } finally {
    rmSync(tempDir, { recursive: true, force: true })
  }
}

function nativeTarget() {
  const os = platform()
  const cpu = arch()
  const target = nativeTargets.find((item) => item.node.platform === os && item.node.arch === cpu)
  if (target?.libc === 'glibc' && linuxLibc() !== 'glibc') {
    throw new Error(`Unsupported Tovuk native target: ${os}/${cpu} requires glibc Linux. Alpine/musl Linux is not supported by the published native binaries yet.`)
  }
  if (target) {
    return target
  }
  throw new Error(`Unsupported Tovuk native target: ${os}/${cpu}`)
}

function nativeBinaryName() {
  return platform() === 'win32' ? 'tovuk-native.exe' : 'tovuk-native'
}

function linuxLibc() {
  if (platform() !== 'linux') {
    return ''
  }
  const report = process.report?.getReport?.()
  const glibcVersion = report?.header?.glibcVersionRuntime
  return typeof glibcVersion === 'string' && glibcVersion.length > 0 ? 'glibc' : 'musl'
}

function download(url, destination, redirects = 0) {
  return new Promise((resolve, reject) => {
    const request = get(url, (response) => {
      if (response.statusCode && response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) {
        if (redirects >= MAX_REDIRECTS) {
          response.resume()
          reject(new Error(`too many redirects while downloading ${url}`))
          return
        }
        response.resume()
        download(new URL(response.headers.location, url).toString(), destination, redirects + 1).then(resolve, reject)
        return
      }
      if (response.statusCode !== 200) {
        response.resume()
        reject(new Error(`HTTP ${response.statusCode ?? 'unknown'}`))
        return
      }
      const file = createWriteStream(destination, { flags: 'wx', mode: 0o755 })
      response.pipe(file)
      file.on('finish', () => {
        file.close((error) => {
          if (error) {
            reject(error)
            return
          }
          resolve()
        })
      })
      file.on('error', reject)
    })
    request.setTimeout(REQUEST_TIMEOUT_MS, () => {
      request.destroy(new Error(`request timed out after ${REQUEST_TIMEOUT_MS / 1000} seconds`))
    })
    request.on('error', reject)
  })
}

function fetchText(url, redirects = 0) {
  return new Promise((resolve, reject) => {
    const request = get(url, (response) => {
      if (response.statusCode && response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) {
        if (redirects >= MAX_REDIRECTS) {
          response.resume()
          reject(new Error(`too many redirects while downloading ${url}`))
          return
        }
        response.resume()
        fetchText(new URL(response.headers.location, url).toString(), redirects + 1).then(resolve, reject)
        return
      }
      if (response.statusCode !== 200) {
        response.resume()
        reject(new Error(`HTTP ${response.statusCode ?? 'unknown'}`))
        return
      }
      const chunks = []
      let size = 0
      response.on('data', (chunk) => {
        size += chunk.length
        if (size > 4096) {
          response.destroy(new Error('checksum response is too large'))
          return
        }
        chunks.push(chunk)
      })
      response.on('end', () => resolve(Buffer.concat(chunks).toString('utf8')))
      response.on('error', reject)
    })
    request.setTimeout(REQUEST_TIMEOUT_MS, () => {
      request.destroy(new Error(`request timed out after ${REQUEST_TIMEOUT_MS / 1000} seconds`))
    })
    request.on('error', reject)
  })
}

function parseChecksum(text, asset) {
  const line = text.split(/\r?\n/).map((item) => item.trim()).find(Boolean)
  if (!line) {
    throw new Error(`checksum file for ${asset} is empty`)
  }
  const [digest, ...nameParts] = line.split(/\s+/)
  if (!/^[a-fA-F0-9]{64}$/.test(digest)) {
    throw new Error(`checksum file for ${asset} does not contain a SHA-256 digest`)
  }
  if (nameParts.length > 0) {
    const listedAsset = basename(nameParts.join(' ').replace(/^\*/, ''))
    if (listedAsset !== asset) {
      throw new Error(`checksum file names ${listedAsset}, expected ${asset}`)
    }
  }
  return digest.toLowerCase()
}

function verifySha256(path, expectedSha256) {
  const actualSha256 = createHash('sha256').update(readFileSync(path)).digest('hex')
  if (actualSha256 !== expectedSha256) {
    throw new Error(`native binary checksum mismatch: expected ${expectedSha256}, got ${actualSha256}`)
  }
}

function assertRegularFile(path) {
  if (!statSync(path).isFile()) {
    throw new Error('downloaded native binary is not a regular file')
  }
}
