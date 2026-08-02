import { fireEvent, render, waitFor } from '@testing-library/vue'
import { createI18n } from 'vue-i18n'
import CatalogPanel from './CatalogPanel.vue'
import { messages } from '../i18n'
import type { SimulatorSession } from '../simulatorSession'
import type { Catalog } from '../types'

const catalog: Catalog = {
  groups: [{ id: 'g', title: 'Group', settings: [{ id: 'g.on', label: 'Enabled', help: '', default: false, control: { kind: 'boolean' } }] }],
  presets: [{ id: 'p', label: 'Preset', values: { 'g.on': true } }],
  articles: [{ id: 'a', title: 'Article', summary: 'A useful summary.', content_path: 'a.html', demonstrations: [{ id: 'd', label: 'Try it', presets: ['p'] }] }],
}
const empty: SimulatorSession = { selectedDemonstrations: {}, manual: {}, maskedFallback: {} }
const global = { plugins: [createI18n({ legacy: false, locale: 'en', messages })] }

test('selects a card demonstration directly without a decision dialog', async () => {
  const view = render(CatalogPanel, { props: { catalog, session: empty, effective: { 'g.on': false } }, global })
  await fireEvent.click(view.getByRole('button', { name: 'Try it' }))
  expect(view.emitted().demonstration?.[0]).toEqual(['a', 'd'])
  expect(view.queryByText('Add presets')).toBeNull()
})

test('moves discretely between cards from indicators and desktop arrows', async () => {
  const second = { id: 'b', title: 'Second article', content_path: 'b.html', demonstrations: [] }
  const view = render(CatalogPanel, { props: { catalog: { ...catalog, articles: [...catalog.articles, second] }, session: empty, effective: { 'g.on': false } }, global })

  await fireEvent.click(view.getByRole('button', { name: 'Go to Second article' }))
  expect(view.getByRole('button', { name: 'Go to Second article' }).getAttribute('aria-current')).toBe('true')

  await fireEvent.click(view.getByRole('button', { name: 'Previous article' }))
  expect(view.getByRole('button', { name: 'Go to Article' }).getAttribute('aria-current')).toBe('true')
})

test('opens an article and selecting its demonstration closes it', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: true, text: async () => '<p>Article body</p>' }))
  const view = render(CatalogPanel, { props: { catalog, session: empty, effective: { 'g.on': false } }, global })
  await fireEvent.click(view.getByRole('button', { name: /Open Article/ }))
  await waitFor(() => expect(view.getByText('Article body')).toBeTruthy())
  await fireEvent.click(view.getAllByRole('button', { name: 'Try it' }).at(-1)!)
  expect(view.emitted().demonstration?.[0]).toEqual(['a', 'd'])
  expect(view.queryByText('Article body')).toBeNull()
  vi.unstubAllGlobals()
})

test('offers article provenance for preset values and reset for manual values', async () => {
  const active: SimulatorSession = { ...empty, selectedDemonstrations: { a: 'd' } }
  const view = render(CatalogPanel, { props: { catalog, session: active, effective: { 'g.on': true } }, global })
  expect(view.getByRole('button', { name: 'Learn why Enabled is set' })).toBeTruthy()

  await view.rerender({ catalog, session: { ...active, manual: { 'g.on': false } }, effective: { 'g.on': false } })
  await fireEvent.click(view.getByRole('button', { name: 'Reset Enabled' }))
  expect(view.emitted().reset?.[0]).toEqual(['g.on'])
})

test('shows article loading failures in the dialog', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, statusText: 'Missing' }))
  const view = render(CatalogPanel, { props: { catalog, session: empty, effective: { 'g.on': false } }, global })
  await fireEvent.click(view.getByRole('button', { name: /Open Article/ }))
  await waitFor(() => expect(view.getByRole('alert').textContent).toBe('This article could not be loaded.'))
  vi.unstubAllGlobals()
})
