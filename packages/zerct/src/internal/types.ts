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

export interface BuildConfig extends JsonObject {
  command: string
  check: string
  output?: string
}

export interface RunConfig extends JsonObject {
  command?: string
  port: number
  health: string
}

export interface ResourceConfig extends JsonObject {
  memory: string
  cpu: string
  idle_timeout_minutes: number
}

export interface ZerctConfig extends JsonObject {
  name?: string
  kind: ProjectKind
  build: BuildConfig
  run: RunConfig
  resources: ResourceConfig
}

export interface DoctorCheck extends JsonObject {
  name: string
  ok: boolean
  message: string
  agent_instruction: string
}

export interface DoctorReport extends JsonObject {
  ok: boolean
  project: string
  config: ZerctConfig | null
  checks: DoctorCheck[]
}

export interface ProjectDoctorReport extends DoctorReport {
  relative: string
}

export interface WorkspaceDoctorReport extends JsonObject {
  ok: boolean
  workspace: string
  projects: ProjectDoctorReport[]
}

export interface DeployProjectInfo {
  dir: string
  relative: string
  name: string
  kind: DiscoveredProjectKind
}

export interface DeployPlanProject {
  project: DeployProjectInfo
  wantsDatabase: boolean
}

export interface FrontendSourceReport {
  typescript: string[]
  javascript: string[]
}

export interface LoginStartResponse extends JsonObject {
  loginUrl?: string
  userCode?: string
  deviceCode?: string
  expiresInSeconds?: number
  intervalSeconds?: number
}

export interface LoginPollResponse extends JsonObject {
  status?: string
  token?: string
  email?: string
  intervalSeconds?: number
}

export interface AppSummary extends JsonObject {
  id?: string
  name?: string
  url?: string
  databaseStorageMib?: number
}

export interface UsageResponse extends JsonObject {
  usage?: JsonObject
  limits?: JsonObject
}

export interface AppsResponse extends JsonObject {
  apps: AppSummary[]
}

export interface BuildJob extends JsonObject {
  id: string
}

export interface AppDeployTarget extends JsonObject {
  id: string
  url: string
}

export interface BuildRecord extends JsonObject {
  id: string
  status: string
}

export interface BuildStatusResponse extends JsonObject {
  build?: BuildRecord
}

export interface DeployResponse extends JsonObject {
  app: AppDeployTarget
  build_job: BuildJob
  final_build?: BuildRecord | null
}

export interface WorkspaceDeployResult {
  project: DeployProjectInfo
  wantsDatabase: boolean
  response: DeployResponse
  finalBuild?: BuildRecord
}

export interface LogLine extends JsonObject {
  timestamp: string
  stream: string
  message: string
}

export interface LogsResponse extends JsonObject {
  lines: LogLine[]
  has_more: boolean
  next_cursor: string
}

export interface CheckoutResponse extends JsonObject {
  checkout: {
    reason?: string
    url: string
  }
}

export type FileVisitor = (file: string, relative: string) => void
export type PathVisitor = (file: string) => void
