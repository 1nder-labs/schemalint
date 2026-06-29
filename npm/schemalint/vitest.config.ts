import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: ['src/__tests__/test_*.ts'],
    exclude: ['**/fixtures/**', '**/node_modules/**'],
    testTimeout: 30000,
  },
});
