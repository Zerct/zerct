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

const npmPackage = JSON.parse(readFileSync('packages/tovuk/package.json', 'utf8'))
const pyproject = readFileSync('packages/tovuk-py/pyproject.toml', 'utf8')
const pyInit = readFileSync('packages/tovuk-py/src/tovuk/__init__.py', 'utf8')
const pythonCli = readFileSync('packages/tovuk-py/src/tovuk/cli.py', 'utf8')
const cargoToml = readFileSync('crates/tovuk/Cargo.toml', 'utf8')
const cargoLock = readFileSync('crates/tovuk/Cargo.lock', 'utf8')
const cargoCli = readFileSync('crates/tovuk/src/main.rs', 'utf8')
const formula = readFileSync('Formula/tovuk.rb', 'utf8')

const pyprojectVersion = match(pyproject, /^version = "([^"]+)"/mu, 'PyPI project version')
const pyInitVersion = match(pyInit, /__version__ = "([^"]+)"/u, 'Python package version')
const cargoTomlVersion = match(cargoToml, /^version = "([^"]+)"/mu, 'Cargo.toml version')
const cargoLockVersion = match(cargoLock, /name = "tovuk"\nversion = "([^"]+)"/u, 'Cargo.lock version')
const cargoCliVersion = match(cargoCli, /const VERSION: &str = "([^"]+)"/u, 'Cargo CLI version')

for (const [label, version] of [
  ['PyPI project', pyprojectVersion],
  ['Python package', pyInitVersion],
  ['Cargo.toml', cargoTomlVersion],
  ['Cargo.lock', cargoLockVersion],
  ['Cargo CLI', cargoCliVersion]
]) {
  if (version !== npmPackage.version) {
    fail(`${label} ${version} does not match npm package ${npmPackage.version}`)
  }
}

if (!pythonCli.includes('releases/download/v{__version__}') || !pythonCli.includes('tovuk-{__version__}-')) {
  fail('Python native binary downloader must derive release assets from __version__')
}

if (!formula.includes(`tag: "v${npmPackage.version}"`)) {
  fail(`Homebrew formula must pin v${npmPackage.version}`)
}

console.log('Checked package version consistency.')
