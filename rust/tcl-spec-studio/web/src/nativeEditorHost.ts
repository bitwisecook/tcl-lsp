// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

import type {
  EditorHost,
  EditorHostOptions,
  MonacoHostModule,
  OutputSurfaceSpec,
  SurfaceSpec,
} from "./editorHost.js";

declare const __SPEC_STUDIO_FRONTEND_VERSION__: string;
export const buildVersion = __SPEC_STUDIO_FRONTEND_VERSION__;

type Surface = "dsl" | "sample" | "rust" | "stub";

interface HostUpdate {
  type: "tclSpecStudioHostUpdate";
  surface: Surface;
  text: string;
}

function openButton(container: HTMLElement, surface: Surface, label: string): void {
  container.replaceChildren();
  const button = document.createElement("button");
  button.type = "button";
  button.className = "primary native-editor-button";
  button.textContent = `Open ${label} in editor`;
  button.addEventListener("click", () => {
    window.__tclSpecStudioHost?.postMessage({ type: "openSurface", surface });
  });
  container.append(button);
}

function outputText(spec: OutputSurfaceSpec): string {
  return spec.fallback.textContent ?? "";
}

export async function mountEditors(options: EditorHostOptions): Promise<EditorHost> {
  const bridge = window.__tclSpecStudioHost;
  if (!bridge) throw new Error("the native editor bridge was not installed");

  openButton(options.dsl.container, "dsl", "Pack DSL");
  openButton(options.sample.container, "sample", "Test Tcl");
  openButton(options.rust.container, "rust", "generated Rust");
  openButton(options.stub.container, "stub", "generated stub");

  const texts: Record<Surface, string> = {
    dsl: options.dsl.textarea.value,
    sample: options.sample.textarea.value,
    rust: outputText(options.rust),
    stub: outputText(options.stub),
  };

  const publish = (surface: Surface, text: string): void => {
    if (texts[surface] === text) return;
    texts[surface] = text;
    bridge.postMessage({ type: "surfaceUpdate", surface, text });
  };

  const acceptEdit = (spec: SurfaceSpec, surface: "dsl" | "sample", text: string): void => {
    if (texts[surface] === text) return;
    texts[surface] = text;
    spec.textarea.value = text;
    spec.onChange(text);
  };

  window.addEventListener("message", (event: MessageEvent<unknown>) => {
    const update = event.data as Partial<HostUpdate> | undefined;
    if (update?.type !== "tclSpecStudioHostUpdate" || typeof update.text !== "string") return;
    if (update.surface === "dsl") acceptEdit(options.dsl, "dsl", update.text);
    if (update.surface === "sample") acceptEdit(options.sample, "sample", update.text);
  });

  bridge.postMessage({ type: "surfaceUpdate", surface: "dsl", text: texts.dsl });
  bridge.postMessage({ type: "surfaceUpdate", surface: "sample", text: texts.sample });
  bridge.postMessage({ type: "surfaceUpdate", surface: "rust", text: texts.rust });
  bridge.postMessage({ type: "surfaceUpdate", surface: "stub", text: texts.stub });
  bridge.postMessage({ type: "studioReady" });
  options.report("using the editor host's native Tcl language support", "ok");

  return {
    setDslText: (text) => publish("dsl", text),
    setSampleText: (text) => publish("sample", text),
    setRustText: (text) => publish("rust", text),
    setStubText: (text) => publish("stub", text),
    setDialect: (dialect) => bridge.postMessage({ type: "dialectUpdate", dialect }),
    layout: () => undefined,
    lspReady: true,
  };
}

const moduleShape: MonacoHostModule = { buildVersion, mountEditors };
void moduleShape;
