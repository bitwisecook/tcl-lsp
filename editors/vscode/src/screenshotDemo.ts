/**
 * Screenshot demo driver -- orchestrates VS Code to showcase extension
 * features for automated screenshot capture.
 *
 * This module is only imported when __SCREENSHOT_MODE__ is true (set by
 * esbuild --define).  It registers a command that runs through a sequence
 * of "scenes", each setting up editor state and signalling the outer shell
 * script to capture a screenshot.
 *
 * Architecture:
 *   - The shell script (scripts/screenshots.sh) runs from Terminal which
 *     has macOS Screen Recording permission and captures from the VS Code
 *     window ID.
 *   - This module writes process markers (including the extension host
 *     parent PID, usually VS Code main) to help the shell resolve the exact
 *     window for this run.
 *   - This module signals "capture <name>" by writing a .ready file.
 *   - The shell script watches for .ready files, runs screencapture, and
 *     writes a .captured file.
 *   - This module polls for .captured (every 50ms) and moves on.
 */

import * as vscode from "vscode";
import * as path from "path";
import * as fs from "fs";
import * as os from "os";
import { buildIruleEventSkeleton } from "./iruleSkeleton";
import { loadTemplateSnippets, renderTemplateSnippet } from "./templateSnippets";
import { getClient } from "./extension";
import {
  switchCompilerExplorerTab,
  closeCompilerExplorer,
  waitForCompileComplete,
  forceCompileFromActiveEditor,
} from "./compilerExplorer.js";

// Types

interface Scene {
  /** Used as the screenshot filename prefix (e.g. "01-diagnostics"). */
  name: string;
  /** Set up editor state for this scene. */
  run: () => Promise<void>;
  /** When true, keep editors from the previous scene open (e.g. for
   *  continuation scenes like formatting-after). */
  keepEditors?: boolean;
  /** When true, only run when AI chat is available and authenticated. */
  requiresAi?: boolean;
}

interface SampleCursor {
  line: number;
  col: number;
}

interface ScreenshotSample {
  content: string;
  cursor?: SampleCursor;
}

// Screenshot signalling (extension ↔ shell script)

const OUTPUT_DIR =
  process.env.SCREENSHOT_OUTPUT_DIR || path.join(process.cwd(), "build", "screenshots");
const AI_STARTED_MARKER = path.join(OUTPUT_DIR, ".ai-started");
const AI_SUCCESS_MARKER = path.join(OUTPUT_DIR, ".ai-success");
const AI_DONE_MARKER = path.join(OUTPUT_DIR, ".ai-done");
const VSCODE_MAIN_PID_MARKER = path.join(OUTPUT_DIR, ".vscode-main-pid");
const EXT_HOST_PID_MARKER = path.join(OUTPUT_DIR, ".vscode-ext-host-pid");
const CHAT_SUBMIT_COMMANDS = [
  "workbench.action.chat.submit",
  "workbench.action.chat.acceptInput",
  "interactive.acceptInput",
  "workbench.action.chat.send",
];
const SCENE_SCRATCH_DIR = path.join(os.tmpdir(), "tcl-lsp-screenshot-scenes");
const SAMPLE_CARET_MARKER_SUFFIX = "^--- cursor";
const SAMPLE_LEFT_MARGIN_MARKER = /^(\s*)#\s*<<---\s*cursor on left margin\s*$/;
const SAMPLE_ONE_IN_MARGIN_MARKER = /^(\s*)#\s*`---\s*cursor one in from left margin\s*$/;

let aiScenesEnabled = true;

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function unlinkIfExists(filePath: string): void {
  try {
    fs.unlinkSync(filePath);
  } catch {}
}

function clearAiMarkers(): void {
  unlinkIfExists(AI_STARTED_MARKER);
  unlinkIfExists(AI_SUCCESS_MARKER);
  unlinkIfExists(AI_DONE_MARKER);
}

function writeProcessMarkers(): void {
  fs.mkdirSync(OUTPUT_DIR, { recursive: true });
  fs.writeFileSync(VSCODE_MAIN_PID_MARKER, `${process.ppid}\n`, "utf8");
  fs.writeFileSync(EXT_HOST_PID_MARKER, `${process.pid}\n`, "utf8");
  console.log(
    `[screenshot-demo] Process markers: main(pid from ppid)=${process.ppid} ext-host=${process.pid}`,
  );
}

async function awaitAiRequestStarted(timeoutMs = 10_000): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (fs.existsSync(AI_STARTED_MARKER)) {
      unlinkIfExists(AI_STARTED_MARKER);
      console.log("[screenshot-demo] AI request started");
      return;
    }
    await sleep(200);
  }
  throw new Error(`AI request did not start within ${timeoutMs}ms`);
}

/**
 * Signal the shell script to capture a screenshot, then wait for it.
 *
 * Protocol:
 *   1. Extension writes  OUTPUT_DIR/<name>.ready
 *   2. Shell   writes    OUTPUT_DIR/<name>.captured  (and removes .ready)
 *   3. Extension removes .captured and proceeds
 */
async function captureScreenshot(sceneName: string): Promise<void> {
  fs.mkdirSync(OUTPUT_DIR, { recursive: true });

  const readyFile = path.join(OUTPUT_DIR, `${sceneName}.ready`);
  const capturedFile = path.join(OUTPUT_DIR, `${sceneName}.captured`);

  // Signal readiness.
  fs.writeFileSync(readyFile, "", "utf8");
  console.log(`[screenshot-demo] Signalled: ${sceneName}`);

  // Wait for the shell to capture (poll every 50ms, timeout 15s).
  const deadline = Date.now() + 15_000;
  while (Date.now() < deadline) {
    if (fs.existsSync(capturedFile)) {
      try {
        fs.unlinkSync(capturedFile);
      } catch {}
      console.log(`[screenshot-demo] Captured: ${sceneName}`);
      return;
    }
    await sleep(50);
  }
  console.error(`[screenshot-demo] Timeout waiting for capture: ${sceneName}`);
}

/**
 * Wait for an AI chat response to complete successfully.
 *
 * The chat participant handler writes:
 *   - `.ai-success` only when the request completed successfully
 *   - `.ai-done` for every completion path (success or error)
 * We require success before proceeding with screenshots.
 */
async function awaitAiResponse(timeoutMs = 60_000): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (fs.existsSync(AI_DONE_MARKER)) {
      const success = fs.existsSync(AI_SUCCESS_MARKER);
      unlinkIfExists(AI_DONE_MARKER);
      unlinkIfExists(AI_SUCCESS_MARKER);
      if (!success) {
        throw new Error(
          "AI request finished without a successful response (GitHub/Copilot likely not signed in)",
        );
      }
      console.log("[screenshot-demo] AI response complete");
      return;
    }
    await sleep(200);
  }
  throw new Error(`AI response timed out after ${timeoutMs}ms`);
}

async function hasGitHubAuthSession(): Promise<boolean> {
  // Try multiple approaches — the silent flag can miss sessions in
  // test-electron when the auth provider hasn't been warmed up yet.
  const scopes = ["read:user"];
  for (const silent of [true, false]) {
    try {
      const session = await vscode.authentication.getSession("github", scopes, {
        createIfNone: false,
        silent,
      });
      if (session) {
        console.log(`[screenshot-demo] GitHub auth found (silent=${silent})`);
        return true;
      }
    } catch {
      // Expected when silent=false and user dismisses the prompt
    }
  }
  // Also check for Copilot's own auth provider which uses "github-enterprise"
  // in some configurations.
  try {
    const session = await vscode.authentication.getSession("github-enterprise", scopes, {
      createIfNone: false,
      silent: true,
    });
    if (session) {
      console.log("[screenshot-demo] GitHub Enterprise auth found");
      return true;
    }
  } catch {}
  return false;
}

async function hasChatModelsAvailable(): Promise<boolean> {
  // Retry a few times — models may take a moment to register after login.
  for (let i = 0; i < 3; i++) {
    try {
      const models = await vscode.lm.selectChatModels();
      if (models.length > 0) {
        console.log(`[screenshot-demo] ${models.length} chat model(s) available`);
        return true;
      }
    } catch {}
    if (i < 2) {
      await sleep(2000);
    }
  }
  return false;
}

