import { defineConfig } from "vite";
import wasm from "vite-plugin-wasm";

export default defineConfig({
  plugins: [wasm()],
  optimizeDeps: {
    exclude: ["@cooklang/cooklang-ts"],
  },
  build: {
    target: "esnext",
  }
});
