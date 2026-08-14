// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

// The Pack DSL tab's overlay editor: a `<pre>` paint layer kept in sync with
// the plain `#dslText` textarea underneath it, plus a thin line-marker strip
// for loader notices. This is deliberately the minimum viable overlay, not a
// real editor component — the textarea stays the source of truth and the one
// thing wired to input events; everything here only *paints*. A real code
// editor (Monaco, backed by the LSP itself compiled to wasm) is the intended
// next slice, at which point this whole file is replaced rather than grown.

import { byId, clear, el } from "./dom.js";
import type { DslToken, PackNotice, StudioWasm } from "./types.js";

/**
 * One contiguous run of the source: either a classified token (carrying the
 * original `DslToken` it came from, so a hover lookup never has to guess
 * which occurrence of a repeated word a chunk belongs to) or an unclassified
 * gap (whitespace, brace/bracket delimiters — plain text either way).
 */
interface Chunk {
  text: string;
  token: DslToken | null;
}

/**
 * Split `source` into contiguous chunks covering every byte, using the
 * **byte** offsets `tokens` carry.
 *
 * `dsl_highlight` reports spans in UTF-8 byte offsets (matching the byte
 * spans every other wasm export in this app uses — see `pack_test_inspect`'s
 * doc comment), but a JavaScript string indexes by UTF-16 code unit. Slicing
 * `source.slice(start, end)` directly is only correct while the document is
 * pure ASCII; going through `TextEncoder`/`TextDecoder` keeps it correct for
 * any `.tclspec` comment or description that isn't.
 */
function byteChunks(source: string, tokens: DslToken[]): Chunk[] {
  const bytes = new TextEncoder().encode(source);
  const decoder = new TextDecoder();
  const sorted = [...tokens].sort((a, b) => a.start - b.start);
  const chunks: Chunk[] = [];
  let pos = 0;
  for (const tok of sorted) {
    if (tok.start < pos) continue; // defensive: tokens must not overlap
    if (tok.start > pos)
      chunks.push({ text: decoder.decode(bytes.subarray(pos, tok.start)), token: null });
    chunks.push({ text: decoder.decode(bytes.subarray(tok.start, tok.end)), token: tok });
    pos = tok.end;
  }
  if (pos < bytes.length) chunks.push({ text: decoder.decode(bytes.subarray(pos)), token: null });
  return chunks;
}

/** Classes worth a hover lookup — DSL vocabulary, not pack-author data. */
const HOVERABLE = new Set(["command-word", "property-flag"]);

/** Cap on a native tooltip's length — the browser renders it verbatim. */
const MAX_TITLE = 480;

/**
 * Repaint the DSL highlight overlay for `source` into `<pre id="dslHl">`.
 *
 * Hover is native browser tooltips (`title=`), set per span from `dsl_hover`
 * at render time rather than computed from mouse position — there is no
 * pixel-to-byte-offset caret math here, on purpose: every span already knows
 * its own byte offset, so asking the wasm module for that offset's hover is
 * direct instead of reconstructed from where the pointer happens to be.
 */
export function renderDslHighlight(wasm: StudioWasm, source: string): void {
  const pre = byId("dslHl");
  clear(pre);
  let tokens: DslToken[];
  try {
    tokens = JSON.parse(wasm.dsl_highlight(source)) as DslToken[];
  } catch {
    pre.appendChild(document.createTextNode(source));
    return;
  }
  for (const chunk of byteChunks(source, tokens)) {
    if (!chunk.token) {
      pre.appendChild(document.createTextNode(chunk.text));
      continue;
    }
    const span = el("span", { class: `tk-${chunk.token.class}`, text: chunk.text });
    if (HOVERABLE.has(chunk.token.class)) attachHoverTitle(wasm, source, chunk.token, span);
    pre.appendChild(span);
  }
  // A trailing newline in a textarea does not add a visible last line unless
  // something follows it — match that so the overlay's line count never
  // drifts one short of the textarea's.
  pre.appendChild(document.createTextNode("\n"));
}

/** Set `span`'s native tooltip from `dsl_hover`, swallowing any failure. */
function attachHoverTitle(
  wasm: StudioWasm,
  source: string,
  tok: DslToken,
  span: HTMLElement,
): void {
  try {
    const hover = JSON.parse(wasm.dsl_hover(source, tok.start)) as {
      found: boolean;
      title: string;
      body: string;
    };
    if (!hover.found) return;
    let text = hover.body ? `${hover.title}\n\n${hover.body}` : hover.title;
    if (text.length > MAX_TITLE) text = `${text.slice(0, MAX_TITLE)}…`;
    span.title = text;
  } catch {
    // No hover is not an error worth surfacing on a paint pass.
  }
}

/**
 * Repaint the validate-notice line markers into `<div id="dslMarkers">`.
 *
 * One tinted strip per source line carrying a notice, positioned by
 * `line-height * (line - 1)` against the shared metrics `syncDslMetrics`
 * reads off the live textarea — no gutter column, no per-character mapping,
 * just "this line has something to say", with the notice text as the
 * marker's native tooltip.
 */
export function renderDslMarkers(notices: PackNotice[]): void {
  const layer = byId("dslMarkers");
  clear(layer);
  if (notices.length === 0) return;
  const lineHeight = dslLineHeightPx();
  const byLine = new Map<number, string[]>();
  for (const notice of notices) {
    const reasons = byLine.get(notice.line) ?? [];
    reasons.push(notice.reason);
    byLine.set(notice.line, reasons);
  }
  for (const [line, reasons] of byLine) {
    layer.appendChild(
      el("div", {
        class: "dsl-marker-line",
        style: `top:${(line - 1) * lineHeight}px;height:${lineHeight}px`,
        title: reasons.join("\n"),
      }),
    );
  }
}

/** The textarea's computed line height in pixels, read once per repaint. */
function dslLineHeightPx(): number {
  const area = byId<HTMLTextAreaElement>("dslText");
  const parsed = Number.parseFloat(getComputedStyle(area).lineHeight);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : 18;
}

/** Keep the overlay layers scrolled with the textarea driving them. */
export function syncDslScroll(): void {
  const area = byId<HTMLTextAreaElement>("dslText");
  const pre = byId("dslHl");
  const markers = byId("dslMarkers");
  pre.scrollTop = area.scrollTop;
  pre.scrollLeft = area.scrollLeft;
  markers.scrollTop = area.scrollTop;
}
