import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  base: './',
  plugins: [vue(), tailwindcss()],
  test: { environment: 'jsdom', globals: true, include: ['src/**/*.test.ts'] },
  build: { target: 'es2022' },
})
