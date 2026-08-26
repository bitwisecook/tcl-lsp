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
 * The **browser** entry point (`package.json` `browser`), for vscode.dev,
 * github.dev, and any other web extension host.
 *
 * The server is the same `LspService<Backend>` the native binary runs, compiled
 * to wasm and driven over `postMessage` by
 * `rust/tcl-lsp-server-wasm/worker.js`, whose three-file `dist/` is staged into
 * this extension at `dist/web/`. There is no filesystem in that build: every
 * closed file reaches it through the source store
 * (`docs/design/contracts/lsp-source-store.md`), which is what
 * `./webWorkspaceSync` fills.
 *
 * `./extension.ts` remains the node entry and is unchanged in behaviour. What
 * this entry deliberately does NOT register, and why, is listed at
 * `WEB_UNSUPPORTED` below — each of those needs a real migration, not a shim.
 */

import * as vscode from "vscode";
import {
  commands,
  ExtensionContext,
  Range,
  StatusBarAlignment,
  StatusBarItem,
  TextEditor,
  window,
  workspace,
} from "vscode";
import { LanguageClient } from "vscode-languageclient/browser";
import { workerLspTransports } from "./webLspTransport";
import { DIALECT_LABELS } from "./chat/dialectCatalog";
import {
  ANY_SCHEME,
  buildClientOptions,
  DEFAULT_DIALECT,
  EDITOR_SETTINGS_AFFECTING_FEATURES,
  LspWorkspaceEdit,
  resolveAllFeatureToggles,
  workspaceEditFromLsp,
} from "./clientCore";
import { DiffDiagnosticsSuppressor } from "./diffAnalysis";
import { buildIruleEventSkeleton, COMMON_IRULE_EVENTS } from "./iruleSkeleton";
import { isTclLanguage } from "./languageIds";
import { PackFileExtension, syncPackFileAssociations } from "./packAssociations";
import { escapeTclText, transformSelection, unescapeTclText } from "./selectionTransforms";
import { convertShowReferencesArgs, JsonLocation, JsonPosition } from "./showReferences";
import { ensureStickyScrollDefaultModel } from "./stickyScrollHealth";
import {
  parseTemplateSnippetCatalog,
  renderTemplateSnippet,
  TEMPLATE_SNIPPET_RELATIVE_PATH,
} from "./templateSnippetsCatalog";
import {
  deriveSyncGlobs,
  readBudget,
  SyncManifest,
  workerSourceStoreHost,
  WorkspaceStoreSync,
} from "./webWorkspaceSync";

/**
 * Desktop-only features, and what each one needs before it can run on the web.
 *
 * Kept as data rather than scattered comments so the list is greppable and the
 * "Tcl LSP: … is desktop-only" message can name the reason.
 */
const WEB_UNSUPPORTED: Record<string, string> = {
  "tclLsp.runRuntimeValidation":
    "runs tclsh as a child process; the web host has no process to run",
  "tclLsp.openCompilerExplorer":
    "the explorer webview reads its wasm module and static assets with fs; migrate to workspace.fs + asWebviewUri",
  "tclLsp.openSpecStudio":
    "the studio's session storage uses fs.existsSync; migrate to workspace.fs.stat",
  "tclLsp.openTkPreview":
    "the preview panel's CSP nonce uses node crypto and its offset mapping uses Buffer; both have web equivalents",
  "tclLsp.scaffoldPackageStarter": "writes the scaffold with fs; migrate to workspace.fs.writeFile",
  "tclLsp.exportConfig": "the server writes the exported file to disk; the wasm server has no disk",
  "tclLsp.selectTclInstallation":
    "discovers Tcl installations on disk; there is no disk to discover them on",
  "tclLsp.unminifyError": "decodes the picked symbol map with Buffer",
  "tclLsp.extractAllRules": "writes the extracted rules with Buffer-encoded bytes",
  "tclLsp.copyFileAsBase64": "reads the file with fs and encodes with Buffer",
  "tclLsp.copyFileAsGzipBase64": "reads the file with fs and compresses with node zlib",
};

let client: LanguageClient | undefined;
let dialectStatusBarItem: StatusBarItem;
let versionStatusBarItem: StatusBarItem;
let outputChannel: vscode.OutputChannel;

export function getClient(): LanguageClient | undefined {
  return client;
}

