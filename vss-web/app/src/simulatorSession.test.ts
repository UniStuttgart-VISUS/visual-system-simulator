import { describe, expect, test } from 'vitest'
import { activePresets, createSession, editSetting, effectiveValues, resetSetting, selectDemonstration, setEyeMode, sourceArticle, type SimulatorSession } from './simulatorSession'
import type { Catalog } from './types'

const catalog: Catalog = {
  groups: [],
  presets: [
    { id: 'near', label: 'Near', both: { focus: 1, glare: 0.5 }, left: {}, right: {} },
    { id: 'far', label: 'Far', both: { focus: 2 }, left: {}, right: {} },
    { id: 'same-glare', label: 'Same glare', both: { glare: 0.5 }, left: {}, right: {} },
    { id: 'different-focus', label: 'Different focus', both: { focus: 3 }, left: {}, right: {} },
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

const empty: SimulatorSession = setEyeMode(createSession(), 'both')

const eyeCatalog: Catalog = {
  groups: [],
  presets: [
    { id: 'strabismus', label: 'Strabismus', both: {}, left: { axis: 0.05 }, right: { axis: -0.05 } },
    { id: 'cataract', label: 'Cataract', both: { blur: 25 }, left: {}, right: {} },
  ],
  articles: [
    { id: 'strabismus', title: 'Strabismus', content_path: '', demonstrations: [
      { id: 'strabismus', label: 'Strabismus', presets: ['strabismus'] },
    ] },
    { id: 'cataract', title: 'Cataract', content_path: '', demonstrations: [
      { id: 'cataract', label: 'Cataract', presets: ['cataract'] },
    ] },
  ],
}

describe('simulator session', () => {
  test('intrinsic preset selects both and later symmetric preset targets the visible left eye', () => {
    let session = createSession()
    expect(session.eyeMode).toBe('left')

    session = selectDemonstration(session, eyeCatalog, 'strabismus', 'strabismus')
    expect(session.eyeMode).toBe('both')
    expect(effectiveValues(session, eyeCatalog, 'left')).toMatchObject({ axis: 0.05 })
    expect(effectiveValues(session, eyeCatalog, 'right')).toMatchObject({ axis: -0.05 })

    session = setEyeMode(session, 'left')
    session = selectDemonstration(session, eyeCatalog, 'cataract', 'cataract')
    expect(session.eyeMode).toBe('left')
    expect(effectiveValues(session, eyeCatalog, 'left')).toMatchObject({ axis: 0.05, blur: 25 })
    expect(effectiveValues(session, eyeCatalog, 'right')).toMatchObject({ axis: -0.05 })
    expect(effectiveValues(session, eyeCatalog, 'right')).not.toHaveProperty('blur')
  })

  test('both edit masks only the matching eye values and reset reveals them', () => {
    let session = setEyeMode(createSession(), 'left')
    session = editSetting(session, 'blur', 20)
    session = setEyeMode(session, 'right')
    session = editSetting(session, 'blur', 30)
    session = editSetting(session, 'axis', -0.05)

    session = setEyeMode(session, 'both')
    session = editSetting(session, 'blur', 10)
    expect(effectiveValues(session, eyeCatalog, 'left')).toMatchObject({ blur: 10 })
    expect(effectiveValues(session, eyeCatalog, 'right')).toMatchObject({ blur: 10, axis: -0.05 })

    session = resetSetting(session, 'blur', eyeCatalog)
    expect(effectiveValues(session, eyeCatalog, 'left')).toMatchObject({ blur: 20 })
    expect(effectiveValues(session, eyeCatalog, 'right')).toMatchObject({ blur: 30, axis: -0.05 })
  })

  test('replaces same-article and conflicting demonstrations while combining compatible ones', () => {
    const near = selectDemonstration(empty, catalog, 'vision', 'near-demo')
    const combined = selectDemonstration(near, catalog, 'glare', 'glare-demo')
    expect(activePresets(combined, catalog)).toEqual(['near', 'same-glare'])

    const far = selectDemonstration(combined, catalog, 'vision', 'far-demo')
    expect(far.layers.both.selectedDemonstrations).toEqual({ vision: 'far-demo', glare: 'glare-demo' })
    expect(activePresets(far, catalog)).toEqual(['far', 'same-glare'])

    const disabled = selectDemonstration(far, catalog, 'vision', 'far-demo')
    expect(activePresets(disabled, catalog)).toEqual(['same-glare'])

    const conflicting = selectDemonstration(near, catalog, 'condition', 'condition-demo')
    expect(conflicting.layers.both.selectedDemonstrations).toEqual({ condition: 'condition-demo' })
    expect(activePresets(conflicting, catalog)).toEqual(['different-focus'])
  })

  test('restores a manual value after the last relevant preset is disabled', () => {
    const manual = editSetting(empty, 'focus', 9)
    const active = selectDemonstration(manual, catalog, 'vision', 'near-demo')
    expect(active.layers.both.manual).toEqual({})
    expect(active.layers.both.maskedFallback).toEqual({ focus: 9 })

    const disabled = selectDemonstration(active, catalog, 'vision', 'near-demo')
    expect(disabled.layers.both.manual).toEqual({ focus: 9 })
    expect(disabled.layers.both.maskedFallback).toEqual({})
  })

  test('keeps the newest manual value and reset reveals the active preset', () => {
    const before = editSetting(empty, 'focus', 9)
    const active = selectDemonstration(before, catalog, 'vision', 'near-demo')
    const newer = editSetting(active, 'focus', 7)
    const reset = resetSetting(newer, 'focus', catalog)
    expect(sourceArticle(reset, catalog, 'focus')).toBe('vision')

    const disabled = selectDemonstration(newer, catalog, 'vision', 'near-demo')
    expect(disabled.layers.both.manual).toEqual({ focus: 7 })
    expect(disabled.layers.both.maskedFallback).toEqual({})
  })

  test('switching presets promotes a newer manual value to the fallback', () => {
    const near = selectDemonstration(editSetting(empty, 'focus', 9), catalog, 'vision', 'near-demo')
    const edited = editSetting(near, 'focus', 7)
    const far = selectDemonstration(edited, catalog, 'vision', 'far-demo')
    expect(far.layers.both.manual).toEqual({})
    expect(far.layers.both.maskedFallback).toEqual({ focus: 7 })

    const disabled = selectDemonstration(far, catalog, 'vision', 'far-demo')
    expect(disabled.layers.both.manual).toEqual({ focus: 7 })
  })
})