async function submitChatIfNeeded(): Promise<boolean> {
  const available = new Set(await vscode.commands.getCommands(true));
  for (const command of CHAT_SUBMIT_COMMANDS) {
    if (!available.has(command)) {
      continue;
    }
    try {
      await vscode.commands.executeCommand(command);
      console.log(`[screenshot-demo] Submitted chat with ${command}`);
      return true;
    } catch {}
  }
  return false;
}

interface AiQueryOptions {
  allowSubmitFallback?: boolean;
  startNewChat?: boolean;
  settleDelayMs?: number;
}

async function runAiChatQuery(
  query: string,
  timeoutMs = 90_000,
  options: AiQueryOptions = {},
): Promise<void> {
  const allowSubmitFallback = options.allowSubmitFallback ?? true;
  const startNewChat = options.startNewChat ?? true;
  const settleDelayMs = options.settleDelayMs ?? 2_200;

  if (startNewChat) {
    // Some VS Code builds keep stale responses visible unless we
    // aggressively reset the thread first.
    for (let i = 0; i < 2; i++) {
      try {
        await vscode.commands.executeCommand("workbench.action.chat.new");
      } catch {}
      await sleep(220);
    }
    for (const clearCommand of [
      "workbench.action.chat.clear",
      "workbench.action.closeAuxiliaryBar",
    ]) {
      try {
        await vscode.commands.executeCommand(clearCommand);
      } catch {}
    }
    await sleep(360);
  }

  clearAiMarkers();

  await vscode.commands.executeCommand("workbench.action.chat.open", {
    query,
    isPartialQuery: false,
  });

  try {
    await awaitAiRequestStarted(10_000);
  } catch (err) {
    if (!allowSubmitFallback) {
      throw err;
    }
    const submitted = await submitChatIfNeeded();
    if (!submitted) {
      throw err;
    }
    await awaitAiRequestStarted(10_000);
  }

  await awaitAiResponse(timeoutMs);
  await sleep(settleDelayMs);
}

async function openAnyAvailableSignInFlow(): Promise<boolean> {
  const available = new Set(await vscode.commands.getCommands(true));
  const signInCandidates = [
    "github.copilot.signIn",
    "github.copilot-chat.signIn",
    "workbench.action.chat.triggerSetup",
    "workbench.action.chat.open",
    "workbench.action.openSettings",
  ];
  for (const command of signInCandidates) {
    if (!available.has(command)) {
      continue;
    }
    try {
      if (command === "workbench.action.chat.open") {
        await vscode.commands.executeCommand(command, {
          query: "@irule /help",
          isPartialQuery: false,
        });
      } else {
        await vscode.commands.executeCommand(command);
      }
      console.log(`[screenshot-demo] Triggered sign-in helper command: ${command}`);
      return true;
    } catch {}
  }
  return false;
}

function getCopilotChatExtension(): vscode.Extension<unknown> | undefined {
  return (
    vscode.extensions.getExtension("GitHub.copilot-chat") ??
    vscode.extensions.getExtension("github.copilot-chat") ??
    vscode.extensions.all.find((ext) => ext.id.toLowerCase() === "github.copilot-chat")
  );
}

async function waitForCopilotChatExtension(
  timeoutMs = 12_000,
): Promise<vscode.Extension<unknown> | undefined> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const ext = getCopilotChatExtension();
    if (ext) {
      return ext;
    }
    await sleep(250);
  }
  return undefined;
}

// Editor helpers

/**
 * Throw if Copilot Chat is not installed.  The catch in the scene loop
 * will skip capture so we don't produce useless screenshots.
 *
 * Note: In VS Code 1.109+ the base Copilot is built into the core binary
 * (no separate `GitHub.copilot` extension), so we only check for the chat
 * extension which remains a marketplace install.
 */
function requireCopilot(): void {
  if (!aiScenesEnabled) {
    throw new Error("AI scenes are disabled for this run");
  }
  if (!getCopilotChatExtension()) {
    throw new Error("GitHub Copilot Chat extension not available — skipping AI scene");
  }
}

/**
 * Open the Problems (Markers) panel at the bottom of the editor.
 *
 * In the test-electron environment the bottom panel may start with zero
 * persisted height.  We create a temporary terminal to force the panel area
 * to allocate real height, switch to the Problems tab, then allow the UI
 * to settle before capture.
 *
 * The terminal is intentionally NOT disposed here — disposing it before
 * capture can cause the panel to collapse.  It gets cleaned up when
 * closeAllEditors / closePanel runs for the next scene.
 */
async function showProblemsPanel(): Promise<void> {
  // 1. Force the panel area open by creating a visible terminal.
  //    This is intentionally NOT disposed — disposing before capture can
  //    cause the panel to collapse.
  const term = vscode.window.createTerminal("_screenshot-panel");
  term.show(false); // preserveFocus=true → don't steal focus from editor
  await sleep(800);

  // 2. Switch to the Problems tab.
  try {
    await vscode.commands.executeCommand("workbench.actions.view.problems");
  } catch {}
  await sleep(500);
  try {
    await vscode.commands.executeCommand("workbench.action.problems.focus");
  } catch {}
  await sleep(400);
  await sortProblemsBySeverity();
  await sleep(200);

  // 3. Reset the panel to a consistent ~1/3 height.  VS Code persists the
  //    panel height across open/close cycles, so repeated increaseViewSize
  //    calls compound and eventually the panel consumes the whole tab.
  //    Fix: shrink to minimum first, then grow to the desired size.
  try {
    await vscode.commands.executeCommand("workbench.action.focusPanel");
    await sleep(100);
    // Shrink to minimum (overshoot to ensure we hit the floor).
    for (let i = 0; i < 20; i++) {
      await vscode.commands.executeCommand("workbench.action.decreaseViewSize");
    }
    // Grow to ~1/3 of the window.
    for (let i = 0; i < 6; i++) {
      await vscode.commands.executeCommand("workbench.action.increaseViewSize");
    }
  } catch {}
  await sleep(300);

  // 4. Return focus to the editor so the cursor stays visible.
  try {
    await vscode.commands.executeCommand("workbench.action.focusActiveEditorGroup");
  } catch {}
  await sleep(500);
}

function workspaceRoot(): string {
  const ws = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  if (!ws) {
    throw new Error("No workspace folder open");
  }
  return ws;
}

function screenshotSamplesDir(): string {
  return path.join(workspaceRoot(), "samples", "for_screenshots");
}

function parseScreenshotSample(content: string): ScreenshotSample {
  const inputLines = content.replace(/\r\n/g, "\n").split("\n");
  const outputLines: string[] = [];
  let cursor: SampleCursor | undefined;

  for (const line of inputLines) {
    const isCommentLine = /^\s*#/.test(line);
    const leftMarginMatch = line.match(SAMPLE_LEFT_MARGIN_MARKER);
    const oneInMarginMatch = line.match(SAMPLE_ONE_IN_MARGIN_MARKER);

    if (isCommentLine && leftMarginMatch) {
      if (!cursor) {
        cursor = { line: outputLines.length, col: 0 };
      }
      continue;
    }

    if (isCommentLine && oneInMarginMatch) {
      if (!cursor) {
        cursor = { line: outputLines.length, col: 1 };
      }
      continue;
    }

    const caretMarkerIndex = line.indexOf(SAMPLE_CARET_MARKER_SUFFIX);
    if (isCommentLine && caretMarkerIndex >= 0) {
      if (!cursor) {
        const caretColumn = Math.max(line.indexOf("^"), 0);
        cursor = {
          line: Math.max(outputLines.length - 1, 0),
          col: caretColumn,
        };
      }
      continue;
    }

    outputLines.push(line);
  }

  if (cursor) {
    const maxLine = Math.max(outputLines.length - 1, 0);
    const clampedLine = Math.min(Math.max(cursor.line, 0), maxLine);
    const lineText = outputLines[clampedLine] ?? "";
    cursor = {
      line: clampedLine,
      col: Math.min(Math.max(cursor.col, 0), lineText.length),
    };
  }

  return {
    content: outputLines.join("\n"),
    cursor,
  };
}

function loadScreenshotSample(fileName: string): ScreenshotSample {
  const samplePath = path.join(screenshotSamplesDir(), fileName);
  const sampleText = fs.readFileSync(samplePath, "utf8");
  return parseScreenshotSample(sampleText);
}