export async function activate(context: ExtensionContext) {
  const activateStart = Date.now();
  outputChannel = window.createOutputChannel("Tcl Language Server");
  context.subscriptions.push(outputChannel);
  const ch = outputChannel;
  const log = (message: string) => ch.appendLine(message);

  const webAssets = vscode.Uri.joinPath(context.extensionUri, "dist", "web");
  const workerUri = vscode.Uri.joinPath(webAssets, "worker.js");
  let worker: Worker;
  try {
    // The name is the worker's asset base, and it is load-bearing rather than a
    // label. The web extension host cannot start a cross-origin worker
    // directly, so it wraps this URL in a same-origin blob that
    // `importScripts()`es it — inside which `self.location` is that opaque
    // `blob:` URL and cannot resolve `tcl_lsp_server_wasm.js` beside it. The
    // host forwards `options` to the real `Worker`, so `self.name` is the one
    // channel that survives; `worker.js`'s `assetBaseUrl` reads it.
    worker = new Worker(workerUri.toString(true), { name: `${webAssets.toString(true)}/` });
  } catch (err) {
    window.showErrorMessage(
      "Tcl LSP: could not start the language server worker. The web build's " +
        `assets are missing from this install (expected ${workerUri.toString()}).`,
    );
    log(`[web] worker construction failed: ${String(err)}`);
    return;
  }
  log(`[web] language server worker: ${workerUri.toString()}`);

  // Fill the server's in-memory store BEFORE the client starts: `initialized`
  // is what loads the pack set and runs the workspace scan, and the worker
  // backlogs anything that arrives before wasm init, so posting now is both
  // safe and the only moment that works.
  const store = workerSourceStoreHost(worker);
  const globs = deriveSyncGlobs(context.extension.packageJSON as SyncManifest);
  log(`[web] workspace sync globs: ${globs.join(" ")}`);
  const sync = new WorkspaceStoreSync(store, globs, log);
  context.subscriptions.push({ dispose: () => sync.dispose() });
  const budget = readBudget();
  await sync.primeSpecPacks(vscode.Uri.joinPath(context.extensionUri, "dist", "web", "specs"));
  const primeStart = Date.now();
  await sync.primeWorkspace(budget);
  log(`[timing] workspace store prime: ${Date.now() - primeStart}ms`);

  const diffSuppressor = new DiffDiagnosticsSuppressor();
  context.subscriptions.push(diffSuppressor);

  // No scheme filter: a web workspace is served by whatever filesystem provider
  // the host registered (`vscode-vfs` on github.dev, `vscode-test-web` under
  // the web smoke test), so a `file:`-only selector matches nothing.
  client = new LanguageClient(
    "tcl-lsp",
    "Tcl Language Server",
    async () => workerLspTransports(worker),
    buildClientOptions(diffSuppressor, ANY_SCHEME),
  );

  registerStatusBar(context);
  registerCommands(context);
  registerScratchRuleWriteBack(context);

  context.subscriptions.push(
    workspace.onDidChangeConfiguration((e) => {
      if (e.affectsConfiguration("tclLsp.dialect")) {
        updateDialectStatusBar();
        updateIruleContext(window.activeTextEditor);
      }
      if (client && EDITOR_SETTINGS_AFFECTING_FEATURES.some((s) => e.affectsConfiguration(s))) {
        void client.sendNotification("workspace/didChangeConfiguration", {
          settings: { tclLsp: { features: resolveAllFeatureToggles() } },
        });
      }
    }),
  );

  const clientStartTime = Date.now();
  await client.start();
  log(`[timing] client.start: ${Date.now() - clientStartTime}ms`);

  // Live from here on: an upsert plus the ordinary `didChangeWatchedFiles` an
  // editor sends for a file changed outside it.
  sync.watch(budget, (changes) => {
    void client?.sendNotification("workspace/didChangeWatchedFiles", { changes });
  });

  void ensureStickyScrollDefaultModel(context, ch);
  registerPackAssociationSync(context, ch);

  log(`[timing] extension activation: ${Date.now() - activateStart}ms`);
  return { getClient };
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
    client = undefined;
  }
}

function registerStatusBar(context: ExtensionContext): void {
  dialectStatusBarItem = window.createStatusBarItem(StatusBarAlignment.Right, 100);
  dialectStatusBarItem.command = "tclLsp.selectDialect";
  dialectStatusBarItem.tooltip = "Tcl dialect -- click to change";
  updateDialectStatusBar();
  context.subscriptions.push(dialectStatusBarItem);

  const extVersion = context.extension.packageJSON.version as string;
  versionStatusBarItem = window.createStatusBarItem(StatusBarAlignment.Right, 99);
  versionStatusBarItem.text = `tcl-lsp v${extVersion} (web)`;
  versionStatusBarItem.tooltip = `Tcl Language Server v${extVersion} — running in the browser`;
  context.subscriptions.push(versionStatusBarItem);

  context.subscriptions.push(
    window.onDidChangeActiveTextEditor((editor) => onActiveEditorChanged(editor)),
  );
  onActiveEditorChanged(window.activeTextEditor);
}

/**
 * The status bar reports the *configured* dialect.
 *
 * Per-document detection (the `# tcl-dialect:` directive, the shebang, the
 * language id) is the server's job and it runs it per document either way; the
 * node entry additionally mirrors it client-side for the label, using the
 * generated `LANGUAGE_ID_DIALECTS` table that `cargo xtask
 * gen-editor-extensions` writes into `extension.ts` by path. Mirroring it here
 * too would mean either duplicating that table or moving the generator's
 * target, so the web label stays with the configured value until the table has
 * a shared home.
 */
function updateDialectStatusBar(): void {
  const dialect = workspace.getConfiguration("tclLsp").get<string>("dialect", DEFAULT_DIALECT);
  dialectStatusBarItem.text = `$(symbol-misc) ${DIALECT_LABELS[dialect] ?? dialect}`;
}

