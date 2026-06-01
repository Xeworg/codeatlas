/**
 * i18n — T6.2: Spanish catalog helper
 *
 * - Runtime language fixed to Spanish (v2 scope).
 * - No language selector in v2.
 * - Fallback: if key missing, returns literal key + warning in dev mode.
 */

import catalog from '../locales/es.json'

type Catalog = typeof catalog

function getNestedValue(obj: unknown, path: string): unknown {
  return path.split('.').reduce<unknown>((acc, part) => {
    if (acc && typeof acc === 'object' && part in acc) {
      return (acc as Record<string, unknown>)[part]
    }
    return undefined
  }, obj)
}

export function t(key: string, vars?: Record<string, string>): string {
  const value = getNestedValue(catalog as unknown as Catalog, key)

  if (value === undefined || typeof value !== 'string') {
    if (import.meta.env?.DEV) {
      console.warn(`[i18n] Missing key: "${key}" — returning literal key`)
    }
    return key
  }

  if (!vars) return value

  // Substitute {{var}} placeholders
  return value.replace(/\{\{(\w+)\}\}/g, (_, varName) => {
    return vars[varName] ?? `{{${varName}}}`
  })
}
