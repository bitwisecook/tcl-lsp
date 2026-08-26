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

import {
  closeSync,
  existsSync,
  fstatSync,
  mkdirSync,
  mkdtempSync,
  openSync,
  readSync,
  rmSync,
  writeFileSync,
} from "fs";
import * as path from "path";
import { execFile } from "child_process";
import { gzipSync } from "zlib";
import { tmpdir } from "os";
import { promisify } from "util";
import * as vscode from "vscode";
import { setDiagramPanelExtensionUri } from "./diagramPanel";
import {
  commands,
  ExtensionContext,
  Range,
  StatusBarAlignment,
  StatusBarItem,
  TextDocument,
  TextEditor,
  window,
  workspace,
} from "vscode";
import { LanguageClient, ServerOptions } from "vscode-languageclient/node";
import {
  buildClientOptions,
  DEFAULT_DIALECT,
  EDITOR_SETTINGS_AFFECTING_FEATURES,
  LspWorkspaceEdit,
  resolveAllFeatureToggles,
  workspaceEditFromLsp,
} from "./clientCore";
import { bundlePlatformDir, resolveRustServer } from "./serverResolution";
import { escapeTclText, transformSelection, unescapeTclText } from "./selectionTransforms";
import { registerIruleParticipant } from "./chat/iruleParticipant";
import { registerTclParticipant } from "./chat/tclParticipant";
import { registerTkParticipant } from "./chat/tkParticipant";
import {
  buildRuntimeValidationChecker,
  resolveRuntimeValidationAdapter,
  runtimeValidationAdapterLabel,
  RuntimeValidationAdapterMode,
} from "./runtimeValidation";
import { convertShowReferencesArgs, JsonLocation, JsonPosition } from "./showReferences";
import {
  buildPackageScaffold,
  packageDirectoryName,
  validateCommandName,
  validatePackageName,
  validateVersion,
} from "./scaffold";
import { buildIruleEventSkeleton, COMMON_IRULE_EVENTS } from "./iruleSkeleton";
import { loadTemplateSnippets, renderTemplateSnippet, TemplateSnippet } from "./templateSnippets";
import {
  explorerDocChanged,
  explorerEditorChanged,
  openCompilerExplorer,
} from "./compilerExplorer";
import { openTkPreview, tkPreviewDocChanged, tkPreviewEditorChanged } from "./tkPreviewPanel";
import { openSpecStudio, prepareSpecStudioStorage } from "./specStudio";
import { registerHighlightingHealthChecks } from "./highlightingHealth";
import { ensureStickyScrollDefaultModel } from "./stickyScrollHealth";
import { DiffDiagnosticsSuppressor } from "./diffAnalysis";
import { TCL_LANGUAGE_IDS, isTclLanguage, tclLanguageIdForPath } from "./languageIds";
import { PackFileExtension, syncPackFileAssociations } from "./packAssociations";

const execFileAsync = promisify(execFile);

let client: LanguageClient;
let dialectStatusBarItem: StatusBarItem;
let versionStatusBarItem: StatusBarItem;
let activeDialect = "tcl8.6";

export function getClient(): LanguageClient {
  return client;
}

export function getActiveDialect(): string {
  return activeDialect;
}

export function isAiEnabled(): boolean {
  return workspace.getConfiguration("tclLsp.ai").get<boolean>("enabled", true);
}

// @generated:dialect-labels:begin
const DIALECT_LABELS: Record<string, string> = {
  bpf: "BPF",
  "cadence-eda-tcl": "Cadence EDA Tcl",
  expect: "Expect",
  "f5-bigip": "F5 BIG-IP",
  "f5-iapps": "F5 iApps",
  "f5-irules": "F5 iRules",
  "f5-tmsh": "F5 tmsh Scripts",
  "intel-quartus-eda-tcl": "Intel Quartus EDA Tcl",
  "mentor-eda-tcl": "Mentor EDA Tcl",
  "microchip-libero-eda-tcl": "Microchip Libero EDA Tcl",
  spectcl: "SpecTcl",
  "synopsys-eda-tcl": "Synopsys EDA Tcl",
  "tcl8.4": "Tcl 8.4",
  "tcl8.5": "Tcl 8.5",
  "tcl8.6": "Tcl 8.6",
  "tcl9.0": "Tcl 9.0",
  "tcl9.1": "Tcl 9.1",
  "xilinx-eda-tcl": "Xilinx EDA Tcl",
};
// @generated:dialect-labels:end

// Sourced from ./languageIds (a vscode-free module) and re-exported so existing
// importers that pull these from ./extension keep working.
export { TCL_LANGUAGE_IDS, isTclLanguage };

/**
 * Map language IDs that imply a specific dialect.
 *
 * Keys are *language ids* (undotted — see `./languageIds`); values are
 * *dialect* names, which keep their dots. The two namespaces are distinct:
 * `tcl84` is what VS Code calls the language, `tcl8.4` is what the server
 * calls the dialect.
 */
// @generated:language-id-dialects:begin -- cargo xtask gen-editor-extensions
const LANGUAGE_ID_DIALECTS: Record<string, string> = {
  "tcl-cadence": "cadence-eda-tcl",
  "tcl-expect": "expect",
  "tcl-bigip": "f5-bigip",
  "tcl-iapp": "f5-iapps",
  "tcl-irule": "f5-irules",
  "tcl-tmsh": "f5-tmsh",
  "tcl-quartus": "intel-quartus-eda-tcl",
  "tcl-mentor": "mentor-eda-tcl",
  "tcl-microchip": "microchip-libero-eda-tcl",
  tclspec: "spectcl",
  "tcl-synopsys": "synopsys-eda-tcl",
  tcl84: "tcl8.4",
  tcl85: "tcl8.5",
  tcl86: "tcl8.6",
  tcl90: "tcl9.0",
  tcl91: "tcl9.1",
  "tcl-xilinx": "xilinx-eda-tcl",
  "tcl-apl": "f5-iapps",
};
// @generated:language-id-dialects:end

const TCL_VERSION_DIALECTS: Record<string, string> = {
  "8.4": "tcl8.4",
  "8.5": "tcl8.5",
  "8.6": "tcl8.6",
  "9.0": "tcl9.0",
  "9.1": "tcl9.1",
};

// LSP wire types (used by rename partitioning) live in ./clientCore, shared
// with the browser entry.

interface RenamePartitionResult {
  success: boolean;
  edit?: LspWorkspaceEdit;
  error?: string;
}

let outputChannel: vscode.OutputChannel;

function getOutputChannel(): vscode.OutputChannel {
  if (!outputChannel) {
    outputChannel = window.createOutputChannel("Tcl Language Server");
  }
  return outputChannel;
}

// The server ladder — explicit path, bundled native binary, dev checkout, and
// finally the universal VSIX's bundled WASI module — lives in
// `./serverResolution`, which is free of the `vscode` API so the whole of it is
// unit-testable outside an extension host.

