// Spanish error messages for frontend ErrorCode values.
//
// This module is the i18n boundary for user-facing error strings.
// Long-term direction: presentation-owned user-facing error messaging.

import type { ErrorCode } from '../../lib/types'

/**
 * Maps ErrorCode values to Spanish user-facing messages.
 * Coverage must include every code in the frontend ErrorCode union.
 *
 * Special cases:
 * - UNREACHABLE_AI_NOT_CONFIGURED: used by toUserMessage when UNREACHABLE has details.reason === "AI not configured"
 */
export const esErrorMessages: Record<ErrorCode, string> = {
  INVALID_KEY: 'La clave de API no es válida. Verificá que esté correcta y no haya expirado.',
  RATE_LIMITED: 'Se excedió el límite de solicitudes. Esperá un momento antes de intentar de nuevo.',
  TOKEN_LIMIT: 'El contexto es demasiado largo para el modelo. Intentá con un nodo diferente.',
  UNREACHABLE: 'No se pudo conectar al proveedor de IA. Verificá tu conexión a internet.',
  PATH_NOT_FOUND: 'No se encontró el archivo o proyecto solicitado.',
  PROJECT_EXISTS: 'Ya existe un proyecto en esta ubicación.',
  ACCESS_DENIED: 'No tenés permisos para acceder a este recurso.',
  SCAN_TIMEOUT: 'El escaneo tardó demasiado y fue cancelado. Intentá con un proyecto más pequeño.',
  INTERNAL: 'Ocurrió un error inesperado. Intentá de nuevo.',
}

/**
 * Special-case message for AI not configured (UNREACHABLE + details.reason === "AI not configured").
 * This is looked up by toUserMessage in errors.ts, not via the standard ErrorCode map.
 */
export const UNREACHABLE_AI_NOT_CONFIGURED = 'La IA no está configurada. Agregá tu clave de API para continuar.'