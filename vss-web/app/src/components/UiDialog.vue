<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
const emit = defineEmits<{ close: [] }>(); const dialog = ref<HTMLDialogElement>(); let returnFocus: HTMLElement | null = null
const key = (event: KeyboardEvent) => { if (event.key === 'Escape') emit('close') }
onMounted(() => { returnFocus = document.activeElement as HTMLElement | null; const element = dialog.value; if (element && typeof element.showModal === 'function') element.showModal(); else element?.setAttribute('open', ''); element?.querySelector<HTMLElement>('button, [href], input, select, textarea')?.focus(); window.addEventListener('keydown', key) }); onUnmounted(() => { window.removeEventListener('keydown', key); returnFocus?.focus() })
</script>
<template><dialog ref="dialog" class="dialog" @cancel.prevent="emit('close')"><slot /></dialog></template>