function setCursorFromSample(editor: vscode.TextEditor, sample: ScreenshotSample): void {
  if (!sample.cursor) {
    return;
  }
  setCursor(editor, sample.cursor.line, sample.cursor.col);
}

async function closeAllEditors(): Promise<void> {
  await vscode.commands.executeCommand("workbench.action.closeAllEditors");
  await sleep(100);
}

async function openExampleFile(relativePath: string): Promise<vscode.TextEditor> {
  const uri = vscode.Uri.file(path.join(workspaceRoot(), relativePath));
  const doc = await vscode.workspace.openTextDocument(uri);
  return vscode.window.showTextDocument(doc, vscode.ViewColumn.One);
}

async function ensureTwoColumnEditorLayout(): Promise<void> {
  // Reset layout first so split scenes never accumulate extra groups
  // from earlier interactions.
  try {
    await vscode.commands.executeCommand("workbench.action.editorLayoutSingle");
  } catch {}
  await sleep(120);
  try {
    await vscode.commands.executeCommand("workbench.action.editorLayoutTwoColumns");
  } catch {}
  await sleep(180);
}

async function ensureSingleEditorLayout(): Promise<void> {
  try {
    await vscode.commands.executeCommand("workbench.action.editorLayoutSingle");
  } catch {}
  await sleep(140);
}

async function openSplitScratchEditorsWithContent(
  leftFileName: string,
  rightFileName: string,
  leftContent: string,
  rightContent: string,
): Promise<{ left: vscode.TextEditor; right: vscode.TextEditor }> {
  await ensureTwoColumnEditorLayout();

  fs.mkdirSync(SCENE_SCRATCH_DIR, { recursive: true });
  const leftPath = path.join(SCENE_SCRATCH_DIR, leftFileName);
  const rightPath = path.join(SCENE_SCRATCH_DIR, rightFileName);
  fs.writeFileSync(leftPath, leftContent, "utf8");
  fs.writeFileSync(rightPath, rightContent, "utf8");

  const leftDoc = await vscode.workspace.openTextDocument(vscode.Uri.file(leftPath));
  const rightDoc = await vscode.workspace.openTextDocument(vscode.Uri.file(rightPath));

  const leftEditor = await vscode.window.showTextDocument(leftDoc, {
    viewColumn: vscode.ViewColumn.One,
    preserveFocus: true,
    preview: false,
  });
  const rightEditor = await vscode.window.showTextDocument(rightDoc, {
    viewColumn: vscode.ViewColumn.Two,
    preserveFocus: false,
    preview: false,
  });
  return { left: leftEditor, right: rightEditor };
}

async function openSplitScratchEditors(
  leftFileName: string,
  rightFileName: string,
  content: string,
): Promise<{ left: vscode.TextEditor; right: vscode.TextEditor }> {
  return openSplitScratchEditorsWithContent(leftFileName, rightFileName, content, content);
}

async function openScratchEditor(
  fileName: string,
  content: string,
  viewColumn = vscode.ViewColumn.One,
): Promise<vscode.TextEditor> {
  await ensureSingleEditorLayout();
  fs.mkdirSync(SCENE_SCRATCH_DIR, { recursive: true });
  const filePath = path.join(SCENE_SCRATCH_DIR, fileName);
  fs.writeFileSync(filePath, content, "utf8");
  const doc = await vscode.workspace.openTextDocument(vscode.Uri.file(filePath));
  return vscode.window.showTextDocument(doc, {
    viewColumn,
    preserveFocus: false,
    preview: false,
  });
}

function setCursor(editor: vscode.TextEditor, line: number, col: number): void {
  const pos = new vscode.Position(line, col);
  editor.selection = new vscode.Selection(pos, pos);
  editor.revealRange(new vscode.Range(pos, pos), vscode.TextEditorRevealType.InCenter);
}

function keepCursorVisible(editor: vscode.TextEditor): void {
  const pos = editor.selection.active;
  editor.selection = new vscode.Selection(pos, pos);
  editor.revealRange(
    new vscode.Range(pos, pos),
    vscode.TextEditorRevealType.InCenterIfOutsideViewport,
  );
}

/**
 * Find a document symbol by name and position the cursor on it.
 *
 * Uses the LSP documentSymbol provider to walk the symbol tree, so cursor
 * positions adapt automatically if the example file is edited.  Falls back
 * to a text search if no symbol provider is registered yet.
 */
async function setCursorOnSymbol(
  editor: vscode.TextEditor,
  symbolName: string,
  timeoutMs = 5_000,
): Promise<void> {
  const uri = editor.document.uri;

  // Retry symbol lookup — the LSP may still be indexing after file open.
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const symbols = (await vscode.commands.executeCommand(
      "vscode.executeDocumentSymbolProvider",
      uri,
    )) as vscode.DocumentSymbol[] | vscode.SymbolInformation[] | undefined;

    if (symbols && symbols.length > 0) {
      const found = findSymbolByName(symbols, symbolName);
      if (found) {
        const pos =
          "selectionRange" in found ? found.selectionRange.start : found.location.range.start;
        editor.selection = new vscode.Selection(pos, pos);
        editor.revealRange(new vscode.Range(pos, pos), vscode.TextEditorRevealType.InCenter);
        return;
      }
    }
    await sleep(200);
  }

  // Fallback: search the document text for the symbol name.
  const text = editor.document.getText();
  const idx = text.indexOf(symbolName);
  if (idx >= 0) {
    const pos = editor.document.positionAt(idx);
    editor.selection = new vscode.Selection(pos, pos);
    editor.revealRange(new vscode.Range(pos, pos), vscode.TextEditorRevealType.InCenter);
    console.log(
      `[screenshot-demo] Symbol "${symbolName}" found via text search at ${pos.line}:${pos.character}`,
    );
    return;
  }

  console.error(`[screenshot-demo] Symbol "${symbolName}" not found — cursor not positioned`);
}

/**
 * Position the cursor on the first occurrence of `text` in the document.
 *
 * Useful for call sites and other non-symbol tokens that wouldn't appear in
 * the document symbol tree.
 */
function setCursorOnText(editor: vscode.TextEditor, text: string): void {
  const docText = editor.document.getText();
  const idx = docText.indexOf(text);
  if (idx >= 0) {
    const pos = editor.document.positionAt(idx);
    editor.selection = new vscode.Selection(pos, pos);
    editor.revealRange(new vscode.Range(pos, pos), vscode.TextEditorRevealType.InCenter);
  } else {
    console.error(`[screenshot-demo] Text "${text}" not found in document`);
  }
}

/** Recursively search for a symbol by name in a DocumentSymbol/SymbolInformation tree. */
function findSymbolByName(
  symbols: (vscode.DocumentSymbol | vscode.SymbolInformation)[],
  name: string,
): vscode.DocumentSymbol | vscode.SymbolInformation | undefined {
  for (const sym of symbols) {
    if (sym.name === name) {
      return sym;
    }
    // DocumentSymbol has children; SymbolInformation does not.
    if ("children" in sym && sym.children.length > 0) {
      const child = findSymbolByName(sym.children, name);
      if (child) {
        return child;
      }
    }
  }
  return undefined;
}

async function waitForDiagnostics(
  uri: vscode.Uri,
  minCount: number,
  timeoutMs = 10_000,
): Promise<vscode.Diagnostic[]> {
  const immediate = vscode.languages.getDiagnostics(uri);
  if (immediate.length >= minCount) {
    return immediate;
  }

  return new Promise<vscode.Diagnostic[]>((resolve) => {
    let resolved = false;
    const finish = (diags: vscode.Diagnostic[]) => {
      if (resolved) return;
      resolved = true;
      clearTimeout(timer);
      clearInterval(poller);
      disposable.dispose();
      resolve(diags);
    };
    const timer = setTimeout(() => finish(vscode.languages.getDiagnostics(uri)), timeoutMs);
    const disposable = vscode.languages.onDidChangeDiagnostics((e) => {
      if (e.uris.some((u) => u.toString() === uri.toString())) {
        const diags = vscode.languages.getDiagnostics(uri);
        if (diags.length >= minCount) finish(diags);
      }
    });
    const poller = setInterval(() => {
      const diags = vscode.languages.getDiagnostics(uri);
      if (diags.length >= minCount) finish(diags);
    }, 200);
  });
}

