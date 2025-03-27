[![CI](https://github.com/brief-jetzt/wasm-typst/actions/workflows/CI.yml/badge.svg)](https://github.com/brief-jetzt/wasm-typst/actions/workflows/CI.yml) [![CD](https://github.com/brief-jetzt/wasm-typst/actions/workflows/CD.yml/badge.svg)](https://github.com/brief-jetzt/wasm-typst/actions/workflows/CD.yml) ![version](https://img.shields.io/npm/v/%40brief-jetzt/wasm-typst) ![downloads](https://img.shields.io/npm/dm/%40brief-jetzt/wasm-typst) ![License](https://img.shields.io/npm/l/%40brief-jetzt%2Fwasm-typst)


# wasm bindings for typst

This package allows you to use the [typst][typst] library in the browser.

- [npm package][npm-package]

## Usage

TODO

## Developing

Running tests:

```sh
wasm-pack test --chrome --firefox --headless
```

Building the package:

```
wasm-pack build
```

You can then install the package in another npm project:

```sh
cd <your-npm-project>
npm install <path-to-this-repo>/pkg
```

If you are installing it in a project that already has `@brief-jetzt/wasm-typst` installed, you may want to
modify it's `package.json`: Make sure that the local dependency has the prefix `@brief-jetzt` set, so that
your import statements keep working:

```
[…]
  "dependencies": {
    "@brief-jetzt/wasm-typst": "file:../../wasm-typst/pkg",
    […]
```

Run `npm i`.

---

Note: The wasm bindings are based on this [tutorial][wasm-tutorial]

[wasm-tutorial]: https://rustwasm.github.io/docs/wasm-pack/tutorials/index.html
[typst]: https://github.com/typst/typst
[npm-package]: https://www.npmjs.com/package/@brief-jetzt/wasm-typst
