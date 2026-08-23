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

// Monaco, driven by the real Tcl language server.
//
// This module is the studio's *lazy* half: it is the entry point of a second
// bundle (`assets/monaco-host.js`), imported on demand the first time the
// author opens an editor tab. Everything Monaco — some 2.5 MB of it — lives
// behind that boundary, so a visitor who only browses the registry never
// downloads an editor. If it cannot load, the editor is explicitly unavailable.
//
// The bridge below is deliberately thin. Every provider forwards to
// `LspClient` and translates the reply; not one of them decides what a word
// means. Semantic colouring, hovers, completions, formatting, and diagnostics
// are all the same answers the native server gives VS Code — which is the whole
// point of compiling the server to wasm rather than reimplementing a tenth of
// it in TypeScript.

import * as monaco from "monaco-editor/editor/editor.api.js";
// `editor.api` exposes provider registration but deliberately does not install
// the editor controllers which consume those providers. Keep this list
// explicit so the lazy bundle contains only the LSP-backed features Studio
// offers, while still making registration do something in the actual editor.
import "monaco-editor/features/format/register.js";
import "monaco-editor/features/hover/register.js";
import "monaco-editor/features/parameterHints/register.js";
import "monaco-editor/features/semanticTokens/register.js";
import "monaco-editor/features/suggest/register.js";
import "monaco-editor/languages/definitions/rust/register.js";
import type {
  CompletionItem as LspCompletionItem,
  CompletionList as LspCompletionList,
  Diagnostic as LspDiagnostic,
  MarkupContent,
  TextEdit as LspTextEdit,
} from "vscode-languageserver-protocol";

import type {
  EditorHost,
  EditorHostOptions,
  OutputSurfaceSpec,
  SurfaceSpec,
} from "./editorHost.js";
import { LspClient, startLspClient } from "./lspClient.js";
import { registerTclGrammar } from "./textmateHost.js";

/** The Monaco language id both surfaces' models are registered under. */
const SPECTCL_LANGUAGE = "spectcl";
const TCL_LANGUAGE = "tcl";

/** Build provenance checked by the controller before this chunk is mounted. */
declare const __SPEC_STUDIO_FRONTEND_VERSION__: string;
export const buildVersion = __SPEC_STUDIO_FRONTEND_VERSION__;

/**
 * Virtual document URIs.
 *
 * They are never read from disk, but they deliberately use the `file:` scheme:
 * the native server's document pipeline derives paths, extensions, and dialect
 * hints from file URIs. Monaco's usual `inmemory:` scheme lets `initialize`
 * succeed while later semantic-token and hover requests fall off that path,
 * leaving a healthy worker apparently doing nothing.
 */
const DSL_URI = "file:///__tcl_spec_studio__/pack.tclspec";
const SAMPLE_URI = "file:///__tcl_spec_studio__/sample.tcl";
const RUST_URI = "file:///__tcl_spec_studio__/command.rs";
const STUB_URI = "file:///__tcl_spec_studio__/stubs.tcl";
const EXPLORER_URI = "file:///__tcl_compiler_explorer__/source.tcl";

/** How long keystrokes settle before the change is pushed on. */
const SETTLE_MS = 120;

/** The readiness probe must degrade rather than leave the editor mount hung. */
const LSP_READINESS_TIMEOUT_MS = 10_000;

/** Whether a Monaco instance has already had the languages registered. */
let languagesRegistered = false;

/** Register both language ids and a minimal bracket/comment configuration. */
function registerLanguages(): void {
  if (languagesRegistered) return;
  languagesRegistered = true;
  for (const id of [SPECTCL_LANGUAGE, TCL_LANGUAGE]) {
    monaco.languages.register({
      id,
      extensions: id === SPECTCL_LANGUAGE ? [".tclspec"] : [".tcl", ".tm", ".test"],
      aliases: [id],
    });
    // Everything *semantic* comes from the server. This configuration covers
    // only what the server is not asked about: which characters pair up, and
    // what a comment looks like to the "toggle comment" keystroke.
    monaco.languages.setLanguageConfiguration(id, {
      comments: { lineComment: "#" },
      brackets: [
        ["{", "}"],
        ["[", "]"],
      ],
      autoClosingPairs: [
        { open: "{", close: "}" },
        { open: "[", close: "]" },
        { open: '"', close: '"' },
      ],
      surroundingPairs: [
        { open: "{", close: "}" },
        { open: "[", close: "]" },
        { open: '"', close: '"' },
      ],
    });
  }
}

