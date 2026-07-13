<script setup lang="ts">
import type { Article, Catalog, Preset, Setting, Value } from '../types'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import UiDialog from './UiDialog.vue'
const props = defineProps<{ catalog: Catalog; active: string[]; overrides: Record<string, Value>; effective: Record<string, Value> }>()
const emit = defineEmits<{ presets: [value: string[]]; override: [id: string, value: Value]; reset: [id?: string] }>()
const { t } = useI18n(); const expanded = ref(new Set(props.catalog.groups.map(g => g.id))); const article = ref<Article>(); const articleHtml = ref(''); const articleError = ref(false); const removing = ref<Preset>(); const demo = ref<string[]>()
function togglePreset(preset: Preset) { if (!props.active.includes(preset.id)) emit('presets', [...props.active, preset.id]); else if (Object.keys(preset.values).some(id => id in props.overrides)) removing.value = preset; else emit('presets', props.active.filter(id => id !== preset.id)) }
function remove(discard: boolean) { const preset = removing.value!; emit('presets', props.active.filter(id => id !== preset.id)); if (discard) Object.keys(preset.values).forEach(id => emit('reset', id)); removing.value = undefined }
async function openArticle(value: Article) { article.value = value; articleHtml.value = ''; articleError.value = false; try { const response = await fetch(`./articles/${value.content_path}`); if (!response.ok) throw new Error(response.statusText); articleHtml.value = (await response.text()).replaceAll('src="', 'src="./articles/') } catch { articleError.value = true } }
function setNumber(setting: Setting, raw: string | number) { const n = Number(raw); if (!Number.isFinite(n)) return; emit('override', setting.id, Math.min(setting.control.max ?? Infinity, Math.max(setting.control.min ?? -Infinity, n))) }
function pointPart(setting: Setting, index: number, raw: string) { const current = (props.effective[setting.id] as [number, number]) ?? [0, 0]; const next: [number, number] = [...current]; next[index] = Number(raw) || 0; emit('override', setting.id, next) }
const activeSet = computed(() => new Set(props.active))
</script>
<template>
  <section class="catalog-section"><h2>{{ t('app.articles') }}</h2><div class="chips"><button v-for="item in catalog.articles" :key="item.id" class="chip" @click="openArticle(item)">{{ item.title }}</button></div></section>
  <section class="catalog-section"><h2>{{ t('app.presets') }}</h2><div class="chips"><button v-for="preset in catalog.presets" :key="preset.id" class="chip" :class="{ active: activeSet.has(preset.id) }" :aria-pressed="activeSet.has(preset.id)" @click="togglePreset(preset)">{{ preset.label }}</button><button v-if="Object.keys(overrides).length" class="link-button" @click="emit('reset')">↶ {{ t('app.resetManual') }}</button></div></section>
  <section class="catalog-section settings"><h2>{{ t('app.settings') }}</h2>
    <article v-for="group in catalog.groups" :key="group.id" class="group">
      <header @click="expanded.has(group.id) ? expanded.delete(group.id) : expanded.add(group.id)"><button class="group-title" :aria-expanded="expanded.has(group.id)">{{ group.title }}</button><span>{{ expanded.has(group.id) ? '−' : '+' }}</span></header>
      <div v-if="expanded.has(group.id)" class="setting-list"><div v-for="setting in group.settings.filter(item => item.control.kind !== 'text')" :key="setting.id" class="setting-row">
        <label :for="setting.id">{{ setting.label }} <small v-if="setting.unit">{{ setting.unit }}</small></label><button v-if="setting.id in overrides" class="reset" :aria-label="t('app.reset')" @click="emit('reset', setting.id)">↶</button>
        <input v-if="setting.control.kind === 'boolean'" :id="setting.id" class="toggle" type="checkbox" :checked="Boolean(effective[setting.id])" @change="emit('override', setting.id, ($event.target as HTMLInputElement).checked)">
        <input v-else-if="setting.control.kind === 'number'" :id="setting.id" type="number" :value="effective[setting.id]" :min="setting.control.min ?? undefined" :max="setting.control.max ?? undefined" :step="setting.control.step" @change="setNumber(setting, ($event.target as HTMLInputElement).value)">
        <select v-else-if="setting.control.kind === 'choice'" :id="setting.id" :value="effective[setting.id]" @change="setNumber(setting, ($event.target as HTMLSelectElement).value)"><option v-for="choice in setting.control.choices" :key="choice.value" :value="choice.value">{{ choice.label }}</option></select>
        <span v-else-if="setting.control.kind === 'point'" class="point"><input :aria-label="`${setting.label} X`" type="number" :value="(effective[setting.id] as [number,number])?.[0]" @change="pointPart(setting,0,($event.target as HTMLInputElement).value)"><input :aria-label="`${setting.label} Y`" type="number" :value="(effective[setting.id] as [number,number])?.[1]" @change="pointPart(setting,1,($event.target as HTMLInputElement).value)"></span>
      </div></div>
    </article>
  </section>
  <UiDialog v-if="article" @close="article=undefined"><div class="dialog-head"><h2>{{ article.title }}</h2><button :aria-label="t('app.close')" @click="article=undefined">×</button></div><p v-if="articleError" role="alert">{{ t('app.articleError') }}</p><p v-else-if="!articleHtml" class="muted" aria-live="polite">{{ t('app.loadingArticle') }}</p><div v-else class="article-body" v-html="articleHtml"/><div v-if="!articleError" class="dialog-actions"><button v-for="item in article.demonstrations" :key="item.id" class="primary" @click="demo=item.presets; article=undefined">{{ item.label }}</button></div></UiDialog>
  <UiDialog v-if="removing" @close="removing=undefined"><h2>{{ t('app.removePreset',{name:removing.label}) }}</h2><p>{{ t('app.presetConflict') }}</p><div class="dialog-actions"><button class="primary" @click="remove(false)">{{ t('app.keepOverrides') }}</button><button @click="remove(true)">{{ t('app.discardOverrides') }}</button><button @click="removing=undefined">{{ t('app.cancel') }}</button></div></UiDialog>
  <UiDialog v-if="demo" @close="demo=undefined"><h2>{{ t('app.applyDemo') }}</h2><p>{{ t('app.demoMessage') }}</p><div class="dialog-actions"><button class="primary" @click="emit('presets',demo!); emit('reset'); demo=undefined">{{ t('app.replace') }}</button><button @click="emit('presets',[...new Set([...active,...demo!])]); demo=undefined">{{ t('app.add') }}</button><button @click="demo=undefined">{{ t('app.cancel') }}</button></div></UiDialog>
</template>