export async function activate(context: ExtensionContext) {
  setDiagramPanelExtensionUri(context.extensionUri);
  await prepareSpecStudioStorage();
  const activateStart = Date.now();
  const ch = getOutputChannel();
  const config = workspace.getConfiguration("tclLsp");
  const configuredServerPath = config.get<string>("serverPath", "");

  // The extension prefers the native Rust `tcl-lsp-server`.  The universal
  // VSIX bundles one binary per platform under `server/<platform>-<arch>/`; a
  // dev checkout builds it with `cargo build -p tcl-lsp-server` and it is
  // picked up from `target/{release,debug}/`.  An explicit binary can be
  // supplied via `tclLsp.rustServerPath` or the `TCL_LSP_SERVER_BIN` env var
  // (the latter lets test harnesses point at a freshly built binary without
  // writing workspace settings).  Failing all of those, the universal VSIX also
  // carries the same server as a WASI module — see ./wasiServer.
  const resolved = resolveRustServer(
    process.env.TCL_LSP_SERVER_BIN || config.get<string>("rustServerPath", ""),
    configuredServerPath,
    context.extensionPath,
  );
  if (!resolved) {
    window.showErrorMessage(
      `Tcl LSP: no tcl-lsp-server found for ${bundlePlatformDir()}. ` +
        "In a checkout, build it with `cargo build -p tcl-lsp-server` (or `make rust-server`), " +
        "or set 'tclLsp.rustServerPath' to a native binary. A packaged install reaching this " +
        "point ships neither a binary for your OS/architecture nor the WebAssembly (WASI) " +
        "fallback module — install the universal .vsix, which carries it.",
    );
    return;
  }

  // Optional suppression of diagnostics for files viewed only in a diff editor
  // (Git changes, "Compare With…"); gated by
  // `tclLsp.suppressDiagnosticsInDiffEditors` (default off). Installed as the
  // `handleDiagnostics` middleware by `buildClientOptions`.
  const diffSuppressor = new DiffDiagnosticsSuppressor();
  context.subscriptions.push(diffSuppressor);

  // Client options are shared with the browser entry — see ./clientCore.
  let clientOptions = buildClientOptions(diffSuppressor);

  let serverOptions: ServerOptions;
  if (resolved.kind === "native") {
    ch.appendLine(`Using native tcl-lsp-server: ${resolved.path}`);
    serverOptions = {
      command: resolved.path,
      args: [],
      options: { cwd: path.dirname(resolved.path) },
    };
  } else {
    // No native binary for this architecture: run the bundled WASI module under
    // VS Code's WASM host.  Imported here rather than at the top of the file so
    // every platform that DID resolve a binary never evaluates the module or
    // the two `@vscode/wasm-wasi*` packages it pulls in.
    ch.appendLine(
      `No native tcl-lsp-server for ${bundlePlatformDir()} — using the bundled WASI module: ` +
        resolved.modulePath,
    );
    const { startWasiServer } = await import("./wasiServer.js");
    const wasi = await startWasiServer(context, resolved.modulePath, ch);
    if (!wasi) {
      // The host runtime is missing and the user declined to install it;
      // `startWasiServer` has already said so.
      return;
    }
    serverOptions = wasi.serverOptions;
    clientOptions = { ...clientOptions, uriConverters: wasi.uriConverters };
  }

  client = new LanguageClient("tcl-lsp", "Tcl Language Server", serverOptions, clientOptions);

  // Status bar: dialect indicator

  dialectStatusBarItem = window.createStatusBarItem(StatusBarAlignment.Right, 100);
  dialectStatusBarItem.command = "tclLsp.selectDialect";
  dialectStatusBarItem.tooltip = "Tcl dialect -- click to change";
  updateDialectStatusBar();
  context.subscriptions.push(dialectStatusBarItem);

  // Status bar: version indicator

  const extVersion = context.extension.packageJSON.version as string;
  versionStatusBarItem = window.createStatusBarItem(StatusBarAlignment.Right, 99);
  versionStatusBarItem.text = `tcl-lsp v${extVersion}`;
  versionStatusBarItem.tooltip = `Tcl Language Server v${extVersion}`;
  context.subscriptions.push(versionStatusBarItem);

  // Show/hide based on active editor language.
  context.subscriptions.push(
    window.onDidChangeActiveTextEditor((editor) => {
      onActiveEditorChanged(editor);
      applyDialectForEditor(editor);
      explorerEditorChanged();
      tkPreviewEditorChanged();
    }),
  );
  onActiveEditorChanged(window.activeTextEditor);

  // Warn once when semantic highlighting can't reach a Tcl buffer (wrong
  // language association, or semantic highlighting disabled) — grammar-only
  // colouring mis-highlights keywords inside braced strings (issue #749).
  registerHighlightingHealthChecks(context);

  // Update status bar label when manually changing settings, and re-push
  // resolved feature toggles when the underlying editor globals change.
  // The list of inherited editor settings lives in ./clientCore, shared with
  // the browser entry.
  context.subscriptions.push(
    workspace.onDidChangeConfiguration((e) => {
      if (e.affectsConfiguration("tclLsp.dialect")) {
        // The setting changed (``tclLsp.selectDialect``, or the user editing
        // settings.json).  The server picks it up from its own
        // ``workspace/configuration`` pull; all that is left here is the
        // status-bar label and the iRules context key.  A focused document
        // with its own detected dialect re-asserts the label on the next
        // editor/document event.
        setActiveDialectLabel(
          workspace.getConfiguration("tclLsp").get<string>("dialect", DEFAULT_DIALECT),
        );
      }
      // When a VS Code editor setting that a feature toggle inherits from
      // changes, re-push resolved feature values so the server stays in sync.
      if (client && EDITOR_SETTINGS_AFFECTING_FEATURES.some((s) => e.affectsConfiguration(s))) {
        const resolved = resolveAllFeatureToggles();
        void client.sendNotification("workspace/didChangeConfiguration", {
          settings: { tclLsp: { features: resolved } },
        });
      }
    }),
  );

  // Re-evaluate dialect when active document content changes (for shebang edits).
  context.subscriptions.push(
    workspace.onDidChangeTextDocument((e) => {
      const editor = window.activeTextEditor;
      if (!editor || editor.document.uri.toString() !== e.document.uri.toString()) {
        return;
      }
      if (!isTclLanguage(e.document.languageId)) {
        return;
      }
      const touchesDirectiveLines = e.contentChanges.some(
        (change) => change.range.start.line < DIALECT_DIRECTIVE_SCAN_LINES,
      );
      if (touchesDirectiveLines) {
        applyDialectForDocument(e.document);
      }
      explorerDocChanged();
      tkPreviewDocChanged();
    }),
  );

  // Also handle language-ready open events for newly opened files.
  context.subscriptions.push(
    workspace.onDidOpenTextDocument((document) => {
      if (window.activeTextEditor?.document.uri.toString() === document.uri.toString()) {
        applyDialectForDocument(document);
      }
    }),
  );

  context.subscriptions.push(
    workspace.onDidSaveTextDocument((document) => {
      if (!isTclLanguage(document.languageId)) {
        return;
      }
      const runtimeCfg = workspace.getConfiguration("tclLsp.runtimeValidation");
      if (!runtimeCfg.get<boolean>("enabled", false)) {
        return;
      }
      void runRuntimeValidationForDocument(document, { quiet: true });
    }),
  );

  context.subscriptions.push(
    commands.registerCommand("tclLsp.restartServer", restartServer),
    commands.registerCommand("tclLsp.selectDialect", selectDialect),
    commands.registerCommand("tclLsp.selectTclInstallation", selectTclInstallation),
    commands.registerCommand("tclLsp.exportConfig", exportConfig),
    commands.registerCommand("tclLsp.optimiseDocument", optimiseDocument),
    commands.registerCommand("tclLsp.showOptimisations", showOptimisations),
    commands.registerCommand("tclLsp.fixAllSafeIssues", fixAllSafeIssues),
    commands.registerCommand("tclLsp.toggleDiagnostics", toggleDiagnostics),
    commands.registerCommand("tclLsp.toggleOptimiser", toggleOptimiser),
    commands.registerCommand("tclLsp.toggleAi", toggleAi),
    commands.registerCommand("tclLsp.insertPackageRequire", insertPackageRequire),
    commands.registerCommand("tclLsp.scaffoldPackageStarter", scaffoldPackageStarter),
    commands.registerCommand("tclLsp.insertIruleEventSkeleton", insertIruleEventSkeleton),
    commands.registerCommand("tclLsp.insertTemplateSnippet", () =>
      insertTemplateSnippet(context.extensionPath),
    ),
    commands.registerCommand("tclLsp.runRuntimeValidation", runRuntimeValidation),
    commands.registerCommand("tclLsp.openCompilerExplorer", openCompilerExplorer),
    commands.registerCommand("tclLsp.openSpecStudio", () => openSpecStudio(context, client)),
    commands.registerCommand("tclLsp.openTkPreview", openTkPreview),
    commands.registerCommand("tclLsp.formatDocument", formatDocument),
    commands.registerCommand("tclLsp.minifyDocument", minifyDocument),
    commands.registerCommand("tclLsp.minimizeDiagnostic", minimizeDiagnostic),
    commands.registerCommand("tclLsp.unminifyError", unminifyError),
    commands.registerCommand("tclLsp.escapeSelection", escapeSelection),
    commands.registerCommand("tclLsp.unescapeSelection", unescapeSelection),
    commands.registerCommand("tclLsp.base64EncodeSelection", base64EncodeSelection),
    commands.registerCommand("tclLsp.base64DecodeSelection", base64DecodeSelection),
    commands.registerCommand("tclLsp.copyFileAsBase64", copyFileAsBase64),
    commands.registerCommand("tclLsp.copyFileAsGzipBase64", copyFileAsGzipBase64),
    commands.registerCommand("tclLsp.translateXc", translateXc),
    // Wrapper for the built-in editor.action.showReferences: the LSP server
    // emits this command on its code lenses with URI/Position/Location args
    // serialised as JSON primitives, but the built-in command validates its
    // arguments against vscode.Uri/Position/Array constraints and rejects
    // raw strings/plain objects. Convert here before delegating.
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
  );

  context.subscriptions.push(
    commands.registerCommand("tclLsp.insertIrule", async (code: string) => {
      const doc = await vscode.workspace.openTextDocument({ language: "tcl", content: code });
      await vscode.window.showTextDocument(doc);
    }),
    commands.registerCommand("tclLsp.applyFix", async (code: string, uriString: string) => {
      const uri = vscode.Uri.parse(uriString);
      const doc = await vscode.workspace.openTextDocument(uri);
      const editor = await vscode.window.showTextDocument(doc);
      const fullRange = new Range(0, 0, doc.lineCount, 0);
      await editor.edit((edit) => edit.replace(fullRange, code));
    }),
    commands.registerCommand("tclLsp.extractRule", extractRuleAtCursor),
    commands.registerCommand("tclLsp.extractRulePick", extractRulePick),
    commands.registerCommand("tclLsp.extractAllRules", extractAllRules),
    commands.registerCommand("tclLsp.extractLinkedObjects", extractLinkedObjectsAtCursor),
    commands.registerCommand("tclLsp.bigipCleanup", generateBigipCleanupScript),
    commands.registerCommand("tclLsp.renamePartition", renamePartition),
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
    commands.registerCommand("tclLsp.generateDocstring", async () => {
      const editor = window.activeTextEditor;
      if (!editor) return;
      const pos = editor.selection.active;
      const range = new Range(pos, pos);
      const actions = await commands.executeCommand<vscode.CodeAction[]>(
        "vscode.executeCodeActionProvider",
        editor.document.uri,
        range,
      );
      const docAction = actions?.find((a) => a.title.startsWith("Generate docstring"));
      if (docAction?.edit) {
        await vscode.workspace.applyEdit(docAction.edit);
      } else {
        window.showInformationMessage("No proc found at cursor, or it already has a docstring.");
      }
    }),
  );

  const clientStartTime = Date.now();
  await client.start();
  ch.appendLine(`[timing] client.start: ${Date.now() - clientStartTime}ms`);
  void ensureStickyScrollDefaultModel(context, ch);
  applyDialectForEditor(window.activeTextEditor);
  registerPackAssociationSync(context, ch);

  try {
    registerIruleParticipant(context);
    registerTclParticipant(context);
    registerTkParticipant(context);
  } catch {
    // Chat Participants API unavailable (e.g. Copilot not installed)
  }

  // Screenshot demo -- only included in screenshot builds.
  // __SCREENSHOT_MODE__ is set by esbuild --define; in the normal bundle it
  // is false and esbuild dead-code-eliminates this entire block.
  if (__SCREENSHOT_MODE__) {
    const { registerScreenshotDemo } = await import("./screenshotDemo.js");
    registerScreenshotDemo(context);
  }

  ch.appendLine(`[timing] extension activation: ${Date.now() - activateStart}ms`);
  return { getClient, applyDialectForDocument };
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
  }
}

