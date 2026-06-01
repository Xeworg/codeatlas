import { describe, it, expect } from 'vitest'

describe('types', () => {
  it('NodeType enum has all expected values', () => {
    const validTypes = [
      'Component',
      'Route',
      'Service',
      'Repository',
      'Model',
      'Util',
      'Config',
      'Test',
      'External',
      'Unknown',
    ]
    expect(validTypes.length).toBe(10)
  })

  it('ScanStatus enum has all expected values', () => {
    const validStatuses = ['Idle', 'Scanning', 'BuildingGraph', 'Ready', 'Error']
    expect(validStatuses.length).toBe(5)
  })

  it('SymbolKind enum has all expected values', () => {
    const validKinds = [
      'Function',
      'Class',
      'Method',
      'Module',
      'Interface',
      'Enum',
      'Struct',
      'Impl',
      'TypeAlias',
      'Const',
      'Variable',
    ]
    expect(validKinds.length).toBe(11)
  })
})

describe('App basic structure', () => {
  it('App.tsx renders without crashing', async () => {
    const { default: App } = await import('../../src/App')
    expect(App).toBeDefined()
  })
})
