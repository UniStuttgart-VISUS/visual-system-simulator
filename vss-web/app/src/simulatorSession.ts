import type { Catalog, Demonstration, Value } from './types'

export interface SimulatorSession {
  selectedDemonstrations: Record<string, string>
  manual: Record<string, Value>
  maskedFallback: Record<string, Value>
}

export function activePresets(session: SimulatorSession, catalog: Catalog): string[] {
  return [...new Set(catalog.articles.flatMap(article =>
    article.demonstrations.find(item => item.id === session.selectedDemonstrations[article.id])?.presets ?? [],
  ))]
}

export function selectDemonstration(session: SimulatorSession, catalog: Catalog, articleId: string, demonstrationId: string): SimulatorSession {
  const article = catalog.articles.find(item => item.id === articleId)
  const selected = article?.demonstrations.find(item => item.id === demonstrationId)
  if (!article || !selected) return session

  const before = activePresets(session, catalog)
  const selections = { ...session.selectedDemonstrations }
  if (selections[articleId] === demonstrationId) {
    delete selections[articleId]
  } else {
    for (const other of catalog.articles) {
      const demonstration = other.demonstrations.find(item => item.id === selections[other.id])
      if (other.id !== articleId && demonstration && demonstrationsConflict(catalog, selected, demonstration)) delete selections[other.id]
    }
    selections[articleId] = demonstrationId
  }

  return cascadePresetTransition(catalog, before, activePresets({ ...session, selectedDemonstrations: selections }, catalog), {
    ...session,
    selectedDemonstrations: selections,
  })
}

export function editSetting(session: SimulatorSession, settingId: string, value: Value): SimulatorSession {
  return { ...session, manual: { ...session.manual, [settingId]: value } }
}

export function resetSetting(session: SimulatorSession, settingId: string): SimulatorSession {
  const manual = { ...session.manual }
  delete manual[settingId]
  return { ...session, manual }
}

export function sourceArticle(session: SimulatorSession, catalog: Catalog, settingId: string): string | undefined {
  if (settingId in session.manual) return undefined
  const active = new Set(activePresets(session, catalog))
  const preset = catalog.presets.find(item => active.has(item.id) && settingId in item.values)
  return preset && catalog.articles.find(article => article.demonstrations.some(item => item.presets.includes(preset.id)))?.id
}

function demonstrationsConflict(catalog: Catalog, selected: Demonstration, other: Demonstration): boolean {
  const values = Object.fromEntries(selected.presets.flatMap(id => Object.entries(catalog.presets.find(item => item.id === id)?.values ?? {})))
  return other.presets.flatMap(id => Object.entries(catalog.presets.find(item => item.id === id)?.values ?? {}))
    .some(([setting, value]) => setting in values && !valuesEqual(values[setting], value))
}

function valuesEqual(left: Value, right: Value): boolean {
  return Array.isArray(left) && Array.isArray(right)
    ? left[0] === right[0] && left[1] === right[1]
    : left === right
}

function cascadePresetTransition(catalog: Catalog, before: string[], after: string[], session: SimulatorSession): SimulatorSession {
  const settings = (presets: string[]) => new Set(presets.flatMap(id => Object.keys(catalog.presets.find(item => item.id === id)?.values ?? {})))
  const beforeSettings = settings(before)
  const afterSettings = settings(after)
  const manual = { ...session.manual }
  const maskedFallback = { ...session.maskedFallback }

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
  return { ...session, manual, maskedFallback }
}
