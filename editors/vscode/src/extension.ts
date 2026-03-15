import {
  existsSync,
  mkdirSync,
  readFileSync,
  realpathSync,
  statSync,
  unlinkSync,
  writeFileSync,
} from "fs";
import * as path from "path";
import { execFile } from "child_process";
import { gzipSync } from "zlib";
import { homedir, platform, tmpdir } from "os";
import { promisify } from "util";
import * as vscode from "vscode";
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
import { LanguageClient, LanguageClientOptions, ServerOptions } from "vscode-languageclient/node";
import { registerIruleParticipant } from "./chat/iruleParticipant";
import { registerTclParticipant } from "./chat/tclParticipant";
import { registerTkParticipant } from "./chat/tkParticipant";
import {
  buildRuntimeValidationChecker,
  resolveRuntimeValidationAdapter,
  runtimeValidationAdapterLabel,
  RuntimeValidationAdapterMode,
} from "./runtimeValidation";
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

const DIALECT_LABELS: Record<string, string> = {
  "tcl8.4": "Tcl 8.4",
  "tcl8.5": "Tcl 8.5",
  "tcl8.6": "Tcl 8.6",
  "tcl9.0": "Tcl 9.0",
  "f5-irules": "F5 iRules",
  "f5-iapps": "F5 iApps",
  "f5-bigip": "F5 BIG-IP Config",
  "eda-tools": "EDA Tools",
  expect: "Expect",
};

const DEFAULT_DIALECT = "tcl8.6";

const TCL_LANGUAGE_IDS = new Set([
  "tcl",
  "tcl-irule",
  "tcl-iapp",
  "tcl-bigip",
  "tcl8.4",
  "tcl8.5",
  "tcl9.0",
  "tcl-eda",
  "tcl-expect",
]);

export function isTclLanguage(languageId: string): boolean {
  return TCL_LANGUAGE_IDS.has(languageId);
}

/** Map language IDs that imply a specific dialect. */
const LANGUAGE_ID_DIALECTS: Record<string, string> = {
  "tcl-irule": "f5-irules",
  "tcl-iapp": "f5-iapps",
  "tcl-bigip": "f5-bigip",
  "tcl8.4": "tcl8.4",
  "tcl8.5": "tcl8.5",
  "tcl9.0": "tcl9.0",
  "tcl-eda": "eda-tools",
  "tcl-expect": "expect",
};

const TCL_VERSION_DIALECTS: Record<string, string> = {
  "8.4": "tcl8.4",
  "8.5": "tcl8.5",
  "8.6": "tcl8.6",
  "9.0": "tcl9.0",
};

// Python discovery

const MIN_PYTHON_MAJOR = 3;
const MIN_PYTHON_MINOR = 10;
const MAX_PYTHON_MINOR_SCAN = 15;

interface PythonInfo {
  path: string;
  version: string;
  major: number;
  minor: number;
  patch: number;
  source: string;
}

let outputChannel: vscode.OutputChannel;

function getOutputChannel(): vscode.OutputChannel {
  if (!outputChannel) {
    outputChannel = window.createOutputChannel("Tcl Language Server");
  }
  return outputChannel;
}

/** Run `<command> --version` and parse the result. */
async function probePython(
  command: string,
  args: string[] = [],
): Promise<Omit<PythonInfo, "source"> | undefined> {
  try {
    const versionArgs = [...args, "--version"];
    const { stdout, stderr } = await execFileAsync(command, versionArgs, { timeout: 3000 });
    const output = (stdout || stderr).trim();
    const match = output.match(/Python\s+(\d+)\.(\d+)\.(\d+)/);
    if (!match) return undefined;
    const major = parseInt(match[1], 10);
    const minor = parseInt(match[2], 10);
    const patch = parseInt(match[3], 10);
    if (major < MIN_PYTHON_MAJOR) return undefined;
    if (major === MIN_PYTHON_MAJOR && minor < MIN_PYTHON_MINOR) return undefined;
    // Resolve the real path to deduplicate symlinks.
    let resolvedPath: string;
    if (args.length === 0) {
      try {
        // For a bare command, find its absolute path first.
        const whichCmd = platform() === "win32" ? "where" : "which";
        const { stdout: whichOut } = await execFileAsync(whichCmd, [command], { timeout: 2000 });
        const absPath = whichOut.trim().split(/\r?\n/)[0];
        resolvedPath = absPath ? realpathSync(absPath) : command;
      } catch {
        resolvedPath = command;
      }
    } else {
      // For commands like "py -3.12", the command itself is the path.
      resolvedPath = command;
    }
    return { path: resolvedPath, version: `${major}.${minor}.${patch}`, major, minor, patch };
  } catch {
    return undefined;
  }
}

