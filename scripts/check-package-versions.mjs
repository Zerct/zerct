import { readFileSync } from 'node:fs'
import { execFileSync } from 'node:child_process'

function fail(message) {
  console.error(`package version check failed: ${message}`)
  process.exit(1)
}

function match(source, pattern, label) {
  const result = source.match(pattern)
  if (!result?.[1]) {
    fail(`could not read ${label}`)
  }
  return result[1]
}

const npmPackage = JSON.parse(readFileSync('packages/tovuk/package.json', 'utf8'))
const formula = readFileSync('Formula/tovuk.rb', 'utf8')
const npmCliVersion = execFileSync('packages/tovuk/src/tovuk.ts', ['--version'], {
  env: {
    ...process.env,
    PATH: `packages/tovuk/node_modules/.bin:${process.env.PATH ?? ''}`
  },
  encoding: 'utf8'
}).trim()
if (npmPackage.version !== npmCliVersion) {
  fail(`npm package.json ${npmPackage.version} does not match CLI ${npmCliVersion}`)
}
const formulaNpmVersion = match(
  formula,
  /url "https:\/\/registry\.npmjs\.org\/tovuk\/-\/tovuk-([^"]+)\.tgz"/u,
  'Homebrew npm package version'
)
if (formulaNpmVersion !== npmPackage.version) {
  fail(`Homebrew formula npm version ${formulaNpmVersion} does not match npm package ${npmPackage.version}`)
}

const pyproject = readFileSync('packages/tovuk-py/pyproject.toml', 'utf8')
const pyInit = readFileSync('packages/tovuk-py/src/tovuk/__init__.py', 'utf8')
const pythonCli = readFileSync('packages/tovuk-py/src/tovuk/cli.py', 'utf8')
const pyprojectVersion = match(pyproject, /^version = "([^"]+)"/mu, 'PyPI project version')
const pyInitVersion = match(pyInit, /__version__ = "([^"]+)"/u, 'Python package version')
const pythonNpmVersion = match(pythonCli, /NPM_PACKAGE_VERSION = "([^"]+)"/u, 'Python delegated npm version')
if (pyprojectVersion !== pyInitVersion) {
  fail(`pyproject ${pyprojectVersion} does not match __init__ ${pyInitVersion}`)
}
if (pythonNpmVersion !== npmPackage.version) {
  fail(`Python delegated npm version ${pythonNpmVersion} does not match npm package ${npmPackage.version}`)
}

const cargoToml = readFileSync('crates/tovuk/Cargo.toml', 'utf8')
const cargoLock = readFileSync('crates/tovuk/Cargo.lock', 'utf8')
const cargoCli = readFileSync('crates/tovuk/src/main.rs', 'utf8')
const cargoTomlVersion = match(cargoToml, /^version = "([^"]+)"/mu, 'Cargo.toml version')
const cargoLockVersion = match(cargoLock, /name = "tovuk"\nversion = "([^"]+)"/u, 'Cargo.lock version')
const cargoCliVersion = match(cargoCli, /const VERSION: &str = "([^"]+)"/u, 'Cargo CLI version')
const cargoNpmVersion = match(cargoCli, /const NPM_PACKAGE_VERSION: &str = "([^"]+)"/u, 'Cargo delegated npm version')
if (cargoTomlVersion !== cargoLockVersion || cargoTomlVersion !== cargoCliVersion) {
  fail(`Cargo.toml ${cargoTomlVersion}, Cargo.lock ${cargoLockVersion}, and CLI ${cargoCliVersion} must match`)
}
if (cargoNpmVersion !== npmPackage.version) {
  fail(`Cargo delegated npm version ${cargoNpmVersion} does not match npm package ${npmPackage.version}`)
}

console.log('Checked package version consistency.')
