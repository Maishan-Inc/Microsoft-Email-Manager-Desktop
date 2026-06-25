import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [svelte()],
  // 防止 vite 屏蔽 Rust 错误
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // tauri 后端目录交由 cargo 监听
      ignored: ["**/src-tauri/**"],
    },
  },
  // 让构建产物更小、更省内存
  build: {
    target: "esnext",
    minify: "esbuild",
    sourcemap: false,
  },
});
