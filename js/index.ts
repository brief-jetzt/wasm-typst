// Ergonomic wrapper around the raw wasm `World`. Each renderer owns one wasm
// instance — construct as many as you need.
//
// Single-value updates (updateSource/updateFile) go through the wasm World's
// incremental methods, so the Rust side reuses the existing parse tree and only
// reparses the changed span. The World owns the authoritative source/file state;
// this wrapper is a thin, ergonomic front for it.

// The `../pkg/...` specifier is rewritten to `./wasm_typst.js` at build time
// (tsdown onSuccess), once this file has been emitted into pkg/ next to it.
import {
  World,
  FontInput,
  SourceInput,
  FileInput,
  DiagnosticSeverity,
  TooltipKind,
} from "../pkg/wasm_typst.js";

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

/**
 * A point in time: unix epoch milliseconds, a `Date`, or anything with a
 * `Temporal.Instant`-shaped `epochMilliseconds` (so `Temporal.Instant` works
 * without this package depending on Temporal types).
 */
export type Instant = number | Date | { readonly epochMilliseconds: number };

function toEpochMillis(now: Instant | undefined): number {
  if (now === undefined) return Date.now();
  if (typeof now === "number") return now;
  if (now instanceof Date) return now.getTime();
  return now.epochMilliseconds;
}

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

/** One autocomplete suggestion. */
export interface Completion {
  kind:
    | "syntax"
    | "func"
    | "type"
    | "param"
    | "constant"
    | "path"
    | "package"
    | "label"
    | "font"
    | "symbol";
  /** The label the completion is shown with. */
  label: string;
  /**
   * The completed version of the input, possibly using snippet syntax like
   * `${lhs} + ${rhs}`. Defaults to the label when absent.
   */
  apply?: string;
  /** An optional short description. */
  detail?: string;
}

export interface Completions {
  /** Byte offset from which the completions replace text (up to the cursor). */
  from: number;
  completions: Completion[];
}

/** Hover information for a cursor position. */
export interface Tooltip {
  kind: "text" | "code";
  text: string;
}

/**
 * A position in the rendered preview: millimeters from the top-left of the
 * given 0-based page. Mirrors the coordinates `goToDefinition` accepts.
 */
export interface PreviewPosition {
  page: number;
  x: number;
  y: number;
}

/**
 * Common render request fields. `now` feeds `datetime.today()` and the PDF
 * creation timestamp; defaults to `Date.now()`. Pass a fixed value for
 * reproducible output.
 */
interface RenderRequestBase {
  input?: Inputs;
  now?: Instant;
}

export interface TypstRenderer extends Disposable {
  render(req: RenderRequestBase & { type: "pdf" }): RenderResult<Uint8Array>;
  render(req: RenderRequestBase & { type: "svg" }): RenderResult<string>;
  /** One SVG string per page, in page order. */
  render(req: RenderRequestBase & { type: "svg-pages" }): RenderResult<string[]>;
  /** One PNG per page, in page order, at `pixelPerPt` resolution (default 1). */
  render(req: RenderRequestBase & { type: "png-pages"; pixelPerPt?: number }): RenderResult<Uint8Array[]>;
  /**
   * Render a single 0-based page to PNG at `pixelPerPt` resolution (default 1,
   * i.e. one pixel per typographic point). Requires a prior render(); undefined
   * when the page is out of bounds.
   */
  renderPng(page: number, pixelPerPt?: number): Uint8Array | undefined;
  /**
   * Map a click at (xMm, yMm) millimeters from the top-left of the given
   * 0-based page to a source position ("go to definition"). Requires a prior
   * render(); returns undefined when the click hits nothing jumpable.
   */
  goToDefinition(page: number, xMm: number, yMm: number): DefinitionPosition | undefined;
  /**
   * Autocomplete suggestions for the source at `path`, at byte offset
   * `cursor`. `explicit` says whether the user explicitly asked for
   * completions (e.g. Ctrl+Space) rather than them popping up while typing.
   * Suggestions improve after a render() (labels, for instance, need one).
   */
  autocomplete(path: string, cursor: number, explicit?: boolean): Completions | undefined;
  /**
   * Hover tooltip for the source at `path`, at byte offset `cursor`.
   * Tooltips improve after a render() (values, for instance, need one).
   */
  tooltip(path: string, cursor: number): Tooltip | undefined;
  /**
   * Map a cursor position in the source at `path` to positions in the rendered
   * preview (the reverse of goToDefinition). Requires a prior render(); empty
   * when the cursor is not on text.
   */
  jumpFromCursor(path: string, cursor: number): PreviewPosition[];
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

  render(req: RenderRequestBase & { type: "pdf" }): RenderResult<Uint8Array>;
  render(req: RenderRequestBase & { type: "svg" }): RenderResult<string>;
  render(req: RenderRequestBase & { type: "svg-pages" }): RenderResult<string[]>;
  render(req: RenderRequestBase & { type: "png-pages"; pixelPerPt?: number }): RenderResult<Uint8Array[]>;
  render(req: RenderRequestBase & {
    type: "pdf" | "svg" | "svg-pages" | "png-pages";
    pixelPerPt?: number;
  }): RenderResult<Uint8Array | string | string[] | Uint8Array[]> {
    // Copy each wasm diagnostic into a plain object and free the handle so
    // consumers never have to manage wasm memory themselves.
    const diagnostics = this.#world.compile(req.input ?? {}, toEpochMillis(req.now)).map(d => {
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
          : req.type === "png-pages"
            ? this.#renderPngPages(req.pixelPerPt)
            : this.#world.render_svg();
    return { output, diagnostics };
  }

  #renderPngPages(pixelPerPt: number = 1): Uint8Array[] {
    const pages: Uint8Array[] = [];
    for (let page = 0; page < this.#world.getPageCount(); page++) {
      const png = this.#world.render_png(page, pixelPerPt);
      if (png !== undefined) pages.push(png);
    }
    return pages;
  }

  renderPng(page: number, pixelPerPt: number = 1): Uint8Array | undefined {
    return this.#world.render_png(page, pixelPerPt);
  }

  autocomplete(path: string, cursor: number, explicit: boolean = false): Completions | undefined {
    const result = this.#world.autocomplete(path, cursor, explicit);
    if (result === undefined) return undefined;
    try {
      return {
        from: result.from,
        completions: result.completions.map(c => {
          try {
            return {
              kind: c.kind as Completion["kind"],
              label: c.label,
              apply: c.apply,
              detail: c.detail,
            };
          } finally {
            c.free();
          }
        }),
      };
    } finally {
      result.free();
    }
  }

  tooltip(path: string, cursor: number): Tooltip | undefined {
    const tooltip = this.#world.tooltip(path, cursor);
    if (tooltip === undefined) return undefined;
    try {
      return {
        // Crosses the wasm boundary as a number; string is nicer to consume.
        kind: tooltip.kind === TooltipKind.Code ? ("code" as const) : ("text" as const),
        text: tooltip.text,
      };
    } finally {
      tooltip.free();
    }
  }

  jumpFromCursor(path: string, cursor: number): PreviewPosition[] {
    return this.#world.jumpFromCursor(path, cursor).map(pos => {
      try {
        return { page: pos.page, x: pos.x, y: pos.y };
      } finally {
        pos.free();
      }
    });
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
