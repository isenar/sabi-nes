import { defineConfig } from 'vite';

export default defineConfig({
  build: {
    target: 'es2022',
  },
  optimizeDeps: {
    exclude: ['sabi_nes_wasm'],
  },
});
