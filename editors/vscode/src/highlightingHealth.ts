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

import { readFileSync } from "fs";
import * as path from "path";
import {
  ConfigurationTarget,
  ExtensionContext,
  TextDocument,
  extensions,
  languages,
  window,
  workspace,
} from "vscode";
import { EXTENSION_LANGUAGE_IDS, isTclLanguage } from "./languageIds";

// Highlighting-health notices.
//
// tcl-lsp's semantic tokens are what let it treat, say, `for`/`else`/`error`
// inside `argparse -help {...}` as plain string text rather than keywords: a
// TextMate grammar (ours or a third party's) is context-free regex and cannot
// know an unknown command's braced argument is data, not a script body, so it
// colours those words as keywords.  The semantic-token overlay corrects that —
// but only when it actually reaches the buffer.  Two situations silently defeat
// it, and this module surfaces each once (issue #749):
//
//   1. The file opened under a non-Tcl language id (another extension won the
//      file association), so our language client never attaches.
//   2. The client is attached, but semantic highlighting is off — disabled by
//      `tclLsp.features.semanticTokens`, by `editor.semanticHighlighting.enabled`,
//      or by a colour theme that doesn't opt in — leaving grammar-only colours.

const CONFIG_ENABLED_KEY = "notifications.highlightingHealth";

export const DISMISS_LANGUAGE_MISMATCH = "tclLsp.highlightHealth.languageMismatch.dismissed";
export const DISMISS_SEMANTIC_OFF = "tclLsp.highlightHealth.semanticOff.dismissed";

const SWITCH_ACTION = "Switch to Tcl";
const ENABLE_ACTION = "Enable";
const DISMISS_ACTION = "Don't show again";

// Extensions that our language contributions claim but that are too generic to
// treat as "should be Tcl" — flagging a foreign `.test`/`.apl`/`.exp` file would
// be a false positive.
//
// `.do` and `.globals` are here for the same reason and were missed: `.do`
// belongs to ModelSim/Questa macros *and* to a dozen unrelated tools, and
// `.globals` to as many again — both collide with foreign files more often
// than `.scf`, which was already listed (issue #1625).
const GENERIC_EXTENSIONS = new Set([".test", ".impl", ".scf", ".exp", ".apl", ".do", ".globals"]);

// Shown at most once per session per check (in addition to the permanent
// "Don't show again" dismissal) to keep the notices quiet.
let languageMismatchShown = false;
let semanticOffShown = false;

/** Reset the per-session "already shown" flags. Exposed for testing. */
export function resetHighlightingHealthSession(): void {
  languageMismatchShown = false;
  semanticOffShown = false;
}

export type SemanticReason = "featureOff" | "editorOff" | "themeUnsupported";

export type SemanticStatus =
  { effective: true; reason: "on" } | { effective: false; reason: SemanticReason };

/**
 * The side-effecting collaborators the checks depend on. Production builds this
 * from the VS Code API and the extension context; tests inject fakes so the
 * decision/dispatch logic can be exercised without real notifications or global
 * settings mutations.
 */
export interface HealthContext {
  isDismissed(key: string): boolean;
  dismiss(key: string): void;
  ownedExtensions: Set<string>;
  resolveSemantic(document: TextDocument): SemanticStatus;
  notify(message: string, ...actions: string[]): Thenable<string | undefined>;
  switchLanguage(document: TextDocument, languageId: string): void;
  enableSemantic(reason: SemanticReason, document: TextDocument): void;
}

/** Register the highlighting-health checks against the active editor. */
export function registerHighlightingHealthChecks(context: ExtensionContext): void {
  const ctx = buildProductionContext(context);

  const run = (document: TextDocument | undefined): void => {
    if (!document) {
      return;
    }
    if (!workspace.getConfiguration("tclLsp").get<boolean>(CONFIG_ENABLED_KEY, true)) {
      return;
    }
    void runHighlightingHealthChecks(document, ctx);
  };

  context.subscriptions.push(
    window.onDidChangeActiveTextEditor((editor) => run(editor?.document)),
    // Opening resolves the language id even when the file isn't the active tab.
    workspace.onDidOpenTextDocument((document) => run(document)),
  );

  run(window.activeTextEditor?.document);
}

