import { ZerctError } from './errors.ts'
import { isJsonObject, parseJson, stringField } from './json.ts'
import type { AgentErrorPayload, CliOptions, JsonValue } from './types.ts'

interface AgentErrorContext {
  cli: CliOptions
  route: string
  token: string | null
}

const BILLING_CHECKOUT_ROUTE = '/v1/billing/checkout'

async function enrichAgentErrorPayload(
  context: AgentErrorContext,
  payload: AgentErrorPayload
): Promise<void> {
  if (!shouldCreateCheckoutUrl(context, payload)) {
    return
  }

  const checkoutUrl = await createCheckoutUrl(context.cli, context.token, payload.message)
  if (checkoutUrl) {
    payload.checkout_url = checkoutUrl
  }
}

async function paymentRequiredAgentError(
  cli: CliOptions,
  token: string,
  message: string,
  agentInstruction: string
): Promise<ZerctError> {
  const payload: AgentErrorPayload = {
    code: 'payment_required',
    message,
    agent_instruction: agentInstruction,
    docs_url: null,
    checkout_url: null
  }
  await enrichAgentErrorPayload({ cli, route: 'local:preflight', token }, payload)
  return new ZerctError(payload, cli.json, 1)
}

function shouldCreateCheckoutUrl(
  context: AgentErrorContext,
  payload: AgentErrorPayload
): boolean {
  return payload.code === 'payment_required'
    && !payload.checkout_url
    && Boolean(context.token)
    && context.route !== BILLING_CHECKOUT_ROUTE
}

async function createCheckoutUrl(
  cli: CliOptions,
  token: string | null,
  reason: string
): Promise<string> {
  if (!token) {
    return ''
  }

  try {
    const response = await fetch(`${cli.apiUrl}${BILLING_CHECKOUT_ROUTE}`, {
      body: JSON.stringify({
        reason: reason || 'Plan limit reached.',
        target_plan: 'pro'
      }),
      headers: new Headers({
        accept: 'application/json',
        authorization: `Bearer ${token}`,
        'content-type': 'application/json'
      }),
      method: 'POST'
    })
    if (!response.ok) {
      return ''
    }
    return checkoutUrlFromJson(parseJson(await response.text()))
  } catch {
    return ''
  }
}

function checkoutUrlFromJson(value: JsonValue | null): string {
  if (!isJsonObject(value)) {
    return ''
  }
  const checkoutValue = value['checkout'] ?? null
  const checkout = isJsonObject(checkoutValue) ? checkoutValue : {}
  return stringField(checkout, 'url')
}

export { enrichAgentErrorPayload, paymentRequiredAgentError }
