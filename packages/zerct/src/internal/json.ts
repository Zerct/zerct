import type { JsonObject, JsonValue } from './types.ts'

function parseJson(text: string): JsonValue | null {
  if (!text.trim()) {
    return null
  }
  try {
    return toJsonValue(JSON.parse(text)) ?? null
  } catch {
    return null
  }
}

function toJsonValue(value: unknown): JsonValue | undefined {
  if (value === null || typeof value === 'string' || typeof value === 'boolean') {
    return value
  }
  if (typeof value === 'number') {
    return Number.isFinite(value) ? value : undefined
  }
  if (isUnknownArray(value)) {
    const items: JsonValue[] = []
    for (const item of value) {
      const parsed = toJsonValue(item)
      if (parsed === undefined) {
        return undefined
      }
      items.push(parsed)
    }
    return items
  }
  if (isUnknownRecord(value)) {
    const object: JsonObject = {}
    for (const [key, item] of Object.entries(value)) {
      const parsed = toJsonValue(item)
      if (parsed === undefined) {
        return undefined
      }
      object[key] = parsed
    }
    return object
  }
  return undefined
}

function isUnknownArray(value: unknown): value is readonly unknown[] {
  return Array.isArray(value)
}

function isUnknownRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function isJsonObject(value: JsonValue | null): value is JsonObject {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
}

function jsonObjectOrEmpty(value: JsonValue | null): JsonObject {
  return isJsonObject(value) ? value : {}
}

function jsonObjectField(source: JsonObject, key: string): JsonObject {
  return jsonObjectOrEmpty(source[key] ?? null)
}

function optionalJsonObjectField(source: JsonObject, key: string): JsonObject | null {
  const value = source[key] ?? null
  return isJsonObject(value) ? value : null
}

function jsonArrayField(source: JsonObject, key: string): JsonValue[] {
  const value = source[key]
  return Array.isArray(value) ? value : []
}

function stringField(source: JsonObject, key: string): string {
  const value = source[key]
  return typeof value === 'string' ? value : ''
}

function numberField(source: JsonObject, key: string): number {
  const value = source[key]
  return typeof value === 'number' ? value : Number(value ?? 0)
}

export {
  parseJson,
  isJsonObject,
  jsonObjectOrEmpty,
  jsonObjectField,
  optionalJsonObjectField,
  jsonArrayField,
  stringField,
  numberField
}
