// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
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
  /** IDE-issued revision of the exact DSL/sample document image. */
  sourceRevision: number;
  /** Present only when acknowledging or refusing a browser DSL write. */
  webGeneration?: number;
  accepted?: boolean;
  retryable?: boolean;
  /** True while this readable DSL image is not yet the server-published source. */
  publicationPending?: boolean;
  /** Client-local monotonic correlation for publication attempts on one panel. */
  publicationAttempt?: number | null;
}

type GeneratedOutput = { text: string; sourceBasis: string | null };

function openButton(container: HTMLElement, surface: Surface, label: string): void {
  container.replaceChildren();
  const button = document.createElement("button");
  button.type = "button";
  button.className = "primary native-editor-button";
  button.textContent = `Open ${label} beside Studio`;
  button.addEventListener("click", () => {
    window.__tclSpecStudioHost?.postMessage({ type: "openSurface", surface });
  });
  container.append(button);
}

function outputText(spec: OutputSurfaceSpec): string {
  return spec.source.textContent ?? "";
}

/** Delegate every code surface to the IDE's ordinary file-editor tabs. */
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

  let dslSourceRevision: number | undefined;
  let acknowledgedDslSource: string | undefined;
  let pendingDslPublication: { revision: number; text: string; attempt: number } | undefined;
  let latestDslPublicationAttempt = -1;
  let sampleSourceRevision: number | undefined;
  let webGeneration = 0;
  let pendingDsl: { generation: number; text: string } | undefined;
  let pendingSample: { generation: number; text: string } | undefined;
  const deferredOutputs: Partial<Record<"rust" | "stub", GeneratedOutput>> = {
    rust: { text: texts.rust, sourceBasis: options.rust.sourceBasis },
    stub: { text: texts.stub, sourceBasis: options.stub.sourceBasis },
  };
  const publishedOutputs: Partial<Record<"rust" | "stub", {
    revision: number;
    text: string;
    sourceBasis: string | null;
  }>> = {};

  const postDsl = (text: string, generation: number, bootstrap = false): void => {
    pendingDsl = { generation, text };
    bridge.postMessage({
      type: "surfaceUpdate",
      surface: "dsl",
      text,
      expectedSourceRevision: dslSourceRevision,
      webGeneration: generation,
      bootstrap,
    });
  };

  const postSample = (text: string, generation: number, bootstrap = false): void => {
    pendingSample = { generation, text };
    bridge.postMessage({
      type: "surfaceUpdate",
      surface: "sample",
      text,
      expectedSourceRevision: sampleSourceRevision,
      webGeneration: generation,
      bootstrap,
    });
  };

  const postGeneratedOutput = (surface: "rust" | "stub", output: GeneratedOutput): void => {
    const sourceRevision = dslSourceRevision;
    if (
      sourceRevision === undefined ||
      pendingDsl ||
      pendingDslPublication ||
      acknowledgedDslSource === undefined ||
      output.sourceBasis === null ||
      output.sourceBasis !== acknowledgedDslSource
    ) return;
    const previouslyPosted = publishedOutputs[surface];
    if (previouslyPosted?.revision === sourceRevision && previouslyPosted.text === output.text &&
        previouslyPosted.sourceBasis === output.sourceBasis) return;
    bridge.postMessage({
      type: "surfaceUpdate",
      surface,
      text: output.text,
      basedOnDslRevision: sourceRevision,
    });
    publishedOutputs[surface] = { revision: sourceRevision, text: output.text, sourceBasis: output.sourceBasis };
  };

  const publishDeferredOutputs = (): void => {
    if (pendingDsl || pendingDslPublication || dslSourceRevision === undefined || acknowledgedDslSource === undefined) return;
    for (const surface of ["rust", "stub"] as const) {
      const output = deferredOutputs[surface];
      if (output === undefined) continue;
      postGeneratedOutput(surface, output);
    }
  };

  const publish = (
    surface: Surface,
    text: string,
    sourceBasis: string | null = null,
  ): void => {
    const unchanged = texts[surface] === text;
    if (unchanged && (surface === "dsl" || surface === "sample")) return;
    texts[surface] = text;
    if (surface === "dsl") {
      postDsl(text, ++webGeneration);
    } else if (surface === "sample") {
      postSample(text, ++webGeneration);
    } else {
      const output = { text, sourceBasis };
      deferredOutputs[surface] = output;
      if (pendingDsl || pendingDslPublication || dslSourceRevision === undefined || acknowledgedDslSource === undefined) {
        return;
      }
      if (sourceBasis !== acknowledgedDslSource) return;
      postGeneratedOutput(surface, output);
    }
  };

  const acceptEdit = (
    spec: SurfaceSpec,
    surface: "dsl" | "sample",
    text: string,
    update: HostUpdate,
  ): void => {
    const currentRevision = surface === "dsl" ? dslSourceRevision : sampleSourceRevision;
    if (currentRevision !== undefined && update.sourceRevision < currentRevision) return;
    if (surface === "dsl" && typeof update.publicationAttempt === "number" &&
        Number.isSafeInteger(update.publicationAttempt)) {
      if (update.publicationAttempt < latestDslPublicationAttempt) return;
      latestDslPublicationAttempt = update.publicationAttempt;
    }
    if (surface === "dsl") {
      const previousRevision = dslSourceRevision;
      const previousSource = acknowledgedDslSource;
      dslSourceRevision = update.sourceRevision;
      acknowledgedDslSource = text;
      const attempt = typeof update.publicationAttempt === "number" && Number.isSafeInteger(update.publicationAttempt)
        ? update.publicationAttempt
        : -1;
      if (update.publicationPending === true) {
        pendingDslPublication = { revision: update.sourceRevision, text, attempt };
        delete publishedOutputs.rust;
        delete publishedOutputs.stub;
      } else if (update.publicationPending === false) {
        const pending = pendingDslPublication;
        const sameAttemptReady = pending !== undefined && attempt === pending.attempt &&
          pending.revision === update.sourceRevision && pending.text === text;
        const newerImageReady = pending !== undefined && attempt > pending.attempt &&
          update.sourceRevision > pending.revision;
        if (pending === undefined || sameAttemptReady || newerImageReady) pendingDslPublication = undefined;
      } else if (previousRevision !== update.sourceRevision || previousSource !== text) {
        // Older bridge hosts have no publication handshake. Preserve their
        // accepted-write behavior only when no newer pending lease exists.
        if (pendingDslPublication === undefined && update.accepted !== true) {
          pendingDslPublication = { revision: update.sourceRevision, text, attempt };
        }
      }
      if (typeof update.webGeneration === "number") {
        if (pendingDsl && pendingDsl.generation !== update.webGeneration) {
          // An older queued acknowledgement advances the host revision but
          // cannot adopt or overwrite the newer page edit.
          if (update.accepted === false && update.retryable === false) {
            pendingDsl = undefined;
          } else {
            postDsl(pendingDsl.text, pendingDsl.generation);
            return;
          }
        }
        if (pendingDsl?.generation === update.webGeneration && update.accepted === false) {
          if (update.retryable !== false) {
            // The IDE changed since this page write was based. Keep the newer
            // page text and retry it against the newly issued host revision.
            postDsl(pendingDsl.text, pendingDsl.generation);
            return;
          }
          // The host source owner refused the write (for example, an unowned
          // generated pathname). Adopt its actual source rather than retrying
          // a write the owner has explicitly declined.
          pendingDsl = undefined;
        }
        if (pendingDsl?.generation === update.webGeneration && update.accepted !== false) {
          pendingDsl = undefined;
        }
      } else if (pendingDsl) {
        // A real IDE edit wins over a queued page write; the generated outputs
        // will be recomputed from this newly accepted source image.
        pendingDsl = undefined;
      }
    } else {
      sampleSourceRevision = update.sourceRevision;
      if (typeof update.webGeneration === "number") {
        if (pendingSample && pendingSample.generation !== update.webGeneration) {
          if (update.accepted === false && update.retryable === false) {
            pendingSample = undefined;
          } else {
            postSample(pendingSample.text, pendingSample.generation);
            return;
          }
        }
        if (pendingSample?.generation === update.webGeneration && update.accepted === false) {
          if (update.retryable !== false) {
            postSample(pendingSample.text, pendingSample.generation);
            return;
          }
          pendingSample = undefined;
        }
        if (pendingSample?.generation === update.webGeneration && update.accepted !== false) {
          pendingSample = undefined;
        }
      } else if (pendingSample) {
        pendingSample = undefined;
      }
    }
    if (texts[surface] === text) {
      publishDeferredOutputs();
      return;
    }
    if (surface === "dsl") {
      // These projections were computed from the previous page source. A real
      // host-authored replacement must trigger fresh projections first.
      delete deferredOutputs.rust;
      delete deferredOutputs.stub;
    }
    texts[surface] = text;
    spec.textarea.value = text;
    spec.onChange(text);
    publishDeferredOutputs();
  };

  // Only the IDE bridge may drive these surfaces. VS Code's webview host posts
  // through the frame embedding the studio, and that frame's origin is a
  // per-webview `vscode-webview://<id>` rather than a constant, so it is checked
  // against this document's own origin and against the embedder's identity;
  // JetBrains' JCEF instead dispatches a synthetic event straight into the
  // document, which carries an empty origin. Any other window is ignored.
  const hostFrame = window.parent === window ? null : window.parent;
  window.addEventListener("message", (event: MessageEvent<unknown>) => {
    const fromHost =
      event.origin === "" ||
      event.origin === window.location.origin ||
      (hostFrame !== null && event.source === hostFrame);
    if (!fromHost) return;
    const update = event.data as Partial<HostUpdate> | undefined;
    if (
      update?.type !== "tclSpecStudioHostUpdate" ||
      typeof update.text !== "string" ||
      typeof update.sourceRevision !== "number" ||
      !Number.isSafeInteger(update.sourceRevision) ||
      update.sourceRevision < 0 ||
      (update.publicationAttempt !== undefined && update.publicationAttempt !== null &&
        (typeof update.publicationAttempt !== "number" || !Number.isSafeInteger(update.publicationAttempt) ||
          update.publicationAttempt < 0)) ||
      (update.surface === "dsl" && update.publicationPending === true &&
        (typeof update.publicationAttempt !== "number" || !Number.isSafeInteger(update.publicationAttempt) ||
          update.publicationAttempt < 0))
    ) return;
    if (update.surface === "dsl") acceptEdit(options.dsl, "dsl", update.text, update as HostUpdate);
    if (update.surface === "sample") acceptEdit(options.sample, "sample", update.text, update as HostUpdate);
  });

  for (const surface of Object.keys(texts) as Surface[]) {
    if (surface === "dsl") {
      postDsl(texts.dsl, ++webGeneration, true);
    } else if (surface === "sample") {
      postSample(texts.sample, ++webGeneration, true);
    }
  }
  // The selection the studio mounts with, not just the ones it changes to.
  //
  // `setDialect` is called from the picker's change handler alone, so without
  // this the host never hears the initial dialect — the default, or one
  // restored from a previous session — and the sample is materialised as
  // `test.tcl` and analysed as generic Tcl until the user touches the selector.
  // The Monaco host has no equivalent gap: it sets the language id when it
  // builds the model. Sent with the opening surface texts because it is the
  // same kind of fact: the state the surfaces start in.
  bridge.postMessage({ type: "dialectUpdate", dialect: options.dialect });
  options.report("using the IDE's native file editor beside Spec Studio", "ok");

  return {
    setDslText: (text) => publish("dsl", text),
    setSampleText: (text) => publish("sample", text),
    setRustText: (text, sourceBasis) => publish("rust", text, sourceBasis),
    setStubText: (text, sourceBasis) => publish("stub", text, sourceBasis),
    setDialect: (dialect) => bridge.postMessage({ type: "dialectUpdate", dialect }),
    layout: () => undefined,
    lspReady: true,
  };
}

const moduleShape: MonacoHostModule = { buildVersion, mountEditors };
void moduleShape;
