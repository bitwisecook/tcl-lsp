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

/**
 * The parts of the language client both entry points share.
 *
 * `extension.ts` is the **node** entry (`main`): it runs the native
 * `tcl-lsp-server` binary over stdio. `extensionBrowser.ts` is the **browser**
 * entry (`browser`): it runs the same `LspService<Backend>` compiled to wasm in
 * a Web Worker. Everything in this module is platform-neutral — it imports
 * `vscode` and the language client's *common* API surface
 * (`vscode-languageclient`, which resolves to `lib/common/api`), never
 * `vscode-languageclient/node`, `vscode-languageclient/browser`, or a node
 * builtin — so the browser bundle can take it verbatim.
 *
 * Deliberately NOT here: `DIALECT_LABELS` and `LANGUAGE_ID_DIALECTS`. Both are
 * generated blocks that `cargo xtask gen-editor-dialects` /
 * `gen-editor-extensions` write into `editors/vscode/src/extension.ts` by path,
 * so moving them would mean moving the generator's target too. The browser
 * entry reads dialect labels from the generated, vscode-free
 * `./chat/dialectCatalog` instead, which is the same projection of
 * `tcl_dialect::DialectProfile::all()`.
 */

import { Range, Uri, workspace, WorkspaceEdit } from "vscode";
import type { LanguageClientOptions } from "vscode-languageclient";
import type { DiffDiagnosticsSuppressor } from "./diffAnalysis";
import { TCL_LANGUAGE_IDS } from "./languageIds";

export const DEFAULT_DIALECT = "tcl8.6";

// LSP wire types, for the server commands that answer with an edit of their own
// rather than through a protocol request (the BIG-IP partition rename).

export interface LspPosition {
  line: number;
  character: number;
}

export interface LspRange {
  start: LspPosition;
  end: LspPosition;
}

export interface LspTextEdit {
  range: LspRange;
  newText?: string;
  new_text?: string;
}

export interface LspWorkspaceEdit {
  changes?: Record<string, LspTextEdit[]>;
}

/** Convert a server-supplied workspace edit into one VS Code can apply. */
export function workspaceEditFromLsp(edit: LspWorkspaceEdit): WorkspaceEdit {
  const workspaceEdit = new WorkspaceEdit();
  for (const [uriString, textEdits] of Object.entries(edit.changes ?? {})) {
    const uri = Uri.parse(uriString);
    for (const textEdit of textEdits) {
      const newText = textEdit.newText ?? textEdit.new_text;
      if (newText === undefined) {
        continue;
      }
      workspaceEdit.replace(
        uri,
        new Range(
          textEdit.range.start.line,
          textEdit.range.start.character,
          textEdit.range.end.line,
          textEdit.range.end.character,
        ),
        newText,
      );
    }
  }
  return workspaceEdit;
}

/**
 * Map from feature toggle key to the VS Code editor setting it inherits from.
 * Features not listed here default to true when null.
 */