async function waitForDiagnosticsReady(
  uri: vscode.Uri,
  options: {
    minCount: number;
    minErrors?: number;
    minWarnings?: number;
    timeoutMs?: number;
    stableMs?: number;
  },
): Promise<vscode.Diagnostic[]> {
  const minErrors = options.minErrors ?? 0;
  const minWarnings = options.minWarnings ?? 0;
  const timeoutMs = options.timeoutMs ?? 12_000;
  const stableMs = options.stableMs ?? 900;

  const deadline = Date.now() + timeoutMs;
  let goodSince = 0;
  let latest = vscode.languages.getDiagnostics(uri);

  while (Date.now() < deadline) {
    latest = vscode.languages.getDiagnostics(uri);
    const errors = latest.filter((d) => d.severity === vscode.DiagnosticSeverity.Error).length;
    const warnings = latest.filter((d) => d.severity === vscode.DiagnosticSeverity.Warning).length;
    const good =
      latest.length >= options.minCount && errors >= minErrors && warnings >= minWarnings;
    if (good) {
      if (goodSince === 0) {
        goodSince = Date.now();
      }
      if (Date.now() - goodSince >= stableMs) {
        return latest;
      }
    } else {
      goodSince = 0;
    }
    await sleep(150);
  }

  return latest;
}

async function waitForSemanticTokens(
  uri: vscode.Uri,
  minDataEntries = 40,
  timeoutMs = 10_000,
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const tokens = (await vscode.commands.executeCommand(
        "vscode.provideDocumentSemanticTokens",
        uri,
      )) as { data?: Uint32Array | number[] } | undefined;
      const count = tokens?.data?.length ?? 0;
      if (count >= minDataEntries) {
        return;
      }
    } catch {}
    await sleep(200);
  }
  console.log(`[screenshot-demo] Semantic tokens timeout for ${uri.toString()}`);
}

async function sortProblemsBySeverity(): Promise<void> {
  for (const cmd of ["problems.action.sortBySeverity", "problems.action.focusErrors"]) {
    try {
      await vscode.commands.executeCommand(cmd);
    } catch {}
  }
}

async function openCompilerExplorerForActiveEditor(timeoutMs = 12_000): Promise<void> {
  try {
    await vscode.commands.executeCommand("workbench.action.focusActiveEditorGroup");
  } catch {}
  await sleep(100);

  // First open creates/reveals the panel.
  await vscode.commands.executeCommand("tclLsp.openCompilerExplorer");
  await sleep(350);

  // Second open on an existing panel pushes active editor source immediately.
  let compiled = waitForCompileComplete(timeoutMs);
  await vscode.commands.executeCommand("tclLsp.openCompilerExplorer");
  let compiledOk = await compiled;

  // If the webview missed the initial compile trigger, force one directly.
  if (!compiledOk) {
    compiled = waitForCompileComplete(8_000);
    const forced = await forceCompileFromActiveEditor();
    compiledOk = forced ? await compiled : false;
  }
  if (!compiledOk) {
    throw new Error("Compiler explorer did not compile source in time");
  }
  await sleep(450);
}

interface AppliedOptimisationSummary {
  code: string;
  message: string;
  line: number | null;
}

interface ApplyOptimisationsResult {
  count: number;
  source: string | undefined;
  optimisations: AppliedOptimisationSummary[];
}

async function applyAllOptimisations(editor: vscode.TextEditor): Promise<ApplyOptimisationsResult> {
  const client = getClient();
  if (!client) {
    return { count: 0, source: undefined, optimisations: [] };
  }

  const uri = editor.document.uri.toString();
  const result = (await client.sendRequest("workspace/executeCommand", {
    command: "tcl-lsp.optimiseDocument",
    arguments: [uri],
  })) as {
    optimisations?: Array<Record<string, unknown>>;
    source?: string;
  } | null;

  const rawOptimisations = result?.optimisations ?? [];
  const source = result?.source;
  if (rawOptimisations.length === 0 || typeof source !== "string") {
    return { count: 0, source: undefined, optimisations: [] };
  }

  const optimisations: AppliedOptimisationSummary[] = rawOptimisations.map((item) => {
    const range = item.range as
      | {
          start?: { line?: number; character?: number };
          end?: { line?: number; character?: number };
        }
      | undefined;
    return {
      code: typeof item.code === "string" ? item.code : "",
      message: typeof item.message === "string" ? item.message : "",
      line: typeof range?.start?.line === "number" ? range.start.line + 1 : null,
    };
  });

  const fullRange = new vscode.Range(
    editor.document.positionAt(0),
    editor.document.positionAt(editor.document.getText().length),
  );
  await editor.edit((editBuilder) => {
    editBuilder.replace(fullRange, source);
  });
  return {
    count: optimisations.length,
    source,
    optimisations,
  };
}

async function applyAllSafeFixes(editor: vscode.TextEditor): Promise<number> {
  const client = getClient();
  if (!client) {
    return 0;
  }

  const uri = editor.document.uri.toString();
  const result = (await client.sendRequest("workspace/executeCommand", {
    command: "tcl-lsp.fixAllSafeIssues",
    arguments: [uri],
  })) as { applied?: Array<Record<string, unknown>>; source?: string } | null;

  const applied = result?.applied ?? [];
  const source = result?.source;
  if (applied.length === 0 || typeof source !== "string") {
    return 0;
  }

  const fullRange = new vscode.Range(
    editor.document.positionAt(0),
    editor.document.positionAt(editor.document.getText().length),
  );
  await editor.edit((editBuilder) => {
    editBuilder.replace(fullRange, source);
  });
  return applied.length;
}

async function waitForDocumentTextChange(
  document: vscode.TextDocument,
  previousText: string,
  timeoutMs = 6_000,
): Promise<boolean> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (document.getText() !== previousText) {
      return true;
    }
    await sleep(120);
  }
  return false;
}

function ensureCursorInVisibleEditor(): void {
  const editor =
    vscode.window.activeTextEditor ??
    vscode.window.visibleTextEditors.find((visible) =>
      isTclLanguageId(visible.document.languageId),
    );
  if (!editor) {
    return;
  }
  keepCursorVisible(editor);
}

function isTclLanguageId(languageId: string): boolean {
  return (
    languageId === "tcl" ||
    languageId === "tcl-irule" ||
    languageId === "tcl-iapp" ||
    languageId === "tcl-bigip" ||
    languageId === "tcl8.4" ||
    languageId === "tcl8.5" ||
    languageId === "tcl9.0" ||
    languageId === "tcl-eda"
  );
}

async function configureScreenshotCursor(): Promise<void> {
  // Screenshot runs use a dedicated user-data-dir, so setting global editor
  // prefs here does not alter the user's normal VS Code profile.
  const target = vscode.ConfigurationTarget.Global;
  try {
    await vscode.workspace.getConfiguration("editor").update("cursorBlinking", "solid", target);
  } catch {}
  try {
    await vscode.workspace
      .getConfiguration("terminal.integrated")
      .update("cursorBlinking", false, target);
  } catch {}
  try {
    await vscode.workspace.getConfiguration("editor").update("inlayHints.enabled", "off", target);
  } catch {}
}

async function openAiSceneEditor(
  fileName: string,
  sampleFileName = "ai-scene.irul",
): Promise<vscode.TextEditor> {
  const target = vscode.workspace.workspaceFolders?.length
    ? undefined
    : vscode.ConfigurationTarget.Global;
  await vscode.workspace.getConfiguration("tclLsp").update("dialect", "f5-irules", target);
  const sample = loadScreenshotSample(sampleFileName);
  const editor = await openScratchEditor(fileName, sample.content);
  await waitForSemanticTokens(editor.document.uri, 40, 12_000);
  setCursorFromSample(editor, sample);
  if (!sample.cursor) {
    setCursorOnText(editor, 'if {[HTTP::uri] eq "/admin"}');
  }
  keepCursorVisible(editor);
  return editor;
}

// Scene definitions

