import { defineConfig, devices } from '@playwright/test'
export default defineConfig({ testDir: './tests/e2e', use: { baseURL: 'http://127.0.0.1:4173' }, webServer: { command: 'vite --host 127.0.0.1 --port 4173', port: 4173, reuseExistingServer: true }, projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'], channel: 'chrome' } }] })