export const FEATURE_EDITOR_DEFAULTS: Record<string, () => boolean> = {
  hover: () => {
    const v = workspace.getConfiguration("editor").get<string | boolean>("hover.enabled", true);
    return v !== "off" && v !== false;
  },
  semanticTokens: () => {
    const v = workspace
      .getConfiguration("editor")
      .get<boolean | string>("semanticHighlighting.enabled", true);
    return v !== false; // "configuredByTheme" and true both resolve to enabled
  },
  // Deliberately NOT inherited from `editor.folding` (issue #1122). Vanilla
  // VS Code's sticky-scroll model provider calls
  // `FoldingController.getFoldingRangeProviders` unconditionally — it never
  // reads `EditorOption.folding` — so a user who has switched the folding UI
  // off in vanilla VS Code still gets provider-based sticky scroll. Our
  // toggle used to inherit `editor.folding`, so the same user's Tcl files
  // got NO folding ranges at all; since VS Code >=1.105 treats an empty
  // folding-range array as a *terminal* sticky model (only null/undefined
  // falls through to the indentation heuristic), that silently killed
  // sticky scroll for every Tcl file — a divergence from platform semantics
  // and the most consistent explanation of the bug report. Folding stays
  // on unless a user explicitly sets `tclLsp.features.folding: false`.
  folding: () => true,
  signatureHelp: () =>
    workspace.getConfiguration("editor").get<boolean>("parameterHints.enabled", true),
  // Inlay hints split into two opt-in families, both off unless the user
  // explicitly enables them (they do not inherit editor.inlayHints.enabled).
  inlayTypeHints: () => false,
  inlayParameterHints: () => false,
  documentHighlight: () => {
    const v = workspace
      .getConfiguration("editor")
      .get<string | boolean>("occurrencesHighlight", "singleFile");
    return v !== "off" && v !== false;
  },
  codeLens: () => workspace.getConfiguration("editor").get<boolean>("codeLens", true),
  linkedEditingRange: () =>
    workspace.getConfiguration("editor").get<boolean>("linkedEditing", false),
  // Opt-in, off unless the user explicitly enables them — same rationale as
  // the inlay-hint families above. Without an entry here, an unset (null)
  // value falls through resolveFeatureToggle's "no resolver -> true" default,
  // which silently turns on workspace-wide cross-file scanning / XC100-301
  // translatability lints for every user who has never touched the setting.
  xcDiagnostics: () => false,
  crossFileResolution: () => false,
};

/**
 * The VS Code editor settings a feature toggle inherits from, so a change to
 * one can be re-pushed to the server.
 *
 * `editor.folding` is deliberately absent (issue #1122): `features.folding` no
 * longer inherits it, so changing it has nothing to re-push.
 */
export const EDITOR_SETTINGS_AFFECTING_FEATURES = [
  "editor.hover.enabled",
  "editor.semanticHighlighting.enabled",
  "editor.parameterHints.enabled",
  "editor.inlayHints.enabled",
  "editor.occurrencesHighlight",
  "editor.codeLens",
  "editor.linkedEditing",
];

/**
 * Resolve a tri-state feature toggle (boolean | null) to a concrete boolean.
 * When the user has not set an explicit value (null), the toggle inherits
 * from the corresponding VS Code editor setting where applicable, or
 * defaults to true.
 */
export function resolveFeatureToggle(key: string, value: boolean | null): boolean {
  if (typeof value === "boolean") return value;
  const resolver = FEATURE_EDITOR_DEFAULTS[key];
  return resolver ? resolver() : true;
}

/**
 * Read all tclLsp.features.* settings and resolve null values to concrete
 * booleans using VS Code editor globals.
 */
export function resolveAllFeatureToggles(): Record<string, boolean> {
  const features = workspace.getConfiguration("tclLsp.features");
  const resolved: Record<string, boolean> = {};
  for (const key of Object.keys(FEATURE_EDITOR_DEFAULTS)) {
    const val = features.get<boolean | null>(key, null);
    resolved[key] = resolveFeatureToggle(key, val);
  }
  return resolved;
}

/**
 * The URI schemes the desktop host serves Tcl documents on.
 *
 * The browser host passes {@link ANY_SCHEME} instead: a web workspace lives on
 * whatever scheme its filesystem provider registered (`vscode-vfs` on
 * github.dev, `vscode-test-web` under `@vscode/test-web`, `memfs`, …), there is
 * no enumerable list of them, and a selector pinned to `file` matches nothing
 * at all there. The failure is quiet: the server answers every request put to
 * it directly, while the editor shows no tokens and no diagnostics, because no
 * provider was ever registered for the document.
 */
export const DESKTOP_DOCUMENT_SCHEMES = ["file", "untitled"] as const;

/**
 * Match Tcl documents on any URI scheme.
 *
 * Spelled `null` rather than `undefined` on purpose: `undefined` is exactly
 * what re-triggers a default parameter, so an explicit "no scheme filter"
 * argument would silently become the desktop list.
 */
