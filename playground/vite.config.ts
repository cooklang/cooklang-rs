import { defineConfig } from 'vite'

export default defineConfig({
  optimizeDeps: {
    exclude: ['@cooklang/cooklang-ts']
  }
})