function buildScenes(): Scene[] {
  return [
    {
      name: "01-diagnostics-overview",
      run: async () => {
        const editor = await openExampleFile(
          "samples/for_screenshots/05-security-taint-before.irul",
        );
        await waitForSemanticTokens(editor.document.uri, 80, 12_000);
        await waitForDiagnosticsReady(editor.document.uri, {
          minCount: 8,
          minErrors: 1,
          minWarnings: 1,
          stableMs: 1_200,
          timeoutMs: 15_000,
        });
        setCursor(editor, 13, 4);
        await showProblemsPanel();
        keepCursorVisible(editor);
        await sleep(500);
      },
    },
    {
      name: "02-hover-proc",
      run: async () => {
        await closePanel();
        const editor = await openExampleFile("samples/for_screenshots/03-completions.tcl");
        await waitForSemanticTokens(editor.document.uri, 60, 10_000);
        await sleep(300);
        // Position on "::math::add" call to show hover with signature info.
        // Use text search for the qualified call since it's not a symbol def.
        setCursorOnText(editor, "::math::add");
        await vscode.commands.executeCommand("editor.action.showHover");
        keepCursorVisible(editor);
        await sleep(900);
      },
    },
    {
      name: "03-completions",
      run: async () => {
        await closePanel();
        const sample = loadScreenshotSample("03-completions.tcl");
        const editor = await openScratchEditor("completions-scene.tcl", sample.content);
        await waitForSemanticTokens(editor.document.uri, 20, 8_000);
        await sleep(300);
        setCursorFromSample(editor, sample);
        if (!sample.cursor) {
          setCursor(editor, 1, 3);
        }
        await vscode.commands.executeCommand("editor.action.triggerSuggest");
        keepCursorVisible(editor);
        await sleep(1_000);
      },
    },
    {
      name: "04-quickfix",
      run: async () => {
        await closePanel();
        const editor = await openExampleFile(
          "samples/for_screenshots/05-security-taint-before.irul",
        );
        await waitForDiagnostics(editor.document.uri, 3);
        setCursor(editor, 13, 4);
        // Give the code action provider generous time to register after
        // diagnostics arrive — the provider runs asynchronously and the
        // larger window means more rendering work before actions are ready.
        await sleep(2000);
        await vscode.commands.executeCommand("editor.action.quickFix");
        await sleep(600);
      },
    },
    {
      name: "05-security-taint",
      run: async () => {
        await closePanel();
        const target = vscode.workspace.workspaceFolders?.length
          ? undefined
          : vscode.ConfigurationTarget.Global;
        await vscode.workspace.getConfiguration("tclLsp").update("dialect", "f5-irules", target);

        const beforeSample = loadScreenshotSample("05-security-taint-before.irul");
        const afterSample = loadScreenshotSample("05-security-taint-after.irul");

        const split = await openSplitScratchEditorsWithContent(
          "security-taint-before-left.irul",
          "security-taint-after-right.irul",
          beforeSample.content,
          afterSample.content,
        );
        await Promise.all([
          waitForSemanticTokens(split.left.document.uri, 10, 8_000),
          waitForSemanticTokens(split.right.document.uri, 10, 8_000),
        ]);
        await waitForDiagnosticsReady(split.left.document.uri, {
          minCount: 2,
          minWarnings: 2,
          timeoutMs: 12_000,
          stableMs: 900,
        });
        await vscode.window.showTextDocument(split.left.document, {
          viewColumn: vscode.ViewColumn.One,
          preserveFocus: false,
          preview: false,
        });
        setCursorFromSample(split.left, beforeSample);
        setCursorFromSample(split.right, afterSample);
        if (!beforeSample.cursor) {
          setCursorOnText(split.left, "eval $payload");
        }
        if (!afterSample.cursor) {
          setCursorOnText(split.right, "set payload_len");
        }
        await showProblemsPanel();
        keepCursorVisible(split.left);
        await sleep(700);
      },
    },
    {
      name: "07-formatting-after",
      run: async () => {
        await closePanel();
        // Deliberately ugly code shown in both panes. We format only the
        // right pane so this single screenshot is a before/after pair.
        const sample = loadScreenshotSample("07-formatting-side-by-side.tcl");
        const split = await openSplitScratchEditors(
          "formatting-before-left.tcl",
          "formatting-after-right.tcl",
          sample.content,
        );
        split.left.options = { tabSize: 4, insertSpaces: true };
        split.right.options = { tabSize: 4, insertSpaces: true };
        await Promise.all([
          waitForSemanticTokens(split.left.document.uri, 30, 8_000),
          waitForSemanticTokens(split.right.document.uri, 30, 8_000),
        ]);
        setCursorFromSample(split.left, sample);
        setCursorFromSample(split.right, sample);
        if (!sample.cursor) {
          setCursor(split.left, 4, 8);
          setCursor(split.right, 4, 8);
        }
        await vscode.window.showTextDocument(split.right.document, {
          viewColumn: vscode.ViewColumn.Two,
          preserveFocus: false,
          preview: false,
        });
        const beforeFormat = split.right.document.getText();
        await vscode.commands.executeCommand("editor.action.formatDocument");
        const formatChanged = await waitForDocumentTextChange(split.right.document, beforeFormat);
        if (!formatChanged) {
          console.log("[screenshot-demo] formatting scene: right pane text unchanged after format");
        }
        if (sample.cursor) {
          setCursor(split.right, sample.cursor.line, sample.cursor.col);
        } else {
          setCursor(split.right, 4, 8);
        }
        keepCursorVisible(split.right);
        await sleep(700);
      },
    },
    {
      name: "08-style-warnings",
      run: async () => {
        await closePanel();
        const sample = loadScreenshotSample("08-style-warnings-before.tcl");
        const split = await openSplitScratchEditors(
          "style-warnings-before-left.tcl",
          "style-warnings-after-right.tcl",
          sample.content,
        );
        split.left.options = { tabSize: 4, insertSpaces: true };
        split.right.options = { tabSize: 4, insertSpaces: true };
        await Promise.all([
          waitForSemanticTokens(split.left.document.uri, 50, 10_000),
          waitForSemanticTokens(split.right.document.uri, 50, 10_000),
        ]);
        await waitForDiagnosticsReady(split.left.document.uri, {
          minCount: 2,
          minWarnings: 1,
          timeoutMs: 12_000,
          stableMs: 800,
        });
        await waitForDiagnosticsReady(split.right.document.uri, {
          minCount: 2,
          minWarnings: 1,
          timeoutMs: 12_000,
          stableMs: 800,
        });
        await vscode.window.showTextDocument(split.right.document, {
          viewColumn: vscode.ViewColumn.Two,
          preserveFocus: false,
          preview: false,
        });
        const beforeFixes = split.right.document.getText();
        const applied = await applyAllSafeFixes(split.right);
        if (applied === 0) {
          console.log("[screenshot-demo] style warnings scene: no safe fixes applied");
        }
        const fixesChanged = await waitForDocumentTextChange(
          split.right.document,
          beforeFixes,
          8_000,
        );
        if (!fixesChanged) {
          console.log(
            "[screenshot-demo] style warnings scene: right pane text unchanged after fixes",
          );
        }
        await waitForSemanticTokens(split.right.document.uri, 30, 10_000);
        setCursorFromSample(split.left, sample);
        if (!sample.cursor) {
          setCursor(split.left, 4, 4);
        }
        // Keep right cursor near transformed style warnings after fixes.
        setCursorOnText(split.right, "if {");
        await vscode.window.showTextDocument(split.right.document, {
          viewColumn: vscode.ViewColumn.Two,
          preserveFocus: false,
          preview: false,
        });
        keepCursorVisible(split.right);
        await sleep(900);
      },
    },
    {
      name: "09-semantic-highlighting",
      run: async () => {
        await closePanel();
        const editor = await openExampleFile("samples/for_screenshots/03-completions.tcl");
        await sleep(500);
        setCursor(editor, 0, 0);
        editor.revealRange(new vscode.Range(0, 0, 14, 0), vscode.TextEditorRevealType.AtTop);
      },
    },
    {
      name: "10-compiler-explorer",
      run: async () => {
        const editor = await openExampleFile("samples/for_screenshots/22-optimiser-before.tcl");
        await waitForSemanticTokens(editor.document.uri, 60, 10_000);
        setCursor(editor, 6, 10);
        await openCompilerExplorerForActiveEditor(12_000);
        keepCursorVisible(editor);
        await sleep(350);
      },
    },

    // -- Compiler explorer tab variants --
    {
      name: "11-compiler-cfg",
      keepEditors: true,
      run: async () => {
        const editor = vscode.window.activeTextEditor;
        await openCompilerExplorerForActiveEditor(12_000);
        switchCompilerExplorerTab("cfg-pre");
        if (editor) {
          keepCursorVisible(editor);
        }
        await sleep(700);
      },
    },
    {
      name: "12-compiler-ssa",
      keepEditors: true,
      run: async () => {
        const editor = vscode.window.activeTextEditor;
        await openCompilerExplorerForActiveEditor(12_000);
        switchCompilerExplorerTab("cfg-post");
        if (editor) {
          keepCursorVisible(editor);
        }
        await sleep(700);
      },
    },
    {
      name: "13-compiler-optimiser",
      keepEditors: true,
      run: async () => {
        const editor = vscode.window.activeTextEditor;
        await openCompilerExplorerForActiveEditor(12_000);
        switchCompilerExplorerTab("opt");
        if (editor) {
          keepCursorVisible(editor);
        }
        await sleep(700);
      },
    },
    {
      name: "14-compiler-irule",
      run: async () => {
        // Show the compiler explorer with an iRule instead of plain Tcl.
        const editor = await openExampleFile("samples/for_screenshots/ai-scene.irul");
        await waitForSemanticTokens(editor.document.uri, 80, 12_000);
        setCursor(editor, 2, 8);
        await openCompilerExplorerForActiveEditor(12_000);
        switchCompilerExplorerTab("ir");
        keepCursorVisible(editor);
        await sleep(900);
      },
    },

    // -- Additional LSP features --
    {
      name: "15-definition",
      run: async () => {
        await closePanel();
        const sample = loadScreenshotSample("15-definition-long.tcl");
        const editor = await openScratchEditor("definition-long-scene.tcl", sample.content);
        await waitForSemanticTokens(editor.document.uri, 40, 10_000);
        setCursorFromSample(editor, sample);
        if (!sample.cursor) {
          await setCursorOnText(editor, "heavy_add $lhs $rhs");
        }
        await vscode.commands.executeCommand("editor.action.peekDefinition");
        keepCursorVisible(editor);
        await sleep(1_250);
      },
    },
    {
      name: "16-references",
      run: async () => {
        await closePanel();
        const sample = loadScreenshotSample("16-references-long.tcl");
        const editor = await openScratchEditor("references-long-scene.tcl", sample.content);
        await waitForSemanticTokens(editor.document.uri, 40, 10_000);
        setCursorFromSample(editor, sample);
        if (!sample.cursor) {
          await setCursorOnText(editor, "heavy_add $left $right");
        }
        await vscode.commands.executeCommand("editor.action.referenceSearch.trigger");
        keepCursorVisible(editor);
        await sleep(1_250);
      },
    },
    {
      name: "17-document-symbols",
      run: async () => {
        await closePanel();
        const editor = await openExampleFile("samples/for_screenshots/03-completions.tcl");
        await sleep(500);
        // Position on a symbol so the file looks natural.
        await setCursorOnSymbol(editor, "add");
        await vscode.commands.executeCommand("workbench.action.gotoSymbol");
        await sleep(600);
      },
    },
    {
      name: "18-rename",
      run: async () => {
        await closePanel();
        const baseFile = path.join(workspaceRoot(), "samples/for_screenshots/03-completions.tcl");
        const source = fs.readFileSync(baseFile, "utf8");
        const split = await openSplitScratchEditors(
          "rename-before-left.tcl",
          "rename-after-right.tcl",
          source,
        );
        await Promise.all([
          waitForSemanticTokens(split.left.document.uri, 60, 10_000),
          waitForSemanticTokens(split.right.document.uri, 60, 10_000),
        ]);
        await split.right.edit((editBuilder) => {
          const text = split.right.document.getText();
          const renamed = text.replace(/\badd\b/g, "sum_values");
          const fullRange = new vscode.Range(
            split.right.document.positionAt(0),
            split.right.document.positionAt(text.length),
          );
          editBuilder.replace(fullRange, renamed);
        });
        await Promise.all([
          waitForSemanticTokens(split.left.document.uri, 60, 10_000),
          waitForSemanticTokens(split.right.document.uri, 60, 10_000),
        ]);
        await sleep(350);
        setCursorOnText(split.left, "proc add");
        setCursorOnText(split.right, "proc sum_values");
        keepCursorVisible(split.right);
        await sleep(900);
      },
    },
    {
      name: "19-signature-help",
      run: async () => {
        await closePanel();
        const sample = loadScreenshotSample("19-signature-help.tcl");
        const editor = await openScratchEditor("signature-help-scene.tcl", sample.content);
        await waitForSemanticTokens(editor.document.uri, 30, 8_000);
        setCursorFromSample(editor, sample);
        if (!sample.cursor) {
          setCursor(editor, 6, 22);
        }
        await vscode.commands.executeCommand("editor.action.triggerParameterHints");
        keepCursorVisible(editor);
        await sleep(1_400);
      },
    },
    {
      name: "20-folding",
      run: async () => {
        await closePanel();
        const sample = loadScreenshotSample("20-folding.tcl");
        const split = await openSplitScratchEditors(
          "folding-before-left.tcl",
          "folding-after-right.tcl",
          sample.content,
        );
        await Promise.all([
          waitForSemanticTokens(split.left.document.uri, 20, 8_000),
          waitForSemanticTokens(split.right.document.uri, 20, 8_000),
        ]);
        await sleep(350);

        // Fold only the right pane so left remains the before state.
        await vscode.window.showTextDocument(split.right.document, {
          viewColumn: vscode.ViewColumn.Two,
          preserveFocus: false,
          preview: false,
        });
        await vscode.commands.executeCommand("editor.unfoldAll");
        await sleep(200);
        if (sample.cursor) {
          setCursor(split.right, sample.cursor.line, sample.cursor.col);
        } else {
          setCursor(split.right, 4, 0);
        }
        await vscode.commands.executeCommand("editor.fold");
        await sleep(450);
        setCursorFromSample(split.left, sample);
        setCursor(split.right, 4, 5);
        keepCursorVisible(split.right);
      },
    },
    {
      name: "21-inlay-hints",
      run: async () => {
        await closePanel();
        const sample = loadScreenshotSample("21-binary-hover.tcl");
        const editor = await openScratchEditor("binary-hover-scene.tcl", sample.content);
        await waitForSemanticTokens(editor.document.uri, 35, 10_000);
        editor.revealRange(new vscode.Range(0, 0, 5, 0), vscode.TextEditorRevealType.AtTop);
        await sleep(450);
        setCursorFromSample(editor, sample);
        if (!sample.cursor) {
          setCursorOnText(editor, "c3S4i@20f");
        }
        await vscode.commands.executeCommand("editor.action.showHover");
        keepCursorVisible(editor);
        await sleep(1_250);
      },
    },
    {
      name: "22-optimiser",
      run: async () => {
        await closePanel();
        const sample = loadScreenshotSample("22-optimiser-before.tcl");
        const split = await openSplitScratchEditors(
          "optimiser-before-left.tcl",
          "optimiser-after-right.tcl",
          sample.content,
        );

        split.left.options = { tabSize: 4, insertSpaces: true };
        split.right.options = { tabSize: 4, insertSpaces: true };

        await Promise.all([
          waitForSemanticTokens(split.left.document.uri, 60, 10_000),
          waitForSemanticTokens(split.right.document.uri, 60, 10_000),
        ]);
        await waitForDiagnosticsReady(split.left.document.uri, {
          minCount: 8,
          minWarnings: 6,
          stableMs: 900,
          timeoutMs: 14_000,
        });

        await vscode.window.showTextDocument(split.right.document, {
          viewColumn: vscode.ViewColumn.Two,
          preserveFocus: false,
          preview: false,
        });
        setCursorFromSample(split.left, sample);
        setCursorFromSample(split.right, sample);
        if (!sample.cursor) {
          setCursor(split.left, 10, 4);
          setCursor(split.right, 10, 4);
        }
        const optimisationResult = await applyAllOptimisations(split.right);
        if (optimisationResult.count === 0 || typeof optimisationResult.source !== "string") {
          console.log("[screenshot-demo] optimiser scene: no rewrites applied");
        } else {
          const commentedOriginal = sample.content
            .split("\n")
            .map((line) => `# ${line}`)
            .join("\n");
          const optimisationNotes = optimisationResult.optimisations
            .slice(0, 10)
            .map((opt, index) => {
              const lineLabel = opt.line != null ? `line ${opt.line}` : "line ?";
              const codeLabel = opt.code || "O???";
              const message = opt.message || "optimisation applied";
              return `# ${index + 1}. ${codeLabel} (${lineLabel}) - ${message}`;
            });
          if (optimisationNotes.length === 0) {
            optimisationNotes.push("# (No optimisation metadata returned by the server)");
          }

          const annotatedRight = [
            "# Optimisation walkthrough (after applying all safe rewrites)",
            "# Source references: tcl-lsp.optimiseDocument, compiler explorer callouts",
            "",
            "# Original input (commented):",
            commentedOriginal,
            "",
            "# Applied optimisation notes:",
            ...optimisationNotes,
            "",
            "# Optimised output:",
            optimisationResult.source,
            "",
          ].join("\n");

          await split.right.edit((editBuilder) => {
            const fullRange = new vscode.Range(
              split.right.document.positionAt(0),
              split.right.document.positionAt(split.right.document.getText().length),
            );
            editBuilder.replace(fullRange, annotatedRight);
          });
          setCursorOnText(split.right, "# Optimised output:");
          const outputLine = split.right.selection.active.line + 2;
          setCursor(split.right, outputLine, 4);
        }
        await waitForSemanticTokens(split.right.document.uri, 40, 10_000);
        setCursorFromSample(split.left, sample);
        if (!sample.cursor) {
          setCursor(split.left, 10, 4);
        }
        keepCursorVisible(split.right);
        await sleep(1_250);
      },
    },
    {
      name: "23-irule-skeleton",
      run: async () => {
        await closePanel();
        const seedEvents = ["RULE_INIT", "HTTP_REQUEST", "HTTP_RESPONSE", "CLIENT_CLOSED"];
        const seededSkeleton = buildIruleEventSkeleton(seedEvents);
        const editor = await openScratchEditor("irule-skeleton-single.irul", seededSkeleton);
        await waitForSemanticTokens(editor.document.uri, 60, 10_000);
        setCursorOnText(editor, "when HTTP_REQUEST");
        // Reopen the picker so the pre-selected events are visible.
        void vscode.commands.executeCommand("tclLsp.insertIruleEventSkeleton");
        keepCursorVisible(editor);
        await sleep(900);
      },
    },
    {
      name: "24-template-snippets",
      run: async () => {
        await closePanel();
        const extension = vscode.extensions.getExtension("tcl-lsp.tcl-lsp");
        const snippets = extension ? loadTemplateSnippets(extension.extensionPath) : [];
        const seedSnippet =
          snippets.find((snippet) => snippet.name === "Catch with Result") ?? snippets[0];
        const fallbackSample = loadScreenshotSample("24-template-fallback.irul");
        const seedContent = seedSnippet
          ? renderTemplateSnippet(seedSnippet)
          : fallbackSample.content;
        const editor = await openScratchEditor("template-single.irul", seedContent);
        await waitForSemanticTokens(editor.document.uri, 40, 10_000);
        if (seedSnippet) {
          setCursorOnText(editor, "catch");
        } else {
          setCursorFromSample(editor, fallbackSample);
        }
        await sleep(350);
        // Reopen the snippet picker with a realistic seeded buffer.
        void vscode.commands.executeCommand("tclLsp.insertTemplateSnippet");
        keepCursorVisible(editor);
        await sleep(900);
      },
    },
    {
      name: "25-dialect-selection",
      run: async () => {
        await closePanel();
        const target = vscode.workspace.workspaceFolders?.length
          ? undefined
          : vscode.ConfigurationTarget.Global;
        await vscode.workspace.getConfiguration("tclLsp").update("dialect", "f5-irules", target);
        await sleep(250);
        const editor = await openExampleFile(
          "samples/for_screenshots/05-security-taint-before.irul",
        );
        await waitForSemanticTokens(editor.document.uri, 80, 10_000);
        setCursor(editor, 0, 5);
        await sleep(350);
        // Fire without await — the QuickPick must stay open for capture.
        void vscode.commands.executeCommand("tclLsp.selectDialect");
        keepCursorVisible(editor);
        await sleep(900);
      },
    },

    // -- AI Chat features --
    // These require Copilot Chat to be installed and authenticated.
    // When Copilot is unavailable the scenes throw so the capture loop
    // skips them rather than producing useless editor-only screenshots.
    {
      name: "26-ai-create",
      requiresAi: true,
      run: async () => {
        requireCopilot();
        closeCompilerExplorer();
        const editor = await openAiSceneEditor("ai-create-scene.irul");
        await setCursorOnText(editor, 'if {[HTTP::uri] eq "/admin"}');
        await runAiChatQuery(
          "@irule /create Create a valid iRule with one HTTP_REQUEST branch: redirect /old to /new and log all other requests. Return only the iRule code in a tcl code block.",
          140_000,
          { settleDelayMs: 7_000 },
        );
      },
    },
    {
      name: "27-ai-explain",
      requiresAi: true,
      run: async () => {
        requireCopilot();
        const editor = await openAiSceneEditor("ai-explain-scene.irul");
        await setCursorOnText(editor, 'if {[HTTP::uri] eq "/admin"}');
        await runAiChatQuery(
          "@irule /explain Explain this iRule's single branch, return behaviour, and security impact.",
          140_000,
          { settleDelayMs: 8_000 },
        );
      },
    },
    {
      name: "28-ai-diagram",
      requiresAi: true,
      run: async () => {
        requireCopilot();
        const editor = await openAiSceneEditor("ai-diagram-scene.irul");
        await setCursorOnText(editor, "when HTTP_REQUEST");
        await runAiChatQuery(
          "@irule /diagram Show a Mermaid flowchart for this iRule and label both branch outcomes.",
          140_000,
          { settleDelayMs: 8_000 },
        );
      },
    },
    {
      name: "29-ai-validate",
      requiresAi: true,
      run: async () => {
        requireCopilot();
        const editor = await openAiSceneEditor("ai-validate-scene.irul");
        await setCursorOnText(editor, "when HTTP_REQUEST");
        await runAiChatQuery("@irule /validate", 120_000, { settleDelayMs: 5_000 });
      },
    },
    {
      name: "30-ai-review",
      requiresAi: true,
      run: async () => {
        requireCopilot();
        const editor = await openAiSceneEditor("ai-review-scene.irul");
        await setCursorOnText(editor, 'if {[HTTP::uri] eq "/admin"}');
        await runAiChatQuery(
          "@irule /review Review this iRule for auth bypass, header trust, and redirect risks. Give 3 actionable fixes with code examples.",
          140_000,
          { settleDelayMs: 7_000 },
        );
      },
    },
    {
      name: "31-ai-help",
      requiresAi: true,
      run: async () => {
        requireCopilot();
        const editor = await openAiSceneEditor("ai-help-scene.irul");
        await setCursorOnText(editor, "when HTTP_REQUEST");
        await runAiChatQuery("@irule /help optimise", 120_000, {
          settleDelayMs: 5_500,
        });
      },
    },
  ];
}

