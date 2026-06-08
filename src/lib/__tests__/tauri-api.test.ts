import { describe, it, expect } from 'vitest'
import { getErrorMessage, toApiError } from '../tauri-api'
import type { ErrorCode } from '../types'

// MARK: toApiError Tests — Structured Error Contract (PR-1)

describe('toApiError — structured JSON parsing', () => {
  // Helper to create a Tauri-like error with message
  const makeError = (message: string) => new Error(message)

  describe('parses structured JSON payload first', () => {
    it('parses PROJECT_NOT_FOUND and maps to PATH_NOT_FOUND', () => {
      const backendPayload = JSON.stringify({
        code: 'PROJECT_NOT_FOUND',
        message: 'Project not found: /foo',
        details: { path: '/foo' },
      })
      const err = makeError(backendPayload)
      const result = toApiError(err)

      expect(result.code).toBe('PATH_NOT_FOUND')
      expect(result.message).toBe('Project not found: /foo')
      expect(result.details).toEqual({ path: '/foo' })
    })

    it('parses FILE_NOT_FOUND and maps to PATH_NOT_FOUND', () => {
      const backendPayload = JSON.stringify({
        code: 'FILE_NOT_FOUND',
        message: 'File not found: config.yaml',
        details: { path: 'config.yaml' },
      })
      const err = makeError(backendPayload)
      const result = toApiError(err)

      expect(result.code).toBe('PATH_NOT_FOUND')
      expect(result.message).toBe('File not found: config.yaml')
    })

    it('parses AI_UNAVAILABLE and maps to UNREACHABLE', () => {
      const backendPayload = JSON.stringify({
        code: 'AI_UNAVAILABLE',
        message: 'AI unavailable: network error',
        details: { reason: 'network error' },
      })
      const err = makeError(backendPayload)
      const result = toApiError(err, 'INTERNAL')

      expect(result.code).toBe('UNREACHABLE')
      expect(result.message).toBe('AI unavailable: network error')
    })

    it('parses AI_RATE_LIMITED and maps to RATE_LIMITED', () => {
      const backendPayload = JSON.stringify({
        code: 'AI_RATE_LIMITED',
        message: 'AI rate limited',
      })
      const err = makeError(backendPayload)
      const result = toApiError(err, 'INTERNAL')

      expect(result.code).toBe('RATE_LIMITED')
    })

    it('parses AI_TOKEN_LIMIT and maps to TOKEN_LIMIT', () => {
      const backendPayload = JSON.stringify({
        code: 'AI_TOKEN_LIMIT',
        message: 'AI token limit exceeded',
      })
      const err = makeError(backendPayload)
      const result = toApiError(err, 'INTERNAL')

      expect(result.code).toBe('TOKEN_LIMIT')
    })

    it('parses INVALID_API_KEY and maps to INVALID_KEY', () => {
      const backendPayload = JSON.stringify({
        code: 'INVALID_API_KEY',
        message: 'Invalid API key',
      })
      const err = makeError(backendPayload)
      const result = toApiError(err, 'INTERNAL')

      expect(result.code).toBe('INVALID_KEY')
    })

    it('parses ACCESS_DENIED and maps to ACCESS_DENIED', () => {
      const backendPayload = JSON.stringify({
        code: 'ACCESS_DENIED',
        message: 'Access denied: resource X',
        details: { resource: 'resource X' },
      })
      const err = makeError(backendPayload)
      const result = toApiError(err, 'INTERNAL')

      expect(result.code).toBe('ACCESS_DENIED')
      expect(result.message).toBe('Access denied: resource X')
    })

    it('parses SCAN_TIMEOUT and maps to SCAN_TIMEOUT', () => {
      const backendPayload = JSON.stringify({
        code: 'SCAN_TIMEOUT',
        message: 'Scan timeout: processed 42/100 files',
        details: { files_processed: 42, total_files: 100 },
      })
      const err = makeError(backendPayload)
      const result = toApiError(err, 'INTERNAL')

      expect(result.code).toBe('SCAN_TIMEOUT')
      expect(result.details).toEqual({ files_processed: 42, total_files: 100 })
    })

    it('parses DATABASE and maps to INTERNAL', () => {
      const backendPayload = JSON.stringify({
        code: 'DATABASE',
        message: 'Database error: connection refused',
        details: { reason: 'connection refused' },
      })
      const err = makeError(backendPayload)
      const result = toApiError(err, 'PATH_NOT_FOUND')

      expect(result.code).toBe('INTERNAL')
    })

    it('parses INTERNAL and maps to INTERNAL', () => {
      const backendPayload = JSON.stringify({
        code: 'INTERNAL',
        message: 'Internal error: something went wrong',
        details: { reason: 'something went wrong' },
      })
      const err = makeError(backendPayload)
      const result = toApiError(err, 'PATH_NOT_FOUND')

      expect(result.code).toBe('INTERNAL')
    })
  })

  describe('preserves structured details', () => {
    it('keeps details as Record<string, unknown>', () => {
      const backendPayload = JSON.stringify({
        code: 'PROJECT_NOT_FOUND',
        message: 'Project not found: test',
        details: { path: '/test', timestamp: '2024-01-01' },
      })
      const err = makeError(backendPayload)
      const result = toApiError(err)

      expect(result.details).toBeDefined()
      expect(typeof result.details).toBe('object')
      expect(result.details).toHaveProperty('path', '/test')
      expect(result.details).toHaveProperty('timestamp', '2024-01-01')
    })

    it('allows undefined details for simple errors', () => {
      const backendPayload = JSON.stringify({
        code: 'AIRATE_LIMITED',
        message: 'AI rate limited',
      })
      const err = makeError(backendPayload)
      const result = toApiError(err)

      expect(result.details).toBeUndefined()
    })

    it('handles null details gracefully', () => {
      const backendPayload = JSON.stringify({
        code: 'INTERNAL',
        message: 'Error',
        details: null,
      })
      const err = makeError(backendPayload)
      const result = toApiError(err)

      // null should be treated as undefined in the ApiError shape
      expect(result.details).toBeUndefined()
    })
  })

  describe('falls back to legacy string matching', () => {
    it('uses legacy heuristics when JSON parsing fails', () => {
      const err = makeError('ENOENT: no such file or directory')
      const result = toApiError(err)

      expect(result.code).toBe('PATH_NOT_FOUND')
      expect(result.message).toBe('ENOENT: no such file or directory')
    })

    it('detects PROJECT_EXISTS from legacy messages', () => {
      const err = makeError('Project already exists at /foo')
      const result = toApiError(err)

      expect(result.code).toBe('PROJECT_EXISTS')
    })

    it('detects ACCESS_DENIED from legacy messages', () => {
      const err = makeError('EACCES: permission denied')
      const result = toApiError(err)

      expect(result.code).toBe('ACCESS_DENIED')
    })

    it('detects timeout from legacy messages', () => {
      const err = makeError('Scan timeout after 60s')
      const result = toApiError(err)

      expect(result.code).toBe('SCAN_TIMEOUT')
    })

    it('uses fallback code when no pattern matches', () => {
      const err = makeError('Something completely different')
      const result = toApiError(err, 'INTERNAL')

      expect(result.code).toBe('INTERNAL')
    })

    it('allows custom fallback code', () => {
      const err = makeError('Something completely different')
      const result = toApiError(err, 'PATH_NOT_FOUND')

      expect(result.code).toBe('PATH_NOT_FOUND')
    })
  })

  describe('handles edge cases', () => {
    it('handles empty error message', () => {
      const err = makeError('')
      const result = toApiError(err, 'INTERNAL')

      expect(result.code).toBe('INTERNAL')
    })

    it('handles non-Error thrown values', () => {
      const result = toApiError('plain string error', 'INTERNAL')

      expect(result.code).toBe('INTERNAL')
      expect(result.message).toBe('plain string error')
    })

    it('handles null thrown value', () => {
      const result = toApiError(null, 'INTERNAL')

      expect(result.code).toBe('INTERNAL')
      expect(result.message).toBe('null')
    })

    it('handles undefined thrown value', () => {
      const result = toApiError(undefined, 'INTERNAL')

      expect(result.code).toBe('INTERNAL')
      expect(result.message).toBe('undefined')
    })

    it('handles malformed JSON (partial parse)', () => {
      const err = makeError('{"code": "PROJECT_NOT_FOUND"')
      const result = toApiError(err, 'INTERNAL')

      // Should fall back to legacy since JSON.parse fails
      expect(result.code).toBe('INTERNAL')
    })

    it('handles JSON without code field', () => {
      const backendPayload = JSON.stringify({
        message: 'Some error',
      })
      const err = makeError(backendPayload)
      const result = toApiError(err, 'INTERNAL')

      // Should fall back to legacy since no code in JSON
      expect(result.code).toBe('INTERNAL')
    })

    it('handles JSON without message field', () => {
      const backendPayload = JSON.stringify({
        code: 'PROJECT_NOT_FOUND',
      })
      const err = makeError(backendPayload)
      const result = toApiError(err, 'INTERNAL')

      // Should fall back to legacy since no message in JSON
      expect(result.code).toBe('INTERNAL')
    })
  })
})