/** LSP severity (1..4) → Monaco marker severity. */
function markerSeverity(severity: number | undefined): monaco.MarkerSeverity {
  switch (severity) {
    case 1:
      return monaco.MarkerSeverity.Error;
    case 2:
      return monaco.MarkerSeverity.Warning;
    case 3:
      return monaco.MarkerSeverity.Info;
    default:
      return monaco.MarkerSeverity.Hint;
  }
}

/** LSP diagnostics → Monaco markers, 0-based lines becoming 1-based. */
function toMarkers(items: LspDiagnostic[]): monaco.editor.IMarkerData[] {
  return items.map((d) => ({
    severity: markerSeverity(d.severity),
    // LSP 3.18 widened a diagnostic's message to markup; Monaco's marker takes
    // plain text, so a markup message contributes its raw value.
    message: typeof d.message === "string" ? d.message : d.message.value,
    code: typeof d.code === "number" ? String(d.code) : (d.code ?? undefined),
    source: d.source ?? "tcl-lsp",
    startLineNumber: d.range.start.line + 1,
    startColumn: d.range.start.character + 1,
    endLineNumber: d.range.end.line + 1,
    endColumn: d.range.end.character + 1,
  }));
}

/** An LSP markup/plain hover body as Monaco markdown. */
function toMarkdown(value: string | MarkupContent | { language: string; value: string }): string {
  if (typeof value === "string") return value;
  if ("kind" in value) return value.value;
  return `\`\`\`${value.language}\n${value.value}\n\`\`\``;
}

/** An LSP text edit as a Monaco single-edit operation. */
function toEditOperation(edit: LspTextEdit): monaco.languages.TextEdit {
  return {
    range: {
      startLineNumber: edit.range.start.line + 1,
      startColumn: edit.range.start.character + 1,
      endLineNumber: edit.range.end.line + 1,
      endColumn: edit.range.end.character + 1,
    },
    text: edit.newText,
  };
}

/** The completion items out of either reply shape. */
function completionItems(
  reply: LspCompletionItem[] | LspCompletionList | null,
): LspCompletionItem[] {
  if (!reply) return [];
  return Array.isArray(reply) ? reply : reply.items;
}

/** Zero-based LSP position of `needle` in `text`, when present. */
function positionOf(text: string, needle: string): { line: number; character: number } | null {
  const offset = text.indexOf(needle);
  if (offset < 0) return null;
  const before = text.slice(0, offset);
  const lines = before.split("\n");
  return { line: lines.length - 1, character: lines[lines.length - 1]?.length ?? 0 };
}

/**
 * Register every provider that forwards to the server.
 *
 * Registered once per language id per page: Monaco providers are global to the
 * `monaco.languages` namespace, not per editor, so both models are served by
 * one registration each and the model's own URI decides which document the
 * request names.
 */