// Main entry point

/**
 * Verify AI chat is genuinely ready before running AI scenes.
 * We only proceed when a real request starts and completes.
 */
async function prepareAiScenes(): Promise<boolean> {
  let copilot = getCopilotChatExtension();
  if (!copilot) {
    console.log(
      "[screenshot-demo] Copilot Chat not detected yet — showing login prompt anyway for manual sign-in",
    );
  } else if (!copilot.isActive) {
    try {
      await copilot.activate();
      await sleep(250);
    } catch (err) {
      console.error("[screenshot-demo] Copilot Chat activation failed:", err);
    }
  }

  for (let attempt = 0; attempt < 3; attempt++) {
    // Re-check availability each attempt in case extension activation/login
    // finished after startup.
    if (!copilot) {
      copilot = await waitForCopilotChatExtension(500);
      if (copilot && !copilot.isActive) {
        try {
          await copilot.activate();
          await sleep(250);
        } catch (err) {
          console.error("[screenshot-demo] Copilot Chat activation failed:", err);
        }
      }
    }
    const message = !copilot
      ? "Copilot Chat is not detected yet. Open Sign-In (or install/enable Copilot Chat), then click 'Verify AI Login'."
      : attempt === 0
        ? "Sign in to GitHub Copilot and choose Claude Opus, then click 'Verify AI Login'."
        : "AI login check failed. Confirm GitHub Copilot is signed in, then click 'Retry AI Check'.";
    const verifyLabel = attempt === 0 ? "Verify AI Login" : "Retry AI Check";
    console.log(
      `[screenshot-demo] Showing AI login prompt (attempt ${attempt + 1}/3, copilotDetected=${Boolean(copilot)})`,
    );
    const choice = await vscode.window.showInformationMessage(
      message,
      verifyLabel,
      "Open Sign-In",
      "Skip AI Screenshots",
    );
    if (!choice) {
      console.log("[screenshot-demo] AI login toast dismissed — prompting again");
      await sleep(400);
      continue;
    }
    if (choice === "Open Sign-In") {
      const started = await openAnyAvailableSignInFlow();
      if (!started) {
        console.error("[screenshot-demo] No sign-in command was available");
      }
      // Allow user time to complete auth in browser or accounts UI.
      await sleep(1_200);
      attempt -= 1;
      continue;
    }
    if (choice !== verifyLabel) {
      console.log("[screenshot-demo] AI scenes skipped by user");
      return false;
    }

    // Check auth and models, but don't block on failure — the real test
    // is whether a chat query actually works.
    const [authReady, modelsReady] = await Promise.all([
      hasGitHubAuthSession(),
      hasChatModelsAvailable(),
    ]);
    console.log(`[screenshot-demo] Auth check: authReady=${authReady}, modelsReady=${modelsReady}`);
    if (!authReady && !modelsReady) {
      // Neither auth nor models detected — no point trying the query.
      console.error("[screenshot-demo] AI login check failed: no auth and no models");
      const retryChoice = await vscode.window.showWarningMessage(
        "AI sign-in is not ready yet. Use 'Open Sign-In' and complete GitHub/Copilot auth, then retry.",
        "Retry AI Check",
        "Open Sign-In",
        "Skip AI Screenshots",
      );
      if (retryChoice === "Open Sign-In") {
        const started = await openAnyAvailableSignInFlow();
        if (!started) {
          console.error("[screenshot-demo] No sign-in command was available");
        }
        await sleep(1_200);
        attempt -= 1;
        continue;
      }
      if (retryChoice === "Skip AI Screenshots") {
        console.log("[screenshot-demo] AI scenes skipped by user");
        return false;
      }
      continue;
    }

    // Even if one check fails, attempt the verification query — Copilot
    // may work through its own auth flow independent of the VS Code
    // authentication API.
    try {
      await runAiChatQuery("@irule /event HTTP_REQUEST", 60_000, {
        allowSubmitFallback: true,
      });
      try {
        await vscode.commands.executeCommand("workbench.action.chat.new");
      } catch {}
      console.log("[screenshot-demo] AI scenes enabled");
      return true;
    } catch (err) {
      console.error("[screenshot-demo] AI readiness check failed:", err);
    }
  }

  console.log("[screenshot-demo] AI scenes disabled after repeated readiness failures");
  return false;
}

