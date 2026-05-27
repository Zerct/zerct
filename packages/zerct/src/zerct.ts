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

  switch (cli.command) {
    case 'init':
      initProject(projectPath(cli.args[0]), cli.template)
      break
    case 'install':
      installProject(projectPath(cli.args[0]), cli.template)
      break
    case 'doctor':
      doctorProject(projectPath(cli.args[0]), cli.json)
      break
    case 'preview':
      previewProject(projectPath(cli.args[0]), cli.port)
      break
    case 'login':
      await login(cli)
      break
    case 'deploy':
      await deploy(projectPath(cli.args[0]), cli)
      break
    case 'capabilities':
      await capabilities(cli)
      break
    case 'me':
      await me(cli)
      break
    case 'usage':
      await usage(cli)
      break
    case 'activity':
      await activity(cli)
      break
    case 'apps':
      await apps(cli)
      break
    case 'overview':
      await overview(cli)
      break
    case 'deploys':
      await deploys(cli)
      break
    case 'builds':
      await builds(cli)
      break
    case 'logs':
      await logs(cli)
      break
    case 'status':
      await status(cli)
      break
    case 'inspect':
      await inspect(cli)
      break
    case 'db':
    case 'database':
      await database(cli)
      break
    case 'env':
      await envCommand(cli)
      break
    case 'domains':
      await domainsCommand(cli)
      break
    case 'billing':
      await billing(cli)
      break
    default:
      throw agentError('unknown_command', 'Unknown Zerct command.', 'Run `npx @zerct/zerct --help` and retry with a supported command.', cli.json)
  }
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