/**
 * Keep the `tclLsp.isIruleDialect` context key current.
 *
 * Two editor-menu entries are gated on it (`tclLsp.insertIruleEventSkeleton`,
 * `tclLsp.translateXc`), so a host that never sets it registers those commands
 * and then hides them from the menu that is meant to offer them.
 *
 * The language id is the node entry's own highest-priority dialect signal, and
 * it is the one signal available here without the generated
 * `LANGUAGE_ID_DIALECTS` table; the configured dialect covers a file typed as
 * plain `tcl` in an iRules workspace.
 */
function updateIruleContext(editor: TextEditor | undefined): void {
  const configured = workspace.getConfiguration("tclLsp").get<string>("dialect", DEFAULT_DIALECT);
  const isIrule = editor?.document.languageId === "tcl-irule" || configured === "f5-irules";
  void commands.executeCommand("setContext", "tclLsp.isIruleDialect", isIrule);
}

function onActiveEditorChanged(editor: TextEditor | undefined): void {
  updateIruleContext(editor);
  if (editor && isTclLanguage(editor.document.languageId)) {
    dialectStatusBarItem.show();
    versionStatusBarItem.show();
  } else {
    dialectStatusBarItem.hide();
    versionStatusBarItem.hide();
  }
}

function registerCommands(context: ExtensionContext): void {
  context.subscriptions.push(
    commands.registerCommand("tclLsp.restartServer", restartServer),
    commands.registerCommand("tclLsp.selectDialect", selectDialect),
    commands.registerCommand("tclLsp.optimiseDocument", optimiseDocument),
    commands.registerCommand("tclLsp.showOptimisations", showOptimisations),
    commands.registerCommand("tclLsp.fixAllSafeIssues", fixAllSafeIssues),
    commands.registerCommand("tclLsp.toggleDiagnostics", () =>
      toggle("tclLsp.features", "diagnostics", "Tcl diagnostics"),
    ),
    commands.registerCommand("tclLsp.toggleOptimiser", () =>
      toggle("tclLsp.optimiser", "enabled", "Tcl optimiser suggestions"),
    ),
    commands.registerCommand("tclLsp.toggleAi", () =>
      toggle("tclLsp.ai", "enabled", "Tcl AI features"),
    ),
    commands.registerCommand("tclLsp.insertPackageRequire", insertPackageRequire),
    commands.registerCommand("tclLsp.insertIruleEventSkeleton", insertIruleEventSkeleton),
    commands.registerCommand("tclLsp.insertTemplateSnippet", () =>
      insertTemplateSnippet(context.extensionUri),
    ),
    commands.registerCommand("tclLsp.formatDocument", formatDocument),
    commands.registerCommand("tclLsp.minifyDocument", minifyDocument),
    commands.registerCommand("tclLsp.minimizeDiagnostic", minimizeDiagnostic),
    commands.registerCommand("tclLsp.translateXc", translateXc),
    // The BIG-IP workflow. Every one of these is a server command plus editor
    // work, with no filesystem and no process anywhere in it, so it runs here
    // unchanged. (`tclLsp.extractAllRules` is the exception — it writes the
    // extracted rules out — and is listed in WEB_UNSUPPORTED.)
    commands.registerCommand("tclLsp.extractRule", extractRuleAtCursor),
    commands.registerCommand("tclLsp.extractRulePick", extractRulePick),
    commands.registerCommand("tclLsp.extractLinkedObjects", extractLinkedObjectsAtCursor),
    commands.registerCommand("tclLsp.bigipCleanup", generateBigipCleanupScript),
    commands.registerCommand("tclLsp.renamePartition", renamePartition),
    commands.registerCommand("tclLsp.escapeSelection", () =>
      transformSelection(escapeTclText, "Escaped", "escape"),
    ),
    commands.registerCommand("tclLsp.unescapeSelection", () =>
      transformSelection(unescapeTclText, "Unescaped", "unescape"),
    ),
    commands.registerCommand("tclLsp.base64EncodeSelection", () =>
      transformSelection(base64EncodeText, "Base64-encoded", "base64-encode"),
    ),
    commands.registerCommand("tclLsp.base64DecodeSelection", () =>
      transformSelection(base64DecodeText, "Base64-decoded", "base64-decode"),
    ),
    commands.registerCommand(
      "tcl-lsp.showReferences",
      async (uriString: string, position: JsonPosition, locations: ReadonlyArray<JsonLocation>) => {
        const args = convertShowReferencesArgs(uriString, position, locations);
        await commands.executeCommand(
          "editor.action.showReferences",
          args.uri,
          args.position,
          args.locations,
        );
      },
    ),
    commands.registerCommand("tclLsp.insertIrule", async (code: string) => {
      const doc = await workspace.openTextDocument({ language: "tcl", content: code });
      await window.showTextDocument(doc);
    }),
    commands.registerCommand("tclLsp.applyFix", async (code: string, uriString: string) => {
      const uri = vscode.Uri.parse(uriString);
      const doc = await workspace.openTextDocument(uri);
      const editor = await window.showTextDocument(doc);
      await editor.edit((edit) => edit.replace(new Range(0, 0, doc.lineCount, 0), code));
    }),
    commands.registerCommand(
      "tclLsp.renameSymbolAtPosition",
      async (line: number, startChar: number, endChar: number) => {
        const editor = window.activeTextEditor;
        if (!editor) return;
        const pos = new vscode.Position(line, startChar);
        const endPos = new vscode.Position(line, endChar);
        editor.selection = new vscode.Selection(pos, endPos);
        editor.revealRange(new Range(pos, endPos));
        await commands.executeCommand("editor.action.rename");
      },
    ),
    commands.registerCommand("tclLsp.generateDocstring", generateDocstring),
  );

  // Everything the manifest contributes that this host cannot honour answers
  // with the reason rather than "command not found".
  for (const [command, reason] of Object.entries(WEB_UNSUPPORTED)) {
    context.subscriptions.push(
      commands.registerCommand(command, () => {
        window.showWarningMessage(
          `Tcl LSP: "${command}" is desktop-only in this release — ${reason}.`,
        );
      }),
    );
  }
}