/** Clean up VS Code UI for pristine screenshots. */
async function cleanUpUI(): Promise<void> {
  const cmds = [
    "workbench.action.closeAuxiliaryBar", // Copilot / AI chat panel
    "workbench.action.closeSidebar", // Explorer sidebar
    // NOTE: do NOT close the bottom panel here — once closed, VS Code
    // persists a zero-height layout and no command can reopen it reliably
    // in the test-electron environment.  Scenes that need a panel-free
    // editor close it themselves.
    "notifications.clearAll", // Dismiss toasts
    "notifications.hideToasts", // Hide remaining toasts
  ];
  for (const cmd of cmds) {
    try {
      await vscode.commands.executeCommand(cmd);
    } catch {}
  }
  await sleep(200);
}

/**
 * Dismiss any QuickPick, input box, or context menu left open by a scene.
 *
 * Sending Escape via `closeFocusedStickyWidget` closes QuickPick /
 * InputBox overlays.  `focusActiveEditorGroup` pulls focus back to the
 * editor so the next scene can manipulate it.
 */
async function dismissOverlays(): Promise<void> {
  for (const cmd of [
    "workbench.action.closeFocusedStickyWidget",
    "workbench.action.closeQuickOpen",
    "workbench.action.focusActiveEditorGroup",
  ]) {
    try {
      await vscode.commands.executeCommand(cmd);
    } catch {}
  }
  await sleep(100);
}