/** Run both checks for a document against the given collaborators. */
export async function runHighlightingHealthChecks(
  document: TextDocument,
  ctx: HealthContext,
): Promise<void> {
  await checkLanguageMismatch(document, ctx);
  await checkSemanticHighlighting(document, ctx);
}

/**
 * Check 1: a file whose extension we own opened under a non-Tcl language id, so
 * a foreign extension owns the association and our client never attaches.
 */
async function checkLanguageMismatch(document: TextDocument, ctx: HealthContext): Promise<void> {
  if (languageMismatchShown || ctx.isDismissed(DISMISS_LANGUAGE_MISMATCH)) {
    return;
  }
  if (document.uri.scheme !== "file" || isTclLanguage(document.languageId)) {
    return;
  }
  const ext = path.extname(document.fileName).toLowerCase();
  if (!ctx.ownedExtensions.has(ext)) {
    return;
  }

  languageMismatchShown = true;
  const targetId = tclLanguageIdForExtension(ext);
  const name = path.basename(document.fileName);
  const choice = await ctx.notify(
    `"${name}" opened as "${document.languageId}", not Tcl — another extension likely owns this ` +
      `file type. tcl-lsp's semantic highlighting and analysis won't apply until it's set to Tcl.`,
    SWITCH_ACTION,
    DISMISS_ACTION,
  );
  if (choice === SWITCH_ACTION) {
    ctx.switchLanguage(document, targetId);
  } else if (choice === DISMISS_ACTION) {
    ctx.dismiss(DISMISS_LANGUAGE_MISMATCH);
  }
}

/**
 * Check 2: our client is attached to a Tcl document, but semantic highlighting
 * is off, so only the (context-free) TextMate grammar colours the buffer.
 */
async function checkSemanticHighlighting(
  document: TextDocument,
  ctx: HealthContext,
): Promise<void> {
  if (semanticOffShown || ctx.isDismissed(DISMISS_SEMANTIC_OFF)) {
    return;
  }
  if (!isTclLanguage(document.languageId)) {
    return;
  }
  const status = ctx.resolveSemantic(document);
  if (status.effective) {
    return;
  }

  semanticOffShown = true;
  const detail =
    status.reason === "featureOff"
      ? "Semantic highlighting is disabled for Tcl (tclLsp.features.semanticTokens = false)"
      : status.reason === "editorOff"
        ? "editor.semanticHighlighting.enabled is off"
        : "the active colour theme doesn't support semantic highlighting";
  const choice = await ctx.notify(
    `${detail}, so Tcl falls back to TextMate colouring. That grammar can mis-colour keywords ` +
      `like \`for\`/\`else\`/\`error\` inside braced string arguments of unknown commands. ` +
      `Enabling semantic highlighting lets tcl-lsp override it.`,
    ENABLE_ACTION,
    DISMISS_ACTION,
  );
  if (choice === ENABLE_ACTION) {
    ctx.enableSemantic(status.reason, document);
  } else if (choice === DISMISS_ACTION) {
    ctx.dismiss(DISMISS_SEMANTIC_OFF);
  }
}

/** Build the production collaborators from the VS Code API and extension context. */
function buildProductionContext(context: ExtensionContext): HealthContext {
  const owned = ownedTclExtensions(context);
  return {
    isDismissed: (key) => context.globalState.get<boolean>(key) === true,
    dismiss: (key) => void context.globalState.update(key, true),
    ownedExtensions: owned,
    resolveSemantic: resolveSemanticHighlighting,
    notify: (message, ...actions) => window.showWarningMessage(message, ...actions),
    switchLanguage: (document, languageId) =>
      void languages.setTextDocumentLanguage(document, languageId),
    enableSemantic: (reason, document) => void enableSemanticHighlighting(reason, document),
  };
}