/**
 * Keep workspace `files.associations` in step with the extensions the
 * discovered SpecTcl packs claim (issue #1626).
 *
 * Driven by the server's `tcl-lsp/specPacksReloaded` push, not by watching the
 * filesystem here. The client cannot derive the right moment on its own: it
 * sees the same `.tclspec` write the server does, but not when the server has
 * finished reloading, so reacting to the event directly read the *previous*
 * pack set with nothing scheduled to ask again. A workspace-scoped watcher
 * also never fires for a pack under an absolute `tclLsp.specPacks` root, which
 * the server watches and it cannot. The push settles both, and it arrives
 * after every consequence of the reload has landed (review finding P1-2).
 *
 * One pull still happens, immediately: the server notifies on every reload
 * including the startup one, but a client that finished starting *after* that
 * notification was emitted would otherwise wait for an edit that may never
 * come.
 */
function registerPackAssociationSync(context: ExtensionContext, ch: vscode.OutputChannel): void {
  if (client) {
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
  }
  void syncPackAssociationsNow(context, ch);
}

/**
 * One reconciliation pass driven by a pull rather than the push.
 *
 * Only the startup catch-up uses this; everything after it arrives as a
 * notification. Skips entirely until the server reports a completed reload,
 * because reconciliation is destructive by design — a pass taken against a
 * server that has simply not discovered anything yet would retire every
 * association it had written.
 */
async function syncPackAssociationsNow(
  context: ExtensionContext,
  ch: vscode.OutputChannel,
): Promise<void> {
  if (!client) {
    return;
  }
  try {
    // The pack set is process-global on the server — one reload covers the
    // bundled tier and every workspace folder — so this asks with no URI
    // rather than implying a per-folder answer.
    const result = (await client.sendRequest("workspace/executeCommand", {
      command: "tcl-lsp.getEffectiveConfig",
      arguments: [""],
    })) as {
      pack_file_extensions?: PackFileExtension[];
      spec_packs_loaded?: unknown[];
    } | null;
    if (!result?.spec_packs_loaded?.length) {
      return;
    }
    await syncPackFileAssociations(context, result.pack_file_extensions ?? [], (message) =>
      ch.appendLine(message),
    );
  } catch (err) {
    // A server still starting (or already stopped) has nothing to say about
    // packs yet, and the notification will bring the answer when it does.
    ch.appendLine(`[packs] file-association sync skipped: ${String(err)}`);
  }
}

// Helpers

function updateDialectStatusBar(): void {
  const label = DIALECT_LABELS[activeDialect] ?? activeDialect;
  dialectStatusBarItem.text = `$(symbol-misc) ${label}`;
}

function onActiveEditorChanged(editor: TextEditor | undefined): void {
  if (editor && isTclLanguage(editor.document.languageId)) {
    dialectStatusBarItem.show();
    versionStatusBarItem.show();
  } else {
    dialectStatusBarItem.hide();
    versionStatusBarItem.hide();
  }
}

// Command handlers

async function restartServer(): Promise<void> {
  if (client) {
    await client.stop();
    await client.start();
    window.showInformationMessage("Tcl Language Server restarted.");
  }
}

async function exportConfig(): Promise<void> {
  if (!client) {
    return;
  }
  const result = (await client.sendRequest("workspace/executeCommand", {
    command: "tcl-lsp.exportConfig",
    arguments: [],
  })) as { success: boolean; path?: string; error?: string } | null;
  if (result?.success) {
    window.showInformationMessage(`Settings exported to ${result.path}`);
  } else {
    window.showErrorMessage(`Failed to export settings: ${result?.error ?? "unknown error"}`);
  }
}

async function selectDialect(): Promise<void> {
  const items = Object.entries(DIALECT_LABELS).map(([value, label]) => ({
    label,
    description: value,
    value,
  }));

  const current = activeDialect || DEFAULT_DIALECT;
  const picked = await new Promise<(typeof items)[number] | undefined>((resolve) => {
    const quickPick = window.createQuickPick<(typeof items)[number]>();
    quickPick.title = "Select Tcl Dialect";
    quickPick.placeholder = `Current dialect: ${DIALECT_LABELS[current] ?? current}`;
    quickPick.items = items;
    const currentItem = items.find((item) => item.value === current);
    if (currentItem) {
      quickPick.activeItems = [currentItem];
    }

    let done = false;
    const finish = (value: (typeof items)[number] | undefined) => {
      if (done) {
        return;
      }
      done = true;
      disposeAccept.dispose();
      disposeHide.dispose();
      quickPick.dispose();
      resolve(value);
    };

    const disposeAccept = quickPick.onDidAccept(() => {
      const [selected] =
        quickPick.selectedItems.length > 0 ? quickPick.selectedItems : quickPick.activeItems;
      finish(selected);
    });
    const disposeHide = quickPick.onDidHide(() => finish(undefined));
    quickPick.show();
  });

  if (picked) {
    const target = workspace.workspaceFolders?.length
      ? undefined
      : vscode.ConfigurationTarget.Global;
    await workspace.getConfiguration("tclLsp").update("dialect", picked.value, target);
    await setServerDialect(picked.value);
  }
}

interface TclInstallation {
  version: string;
  tclLibrary: string;
  autoPath: string[];
}

interface TclInstallationsResult {
  installations: TclInstallation[];
  activeAutoPath: string[];
  editorLibraryPaths: string[];
}

/**
 * "Select Tcl Installation" — ask the server which Tcl installations it
 * discovered on disk (it never runs tclsh), let the user pick one or enter a
 * custom path, and write the chosen `auto_path` root(s) to
 * `tclLsp.libraryPaths` so the package database (and #723 W120 resolution) can
 * see system-installed packages like Tk.
 */
async function selectTclInstallation(): Promise<void> {
  const client = getClient();
  if (!client) {
    window.showWarningMessage("Tcl language server is not running.");
    return;
  }
  const result = (await client.sendRequest("workspace/executeCommand", {
    command: "tcl-lsp.listTclInstallations",
    arguments: [],
  })) as TclInstallationsResult | null;

  const installs = result?.installations ?? [];
  type Item = vscode.QuickPickItem & { paths?: string[]; custom?: boolean; clear?: boolean };
  const items: Item[] = installs.map((inst) => ({
    label: inst.version ? `Tcl ${inst.version}` : "Tcl",
    description: inst.tclLibrary,
    detail: inst.autoPath.join("  ·  "),
    paths: inst.autoPath,
  }));
  items.push({ label: "$(folder) Enter a custom path…", custom: true });
  if ((result?.editorLibraryPaths?.length ?? 0) > 0) {
    items.push({ label: "$(clear-all) Clear configured library paths", clear: true });
  }

  const picked = await window.showQuickPick(items, {
    title: "Select Tcl Installation",
    placeHolder:
      installs.length > 0
        ? "Pick a discovered Tcl installation, or enter your own path"
        : "No installations discovered — enter a path to your Tcl library",
  });
  if (!picked) {
    return;
  }

  let paths: string[] | undefined;
  if (picked.clear) {
    paths = [];
  } else if (picked.custom) {
    const entered = await window.showInputBox({
      title: "Tcl library / auto_path directory",
      prompt: "Absolute path to a Tcl library directory (containing pkgIndex.tcl files)",
      ignoreFocusOut: true,
    });
    if (entered === undefined) {
      return;
    }
    paths = entered.trim() ? [entered.trim()] : [];
  } else {
    paths = picked.paths ?? [];
  }

  const target = workspace.workspaceFolders?.length ? undefined : vscode.ConfigurationTarget.Global;
  await workspace.getConfiguration("tclLsp").update("libraryPaths", paths, target);
  window.showInformationMessage(
    paths.length > 0
      ? `Tcl library paths set to: ${paths.join(", ")}`
      : "Cleared configured Tcl library paths.",
  );
}

