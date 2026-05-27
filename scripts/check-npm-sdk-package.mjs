import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs'
import path from 'node:path'

const repoRoot = path.resolve(import.meta.dirname, '..')
const packageDir = path.join(repoRoot, 'packages', 'zerct')
const packageJsonPath = path.join(packageDir, 'package.json')
const tsconfigPath = path.join(packageDir, 'tsconfig.json')
const binPath = path.join(packageDir, 'src', 'zerct.ts')

const disallowedJavaScriptExtensions = new Set(['.js', '.jsx', '.mjs', '.cjs'])
const requiredFiles = ['src', 'tsconfig.json', 'README.md']
const forbiddenLifecycleScripts = [
  'preinstall',
  'install',
  'postinstall',
  'prepare',
  'prepublish',
  'prepublishOnly',
  'prepack',
  'postpack'
]
const requiredRuntimeDependencies = ['tsx']
const requiredDevelopmentDependencies = [
  '@types/node',
  '@typescript/native-preview',
  'oxlint',
  'oxlint-tsgolint',
  'publint',
  'type-coverage',
  'typescript'
]

const requiredPackageScripts = {
  check: 'npm run check:policy && npm run typecheck && npm run type-coverage && npm run lint && npm run lint:package && npm run check:deps && npm run runtime && npm run pack:dry',
  'check:deps': 'npm ls --all && npm audit --audit-level=moderate && npm audit signatures --omit=dev',
  'check:policy': 'node ../../scripts/check-npm-sdk-package.mjs',
  lint: 'oxlint src -D correctness -D suspicious -D perf -A no-await-in-loop --deny-warnings --type-aware --type-check --tsconfig tsconfig.json --promise-plugin --node-plugin --report-unused-disable-directives',
  'lint:package': 'publint --strict --pack npm',
  'pack:dry': 'npm pack --dry-run',
  runtime: 'src/zerct.ts --version',
  'type-coverage': 'type-coverage --project tsconfig.json --strict --at-least 100',
  typecheck: 'npm run typecheck:tsc && npm run typecheck:tsgo',
  'typecheck:tsc': 'tsc --noEmit -p tsconfig.json',
  'typecheck:tsgo': 'tsgo --noEmit -p tsconfig.json'
}

const requiredCompilerOptions = {
  target: 'ES2023',
  module: 'NodeNext',
  moduleResolution: 'NodeNext',
  rootDir: 'src',
  allowImportingTsExtensions: true,
  allowJs: false,
  checkJs: false,
  declaration: true,
  noEmit: true,
  noEmitOnError: true,
  strict: true,
  strictBuiltinIteratorReturn: true,
  noImplicitAny: true,
  strictNullChecks: true,
  strictFunctionTypes: true,
  strictBindCallApply: true,
  strictPropertyInitialization: true,
  noImplicitThis: true,
  useUnknownInCatchVariables: true,
  alwaysStrict: true,
  exactOptionalPropertyTypes: true,
  noUncheckedIndexedAccess: true,
  noUncheckedSideEffectImports: true,
  noPropertyAccessFromIndexSignature: true,
  noImplicitOverride: true,
  noImplicitReturns: true,
  noFallthroughCasesInSwitch: true,
  noUnusedLocals: true,
  noUnusedParameters: true,
  allowUnreachableCode: false,
  allowUnusedLabels: false,
  verbatimModuleSyntax: true,
  isolatedModules: true,
  isolatedDeclarations: true,
  erasableSyntaxOnly: true,
  moduleDetection: 'force',
  resolvePackageJsonExports: true,
  resolvePackageJsonImports: true,
  maxNodeModuleJsDepth: 0,
  forceConsistentCasingInFileNames: true,
  skipLibCheck: false
}

