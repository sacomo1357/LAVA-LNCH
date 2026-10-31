import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    // Rust build output is huge and locked while building; never watch it
    watch: { ignored: ['**/src-tauri/**'] },
  },
})
