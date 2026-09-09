import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import autoprefixer from 'autoprefixer';
import path from 'path';
import tailwindcss from 'tailwindcss';

export default defineConfig({
  plugins: [svelte()],
  css: {
    postcss: {
      plugins: [tailwindcss(), autoprefixer()],
    },
  },
  clearScreen: false,
  server: {
    host: '127.0.0.1',
    port: 5173,
    strictPort: true,
    // target/ 是 cargo 构建产物目录：dev 模式下 cargo 与 vite 并行运行，
    // 监视其中的 dll/中间产物会在写入瞬间以 EBUSY 崩溃（Windows 实测）。
    watch: {
      ignored: ['**/target/**'],
    },
    cors: {
      origin: /^https?:\/\/(?:127\.0\.0\.1|localhost)(?::\d+)?$/,
    },
    hmr: {
      host: '127.0.0.1',
    },
  },
  envPrefix: ['VITE_', 'TAURI_'],
  optimizeDeps: {
    entries: ['index.html'],
  },
  build: {
    target: ['es2021', 'chrome100', 'safari13'],
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
  resolve: {
    alias: {
      '$lib': path.resolve('./src/lib'),
    },
  },
});
