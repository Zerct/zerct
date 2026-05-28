type JsonPrimitive = string | number | boolean | null
export type JsonValue = JsonPrimitive | JsonObject | JsonValue[]
export interface JsonObject {
  [key: string]: JsonValue | undefined
}

export type ApiMethod = 'DELETE' | 'GET' | 'POST' | 'PUT'
export type ProjectKind = 'rust_backend' | 'static_frontend'
export type DiscoveredProjectKind = ProjectKind | 'unknown'
export type TemplateName = 'fullstack-rust-tanstack' | 'rust-api' | 'tanstack-static-frontend'

export interface AgentErrorPayload extends JsonObject {
  code: string
  message: string
  agent_instruction: string | null
  docs_url: string | null
  checkout_url: string | null
}

export interface CliOptions {
  command: string
  args: string[]
  apiUrl: string
  app: string
  build: string
  deploy: string
  limit: string
  cursor: string
  failingCommand: string
  firstLogLine: string
  token: string
  template: string
  severity: string
  port: number
  waitTimeoutSeconds: number
  json: boolean
  database: boolean
  wait: boolean
  help: boolean
  version: boolean
}

export interface PackageManifest {
  name?: string
  scripts?: Record<string, string | undefined>
}

export type BuildConfig = JsonObject & { command: string; check: string; output?: string }
export type RunConfig = JsonObject & { command?: string; port: number; health: string }
export type ResourceConfig = JsonObject & { memory: string; cpu: string; idle_timeout_minutes: number }
export type TovukConfig = JsonObject & { name?: string; kind: ProjectKind; build: BuildConfig; run: RunConfig; resources: ResourceConfig }
export type DoctorCheck = JsonObject & { name: string; ok: boolean; message: string; agent_instruction: string | null }
export type DoctorReport = JsonObject & { ok: boolean; project: string; config: TovukConfig | null; checks: DoctorCheck[] }
export type ProjectDoctorReport = DoctorReport & { relative: string }
export type WorkspaceDoctorReport = JsonObject & { ok: boolean; workspace: string; projects: ProjectDoctorReport[] }
export type DeployProjectInfo = { dir: string; relative: string; name: string; kind: DiscoveredProjectKind }
export type DeployPlanProject = { project: DeployProjectInfo; wantsDatabase: boolean }
export type FrontendSourceReport = { typescript: string[]; javascript: string[] }
export type LoginStartResponse = JsonObject & { loginUrl?: string; userCode?: string; deviceCode?: string; expiresInSeconds?: number; intervalSeconds?: number }
export type LoginPollResponse = JsonObject & { status?: string; token?: string; email?: string; intervalSeconds?: number }
export type AppSummary = JsonObject & { id?: string; name?: string; url?: string; databaseStorageMib?: number }
export type AppsResponse = JsonObject & { apps: AppSummary[] }
export type BuildJob = JsonObject & { id: string }
export type AppDeployTarget = JsonObject & { id: string; url: string }
export type BuildRecord = JsonObject & { id: string; status: string }
export type BuildStatusResponse = JsonObject & { build?: BuildRecord }
export type DeployResponse = JsonObject & { app: AppDeployTarget; build_job: BuildJob; final_build?: BuildRecord | null }
export type WorkspaceDeployResult = { project: DeployProjectInfo; wantsDatabase: boolean; response: DeployResponse; finalBuild?: BuildRecord }
export type LogLine = JsonObject & { timestamp: string; stream: string; message: string }
export type LogsResponse = JsonObject & { lines: LogLine[]; has_more: boolean; next_cursor: string }
export type CheckoutResponse = JsonObject & { checkout: { reason?: string; url: string } }

export type FileVisitor = (file: string, relative: string) => void
export type PathVisitor = (file: string) => void
