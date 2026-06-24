import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import { viteSingleFile } from 'vite-plugin-singlefile'

// singlefile：把 JS/CSS 全内联进一个 HTML → 仍可 `include_str!` 进 eqc.exe（自包含、离线）。
export default defineConfig({
  plugins: [svelte(), viteSingleFile()],
  build: {
    target: 'es2020',
    assetsInlineLimit: 100000000,
    chunkSizeWarningLimit: 100000000,
  },
})
