import { createTypstRenderer } from "@brief-jetzt/wasm-typst";

// Fonts come from an npm font package (Fontsource itself only ships
// woff/woff2, which typst's font parser can't read — this one ships TTF).
// Vite's `?url` import emits the file as an asset and gives us its URL.
import interRegular from "@expo-google-fonts/inter/400Regular/Inter_400Regular.ttf?url";
import interItalic from "@expo-google-fonts/inter/400Regular_Italic/Inter_400Regular_Italic.ttf?url";
import interBold from "@expo-google-fonts/inter/700Bold/Inter_700Bold.ttf?url";

const editor = document.querySelector<HTMLTextAreaElement>("#editor")!;
const pagesEl = document.querySelector<HTMLDivElement>("#pages")!;
const statusEl = document.querySelector<HTMLElement>("#status")!;

async function loadFont(url: string) {
  const data = new Uint8Array(await (await fetch(url)).arrayBuffer());
  return { path: url.split("/").at(-1)!, data };
}

const fonts = await Promise.all(
  [interRegular, interItalic, interBold].map(loadFont),
);

const renderer = createTypstRenderer({
  fonts,
  sources: { "main.typ": editor.value },
});

function rerender() {
  const { output, diagnostics } = renderer.render({ type: "svg-pages" });
  statusEl.textContent =
    diagnostics.map(d => `${d.severity}: ${d.path ?? "?"}:${d.line ?? "?"}: ${d.message}`).join("\n")
    || `ok — ${renderer.getPageCount()} page(s)`;
  pagesEl.replaceChildren(
    ...output.map((svg, i) => {
      const div = document.createElement("div");
      div.className = "page";
      div.dataset.page = String(i);
      div.innerHTML = svg;
      return div;
    }),
  );
}

// Incremental updates: only the changed source is re-synced; the Rust side
// reuses the existing parse tree and reparses just the changed span.
editor.addEventListener("input", () => {
  renderer.updateSource("main.typ", editor.value);
  rerender();
});

// Go to definition: click on a rendered page → jump the editor cursor to the
// matching source position.
pagesEl.addEventListener("click", (e) => {
  const pageEl = (e.target as Element).closest<HTMLElement>(".page");
  if (!pageEl) return;
  const page = Number(pageEl.dataset.page);
  const size = renderer.getPageSize(page);
  if (!size) return;
  // Map the click from CSS pixels to millimeters on the page.
  const rect = pageEl.querySelector("svg")!.getBoundingClientRect();
  const xMm = ((e.clientX - rect.left) / rect.width) * size.width;
  const yMm = ((e.clientY - rect.top) / rect.height) * size.height;
  const pos = renderer.goToDefinition(page, xMm, yMm);
  if (!pos || pos.path !== "main.typ") return;
  const offset = byteToUtf16Offset(editor.value, pos.cursor);
  editor.focus();
  editor.setSelectionRange(offset, offset);
});

/** The wasm side reports byte offsets; textarea selections are UTF-16. */
function byteToUtf16Offset(text: string, byteOffset: number): number {
  const bytes = new TextEncoder().encode(text).subarray(0, byteOffset);
  return new TextDecoder().decode(bytes).length;
}

rerender();
