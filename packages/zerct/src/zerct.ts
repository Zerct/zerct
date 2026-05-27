#!/usr/bin/env tsx
import { VERSION, HELP } from './internal/constants.ts'
import { parseArgs, projectPath } from './internal/args.ts'
import { agentError, printAgentError, ZerctError } from './internal/errors.ts'
import { initProject, installProject } from './internal/templates.ts'
import { doctorProject } from './internal/doctor.ts'
import { previewProject } from './internal/preview.ts'
import { login } from './internal/auth.ts'
import { deploy } from './internal/deploy.ts'
import { apps, builds, activity, billing, capabilities, database, deploys, domainsCommand, envCommand, inspect, logs, me, overview, status, usage } from './internal/commands.ts'
import type { CliOptions } from './internal/types.ts'

type CommandHandler = (cli: CliOptions) => Promise<void> | void

const COMMANDS = new Map<string, CommandHandler>([
  ['init', (cli): void => initProject(projectPath(cli.args[0]), cli.template)],
  ['install', (cli): void => installProject(projectPath(cli.args[0]), cli.template)],
  ['doctor', (cli): void => doctorProject(projectPath(cli.args[0]), cli.json)],
  ['preview', (cli): void => previewProject(projectPath(cli.args[0]), cli.port)],
  ['login', login],
  ['deploy', (cli): Promise<void> => deploy(projectPath(cli.args[0]), cli)],
  ['capabilities', capabilities],
  ['me', me],
  ['usage', usage],
  ['activity', activity],
  ['apps', apps],
  ['overview', overview],
  ['deploys', deploys],
  ['builds', builds],
  ['logs', logs],
  ['status', status],
  ['inspect', inspect],
  ['db', database],
  ['database', database],
  ['env', envCommand],
  ['domains', domainsCommand],
  ['billing', billing]
])

async function main(): Promise<void> {
  const cli = parseArgs(process.argv.slice(2))

  if (cli.help) {
    console.log(HELP)
    return
  }

  if (cli.version) {
    console.log(VERSION)
    return
  }

  const command = COMMANDS.get(cli.command)
  if (!command) {
    throw agentError('unknown_command', 'Unknown Zerct command.', 'Run `npx @zerct/zerct --help` and retry with a supported command.', cli.json)
  }
  await command(cli)
}

main().catch((error: unknown): void => {
  if (error instanceof ZerctError) {
    printAgentError(error.payload, error.json)
    process.exitCode = error.exitCode
    return
  }

  const message = error instanceof Error ? error.message : String(error)
  console.error(`zerct failed: ${message}`)
  process.exitCode = 1
})