/**
 * Pure decision for whether the semantic-token overlay is effective, given the
 * three inputs that govern it. Only returns `effective: false` when the overlay
 * is positively off — an undetermined theme (`themeSupports === undefined`) is
 * treated as on, to avoid false-positive nagging. Exported for unit testing.
 *
 * @param feature       `tclLsp.features.semanticTokens` (boolean | null).
 * @param editorSetting `editor.semanticHighlighting.enabled` (boolean | "configuredByTheme").
 * @param themeSupports the active theme's opt-in, or undefined if unknown.
 */
export function decideSemanticStatus(
  feature: boolean | null,
  editorSetting: boolean | string,
  themeSupports: boolean | undefined,
): SemanticStatus {
  if (feature === false) {
    return { effective: false, reason: "featureOff" };
  }
  if (editorSetting === false) {
    return { effective: false, reason: "editorOff" };
  }
  if (editorSetting === true) {
    return { effective: true, reason: "on" };
  }
  // "configuredByTheme": follows the active theme's opt-in flag.
  if (themeSupports === false) {
    return { effective: false, reason: "themeUnsupported" };
  }
  return { effective: true, reason: "on" };
}

/** Resolve whether the semantic-token overlay is effective for `document`. */
function resolveSemanticHighlighting(document: TextDocument): SemanticStatus {
  // tclLsp.features.semanticTokens: boolean | null (null inherits the editor).
  const feature = workspace
    .getConfiguration("tclLsp", document)
    .get<boolean | null>("features.semanticTokens", null);
  // editor.semanticHighlighting.enabled: true | false | "configuredByTheme".
  const editorSetting = workspace
    .getConfiguration("editor", document)
    .get<boolean | string>("semanticHighlighting.enabled", "configuredByTheme");
  // Only consult the (relatively expensive) theme scan when it can matter.
  const themeSupports =
    feature !== false && editorSetting !== false && editorSetting !== true
      ? themeSupportsSemanticHighlighting()
      : undefined;
  return decideSemanticStatus(feature, editorSetting, themeSupports);
}

/**
 * Best-effort read of the active colour theme's `semanticHighlighting` opt-in.
 * Returns `undefined` when it can't be determined (in which case we don't warn).
 */
function themeSupportsSemanticHighlighting(): boolean | undefined {
  const themeName = workspace.getConfiguration("workbench").get<string>("colorTheme");
  if (!themeName) {
    return undefined;
  }
  for (const ext of extensions.all) {
    const themes = ext.packageJSON?.contributes?.themes;
    if (!Array.isArray(themes)) {
      continue;
    }
    for (const theme of themes) {
      if (theme.label !== themeName && theme.id !== themeName) {
        continue;
      }
      if (typeof theme.semanticHighlighting === "boolean") {
        return theme.semanticHighlighting;
      }
      // Not declared in package.json — the flag may live in the theme file.
      if (typeof theme.path === "string") {
        try {
          const content = readFileSync(path.join(ext.extensionPath, theme.path), "utf8");
          const match = content.match(/"semanticHighlighting"\s*:\s*(true|false)/);
          if (match) {
            return match[1] === "true";
          }
        } catch {
          // Unreadable theme file — fall through to undefined.
        }
      }
      return undefined;
    }
  }
  return undefined;
}

/** The subset of `WorkspaceConfiguration.inspect()` scope values we consult. */
export interface ConfigInspection {
  workspaceFolderLanguageValue?: unknown;
  workspaceFolderValue?: unknown;
  workspaceLanguageValue?: unknown;
  workspaceValue?: unknown;
  globalLanguageValue?: unknown;
  globalValue?: unknown;
}

export interface EnableTarget {
  target: ConfigurationTarget;
  overrideInLanguage: boolean;
}

/**
 * Choose where to write `true` so the setting actually becomes effective for the
 * document. Writing only to Global is wrong: a Workspace/Folder/language-scoped
 * `false` (or theme-driven `configuredByTheme`) is more specific and would keep
 * the effective value off. So we override at the *narrowest* scope that
 * currently forces the value off; if nothing explicitly forces it off (e.g. a
 * theme-driven default), we override at the narrowest writable scope. Exported
 * for unit testing.
 */
