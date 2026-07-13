import init, { Simulator, catalog, compose_settings } from './wasm/vss_web.js'
import type { Catalog, Value } from './types'

export async function startSimulator(parent: string) {
  await init()
  return Simulator.create(parent)
}
export const getCatalog = (locale: string) => JSON.parse(catalog(locale)) as Catalog
export const compose = (locale: string, presets: string[], overrides: Record<string, Value>) => JSON.parse(compose_settings(locale, JSON.stringify(presets), JSON.stringify(overrides))) as Record<string, Value>