function registerProviders(client: LspClient): void {
  const legend: monaco.languages.SemanticTokensLegend = {
    // Semantic-token legends contain token *identifiers*, not TextMate
    // scopes. Monaco's built-in themes already colour the standard LSP names
    // (`keyword`, `variable`, `function`, ...). Replacing them with scopes such
    // as `entity.name.function` leaves Monaco unable to match any rule and
    // turns the whole Tcl document back into generic text.
    tokenTypes: [...client.legend.tokenTypes],
    tokenModifiers: [...client.legend.tokenModifiers],
  };

  for (const language of [SPECTCL_LANGUAGE, TCL_LANGUAGE]) {
    monaco.languages.registerDocumentSemanticTokensProvider(language, {
      getLegend: () => legend,
      provideDocumentSemanticTokens: async (model) => {
        // The headless deployment test can turn off Monaco's consumer without
        // touching the transport.  That mutation proves the readiness probe
        // below is an explicit LSP request, rather than an accidental pass
        // because Monaco happened to schedule this provider first.
        if (
          (
            globalThis as typeof globalThis & {
              __tclSpecStudioTestSuppressMonacoSemanticProvider?: boolean;
            }
          ).__tclSpecStudioTestSuppressMonacoSemanticProvider
        ) {
          return null;
        }
        const reply = await client.semanticTokens(model.uri.toString());
        if (!reply) return null;
        return { data: Uint32Array.from(reply.data), resultId: reply.resultId };
      },
      releaseDocumentSemanticTokens: () => {
        // The server is stateless per request here — nothing to release.
      },
    });

    monaco.languages.registerHoverProvider(language, {
      provideHover: async (model, position) => {
        const hover = await client.hover(
          model.uri.toString(),
          position.lineNumber - 1,
          position.column - 1,
        );
        if (!hover?.contents) return null;
        const parts = Array.isArray(hover.contents) ? hover.contents : [hover.contents];
        const contents = parts.map((part) => ({ value: toMarkdown(part) }));
        return contents.length ? { contents } : null;
      },
    });

    monaco.languages.registerCompletionItemProvider(language, {
      triggerCharacters: ["$", "-", ":"],
      provideCompletionItems: async (model, position) => {
        const reply = await client.completion(
          model.uri.toString(),
          position.lineNumber - 1,
          position.column - 1,
        );
        const word = model.getWordUntilPosition(position);
        const range = {
          startLineNumber: position.lineNumber,
          endLineNumber: position.lineNumber,
          startColumn: word.startColumn,
          endColumn: word.endColumn,
        };
        return {
          suggestions: completionItems(reply).map((item) => ({
            label: item.label,
            // LSP's `CompletionItemKind` is 1-based over the same ordered set
            // Monaco enumerates from 0, so the offset is the whole conversion.
            kind: (item.kind ?? 1) - 1,
            insertText: item.insertText ?? item.label,
            detail: item.detail,
            documentation: item.documentation ? toMarkdown(item.documentation) : undefined,
            sortText: item.sortText,
            filterText: item.filterText,
            range,
          })),
        };
      },
    });

    monaco.languages.registerSignatureHelpProvider(language, {
      signatureHelpTriggerCharacters: [" "],
      provideSignatureHelp: async (model, position) => {
        const help = await client.signatureHelp(
          model.uri.toString(),
          position.lineNumber - 1,
          position.column - 1,
        );
        if (!help) return null;
        return {
          value: {
            signatures: help.signatures.map((signature) => ({
              label: signature.label,
              documentation: signature.documentation
                ? toMarkdown(signature.documentation)
                : undefined,
              parameters: (signature.parameters ?? []).map((parameter) => ({
                label: parameter.label,
                documentation: parameter.documentation
                  ? toMarkdown(parameter.documentation)
                  : undefined,
              })),
            })),
            activeSignature: help.activeSignature ?? 0,
            activeParameter: help.activeParameter ?? 0,
          },
          dispose: () => {
            // Nothing retained.
          },
        };
      },
    });

    monaco.languages.registerDocumentFormattingEditProvider(language, {
      provideDocumentFormattingEdits: async (model, options) => {
        const edits = await client.formatting(
          model.uri.toString(),
          options.tabSize,
          options.insertSpaces,
        );
        return (edits ?? []).map(toEditOperation);
      },
    });
  }
}

/** The editor options both surfaces share. */
function editorOptions(): monaco.editor.IStandaloneEditorConstructionOptions {
  return {
    automaticLayout: true,
    minimap: { enabled: false },
    scrollBeyondLastLine: false,
    fontSize: 13,
    lineNumbers: "on",
    renderLineHighlight: "line",
    tabSize: 4,
    insertSpaces: true,
    wordBasedSuggestions: "off",
    "semanticHighlighting.enabled": true,
    fixedOverflowWidgets: true,
  };
}

/** Follow the page's light/dark preference with Monaco's built-in themes. */
function currentTheme(): string {
  return window.matchMedia?.("(prefers-color-scheme: dark)").matches ? "vs-dark" : "vs";
}

/**
 * Add Monaco's stylesheet, which esbuild emits beside this chunk.
 *
 * The page cannot link it up front: it belongs to a bundle that may never be
 * loaded, and linking it eagerly would fetch 75 KB of editor CSS for a visitor
 * who only browses the registry. Resolving it from `import.meta.url` keeps the
 * pair together wherever the dist is deployed — a subdirectory on Pages, a
 * local `python3 -m http.server`, anywhere.
 */
