

import type { AgentErrorPayload } from './types.ts'

function agentError(code: string, message: string, agentInstruction: string, json: boolean): ZerctError {
  return new ZerctError({
    code,
    message,
    agent_instruction: agentInstruction,
    docs_url: null,
    checkout_url: null
  }, json, 1)
}

function printAgentError(payload: AgentErrorPayload, json: boolean): void {
  if (json) {
    console.error(JSON.stringify(payload, null, 2))
    return
  }

  console.error(payload.message || 'Zerct command failed.')
  if (payload.agent_instruction) {
    console.error(`agent_instruction: ${payload.agent_instruction}`)
  }
  if (payload.docs_url) {
    console.error(`docs: ${payload.docs_url}`)
  }
  if (payload.checkout_url) {
    console.error(`checkout: ${payload.checkout_url}`)
  }
}

class ZerctError extends Error {
  payload: AgentErrorPayload
  json: boolean
  exitCode: number

  constructor(payload: AgentErrorPayload, json: boolean, exitCode: number) {
    super(payload.message || 'Zerct command failed.')
    this.payload = payload
    this.json = json
    this.exitCode = exitCode
  }
}

export { agentError, printAgentError, ZerctError }
