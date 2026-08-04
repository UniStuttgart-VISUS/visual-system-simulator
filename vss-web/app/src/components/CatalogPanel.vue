<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { activePresets, currentLayer, selectedDemonstration as selectedDemonstrationId, sourceArticle, type SimulatorSession } from '../simulatorSession'
import type { Article, Catalog, Setting, Value } from '../types'
import UiDialog from './UiDialog.vue'

const props = defineProps<{ catalog: Catalog; session: SimulatorSession; effective: Record<string, Value> }>()
const emit = defineEmits<{ demonstration: [articleId: string, demonstrationId: string]; override: [id: string, value: Value]; reset: [id: string] }>()
const { t } = useI18n()
const expanded = ref(new Set(props.catalog.groups.map(group => group.id)))
const article = ref<Article>()
const articleHtml = ref('')
const articleError = ref(false)
const currentArticle = ref(0)
const gallery = ref<HTMLElement>()
const failedImages = ref(new Set<string>())
const active = computed(() => new Set(activePresets(props.session, props.catalog)))

function selectedDemonstration(value: Article) {
  return value.demonstrations.find(item => item.id === selectedDemonstrationId(props.session, value.id))
}

function articleLabel(value: Article, index: number) {
  const selected = selectedDemonstration(value)
  return [
    value.title,
    t('app.articlePosition', { current: index + 1, total: props.catalog.articles.length }),
    selected && t('app.activeVariant', { variant: selected.label }),
  ].filter(Boolean).join(', ')
}

function demonstrationLabel(value: Article, demonstrationId: string, label: string) {
  if (label.toLocaleLowerCase() !== value.title.toLocaleLowerCase()) return label
  return selectedDemonstrationId(props.session, value.id) === demonstrationId ? t('app.deactivate') : t('app.activate')
}

function select(value: Article, demonstrationId: string, closeArticle = false) {
  emit('demonstration', value.id, demonstrationId)
  if (closeArticle) article.value = undefined
}

async function openArticle(value: Article) {
  article.value = value
  articleHtml.value = ''
  articleError.value = false
  try {
    const response = await fetch(`./articles/${value.content_path}`)
    if (!response.ok) throw new Error(response.statusText)
    articleHtml.value = (await response.text()).replaceAll('src="', 'src="./articles/')
  } catch {
    articleError.value = true
  }
}

function updateCurrentArticle(event: Event) {
  const gallery = event.currentTarget as HTMLElement
  const cards = [...gallery.querySelectorAll<HTMLElement>('.article-card')]
  const left = gallery.getBoundingClientRect().left
  const closest = cards.reduce((best, card, index) =>
    Math.abs(card.getBoundingClientRect().left - left) < best.distance
      ? { index, distance: Math.abs(card.getBoundingClientRect().left - left) }
      : best,
  { index: 0, distance: Infinity })
  currentArticle.value = closest.index
}

function scrollToArticle(index: number) {
  const next = Math.max(0, Math.min(props.catalog.articles.length - 1, index))
  currentArticle.value = next
  const card = gallery.value?.querySelectorAll<HTMLElement>('.article-card')[next]
  if (card && gallery.value) gallery.value.scrollTo?.({ left: card.offsetLeft - gallery.value.offsetLeft, behavior: 'smooth' })
}

function setNumber(setting: Setting, raw: string | number) {
  const number = Number(raw)
  if (!Number.isFinite(number)) return
  emit('override', setting.id, Math.min(setting.control.max ?? Infinity, Math.max(setting.control.min ?? -Infinity, number)))
}

function pointPart(setting: Setting, index: number, raw: string) {
  const current = (props.effective[setting.id] as [number, number]) ?? [0, 0]
  const next: [number, number] = [...current]
  next[index] = Number(raw) || 0
  emit('override', setting.id, next)
}
</script>