async function loadStylesheet(href: string): Promise<void> {
  if (document.querySelector(`link[href="${href}"]`)) return;
  await new Promise<void>((resolve) => {
    const link = document.createElement("link");
    link.rel = "stylesheet";
    link.href = href;
    // Resolve either way: a missing stylesheet costs the editor its looks, not
    // its function, and refusing to mount over it would be the worse failure.
    link.addEventListener("load", () => resolve());
    link.addEventListener("error", () => resolve());
    document.head.appendChild(link);
  });
}

/** One mounted surface: its editor, its model, and its change plumbing. */
class Surface {
  readonly editor: monaco.editor.IStandaloneCodeEditor;

  private readonly spec: SurfaceSpec;

  private readonly client: LspClient | null;

  private readonly uri: string;

  private languageId: string;

  /** Set while the studio is writing text in, so `onChange` does not re-fire. */
  private applying = false;

  private timer: number | undefined;

  private readonly highlights: monaco.editor.IEditorDecorationsCollection;

  constructor(
    spec: SurfaceSpec,
    uri: string,
    language: string,
    languageId: string,
    client: LspClient | null,
  ) {
    this.spec = spec;
    this.client = client;
    this.uri = uri;
    this.languageId = languageId;
    spec.container.dataset.monacoLanguage = language;
    spec.container.dataset.lspLanguage = languageId;
    spec.container.dataset.documentUri = uri;

    // The textarea stays in the DOM, hidden, as the studio's form/state
    // plumbing (`state.sample`, or the DSL's persisted text). It is not an
    // alternate editor when Monaco is unavailable.
    spec.textarea.hidden = true;
    spec.container.classList.add("monaco-mounted");

    const model = monaco.editor.createModel(spec.textarea.value, language, monaco.Uri.parse(uri));
    this.editor = monaco.editor.create(spec.container, { ...editorOptions(), model });
    this.highlights = this.editor.createDecorationsCollection();
    client?.openDocument(uri, languageId, spec.textarea.value);

    this.editor.onDidChangeModelContent(() => {
      if (this.applying) return;
      const text = this.editor.getValue();
      spec.textarea.value = text;
      client?.changeDocument(uri, text);
      if (this.timer !== undefined) window.clearTimeout(this.timer);
      this.timer = window.setTimeout(() => spec.onChange(text), SETTLE_MS);
    });
  }

  /** Write text in from the studio without echoing it back out. */
  setText(text: string): void {
    if (this.editor.getValue() === text) return;
    this.applying = true;
    try {
      // `pushEditOperations` rather than `setValue`: it keeps the undo stack
      // and the caret, which matters because a form edit writes the whole
      // document back on every settled keystroke.
      const model = this.editor.getModel();
      if (!model) return;
      model.pushEditOperations([], [{ range: model.getFullModelRange(), text }], () => null);
      this.spec.textarea.value = text;
      this.client?.changeDocument(this.uri, text);
    } finally {
      this.applying = false;
    }
  }

  /** Re-open the document under a new LSP language id (the studio's dialect). */
  setLanguageId(languageId: string): void {
    if (this.languageId === languageId) return;
    this.languageId = languageId;
    this.spec.container.dataset.lspLanguage = languageId;
    this.client?.openDocument(this.uri, languageId, this.editor.getValue());
  }

  /** Re-tokenise after the first successful server analysis. */
  refreshLanguageFeatures(): void {
    const model = this.editor.getModel();
    if (model) monaco.editor.setModelLanguage(model, model.getLanguageId());
  }

  layout(): void {
    this.editor.layout();
  }

  /** Highlight compiler-result source ranges in the shared Explorer editor. */
  setHighlights(ranges: Array<{ start: number; end: number }>): void {
    const model = this.editor.getModel();
    if (!model) return;
    this.highlights.set(
      ranges.map((range) => ({
        range: monaco.Range.fromPositions(
          model.getPositionAt(range.start),
          model.getPositionAt(range.end),
        ),
        options: { inlineClassName: "tcl-explorer-source-highlight" },
      })),
    );
    const first = ranges[0];
    if (first) {
      this.editor.revealRangeInCenter(
        monaco.Range.fromPositions(
          model.getPositionAt(first.start),
          model.getPositionAt(first.end),
        ),
      );
    }
  }

