// Ergonomic wrapper around the raw wasm `World`. Each renderer owns one wasm
// instance — construct as many as you need.
//
// Single-value updates (updateSource/updateFile) go through the wasm World's
// incremental methods, so the Rust side reuses the existing parse tree and only
// reparses the changed span. The World owns the authoritative source/file state;
// this wrapper is a thin, ergonomic front for it.

// The `../pkg/...` specifier is rewritten to `./wasm_typst.js` at build time
// (tsdown onSuccess), once this file has been emitted into pkg/ next to it.
import { World, FontInput, SourceInput, FileInput, DiagnosticSeverity } from "../pkg/wasm_typst.js";

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

/** A compiler diagnostic with its source location resolved. */
export interface Diagnostic {
  severity: "error" | "warning";
  message: string;
  /**
   * Source path as passed in `sources`/`updateSource`, e.g. "main.typ";
   * undefined when the diagnostic is not tied to a file.
   */
  path?: string;
  /** Byte range into that source. */
  start?: number;
  end?: number;
  /** 1-based line/column of `start`. */
  line?: number;
  column?: number;
  /** Suggestions on how to fix the problem. */
  hints: string[];
}

export interface RenderResult<T> {
  output: T;
  /**
   * Compiler diagnostics, errors first (empty when the compile was clean).
   * On a hard error `output` reflects the previously compiled document (or is
   * empty) and the errors here explain why.
   */
  diagnostics: Diagnostic[];
}

/** Where a click in the rendered document points to in the sources. */
export interface DefinitionPosition {
  /** Source path as passed in `sources`/`updateSource`, e.g. "main.typ". */
  path: string;
  /** Byte offset into that source. */
  cursor: number;
}

/** Page dimensions in millimeters. */
export interface PageSize {
  width: number;
  height: number;
}

export interface TypstRenderer extends Disposable {
  render(req: { type: "pdf"; input?: Inputs }): RenderResult<Uint8Array>;
  render(req: { type: "svg"; input?: Inputs }): RenderResult<string>;
  /** One SVG string per page, in page order. */
  render(req: { type: "svg-pages"; input?: Inputs }): RenderResult<string[]>;
  /**
   * Map a click at (xMm, yMm) millimeters from the top-left of the given
   * 0-based page to a source position ("go to definition"). Requires a prior
   * render(); returns undefined when the click hits nothing jumpable.
   */
  goToDefinition(page: number, xMm: number, yMm: number): DefinitionPosition | undefined;
  /** Number of pages in the compiled document (0 before the first render()). */
  getPageCount(): number;
  /** Page dimensions in mm; undefined if page is out of bounds or nothing rendered yet. */
  getPageSize(page: number): PageSize | undefined;
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
    const sources = Object.entries(opts.sources ?? {}).map(([p, s]) => SourceInput.new(p, s));
    const files = Object.entries(opts.files ?? {}).map(([p, d]) => FileInput.new(p, d));
    this.#world.setSourcesAndFiles(sources, files);
  }

  render(req: { type: "pdf"; input?: Inputs }): RenderResult<Uint8Array>;
  render(req: { type: "svg"; input?: Inputs }): RenderResult<string>;
  render(req: { type: "svg-pages"; input?: Inputs }): RenderResult<string[]>;
  render(req: {
    type: "pdf" | "svg" | "svg-pages";
    input?: Inputs;
  }): RenderResult<Uint8Array | string | string[]> {
    // Copy each wasm diagnostic into a plain object and free the handle so
    // consumers never have to manage wasm memory themselves.
    const diagnostics = this.#world.compile(req.input ?? {}).map(d => {
      try {
        return {
          // Crosses the wasm boundary as a number; string is nicer to consume.
          severity: d.severity === DiagnosticSeverity.Error ? ("error" as const) : ("warning" as const),
          message: d.message,
          path: d.path,
          start: d.start,
          end: d.end,
          line: d.line,
          column: d.column,
          hints: d.hints,
        };
      } finally {
        d.free();
      }
    });
    const output =
      req.type === "pdf"
        ? this.#world.render_pdf()
        : req.type === "svg-pages"
          ? this.#world.renderSvgPages()
          : this.#world.render_svg();
    return { output, diagnostics };
  }

  goToDefinition(page: number, xMm: number, yMm: number): DefinitionPosition | undefined {
    // Copy into a plain object and free the wasm handle so consumers never
    // have to manage wasm memory themselves.
    const pos = this.#world.goToDefinition(page, xMm, yMm);
    if (pos === undefined) return undefined;
    try {
      return { path: pos.path, cursor: pos.cursor };
    } finally {
      pos.free();
    }
  }

  getPageCount(): number {
    return this.#world.getPageCount();
  }

  getPageSize(page: number): PageSize | undefined {
    const size = this.#world.getPageSize(page);
    if (size === undefined) return undefined;
    try {
      return { width: size.width, height: size.height };
    } finally {
      size.free();
    }
  }

  updateSource(path: string, content: string): void {
    this.#world.updateSource(path, content);
  }

  updateFile(path: string, data: Uint8Array): void {
    this.#world.updateFile(path, data);
  }

  setFonts(fonts: FontSource[]): void {
    this.#world.setFonts(fonts.map(f => FontInput.new(f.path, f.data)));
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

export function createTypstRenderer(opts?: TypstRendererOptions): TypstRenderer {
  return new Renderer(opts);
}