function fail(message) {
  console.error(`npm SDK package check failed: ${message}`)
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

function assertIncludesEvery(actual, expected, label) {
  const missing = expected.filter((item) => !actual.includes(item))
  if (missing.length > 0) {
    fail(`${label} is missing ${missing.join(', ')}`)
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
  const actualKeys = Object.keys(object).sort()
  const sortedExpectedKeys = [...expectedKeys].sort()
  const unexpected = actualKeys.filter((key) => !sortedExpectedKeys.includes(key))
  const missing = sortedExpectedKeys.filter((key) => !actualKeys.includes(key))

  if (unexpected.length > 0 || missing.length > 0) {
    fail(`${label} must have exactly ${sortedExpectedKeys.join(', ')}; unexpected: ${unexpected.join(', ') || 'none'}; missing: ${missing.join(', ') || 'none'}`)
  }
}

function walkFiles(directory, ignoredDirectories = new Set(['node_modules'])) {
  const files = []
  for (const entry of readdirSync(directory)) {
    if (ignoredDirectories.has(entry)) {
      continue
    }

    const entryPath = path.join(directory, entry)
    const entryStat = statSync(entryPath)
    if (entryStat.isDirectory()) {
      files.push(...walkFiles(entryPath, ignoredDirectories))
      continue
    }

    files.push(entryPath)
  }
  return files
}

function relativeToPackage(filePath) {
  return path.relative(packageDir, filePath)
}

const packageJson = assertObject(readJson(packageJsonPath, 'packages/zerct/package.json'), 'package.json')
const tsconfig = assertObject(readJson(tsconfigPath, 'packages/zerct/tsconfig.json'), 'tsconfig.json')
const compilerOptions = assertObject(tsconfig.compilerOptions, 'tsconfig.compilerOptions')

const javascriptFiles = walkFiles(packageDir).filter((filePath) => disallowedJavaScriptExtensions.has(path.extname(filePath)))
if (javascriptFiles.length > 0) {
  fail(`packages/zerct must stay TypeScript-only: ${javascriptFiles.map(relativeToPackage).join(', ')}`)
}

assertEqual(packageJson.name, 'zerct', 'package name')
assertEqual(packageJson.type, 'module', 'package type')
assertEqual(packageJson.description, 'Deploy Rust backends and static frontends to Zerct.', 'package description')
assertEqual(packageJson.homepage, 'https://zerct.com', 'package homepage')
assertEqual(packageJson.license, 'MIT', 'package license')
assertEqual(packageJson.private, undefined, 'package private flag')
assertEqual(assertObject(packageJson.engines, 'engines').node, '>=18.17', 'Node engine')
assertEqual(assertObject(packageJson.publishConfig, 'publishConfig').access, 'public', 'publish access')
assertEqual(assertObject(packageJson.repository, 'repository').directory, 'packages/zerct', 'repository directory')
assertEqual(assertObject(packageJson.bin, 'package bin').zerct, 'src/zerct.ts', 'zerct bin path')
assertEqual(assertObject(packageJson.dependencies, 'dependencies').tsx, '^4.22.3', 'runtime tsx dependency')
assertKeysExactly(packageJson.dependencies, requiredRuntimeDependencies, 'runtime dependencies')
assertIncludesEvery(Object.keys(assertObject(packageJson.devDependencies, 'devDependencies')), requiredDevelopmentDependencies, 'devDependencies')
assertArrayExactly(assertArray(packageJson.files, 'files'), requiredFiles, 'published files')

for (const file of requiredFiles) {
  if (!existsSync(path.join(packageDir, file))) {
    fail(`published file entry does not exist: ${file}`)
  }
}

const binSource = readFileSync(binPath, 'utf8')
if (!binSource.startsWith('#!/usr/bin/env tsx\n')) {
  fail('src/zerct.ts must keep the tsx shebang')
}
if ((statSync(binPath).mode & 0o111) === 0) {
  fail('src/zerct.ts must stay executable')
}

const packageScripts = assertObject(packageJson.scripts, 'scripts')
for (const script of forbiddenLifecycleScripts) {
  if (Object.hasOwn(packageScripts, script)) {
    fail(`forbidden lifecycle script is present: ${script}`)
  }
}

assertKeysExactly(packageScripts, Object.keys(requiredPackageScripts), 'scripts')
for (const [script, command] of Object.entries(requiredPackageScripts)) {
  assertEqual(packageScripts[script], command, `${script} script`)
}

for (const [option, expectedValue] of Object.entries(requiredCompilerOptions)) {
  assertEqual(compilerOptions[option], expectedValue, `compiler option ${option}`)
}

assertIncludesEvery(assertArray(compilerOptions.lib, 'compilerOptions.lib'), ['ES2023', 'DOM'], 'compilerOptions.lib')
assertIncludesEvery(assertArray(compilerOptions.types, 'compilerOptions.types'), ['node'], 'compilerOptions.types')
assertIncludesEvery(assertArray(tsconfig.include, 'tsconfig.include'), ['src/**/*.ts'], 'tsconfig.include')

console.log('Checked npm SDK package policy.')