  /** Paint the server's diagnostics for this document. */
  setDiagnostics(items: LspDiagnostic[]): void {
    const model = this.editor.getModel();
    if (model) monaco.editor.setModelMarkers(model, "tcl-lsp", toMarkers(items));
  }

  get documentUri(): string {
    return this.uri;
  }
}

/** A read-only Monaco surface used by both generated-code panes. */
class OutputSurface {
  readonly editor: monaco.editor.IStandaloneCodeEditor;

  private readonly model: monaco.editor.ITextModel;

  private readonly client: LspClient | null;

  private readonly uri: string;

  constructor(
    spec: OutputSurfaceSpec,
    uri: string,
    language: string,
    languageId: string | null,
    client: LspClient | null,
  ) {
    this.client = languageId ? client : null;
    this.uri = uri;
    spec.container.dataset.monacoLanguage = language;
    if (languageId) spec.container.dataset.lspLanguage = languageId;
    spec.container.dataset.documentUri = uri;
    spec.container.classList.add("monaco-mounted");
    this.model = monaco.editor.createModel(
      spec.source.textContent ?? "",
      language,
      monaco.Uri.parse(uri),
    );
    this.editor = monaco.editor.create(spec.container, {
      ...editorOptions(),
      model: this.model,
      readOnly: true,
      domReadOnly: true,
    });
    if (languageId) this.client?.openDocument(uri, languageId, this.model.getValue());
  }

  setText(text: string): void {
    if (this.model.getValue() === text) return;
    this.model.setValue(text);
    this.client?.changeDocument(this.uri, text);
  }

  layout(): void {
    this.editor.layout();
  }

  refreshLanguageFeatures(): void {
    monaco.editor.setModelLanguage(this.model, this.model.getLanguageId());
  }

  setDiagnostics(items: LspDiagnostic[]): void {
    monaco.editor.setModelMarkers(this.model, "tcl-lsp", toMarkers(items));
  }

  get documentUri(): string {
    return this.uri;
  }
}

/**
 * Stand both editors up, with the language server behind them when it boots.
 *
 * The server failing is *not* a failure of this call: Monaco is worth having
 * on its own, so a dead worker degrades to an editor with brackets and
 * bracket-matching and no analysis, and says so. Only Monaco itself failing —
 * which means this module never loaded — makes the editor unavailable.
 */
