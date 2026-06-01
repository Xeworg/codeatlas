/**
 * i18n — T6.6 Tests
 * Strict TDD RED phase: tests written before implementation.
 */

import { describe, it, expect } from 'vitest'

// T6.6 RED: These tests define expected behavior for the i18n foundation.
// They will FAIL until t() helper and es.json are implemented.

describe('i18n t() helper', () => {
  it('returns literal key when key does not exist in catalog', async () => {
    // T6.6: t('nonexistent.key') → returns literal key + dev warning
    const { t } = await import('../../src/lib/i18n')
    expect(t('nonexistent.key')).toBe('nonexistent.key')
  })

  it('returns string from es.json catalog for valid key', async () => {
    // T6.6: t('common.loading') → returns string from catalog
    const { t } = await import('../../src/lib/i18n')
    const result = t('common.loading')
    expect(typeof result).toBe('string')
    expect(result.length).toBeGreaterThan(0)
    expect(result).not.toBe('common.loading')
  })

  it('supports dot-notation key resolution', async () => {
    // T6.6: Resolves from es.json with dot notation
    const { t } = await import('../../src/lib/i18n')
    const result = t('architecture.title')
    expect(typeof result).toBe('string')
    expect(result.length).toBeGreaterThan(0)
  })

  it('supports variable substitution', async () => {
    // T6.2: t(key, { count: 5 }) → substitutes {{count}}
    const { t } = await import('../../src/lib/i18n')
    const result = t('impact.affectedFiles', { count: '5' })
    expect(typeof result).toBe('string')
    // Should contain the substituted value
    expect(result).toContain('5')
  })

  it('returns catalog string with variables replaced', async () => {
    const { t } = await import('../../src/lib/i18n')
    const result = t('common.count', { count: '99' })
    expect(result).toContain('99')
    expect(result).not.toBe('common.count')
  })
})
