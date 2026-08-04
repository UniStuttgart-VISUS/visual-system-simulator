import type { Catalog, Demonstration, Preset, Value } from './types'

export type EyeMode = 'left' | 'both' | 'right'
type Eye = Exclude<EyeMode, 'both'>

export interface SessionLayer {
  selectedDemonstrations: Record<string, string>
  manual: Record<string, Value>
  maskedFallback: Record<string, Value>
  maskedByBoth: Record<string, true>
}

export interface SimulatorSession {
  eyeMode: EyeMode
  layers: Record<EyeMode, SessionLayer>
}

const emptyLayer = (): SessionLayer => ({
  selectedDemonstrations: {},
  manual: {},
  maskedFallback: {},
  maskedByBoth: {},
})

export function createSession(hasEyeSpecificContent = false): SimulatorSession {
  return {
    eyeMode: hasEyeSpecificContent ? 'both' : 'left',
    layers: { left: emptyLayer(), both: emptyLayer(), right: emptyLayer() },
  }
}

export function setEyeMode(session: SimulatorSession, eyeMode: EyeMode): SimulatorSession {
  return { ...session, eyeMode }
}

export function currentLayer(session: SimulatorSession): SessionLayer {
  return session.layers[session.eyeMode]
}

const preset = (catalog: Catalog, id: string) => catalog.presets.find(item => item.id === id)
const intrinsic = (value: Preset) => Object.keys(value.left).length > 0 || Object.keys(value.right).length > 0

function presetValues(value: Preset, target: EyeMode): Record<string, Value> {
  if (Object.keys(value.both).length > 0) return value.both
  return target === 'both' ? {} : value[target]
}

function layerPresets(layer: SessionLayer, catalog: Catalog): string[] {
  return [...new Set(catalog.articles.flatMap(article =>
    article.demonstrations.find(item => item.id === layer.selectedDemonstrations[article.id])?.presets ?? [],
  ))]
}

export function activePresets(session: SimulatorSession, catalog: Catalog): string[] {
  if (session.eyeMode !== 'both') return layerPresets(session.layers[session.eyeMode], catalog)
  const shared = layerPresets(session.layers.both, catalog)
  const authored = [...layerPresets(session.layers.left, catalog), ...layerPresets(session.layers.right, catalog)]
    .filter(id => preset(catalog, id) && intrinsic(preset(catalog, id)!))
  return [...new Set([...shared, ...authored])]
}

export function selectedDemonstration(session: SimulatorSession, articleId: string): string | undefined {
  if (session.eyeMode !== 'both') return session.layers[session.eyeMode].selectedDemonstrations[articleId]
  return session.layers.both.selectedDemonstrations[articleId]
    ?? session.layers.left.selectedDemonstrations[articleId]
    ?? session.layers.right.selectedDemonstrations[articleId]
}

function demonstrationValues(catalog: Catalog, demonstration: Demonstration, target: EyeMode) {
  return Object.fromEntries(demonstration.presets.flatMap(id => Object.entries(
    preset(catalog, id) ? presetValues(preset(catalog, id)!, target) : {},
  )))
}

function demonstrationsConflict(catalog: Catalog, selected: Demonstration, other: Demonstration, target: EyeMode): boolean {
  const values = demonstrationValues(catalog, selected, target)
  return Object.entries(demonstrationValues(catalog, other, target))
    .some(([setting, value]) => setting in values && !valuesEqual(values[setting], value))
}

function valuesEqual(left: Value, right: Value): boolean {
  return Array.isArray(left) && Array.isArray(right)
    ? left[0] === right[0] && left[1] === right[1]
    : left === right
}

function transitionLayer(
  layer: SessionLayer,
  catalog: Catalog,
  target: EyeMode,
  articleId: string,
  selected: Demonstration,
  enabled: boolean,
): SessionLayer {
  const before = layerPresets(layer, catalog)
  const selections = { ...layer.selectedDemonstrations }
  if (!enabled) {
    delete selections[articleId]
  } else {
    for (const article of catalog.articles) {
      const other = article.demonstrations.find(item => item.id === selections[article.id])
      if (article.id !== articleId && other && demonstrationsConflict(catalog, selected, other, target)) {
        delete selections[article.id]
      }
    }
    selections[articleId] = selected.id
  }
  const next = { ...layer, selectedDemonstrations: selections }
  return cascadePresetTransition(catalog, target, before, layerPresets(next, catalog), next)
}

function settingsForPresets(catalog: Catalog, target: EyeMode, presets: string[]) {
  return new Set(presets.flatMap(id => {
    const value = preset(catalog, id)
    return value ? Object.keys(presetValues(value, target)) : []
  }))
}

function cascadePresetTransition(
  catalog: Catalog,
  target: EyeMode,
  before: string[],
  after: string[],
  layer: SessionLayer,
): SessionLayer {
  const beforeSettings = settingsForPresets(catalog, target, before)
  const afterSettings = settingsForPresets(catalog, target, after)
  const manual = { ...layer.manual }
  const maskedFallback = { ...layer.maskedFallback }
  for (const setting of new Set([...beforeSettings, ...afterSettings])) {
    if (afterSettings.has(setting)) {
      if (setting in manual) {
        maskedFallback[setting] = manual[setting]
        delete manual[setting]
      }
    } else if (beforeSettings.has(setting)) {
      if (setting in manual) delete maskedFallback[setting]
      else if (setting in maskedFallback) {
        manual[setting] = maskedFallback[setting]
        delete maskedFallback[setting]
      }
    }
  }
  return { ...layer, manual, maskedFallback }
}