// Commands

async function restartServer(): Promise<void> {
  if (client) {
    await client.stop();
    await client.start();
    window.showInformationMessage("Tcl Language Server restarted.");
  }
}

async function selectDialect(): Promise<void> {
  const current = workspace.getConfiguration("tclLsp").get<string>("dialect", DEFAULT_DIALECT);
  const items = Object.entries(DIALECT_LABELS).map(([value, label]) => ({
    label,
    description: value,
    value,
  }));
  const picked = await window.showQuickPick(items, {
    title: "Select Tcl Dialect",
    placeHolder: `Current dialect: ${DIALECT_LABELS[current] ?? current}`,
  });
  if (!picked) {
    return;
  }
  const target = workspace.workspaceFolders?.length ? undefined : vscode.ConfigurationTarget.Global;
  await workspace.getConfiguration("tclLsp").update("dialect", picked.value, target);
  updateDialectStatusBar();
}

async function toggle(section: string, key: string, label: string): Promise<void> {
  const config = workspace.getConfiguration(section);
  const current = config.get<boolean>(key, true);
  await config.update(key, !current, undefined);
  window.showInformationMessage(`${label} ${!current ? "enabled" : "disabled"}.`);
}

function activeTclEditor(action: string): vscode.TextEditor | undefined {
  const editor = window.activeTextEditor;
  if (!editor || !isTclLanguage(editor.document.languageId)) {
    window.showWarningMessage(`Open a Tcl file to ${action}.`);
    return undefined;
  }
  return editor;
}

async function executeServerCommand<T>(command: string, args: unknown[]): Promise<T | null> {
  if (!client) {
    window.showWarningMessage("The Tcl language server is not running.");
    return null;
  }
  return (await client.sendRequest("workspace/executeCommand", {
    command,
    arguments: args,
  })) as T | null;
}

async function optimiseDocument(): Promise<void> {
  const editor = activeTclEditor("run optimisations");
  if (!editor) return;
  const result = await executeServerCommand<{
    optimisations: Array<Record<string, unknown>>;
    source: string;
  }>("tcl-lsp.optimiseDocument", [editor.document.uri.toString()]);
  if (!result?.optimisations?.length) {
    window.showInformationMessage("No optimisations found.");
    return;
  }
  const count = result.optimisations.length;
  const proceed = await window.showInformationMessage(
    `Apply ${count} optimisation${count === 1 ? "" : "s"}?`,
    "Apply",
    "Cancel",
  );
  if (proceed !== "Apply") return;
  const fullRange = editor.document.validateRange(new Range(0, 0, Infinity, Infinity));
  await editor.edit((editBuilder) => editBuilder.replace(fullRange, result.source));
  window.showInformationMessage(`Applied ${count} optimisation${count === 1 ? "" : "s"}.`);
}

async function showOptimisations(): Promise<void> {
  const editor = activeTclEditor("view optimisations");
  if (!editor) return;
  const result = await executeServerCommand<{ optimisations: Array<Record<string, unknown>> }>(
    "tcl-lsp.optimiseDocument",
    [editor.document.uri.toString()],
  );
  if (!result?.optimisations?.length) {
    window.showInformationMessage("No optimisations found.");
    return;
  }
  const items = result.optimisations.map((opt) => ({
    label: `$(lightbulb) ${opt.code as string}`,
    description: opt.message as string,
    detail: opt.hintOnly
      ? `Line ${(opt.startLine as number) + 1}: (hint — no auto-fix)`
      : `Line ${(opt.startLine as number) + 1}: ${opt.replacement as string}`,
  }));
  await window.showQuickPick(items, {
    placeHolder: `${items.length} optimisation suggestion${items.length === 1 ? "" : "s"}`,
    title: "Optimisation Suggestions",
    canPickMany: false,
  });
}

