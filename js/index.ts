// Ergonomic wrapper around the raw wasm `World`. Each renderer owns one wasm
// instance — construct as many as you need.
//
// Single-value updates (updateSource/updateFile) go through the wasm World's
// incremental methods, so the Rust side reuses the existing parse tree and only
// reparses the changed span. The World owns the authoritative source/file state;
// this wrapper is a thin, ergonomic front for it.

// The `../pkg/...` specifier is rewritten to `./wasm_typst.js` at build time
// (tsdown onSuccess), once this file has been emitted into pkg/ next to it.
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
  /** One SVG string per page, in page order. */
  render(req: { type: "svg-pages"; input?: Inputs }): RenderResult<string[]>;
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

  constructor(opts: TypstRendererOptions = {}) {
    this.#world = World.new();
    if (opts.fonts?.length) this.setFonts(opts.fonts);
    // Bulk-load the initial set in one call; later edits go incremental.
    const sources = Object.entries(opts.sources ?? {}).map(([p, s]) =>
      SourceInput.new(p, s),
    );
    const files = Object.entries(opts.files ?? {}).map(([p, d]) =>
      FileInput.new(p, d),
    );
    this.#world.setSourcesAndFiles(sources, files);
  }

  render(req: { type: "pdf"; input?: Inputs }): RenderResult<Uint8Array>;
  render(req: { type: "svg"; input?: Inputs }): RenderResult<string>;
  render(req: { type: "svg-pages"; input?: Inputs }): RenderResult<string[]>;
  render(req: {
    type: "pdf" | "svg" | "svg-pages";
    input?: Inputs;
  }): RenderResult<Uint8Array | string | string[]> {
    const diagnostics = this.#world.compile(req.input ?? {});
    const output =
      req.type === "pdf"
        ? this.#world.render_pdf()
        : req.type === "svg-pages"
          ? this.#world.renderSvgPages()
          : this.#world.render_svg();
    return { output, diagnostics };
  }

  updateSource(path: string, content: string): void {
    this.#world.updateSource(path, content);
  }

  updateFile(path: string, data: Uint8Array): void {
    this.#world.updateFile(path, data);
  }

  setFonts(fonts: FontSource[]): void {
    this.#world.setFonts(fonts.map((f) => FontInput.new(f.path, f.data)));
  }

  update(patch: TypstRendererOptions): void {
    if (patch.fonts) this.setFonts(patch.fonts);
    for (const [path, content] of Object.entries(patch.sources ?? {})) {
      this.#world.updateSource(path, content);
    }
    for (const [path, data] of Object.entries(patch.files ?? {})) {
      this.#world.updateFile(path, data);
    }
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
