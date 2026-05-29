import { execFileSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import path from 'node:path'

const repoRoot = path.resolve(import.meta.dirname, '..')
const binary = process.env.TOVUK_NATIVE_BINARY || path.join(repoRoot, 'packages', 'tovuk', 'bin', 'tovuk')

if (!existsSync(binary)) {
  console.error(`native Tovuk binary does not exist: ${binary}`)
  process.exit(1)
}

execFileSync(binary, ['--version'], { stdio: 'inherit' })
