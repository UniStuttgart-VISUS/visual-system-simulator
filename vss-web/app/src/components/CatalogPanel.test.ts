import { fireEvent, render, waitFor } from '@testing-library/vue'
import { createI18n } from 'vue-i18n'
import CatalogPanel from './CatalogPanel.vue'
import { messages } from '../i18n'
import type { Catalog } from '../types'
const catalog: Catalog = { groups:[{id:'g',title:'Group',settings:[{id:'g.on',label:'Enabled',help:'',default:false,control:{kind:'boolean'}}]}], presets:[{id:'p',label:'Preset',values:{'g.on':true}}], articles:[] }
const global = { plugins:[createI18n({legacy:false,locale:'en',messages})] }
test('activates presets and records manual changes', async () => { const view=render(CatalogPanel,{props:{catalog,active:[],overrides:{},effective:{'g.on':false}},global}); await fireEvent.click(view.getByText('Preset')); expect(view.emitted().presets?.[0]).toEqual([['p']]); await fireEvent.click(view.getByRole('checkbox')); expect(view.emitted().override?.[0]).toEqual(['g.on',true]) })
test('offers a conflict decision when removing a preset', async () => { const view=render(CatalogPanel,{props:{catalog,active:['p'],overrides:{'g.on':false},effective:{'g.on':false}},global}); await fireEvent.click(view.getByText('Preset')); expect(view.getByText('This preset affects manual changes.')).toBeTruthy(); await fireEvent.click(view.getByText('Keep manual changes')); expect(view.emitted().presets?.[0]).toEqual([[]]) })

test('loads an article and adds its demonstration presets', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: true, text: async () => '<p>Article body</p>' }))
  const withArticle: Catalog = { ...catalog, articles: [{ id: 'a', title: 'Article', content_path: 'a.html', demonstrations: [{ id: 'd', label: 'Try it', presets: ['p'] }] }] }
  const view = render(CatalogPanel, { props: { catalog: withArticle, active: [], overrides: {}, effective: { 'g.on': false } }, global })
  await fireEvent.click(view.getByText('Article'))
  await waitFor(() => expect(view.getByText('Article body')).toBeTruthy())
  await fireEvent.click(view.getByText('Try it'))
  await fireEvent.click(view.getByText('Add presets'))
  expect(view.emitted().presets?.[0]).toEqual([['p']])
  vi.unstubAllGlobals()
})

test('shows article loading failures in the dialog', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, statusText: 'Missing' }))
  const withArticle: Catalog = { ...catalog, articles: [{ id: 'a', title: 'Article', content_path: 'missing.html', demonstrations: [] }] }
  const view = render(CatalogPanel, { props: { catalog: withArticle, active: [], overrides: {}, effective: { 'g.on': false } }, global })
  await fireEvent.click(view.getByText('Article'))
  await waitFor(() => expect(view.getByRole('alert').textContent).toBe('This article could not be loaded.'))
  vi.unstubAllGlobals()
})
