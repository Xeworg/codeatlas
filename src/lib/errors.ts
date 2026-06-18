// Error helpers — user-facing message translation
//
// Canonical home for frontend error message logic.
// The backend emits structured IPC errors; these helpers translate
// ErrorCode values to localized user messages.

import type { ErrorCode } from './types'
import { esErrorMessages } from '../locales/es/errors'

/**
 * Translate an error code to a user-friendly Spanish message.
 *
 * Used by hooks and components to display localized, human-readable
 * error messages instead of raw backend messages or error codes.
 */
export function toUserMessage(apiError: { code: ErrorCode; message?: string }): string {
  // INTERNAL errors preserve the backend message as fallback.
  if (apiError.code === 'INTERNAL' && apiError.message) {
    return apiError.message
  }
  const msg = esErrorMessages[apiError.code]
  if (msg) return msg
  return apiError.message || 'Ocurrió un error inesperado. Intentá de nuevo.'
}

/**
 * Extract a human-readable message from any thrown value.
 * Handles Error instances, plain objects, strings, and null/undefined.
 * Preserves `{ code, message }` shape for downstream consumers.
 */
export function getErrorMessage(err: unknown): string {
  if (err === null || err === undefined) return 'Unknown error'
  if (typeof err === 'string') return err
  if (typeof err === 'object' && 'message' in err) {
    const msg = String((err as Record<string, unknown>).message)
    if ('code' in err && typeof (err as Record<string, unknown>).code === 'string') {
      return `${(err as Record<string, unknown>).code} — ${msg}`
    }
    return msg
  }
  return 'Unknown error'
}