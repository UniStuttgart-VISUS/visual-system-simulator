import { describe, expect, test } from 'vitest'
import { activePresets, editSetting, resetSetting, selectDemonstration, sourceArticle, type SimulatorSession } from './simulatorSession'
import type { Catalog } from './types'

const catalog: Catalog = {
  groups: [],
  presets: [
    { id: 'near', label: 'Near', values: { focus: 1, glare: 0.5 } },
    { id: 'far', label: 'Far', values: { focus: 2 } },
    { id: 'same-glare', label: 'Same glare', values: { glare: 0.5 } },
    { id: 'different-focus', label: 'Different focus', values: { focus: 3 } },
  ],
  articles: [
    { id: 'vision', title: 'Vision', content_path: 'vision.html', demonstrations: [
      { id: 'near-demo', label: 'Near', presets: ['near'] },
      { id: 'far-demo', label: 'Far', presets: ['far'] },
    ] },
    { id: 'glare', title: 'Glare', content_path: 'glare.html', demonstrations: [
      { id: 'glare-demo', label: 'Glare', presets: ['same-glare'] },
    ] },
    { id: 'condition', title: 'Condition', content_path: 'condition.html', demonstrations: [
      { id: 'condition-demo', label: 'Condition', presets: ['different-focus'] },
    ] },
  ],
}

const empty: SimulatorSession = { selectedDemonstrations: {}, manual: {}, maskedFallback: {} }

describe('simulator session', () => {
  test('replaces same-article and conflicting demonstrations while combining compatible ones', () => {
    const near = selectDemonstration(empty, catalog, 'vision', 'near-demo')
    const combined = selectDemonstration(near, catalog, 'glare', 'glare-demo')
    expect(activePresets(combined, catalog)).toEqual(['near', 'same-glare'])

    const far = selectDemonstration(combined, catalog, 'vision', 'far-demo')
    expect(far.selectedDemonstrations).toEqual({ vision: 'far-demo', glare: 'glare-demo' })
    expect(activePresets(far, catalog)).toEqual(['far', 'same-glare'])

    const disabled = selectDemonstration(far, catalog, 'vision', 'far-demo')
    expect(activePresets(disabled, catalog)).toEqual(['same-glare'])

    const conflicting = selectDemonstration(near, catalog, 'condition', 'condition-demo')
    expect(conflicting.selectedDemonstrations).toEqual({ condition: 'condition-demo' })
    expect(activePresets(conflicting, catalog)).toEqual(['different-focus'])
  })

  test('restores a manual value after the last relevant preset is disabled', () => {
    const manual = editSetting(empty, 'focus', 9)
    const active = selectDemonstration(manual, catalog, 'vision', 'near-demo')
    expect(active.manual).toEqual({})
    expect(active.maskedFallback).toEqual({ focus: 9 })

    const disabled = selectDemonstration(active, catalog, 'vision', 'near-demo')
    expect(disabled.manual).toEqual({ focus: 9 })
    expect(disabled.maskedFallback).toEqual({})
  })

  test('keeps the newest manual value and reset reveals the active preset', () => {
    const before = editSetting(empty, 'focus', 9)
    const active = selectDemonstration(before, catalog, 'vision', 'near-demo')
    const newer = editSetting(active, 'focus', 7)
    const reset = resetSetting(newer, 'focus')
    expect(sourceArticle(reset, catalog, 'focus')).toBe('vision')

    const disabled = selectDemonstration(newer, catalog, 'vision', 'near-demo')
    expect(disabled.manual).toEqual({ focus: 7 })
    expect(disabled.maskedFallback).toEqual({})
  })

  test('switching presets promotes a newer manual value to the fallback', () => {
    const near = selectDemonstration(editSetting(empty, 'focus', 9), catalog, 'vision', 'near-demo')
    const edited = editSetting(near, 'focus', 7)
    const far = selectDemonstration(edited, catalog, 'vision', 'far-demo')
    expect(far.manual).toEqual({})
    expect(far.maskedFallback).toEqual({ focus: 7 })

    const disabled = selectDemonstration(far, catalog, 'vision', 'far-demo')
    expect(disabled.manual).toEqual({ focus: 7 })
  })
})
