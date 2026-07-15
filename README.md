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
  type: "pdf", // or "svg"
  input: { name: "world" }, // typst sys.inputs
});
// output: Uint8Array (pdf) or string (svg)
// diagnostics: compiler errors/warnings as a string ("" when clean)

renderer.updateSource("main.typ", "Changed");
renderer.update({ sources: { "main.typ": "..." } }); // shallow-merge

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
