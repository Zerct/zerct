#!/usr/bin/env node
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
  const tempPath = join(tmpdir(), `${basename(asset)}-${process.pid}`)
  try {
    await download(url, tempPath)
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
        download(response.headers.location, destination).then(resolve, reject)
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
