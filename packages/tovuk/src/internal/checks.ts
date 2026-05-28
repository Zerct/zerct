import type { DoctorCheck } from './types.ts'

function doctorCheck(name: string, ok: boolean, success: string, failure: string, instruction: string): DoctorCheck {
  return {
    name,
    ok,
    message: ok ? success : failure,
    agent_instruction: ok ? null : instruction
  }
}

export { doctorCheck }
