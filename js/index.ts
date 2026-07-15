// Ergonomic wrapper around the raw wasm `World`. Each renderer owns one wasm
// instance — construct as many as you need.
//
// The wasm `setSourcesAndFiles`/`setFonts` methods fully *replace* their inputs
// (there is no incremental update on the Rust side), so this wrapper keeps the
// full source/file/font state on the JS side and re-sends all of it on every
// update. That state-keeping is the whole point of the wrapper.

// The `../pkg/...` specifier is rewritten to `./wasm_typst.js` at build time
// (build.sh), once this file has been emitted into pkg/ next to it.
import { World, FontInput, SourceInput, FileInput } from "../pkg/wasm_typst.js";

export interface FontSource {
  path: string;
  data: Uint8Array;
}

export interface TypstRendererOptions {
  fonts?: FontSource[];
  /** path -> typst source text */
  sources?: Record<string, string>;
  /** path -> binary asset (images, data, ...) */
  files?: Record<string, Uint8Array>;
}

/** typst `sys.inputs` — passed through to the document. */
export type Inputs = Record<string, string>;

export interface RenderResult<T> {
  output: T;
  /**
   * Compiler diagnostics as a string (empty when the compile was clean). The
   * wasm layer conflates errors and warnings here; on a hard error `output` is
   * empty and this explains why.
   */
  diagnostics: string;
}

export interface TypstRenderer extends Disposable{
  render(req: { type: "pdf"; input?: Inputs }): RenderResult<Uint8Array>;
  render(req: { type: "svg"; input?: Inputs }): RenderResult<string>;
  /** Set or replace a single source, then re-sync. */
  updateSource(path: string, content: string): void;
  /** Set or replace a single binary file, then re-sync. */
  updateFile(path: string, data: Uint8Array): void;
  /** Replace the whole font set. */
  setFonts(fonts: FontSource[]): void;
  /** Shallow-merge sources/files into current state; replace fonts if given. */
  update(patch: TypstRendererOptions): void;
  /** Free the underlying wasm instance. */
  dispose(): void;
}

class Renderer implements TypstRenderer {
  #world: World;
  #fonts: FontSource[];
  #sources = new Map<string, string>();
  #files = new Map<string, Uint8Array>();

  constructor(opts: TypstRendererOptions = {}) {
    this.#world = World.new();
    this.#fonts = opts.fonts ?? [];
    for (const [path, content] of Object.entries(opts.sources ?? {})) {
      this.#sources.set(path, content);
    }
    for (const [path, data] of Object.entries(opts.files ?? {})) {
      this.#files.set(path, data);
    }
    if (this.#fonts.length > 0) this.#syncFonts();
    this.#syncSourcesAndFiles();
  }

  #syncFonts(): void {
    this.#world.setFonts(this.#fonts.map((f) => FontInput.new(f.path, f.data)));
  }

  #syncSourcesAndFiles(): void {
    const sources = [...this.#sources].map(([p, s]) => SourceInput.new(p, s));
    const files = [...this.#files].map(([p, d]) => FileInput.new(p, d));
    this.#world.setSourcesAndFiles(sources, files);
  }

  render(req: { type: "pdf"; input?: Inputs }): RenderResult<Uint8Array>;
  render(req: { type: "svg"; input?: Inputs }): RenderResult<string>;
  render(req: {
    type: "pdf" | "svg";
    input?: Inputs;
  }): RenderResult<Uint8Array | string> {
    const diagnostics = this.#world.compile(req.input ?? {});
    const output =
      req.type === "pdf" ? this.#world.render_pdf() : this.#world.render_svg();
    return { output, diagnostics };
  }

  updateSource(path: string, content: string): void {
    this.#sources.set(path, content);
    this.#syncSourcesAndFiles();
  }

  updateFile(path: string, data: Uint8Array): void {
    this.#files.set(path, data);
    this.#syncSourcesAndFiles();
  }

  setFonts(fonts: FontSource[]): void {
    this.#fonts = fonts;
    this.#syncFonts();
  }

  update(patch: TypstRendererOptions): void {
    if (patch.fonts) this.setFonts(patch.fonts);
    let touched = false;
    for (const [path, content] of Object.entries(patch.sources ?? {})) {
      this.#sources.set(path, content);
      touched = true;
    }
    for (const [path, data] of Object.entries(patch.files ?? {})) {
      this.#files.set(path, data);
      touched = true;
    }
    if (touched) this.#syncSourcesAndFiles();
  }

  dispose(): void {
    this.#world.free();
  }

  [Symbol.dispose](): void {
    this.dispose();
  }
}

export function createTypstRenderer(
  opts?: TypstRendererOptions,
): TypstRenderer {
  return new Renderer(opts);
}

// Escape hatch: raw wasm classes.
export * from "../pkg/wasm_typst.js";
