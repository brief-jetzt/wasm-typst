# wasm-typst example
Vanilla Vite app (no frameworks) showing:
- SVG rendering, one `<svg>` per page (`render({ type: "svg-pages" })`)
- Incremental source updates while typing (`updateSource`)
- Go to definition: click the rendered document, the editor cursor jumps to the matching source position (`goToDefinition` + `getPageSize`)
- Loading fonts from an npm font package (`@expo-google-fonts/inter`. It ships TTF; Fontsource only ships woff/woff2, which typst can't parse)

## Run
```sh
# once, in the repo root: build pkg/
npm ci && npm run build

cd examples
npm install
npm run dev
```