/** Close the bottom panel for scenes that want a full-height editor. */
async function closePanel(): Promise<void> {
  try {
    await vscode.commands.executeCommand("workbench.action.closePanel");
  } catch {}
  await sleep(100);
}

async function waitForShellReady(): Promise<void> {
  const marker = path.join(OUTPUT_DIR, ".shell-ready");
  console.log("[screenshot-demo] Waiting for shell to signal readiness...");
  const deadline = Date.now() + 60_000; // 60s — shell needs time to find the window
  while (Date.now() < deadline) {
    if (fs.existsSync(marker)) {
      try {
        fs.unlinkSync(marker);
      } catch {}
      console.log("[screenshot-demo] Shell is ready");
      return;
    }
    await sleep(100);
  }
  console.error("[screenshot-demo] Shell never signalled ready — proceeding anyway");
}

async function runDemo(): Promise<void> {
  let scenes = buildScenes();

  const sceneFilterRaw = process.env.TCL_LSP_SCREENSHOT_SCENES;
  if (sceneFilterRaw) {
    const wanted = new Set(
      sceneFilterRaw
        .split(",")
        .map((value) => value.trim())
        .filter((value) => value.length > 0),
    );
    scenes = scenes.filter((scene) => wanted.has(scene.name));
    console.log(`[screenshot-demo] Scene filter enabled: ${Array.from(wanted).join(", ")}`);
  }

  console.log(`[screenshot-demo] Starting ${scenes.length} scenes`);
  console.log(`[screenshot-demo] Output: ${OUTPUT_DIR}`);
  writeProcessMarkers();
  await configureScreenshotCursor();

  const hasAiScenes = scenes.some((scene) => scene.requiresAi);
  // Ensure AI is genuinely ready before attempting AI scenes.
  aiScenesEnabled = hasAiScenes ? await prepareAiScenes() : false;

  // Clean up UI chrome before first capture.
  await cleanUpUI();

  // Wait for the shell script to be ready to capture before running scenes.
  await waitForShellReady();

  for (const scene of scenes) {
    if (scene.requiresAi && !aiScenesEnabled) {
      console.log(`[screenshot-demo] Skipping ${scene.name} (AI unavailable)`);
      continue;
    }
    console.log(`[screenshot-demo] Scene: ${scene.name}`);
    // Close all editors and the compiler explorer so only the current
    // file is visible — the compiler explorer reads from the active
    // editor, so stale tabs would leave it empty.
    // Continuation scenes (e.g. formatting-after) set keepEditors to
    // preserve the previous scene's editor state.
    if (!scene.keepEditors) {
      closeCompilerExplorer();
      await closeAllEditors();
      // Reset UI panels so each scene starts clean — individual scenes
      // re-open whatever they need (Problems, chat, compiler explorer).
      await cleanUpUI();
    }
    try {
      await scene.run();
    } catch (err) {
      console.error(`[screenshot-demo] Scene ${scene.name} failed:`, err);
      continue;
    }
    ensureCursorInVisibleEditor();
    await sleep(300); // let the UI settle before capture
    await captureScreenshot(scene.name);
    // Dismiss any open QuickPick, input box, or menu that the scene
    // left open — otherwise the next scene's closeAllEditors /
    // cleanUpUI cannot take focus and the sequence stalls.
    await dismissOverlays();
  }

  // Write a done marker so the outer script knows we finished.
  fs.writeFileSync(path.join(OUTPUT_DIR, ".done"), "", "utf8");
  console.log("[screenshot-demo] All scenes complete");
}

export function registerScreenshotDemo(context: vscode.ExtensionContext): void {
  context.subscriptions.push(
    vscode.commands.registerCommand("tclLsp.runScreenshotDemo", () => runDemo()),
  );
}