/** Regex for the ``# tcl-dialect: <dialect>`` per-file directive. */
const DIALECT_DIRECTIVE_RE = /^#\s*tcl-dialect:\s*(\S+)/i;

/** Maximum number of lines to scan for a ``# tcl-dialect:`` directive. */
const DIALECT_DIRECTIVE_SCAN_LINES = 5;

function detectDialectFromDocument(document: TextDocument): string {
  // 1. Language ID from editor (highest priority — unambiguous).
  const langDialect = LANGUAGE_ID_DIALECTS[document.languageId];
  if (langDialect) {
    return langDialect;
  }

  // 2. Comment directive ``# tcl-dialect: <dialect>`` in the first few lines.
  const scanLines = Math.min(document.lineCount, DIALECT_DIRECTIVE_SCAN_LINES);
  for (let i = 0; i < scanLines; i++) {
    const directiveMatch = document.lineAt(i).text.match(DIALECT_DIRECTIVE_RE);
    if (directiveMatch) {
      const candidate = directiveMatch[1].toLowerCase();
      if (candidate in DIALECT_LABELS) {
        return candidate;
      }
    }
  }

  // 3. Shebang detection. `wish` counts alongside `tclsh`: Tk is a library in
  //    this model, not a dialect, so a `#!/usr/bin/wish8.6` script is a Tcl
  //    8.6 script for exactly the reason a `tclsh8.6` one is. Mirrors the
  //    server's `SHEBANG_TCL_SHELLS` (issue #1625).
  if (document.lineCount > 0) {
    const firstLine = document.lineAt(0).text;
    if (/^#!.*\bexpect\b/i.test(firstLine)) {
      return "expect";
    }
    const shebangMatch = firstLine.match(/^#!.*\b(?:tclsh|wish)(\d+\.\d+)\b/i);
    if (shebangMatch) {
      const versionDialect = TCL_VERSION_DIALECTS[shebangMatch[1]];
      if (versionDialect) {
        return versionDialect;
      }
    }
  }

  // 4. The file's own name — its whole basename, then its extension — through
  //    the generated catalogue projection, so every registered extension is
  //    covered. A hand-written switch here used to answer for 6 of the 25 and
  //    sat *above* the directive and shebang tiers, which is both narrower
  //    and more eager than the server's own order (issue #1625); it now sits
  //    where the server puts it, since a file's content is a stronger signal
  //    than its name.
  const pathLanguage = tclLanguageIdForPath(document.fileName);
  if (pathLanguage) {
    const pathDialect = LANGUAGE_ID_DIALECTS[pathLanguage];
    if (pathDialect) {
      return pathDialect;
    }
  }

  // 5. User's configured default dialect (tclLsp.dialect setting).
  const configured = workspace.getConfiguration("tclLsp").get<string>("dialect", "");
  if (configured && configured in DIALECT_LABELS) {
    return configured;
  }

  // 6. Hardcoded fallback.
  return DEFAULT_DIALECT;
}

/**
 * Update the dialect the UI reports for the focused editor: the status-bar
 * label and the ``tclLsp.isIruleDialect`` context key that gates the iRules
 * commands.  Purely client-side — it tells the server nothing.
 */
function setActiveDialectLabel(dialect: string): void {
  if (activeDialect === dialect) {
    return;
  }
  activeDialect = dialect;
  updateDialectStatusBar();
  void vscode.commands.executeCommand(
    "setContext",
    "tclLsp.isIruleDialect",
    dialect === "f5-irules",
  );
}

/**
 * Change the **session** dialect: update the local label and push
 * ``tclLsp.dialect`` to the server as a configuration change.
 *
 * Session-global, and a real *configuration* change, so this is only for the
 * deliberate whole-session switch — the ``tclLsp.selectDialect`` command, which
 * writes the setting itself and then pushes it here.  It must NOT be used to
 * tell the server about the focused file's dialect: the server detects that per
 * document (``detect_dialect``), and pushing it globally makes one tab's
 * dialect leak into every other open buffer.
 *
 * For a *temporary* dialect that must be put back afterwards, use
 * {@link setSessionDialectOverride} instead — a configuration push is the wrong
 * tool for that and the next ``workspace/configuration`` pull silently undoes
 * it (issue #1217).
 */
export async function setServerDialect(dialect: string): Promise<void> {
  if (activeDialect === dialect) {
    return;
  }
  setActiveDialectLabel(dialect);

  if (!client) {
    return;
  }

  await client.sendNotification("workspace/didChangeConfiguration", {
    settings: { tclLsp: { dialect } },
  });
}

/**
 * Install (``dialect``) or clear (``null``) the server's **session dialect
 * override** — a deliberate, temporary dialect that outranks the configured
 * one and lasts until it is cleared.
 *
 * This is what a flow that runs a buffer under a fixed dialect and puts it back
 * afterwards needs.  Pushing ``tclLsp.dialect`` as a configuration change
 * (:{@link setServerDialect}) does not survive: the server re-pulls
 * ``workspace/configuration`` on all sorts of unrelated events, and the pulled
 * value overwrites the push, so the override's lifetime was "until anything
 * touches settings" (issue #1217).  The override lives in its own server-side
 * slot that no pull touches.
 *
 * Clearing restores whatever the configuration currently resolves to, so there
 * is nothing to save and restore by hand.
 */
export async function setSessionDialectOverride(dialect: string | null): Promise<void> {
  if (!client) {
    return;
  }
  await client.sendRequest("workspace/executeCommand", {
    command: "tcl-lsp.setSessionDialectOverride",
    arguments: dialect === null ? [] : [dialect],
  });
  // Keep the status bar honest about what the server is actually using. On a
  // clear, fall back to the focused editor's own detected dialect — the same
  // answer `applyDialectForDocument` gives — rather than leaving the override's
  // label behind.
  const editor = vscode.window.activeTextEditor;
  setActiveDialectLabel(
    dialect ??
      (editor && isTclLanguage(editor.document.languageId)
        ? detectDialectFromDocument(editor.document)
        : workspace.getConfiguration("tclLsp").get<string>("dialect", DEFAULT_DIALECT)),
  );
}

/**
 * Reflect *document*'s detected dialect in the status bar.
 *
 * Status bar only.  Per-document dialect resolution belongs to the server: it
 * runs the same directive / shebang / ``package require`` / content-signature /
 * extension detection at ``didOpen`` and on every edit, scoped to that one
 * document.  This used to push the focused file's dialect as a session-global
 * ``didChangeConfiguration``, which re-tagged every other open buffer with it.
 */
export function applyDialectForDocument(document: TextDocument): void {
  if (!isTclLanguage(document.languageId)) {
    return;
  }
  setActiveDialectLabel(detectDialectFromDocument(document));
}

function applyDialectForEditor(editor: TextEditor | undefined): void {
  if (!editor || !isTclLanguage(editor.document.languageId)) {
    return;
  }
  applyDialectForDocument(editor.document);
}

async function renamePartition(uriString?: string, oldPartition?: string): Promise<void> {
  if (!client) {
    window.showWarningMessage("The Tcl language server is not running.");
    return;
  }
  const editor = window.activeTextEditor;
  const targetUri = uriString ?? editor?.document.uri.toString();
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
      if (!trimmed) {
        return "Enter a partition name.";
      }
      if (trimmed === "Common") {
        return "Renaming to or from /Common is not supported.";
      }
      if (!/^[A-Za-z0-9_.-]+$/.test(trimmed)) {
        return "Use letters, digits, underscore, dot, or hyphen.";
      }
      return undefined;
    },
  });
  if (newNameInput === undefined) {
    return;
  }

  const newName = newNameInput.trim().replace(/^\/+/, "");
  const result = (await client.sendRequest("workspace/executeCommand", {
    command: "tcl-lsp.renamePartition",
    arguments: [targetUri, currentName, newName],
  })) as RenamePartitionResult | null;

  if (!result?.success || !result.edit) {
    window.showErrorMessage(result?.error ?? "Partition rename did not produce any edits.");
    return;
  }

  const applied = await workspace.applyEdit(workspaceEditFromLsp(result.edit));
  if (!applied) {
    window.showErrorMessage("VS Code rejected the partition rename workspace edit.");
    return;
  }
  window.showInformationMessage(`Renamed partition ${currentName} to ${newName}.`);
}