export const ANY_SCHEME = null;

/**
 * The `LanguageClientOptions` both transports use.
 *
 * Nothing here is transport-specific: the configuration synchronisation and the
 * middleware are the same whether the server is a native binary on stdio or a
 * wasm module in a Web Worker. Only the document selector's *scheme* filter
 * differs — see {@link DESKTOP_DOCUMENT_SCHEMES}.
 */
export function buildClientOptions(
  diffSuppressor: DiffDiagnosticsSuppressor,
  schemes: readonly string[] | typeof ANY_SCHEME = DESKTOP_DOCUMENT_SCHEMES,
): LanguageClientOptions {
  return {
    documentSelector: [
      ...[...TCL_LANGUAGE_IDS].flatMap((language) =>
        schemes ? schemes.map((scheme) => ({ scheme, language })) : [{ language }],
      ),
    ],
    synchronize: {
      configurationSection: "tclLsp",
      // No `fileEvents` watchers here on purpose. The server registers
      // `workspace/didChangeWatchedFiles` dynamically at `initialized`, naming
      // its own extension set (case-folded per character, so `UPPER.TCL` is
      // watched on Linux too) plus `**/.tcl-lsp.ini` for the layered-settings
      // live-reload. Duplicating that list here gave us two sources of truth
      // that had already drifted — the client list was missing `.exp` and
      // `.apl` — and made every watched change arrive twice (issue #1215).
    },
    middleware: {
      handleDiagnostics: (uri, diagnostics, next) =>
        diffSuppressor.handleDiagnostics(uri, diagnostics, next),
      workspace: {
        // Pull path: server requests configuration via workspace/configuration
        // for each workspace folder (plus an unscoped fallback request).
        // The default LanguageClient implementation honours scopeUri and
        // returns the folder's effective settings; we just resolve null
        // feature toggles to booleans for each item.
        configuration: async (params, token, next) => {
          const result = await next(params, token);
          if (Array.isArray(result)) {
            for (let i = 0; i < params.items.length; i++) {
              const section = params.items[i].section;
              if (section === "tclLsp" && result[i] && typeof result[i] === "object") {
                const settings = result[i] as Record<string, unknown>;
                const features = settings.features;
                if (features && typeof features === "object") {
                  const feat = features as Record<string, unknown>;
                  for (const [key, val] of Object.entries(feat)) {
                    if (val === null || val === undefined) {
                      feat[key] = resolveFeatureToggle(key, null);
                    }
                  }
                }
                // `diagnostics.genericVariablePatterns` defaults to `[]`, but the
                // server distinguishes "absent → built-in IRULE4002 patterns"
                // from "explicit empty list → disable the check". Drop the empty
                // default so an unconfigured workspace keeps the defaults (the
                // JetBrains client omits it the same way).
                const diagnostics = settings.diagnostics;
                if (diagnostics && typeof diagnostics === "object") {
                  const diag = diagnostics as Record<string, unknown>;
                  if (
                    Array.isArray(diag.genericVariablePatterns) &&
                    diag.genericVariablePatterns.length === 0
                  ) {
                    delete diag.genericVariablePatterns;
                  }
                  // Same absent-vs-empty distinction for `diagnostics.exclude`
                  // (#1556): a folder answering with the explicit `[]` default
                  // would *replace* an exclude list configured at another layer
                  // (global config file, user settings) with "exclude nothing".
                  if (Array.isArray(diag.exclude) && diag.exclude.length === 0) {
                    delete diag.exclude;
                  }
                }
              }
            }
          }
          return result;
        },
        // Push path is intentionally NOT overridden.  The default
        // LanguageClient behaviour sends an empty didChangeConfiguration
        // notification when settings change, and the server then pulls
        // per-folder via workspace/configuration — that's what we want for
        // multi-folder workspaces (issue #230).  A custom push that reads
        // workspace.getConfiguration("tclLsp") without a scopeUri would
        // clobber per-folder settings with the workspace-merged value.
      },
    },
  };
}