export async function mountEditors(options: EditorHostOptions): Promise<EditorHost> {
  await loadStylesheet(options.stylesheetUrl);
  registerLanguages();
  await registerTclGrammar(
    monaco,
    [SPECTCL_LANGUAGE, TCL_LANGUAGE],
    options.grammarUrl,
    options.onigurumaUrl,
  );
  monaco.editor.setTheme(currentTheme());
  window
    .matchMedia?.("(prefers-color-scheme: dark)")
    .addEventListener?.("change", () => monaco.editor.setTheme(currentTheme()));

  let client: LspClient | null = null;
  let lspReady = false;
  try {
    options.report("starting the language server…");
    client = await startLspClient(options.workerUrl);
    registerProviders(client);
  } catch (e) {
    options.report(
      `the language server could not start (${e instanceof Error ? e.message : String(e)}) — ` +
        "the editor is running without analysis; reload to try again",
      "err",
    );
  }

  const dsl = new Surface(options.dsl, DSL_URI, SPECTCL_LANGUAGE, "spectcl", client);
  const sample = new Surface(options.sample, SAMPLE_URI, TCL_LANGUAGE, options.dialect, client);
  const rust = new OutputSurface(options.rust, RUST_URI, "rust", null, client);
  const stub = new OutputSurface(options.stub, STUB_URI, TCL_LANGUAGE, "spectcl", client);

  if (client) {
    // `initialize` proves only that the worker booted. `Surface` sent didOpen
    // above, so explicitly request the pack's tokens now, in protocol order.
    // Monaco's semantic-token controller is intentionally asynchronous and
    // may decline to schedule a request while its tab is initially hidden;
    // making readiness depend on that implementation detail turned a healthy
    // Pages deployment into a two-minute false failure. Monaco still consumes
    // the same provider for on-screen colouring, but that is verified
    // separately by the browser boot check.
    try {
      const semanticTokens = await Promise.race([
        client.semanticTokens(DSL_URI),
        new Promise<never>((_resolve, reject) =>
          window.setTimeout(
            () => reject(new Error("the pack document semantic-token request timed out")),
            LSP_READINESS_TIMEOUT_MS,
          ),
        ),
      ]);
      const tokenCount = semanticTokens?.data.length ?? 0;
      if (!tokenCount) throw new Error("the pack document returned no semantic tokens");
      const hoverAt = positionOf(options.dsl.textarea.value, "speclib");
      if (hoverAt && !(await client.hover(DSL_URI, hoverAt.line, hoverAt.character))) {
        throw new Error("the pack document returned no hover information");
      }
      // A model mounted while its tab was hidden may have completed its first
      // lexical pass before the LSP's initial registry was warm. Re-applying
      // the same Monaco language is the public way to invalidate that pass;
      // it schedules both the TextMate provider and the registered semantic
      // provider without changing the document or its LSP dialect.
      dsl.refreshLanguageFeatures();
      sample.refreshLanguageFeatures();
      stub.refreshLanguageFeatures();
      lspReady = true;
      options.report(
        `the Tcl language server is running in this page — Pack DSL: SpecTcl on Tcl 9.0; Test: ${options.dialect}`,
        "ok",
      );
    } catch (e) {
      options.report(
        `the language server started but could not analyse the pack (${e instanceof Error ? e.message : String(e)}) — ` +
          "the editor is running without analysis; reload to try again",
        "err",
      );
    }
  }

  client?.onDiagnostics((uri, items) => {
    if (uri === dsl.documentUri) dsl.setDiagnostics(items);
    else if (uri === sample.documentUri) sample.setDiagnostics(items);
    else if (uri === stub.documentUri) stub.setDiagnostics(items);
  });

  return {
    setDslText: (text) => dsl.setText(text),
    setSampleText: (text) => sample.setText(text),
    setRustText: (text) => rust.setText(text),
    setStubText: (text) => stub.setText(text),
    setDialect: (dialect) => sample.setLanguageId(dialect),
    layout: () => {
      dsl.layout();
      sample.layout();
      rust.layout();
      stub.layout();
    },
    get lspReady() {
      return lspReady;
    },
  };
}

/** Options for the Compiler Explorer's single editable Tcl surface. */
export interface TclEditorOptions {
  container: HTMLElement;
  textarea: HTMLTextAreaElement;
  onChange(text: string): void;
  dialect: string;
  workerUrl: string;
  stylesheetUrl: string;
  grammarUrl: string;
  onigurumaUrl: string;
  report?(message: string, kind?: "ok" | "err"): void;
}

/** Mount the same Monaco/TextMate/LSP stack for Compiler Explorer on Pages. */
export async function mountTclEditor(options: TclEditorOptions): Promise<{
  setDialect(dialect: string): void;
  highlightRanges(ranges: Array<{ start: number; end: number }>): void;
  layout(): void;
  readonly lspReady: boolean;
}> {
  await loadStylesheet(options.stylesheetUrl);
  registerLanguages();
  await registerTclGrammar(monaco, [TCL_LANGUAGE], options.grammarUrl, options.onigurumaUrl);
  monaco.editor.setTheme(currentTheme());
  let client: LspClient | null = null;
  try {
    client = await startLspClient(options.workerUrl);
    registerProviders(client);
  } catch (error) {
    options.report?.(
      `language server unavailable: ${error instanceof Error ? error.message : String(error)}`,
      "err",
    );
  }
  const surface = new Surface(
    { container: options.container, textarea: options.textarea, onChange: options.onChange },
    EXPLORER_URI,
    TCL_LANGUAGE,
    options.dialect,
    client,
  );
  client?.onDiagnostics((uri, items) => {
    if (uri === surface.documentUri) surface.setDiagnostics(items);
  });
  options.report?.("using the shared Tcl Monaco editor", "ok");
  const ready = client !== null;
  return {
    setDialect: (dialect) => surface.setLanguageId(dialect),
    highlightRanges: (ranges) => surface.setHighlights(ranges),
    layout: () => surface.layout(),
    get lspReady() {
      return ready;
    },
  };
}