export function chooseEnableTarget(
  info: ConfigInspection | undefined,
  isOff: (value: unknown) => boolean,
  scope: { hasFolder: boolean; hasWorkspace: boolean },
): EnableTarget {
  // Scopes ordered most-specific → least-specific (VS Code merge precedence).
  const scopes: Array<[unknown, ConfigurationTarget, boolean]> = info
    ? [
        [info.workspaceFolderLanguageValue, ConfigurationTarget.WorkspaceFolder, true],
        [info.workspaceFolderValue, ConfigurationTarget.WorkspaceFolder, false],
        [info.workspaceLanguageValue, ConfigurationTarget.Workspace, true],
        [info.workspaceValue, ConfigurationTarget.Workspace, false],
        [info.globalLanguageValue, ConfigurationTarget.Global, true],
        [info.globalValue, ConfigurationTarget.Global, false],
      ]
    : [];
  const offScope = scopes.find(([value]) => value !== undefined && isOff(value));
  if (offScope) {
    return { target: offScope[1], overrideInLanguage: offScope[2] };
  }
  const target = scope.hasFolder
    ? ConfigurationTarget.WorkspaceFolder
    : scope.hasWorkspace
      ? ConfigurationTarget.Workspace
      : ConfigurationTarget.Global;
  return { target, overrideInLanguage: false };
}

async function enableSemanticHighlighting(
  reason: SemanticReason,
  document: TextDocument,
): Promise<void> {
  const [section, key, isOff]: [string, string, (value: unknown) => boolean] =
    reason === "featureOff"
      ? ["tclLsp", "features.semanticTokens", (value) => value === false]
      : // editorOff is an explicit `false`; themeUnsupported means the value is
        // (or defaults to) "configuredByTheme" and the theme doesn't opt in.
        [
          "editor",
          "semanticHighlighting.enabled",
          (value) => value === false || value === "configuredByTheme",
        ];

  const config = workspace.getConfiguration(section, document);
  const { target, overrideInLanguage } = chooseEnableTarget(config.inspect(key), isOff, {
    hasFolder: workspace.getWorkspaceFolder(document.uri) !== undefined,
    hasWorkspace: (workspace.workspaceFolders?.length ?? 0) > 0,
  });
  await config.update(key, true, target, overrideInLanguage);
  window.showInformationMessage("Semantic highlighting enabled for Tcl.");
}

/**
 * Filter a raw list of file extensions to those specific enough to treat as
 * "should be Tcl", dropping generics that other languages commonly own.
 * Exported for unit testing.
 */
export function filterOwnedExtensions(rawExtensions: Iterable<string>): Set<string> {
  const owned = new Set<string>();
  for (const ext of rawExtensions) {
    const lower = String(ext).toLowerCase();
    if (!GENERIC_EXTENSIONS.has(lower)) {
      owned.add(lower);
    }
  }
  return owned;
}

/** Collect the file extensions our language contributions own, minus generics. */
function ownedTclExtensions(context: ExtensionContext): Set<string> {
  const languageDefs = context.extension.packageJSON?.contributes?.languages;
  const raw: string[] = [];
  if (Array.isArray(languageDefs)) {
    for (const def of languageDefs) {
      if (Array.isArray(def?.extensions)) {
        raw.push(...def.extensions);
      }
    }
  }
  return filterOwnedExtensions(raw);
}

/**
 * Pick the most specific Tcl language id for a given file extension, falling
 * back to plain `tcl` for one we do not own. Exported for unit testing.
 *
 * A hand-written switch here knew 4 of the 25 registered extensions, so the
 * "Switch to Tcl" action on a `.sdc` file that had lost its association
 * offered `tcl` rather than `tcl-synopsys` (issue #1625). The answer now comes
 * from the generated catalogue projection, which cannot drift from what
 * `contributes.languages` registers.
 */
export function tclLanguageIdForExtension(ext: string): string {
  const key = ext.toLowerCase();
  return Object.prototype.hasOwnProperty.call(EXTENSION_LANGUAGE_IDS, key)
    ? EXTENSION_LANGUAGE_IDS[key]
    : "tcl";
}
