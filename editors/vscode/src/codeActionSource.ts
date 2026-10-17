// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

import * as vscode from "vscode";
import type { Middleware } from "vscode-languageclient";
import {
  DocumentSourceSnapshot,
  snapshotDocumentSource,
  isDocumentSourceSnapshotCurrent,
  replaceDocumentFromSnapshot,
} from "./chat/codeUtils";
import { isAnalysisContextCurrent, resourceConfiguration } from "./chat/diagnosticAccess";

interface SourceActionReceipt {
  readonly snapshot: DocumentSourceSnapshot;
  readonly configuration: string;
  readonly expectedSource: string;
  readonly rename: readonly [number, number, number] | undefined;
  readonly listener: vscode.Disposable;
  readonly generation: number;
  invalidated: boolean;
}
interface RenameIntent {
  readonly snapshot: DocumentSourceSnapshot;
  readonly configuration: string;
  readonly position: readonly [number, number, number];
  readonly current: () => boolean;
}

const pending = new Map<string, SourceActionReceipt>();
const renames = new Map<string, RenameIntent>();
const MAX_PENDING = 128;
let commandGeneration = 0;
let commandsActive = false;

function removeReceipt(id: string): SourceActionReceipt | undefined {
  const receipt = pending.get(id);
  pending.delete(id);
  receipt?.listener.dispose();
  return receipt;
}

function current(snapshot: DocumentSourceSnapshot, configuration: string): boolean {
  return (
    isDocumentSourceSnapshotCurrent(snapshot) &&
    isAnalysisContextCurrent(snapshot.analysisContext) &&
    resourceConfiguration(snapshot.document) === configuration
  );
}

function strictOffset(
  document: vscode.TextDocument,
  position: vscode.Position,
): number | undefined {
  const offset = document.offsetAt(position);
  if (!document.positionAt(offset).isEqual(position)) return undefined;
  const unit = document.getText().charCodeAt(offset);
  if (offset > 0 && unit >= 0xdc00 && unit <= 0xdfff) return undefined;
  return offset;
}

/** Render the actual single-document edit plan, without interpreting Tcl. */
function editedSource(
  snapshot: DocumentSourceSnapshot,
  action: vscode.CodeAction,
): string | undefined {
  const entries = action.edit?.entries() ?? [];
  if (entries.some(([uri]) => uri.toString() !== snapshot.uri.toString())) return undefined;
  const edits: { start: number; end: number; text: string }[] = [];
  for (const [, documentEdits] of entries) {
    for (const edit of documentEdits) {
      const start = strictOffset(snapshot.document, edit.range.start);
      const end = strictOffset(snapshot.document, edit.range.end);
      if (start === undefined || end === undefined || start > end) return undefined;
      edits.push({ start, end, text: edit.newText });
    }
  }
  // Stable source order retains the supplied LSP order of coincident
  // insertions, including insertions before a replacement at that point.
  edits.sort((a, b) => a.start - b.start);
  let offset = 0;
  let result = "";
  for (const edit of edits) {
    if (edit.start < offset) return undefined;
    result += snapshot.fullSource.slice(offset, edit.start) + edit.text;
    offset = edit.end;
  }
  return result + snapshot.fullSource.slice(offset);
}

function object(value: unknown): Record<string, unknown> | undefined {
  return value !== null && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : undefined;
}

/** The converter drops TextDocumentEdit.version; retain the actual provider
 * document and route ordinary edits through the shared checked editor sink. */
