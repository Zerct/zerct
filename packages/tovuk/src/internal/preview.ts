import { spawn, spawnSync } from 'node:child_process'
import { existsSync, readFileSync, statSync } from 'node:fs'
import { createServer, request as httpRequest } from 'node:http'
import path from 'node:path'
import { parseTovukToml, validateConfig } from './config.ts'
import { runDoctorWorkspace } from './doctor.ts'
import { agentError } from './errors.ts'
import { ensureDirectory } from './project.ts'
import type { TovukConfig } from './types.ts'
import type { IncomingMessage, ServerResponse } from 'node:http'

function previewProject(projectDir: string, port: number): void {
  const config = previewConfig(projectDir)
  previewValidatedProject(projectDir, config, port)
}

function previewConfig(projectDir: string): TovukConfig {
  const report = runDoctorWorkspace(projectDir)
  if ('projects' in report) {
    throw agentError('workspace_preview_unsupported', 'Preview one project at a time.', 'Run `tovuk preview <project-dir>` for one discovered project, or use a fullstack root tovuk.toml.', false)
  }
  if (!report.ok) {
    const firstFailure = report.checks.find((check) => !check.ok)
    throw agentError('doctor_failed', 'Tovuk doctor failed.', firstFailure?.agent_instruction || 'Fix the failed checks and retry `tovuk preview`.', false)
  }

  const config = parseTovukToml(readFileSync(path.join(projectDir, 'tovuk.toml'), 'utf8'), projectDir)
  validateConfig(config)
  return config
}

function previewValidatedProject(projectDir: string, config: TovukConfig, port: number): void {
  if (config.kind === 'fullstack') {
    previewFullstack(projectDir, config, port)
    return
  }
  runShell(config.build.command, projectDir, 'Build failed before preview.')
  if (config.kind === 'static_frontend') {
    previewStatic(projectDir, config.build.output ?? 'dist', port)
    return
  }
  previewRuntime(projectDir, config.run.command ?? '', port || config.run.port)
}

function previewFullstack(projectDir: string, config: TovukConfig, port: number): void {
  const backendDir = path.join(projectDir, config.backend.root ?? '')
  const frontendDir = path.join(projectDir, config.frontend.root ?? '')
  const backendPort = config.backend.port ?? 3000
  runShell(config.backend.build ?? '', backendDir, 'Backend build failed before preview.')
  runShell(config.frontend.build ?? '', frontendDir, 'Frontend build failed before preview.')
  const backend = spawn(config.backend.command ?? '', {
    cwd: backendDir,
    env: { ...process.env, PORT: String(backendPort) },
    shell: true,
    stdio: 'inherit'
  })
  backend.on('error', (error) => {
    throw agentError('preview_failed', 'Backend preview command failed.', error.message, false)
  })
  const stopBackend = (): void => {
    if (!backend.killed) {
      backend.kill('SIGTERM')
    }
  }
  process.once('SIGINT', stopBackend)
  process.once('SIGTERM', stopBackend)
  serveStatic(path.join(frontendDir, config.frontend.output ?? 'dist'), port || 4173, backendPort)
}

function previewStatic(projectDir: string, output: string, port: number): void {
  serveStatic(path.join(projectDir, output), port || 4173)
}

function previewRuntime(projectDir: string, command: string, port: number): void {
  console.log(`preview http://127.0.0.1:${port}`)
  const result = spawnSync(command, {
    cwd: projectDir,
    env: { ...process.env, PORT: String(port) },
    shell: true,
    stdio: 'inherit'
  })
  if (result.error) {
    throw agentError('preview_failed', 'Preview command failed.', result.error.message, false)
  }
  if (result.status !== 0) {
    throw agentError('preview_failed', 'Preview command exited with an error.', 'Fix the local runtime command and retry `tovuk preview`.', false)
  }
}

function runShell(command: string, projectDir: string, failureMessage: string): void {
  console.log(command)
  const result = spawnSync(command, {
    cwd: projectDir,
    env: process.env,
    shell: true,
    stdio: 'inherit'
  })
  if (result.error) {
    throw agentError('command_failed', failureMessage, result.error.message, false)
  }
  if (result.status !== 0) {
    throw agentError('command_failed', failureMessage, 'Fix the command output above, then retry.', false)
  }
}

function serveStatic(root: string, port: number, apiProxyPort?: number): void {
  ensureDirectory(root)
  const server = createServer((request, response) => {
    const pathname = decodeURIComponent(new URL(request.url || '/', `http://127.0.0.1:${port}`).pathname)
    if (apiProxyPort && (pathname === '/api' || pathname.startsWith('/api/'))) {
      proxyToBackend(request, response, apiProxyPort)
      return
    }
    const target = staticTarget(root, pathname)
    if (!target) {
      response.writeHead(404, { 'content-type': 'text/plain; charset=utf-8' })
      response.end('not found')
      return
    }
    response.writeHead(200, { 'content-type': contentType(target) })
    response.end(readFileSync(target))
  })
  server.listen(port, '127.0.0.1', () => {
    console.log(`preview http://127.0.0.1:${port}`)
  })
}

function proxyToBackend(clientRequest: IncomingMessage, response: ServerResponse, port: number): void {
  const upstream = httpRequest({
    hostname: '127.0.0.1',
    port,
    method: clientRequest.method,
    path: clientRequest.url ?? '/',
    headers: { ...clientRequest.headers, host: `127.0.0.1:${port}` }
  }, (upstreamResponse) => {
    response.writeHead(upstreamResponse.statusCode ?? 502, upstreamResponse.headers)
    upstreamResponse.pipe(response)
  })
  upstream.on('error', () => {
    response.writeHead(502, { 'content-type': 'text/plain; charset=utf-8' })
    response.end('backend unavailable')
  })
  clientRequest.pipe(upstream)
}

function staticTarget(root: string, pathname: string): string {
  const safePath = pathname.replace(/^\/+/u, '')
  const candidate = path.resolve(root, safePath || 'index.html')
  if (!candidate.startsWith(path.resolve(root) + path.sep) && candidate !== path.resolve(root)) {
    return ''
  }
  if (existsSync(candidate) && statSync(candidate).isFile()) {
    return candidate
  }
  const index = path.join(root, 'index.html')
  return existsSync(index) ? index : ''
}

function contentType(file: string): string {
  if (file.endsWith('.html')) {
    return 'text/html; charset=utf-8'
  }
  if (file.endsWith('.css')) {
    return 'text/css; charset=utf-8'
  }
  if (file.endsWith('.js') || file.endsWith('.mjs')) {
    return 'text/javascript; charset=utf-8'
  }
  if (file.endsWith('.json')) {
    return 'application/json; charset=utf-8'
  }
  if (file.endsWith('.svg')) {
    return 'image/svg+xml'
  }
  return 'application/octet-stream'
}

export { previewProject }
