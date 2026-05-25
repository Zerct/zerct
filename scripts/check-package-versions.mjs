import { readFileSync } from 'node:fs'

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

const npmPackage = JSON.parse(readFileSync('packages/zerct/package.json', 'utf8'))
const npmCli = readFileSync('packages/zerct/bin/zerct.js', 'utf8')
const npmCliVersion = match(npmCli, /const VERSION = '([^']+)'/u, 'npm CLI version')
if (npmPackage.version !== npmCliVersion) {
  fail(`npm package.json ${npmPackage.version} does not match CLI ${npmCliVersion}`)
}

const pyproject = readFileSync('packages/zerct-py/pyproject.toml', 'utf8')
const pyInit = readFileSync('packages/zerct-py/src/zerct/__init__.py', 'utf8')
const pyprojectVersion = match(pyproject, /^version = "([^"]+)"/mu, 'PyPI project version')
const pyInitVersion = match(pyInit, /__version__ = "([^"]+)"/u, 'Python package version')
if (pyprojectVersion !== pyInitVersion) {
  fail(`pyproject ${pyprojectVersion} does not match __init__ ${pyInitVersion}`)
}

const cargoToml = readFileSync('crates/zerct/Cargo.toml', 'utf8')
const cargoLock = readFileSync('crates/zerct/Cargo.lock', 'utf8')
const cargoCli = readFileSync('crates/zerct/src/main.rs', 'utf8')
const cargoTomlVersion = match(cargoToml, /^version = "([^"]+)"/mu, 'Cargo.toml version')
const cargoLockVersion = match(cargoLock, /name = "zerct"\nversion = "([^"]+)"/u, 'Cargo.lock version')
const cargoCliVersion = match(cargoCli, /const VERSION: &str = "([^"]+)"/u, 'Cargo CLI version')
if (cargoTomlVersion !== cargoLockVersion || cargoTomlVersion !== cargoCliVersion) {
  fail(`Cargo.toml ${cargoTomlVersion}, Cargo.lock ${cargoLockVersion}, and CLI ${cargoCliVersion} must match`)
}

console.log('Checked package version consistency.')