function updateLayer(session: SimulatorSession, target: EyeMode, layer: SessionLayer): SimulatorSession {
  return { ...session, layers: { ...session.layers, [target]: layer } }
}

function maskSettings(session: SimulatorSession, targets: Eye[], settings: Iterable<string>, masked: boolean) {
  let next = session
  for (const target of targets) {
    const layer = next.layers[target]
    const maskedByBoth = { ...layer.maskedByBoth }
    for (const setting of settings) {
      if (masked) maskedByBoth[setting] = true
      else delete maskedByBoth[setting]
    }
    next = updateLayer(next, target, { ...layer, maskedByBoth })
  }
  return next
}

function sharedSettings(session: SimulatorSession, catalog: Catalog) {
  return new Set([
    ...Object.keys(session.layers.both.manual),
    ...settingsForPresets(catalog, 'both', layerPresets(session.layers.both, catalog)),
  ])
}

export function selectDemonstration(
  session: SimulatorSession,
  catalog: Catalog,
  articleId: string,
  demonstrationId: string,
): SimulatorSession {
  const article = catalog.articles.find(item => item.id === articleId)
  const selected = article?.demonstrations.find(item => item.id === demonstrationId)
  if (!article || !selected) return session
  const presets = selected.presets.map(id => preset(catalog, id)).filter((item): item is Preset => Boolean(item))
  const isIntrinsic = presets.some(intrinsic)
  const targets: EyeMode[] = isIntrinsic
    ? (['left', 'right'] as Eye[]).filter(target => presets.some(item => Object.keys(item[target]).length > 0))
    : [session.eyeMode]
  const enabled = !targets.every(target => session.layers[target].selectedDemonstrations[articleId] === demonstrationId)
  let next = session
  for (const target of targets) {
    next = updateLayer(next, target, transitionLayer(next.layers[target], catalog, target, articleId, selected, enabled))
    if (target !== 'both' && enabled) {
      next = maskSettings(next, [target], Object.keys(demonstrationValues(catalog, selected, target)), false)
    }
  }
  if (targets.includes('both')) {
    const changed = Object.keys(demonstrationValues(catalog, selected, 'both'))
    if (enabled) next = maskSettings(next, ['left', 'right'], changed, true)
    else {
      const owned = sharedSettings(next, catalog)
      next = maskSettings(next, ['left', 'right'], changed.filter(setting => !owned.has(setting)), false)
    }
  }
  return isIntrinsic && enabled ? { ...next, eyeMode: 'both' } : next
}

export function editSetting(session: SimulatorSession, settingId: string, value: Value): SimulatorSession {
  const target = session.eyeMode
  const layer = session.layers[target]
  let next = updateLayer(session, target, { ...layer, manual: { ...layer.manual, [settingId]: value } })
  return target === 'both'
    ? maskSettings(next, ['left', 'right'], [settingId], true)
    : maskSettings(next, [target], [settingId], false)
}

export function resetSetting(session: SimulatorSession, settingId: string, catalog?: Catalog): SimulatorSession {
  const target = session.eyeMode
  const layer = session.layers[target]
  const manual = { ...layer.manual }
  delete manual[settingId]
  let next = updateLayer(session, target, { ...layer, manual })
  if (target === 'both' && (!catalog || !sharedSettings(next, catalog).has(settingId))) {
    next = maskSettings(next, ['left', 'right'], [settingId], false)
  }
  return next
}

function applyLayer(result: Record<string, Value>, layer: SessionLayer, catalog: Catalog, target: EyeMode) {
  for (const item of catalog.presets) {
    if (!layerPresets(layer, catalog).includes(item.id)) continue
    for (const [setting, value] of Object.entries(presetValues(item, target))) {
      if (!(setting in layer.maskedByBoth)) result[setting] = value
    }
  }
  for (const [setting, value] of Object.entries(layer.manual)) {
    if (!(setting in layer.maskedByBoth)) result[setting] = value
  }
}

export function effectiveValues(session: SimulatorSession, catalog: Catalog, eye: Eye): Record<string, Value> {
  const result = Object.fromEntries(catalog.groups.flatMap(group => group.settings.map(setting => [setting.id, setting.default])))
  applyLayer(result, session.layers.both, catalog, 'both')
  applyLayer(result, session.layers[eye], catalog, eye)
  return result
}

export function editableValues(session: SimulatorSession, catalog: Catalog): Record<string, Value> {
  if (session.eyeMode !== 'both') return effectiveValues(session, catalog, session.eyeMode)
  const result: Record<string, Value> = Object.fromEntries(
    catalog.groups.flatMap(group => group.settings.map(setting => [setting.id, setting.default])),
  )
  applyLayer(result, session.layers.both, catalog, 'both')
  return result
}

export function sourceArticle(session: SimulatorSession, catalog: Catalog, settingId: string): string | undefined {
  const target = session.eyeMode
  const layer = session.layers[target]
  if (settingId in layer.manual) return undefined
  const active = new Set(layerPresets(layer, catalog))
  const owner = catalog.presets.find(item => active.has(item.id) && settingId in presetValues(item, target))
  return owner && catalog.articles.find(article =>
    article.demonstrations.some(item => item.presets.includes(owner.id)),
  )?.id
}
