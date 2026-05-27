import path from 'node:path'
import { DEFAULT_API_URL, DEFAULT_DEPLOY_WAIT_TIMEOUT_SECONDS } from './constants.ts'
import { agentError } from './errors.ts'
import type { CliOptions } from './types.ts'

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
    token: '',
    template: '',
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
    if (arg === '--help' || arg === '-h') {
      cli.help = true
    } else if (arg === '--version' || arg === '-v') {
      cli.version = true
    } else if (arg === '--json') {
      cli.json = true
    } else if (arg === '--database') {
      cli.database = true
    } else if (arg === '--no-database') {
      cli.database = false
    } else if (arg === '--wait') {
      cli.wait = true
    } else if (arg === '--wait-timeout') {
      cli.waitTimeoutSeconds = parsePositiveInteger(requireValue(argv, index, '--wait-timeout'), '--wait-timeout')
      index += 1
    } else if (arg === '--api') {
      cli.apiUrl = requireValue(argv, index, '--api')
      index += 1
    } else if (arg === '--app') {
      cli.app = requireValue(argv, index, '--app')
      index += 1
    } else if (arg === '--build') {
      cli.build = requireValue(argv, index, '--build')
      index += 1
    } else if (arg === '--deploy') {
      cli.deploy = requireValue(argv, index, '--deploy')
      index += 1
    } else if (arg === '--limit') {
      cli.limit = requireValue(argv, index, '--limit')
      index += 1
    } else if (arg === '--cursor') {
      cli.cursor = requireValue(argv, index, '--cursor')
      index += 1
    } else if (arg === '--token') {
      cli.token = requireValue(argv, index, '--token')
      index += 1
    } else if (arg === '--template') {
      cli.template = requireValue(argv, index, '--template')
      index += 1
    } else if (arg === '--port') {
      cli.port = parsePositiveInteger(requireValue(argv, index, '--port'), '--port')
      index += 1
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

function parsePositiveInteger(value: string, name: string): number {
  const parsed = Number.parseInt(value, 10)
  if (!Number.isInteger(parsed) || parsed <= 0) {
    throw agentError('invalid_argument', `${name} must be a positive integer.`, `Pass ${name} as seconds, for example ${name} 900.`, false)
  }
  return parsed
}

function requireValue(argv: string[], index: number, name: string): string {
  const value = argv[index + 1]
  if (!value || value.startsWith('--')) {
    throw agentError('missing_argument', `${name} requires a value.`, `Pass a value after ${name}.`, false)
  }
  return value
}

function projectPath(value: string | undefined): string {
  return path.resolve(value || process.cwd())
}

function trimTrailingSlash(value: string): string {
  return value.replace(/\/+$/u, '')
}

export { parseArgs, projectPath }
