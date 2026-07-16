[![CI](https://github.com/brief-jetzt/wasm-typst/actions/workflows/CI.yml/badge.svg)](https://github.com/brief-jetzt/wasm-typst/actions/workflows/CI.yml) [![CD](https://github.com/brief-jetzt/wasm-typst/actions/workflows/CD.yml/badge.svg)](https://github.com/brief-jetzt/wasm-typst/actions/workflows/CD.yml) ![version](https://img.shields.io/npm/v/%40brief-jetzt/wasm-typst) ![downloads](https://img.shields.io/npm/dm/%40brief-jetzt/wasm-typst) ![License](https://img.shields.io/npm/l/%40brief-jetzt%2Fwasm-typst)

# wasm bindings for typst
This package allows you to use the [typst](https://github.com/typst/typst) library in the browser.

## Usage
The package ships an ergonomic wrapper. Each renderer owns its own wasm instance, so you can create as many as you need:

```sh
npm install @brief-jetzt/wasm-typst
```

```ts
import { createTypstRenderer } from "@brief-jetzt/wasm-typst";

const renderer = createTypstRenderer({
  fonts: [{ path: "MyFont.ttf", data: fontBytes }], // Uint8Array; optional
  sources: {
    "main.typ": "#set text(font: \"My Font\")\nHello #sys.inputs.name",
  },
  // files: { "logo.png": pngBytes },  // optional binary assets
});

const { output, diagnostics } = renderer.render({
  type: "pdf", // or "svg" | "svg-pages" | "png-pages"
  input: { name: "world" }, // typst sys.inputs
  // now: Temporal.Instant | Date | epoch millis — feeds datetime.today() and
  // the PDF timestamp; defaults to Date.now(). Fix it for reproducible output.
});
// output: Uint8Array (pdf) or string (svg); one entry per page for the -pages types
// diagnostics: Diagnostic[] ([] when clean), errors first:
//   { severity: "error" | "warning", message, path?, start?, end?, line?, column?, hints }

renderer.updateSource("main.typ", "Changed");
renderer.update({ sources: { "main.typ": "..." } }); // shallow-merge

// Single-page PNG (thumbnails etc.), 2 pixels per typographic point:
const png = renderer.renderPng(0, 2);

// Editor/IDE helpers (byte offsets into the given source):
renderer.autocomplete("main.typ", 3, true); // { from, completions: [{ kind, label, apply?, detail? }] }?
renderer.tooltip("main.typ", 13); // { kind: "text" | "code", text }?
renderer.goToDefinition(0, 35, 27); // click (page, xMm, yMm) -> { path, cursor }?
renderer.jumpFromCursor("main.typ", 3); // cursor -> [{ page, x, y }] in mm

renderer.dispose(); // frees the wasm instance (or `using renderer = ...`)
```

## Development
Running tests:
```sh
npm test
```

Building the package (runs `wasm-pack build`, then bundles the TS glue into `pkg/`):
```sh
npm ci
npm run build
```

You can then install the package in another npm project:
```sh
cd <your-npm-project>
npm install <path-to-this-repo>/pkg
```

If you are installing it in a project that already has `@brief-jetzt/wasm-typst` installed, you may want to
modify it's `package.json`: Make sure that the local dependency has the prefix `@brief-jetzt` set, so that
your import statements keep working:

```json
  "dependencies": {
    "@brief-jetzt/wasm-typst": "file:../../wasm-typst/pkg",
  }
```

Run `npm i`.