export const provideSourceCodeActions: NonNullable<Middleware["provideCodeActions"]> = async (
  document,
  range,
  context,
  token,
  next,
) => {
  const generation = commandGeneration;
  const snapshot = snapshotDocumentSource(document);
  const configuration = resourceConfiguration(document);
  let configurationChanged = false;
  const observation = vscode.workspace.onDidChangeConfiguration((event) => {
    if (event.affectsConfiguration("tclLsp", document.uri)) configurationChanged = true;
  });
  let actions: Awaited<ReturnType<typeof next>>;
  try {
    actions = await next(document, range, context, token);
  } finally {
    observation.dispose();
  }
  if (token.isCancellationRequested || !actions) return undefined;
  const sourceCurrent =
    commandsActive &&
    commandGeneration === generation &&
    !configurationChanged &&
    configuration !== undefined &&
    current(snapshot, configuration);
  return actions.filter((action) => {
    if (!(action instanceof vscode.CodeAction)) return false;
    const intent = object(
      object((action as vscode.CodeAction & { data?: unknown }).data)?.sourceAction,
    );
    // This is the server's explicit existing dialect adapter, never an
    // inference from absent metadata or a document/URI spelling.
    if (intent?.kind === "legacy-bigip") return true;
    if (
      !sourceCurrent ||
      configuration === undefined ||
      intent?.kind !== "versioned" ||
      intent.uri !== snapshot.uri.toString() ||
      intent.version !== snapshot.version ||
      intent.source !== snapshot.fullSource
    )
      return false;
    const expectedSource = editedSource(snapshot, action);
    if (expectedSource === undefined) return false;
    let rename: readonly [number, number, number] | undefined;
    if (action.command) {
      const args: unknown[] | undefined = action.command.arguments;
      if (
        action.command.command !== "tclLsp.renameSymbolAtPosition" ||
        !args ||
        args.length !== 3 ||
        !args.every(
          (value) => typeof value === "number" && Number.isSafeInteger(value) && value >= 0,
        )
      )
        return false;
      rename = args as [number, number, number];
    }
    let id: string;
    try {
      id = globalThis.crypto.randomUUID();
    } catch {
      return false;
    }
    if (pending.has(id) || renames.has(id)) return false;
    const listener = vscode.workspace.onDidChangeConfiguration((event) => {
      if (event.affectsConfiguration("tclLsp", document.uri)) {
        receipt.invalidated = true;
      }
    });
    const receipt: SourceActionReceipt = {
      snapshot,
      configuration,
      expectedSource,
      rename,
      listener,
      generation,
      invalidated: false,
    };
    pending.set(id, receipt);
    while (pending.size > MAX_PENDING) removeReceipt(pending.keys().next().value!);
    // The VS Code API cannot retain the server's edit version. It must not
    // also apply this version-less edit before our checked command runs.
    action.edit = undefined;
    action.command = {
      title: action.title,
      command: "tclLsp.applySourceCodeAction",
      arguments: [id],
    };
    return true;
  });
};

/** Both hosts use the same original-document writer and rename continuation. */
export function registerSourceActionCommands(): vscode.Disposable {
  const generation = ++commandGeneration;
  commandsActive = true;
  const apply = vscode.commands.registerCommand(
    "tclLsp.applySourceCodeAction",
    async (id: unknown) => {
      if (typeof id !== "string") return false;
      const receipt = pending.get(id);
      if (!receipt) return false;
      // Remove the one-use handle immediately, but retain its settings observer
      // through every await and the shared sink's final admission callback.
      pending.delete(id);
      try {
        const live = () =>
          commandsActive &&
          commandGeneration === generation &&
          receipt.generation === generation &&
          !receipt.invalidated;
        const admit = () => live() && current(receipt.snapshot, receipt.configuration);
        if (!admit()) return false;
        const applied = await replaceDocumentFromSnapshot(
          receipt.snapshot,
          receipt.expectedSource,
          async () => admit(),
        );
        if (!applied || !live() || !current(applied, receipt.configuration)) return false;
        if (receipt.rename) {
          renames.set(id, {
            snapshot: applied,
            configuration: receipt.configuration,
            position: receipt.rename,
            current: live,
          });
          return await vscode.commands.executeCommand<boolean>("tclLsp.renameSymbolAtPosition", id);
        }
        return true;
      } finally {
        receipt.listener.dispose();
        renames.delete(id);
      }
    },
  );
  const rename = vscode.commands.registerCommand(
    "tclLsp.renameSymbolAtPosition",
    async (id: unknown) => {
      if (typeof id !== "string") return false;
      const intent = renames.get(id);
      renames.delete(id);
      if (!intent || !intent.current() || !current(intent.snapshot, intent.configuration))
        return false;
      const editor = await vscode.window.showTextDocument(intent.snapshot.document);
      if (
        !intent.current() ||
        !current(intent.snapshot, intent.configuration) ||
        editor.document !== intent.snapshot.document
      )
        return false;
      const [line, start, end] = intent.position;
      const from = new vscode.Position(line, start);
      const to = new vscode.Position(line, end);
      if (
        strictOffset(editor.document, from) === undefined ||
        strictOffset(editor.document, to) === undefined ||
        start > end
      )
        return false;
      editor.selection = new vscode.Selection(from, to);
      editor.revealRange(new vscode.Range(from, to));
      await vscode.commands.executeCommand("editor.action.rename");
      return true;
    },
  );
  return vscode.Disposable.from(
    apply,
    rename,
    new vscode.Disposable(() => {
      if (commandGeneration === generation) {
        commandsActive = false;
        commandGeneration++;
      }
      for (const id of pending.keys()) removeReceipt(id);
      renames.clear();
    }),
  );
}
