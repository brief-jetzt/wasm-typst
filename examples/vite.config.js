import { defineConfig } from "vite";
import wasm from "vite-plugin-wasm"; // needed for the internal wasm import of wasm-typst

export default defineConfig({
  plugins: [wasm()],
});
