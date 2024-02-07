import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { internalIpV4 } from "internal-ip";

export default defineConfig(async () => ({
  plugins: [vue()],
  clearScreen: false,
  server: {
    host: '0.0.0.0',
    port: 5173,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"]
    },
    hmr: {
      protocol: 'ws',
      host: await internalIpV4(),
      port: 5183,
    },
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: ['es2021', 'chrome100', 'safari13'],
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
}));
