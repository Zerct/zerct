import { existsSync, readFileSync, statSync } from 'node:fs'
import path from 'node:path'

const repoRoot = path.resolve(import.meta.dirname, '..')
const packageDir = path.join(repoRoot, 'packages', 'tovuk')
const packageJsonPath = path.join(packageDir, 'package.json')
const installPath = path.join(packageDir, 'install.mjs')
const binPath = path.join(packageDir, 'bin', 'tovuk')

const requiredFiles = ['bin', 'install.mjs', 'README.md']
const requiredPackageScripts = {
  check: 'npm run check:policy && npm run runtime && npm run pack:dry',
  'check:policy': 'node ../../scripts/check-npm-cli-package.mjs',
  'pack:dry': 'npm pack --dry-run',
  postinstall: 'node install.mjs',
  runtime: 'node ../../scripts/check-npm-native-runtime.mjs'
}

function fail(message) {
  console.error(`npm CLI package check failed: ${message}`)
  process.exit(1)
}

function readJson(filePath, label) {
  try {
    return JSON.parse(readFileSync(filePath, 'utf8'))
  } catch (error) {
    fail(`could not parse ${label}: ${error instanceof Error ? error.message : String(error)}`)
  }
}

function assertObject(value, label) {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    fail(`${label} must be an object`)
  }
  return value
}

function assertArray(value, label) {
  if (!Array.isArray(value)) {
    fail(`${label} must be an array`)
  }
  return value
}

function assertEqual(actual, expected, label) {
  if (actual !== expected) {
    fail(`${label} must be ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`)
  }
}

function assertArrayExactly(actual, expected, label) {
  const sortedActual = [...actual].sort()
  const sortedExpected = [...expected].sort()
  const unexpected = sortedActual.filter((item) => !sortedExpected.includes(item))
  const missing = sortedExpected.filter((item) => !sortedActual.includes(item))
  if (unexpected.length > 0 || missing.length > 0) {
    fail(`${label} must have exactly ${sortedExpected.join(', ')}; unexpected: ${unexpected.join(', ') || 'none'}; missing: ${missing.join(', ') || 'none'}`)
  }
}

function assertKeysExactly(object, expectedKeys, label) {
  assertArrayExactly(Object.keys(object), expectedKeys, label)
}

const packageJson = assertObject(readJson(packageJsonPath, 'packages/tovuk/package.json'), 'package.json')

assertEqual(packageJson.name, 'tovuk', 'package name')
assertEqual(packageJson.type, 'module', 'package type')
assertEqual(packageJson.description, 'Deploy Rust backends, static frontends, and fullstack apps to Tovuk.', 'package description')
assertEqual(packageJson.homepage, 'https://tovuk.com', 'package homepage')
assertEqual(packageJson.license, 'MIT', 'package license')
assertEqual(packageJson.private, undefined, 'package private flag')
assertEqual(assertObject(packageJson.engines, 'engines').node, '>=18.17', 'Node engine')
assertEqual(assertObject(packageJson.publishConfig, 'publishConfig').access, 'public', 'publish access')
assertEqual(assertObject(packageJson.repository, 'repository').directory, 'packages/tovuk', 'repository directory')
assertEqual(assertObject(packageJson.bin, 'package bin').tovuk, 'bin/tovuk', 'tovuk bin path')
assertEqual(packageJson.dependencies, undefined, 'runtime dependencies')
assertEqual(packageJson.devDependencies, undefined, 'development dependencies')
assertArrayExactly(assertArray(packageJson.files, 'files'), requiredFiles, 'published files')

for (const file of requiredFiles) {
  if (!existsSync(path.join(packageDir, file))) {
    fail(`published file entry does not exist: ${file}`)
  }
}

const packageScripts = assertObject(packageJson.scripts, 'scripts')
assertKeysExactly(packageScripts, Object.keys(requiredPackageScripts), 'scripts')
for (const [script, command] of Object.entries(requiredPackageScripts)) {
  assertEqual(packageScripts[script], command, `${script} script`)
}

const installSource = readFileSync(installPath, 'utf8')
for (const snippet of [
  'https://github.com/tovuk/tovuk/releases/download',
  'TOVUK_NATIVE_BINARY',
  'aarch64-apple-darwin',
  'x86_64-unknown-linux-gnu',
  'x86_64-pc-windows-msvc'
]) {
  if (!installSource.includes(snippet)) {
    fail(`install.mjs missing ${snippet}`)
  }
}

if ((statSync(binPath).mode & 0o111) === 0) {
  fail('bin/tovuk must stay executable')
}

console.log('Checked npm native CLI package policy.')
