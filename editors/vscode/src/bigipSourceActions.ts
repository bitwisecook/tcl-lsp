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
 * The BIG-IP and XC source actions both entry points share.
 *
 * Platform-neutral like `./clientCore`: it imports `vscode` only, and reaches
 * the server through a command sender each entry supplies, so the node and
 * browser hosts run the same request shapes. Every request names the
 * document version it was read from; the server answers against that version
 * or refuses, and does any byte arithmetic itself.
 */

import * as vscode from "vscode";
import type { ExtensionContext, Position, TextDocument, TextEditor } from "vscode";
import { LspWorkspaceEdit, OpenDocumentVersion, workspaceEditFromLsp } from "./clientCore";

/** Run one `workspace/executeCommand`; rejects when the server cannot answer. */
export type ServerCommandSender = (command: string, args: unknown[]) => Promise<unknown>;

/** A cursor as the server reads it: an LSP position in a named document version. */
export interface SourcePositionArgument {
  position: { line: number; character: number };
  version: number;
}

/** The typed position argument for `position` in `document` as it stands. */
export function sourcePositionArgument(
  document: TextDocument,
  position: Position,
): SourcePositionArgument {
  return {
    position: { line: position.line, character: position.character },
    version: document.version,
  };
}

/** The reason a server request was refused, when it was. */
export function unavailableReason(value: unknown): string | undefined {
  if (typeof value !== "object" || value === null || !("unavailable" in value)) {
    return undefined;
  }
  const reason = (value as { unavailable: unknown }).unavailable;
  return typeof reason === "string" ? reason : "unknown";
}

// -- Embedded rule scratch editors ------------------------------------------

/**
 * One `ltm rule` / `gtm rule` stanza as the server reported it. The whole
 * object is the receipt `tcl-lsp.writeRuleBack` rejoins the stanza with.
 */
export interface RuleInfo {
  name: string;
  fullPath: string;
  header: string;
  body: string;
  bodyStartOffset: number;
  bodyEndOffset: number;
  uri: string;
  documentVersion: number | null;
  blockStartLine?: number;
}

export function isRuleInfo(value: unknown): value is RuleInfo {
  if (typeof value !== "object" || value === null) return false;
  const rule = value as Partial<RuleInfo>;
  return (
    typeof rule.name === "string" &&
    typeof rule.fullPath === "string" &&
    typeof rule.header === "string" &&
    typeof rule.body === "string" &&
    typeof rule.uri === "string" &&
    typeof rule.bodyStartOffset === "number" &&
    typeof rule.bodyEndOffset === "number"
  );
}

/** The result of writing a scratch body back to its configuration file. */
export type RuleWriteBackOutcome =
  { kind: "unchanged" } | { kind: "written"; rule: RuleInfo } | { kind: "refused"; reason: string };

const WRITE_BACK_MESSAGES: Record<string, string> = {
  stale:
    "The rule changed in the configuration file since it was extracted. Extract it again to edit the current body.",
  body: "The edited body does not parse back as the same rule (check its braces); the configuration file was not changed.",
  rejected: "The configuration file changed before the edit could be applied. Save again.",
  notOpen: "The configuration file this rule came from is no longer open.",
  noServer: "The Tcl language server is not running.",
};

/** The message a refused write-back shows. */
export function ruleWriteBackMessage(reason: string): string {
  return WRITE_BACK_MESSAGES[reason] ?? "Failed to write iRule back to configuration file.";
}

interface ScratchEntry {
  receipt: RuleInfo;
  pending: Promise<RuleWriteBackOutcome>;
}

/**
 * The scratch editors opened from configuration rules, keyed by the scratch
 * document's URI. Saves of one scratch document are serialised, so each
 * write sends the receipt the previous one returned.
 */
export class RuleScratchRegistry {
  private readonly entries = new Map<string, ScratchEntry>();

  constructor(private readonly sender: () => ServerCommandSender | undefined) {}

  /** Start tracking `scratchUri` as an editor of `rule`. */
  track(scratchUri: string, rule: RuleInfo): void {
    this.entries.set(scratchUri, {
      receipt: rule,
      pending: Promise.resolve({ kind: "unchanged" }),
    });
  }

  /** Stop tracking a scratch editor that closed. */
  forget(scratchUri: string): void {
    this.entries.delete(scratchUri);
  }

  isTracked(scratchUri: string): boolean {
    return this.entries.has(scratchUri);
  }

  /** The receipt the next write of `scratchUri` sends. */
  receipt(scratchUri: string): RuleInfo | undefined {
    return this.entries.get(scratchUri)?.receipt;
  }