async function fixAllSafeIssues(): Promise<void> {
  const editor = activeTclEditor("apply safe fixes");
  if (!editor) return;
  const result = await executeServerCommand<{
    source: string;
    applied: Array<{ code: string; description: string; safety: string }>;
  }>("tcl-lsp.fixAllSafeIssues", [editor.document.uri.toString()]);
  if (!result?.applied?.length) {
    window.showInformationMessage("No safe auto-fixes available.");
    return;
  }
  const proceed = await window.showInformationMessage(
    `Apply ${result.applied.length} safe fix${result.applied.length === 1 ? "" : "es"}?`,
    "Apply",
    "Cancel",
  );
  if (proceed !== "Apply") return;
  const fullRange = editor.document.validateRange(new Range(0, 0, Infinity, Infinity));
  await editor.edit((editBuilder) => editBuilder.replace(fullRange, result.source));
  window.showInformationMessage(
    `Applied ${result.applied.length} safe fix${result.applied.length === 1 ? "" : "es"}.`,
  );
}

async function formatDocument(): Promise<void> {
  if (!activeTclEditor("format")) return;
  await commands.executeCommand("editor.action.formatDocument");
}

async function minifyDocument(): Promise<void> {
  const editor = activeTclEditor("minify");
  if (!editor) return;
  const mode = await window.showQuickPick(
    [
      {
        label: "Minify",
        description: "Strip comments and collapse whitespace",
        args: [false, false, false],
      },
      {
        label: "Minify + Compact Names",
        description: "Also shorten local variable names (proc names stay — public identities)",
        args: [true, false, false],
      },
      {
        label: "Minify + Compact Names (Isolated)",
        description: "Self-contained script: also shorten proc names and global variables",
        args: [true, false, true],
      },
      {
        label: "Aggressive",
        description: "Optimise, compact, and alias for maximum compression (adds helper variables)",
        args: [false, true, false],
      },
      {
        label: "Aggressive + Isolated",
        description: "Maximum compression — also compact proc names and global-scope variables",
        args: [false, true, true],
      },
    ],
    { placeHolder: "Select minification mode" },
  );
  if (!mode) return;

  const result = await executeServerCommand<{
    source: string;
    originalLength: number;
    minifiedLength: number;
    symbolMap?: string;
    optimisationsApplied?: number;
  }>("tcl-lsp.minifyDocument", [editor.document.uri.toString(), ...mode.args]);
  if (!result?.source) {
    window.showInformationMessage("Nothing to minify.");
    return;
  }
  const saved = result.originalLength - result.minifiedLength;
  const pct = result.originalLength > 0 ? ((saved / result.originalLength) * 100).toFixed(1) : "0";
  const proceed = await window.showInformationMessage(
    `Minify document? Saves ${saved} characters (${pct}%).`,
    "Apply",
    "Cancel",
  );
  if (proceed !== "Apply") return;
  const fullRange = editor.document.validateRange(new Range(0, 0, Infinity, Infinity));
  await editor.edit((editBuilder) => editBuilder.replace(fullRange, result.source));
  window.showInformationMessage(
    `Minified: ${result.originalLength} → ${result.minifiedLength} characters.`,
  );
  if (result.symbolMap) {
    const doc = await workspace.openTextDocument({ content: result.symbolMap, language: "text" });
    await window.showTextDocument(doc, { viewColumn: vscode.ViewColumn.Beside, preview: true });
  }
}

async function minimizeDiagnostic(uri?: string, code?: string): Promise<void> {
  const docUri = uri ?? window.activeTextEditor?.document.uri.toString();
  if (!docUri || !code) {
    window.showWarningMessage("Create minimal repro: missing document or diagnostic code.");
    return;
  }
  const result = await executeServerCommand<{
    code: string;
    source: string;
    originalLines: number;
    reducedLines: number;
    renamed: boolean;
    reproduces: boolean;
  }>("tcl-lsp.minimizeDiagnostic", [docUri, code]);
  if (!result?.source) {
    window.showInformationMessage(`Could not build a minimal repro for ${code}.`);
    return;
  }
  const header =
    `# Minimal reproducer for ${result.code}\n` +
    `# ${result.originalLines} → ${result.reducedLines} lines` +
    `${result.renamed ? ", identifiers renamed" : ""}` +
    `${result.reproduces ? "" : " (WARNING: does not reproduce)"}\n`;
  const doc = await workspace.openTextDocument({
    content: header + result.source,
    language: "tcl",
  });
  await window.showTextDocument(doc, { preview: false });
}