async function optimiseDocument(): Promise<void> {
  const editor = window.activeTextEditor;
  if (!editor || !isTclLanguage(editor.document.languageId)) {
    window.showWarningMessage("Open a Tcl file to run optimisations.");
    return;
  }

  const uri = editor.document.uri.toString();
  const result = (await client.sendRequest("workspace/executeCommand", {
    command: "tcl-lsp.optimiseDocument",
    arguments: [uri],
  })) as { optimisations: Array<Record<string, unknown>>; source: string } | null;

  if (!result || !result.optimisations || result.optimisations.length === 0) {
    window.showInformationMessage("No optimisations found.");
    return;
  }

  const count = result.optimisations.length;
  const proceed = await window.showInformationMessage(
    `Apply ${count} optimisation${count === 1 ? "" : "s"}?`,
    "Apply",
    "Cancel",
  );

  if (proceed === "Apply") {
    const fullRange = editor.document.validateRange(new Range(0, 0, Infinity, Infinity));
    await editor.edit((editBuilder) => {
      editBuilder.replace(fullRange, result.source);
    });
    window.showInformationMessage(`Applied ${count} optimisation${count === 1 ? "" : "s"}.`);
  }
}

async function minimizeDiagnostic(uri?: string, code?: string): Promise<void> {
  // Invoked from the "Create minimal repro for <code>" code action with
  // (uri, code) arguments.  Round-trips to the server's minimizer and opens
  // the minimal reproducer in a new buffer for pasting into a bug report.
  if (!client) {
    return;
  }
  const editor = window.activeTextEditor;
  const docUri = uri ?? editor?.document.uri.toString();
  if (!docUri || !code) {
    window.showWarningMessage("Create minimal repro: missing document or diagnostic code.");
    return;
  }

  const result = (await client.sendRequest("workspace/executeCommand", {
    command: "tcl-lsp.minimizeDiagnostic",
    arguments: [docUri, code],
  })) as {
    code: string;
    source: string;
    originalLines: number;
    reducedLines: number;
    renamed: boolean;
    reproduces: boolean;
  } | null;

  if (!result || !result.source) {
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

async function minifyDocument(): Promise<void> {
  const editor = window.activeTextEditor;
  if (!editor || !isTclLanguage(editor.document.languageId)) {
    window.showWarningMessage("Open a Tcl file to minify.");
    return;
  }

  const mode = await window.showQuickPick(
    [
      {
        label: "Minify",
        description: "Strip comments and collapse whitespace",
        compact: false,
        aggressive: false,
        isolated: false,
      },
      {
        label: "Minify + Compact Names",
        description: "Also shorten local variable names (proc names stay — public identities)",
        compact: true,
        aggressive: false,
        isolated: false,
      },
      {
        label: "Minify + Compact Names (Isolated)",
        description: "Self-contained script: also shorten proc names and global variables",
        compact: true,
        aggressive: false,
        isolated: true,
      },
      {
        label: "Aggressive",
        description: "Optimise, compact, and alias for maximum compression (adds helper variables)",
        compact: false,
        aggressive: true,
        isolated: false,
      },
      {
        label: "Aggressive + Isolated",
        description: "Maximum compression — also compact proc names and global-scope variables",
        compact: false,
        aggressive: true,
        isolated: true,
      },
    ],
    { placeHolder: "Select minification mode" },
  );
  if (!mode) {
    return;
  }

  const uri = editor.document.uri.toString();
  const result = (await client.sendRequest("workspace/executeCommand", {
    command: "tcl-lsp.minifyDocument",
    arguments: [uri, mode.compact, mode.aggressive, mode.isolated],
  })) as {
    source: string;
    originalLength: number;
    minifiedLength: number;
    symbolMap?: string;
    optimisationsApplied?: number;
  } | null;

  if (!result || !result.source) {
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

  if (proceed === "Apply") {
    const fullRange = editor.document.validateRange(new Range(0, 0, Infinity, Infinity));
    await editor.edit((editBuilder) => {
      editBuilder.replace(fullRange, result.source);
    });
    const optSuffix =
      result.optimisationsApplied != null && result.optimisationsApplied > 0
        ? ` (${result.optimisationsApplied} optimisations applied)`
        : "";
    window.showInformationMessage(
      `Minified: ${result.originalLength} → ${result.minifiedLength} characters.${optSuffix}`,
    );

    if (result.symbolMap) {
      const doc = await workspace.openTextDocument({ content: result.symbolMap, language: "text" });
      await window.showTextDocument(doc, {
        viewColumn: vscode.ViewColumn.Beside,
        preview: true,
      });
    }
  }
}

async function unminifyError(): Promise<void> {
  // Prompt for the error message
  const errorMessage = await window.showInputBox({
    prompt: "Paste the Tcl or iRule error message to translate",
    placeHolder: 'e.g. can\'t read "a": no such variable',
    ignoreFocusOut: true,
  });
  if (!errorMessage) {
    return;
  }

  // Ask user to pick a symbol map file
  const mapFiles = await window.showOpenDialog({
    canSelectMany: false,
    openLabel: "Select Symbol Map",
    filters: { "Text files": ["txt", "map", "sym"], "All files": ["*"] },
  });
  if (!mapFiles || mapFiles.length === 0) {
    return;
  }
  const mapContent = Buffer.from(await vscode.workspace.fs.readFile(mapFiles[0])).toString("utf-8");

  // Optionally load minified and original sources for line remapping
  let minifiedSource = "";
  let originalSource = "";
  const wantLineRemap = await window.showQuickPick(
    [
      { label: "No", description: "Just translate symbol names", remap: false },
      {
        label: "Yes",
        description: "Also remap line numbers (needs minified + original files)",
        remap: true,
      },
    ],
    { placeHolder: "Remap line numbers too?" },
  );
  if (wantLineRemap?.remap) {
    const minFiles = await window.showOpenDialog({
      canSelectMany: false,
      openLabel: "Select Minified Source",
      filters: { "Tcl files": ["tcl", "irul", "irule", "tm"], "All files": ["*"] },
    });
    if (minFiles && minFiles.length > 0) {
      minifiedSource = Buffer.from(await vscode.workspace.fs.readFile(minFiles[0])).toString(
        "utf-8",
      );
    }
    const origFiles = await window.showOpenDialog({
      canSelectMany: false,
      openLabel: "Select Original Source",
      filters: { "Tcl files": ["tcl", "irul", "irule", "tm"], "All files": ["*"] },
    });
    if (origFiles && origFiles.length > 0) {
      originalSource = Buffer.from(await vscode.workspace.fs.readFile(origFiles[0])).toString(
        "utf-8",
      );
    }
  }

  const result = (await client.sendRequest("workspace/executeCommand", {
    command: "tcl-lsp.unminifyError",
    arguments: [errorMessage, mapContent, minifiedSource, originalSource],
  })) as { translatedError: string; changed: boolean } | null;

  if (!result) {
    window.showWarningMessage("Could not translate error message.");
    return;
  }

  if (!result.changed) {
    window.showInformationMessage("No minified symbols found in the error message.");
    return;
  }

  // Show the translated error in an output document
  const doc = await workspace.openTextDocument({
    content: `Original error:\n${errorMessage}\n\nTranslated error:\n${result.translatedError}`,
    language: "text",
  });
  await window.showTextDocument(doc, {
    viewColumn: vscode.ViewColumn.Beside,
    preview: true,
  });
}

async function translateXc(): Promise<void> {
  const editor = window.activeTextEditor;
  if (!editor || !isTclLanguage(editor.document.languageId)) {
    window.showWarningMessage("Open an iRule file to translate to F5 XC.");
    return;
  }

  const source = editor.document.getText();
  if (!source.trim()) {
    window.showWarningMessage("The current file is empty.");
    return;
  }

  try {
    const result = (await client.sendRequest("workspace/executeCommand", {
      command: "tcl-lsp.xcTranslate",
      arguments: [source, "both"],
    })) as Record<string, unknown> | null;

    if (!result || result.error) {
      window.showErrorMessage(
        `XC translation failed: ${(result?.error as string) ?? "unknown error"}`,
      );
      return;
    }

    const coverage = (result.coverage_pct as number) ?? 0;
    const translatable = (result.translatable_count as number) ?? 0;
    const untranslatable = (result.untranslatable_count as number) ?? 0;
    const terraform = (result.terraform as string) ?? "";
    const jsonApi = result.json_api;

    // Open Terraform HCL in a scratch tab
    if (terraform) {
      const hclDoc = await vscode.workspace.openTextDocument({
        content: terraform,
        language: "terraform",
      });
      await vscode.window.showTextDocument(hclDoc, {
        preview: false,
        viewColumn: vscode.ViewColumn.Beside,
        preserveFocus: true,
      });
    }

    // Open JSON API in a scratch tab
    if (jsonApi) {
      const jsonStr = JSON.stringify(jsonApi, null, 2);
      const jsonDoc = await vscode.workspace.openTextDocument({
        content: jsonStr,
        language: "json",
      });
      await vscode.window.showTextDocument(jsonDoc, {
        preview: false,
        viewColumn: vscode.ViewColumn.Beside,
        preserveFocus: true,
      });
    }

    window.showInformationMessage(
      `XC Translation: ${coverage.toFixed(1)}% coverage — ${translatable} translatable, ${untranslatable} untranslatable`,
    );
  } catch (err) {
    window.showErrorMessage(`XC translation failed: ${err}`);
  }
}

async function showOptimisations(): Promise<void> {
  const editor = window.activeTextEditor;
  if (!editor || !isTclLanguage(editor.document.languageId)) {
    window.showWarningMessage("Open a Tcl file to view optimisations.");
    return;
  }

  const uri = editor.document.uri.toString();
  const result = (await client.sendRequest("workspace/executeCommand", {
    command: "tcl-lsp.optimiseDocument",
    arguments: [uri],
  })) as { optimisations: Array<Record<string, unknown>> } | null;

  if (!result || !result.optimisations || result.optimisations.length === 0) {
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
  const editor = window.activeTextEditor;
  if (!editor || !isTclLanguage(editor.document.languageId)) {
    window.showWarningMessage("Open a Tcl file to apply safe fixes.");
    return;
  }

  const uri = editor.document.uri.toString();
  const result = (await client.sendRequest("workspace/executeCommand", {
    command: "tcl-lsp.fixAllSafeIssues",
    arguments: [uri],
  })) as {
    // `safety` is the fix's classification (issue #1195). Only
    // `semantics-equivalent` fixes reach this list — the server applies
    // nothing else in bulk — so it is reported for traceability rather than
    // filtered on here.
    source: string;
    applied: Array<{ code: string; description: string; safety: string }>;
  } | null;

  if (!result || !result.applied || result.applied.length === 0) {
    window.showInformationMessage("No safe auto-fixes available.");
    return;
  }

  const proceed = await window.showInformationMessage(
    `Apply ${result.applied.length} safe fix${result.applied.length === 1 ? "" : "es"}?`,
    "Apply",
    "Cancel",
  );
  if (proceed !== "Apply") {
    return;
  }

  const fullRange = editor.document.validateRange(new Range(0, 0, Infinity, Infinity));
  await editor.edit((editBuilder) => {
    editBuilder.replace(fullRange, result.source);
  });
  window.showInformationMessage(
    `Applied ${result.applied.length} safe fix${result.applied.length === 1 ? "" : "es"}.`,
  );
}

function escapeRegExp(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function packageInsertLine(document: TextDocument): number {
  let line = 0;
  if (document.lineCount > 0 && document.lineAt(0).text.startsWith("#!")) {
    line = 1;
  }
  while (line < document.lineCount) {
    const text = document.lineAt(line).text;
    if (/^\s*package\s+require\b/.test(text)) {
      line += 1;
      continue;
    }
    break;
  }
  return line;
}

async function insertPackageRequire(): Promise<void> {
  const editor = window.activeTextEditor;
  if (!editor || !isTclLanguage(editor.document.languageId)) {
    window.showWarningMessage("Open a Tcl file to insert a package requirement.");
    return;
  }

  const wordRange = editor.document.getWordRangeAtPosition(editor.selection.active, /[\w:]+/);
  const symbol = wordRange ? editor.document.getText(wordRange) : "";

  let suggestions: string[] = [];
  const suggestResult = (await client.sendRequest("workspace/executeCommand", {
    command: "tcl-lsp.suggestPackagesForSymbol",
    arguments: [symbol],
  })) as { suggestions?: string[] } | null;
  suggestions = suggestResult?.suggestions ?? [];

  if (suggestions.length === 0) {
    const listResult = (await client.sendRequest("workspace/executeCommand", {
      command: "tcl-lsp.listKnownPackages",
      arguments: [],
    })) as { packages?: string[] } | null;
    suggestions = listResult?.packages ?? [];
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
  if (!picked) {
    return;
  }

  const existing = new RegExp(`^\\s*package\\s+require\\s+${escapeRegExp(picked)}(?:\\s|$)`, "m");
  if (existing.test(editor.document.getText())) {
    window.showInformationMessage(`'package require ${picked}' already exists.`);
    return;
  }

  const insertAt = packageInsertLine(editor.document);
  const prefix =
    insertAt > 0 && editor.document.lineAt(insertAt - 1).text.trim() !== "" ? "\n" : "";
  const text = `${prefix}package require ${picked}\n`;
  const insertPos =
    insertAt < editor.document.lineCount
      ? new vscode.Position(insertAt, 0)
      : editor.document.lineAt(editor.document.lineCount - 1).range.end;
  const applied = await editor.edit((editBuilder) => {
    editBuilder.insert(insertPos, text);
  });
  if (applied) {
    window.showInformationMessage(`Inserted 'package require ${picked}'.`);
  } else {
    window.showWarningMessage("Failed to insert package require.");
  }
}

function defaultPackageName(): string {
  const activeFolder = window.activeTextEditor
    ? workspace.getWorkspaceFolder(window.activeTextEditor.document.uri)
    : undefined;
  const folder = activeFolder || workspace.workspaceFolders?.[0];
  if (!folder) {
    return "my_package";
  }
  const base = path.basename(folder.uri.fsPath);
  const candidate = packageDirectoryName(base).replace(/^-+/, "");
  return candidate || "my_package";
}

async function scaffoldPackageStarter(): Promise<void> {
  const activeFolder = window.activeTextEditor
    ? workspace.getWorkspaceFolder(window.activeTextEditor.document.uri)
    : undefined;
  const folder = activeFolder || workspace.workspaceFolders?.[0];
  if (!folder) {
    window.showWarningMessage("Open a workspace folder before scaffolding a package.");
    return;
  }

  const packageName = await window.showInputBox({
    title: "Scaffold Tcl Package",
    prompt: "Package name",
    value: defaultPackageName(),
    ignoreFocusOut: true,
    validateInput: (value) => validatePackageName(value) || undefined,
  });
  if (!packageName) {
    return;
  }

  const version = await window.showInputBox({
    title: "Scaffold Tcl Package",
    prompt: "Initial package version",
    value: "0.1.0",
    ignoreFocusOut: true,
    validateInput: (value) => validateVersion(value) || undefined,
  });
  if (!version) {
    return;
  }

  const commandName = await window.showInputBox({
    title: "Scaffold Tcl Package",
    prompt: "Initial exported command name",
    value: "hello",
    ignoreFocusOut: true,
    validateInput: (value) => validateCommandName(value) || undefined,
  });
  if (!commandName) {
    return;
  }

  const scaffold = buildPackageScaffold({
    packageName,
    version,
    commandName,
    minimumTclVersion: "8.6",
  });
  const targetDir = path.join(folder.uri.fsPath, scaffold.directoryName);
  if (existsSync(targetDir)) {
    window.showWarningMessage(
      `Scaffold target '${scaffold.directoryName}' already exists in ${folder.name}.`,
    );
    return;
  }

  for (const file of scaffold.files) {
    const absolutePath = path.join(targetDir, file.relativePath);
    mkdirSync(path.dirname(absolutePath), { recursive: true });
    writeFileSync(absolutePath, file.content, "utf8");
  }

  const sourcePath = path.join(targetDir, scaffold.mainFile);
  const readmePath = path.join(targetDir, scaffold.readmeFile);
  const action = await window.showInformationMessage(
    `Created Tcl package scaffold in '${scaffold.directoryName}'.`,
    "Open Source",
    "Open README",
  );
  const openPath = action === "Open README" ? readmePath : sourcePath;
  const doc = await workspace.openTextDocument(openPath);
  await window.showTextDocument(doc);
}

let lastIruleSkeletonSelection: string[] = [];

function inferIruleEventsFromActiveEditor(): string[] {
  const editor = window.activeTextEditor;
  if (!editor) {
    return [];
  }
  const source = editor.document.getText();
  const selected: string[] = [];
  for (const eventInfo of COMMON_IRULE_EVENTS) {
    const pattern = new RegExp(`\\bwhen\\s+${escapeRegExp(eventInfo.name)}\\b`);
    if (pattern.test(source)) {
      selected.push(eventInfo.name);
    }
  }
  return selected;
}

async function insertIruleEventSkeleton(): Promise<void> {
  const items = COMMON_IRULE_EVENTS.map((eventInfo) => ({
    label: eventInfo.name,
    description: eventInfo.description,
  }));
  const initialSelection =
    lastIruleSkeletonSelection.length > 0
      ? lastIruleSkeletonSelection
      : inferIruleEventsFromActiveEditor();

  const picked = await new Promise<typeof items | undefined>((resolve) => {
    const quickPick = window.createQuickPick<(typeof items)[number]>();
    quickPick.title = "Insert iRule Event Skeleton";
    quickPick.placeholder = "Select one or more events to scaffold";
    quickPick.canSelectMany = true;
    quickPick.ignoreFocusOut = true;
    quickPick.items = items;

    const selected = items.filter((item) => initialSelection.includes(item.label));
    if (selected.length > 0) {
      quickPick.selectedItems = selected;
      quickPick.activeItems = [selected[0]];
    }

    let done = false;
    const finish = (value: typeof items | undefined) => {
      if (done) {
        return;
      }
      done = true;
      disposeAccept.dispose();
      disposeHide.dispose();
      quickPick.dispose();
      resolve(value);
    };

    const disposeAccept = quickPick.onDidAccept(() => {
      finish([...quickPick.selectedItems]);
    });
    const disposeHide = quickPick.onDidHide(() => finish(undefined));
    quickPick.show();
  });

  if (!picked || picked.length === 0) {
    return;
  }

  lastIruleSkeletonSelection = picked.map((entry) => entry.label);

  const skeleton = buildIruleEventSkeleton(picked.map((entry) => entry.label));
  if (!skeleton) {
    window.showWarningMessage("Unable to build iRule skeleton from the selected events.");
    return;
  }

  const doc = await workspace.openTextDocument({
    language: "tcl-irule",
    content: skeleton,
  });
  await window.showTextDocument(doc);
}

async function resolveTemplateEditor(): Promise<TextEditor> {
  const editor = window.activeTextEditor;
  if (editor && isTclLanguage(editor.document.languageId)) {
    return editor;
  }
  const doc = await workspace.openTextDocument({ language: "tcl", content: "" });
  return window.showTextDocument(doc);
}

async function insertTemplateSnippet(extensionPath: string): Promise<void> {
  let snippets: TemplateSnippet[];
  try {
    snippets = loadTemplateSnippets(extensionPath);
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

  if (!picked) {
    return;
  }

  const editor = await resolveTemplateEditor();
  const snippetText = renderTemplateSnippet(picked.snippet);
  await editor.insertSnippet(new vscode.SnippetString(snippetText), editor.selection);
}

async function runRuntimeValidation(): Promise<void> {
  const editor = window.activeTextEditor;
  if (!editor || !isTclLanguage(editor.document.languageId)) {
    window.showWarningMessage("Open a Tcl file to run runtime validation.");
    return;
  }
  await runRuntimeValidationForDocument(editor.document, { quiet: false });
}

async function runRuntimeValidationForDocument(
  document: TextDocument,
  opts: { quiet: boolean },
): Promise<void> {
  const runtimeCfg = workspace.getConfiguration("tclLsp.runtimeValidation");
  const tclshPath = runtimeCfg.get<string>("tclshPath", "tclsh");
  const timeoutMs = runtimeCfg.get<number>("timeoutMs", 5000);
  const adapterMode = runtimeCfg.get<RuntimeValidationAdapterMode>("adapter", "auto");
  const dialect = detectDialectFromDocument(document);
  const adapter = resolveRuntimeValidationAdapter(adapterMode, dialect);
  const adapterLabel = runtimeValidationAdapterLabel(adapter);

  // Per-invocation temp dir — replaces the previous shape where two
  // sibling files lived directly in the shared OS temp dir under a
  // ``Date.now() + Math.random()`` suffix.  ``mkdtempSync`` guarantees
  // a fresh, attacker-unwriteable directory and lets us tear the whole
  // thing down with a single ``rmSync`` (CodeQL
  // ``js/insecure-temporary-file``).
  const tmpDir = mkdtempSync(path.join(tmpdir(), "tcl-lsp-runtime-validate-"));
  const checkerPath = path.join(tmpDir, "checker.tcl");
  const checkerScript = buildRuntimeValidationChecker(adapter);
  const targetPath = path.join(tmpDir, "target.tcl");

  try {
    writeFileSync(checkerPath, checkerScript, "utf8");
    writeFileSync(targetPath, document.getText(), "utf8");

    await execFileAsync(tclshPath, [checkerPath, targetPath], {
      timeout: timeoutMs,
      maxBuffer: 256 * 1024,
    });

    if (!opts.quiet) {
      window.showInformationMessage(`Runtime validation passed (${adapterLabel}).`);
    }
  } catch (error) {
    const err = error as { stderr?: string; stdout?: string; message?: string };
    const details = (err.stderr || err.stdout || err.message || "Validation failed").trim();
    if (opts.quiet) {
      window.setStatusBarMessage(
        `Tcl runtime validation failed (${adapterLabel}): ${details}`,
        6000,
      );
    } else {
      window.showWarningMessage(`Runtime validation failed (${adapterLabel}): ${details}`);
    }
  } finally {
    try {
      rmSync(tmpDir, { recursive: true, force: true });
    } catch {
      // ignore cleanup errors
    }
  }
}

async function toggleDiagnostics(): Promise<void> {
  const config = workspace.getConfiguration("tclLsp.features");
  const current = config.get<boolean>("diagnostics", true);
  await config.update("diagnostics", !current, undefined);
  window.showInformationMessage(`Tcl diagnostics ${!current ? "enabled" : "disabled"}.`);
}

async function toggleOptimiser(): Promise<void> {
  const config = workspace.getConfiguration("tclLsp.optimiser");
  const current = config.get<boolean>("enabled", true);
  await config.update("enabled", !current, undefined);
  window.showInformationMessage(`Tcl optimiser suggestions ${!current ? "enabled" : "disabled"}.`);
}

async function toggleAi(): Promise<void> {
  const config = workspace.getConfiguration("tclLsp.ai");
  const current = config.get<boolean>("enabled", true);
  await config.update("enabled", !current, undefined);
  window.showInformationMessage(`Tcl AI features ${!current ? "enabled" : "disabled"}.`);
}

async function formatDocument(): Promise<void> {
  const editor = window.activeTextEditor;
  if (!editor || !isTclLanguage(editor.document.languageId)) {
    window.showWarningMessage("Open a Tcl file to format.");
    return;
  }
  await commands.executeCommand("editor.action.formatDocument");
}

async function escapeSelection(): Promise<void> {
  await transformSelection(escapeTclText, "Escaped", "escape");
}

async function unescapeSelection(): Promise<void> {
  await transformSelection(unescapeTclText, "Unescaped", "unescape");
}

function base64Encode(text: string): string {
  return Buffer.from(text, "utf-8").toString("base64");
}

function base64Decode(text: string): string {
  return Buffer.from(text.trim(), "base64").toString("utf-8");
}

async function base64EncodeSelection(): Promise<void> {
  await transformSelection(base64Encode, "Base64-encoded", "base64-encode");
}

async function base64DecodeSelection(): Promise<void> {
  await transformSelection(base64Decode, "Base64-decoded", "base64-decode");
}

const MAX_FILE_BYTES = 8192;

/**
 * Read up to ``MAX_FILE_BYTES`` from ``fsPath`` via an open fd so the
 * size check and the read both observe the *same* inode.  A raw
 * ``statSync`` → ``readFileSync`` pair is a TOCTOU window (CodeQL
 * ``js/file-system-race``): an attacker who controls the path could
 * swap a small placeholder for a huge file between the two calls and
 * defeat the limit.
 *
 * Returns ``undefined`` when the file exceeds the cap (caller surfaces
 * a warning to the user).
 */
function readCappedSync(fsPath: string): Buffer | undefined {
  const fd = openSync(fsPath, "r");
  try {
    const size = fstatSync(fd).size;
    if (size > MAX_FILE_BYTES) {
      window.showWarningMessage(`File is ${size} bytes — exceeds ${MAX_FILE_BYTES} byte limit.`);
      return undefined;
    }
    const buf = Buffer.allocUnsafe(size);
    let read = 0;
    while (read < size) {
      const n = readSync(fd, buf, read, size - read, read);
      if (n === 0) break;
      read += n;
    }
    return buf.subarray(0, read);
  } finally {
    try {
      closeSync(fd);
    } catch {
      // ignore close errors
    }
  }
}

async function copyFileAsBase64(uri: vscode.Uri): Promise<void> {
  const raw = readCappedSync(uri.fsPath);
  if (raw === undefined) return;
  const encoded = raw.toString("base64");
  await vscode.env.clipboard.writeText(encoded);
  window.showInformationMessage(`Copied ${raw.length} bytes as base64 (${encoded.length} chars).`);
}

async function copyFileAsGzipBase64(uri: vscode.Uri): Promise<void> {
  const raw = readCappedSync(uri.fsPath);
  if (raw === undefined) return;
  const compressed = gzipSync(raw, { level: 9 });
  const encoded = compressed.toString("base64");
  await vscode.env.clipboard.writeText(encoded);
  window.showInformationMessage(
    `Copied ${raw.length} → ${compressed.length} bytes gzipped as base64 (${encoded.length} chars).`,
  );
}

// BIG-IP rule extraction

interface RuleInfo {
  name: string;
  fullPath: string;
  body: string;
  bodyStartOffset: number;
  bodyEndOffset: number;
  uri: string;
  blockStartLine?: number;
}

interface BigipLinkedObjectResult {
  root: string;
  rootUri: string;
  rootHeader: string;
  roots: Array<{
    id: string;
    uri: string;
    header: string;
  }>;
  maxDepth: number;
  maxNodes: number;
  nodes: Array<{
    id: string;
    uri: string;
    module: string;
    objectType: string;
    identifier: string;
    kind: string | null;
    header: string;
    depth: number;
    sourceOrigin: "base" | "synced" | "script" | null;
    range: {
      start: { line: number; character: number };
      end: { line: number; character: number };
    };
  }>;
  edges: Array<{
    source: string;
    target: string;
    viaProperty: string;
    viaKind: string;
  }>;
}

/**
 * Track scratch editors opened from config rules so we can write changes
 * back when the scratch document is saved.
 */
const scratchRuleMap = new Map<
  string,
  { sourceUri: string; bodyStartOffset: number; bodyEndOffset: number; originalBody: string }
>();

async function openRuleInScratchEditor(rule: RuleInfo): Promise<void> {
  const body = rule.body;
  const doc = await vscode.workspace.openTextDocument({
    language: "tcl-irule",
    content: body,
  });
  await vscode.window.showTextDocument(doc, { preview: false });

  // Track this scratch document for write-back on save.
  scratchRuleMap.set(doc.uri.toString(), {
    sourceUri: rule.uri,
    bodyStartOffset: rule.bodyStartOffset,
    bodyEndOffset: rule.bodyEndOffset,
    originalBody: body,
  });

  window.showInformationMessage(`Editing iRule '${rule.name}' — save to write back to config.`);
}

/**
 * Extract the iRule at the current cursor position and open it in a
 * scratch editor with the f5-irules dialect.
 */
async function extractRuleAtCursor(): Promise<void> {
  const editor = window.activeTextEditor;
  if (!editor || !client) {
    window.showWarningMessage("Open a BIG-IP configuration file first.");
    return;
  }

  const uri = editor.document.uri.toString();
  const offset = editor.document.offsetAt(editor.selection.active);

  const result: RuleInfo | null = await client.sendRequest("workspace/executeCommand", {
    command: "tcl-lsp.extractRule",
    arguments: [uri, offset],
  });

  if (!result) {
    window.showWarningMessage("Cursor is not inside an ltm rule or gtm rule block.");
    return;
  }

  await openRuleInScratchEditor(result);
}

/**
 * Show a quick-pick list of all iRules in the current config file and
 * open the selected one in a scratch editor.
 */
async function extractRulePick(): Promise<void> {
  const editor = window.activeTextEditor;
  if (!editor || !client) {
    window.showWarningMessage("Open a BIG-IP configuration file first.");
    return;
  }

  const uri = editor.document.uri.toString();
  const rules: RuleInfo[] | null = await client.sendRequest("workspace/executeCommand", {
    command: "tcl-lsp.listRules",
    arguments: [uri],
  });

  if (!rules || rules.length === 0) {
    window.showInformationMessage("No ltm rule or gtm rule blocks found in this file.");
    return;
  }

  const items = rules.map((r) => ({
    label: r.name,
    description: r.fullPath,
    rule: r,
  }));

  const picked = await window.showQuickPick(items, {
    placeHolder: "Select an iRule to edit",
  });

  if (picked) {
    await openRuleInScratchEditor(picked.rule);
  }
}

/**
 * Extract all iRules from the current BIG-IP config file and save each
 * to an individual `.irul` file in a user-chosen folder.  The file name
 * is derived from the rule's full object path with `/` replaced by `_`.
 */
async function extractAllRules(): Promise<void> {
  const editor = window.activeTextEditor;
  if (!editor || !client) {
    window.showWarningMessage("Open a BIG-IP configuration file first.");
    return;
  }

  const uri = editor.document.uri.toString();
  const rules: RuleInfo[] | null = await client.sendRequest("workspace/executeCommand", {
    command: "tcl-lsp.listRules",
    arguments: [uri],
  });

  if (!rules || rules.length === 0) {
    window.showInformationMessage("No ltm rule or gtm rule blocks found in this file.");
    return;
  }

  // Ask the user where to save the extracted files.
  const folders = await window.showOpenDialog({
    canSelectFiles: false,
    canSelectFolders: true,
    canSelectMany: false,
    openLabel: "Select output folder",
  });

  if (!folders || folders.length === 0) {
    return;
  }

  const targetDir = folders[0];
  let written = 0;

  for (const rule of rules) {
    // Replace `/` with `_` in the full path, stripping a leading `_`.
    const safeName = rule.fullPath.replace(/\//g, "_").replace(/^_/, "");
    const fileUri = vscode.Uri.joinPath(targetDir, `${safeName}.irul`);
    await vscode.workspace.fs.writeFile(fileUri, Buffer.from(rule.body, "utf-8"));
    written++;
  }

  window.showInformationMessage(`Extracted ${written} iRule(s) to ${targetDir.fsPath}`);
}

/**
 * Extract transitive linked BIG-IP objects from the object under cursor.
 */
async function extractLinkedObjectsAtCursor(): Promise<void> {
  const editor = window.activeTextEditor;
  if (!editor || !client) {
    window.showWarningMessage("Open a BIG-IP configuration file first.");
    return;
  }

  // Collect all cursor/selection positions across the editor.
  const selections = editor.selections;
  const primaryUri = editor.document.uri.toString();
  const primaryOffset = editor.document.offsetAt(selections[0].active);

  // Additional cursors beyond the first are sent as extra_offsets.
  const extraOffsets: Array<[string, number]> = [];
  for (let i = 1; i < selections.length; i++) {
    extraOffsets.push([primaryUri, editor.document.offsetAt(selections[i].active)]);
  }

  const result: BigipLinkedObjectResult | null = await client.sendRequest(
    "workspace/executeCommand",
    {
      command: "tcl-lsp.extractLinkedObjects",
      arguments: [primaryUri, primaryOffset, 5, 400, extraOffsets.length > 0 ? extraOffsets : null],
    },
  );

  if (!result) {
    window.showWarningMessage("No BIG-IP object found at cursor.");
    return;
  }

  const doc = await vscode.workspace.openTextDocument({
    language: "json",
    content: JSON.stringify(result, null, 2),
  });
  await vscode.window.showTextDocument(doc, { preview: false });
}

/**
 * Generate a tmsh script that deletes BIG-IP objects unreferenced by any
 * virtual server.  Walks the active config, opens the resulting script
 * (and a JSON metadata report) side-by-side for review.
 */
interface BigipCleanupResult {
  candidates: Array<{
    fullPath: string;
    kind: string | null;
    deleteCommand: string;
    [key: string]: unknown;
  }>;
  summary: Record<string, number>;
  tmshScript: string;
  [key: string]: unknown;
}

async function generateBigipCleanupScript(): Promise<void> {
  const editor = window.activeTextEditor;
  if (!editor || !client) {
    window.showWarningMessage("Open a BIG-IP configuration file first.");
    return;
  }

  const uri = editor.document.uri.toString();
  const result: BigipCleanupResult | null = await client.sendRequest("workspace/executeCommand", {
    command: "tcl-lsp.bigipCleanup",
    arguments: [[uri], null, false],
  });

  if (!result) {
    window.showWarningMessage(
      "No BIG-IP configuration loaded in the workspace.  Open a bigip.conf and try again.",
    );
    return;
  }

  const scriptDoc = await vscode.workspace.openTextDocument({
    language: "tcl-bigip",
    content: result.tmshScript,
  });
  await vscode.window.showTextDocument(scriptDoc, {
    preview: false,
    viewColumn: vscode.ViewColumn.One,
  });

  const reportDoc = await vscode.workspace.openTextDocument({
    language: "json",
    content: JSON.stringify(result, null, 2),
  });
  await vscode.window.showTextDocument(reportDoc, {
    preview: false,
    viewColumn: vscode.ViewColumn.Beside,
  });
}

// Listen for saves on scratch documents and write changes back to the
// original configuration file.
vscode.workspace.onDidSaveTextDocument(async (doc) => {
  const entry = scratchRuleMap.get(doc.uri.toString());
  if (!entry || !client) {
    return;
  }

  const newBody = doc.getText();
  if (newBody === entry.originalBody) {
    return; // No changes
  }

  const ok: boolean = await client.sendRequest("workspace/executeCommand", {
    command: "tcl-lsp.writeRuleBack",
    arguments: [entry.sourceUri, entry.bodyStartOffset, entry.bodyEndOffset, newBody],
  });

  if (ok) {
    // Update offsets for the next save (body length may have changed).
    const delta = newBody.length - (entry.bodyEndOffset - entry.bodyStartOffset);
    entry.bodyEndOffset += delta;
    entry.originalBody = newBody;
    window.showInformationMessage("iRule written back to configuration file.");
  } else {
    window.showWarningMessage("Failed to write iRule back to configuration file.");
  }
});

// Clean up tracking when scratch documents are closed.
vscode.workspace.onDidCloseTextDocument((doc) => {
  scratchRuleMap.delete(doc.uri.toString());
});
