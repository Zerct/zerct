#!/usr/bin/env node
import { spawnSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const binDir = dirname(fileURLToPath(import.meta.url))
const nativeBinary = join(binDir, process.platform === 'win32' ? 'tovuk-native.exe' : 'tovuk-native')

if (!existsSync(nativeBinary)) {
  printMissingNativeBinary()
  process.exit(1)
}

const result = spawnSync(nativeBinary, process.argv.slice(2), {
  stdio: 'inherit',
  windowsHide: false,
})

if (result.error) {
  printLaunchError(result.error)
  process.exit(1)
}

if (result.signal) {
  process.kill(process.pid, result.signal)
}

process.exit(result.status ?? 1)

function printMissingNativeBinary() {
  if (jsonOutputRequested()) {
    console.error(JSON.stringify({
      code: 'native_binary_unavailable',
      message: 'Tovuk native binary was not installed.',
      agent_instruction: 'Reinstall with npm scripts enabled, install from GitHub Releases, Homebrew, Cargo, or rerun with TOVUK_NATIVE_BINARY pointing to a supported native binary.',
      docs_url: 'https://docs.tovuk.com/reference/packages',
      checkout_url: null,
    }, null, 2))
    return
  }

  console.error('Tovuk native binary was not installed. Reinstall with npm scripts enabled, or install from https://github.com/tovuk/tovuk/releases.')
}

function printLaunchError(error) {
  if (jsonOutputRequested()) {
    console.error(JSON.stringify({
      code: 'native_binary_launch_failed',
      message: `Tovuk native binary could not start: ${error.message}`,
      agent_instruction: 'Reinstall the Tovuk npm package, or install with Homebrew, Cargo, or GitHub Releases.',
      docs_url: 'https://docs.tovuk.com/reference/packages',
      checkout_url: null,
    }, null, 2))
    return
  }

  console.error(`Tovuk native binary could not start: ${error.message}`)
}

function jsonOutputRequested() {
  if (/^json$/i.test(process.env.TOVUK_OUTPUT ?? '')) {
    return true
  }
  const args = process.argv.slice(2)
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index]
    if (arg === '--json' || /^--output=json$/i.test(arg)) {
      return true
    }
    if (arg === '--output' && /^json$/i.test(args[index + 1] ?? '')) {
      return true
    }
  }
  return false
}