  /** Open `rule`'s body in a new scratch editor and track it. */
  async open(rule: RuleInfo): Promise<void> {
    const doc = await vscode.workspace.openTextDocument({
      language: "tcl-irule",
      content: rule.body,
    });
    await vscode.window.showTextDocument(doc, { preview: false });
    this.track(doc.uri.toString(), rule);
    vscode.window.showInformationMessage(
      `Editing iRule '${rule.name}' — save to write back to config.`,
    );
  }

  /**
   * Write `newBody` back through the server. `undefined` when `scratchUri`
   * is not a tracked scratch editor.
   */
  writeBack(scratchUri: string, newBody: string): Promise<RuleWriteBackOutcome> | undefined {
    const entry = this.entries.get(scratchUri);
    if (!entry) return undefined;
    const next = entry.pending
      .then(() => this.send(entry, newBody))
      .catch((): RuleWriteBackOutcome => ({ kind: "refused", reason: "rejected" }));
    entry.pending = next;
    return next;
  }

  private async send(entry: ScratchEntry, newBody: string): Promise<RuleWriteBackOutcome> {
    if (newBody === entry.receipt.body) return { kind: "unchanged" };
    const send = this.sender();
    if (!send) return { kind: "refused", reason: "noServer" };
    const answer = await send("tcl-lsp.writeRuleBack", [entry.receipt, newBody]);
    if (typeof answer !== "object" || answer === null) {
      return { kind: "refused", reason: "rejected" };
    }
    const result = answer as { applied?: unknown; rule?: unknown; reason?: unknown };
    if (result.applied === true && isRuleInfo(result.rule)) {
      entry.receipt = result.rule;
      return { kind: "written", rule: result.rule };
    }
    return {
      kind: "refused",
      reason: typeof result.reason === "string" ? result.reason : "rejected",
    };
  }

  /** Write scratch saves back and forget closed scratch editors. */
  register(context: ExtensionContext): void {
    context.subscriptions.push(
      vscode.workspace.onDidSaveTextDocument(async (doc) => {
        const outcome = this.writeBack(doc.uri.toString(), doc.getText());
        if (!outcome) return;
        const result = await outcome;
        if (result.kind === "written") {
          vscode.window.showInformationMessage("iRule written back to configuration file.");
        } else if (result.kind === "refused") {
          vscode.window.showWarningMessage(ruleWriteBackMessage(result.reason));
        }
      }),
      vscode.workspace.onDidCloseTextDocument((doc) => {
        this.forget(doc.uri.toString());
      }),
    );
  }
}

/** Ask for the rule under the editor's cursor; a message when there is none. */
export async function extractRuleAtCursor(
  send: ServerCommandSender,
  editor: TextEditor,
): Promise<RuleInfo | string> {
  const answer = await send("tcl-lsp.extractRule", [
    editor.document.uri.toString(),
    sourcePositionArgument(editor.document, editor.selection.active),
  ]);
  if (unavailableReason(answer) === "stale") {
    return "The configuration file changed while the rule was being read. Try again.";
  }
  if (!isRuleInfo(answer)) {
    return "Cursor is not inside an ltm rule or gtm rule block.";
  }
  return answer;
}

// -- Linked objects ---------------------------------------------------------

/** Ask for the objects linked from every cursor in `editor`. */
export async function requestLinkedObjects(
  send: ServerCommandSender,
  editor: TextEditor,
  maxDepth = 5,
  maxNodes = 400,
): Promise<unknown> {
  const document = editor.document;
  const uri = document.uri.toString();
  const [primary, ...rest] = editor.selections.map((selection) =>
    sourcePositionArgument(document, selection.active),
  );
  const extra = rest.map((cursor): [string, SourcePositionArgument] => [uri, cursor]);
  return send("tcl-lsp.extractLinkedObjects", [
    uri,
    primary,
    maxDepth,
    maxNodes,
    extra.length > 0 ? extra : null,
  ]);
}

// -- Partition rename -------------------------------------------------------

export interface PartitionRenameResult {
  success: boolean;
  edit?: LspWorkspaceEdit;
  error?: string;
}

/**
 * The document version a partition rename applies to. A code action offered
 * on a stanza carries the version that stanza was read from (its third
 * argument), and that is the identity the rename keeps: the server refuses a
 * document that has moved on since. Only a command run without one (the
 * palette) takes the document's version at invocation.
 */
export function partitionRenameVersion(
  offered: unknown,
  current: number | undefined,
): number | undefined {
  return typeof offered === "number" && Number.isInteger(offered) && offered >= 0
    ? offered
    : current;
}