/** Collect candidate paths from well-known locations per platform. */
function wellKnownPythonPaths(): { command: string; args: string[]; source: string }[] {
  const candidates: { command: string; args: string[]; source: string }[] = [];
  const os = platform();

  if (os === "darwin") {
    // Homebrew — Apple Silicon
    for (let m = MAX_PYTHON_MINOR_SCAN; m >= MIN_PYTHON_MINOR; m--) {
      candidates.push({ command: `/opt/homebrew/bin/python3.${m}`, args: [], source: "Homebrew" });
    }
    candidates.push({ command: "/opt/homebrew/bin/python3", args: [], source: "Homebrew" });
    // Homebrew — Intel
    for (let m = MAX_PYTHON_MINOR_SCAN; m >= MIN_PYTHON_MINOR; m--) {
      candidates.push({
        command: `/usr/local/bin/python3.${m}`,
        args: [],
        source: "Homebrew/python.org",
      });
    }
    candidates.push({ command: "/usr/local/bin/python3", args: [], source: "Homebrew/python.org" });
    // python.org framework
    for (let m = MAX_PYTHON_MINOR_SCAN; m >= MIN_PYTHON_MINOR; m--) {
      candidates.push({
        command: `/Library/Frameworks/Python.framework/Versions/3.${m}/bin/python3`,
        args: [],
        source: "python.org",
      });
    }
  } else if (os === "win32") {
    // Windows py launcher
    for (let m = MAX_PYTHON_MINOR_SCAN; m >= MIN_PYTHON_MINOR; m--) {
      candidates.push({ command: "py", args: [`-3.${m}`], source: "py launcher" });
    }
    // python.org per-user install
    const localAppData = process.env.LOCALAPPDATA || "";
    if (localAppData) {
      for (let m = MAX_PYTHON_MINOR_SCAN; m >= MIN_PYTHON_MINOR; m--) {
        candidates.push({
          command: path.join(localAppData, "Programs", "Python", `Python3${m}`, "python.exe"),
          args: [],
          source: "python.org",
        });
      }
    }
    // python.org system-wide
    const progFiles = process.env.ProgramFiles || "C:\\Program Files";
    for (let m = MAX_PYTHON_MINOR_SCAN; m >= MIN_PYTHON_MINOR; m--) {
      candidates.push({
        command: path.join(progFiles, `Python3${m}`, "python.exe"),
        args: [],
        source: "python.org",
      });
    }
  } else {
    // Linux
    for (let m = MAX_PYTHON_MINOR_SCAN; m >= MIN_PYTHON_MINOR; m--) {
      candidates.push({ command: `/usr/bin/python3.${m}`, args: [], source: "system" });
    }
    for (let m = MAX_PYTHON_MINOR_SCAN; m >= MIN_PYTHON_MINOR; m--) {
      candidates.push({ command: `/usr/local/bin/python3.${m}`, args: [], source: "local" });
    }
  }

  // Anaconda / Miniconda — all platforms (last resort)
  const home = homedir();
  if (os === "win32") {
    for (const dir of ["Miniconda3", "Anaconda3"]) {
      candidates.push({
        command: path.join(home, dir, "python.exe"),
        args: [],
        source: "Anaconda",
      });
    }
  } else {
    for (const dir of ["miniconda3", "anaconda3"]) {
      candidates.push({
        command: path.join(home, dir, "bin", "python3"),
        args: [],
        source: "Anaconda",
      });
    }
  }

  return candidates;
}

