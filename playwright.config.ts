import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: 'tests/ui', timeout: 60000, fullyParallel: false,
  use: { channel: process.env.CI ? undefined : 'chrome', baseURL: 'http://127.0.0.1:1420', trace: 'retain-on-failure' },
  webServer: { command: 'node ./node_modules/vite/bin/vite.js --host 127.0.0.1', url: 'http://127.0.0.1:1420', reuseExistingServer: !process.env.CI },
  reporter: [['list'], ['html', { open: 'never' }]],
});
