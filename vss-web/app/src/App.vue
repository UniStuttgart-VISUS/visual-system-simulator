<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import CatalogPanel from './components/CatalogPanel.vue'
import { detectedLocale } from './i18n'
import { flipRows, loadImage, loadVideo, type MediaSource } from './media'
import { applyWebRetina } from './retinaFrame'
import { compose, getCatalog, startSimulator } from './simulator'
import { activePresets, editSetting, resetSetting, selectDemonstration, type SimulatorSession } from './simulatorSession'
import type { Catalog, Value } from './types'

const { t } = useI18n(); const catalog = ref<Catalog>(); const simulator = ref<Awaited<ReturnType<typeof startSimulator>>>(); const session = ref<SimulatorSession>({ selectedDemonstrations: {}, manual: {}, maskedFallback: {} }); const state = ref<'loading'|'ready'|'unsupported'|'error'>('loading'); const mediaError = ref(''); const fullscreen = ref(false); const preview = ref<HTMLElement>(); let media: MediaSource | undefined; let objectUrl: string | undefined; let settingsTimer = 0; let resizeFrame = 0; let resizeObserver: ResizeObserver | undefined; let rgbd = false; let lastFrame: { pixels: Uint8Array; width: number; height: number } | undefined
const active = computed(() => catalog.value ? activePresets(session.value, catalog.value) : [])
const effective = computed(() => catalog.value ? compose(detectedLocale, active.value, session.value.manual) : {})
function upload(frame: { pixels: Uint8Array; width: number; height: number }) {
  const colorBytes = rgbd ? frame.width * Math.floor(frame.height / 2) * 4 : frame.pixels.length
  const pixels = frame.pixels.slice()
  pixels.set(applyWebRetina(frame.pixels.subarray(0, colorBytes), frame.width, rgbd ? Math.floor(frame.height / 2) : frame.height, effective.value))
  if (rgbd) flipRows(pixels, frame.width, frame.height)
  simulator.value?.post_frame(pixels, frame.width, frame.height, rgbd)
}
function submit(pixels: Uint8Array, width: number, height: number) { lastFrame = { pixels, width, height }; try { upload(lastFrame) } catch { /* a newer video frame wins */ } }
async function replaceMedia(file?: File) { mediaError.value = ''; media?.dispose(); if (objectUrl) URL.revokeObjectURL(objectUrl); const defaultImage = 'marketplace.rgbd.png'; rgbd = (file?.name ?? defaultImage).toLowerCase().includes('.rgbd.'); objectUrl = file ? URL.createObjectURL(file) : `./assets/${defaultImage}`; try { media = file?.type.startsWith('video/') ? await loadVideo(objectUrl, submit) : await loadImage(objectUrl, submit) } catch { mediaError.value = t('app.mediaError') } }
async function initialize() { if (!('gpu' in navigator)) { state.value = 'unsupported'; return } try { simulator.value = await startSimulator('simulator-canvas'); catalog.value = getCatalog(detectedLocale); state.value = 'ready'; await nextTick(); await replaceMedia() } catch (error) { console.error(error); state.value = 'error' } }
watch(effective, value => { clearTimeout(settingsTimer); settingsTimer = window.setTimeout(() => { try { simulator.value?.post_settings(JSON.stringify(value)); if (lastFrame) upload(lastFrame) } catch (error) { console.error(error); state.value = 'error' } }, 100) }, { deep: true })
function setOverride(id: string, value: Value) { session.value = editSetting(session.value, id, value) }
function reset(id: string) { session.value = resetSetting(session.value, id) }
function demonstrate(articleId: string, demonstrationId: string) { if (catalog.value) session.value = selectDemonstration(session.value, catalog.value, articleId, demonstrationId) }
async function enterFullscreen() { try { await preview.value?.requestFullscreen() } catch { mediaError.value = t('app.initError') } }
function fullscreenChange() { fullscreen.value = document.fullscreenElement === preview.value }
function exitFullscreen() { if (document.fullscreenElement) document.exitFullscreen().catch(() => undefined) }
function reload() { window.location.reload() }
onMounted(() => { document.addEventListener('fullscreenchange', fullscreenChange); resizeObserver = new ResizeObserver(() => { cancelAnimationFrame(resizeFrame); resizeFrame = requestAnimationFrame(() => simulator.value?.resize()) }); if (preview.value) resizeObserver.observe(preview.value); initialize() }); onBeforeUnmount(() => { clearTimeout(settingsTimer); cancelAnimationFrame(resizeFrame); resizeObserver?.disconnect(); media?.dispose(); simulator.value?.destroy(); simulator.value?.free(); if (objectUrl) URL.revokeObjectURL(objectUrl); document.removeEventListener('fullscreenchange', fullscreenChange) })
</script>
<template>
  <main v-if="state==='unsupported'" class="center-state"><h1>{{ t('app.webgpuTitle') }}</h1><p>{{ t('app.webgpuBody') }}</p></main>
  <main v-else-if="state==='error'" class="center-state"><h1>{{ t('app.initError') }}</h1><button class="primary" @click="reload">{{ t('app.reload') }}</button></main>
  <main v-else class="app-shell">
    <section ref="preview" class="preview" :class="{ fullscreen }" @click="fullscreen && exitFullscreen()">
      <div id="simulator-canvas" class="canvas-host"/><div v-if="state==='loading'" class="loading">{{ t('app.loading') }}</div>
      <div v-if="!fullscreen" class="preview-actions">
        <label class="primary icon-button file-button" :aria-label="t('app.chooseMedia')" :title="t('app.chooseMedia')">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 16V4m0 0L7 9m5-5 5 5M5 14v5h14v-5"/></svg>
          <input type="file" accept="image/*,video/*" @change="replaceMedia(($event.target as HTMLInputElement).files?.[0])">
        </label>
        <button class="icon-button" :aria-label="t('app.fullscreen')" :title="t('app.fullscreen')" @click="enterFullscreen">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 4H4v5m11-5h5v5M9 20H4v-5m11 5h5v-5"/></svg>
        </button>
      </div>
      <p v-if="mediaError && !fullscreen" class="media-error" role="alert">{{ mediaError }}</p>
    </section>
    <aside v-if="catalog" class="panel"><CatalogPanel :catalog="catalog" :session="session" :effective="effective" @demonstration="demonstrate" @override="setOverride" @reset="reset" /></aside>
  </main>
</template>