/** Discover all Python 3.10+ interpreters, sorted highest-version-first. */
async function discoverPythons(): Promise<PythonInfo[]> {
  const seen = new Set<string>();
  const results: PythonInfo[] = [];

  async function tryCandidate(command: string, args: string[], source: string): Promise<void> {
    const info = await probePython(command, args);
    if (!info) return;
    const key = info.path;
    if (seen.has(key)) return;
    seen.add(key);
    results.push({ ...info, source });
  }

  // 1. Versioned PATH binaries (highest first)
  const pathPromises: Promise<void>[] = [];
  for (let m = MAX_PYTHON_MINOR_SCAN; m >= MIN_PYTHON_MINOR; m--) {
    pathPromises.push(tryCandidate(`python3.${m}`, [], "PATH"));
  }
  pathPromises.push(tryCandidate("python3", [], "PATH"));
  if (platform() === "win32") {
    pathPromises.push(tryCandidate("python", [], "PATH"));
  }
  await Promise.all(pathPromises);

  // 2. uv (if available)
  try {
    const { stdout } = await execFileAsync("uv", ["python", "find", "--no-project"], {
      timeout: 5000,
    });
    const uvPython = stdout.trim();
    if (uvPython) {
      await tryCandidate(uvPython, [], "uv");
    }
  } catch {
    // uv not installed or failed — skip
  }

  // 3. Well-known locations
  const wellKnown = wellKnownPythonPaths();
  const wellKnownPromises = wellKnown.map((c) => tryCandidate(c.command, c.args, c.source));
  await Promise.all(wellKnownPromises);

  // Sort by version descending
  results.sort((a, b) => {
    if (a.major !== b.major) return b.major - a.major;
    if (a.minor !== b.minor) return b.minor - a.minor;
    return b.patch - a.patch;
  });

  return results;
}

/** Log discovery results and select the appropriate interpreter. */
async function resolvePython(configured: string): Promise<PythonInfo | undefined> {
  const ch = getOutputChannel();

  // User specified an explicit path
  if (configured && configured !== "auto") {
    ch.appendLine(`Python: using configured interpreter: ${configured}`);
    const info = await probePython(configured);
    if (info) {
      const result = { ...info, source: "configured" };
      ch.appendLine(`  ${info.version} — OK`);
      return result;
    }
    ch.appendLine("  ERROR: not found or below minimum version (3.10)");
    return undefined;
  }

  // Auto-discovery
  ch.appendLine("Python discovery:");
  const pythons = await discoverPythons();
  if (pythons.length === 0) {
    ch.appendLine("  (none found)");
    return undefined;
  }
  for (const p of pythons) {
    ch.appendLine(`  ${p.path}  ${p.version}  (${p.source})`);
  }
  const selected = pythons[0];
  ch.appendLine(`Selected: ${selected.path} (${selected.version})`);
  return selected;
}

// Server bundle detection

function hasServerBundle(dir: string): boolean {
  return (
    existsSync(path.join(dir, "lsp", "__main__.py")) && existsSync(path.join(dir, "pyproject.toml"))
  );
}

function resolveServerDir(configuredPath: string, extensionPath: string): string {
  const configured = configuredPath.trim();
  if (configured) {
    return configured;
  }
  // Walk up from the extension directory to find the server bundle.
  // Handles both repo-root layouts (extension at /) and nested layouts
  // (extension at editors/vscode/).
  let dir = extensionPath;
  for (let i = 0; i < 3; i++) {
    if (hasServerBundle(dir)) {
      return dir;
    }
    const parent = path.resolve(dir, "..");
    if (parent === dir) break;
    dir = parent;
  }
  return extensionPath;
}