<template>
  <section class="article-discovery" :aria-label="t('app.articles')">
    <div class="article-navigation">
      <div class="article-index" :aria-label="t('app.articleIndex')">
        <button
          v-for="(item, index) in catalog.articles"
          :key="item.id"
          class="article-index-segment"
          :class="{ current: currentArticle === index, active: item.demonstrations.some(demo => demo.presets.some(id => active.has(id))) }"
          :aria-label="t('app.goToArticle', { title: item.title })"
          :aria-current="currentArticle === index ? 'true' : undefined"
          @click="scrollToArticle(index)"
        />
      </div>
      <div class="gallery-arrows" :aria-label="t('app.galleryNavigation')">
        <button :aria-label="t('app.previousArticle')" :disabled="currentArticle === 0" @click="scrollToArticle(currentArticle - 1)">←</button>
        <button :aria-label="t('app.nextArticle')" :disabled="currentArticle === catalog.articles.length - 1" @click="scrollToArticle(currentArticle + 1)">→</button>
      </div>
    </div>
    <div ref="gallery" class="article-gallery" @scroll.passive="updateCurrentArticle">
      <article v-for="(item, index) in catalog.articles" :key="item.id" class="article-card" :aria-label="articleLabel(item, index)">
        <div class="article-card-main">
          <button class="article-open" :aria-label="t('app.openArticle', { title: item.title })" @click="openArticle(item)">
            <img v-if="item.image && !failedImages.has(item.id)" :src="`./articles/${item.image}`" alt="" @error="failedImages.add(item.id)">
            <span v-else class="article-image-fallback" aria-hidden="true">▧</span>
            <span class="article-overlay">
              <strong>{{ item.title }}</strong>
              <small v-if="item.summary">{{ item.summary }}</small>
            </span>
          </button>
          <button class="card-info" :aria-label="t('app.articleInfo', { title: item.title })" @click="openArticle(item)">ⓘ</button>
        </div>
        <div class="demonstration-segments" :class="{ empty: !item.demonstrations.length }">
          <button
            v-for="demonstration in item.demonstrations"
            :key="demonstration.id"
            :class="{ selected: selectedDemonstrationId(session, item.id) === demonstration.id }"
            :aria-pressed="selectedDemonstrationId(session, item.id) === demonstration.id"
            @click="select(item, demonstration.id)"
          >{{ demonstrationLabel(item, demonstration.id, demonstration.label) }}</button>
        </div>
      </article>
    </div>
  </section>

  <section class="catalog-section settings"><h2>{{ t('app.settings') }}</h2>
    <article v-for="group in catalog.groups" :key="group.id" class="group">
      <header @click="expanded.has(group.id) ? expanded.delete(group.id) : expanded.add(group.id)"><button class="group-title" :aria-expanded="expanded.has(group.id)">{{ group.title }}</button><span aria-hidden="true">{{ expanded.has(group.id) ? '−' : '+' }}</span></header>
      <div v-if="expanded.has(group.id)" class="setting-list"><div v-for="setting in group.settings.filter(item => item.control.kind !== 'text')" :key="setting.id" class="setting-row">
        <label :for="setting.id">{{ setting.label }} <small v-if="setting.unit">{{ setting.unit }}</small></label>
        <button v-if="setting.id in currentLayer(session).manual" class="setting-action" :aria-label="t('app.resetSetting', { setting: setting.label })" @click="emit('reset', setting.id)">↶</button>
        <button v-else-if="sourceArticle(session, catalog, setting.id)" class="setting-action" :aria-label="t('app.settingSource', { setting: setting.label })" @click="openArticle(catalog.articles.find(item => item.id === sourceArticle(session, catalog, setting.id))!)">ⓘ</button>
        <input v-if="setting.control.kind === 'boolean'" :id="setting.id" class="toggle" type="checkbox" :checked="Boolean(effective[setting.id])" @change="emit('override', setting.id, ($event.target as HTMLInputElement).checked)">
        <input v-else-if="setting.control.kind === 'number'" :id="setting.id" type="number" :value="effective[setting.id]" :min="setting.control.min ?? undefined" :max="setting.control.max ?? undefined" :step="setting.control.step" @change="setNumber(setting, ($event.target as HTMLInputElement).value)">
        <select v-else-if="setting.control.kind === 'choice'" :id="setting.id" :value="effective[setting.id]" @change="setNumber(setting, ($event.target as HTMLSelectElement).value)"><option v-for="choice in setting.control.choices" :key="choice.value" :value="choice.value">{{ choice.label }}</option></select>
        <span v-else-if="setting.control.kind === 'point'" class="point"><input :aria-label="`${setting.label} X`" type="number" :value="(effective[setting.id] as [number,number])?.[0]" @change="pointPart(setting,0,($event.target as HTMLInputElement).value)"><input :aria-label="`${setting.label} Y`" type="number" :value="(effective[setting.id] as [number,number])?.[1]" @change="pointPart(setting,1,($event.target as HTMLInputElement).value)"></span>
      </div></div>
    </article>
  </section>

  <UiDialog v-if="article" @close="article=undefined">
    <div class="article-dialog">
      <div class="dialog-head"><h2>{{ article.title }}</h2><button :aria-label="t('app.close')" @click="article=undefined">×</button></div>
      <div class="article-dialog-content">
        <p v-if="articleError" role="alert">{{ t('app.articleError') }}</p>
        <p v-else-if="!articleHtml" class="muted" aria-live="polite">{{ t('app.loadingArticle') }}</p>
        <div v-else class="article-body" v-html="articleHtml"/>
      </div>
      <div v-if="article.demonstrations.length" class="demonstration-segments dialog-demonstrations">
        <button
          v-for="demonstration in article.demonstrations"
          :key="demonstration.id"
          :class="{ selected: selectedDemonstrationId(session, article.id) === demonstration.id }"
          :aria-pressed="selectedDemonstrationId(session, article.id) === demonstration.id"
          @click="select(article, demonstration.id, true)"
        >{{ demonstrationLabel(article, demonstration.id, demonstration.label) }}</button>
      </div>
    </div>
  </UiDialog>
</template>
