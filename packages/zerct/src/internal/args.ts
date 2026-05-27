import path from 'node:path'
import { DEFAULT_API_URL, DEFAULT_DEPLOY_WAIT_TIMEOUT_SECONDS } from './constants.ts'
import { agentError } from './errors.ts'
import type { CliOptions } from './types.ts'

type FlagHandler = (cli: CliOptions, argv: readonly string[], index: number) => number
type BooleanCliField = 'database' | 'help' | 'json' | 'version' | 'wait'
type StringCliField = 'apiUrl' | 'app' | 'build' | 'cursor' | 'deploy' | 'failingCommand' | 'firstLogLine' | 'limit' | 'severity' | 'template' | 'token'
type NumberCliField = 'port' | 'waitTimeoutSeconds'

const FLAG_HANDLERS = new Map<string, FlagHandler>([
  ['--help', booleanOption('help', true)],
  ['-h', booleanOption('help', true)],
  ['--version', booleanOption('version', true)],
  ['-v', booleanOption('version', true)],
  ['-V', booleanOption('version', true)],
  ['--json', booleanOption('json', true)],
  ['--database', booleanOption('database', true)],
  ['--no-database', booleanOption('database', false)],
  ['--wait', booleanOption('wait', true)],
  ['--wait-timeout', positiveIntegerOption('--wait-timeout', 'waitTimeoutSeconds')],
  ['--api', stringOption('--api', 'apiUrl')],
  ['--app', stringOption('--app', 'app')],
  ['--build', stringOption('--build', 'build')],
  ['--deploy', stringOption('--deploy', 'deploy')],
  ['--failing-command', stringOption('--failing-command', 'failingCommand')],
  ['--first-log-line', stringOption('--first-log-line', 'firstLogLine')],
  ['--limit', stringOption('--limit', 'limit')],
  ['--cursor', stringOption('--cursor', 'cursor')],
  ['--severity', stringOption('--severity', 'severity')],
  ['--token', stringOption('--token', 'token')],
  ['--template', stringOption('--template', 'template')],
  ['--port', positiveIntegerOption('--port', 'port')]
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
    const handler = FLAG_HANDLERS.get(arg)
    if (handler) {
      index = handler(cli, argv, index)
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

function booleanOption(field: BooleanCliField, value: boolean): FlagHandler {
  return (cli, _argv, index): number => {
    cli[field] = value
    return index
  }
}

function stringOption(name: string, field: StringCliField): FlagHandler {
  return (cli, argv, index): number => {
    cli[field] = requireValue(argv, index, name)
    return index + 1
  }
}

function positiveIntegerOption(name: string, field: NumberCliField): FlagHandler {
  return (cli, argv, index): number => {
    cli[field] = parsePositiveInteger(requireValue(argv, index, name), name)
    return index + 1
  }
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
    `Unknown Zerct option: ${value}.`,
    'Run `npx @zerct/zerct --help`, remove or correct the unsupported option, then retry.',
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
