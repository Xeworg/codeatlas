import { describe, it, expect } from 'vitest'
import { getErrorMessage } from '../tauri-api'

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