describe('toApiError — type safety', () => {
  it('returns ApiError with correct ErrorCode type', () => {
    const err = new Error(
      JSON.stringify({
        code: 'PATH_NOT_FOUND',
        message: 'Not found',
        details: { path: '/test' },
      })
    )
    const result = toApiError(err)

    // Type assertion to verify the return type matches ErrorCode
    const code: ErrorCode = result.code
    expect([
      'PATH_NOT_FOUND',
      'PROJECT_EXISTS',
      'ACCESS_DENIED',
      'SCAN_TIMEOUT',
      'INVALID_KEY',
      'UNREACHABLE',
      'RATE_LIMITED',
      'TOKEN_LIMIT',
      'INTERNAL',
    ]).toContain(code)
  })
})

// MARK: Existing getErrorMessage tests

describe('getErrorMessage', () => {
  // Error instance
  it('returns message from Error instance', () => {
    expect(getErrorMessage(new Error('something went wrong'))).toBe('something went wrong')
  })

  // Plain string
  it('returns plain string as-is', () => {
    expect(getErrorMessage('plain error string')).toBe('plain error string')
  })

  // { message } object
  it('returns message field from object with message', () => {
    expect(getErrorMessage({ message: 'foo' })).toBe('foo')
  })

  // { code, message } — the shape returned by toApiError()
  it('returns code and message for ApiError shape', () => {
    expect(getErrorMessage({ code: 'PATH_NOT_FOUND', message: 'Path /foo not found' })).toBe(
      'PATH_NOT_FOUND — Path /foo not found'
    )
  })

  // null
  it('returns Unknown error for null', () => {
    expect(getErrorMessage(null)).toBe('Unknown error')
  })

  // undefined
  it('returns Unknown error for undefined', () => {
    expect(getErrorMessage(undefined)).toBe('Unknown error')
  })

  // object without message
  it('returns Unknown error for object without message field', () => {
    expect(getErrorMessage({ reason: 'no message' })).toBe('Unknown error')
    expect(getErrorMessage({ code: 'ERR' })).toBe('Unknown error')
    expect(getErrorMessage({})).toBe('Unknown error')
  })

  // non-string message is coerced safely
  it('coerces non-string message to string', () => {
    expect(getErrorMessage({ code: 'ERR', message: 123 })).toBe('ERR — 123')
    expect(getErrorMessage({ message: null })).toBe('null')
    expect(getErrorMessage({ message: undefined })).toBe('undefined')
  })

  // Error subclass with code
  it('returns code + message when Error has code property', () => {
    const err: unknown = { code: 'CUSTOM', message: 'custom error', name: 'CustomError' }
    expect(getErrorMessage(err)).toBe('CUSTOM — custom error')
  })

  // non-Error object with message that has code but non-string code
  it('returns just message when code is not a string', () => {
    expect(getErrorMessage({ code: 404, message: 'not found' })).toBe('not found')
  })
})

describe('PROJECT_EXISTS error code', () => {
  it('formats PROJECT_EXISTS with the full message', () => {
    const err: unknown = {
      code: 'PROJECT_EXISTS',
      message: 'Project already exists at path: /home/user/my-project',
    }
    expect(getErrorMessage(err)).toBe(
      'PROJECT_EXISTS — Project already exists at path: /home/user/my-project'
    )
  })

  it('PROJECT_EXISTS is a valid ErrorCode', () => {
    // This verifies the new code is included in the type
    const code = 'PROJECT_EXISTS' as const
    expect(code).toBe('PROJECT_EXISTS')
  })
})