export async function activate(context: ExtensionContext) {
  const config = workspace.getConfiguration("tclLsp");
  const configuredServerPath = config.get<string>("serverPath", "");

  let serverOptions: ServerOptions;

  // Dev mode: explicit serverPath or running from a git checkout.
  const serverDir = resolveServerDir(configuredServerPath, context.extensionPath);
  if (configuredServerPath || hasServerBundle(serverDir)) {
    if (!hasServerBundle(serverDir)) {
      window.showErrorMessage(
        `Unable to locate Tcl server bundle under '${serverDir}'. Set 'tclLsp.serverPath' to the tcl-lsp project root.`,
      );
    }
    getOutputChannel().appendLine(`Dev mode: using uv in ${serverDir}`);
    serverOptions = {
      command: "uv",
      args: ["run", "--directory", serverDir, "--no-dev", "python", "-m", "lsp"],
      options: { cwd: serverDir },
    };
  } else {
    // VSIX mode: use bundled .pyz with discovered Python.
    const pyzPath = path.join(context.extensionPath, "tcl-lsp-server.pyz");
    if (!existsSync(pyzPath)) {
      window.showErrorMessage(
        "Tcl LSP: bundled server (tcl-lsp-server.pyz) not found. Reinstall the extension or set tclLsp.serverPath.",
      );
      return;
    }

    const configuredPython = config.get<string>("pythonPath", "auto");
    const python = await resolvePython(configuredPython);
    if (!python) {
      const msg =
        configuredPython && configuredPython !== "auto"
          ? `Tcl LSP: configured Python '${configuredPython}' not found or below 3.10.`
          : "Tcl LSP: no Python 3.10+ interpreter found. Install Python from python.org or set tclLsp.pythonPath.";
      window.showErrorMessage(msg);
      return;
    }

    serverOptions = {
      command: python.path,
      args: [pyzPath],
      options: { cwd: context.extensionPath },
    };
  }

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      ...[...TCL_LANGUAGE_IDS].flatMap((lang) => [
        { scheme: "file", language: lang },
        { scheme: "untitled", language: lang },
      ]),
    ],
    synchronize: {
      configurationSection: "tclLsp",
      fileEvents: workspace.createFileSystemWatcher(
        "**/*.{tcl,tk,itcl,tm,irul,irule,iapp,iappimpl,impl}",
      ),
    },
  };

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
      void applyDialectForEditor(editor);
      explorerEditorChanged();
      tkPreviewEditorChanged();
    }),
  );
  onActiveEditorChanged(window.activeTextEditor);

  // Update status bar label when manually changing settings.
  context.subscriptions.push(
    workspace.onDidChangeConfiguration((e) => {
      if (e.affectsConfiguration("tclLsp.dialect")) {
        const configured = workspace
          .getConfiguration("tclLsp")
          .get<string>("dialect", DEFAULT_DIALECT);
        activeDialect = configured;
        updateDialectStatusBar();
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
      const touchesFirstLine = e.contentChanges.some((change) => change.range.start.line === 0);
      if (touchesFirstLine) {
        void applyDialectForDocument(e.document);
      }
      explorerDocChanged();
      tkPreviewDocChanged();
    }),
  );

  // Also handle language-ready open events for newly opened files.
  context.subscriptions.push(
    workspace.onDidOpenTextDocument((document) => {
      if (window.activeTextEditor?.document.uri.toString() === document.uri.toString()) {
        void applyDialectForDocument(document);
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
    commands.registerCommand("tclLsp.openTkPreview", openTkPreview),
    commands.registerCommand("tclLsp.formatDocument", formatDocument),
    commands.registerCommand("tclLsp.minifyDocument", minifyDocument),
    commands.registerCommand("tclLsp.unminifyError", unminifyError),
    commands.registerCommand("tclLsp.escapeSelection", escapeSelection),
    commands.registerCommand("tclLsp.unescapeSelection", unescapeSelection),
    commands.registerCommand("tclLsp.base64EncodeSelection", base64EncodeSelection),
    commands.registerCommand("tclLsp.base64DecodeSelection", base64DecodeSelection),
    commands.registerCommand("tclLsp.copyFileAsBase64", copyFileAsBase64),
    commands.registerCommand("tclLsp.copyFileAsGzipBase64", copyFileAsGzipBase64),
    commands.registerCommand("tclLsp.translateXc", translateXc),
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
  );

  await client.start();
  await applyDialectForEditor(window.activeTextEditor);

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

  return { getClient };
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
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

function detectDialectFromDocument(document: TextDocument): string {
  const langDialect = LANGUAGE_ID_DIALECTS[document.languageId];
  if (langDialect) {
    return langDialect;
  }

  const fileName = document.fileName.toLowerCase();
  const extensionMatch = fileName.match(/(\.[^./\\]+)$/);
  const extension = extensionMatch ? extensionMatch[1] : "";

  let dialect = DEFAULT_DIALECT;
  switch (extension) {
    case ".irul":
    case ".irule":
      return "f5-irules";
    case ".iapp":
    case ".iappimpl":
    case ".impl":
      return "f5-iapps";
    case ".exp":
      return "expect";
    case ".tcl":
    case ".tk":
    case ".itcl":
    case ".tm":
      dialect = DEFAULT_DIALECT;
      break;
    default:
      dialect = DEFAULT_DIALECT;
      break;
  }

  if (document.lineCount > 0) {
    const firstLine = document.lineAt(0).text;
    if (/^#!.*\bexpect\b/i.test(firstLine)) {
      return "expect";
    }
    const shebangMatch = firstLine.match(/^#!.*\btclsh(\d+\.\d+)\b/i);
    if (shebangMatch) {
      const versionDialect = TCL_VERSION_DIALECTS[shebangMatch[1]];
      if (versionDialect) {
        dialect = versionDialect;
      }
    }
  }

  return dialect;
}

export async function setServerDialect(dialect: string): Promise<void> {
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

  if (!client) {
    return;
  }

  await client.sendNotification("workspace/didChangeConfiguration", {
    settings: { tclLsp: { dialect } },
  });
}

async function applyDialectForDocument(document: TextDocument): Promise<void> {
  if (!isTclLanguage(document.languageId)) {
    return;
  }
  const dialect = detectDialectFromDocument(document);
  await setServerDialect(dialect);
}

async function applyDialectForEditor(editor: TextEditor | undefined): Promise<void> {
  if (!editor || !isTclLanguage(editor.document.languageId)) {
    return;
  }
  await applyDialectForDocument(editor.document);
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
      },
      {
        label: "Minify + Compact Names",
        description: "Also shorten variable and proc names",
        compact: true,
        aggressive: false,
      },
      {
        label: "Aggressive",
        description: "Optimise, compact names, and minify for maximum compression",
        compact: false,
        aggressive: true,
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
    arguments: [uri, mode.compact, mode.aggressive],
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
  })) as { source: string; applied: Array<{ code: string; description: string }> } | null;

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
  const base = `${Date.now()}-${Math.random().toString(16).slice(2)}`;

  const checkerPath = path.join(tmpdir(), `tcl-lsp-checker-${base}.tcl`);
  const checkerScript = buildRuntimeValidationChecker(adapter);

  const targetPath = path.join(tmpdir(), `tcl-lsp-target-${base}.tcl`);

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
      unlinkSync(checkerPath);
    } catch {
      // ignore cleanup errors
    }
    try {
      unlinkSync(targetPath);
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

const TCL_ESCAPE_MAP: Record<string, string> = {
  "\\": "\\\\",
  "\n": "\\n",
  "\r": "\\r",
  "\t": "\\t",
  "\b": "\\b",
  "\f": "\\f",
  "\v": "\\v",
  '"': '\\"',
  $: "\\$",
  "[": "\\[",
  "]": "\\]",
};

const TCL_UNESCAPE_MAP: Record<string, string> = {
  "\\": "\\",
  n: "\n",
  r: "\r",
  t: "\t",
  b: "\b",
  f: "\f",
  v: "\v",
  '"': '"',
  $: "$",
  "[": "[",
  "]": "]",
};

function escapeTclText(text: string): string {
  return text.replace(/[\\\n\r\t\b\f\v"$\[\]]/g, (char) => TCL_ESCAPE_MAP[char]);
}

function unescapeTclText(text: string): string {
  return text.replace(
    /\\([\\nrtbfv"\$\[\]])/g,
    (_match, escaped: string) => TCL_UNESCAPE_MAP[escaped],
  );
}

async function transformSelection(
  transform: (text: string) => string,
  pastTenseAction: string,
  infinitiveAction: string,
): Promise<void> {
  const editor = window.activeTextEditor;
  if (!editor || !isTclLanguage(editor.document.languageId)) {
    window.showWarningMessage("Open a Tcl file to transform a selection.");
    return;
  }

  const selections = editor.selections.filter((selection) => !selection.isEmpty);
  if (selections.length === 0) {
    window.showWarningMessage("Select text in a Tcl file first.");
    return;
  }

  const applied = await editor.edit((editBuilder) => {
    for (const selection of selections) {
      const input = editor.document.getText(selection);
      editBuilder.replace(selection, transform(input));
    }
  });

  if (!applied) {
    window.showWarningMessage(`Failed to ${infinitiveAction} selection.`);
    return;
  }

  window.showInformationMessage(
    `${pastTenseAction} ${selections.length} selection${selections.length === 1 ? "" : "s"}.`,
  );
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

async function copyFileAsBase64(uri: vscode.Uri): Promise<void> {
  const fsPath = uri.fsPath;
  const size = statSync(fsPath).size;
  if (size > MAX_FILE_BYTES) {
    window.showWarningMessage(`File is ${size} bytes — exceeds ${MAX_FILE_BYTES} byte limit.`);
    return;
  }
  const encoded = readFileSync(fsPath).toString("base64");
  await vscode.env.clipboard.writeText(encoded);
  window.showInformationMessage(`Copied ${size} bytes as base64 (${encoded.length} chars).`);
}

async function copyFileAsGzipBase64(uri: vscode.Uri): Promise<void> {
  const fsPath = uri.fsPath;
  const size = statSync(fsPath).size;
  if (size > MAX_FILE_BYTES) {
    window.showWarningMessage(`File is ${size} bytes — exceeds ${MAX_FILE_BYTES} byte limit.`);
    return;
  }
  const raw = readFileSync(fsPath);
  const compressed = gzipSync(raw, { level: 9 });
  const encoded = compressed.toString("base64");
  await vscode.env.clipboard.writeText(encoded);
  window.showInformationMessage(
    `Copied ${size} → ${compressed.length} bytes gzipped as base64 (${encoded.length} chars).`,
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