async function translateXc(): Promise<void> {
  const editor = activeTclEditor("translate to F5 XC");
  if (!editor) return;
  const source = editor.document.getText();
  if (!source.trim()) {
    window.showWarningMessage("The current file is empty.");
    return;
  }
  const result = await executeServerCommand<Record<string, unknown>>("tcl-lsp.xcTranslate", [
    source,
    "both",
  ]);
  if (!result || result.error) {
    window.showErrorMessage(
      `XC translation failed: ${(result?.error as string) ?? "unknown error"}`,
    );
    return;
  }
  const terraform = (result.terraform as string) ?? "";
  if (terraform) {
    const hclDoc = await workspace.openTextDocument({ content: terraform, language: "terraform" });
    await window.showTextDocument(hclDoc, {
      preview: false,
      viewColumn: vscode.ViewColumn.Beside,
      preserveFocus: true,
    });
  }
  if (result.json_api) {
    const jsonDoc = await workspace.openTextDocument({
      content: JSON.stringify(result.json_api, null, 2),
      language: "json",
    });
    await window.showTextDocument(jsonDoc, {
      preview: false,
      viewColumn: vscode.ViewColumn.Beside,
      preserveFocus: true,
    });
  }
  window.showInformationMessage(
    `XC Translation: ${((result.coverage_pct as number) ?? 0).toFixed(1)}% coverage — ` +
      `${(result.translatable_count as number) ?? 0} translatable, ` +
      `${(result.untranslatable_count as number) ?? 0} untranslatable`,
  );
}