/**
 * Ask the server to rename a partition in the document at `version`, and
 * convert the edit only while that document is still at that version.
 */
export async function requestPartitionRename(
  send: ServerCommandSender,
  uri: string,
  currentName: string,
  newName: string,
  version: number | undefined,
  versionOf?: OpenDocumentVersion,
): Promise<vscode.WorkspaceEdit | string> {
  const result = (await send("tcl-lsp.renamePartition", [
    uri,
    currentName,
    newName,
    version ?? null,
  ])) as PartitionRenameResult | null;
  if (!result?.success || !result.edit) {
    return result?.error ?? "Partition rename did not produce any edits.";
  }
  const edit = workspaceEditFromLsp(result.edit, versionOf);
  return edit ?? "The document changed after the rename was computed; run the rename again.";
}

// -- XC translation ---------------------------------------------------------

/** The source a translation describes. */
export type XcTranslationSource =
  | { purpose: "document"; uri: string; version: number; dialect: string }
  | { purpose: "standalone"; reason: string };

export type XcTranslationOutcome =
  | { kind: "translated"; payload: Record<string, unknown>; source: XcTranslationSource }
  | { kind: "stale" }
  | { kind: "failed"; error: string };

/** A document snapshot to translate: the exact text at `version`. */
export interface XcDocumentSnapshot {
  uri: string;
  version: number;
  source: string;
}

export function xcDocumentSnapshot(document: TextDocument): XcDocumentSnapshot {
  return { uri: document.uri.toString(), version: document.version, source: document.getText() };
}

function translationPayload(value: unknown): Record<string, unknown> | string {
  if (typeof value !== "object" || value === null) return "the translator returned no result";
  const payload = value as Record<string, unknown>;
  if (typeof payload.error === "string") return payload.error;
  return payload;
}

/** Translate `source` as standalone iRule text, recording why. */
export async function translateXcStandalone(
  send: ServerCommandSender,
  source: string,
  format: string,
  reason: string,
): Promise<XcTranslationOutcome> {
  const payload = translationPayload(await send("tcl-lsp.xcTranslate", [source, format]));
  if (typeof payload === "string") return { kind: "failed", error: payload };
  return { kind: "translated", payload, source: { purpose: "standalone", reason } };
}

/**
 * Translate an open document under its own analysis. A document the server
 * does not analyse as an iRule — or a server without the document command —
 * is translated as standalone text and says so; a snapshot the server no
 * longer holds is `stale`, never silently reparsed.
 */
export async function translateXcDocument(
  send: ServerCommandSender,
  snapshot: XcDocumentSnapshot,
  format: string,
): Promise<XcTranslationOutcome> {
  let answer: unknown;
  try {
    answer = await send("tcl-lsp.xcTranslateDocument", [snapshot, format]);
  } catch {
    return translateXcStandalone(send, snapshot.source, format, "unsupported");
  }
  const reason = unavailableReason(answer);
  if (reason === "stale") return { kind: "stale" };
  if (reason !== undefined) {
    return translateXcStandalone(send, snapshot.source, format, reason);
  }
  const payload = translationPayload(answer);
  if (typeof payload === "string") return { kind: "failed", error: payload };
  const described = payload.source as Partial<XcTranslationSource> & {
    uri?: unknown;
    version?: unknown;
    dialect?: unknown;
  };
  if (
    described?.purpose !== "document" ||
    described.uri !== snapshot.uri ||
    described.version !== snapshot.version ||
    typeof described.dialect !== "string"
  ) {
    return { kind: "failed", error: "the translation does not describe the requested document" };
  }
  return {
    kind: "translated",
    payload,
    source: {
      purpose: "document",
      uri: snapshot.uri,
      version: snapshot.version,
      dialect: described.dialect,
    },
  };
}

const STANDALONE_REASONS: Record<string, string> = {
  notIrules: "the document is not analysed as an iRule",
  notOpen: "the language server does not hold this document",
  analysis: "the document's own analysis is unavailable",
  unsupported: "the language server has no document translation",
  sourceOnly: "the source was supplied as text",
};

/** One line saying what a translation describes. */
export function describeXcSource(source: XcTranslationSource): string {
  if (source.purpose === "document") {
    return `Translated the open document (version ${source.version}, ${source.dialect}) under its own analysis.`;
  }
  const why = STANDALONE_REASONS[source.reason] ?? "the document translation was refused";
  return `Translated the text as standalone iRule source: ${why}.`;
}
