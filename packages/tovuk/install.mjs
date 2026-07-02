#!/usr/bin/env node
import { createHash } from 'node:crypto'
import { createWriteStream, chmodSync, copyFileSync, mkdirSync, readFileSync, renameSync, rmSync } from 'node:fs'
import { get } from 'node:https'
import { arch, platform, tmpdir } from 'node:os'
import { basename, dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const packageRoot = dirname(fileURLToPath(import.meta.url))
const manifest = JSON.parse(readFileSync(join(packageRoot, 'package.json'), 'utf8'))
const target = nativeTarget()
const binaryPath = join(packageRoot, 'bin', 'tovuk')

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
  const asset = `tovuk-${manifest.version}-${target}${target.endsWith('windows-msvc') ? '.exe' : ''}`
  const url = `https://github.com/tovuk/tovuk/releases/download/v${manifest.version}/${asset}`
  const checksumUrl = `${url}.sha256`
  const tempPath = join(tmpdir(), `${basename(asset)}-${process.pid}`)
  try {
    await download(url, tempPath)
    const expectedSha256 = parseChecksum(await fetchText(checksumUrl), asset)
    verifySha256(tempPath, expectedSha256)
    renameSync(tempPath, binaryPath)
    chmodSync(binaryPath, 0o755)
  } catch (error) {
    rmSync(tempPath, { force: true })
    throw new Error(`Could not install native Tovuk binary from ${url}: ${error instanceof Error ? error.message : String(error)}`)
  }
}

function nativeTarget() {
  const os = platform()
  const cpu = arch()
  if (os === 'darwin' && cpu === 'arm64') return 'aarch64-apple-darwin'
  if (os === 'darwin' && cpu === 'x64') return 'x86_64-apple-darwin'
  if (os === 'linux' && cpu === 'arm64') return 'aarch64-unknown-linux-gnu'
  if (os === 'linux' && cpu === 'x64') return 'x86_64-unknown-linux-gnu'
  if (os === 'win32' && cpu === 'x64') return 'x86_64-pc-windows-msvc'
  throw new Error(`Unsupported Tovuk native target: ${os}/${cpu}`)
}

function download(url, destination) {
  return new Promise((resolve, reject) => {
    get(url, (response) => {
      if (response.statusCode && response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) {
        response.resume()
        download(new URL(response.headers.location, url).toString(), destination).then(resolve, reject)
        return
      }
      if (response.statusCode !== 200) {
        response.resume()
        reject(new Error(`HTTP ${response.statusCode ?? 'unknown'}`))
        return
      }
      const file = createWriteStream(destination, { mode: 0o755 })
      response.pipe(file)
      file.on('finish', () => file.close(resolve))
      file.on('error', reject)
    }).on('error', reject)
  })
}

function fetchText(url) {
  return new Promise((resolve, reject) => {
    get(url, (response) => {
      if (response.statusCode && response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) {
        response.resume()
        fetchText(new URL(response.headers.location, url).toString()).then(resolve, reject)
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
    }).on('error', reject)
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