function escapeRegExp(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function packageInsertLine(document: vscode.TextDocument): number {
  let line = 0;
  if (document.lineCount > 0 && document.lineAt(0).text.startsWith("#!")) {
    line = 1;
  }
  while (line < document.lineCount) {
    if (!/^\s*package\s+require\b/.test(document.lineAt(line).text)) {
      break;
    }
    line += 1;
  }
  return line;
}

async function insertPackageRequire(): Promise<void> {
  const editor = activeTclEditor("insert a package requirement");
  if (!editor) return;
  const wordRange = editor.document.getWordRangeAtPosition(editor.selection.active, /[\w:]+/);
  const symbol = wordRange ? editor.document.getText(wordRange) : "";

  let suggestions =
    (
      await executeServerCommand<{ suggestions?: string[] }>("tcl-lsp.suggestPackagesForSymbol", [
        symbol,
      ])
    )?.suggestions ?? [];
  if (suggestions.length === 0) {
    suggestions =
      (await executeServerCommand<{ packages?: string[] }>("tcl-lsp.listKnownPackages", []))
        ?.packages ?? [];
  }
  if (suggestions.length === 0) {
    window.showInformationMessage(
      "No known packages found. Configure tclLsp.libraryPaths to enable package discovery.",
    );
    return;
  }

  const picked = await window.showQuickPick(suggestions, {
    placeHolder: symbol ? `Select package to require for '${symbol}'` : "Select package to require",
    title: "Insert package require",
  });
  if (!picked) return;

  const existing = new RegExp(`^\\s*package\\s+require\\s+${escapeRegExp(picked)}(?:\\s|$)`, "m");
  if (existing.test(editor.document.getText())) {
    window.showInformationMessage(`'package require ${picked}' already exists.`);
    return;
  }
  const insertAt = packageInsertLine(editor.document);
  const prefix =
    insertAt > 0 && editor.document.lineAt(insertAt - 1).text.trim() !== "" ? "\n" : "";
  const insertPos =
    insertAt < editor.document.lineCount
      ? new vscode.Position(insertAt, 0)
      : editor.document.lineAt(editor.document.lineCount - 1).range.end;
  const applied = await editor.edit((editBuilder) =>
    editBuilder.insert(insertPos, `${prefix}package require ${picked}\n`),
  );
  window.showInformationMessage(
    applied ? `Inserted 'package require ${picked}'.` : "Failed to insert package require.",
  );
}

async function insertIruleEventSkeleton(): Promise<void> {
  const picked = await window.showQuickPick(
    COMMON_IRULE_EVENTS.map((eventInfo) => ({
      label: eventInfo.name,
      description: eventInfo.description,
    })),
    {
      title: "Insert iRule Event Skeleton",
      placeHolder: "Select one or more events to scaffold",
      canPickMany: true,
      ignoreFocusOut: true,
    },
  );
  if (!picked || picked.length === 0) return;
  const skeleton = buildIruleEventSkeleton(picked.map((entry) => entry.label));
  if (!skeleton) {
    window.showWarningMessage("Unable to build iRule skeleton from the selected events.");
    return;
  }
  const doc = await workspace.openTextDocument({ language: "tcl-irule", content: skeleton });
  await window.showTextDocument(doc);
}

async function insertTemplateSnippet(extensionUri: vscode.Uri): Promise<void> {
  const catalogUri = vscode.Uri.joinPath(extensionUri, ...TEMPLATE_SNIPPET_RELATIVE_PATH);
  let snippets;
  try {
    const bytes = await workspace.fs.readFile(catalogUri);
    snippets = parseTemplateSnippetCatalog(new TextDecoder("utf-8").decode(bytes));
  } catch (error) {
    const message = error instanceof Error ? error.message : "Unknown error";
    window.showWarningMessage(`Failed to load Tcl snippets: ${message}`);
    return;
  }
  if (snippets.length === 0) {
    window.showWarningMessage("No Tcl snippets available.");
    return;
  }
  const picked = await window.showQuickPick(
    snippets.map((snippet) => ({
      label: snippet.name,
      description: snippet.description,
      detail: `prefix: ${snippet.prefix}`,
      snippet,
    })),
    {
      title: "Insert Tcl Template Snippet",
      placeHolder: "Select a snippet template to insert",
      ignoreFocusOut: true,
    },
  );
  if (!picked) return;

  let editor = window.activeTextEditor;
  if (!editor || !isTclLanguage(editor.document.languageId)) {
    const doc = await workspace.openTextDocument({ language: "tcl", content: "" });
    editor = await window.showTextDocument(doc);
  }
  await editor.insertSnippet(
    new vscode.SnippetString(renderTemplateSnippet(picked.snippet)),
    editor.selection,
  );
}

async function generateDocstring(): Promise<void> {
  const editor = window.activeTextEditor;
  if (!editor) return;
  const pos = editor.selection.active;
  const actions = await commands.executeCommand<vscode.CodeAction[]>(
    "vscode.executeCodeActionProvider",
    editor.document.uri,
    new Range(pos, pos),
  );
  const docAction = actions?.find((a) => a.title.startsWith("Generate docstring"));
  if (docAction?.edit) {
    await workspace.applyEdit(docAction.edit);
  } else {
    window.showInformationMessage("No proc found at cursor, or it already has a docstring.");
  }
}

// The BIG-IP workflow — the same server commands the node entry drives, and
// the same editor work around them. Only `extractAllRules` differs, because it
// writes files.
//
// TODO(web): these five and their node twins in `./extension.ts` are the same
// code twice. The natural next step is one shared `bigipCommands` module taking
// a client accessor; it is not done here because moving them would touch the
// node entry's proven path for no behaviour change.

interface RuleInfo {
  name: string;
  fullPath: string;
  body: string;
  bodyStartOffset: number;
  bodyEndOffset: number;
  uri: string;
  blockStartLine?: number;
}

/**
 * Scratch editors opened from a config rule, so a save writes the body back to
 * the configuration file it came from.
 */
const scratchRuleMap = new Map<
  string,
  { sourceUri: string; bodyStartOffset: number; bodyEndOffset: number; originalBody: string }
>();

async function openRuleInScratchEditor(rule: RuleInfo): Promise<void> {
  const doc = await workspace.openTextDocument({ language: "tcl-irule", content: rule.body });
  await window.showTextDocument(doc, { preview: false });
  scratchRuleMap.set(doc.uri.toString(), {
    sourceUri: rule.uri,
    bodyStartOffset: rule.bodyStartOffset,
    bodyEndOffset: rule.bodyEndOffset,
    originalBody: rule.body,
  });
  window.showInformationMessage(`Editing iRule '${rule.name}' — save to write back to config.`);
}

function registerScratchRuleWriteBack(context: ExtensionContext): void {
  context.subscriptions.push(
    workspace.onDidSaveTextDocument(async (doc) => {
      const entry = scratchRuleMap.get(doc.uri.toString());
      if (!entry) {
        return;
      }
      const newBody = doc.getText();
      if (newBody === entry.originalBody) {
        return;
      }
      const ok = await executeServerCommand<boolean>("tcl-lsp.writeRuleBack", [
        entry.sourceUri,
        entry.bodyStartOffset,
        entry.bodyEndOffset,
        newBody,
      ]);
      if (ok) {
        entry.bodyEndOffset += newBody.length - (entry.bodyEndOffset - entry.bodyStartOffset);
        entry.originalBody = newBody;
        window.showInformationMessage("iRule written back to configuration file.");
      } else {
        window.showWarningMessage("Failed to write iRule back to configuration file.");
      }
    }),
    workspace.onDidCloseTextDocument((doc) => {
      scratchRuleMap.delete(doc.uri.toString());
    }),
  );
}

function activeBigipEditor(): vscode.TextEditor | undefined {
  const editor = window.activeTextEditor;
  if (!editor || !client) {
    window.showWarningMessage("Open a BIG-IP configuration file first.");
    return undefined;
  }
  return editor;
}

async function extractRuleAtCursor(): Promise<void> {
  const editor = activeBigipEditor();
  if (!editor) return;
  const rule = await executeServerCommand<RuleInfo>("tcl-lsp.extractRule", [
    editor.document.uri.toString(),
    editor.document.offsetAt(editor.selection.active),
  ]);
  if (!rule) {
    window.showWarningMessage("Cursor is not inside an ltm rule or gtm rule block.");
    return;
  }
  await openRuleInScratchEditor(rule);
}

async function extractRulePick(): Promise<void> {
  const editor = activeBigipEditor();
  if (!editor) return;
  const rules = await executeServerCommand<RuleInfo[]>("tcl-lsp.listRules", [
    editor.document.uri.toString(),
  ]);
  if (!rules?.length) {
    window.showInformationMessage("No ltm rule or gtm rule blocks found in this file.");
    return;
  }
  const picked = await window.showQuickPick(
    rules.map((rule) => ({ label: rule.name, description: rule.fullPath, rule })),
    { placeHolder: "Select an iRule to edit" },
  );
  if (picked) {
    await openRuleInScratchEditor(picked.rule);
  }
}

async function extractLinkedObjectsAtCursor(): Promise<void> {
  const editor = activeBigipEditor();
  if (!editor) return;
  const primaryUri = editor.document.uri.toString();
  const extraOffsets = editor.selections
    .slice(1)
    .map((selection): [string, number] => [primaryUri, editor.document.offsetAt(selection.active)]);
  const result = await executeServerCommand<Record<string, unknown>>(
    "tcl-lsp.extractLinkedObjects",
    [
      primaryUri,
      editor.document.offsetAt(editor.selections[0].active),
      5,
      400,
      extraOffsets.length > 0 ? extraOffsets : null,
    ],
  );
  if (!result) {
    window.showWarningMessage("No BIG-IP object found at cursor.");
    return;
  }
  const doc = await workspace.openTextDocument({
    language: "json",
    content: JSON.stringify(result, null, 2),
  });
  await window.showTextDocument(doc, { preview: false });
}

async function generateBigipCleanupScript(): Promise<void> {
  const editor = activeBigipEditor();
  if (!editor) return;
  const result = await executeServerCommand<{ tmshScript: string } & Record<string, unknown>>(
    "tcl-lsp.bigipCleanup",
    [[editor.document.uri.toString()], null, false],
  );
  if (!result) {
    window.showWarningMessage(
      "No BIG-IP configuration loaded in the workspace.  Open a bigip.conf and try again.",
    );
    return;
  }
  const scriptDoc = await workspace.openTextDocument({
    language: "tcl-bigip",
    content: result.tmshScript,
  });
  await window.showTextDocument(scriptDoc, { preview: false, viewColumn: vscode.ViewColumn.One });
  const reportDoc = await workspace.openTextDocument({
    language: "json",
    content: JSON.stringify(result, null, 2),
  });
  await window.showTextDocument(reportDoc, {
    preview: false,
    viewColumn: vscode.ViewColumn.Beside,
  });
}

async function renamePartition(uriString?: string, oldPartition?: string): Promise<void> {
  const targetUri = uriString ?? window.activeTextEditor?.document.uri.toString();
  const currentName = (oldPartition ?? "").trim();
  if (!targetUri || !currentName) {
    window.showWarningMessage("Place the cursor on a BIG-IP partition stanza first.");
    return;
  }
  const newNameInput = await window.showInputBox({
    title: "Rename BIG-IP Partition",
    prompt: `New name for ${currentName}`,
    value: currentName,
    valueSelection: [0, currentName.length],
    validateInput: (value) => {
      const trimmed = value.trim().replace(/^\/+/, "");
      if (!trimmed) return "Enter a partition name.";
      if (trimmed === "Common") return "Renaming to or from /Common is not supported.";
      if (!/^[A-Za-z0-9_.-]+$/.test(trimmed)) {
        return "Use letters, digits, underscore, dot, or hyphen.";
      }
      return undefined;
    },
  });
  if (newNameInput === undefined) return;
  const newName = newNameInput.trim().replace(/^\/+/, "");
  const result = await executeServerCommand<{
    success: boolean;
    edit?: LspWorkspaceEdit;
    error?: string;
  }>("tcl-lsp.renamePartition", [targetUri, currentName, newName]);
  if (!result?.success || !result.edit) {
    window.showErrorMessage(result?.error ?? "Partition rename did not produce any edits.");
    return;
  }
  if (!(await workspace.applyEdit(workspaceEditFromLsp(result.edit)))) {
    window.showErrorMessage("VS Code rejected the partition rename workspace edit.");
    return;
  }
  window.showInformationMessage(`Renamed partition ${currentName} to ${newName}.`);
}

// Base64 through the platform's own primitives — the node entry keeps its
// `Buffer` implementation, which this must agree with byte for byte.

function base64EncodeText(text: string): string {
  const bytes = new TextEncoder().encode(text);
  let binary = "";
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  return btoa(binary);
}

function base64DecodeText(text: string): string {
  const binary = atob(text.trim());
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return new TextDecoder("utf-8").decode(bytes);
}

/**
 * Keep workspace `files.associations` in step with the extensions the
 * discovered SpecTcl packs claim (issue #1626) — the same push/pull pair the
 * node entry runs, against the same server.
 */
function registerPackAssociationSync(context: ExtensionContext, ch: vscode.OutputChannel): void {
  if (!client) {
    return;
  }
  context.subscriptions.push(
    client.onNotification(
      "tcl-lsp/specPacksReloaded",
      (params: { pack_file_extensions?: PackFileExtension[] }) => {
        void syncPackFileAssociations(context, params?.pack_file_extensions ?? [], (message) =>
          ch.appendLine(message),
        );
      },
    ),
  );
  void (async () => {
    try {
      const result = await executeServerCommand<{
        pack_file_extensions?: PackFileExtension[];
        spec_packs_loaded?: unknown[];
      }>("tcl-lsp.getEffectiveConfig", [""]);
      if (!result?.spec_packs_loaded?.length) {
        return;
      }
      await syncPackFileAssociations(context, result.pack_file_extensions ?? [], (message) =>
        ch.appendLine(message),
      );
    } catch (err) {
      ch.appendLine(`[packs] file-association sync skipped: ${String(err)}`);
    }
  })();
}
