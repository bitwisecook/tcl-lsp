// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// The in-report architecture / topology editor. Lets the reader adjust the
// estate topology (device roles/tiers/links, network zones, DNS zones, enrichment
// maps) via the manifest DSL, re-run detection live (through the embedded engine
// wasm when the query console is present), re-render the diagram, and persist the
// manifest per-report in localStorage (keyed by the report's UUID). The manifest
// can be exported as a `.tcl` to regenerate a pinned report.

import { DSL_GUIDE } from "./guide";

interface Architecture {
  mermaid?: string;
  deviceCount?: number;
}
interface Model {
  devices?: unknown[];
  architecture?: Architecture;
}

interface WasmNs {
  (bytes: Uint8Array): Promise<unknown>;
  build_architecture?(devicesJson: string, manifest: string): string;
}
interface MermaidNs {
  render(id: string, def: string): Promise<{ svg: string }>;
}

function win() {
  return window as unknown as { wasm_bindgen?: WasmNs; mermaid?: MermaidNs };
}

function readModel(): Model | null {
  const el = document.getElementById("f5-model");
  if (!el || !el.textContent) return null;
  try {
    return JSON.parse(el.textContent) as Model;
  } catch {
    return null;
  }
}

function reportId(): string {
  const id = document.documentElement.getAttribute("data-report-id");
  return id && id.trim() ? id.trim() : "default";
}

function b64ToBytes(b64: string): Uint8Array {
  const bin = atob(b64.trim());
  const u = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) u[i] = bin.charCodeAt(i);
  return u;
}

let wasmReady: Promise<void> | null = null;
function initWasm(): Promise<void> | null {
  const w = win();
  const payload = document.getElementById("f5-wasm");
  if (typeof w.wasm_bindgen !== "function" || !payload || !payload.textContent) return null;
  if (!wasmReady) {
    wasmReady = Promise.resolve(w.wasm_bindgen(b64ToBytes(payload.textContent))).then(() => undefined);
  }
  return wasmReady;
}

export function initArchEditor(): void {
  const model = readModel();
  if (!model || !model.architecture) return;
  // Attach near the architecture diagram when present (Rust report), else above
  // the manual panel (present in both generators), else before the first device.
  const anchor =
    document.getElementById("archDiagram") ||
    document.querySelector<HTMLElement>(".f5q-manual") ||
    document.querySelector<HTMLElement>(".device");
  if (!anchor) return;
  const insertAfter = anchor.id === "archDiagram";

  const key = `f5arch:${reportId()}`;
  const saved = safeGet(key);

  const section = document.createElement("details");
  section.className = "arch-editor";
  section.innerHTML = `
    <summary>✏️ Edit architecture &amp; topology (DSL)</summary>
    <p class="arch-editor-hint">Describe the estate — device roles/tiers, links, network zones,
      DNS zones and enrichment maps — in the manifest DSL. <b>Apply</b> re-runs detection and
      redraws the diagram (needs the query console); <b>Export</b> saves a <code>.tcl</code> you can
      feed back into the generator to pin it. Saved per report in this browser.</p>
    <textarea class="arch-editor-ta" spellcheck="false" rows="12"></textarea>
    <div class="arch-editor-actions">
      <button type="button" class="arch-apply">Apply</button>
      <button type="button" class="arch-export">Export .tcl ↓</button>
      <button type="button" class="arch-reset">Reset</button>
      <span class="arch-editor-status"></span>
    </div>
    <details class="arch-editor-guide"><summary>DSL reference</summary><pre></pre></details>`;
  anchor.parentNode?.insertBefore(section, insertAfter ? anchor.nextSibling : anchor);

  const ta = section.querySelector<HTMLTextAreaElement>(".arch-editor-ta")!;
  const status = section.querySelector<HTMLElement>(".arch-editor-status")!;
  ta.value = saved ?? "";
  ta.placeholder = DSL_GUIDE;
  section.querySelector<HTMLElement>(".arch-editor-guide pre")!.textContent = DSL_GUIDE;

  const setStatus = (m: string) => {
    status.textContent = m;
  };

  section.querySelector<HTMLButtonElement>(".arch-reset")!.addEventListener("click", () => {
    ta.value = "";
    safeSet(key, "");
    setStatus("cleared");
  });

  section.querySelector<HTMLButtonElement>(".arch-export")!.addEventListener("click", () => {
    const text = ta.value.trim();
    if (!text) {
      setStatus("nothing to export");
      return;
    }
    const url = URL.createObjectURL(new Blob([text + "\n"], { type: "text/plain" }));
    const a = document.createElement("a");
    a.href = url;
    a.download = "architecture.tcl";
    document.body.appendChild(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  });

  section.querySelector<HTMLButtonElement>(".arch-apply")!.addEventListener("click", () => {
    const manifest = ta.value;
    safeSet(key, manifest);
    setStatus("saved");
    void applyManifest(model, manifest, setStatus);
  });

  // A previously-saved manifest re-applies on load so the diagram reflects it.
  if (saved && saved.trim()) void applyManifest(model, saved, setStatus);
}

async function applyManifest(model: Model, manifest: string, setStatus: (m: string) => void): Promise<void> {
  const w = win();
  if (typeof w.wasm_bindgen !== "function" || !w.wasm_bindgen.build_architecture) {
    setStatus("saved — regenerate the report (or open the query console) to redraw");
    return;
  }
  const ready = initWasm();
  if (!ready) {
    setStatus("saved — engine unavailable; regenerate to apply");
    return;
  }
  try {
    await ready;
    const archJson = w.wasm_bindgen.build_architecture!(JSON.stringify(model.devices ?? []), manifest);
    const arch = JSON.parse(archJson) as Architecture;
    model.architecture = arch;
    await redrawDiagram(arch);
    setStatus("applied — diagram updated");
  } catch (e) {
    setStatus("error: " + (e instanceof Error ? e.message : String(e)));
  }
}

async function redrawDiagram(arch: Architecture): Promise<void> {
  const host = document.getElementById("archDiagram");
  const mermaid = win().mermaid;
  if (!host || !arch.mermaid || !mermaid) return;
  const res = await mermaid.render("archDiagram-svg-" + Date.now().toString(36), arch.mermaid);
  host.innerHTML = res.svg;
}

function safeGet(k: string): string | null {
  try {
    return localStorage.getItem(k);
  } catch {
    return null;
  }
}
function safeSet(k: string, v: string): void {
  try {
    localStorage.setItem(k, v);
  } catch {
    /* storage unavailable */
  }
}
