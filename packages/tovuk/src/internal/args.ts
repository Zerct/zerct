import path from 'node:path'
import { DEFAULT_API_URL, DEFAULT_DEPLOY_WAIT_TIMEOUT_SECONDS } from './constants.ts'
import { agentError } from './errors.ts'
import type { CliOptions } from './types.ts'

type BooleanCliField = 'database' | 'help' | 'json' | 'version' | 'wait'
type StringCliField = 'apiUrl' | 'app' | 'build' | 'cursor' | 'deploy' | 'failingCommand' | 'firstLogLine' | 'limit' | 'severity' | 'template' | 'token'
type NumberCliField = 'port' | 'waitTimeoutSeconds'
type FlagValueKind = 'none' | 'string' | 'positiveInteger'
type FlagSetter = (cli: CliOptions, value: string | null, name: string) => void

interface FlagSpec {
  valueKind: FlagValueKind
  set: FlagSetter
}

interface ParsedFlag {
  name: string
  inlineValue: string | null
}

const FLAG_SPECS = new Map<string, FlagSpec>([
  ['--help', booleanOption('help', true)],
  ['-h', booleanOption('help', true)],
  ['--version', booleanOption('version', true)],
  ['-v', booleanOption('version', true)],
  ['-V', booleanOption('version', true)],
  ['--json', booleanOption('json', true)],
  ['--database', booleanOption('database', true)],
  ['--no-database', booleanOption('database', false)],
  ['--wait', booleanOption('wait', true)],
  ['--wait-timeout', positiveIntegerOption('waitTimeoutSeconds')],
  ['--api', stringOption('apiUrl')],
  ['--app', stringOption('app')],
  ['--build', stringOption('build')],
  ['--deploy', stringOption('deploy')],
  ['--failing-command', stringOption('failingCommand')],
  ['--first-log-line', stringOption('firstLogLine')],
  ['--limit', stringOption('limit')],
  ['--cursor', stringOption('cursor')],
  ['--severity', stringOption('severity')],
  ['--token', stringOption('token')],
  ['--template', stringOption('template')],
  ['--port', positiveIntegerOption('port')]
])

function parseArgs(argv: string[]): CliOptions {
  const cli: CliOptions = {
    command: 'help',
    args: [],
    apiUrl: DEFAULT_API_URL,
    app: '',
    build: '',
    deploy: '',
    limit: '',
    cursor: '',
    failingCommand: '',
    firstLogLine: '',
    token: '',
    template: '',
    severity: '',
    port: 0,
    waitTimeoutSeconds: DEFAULT_DEPLOY_WAIT_TIMEOUT_SECONDS,
    json: false,
    database: false,
    wait: false,
    help: false,
    version: false
  }

  const positional: string[] = []
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index] ?? ''
    if (arg === '--') {
      positional.push(...argv.slice(index + 1))
      break
    }
    const parsedFlag = parseFlag(arg)
    const spec = FLAG_SPECS.get(parsedFlag.name)
    if (spec) {
      index = applyFlag(cli, spec, parsedFlag, argv, index)
    } else if (arg.startsWith('-')) {
      throw unknownFlag(cli, arg)
    } else {
      positional.push(arg)
    }
  }

  if (positional.length > 0) {
    cli.command = positional[0] ?? 'help'
    cli.args = positional.slice(1)
  }

  cli.apiUrl = trimTrailingSlash(cli.apiUrl)
  return cli
}

function booleanOption(field: BooleanCliField, value: boolean): FlagSpec {
  return {
    valueKind: 'none',
    set: (cli): void => {
      cli[field] = value
    }
  }
}

function stringOption(field: StringCliField): FlagSpec {
  return {
    valueKind: 'string',
    set: (cli, value): void => {
      cli[field] = value ?? ''
    }
  }
}

function positiveIntegerOption(field: NumberCliField): FlagSpec {
  return {
    valueKind: 'positiveInteger',
    set: (cli, value, name): void => {
      cli[field] = parsePositiveInteger(value ?? '', name)
    }
  }
}

function applyFlag(cli: CliOptions, spec: FlagSpec, flag: ParsedFlag, argv: readonly string[], index: number): number {
  if (spec.valueKind === 'none') {
    if (flag.inlineValue !== null) {
      throw agentError('invalid_argument', `${flag.name} does not accept a value.`, `Use ${flag.name} without =value.`, cli.json)
    }
    spec.set(cli, null, flag.name)
    return index
  }

  const value = flag.inlineValue ?? requireValue(argv, index, flag.name)
  if (value === '') {
    throw agentError('missing_argument', `${flag.name} requires a value.`, `Pass a value after ${flag.name}.`, cli.json)
  }
  spec.set(cli, value, flag.name)
  return flag.inlineValue === null ? index + 1 : index
}

function parseFlag(arg: string): ParsedFlag {
  if (!arg.startsWith('--')) {
    return { name: arg, inlineValue: null }
  }
  const separator = arg.indexOf('=')
  return separator > 2
    ? { name: arg.slice(0, separator), inlineValue: arg.slice(separator + 1) }
    : { name: arg, inlineValue: null }
}

function parsePositiveInteger(value: string, name: string): number {
  const parsed = Number.parseInt(value, 10)
  if (!Number.isInteger(parsed) || parsed <= 0) {
    throw agentError('invalid_argument', `${name} must be a positive integer.`, `Pass ${name} as seconds, for example ${name} 900.`, false)
  }
  return parsed
}

function requireValue(argv: readonly string[], index: number, name: string): string {
  const value = argv[index + 1]
  if (!value || value.startsWith('--')) {
    throw agentError('missing_argument', `${name} requires a value.`, `Pass a value after ${name}.`, false)
  }
  return value
}

function unknownFlag(cli: CliOptions, value: string): never {
  throw agentError(
    'unknown_argument',
    `Unknown Tovuk option: ${value}.`,
    'Run `npx tovuk --help`, remove or correct the unsupported option, then retry.',
    cli.json
  )
}

function projectPath(value: string | undefined): string {
  return path.resolve(value || process.cwd())
}

function trimTrailingSlash(value: string): string {
  return value.replace(/\/+$/u, '')
}

export { parseArgs, projectPath }
