<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import CatalogPanel from './components/CatalogPanel.vue'
import { detectedLocale } from './i18n'
import { flipRows, loadImage, loadVideo, type MediaSource } from './media'
import { getCatalog, startSimulator } from './simulator'
import { createSession, editSetting, editableValues, effectiveValues, resetSetting, selectDemonstration, setEyeMode, type EyeMode, type SimulatorSession } from './simulatorSession'
import type { Catalog, Value } from './types'

const { t } = useI18n(); const catalog = ref<Catalog>(); const simulator = ref<Awaited<ReturnType<typeof startSimulator>>>(); const session = ref<SimulatorSession>(createSession()); const state = ref<'loading'|'ready'|'unsupported'|'error'>('loading'); const mediaError = ref(''); const fullscreen = ref(false); const preview = ref<HTMLElement>(); let media: MediaSource | undefined; let objectUrl: string | undefined; let settingsTimer = 0; let resizeFrame = 0; let resizeObserver: ResizeObserver | undefined; let rgbd = false; let lastFrame: { pixels: Uint8Array; width: number; height: number } | undefined
const effective = computed(() => catalog.value ? editableValues(session.value, catalog.value) : {})
const leftEffective = computed(() => catalog.value ? effectiveValues(session.value, catalog.value, 'left') : {})
const rightEffective = computed(() => catalog.value ? effectiveValues(session.value, catalog.value, 'right') : {})
const pointers = new Map<number, { x: number; y: number; type: string }>()
let lastTap: { at: number; x: number; y: number } | undefined
function sendInput(kind: 'gaze_delta'|'view_delta'|'reset_pose', x = 0, y = 0) { try { simulator.value?.semantic_input(kind, x, y) } catch (error) { console.error(error) } }
function pointerDown(event: PointerEvent) {
  preview.value?.setPointerCapture(event.pointerId)
  pointers.set(event.pointerId, { x: event.clientX, y: event.clientY, type: event.pointerType })
  if (event.pointerType === 'touch' && pointers.size === 1) {
    const now = performance.now()
    if (lastTap && now - lastTap.at <= 500 && Math.hypot(event.clientX - lastTap.x, event.clientY - lastTap.y) <= 24) {
      sendInput('reset_pose'); lastTap = undefined
    } else lastTap = { at: now, x: event.clientX, y: event.clientY }
  }
}
function pointerMove(event: PointerEvent) {
  const previous = pointers.get(event.pointerId)
  if (!previous || !preview.value) return
  const width = Math.max(1, preview.value.clientWidth); const height = Math.max(1, preview.value.clientHeight)
  if (event.pointerType === 'touch') {
    const before = [...pointers.values()]
    pointers.set(event.pointerId, { x: event.clientX, y: event.clientY, type: event.pointerType })
    const after = [...pointers.values()]
    if (after.length === 1) sendInput('gaze_delta', (event.clientX - previous.x) / width, (event.clientY - previous.y) / height)
    else if (after.length === 2 && before.length === 2) {
      const centroid = (points: { x: number; y: number }[]) => ({ x: (points[0].x + points[1].x) / 2, y: (points[0].y + points[1].y) / 2 })
      const from = centroid(before); const to = centroid(after)
      sendInput('view_delta', (to.x - from.x) / width, (to.y - from.y) / height)
    }
  } else {
    pointers.set(event.pointerId, { x: event.clientX, y: event.clientY, type: event.pointerType })
    const kind = event.buttons & 1 ? 'gaze_delta' : event.buttons & 2 ? 'view_delta' : undefined
    if (kind) sendInput(kind, (event.clientX - previous.x) / width, (event.clientY - previous.y) / height)
  }
}
function pointerEnd(event: PointerEvent) { pointers.delete(event.pointerId); if (preview.value?.hasPointerCapture(event.pointerId)) preview.value.releasePointerCapture(event.pointerId) }
function upload(frame: { pixels: Uint8Array; width: number; height: number }) {
  const pixels = frame.pixels.slice()
  if (rgbd) flipRows(pixels, frame.width, frame.height)
  simulator.value?.post_frame(pixels, frame.width, frame.height, rgbd)
}
function submit(pixels: Uint8Array, width: number, height: number) { lastFrame = { pixels, width, height }; try { upload(lastFrame) } catch { /* a newer video frame wins */ } }
async function replaceMedia(file?: File) {
  mediaError.value = ''
  const defaultImage = 'marketplace.rgbd.png'
  const nextRgbd = (file?.name ?? defaultImage).toLowerCase().includes('.rgbd.')
  const nextUrl = file ? URL.createObjectURL(file) : `./assets/${defaultImage}`
  let committed = false; let pending: { pixels: Uint8Array; width: number; height: number } | undefined
  const candidateSubmit = (pixels: Uint8Array, width: number, height: number) => {
    if (committed) submit(pixels, width, height)
    else pending = { pixels, width, height }
  }
  try {
    const candidate = file?.type.startsWith('video/') ? await loadVideo(nextUrl, candidateSubmit) : await loadImage(nextUrl, candidateSubmit)
    const previousMedia = media; const previousUrl = objectUrl
    media = candidate; objectUrl = nextUrl; rgbd = nextRgbd; committed = true
    if (pending) submit(pending.pixels, pending.width, pending.height)
    previousMedia?.dispose(); if (previousUrl?.startsWith('blob:')) URL.revokeObjectURL(previousUrl)
    sendInput('reset_pose')
  } catch {
    if (nextUrl.startsWith('blob:')) URL.revokeObjectURL(nextUrl)
    mediaError.value = t('app.mediaError')
  }
}
function mediaSelected(event: Event) { const input = event.target as HTMLInputElement; const file = input.files?.[0]; input.value = ''; if (file) replaceMedia(file) }
async function initialize() { if (!('gpu' in navigator)) { state.value = 'unsupported'; return } try { simulator.value = await startSimulator('simulator-canvas'); catalog.value = getCatalog(detectedLocale); state.value = 'ready'; await nextTick(); await replaceMedia() } catch (error) { console.error(error); state.value = 'error' } }
watch([leftEffective, rightEffective], ([left, right]) => { clearTimeout(settingsTimer); settingsTimer = window.setTimeout(() => { try { simulator.value?.post_settings(JSON.stringify(left), JSON.stringify(right)); if (lastFrame) upload(lastFrame) } catch (error) { console.error(error); state.value = 'error' } }, 100) }, { deep: true })
watch(() => session.value.eyeMode, eyeMode => { try { simulator.value?.set_eye_mode(eyeMode) } catch (error) { console.error(error); state.value = 'error' } })
function setOverride(id: string, value: Value) { session.value = editSetting(session.value, id, value) }
function reset(id: string) { if (catalog.value) session.value = resetSetting(session.value, id, catalog.value) }
function demonstrate(articleId: string, demonstrationId: string) { if (catalog.value) session.value = selectDemonstration(session.value, catalog.value, articleId, demonstrationId) }
function chooseEyeMode(eyeMode: EyeMode) { session.value = setEyeMode(session.value, eyeMode) }
type LockableOrientation = ScreenOrientation & { lock?: (orientation: 'landscape') => Promise<void> }
async function enterFullscreen() { try { await preview.value?.requestFullscreen(); if (session.value.eyeMode === 'both') await (screen.orientation as LockableOrientation)?.lock?.('landscape').catch(() => undefined) } catch { mediaError.value = t('app.initError') } }
function fullscreenChange() { fullscreen.value = document.fullscreenElement === preview.value; if (!fullscreen.value) screen.orientation?.unlock?.() }
function reload() { window.location.reload() }
onMounted(() => { document.addEventListener('fullscreenchange', fullscreenChange); resizeObserver = new ResizeObserver(() => { cancelAnimationFrame(resizeFrame); resizeFrame = requestAnimationFrame(() => simulator.value?.resize()) }); if (preview.value) resizeObserver.observe(preview.value); initialize() }); onBeforeUnmount(() => { clearTimeout(settingsTimer); cancelAnimationFrame(resizeFrame); resizeObserver?.disconnect(); media?.dispose(); simulator.value?.destroy(); simulator.value?.free(); if (objectUrl) URL.revokeObjectURL(objectUrl); document.removeEventListener('fullscreenchange', fullscreenChange) })
</script>
<template>
  <main v-if="state==='unsupported'" class="center-state"><h1>{{ t('app.webgpuTitle') }}</h1><p>{{ t('app.webgpuBody') }}</p></main>
  <main v-else-if="state==='error'" class="center-state"><h1>{{ t('app.initError') }}</h1><button class="primary" @click="reload">{{ t('app.reload') }}</button></main>
  <main v-else class="app-shell">
    <section ref="preview" class="preview" :class="{ fullscreen }" @pointerdown="pointerDown" @pointermove="pointerMove" @pointerup="pointerEnd" @pointercancel="pointerEnd" @dblclick="sendInput('reset_pose')" @contextmenu.prevent>
      <div id="simulator-canvas" class="canvas-host"/><div v-if="state==='loading'" class="loading">{{ t('app.loading') }}</div>
      <div v-if="!fullscreen" class="preview-actions" @pointerdown.stop @click.stop>
        <label class="primary icon-button file-button" :aria-label="t('app.chooseMedia')" :title="t('app.chooseMedia')">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 16V4m0 0L7 9m5-5 5 5M5 14v5h14v-5"/></svg>
          <input type="file" accept="image/*,video/*" @change="mediaSelected">
        </label>
        <div class="preview-controls">
          <div class="eye-selector" :aria-label="t('app.eyeMode')" role="group">
            <button v-for="eyeMode in (['left','both','right'] as EyeMode[])" :key="eyeMode" class="eye-button" :class="{ selected: session.eyeMode === eyeMode }" :aria-label="t(`app.eye.${eyeMode}`)" :title="t(`app.eye.${eyeMode}`)" :aria-pressed="session.eyeMode === eyeMode" @pointerdown.stop @click.stop="chooseEyeMode(eyeMode)">
              <svg viewBox="0 0 32 22" aria-hidden="true">
                <path class="viewer" d="M3 8.5 6 5h20l3 3.5v8L26 19H6l-3-2.5z"/>
                <circle class="lens left" :class="{ active: eyeMode !== 'right' }" cx="11" cy="12" r="4"/>
                <circle class="lens right" :class="{ active: eyeMode !== 'left' }" cx="21" cy="12" r="4"/>
              </svg>
            </button>
          </div>
          <button class="icon-button" :aria-label="t('app.fullscreen')" :title="t('app.fullscreen')" @pointerdown.stop @click.stop="enterFullscreen">
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 4H4v5m11-5h5v5M9 20H4v-5m11 5h5v-5"/></svg>
          </button>
        </div>
      </div>
      <p v-if="mediaError && !fullscreen" class="media-error" role="alert">{{ mediaError }}</p>
    </section>
    <aside v-if="catalog" class="panel"><CatalogPanel :catalog="catalog" :session="session" :effective="effective" @demonstration="demonstrate" @override="setOverride" @reset="reset" /></aside>
  </main>
</template>
