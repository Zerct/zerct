import {
  isJsonObject,
  jsonArrayField,
  jsonObjectField,
  jsonObjectOrEmpty,
  numberField,
  optionalJsonObjectField,
  stringField
} from './api.ts'
import type {
  AppsResponse,
  AppSummary,
  BuildRecord,
  BuildStatusResponse,
  CheckoutResponse,
  DeployResponse,
  JsonObject,
  JsonValue,
  LogLine,
  LogsResponse
} from './types.ts'

function appsResponseFromJson(value: JsonValue | null): AppsResponse {
  return {
    apps: jsonArrayField(jsonObjectOrEmpty(value), 'apps')
      .map(appSummaryFromJson)
      .filter((app): app is AppSummary => app !== null)
  }
}

function appSummaryFromJson(value: JsonValue): AppSummary | null {
  if (!isJsonObject(value)) {
    return null
  }
  const app: AppSummary = {}
  const id = stringField(value, 'id')
  const name = stringField(value, 'name')
  const url = stringField(value, 'url')
  const databaseStorageMib = numberField(value, 'databaseStorageMib')
  if (id) {
    app.id = id
  }
  if (name) {
    app.name = name
  }
  if (url) {
    app.url = url
  }
  if (databaseStorageMib > 0) {
    app.databaseStorageMib = databaseStorageMib
  }
  return app
}

function deployResponseFromJson(value: JsonValue | null): DeployResponse {
  const source = jsonObjectOrEmpty(value)
  const finalBuild = optionalJsonObjectField(source, 'final_build')
  const response: DeployResponse = {
    app: appDeployTargetFromJson(jsonObjectField(source, 'app')),
    build_job: {
      id: stringField(jsonObjectField(source, 'build_job'), 'id')
    }
  }
  if (finalBuild) {
    response.final_build = buildRecordFromJson(finalBuild)
  }
  return response
}

function appDeployTargetFromJson(source: JsonObject): DeployResponse['app'] {
  return {
    id: stringField(source, 'id'),
    url: stringField(source, 'url')
  }
}

function buildStatusResponseFromJson(value: JsonValue | null): BuildStatusResponse {
  const source = jsonObjectOrEmpty(value)
  const build = optionalJsonObjectField(source, 'build')
  return build ? { build: buildRecordFromJson(build) } : {}
}

function buildRecordFromJson(source: JsonObject): BuildRecord {
  return {
    id: stringField(source, 'id'),
    status: stringField(source, 'status')
  }
}

function logsResponseFromJson(value: JsonValue | null): LogsResponse {
  const source = jsonObjectOrEmpty(value)
  const lines = jsonArrayField(source, 'lines')
    .map(logLineFromJson)
    .filter((line): line is LogLine => line !== null)
  return {
    lines,
    has_more: source['has_more'] === true,
    next_cursor: stringField(source, 'next_cursor')
  }
}

function logLineFromJson(value: JsonValue): LogLine | null {
  if (!isJsonObject(value)) {
    return null
  }
  const timestamp = stringField(value, 'timestamp')
  const stream = stringField(value, 'stream')
  const message = stringField(value, 'message')
  return timestamp && stream && message ? { timestamp, stream, message } : null
}

function checkoutResponseFromJson(value: JsonValue | null): CheckoutResponse {
  const source = jsonObjectOrEmpty(value)
  return {
    checkout: {
      url: stringField(jsonObjectField(source, 'checkout'), 'url')
    }
  }
}

export {
  appsResponseFromJson,
  buildStatusResponseFromJson,
  checkoutResponseFromJson,
  deployResponseFromJson,
  logsResponseFromJson
}
