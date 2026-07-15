import { readFileSync, writeFileSync } from "node:fs";
import { defineConfig } from "tsdown";

// Runs after `wasm-pack build` (see the `build` npm script). Emits
// pkg/typst.mjs + pkg/typst.d.mts, then finishes wiring up the pkg/ folder that
// wasm-pack generated so it's ready to publish.
export default defineConfig({
  entry: { typst: "js/index.ts" },
  outDir: "pkg",
  format: "esm",
  dts: true,
  clean: false, // don't wipe the wasm-pack output in pkg/
  // The wasm glue is shipped by wasm-pack; keep it external.
  external: [/wasm_typst\.js$/],
  onSuccess() {
    // 1. The glue is authored against ../pkg/wasm_typst.js (dev path); once
    //    emitted into pkg/ it sits next to wasm_typst.js, so fix the specifier.
    for (const f of ["pkg/typst.mjs", "pkg/typst.d.mts"]) {
      writeFileSync(
        f,
        readFileSync(f, "utf8").replaceAll(
          "../pkg/wasm_typst.js",
          "./wasm_typst.js",
        ),
      );
    }

    // 2. Patch the package.json wasm-pack generated: publish metadata + point
    //    the entry at the glue and ship its files.
    // https://github.com/rustwasm/wasm-pack/issues/427#issuecomment-458180179
    const pkg = JSON.parse(readFileSync("pkg/package.json", "utf8"));
    pkg.name = "@brief-jetzt/wasm-typst";
    pkg.publishConfig = { access: "public" };
    pkg.repository = {
      type: "git",
      url: "https://github.com/brief-jetzt/wasm-typst",
    };
    pkg.main = "typst.mjs";
    pkg.types = "typst.d.mts";
    pkg.files = [...new Set([...pkg.files, "typst.mjs", "typst.d.mts"])];
    pkg.sideEffects = [...new Set([...pkg.sideEffects, "./typst.mjs"])];
    writeFileSync("pkg/package.json", JSON.stringify(pkg, null, 2) + "\n");
  },
});
