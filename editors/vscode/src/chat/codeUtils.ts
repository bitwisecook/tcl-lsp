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

import * as vscode from "vscode";
import { isTclLanguage } from "../languageIds";
import type { CommandContext } from "./types";
import { DEFAULT_DIALECT } from "./dialectCatalog";
import {
  AnalysisContextSnapshot,
  captureAnalysisContext,
  isAnalysisContextCurrent,
} from "./diagnosticAccess";

/**
 * Extract the first fenced code block from LLM response text.
 * Looks for ```tcl or ``` blocks and returns the inner content.
 */
export function extractCodeBlock(text: string): string | undefined {
  // Prefer an explicitly Tcl-ish fence, but accept any tag. Models label these
  // blocks inconsistently — ```irules, ```f5, ```tcl8.6, sometimes nothing at
  // all — and treating an unrecognised tag as "no code block" sent a perfectly
  // good response down the malformed-response fallback path.
  const fence = /```([A-Za-z0-9_.+-]*)[ \t]*\r?\n([\s\S]*?)```/g;
  let firstBlock: string | undefined;
  for (const match of text.matchAll(fence)) {
    const tag = (match[1] ?? "").toLowerCase();
    const body = match[2].trimEnd();
    if (!body.trim()) continue;
    if (tag.startsWith("tcl") || tag.startsWith("irule")) {
      return body;
    }
    firstBlock ??= body;
  }
  return firstBlock;
}

export type DocumentSourceChannel =
  "file-reference" | "location-reference" | "editor-selection" | "editor-document" | "inline-code";

export interface DocumentSourceSnapshot {
  document: vscode.TextDocument;
  uri: vscode.Uri;
  version: number;
  fullSource: string;
  source: string;
  selection?: vscode.Range;
  languageId: string;
  resourceDialectHint: string;
  requestedAnalysisDialect: string | null;
  analysisContext: AnalysisContextSnapshot | undefined;
  sourceChannel: Exclude<DocumentSourceChannel, "inline-code">;
}

export interface ResolvedIruleSource {
  source: string;
  snapshot?: DocumentSourceSnapshot;
  languageId?: string;
  resourceDialectHint: string | undefined;
  requestedAnalysisDialect: string | null;
  sourceChannel: DocumentSourceChannel;
}

export interface DocumentWriteReceipt {
  snapshotId: string;
  uri: vscode.Uri;
  version: number;
  source: string;
  languageId: string;
  resourceDialectHint: string;
  requestedAnalysisDialect: string | null;
  sourceChannel: string;
}

export function snapshotDocumentSource(
  document: vscode.TextDocument,
  selection?: vscode.Range,
  sourceChannel: Exclude<DocumentSourceChannel, "inline-code"> = "editor-document",
): DocumentSourceSnapshot {
  const resourceDialectHint = vscode.workspace
    .getConfiguration("tclLsp", document.uri)
    .get<string>("dialect", DEFAULT_DIALECT);
  return {
    document,
    uri: document.uri,
    version: document.version,
    fullSource: document.getText(),
    source: selection ? document.getText(selection) : document.getText(),
    selection,
    languageId: document.languageId,
    resourceDialectHint,
    requestedAnalysisDialect: null,
    analysisContext: captureAnalysisContext(),
    sourceChannel,
  };
}

/** Whether a retained source selection still names the same document bytes. */
export function isDocumentSourceSnapshotCurrent(snapshot: DocumentSourceSnapshot): boolean {
  const document = snapshot.document;
  return (
    !document.isClosed &&
    sameUri(document.uri, snapshot.uri) &&
    document.version === snapshot.version &&
    document.getText() === snapshot.fullSource &&
    document.languageId === snapshot.languageId &&
    (snapshot.selection === undefined ||
      document.getText(snapshot.selection) === snapshot.source) &&
    (snapshot.selection !== undefined || snapshot.source === snapshot.fullSource)
  );
}

function inlineSource(source: string): ResolvedIruleSource {
  return {
    source,
    resourceDialectHint: undefined,
    requestedAnalysisDialect: null,
    sourceChannel: "inline-code",
  };
}

async function resolveDocumentReference(
  reference: vscode.ChatRequest["references"][number],
): Promise<ResolvedIruleSource | undefined> {
  if (reference.value instanceof vscode.Uri) {
    const document = await vscode.workspace.openTextDocument(reference.value);
    const snapshot = snapshotDocumentSource(document, undefined, "file-reference");
    return {
      source: snapshot.source,
      snapshot,
      languageId: snapshot.languageId,
      resourceDialectHint: snapshot.resourceDialectHint,
      requestedAnalysisDialect: snapshot.requestedAnalysisDialect,
      sourceChannel: "file-reference",
    };
  }
  if (reference.value instanceof vscode.Location) {
    const document = await vscode.workspace.openTextDocument(reference.value.uri);
    const snapshot = snapshotDocumentSource(document, reference.value.range, "location-reference");
    return {
      source: snapshot.source,
      snapshot,
      languageId: snapshot.languageId,
      resourceDialectHint: snapshot.resourceDialectHint,
      requestedAnalysisDialect: snapshot.requestedAnalysisDialect,
      sourceChannel: "location-reference",
    };
  }
  return undefined;
}

/** Resolve source together with its original document and selected extent. */
export async function resolveIruleSource(
  ctx: CommandContext,
): Promise<ResolvedIruleSource | undefined> {
  for (const ref of ctx.request.references) {
    const resolved = await resolveDocumentReference(ref);
    if (resolved) return resolved;
  }

  const editor = vscode.window.activeTextEditor;
  if (editor && isTclLanguage(editor.document.languageId)) {
    const selection = editor.selection.isEmpty ? undefined : editor.selection;
    const sourceChannel = selection ? "editor-selection" : "editor-document";
    const snapshot = snapshotDocumentSource(editor.document, selection, sourceChannel);
    return {
      source: snapshot.source,
      snapshot,
      languageId: snapshot.languageId,
      resourceDialectHint: snapshot.resourceDialectHint,
      requestedAnalysisDialect: snapshot.requestedAnalysisDialect,
      sourceChannel,
    };
  }

  const source = extractCodeBlock(ctx.request.prompt);
  if (source) {
    return inlineSource(source);
  }
  return undefined;
}

/** Compatibility projection for callers that only read source text. */
export async function resolveIruleCode(ctx: CommandContext): Promise<string | undefined> {
  return (await resolveIruleSource(ctx))?.source;
}

const pendingDocumentWrites = new Map<string, DocumentSourceSnapshot>();
let nextDocumentWriteId = 1;
const MAX_PENDING_DOCUMENT_WRITES = 128;

export function documentWriteReceipt(snapshot: DocumentSourceSnapshot): DocumentWriteReceipt {
  if (snapshot.source !== snapshot.fullSource || snapshot.selection) {
    throw new Error("Whole-document write requires a whole-document source snapshot");
  }
  if (!isAnalysisContextCurrent(snapshot.analysisContext)) {
    throw new Error("Analysis context changed before the source action could be published");
  }
  const snapshotId = String(nextDocumentWriteId++);
  pendingDocumentWrites.set(snapshotId, snapshot);
  if (pendingDocumentWrites.size > MAX_PENDING_DOCUMENT_WRITES) {
    const oldest = pendingDocumentWrites.keys().next().value;
    if (oldest !== undefined) pendingDocumentWrites.delete(oldest);
  }
  return {
    snapshotId,
    uri: snapshot.uri,
    version: snapshot.version,
    source: snapshot.fullSource,
    languageId: snapshot.languageId,
    resourceDialectHint: snapshot.resourceDialectHint,
    requestedAnalysisDialect: snapshot.requestedAnalysisDialect,
    sourceChannel: snapshot.sourceChannel,
  };
}

function isUriComponents(value: unknown): value is vscode.Uri {
  if (typeof value !== "object" || value === null) return false;
  const uri = value as Partial<vscode.Uri>;
  return (
    typeof uri.scheme === "string" &&
    typeof uri.authority === "string" &&
    typeof uri.path === "string" &&
    typeof uri.query === "string" &&
    typeof uri.fragment === "string"
  );
}

function sameUri(left: vscode.Uri, right: vscode.Uri): boolean {
  return (
    left.scheme === right.scheme &&
    left.authority === right.authority &&
    left.path === right.path &&
    left.query === right.query &&
    left.fragment === right.fragment
  );
}

export async function replaceDocumentFromSnapshot(
  snapshot: DocumentSourceSnapshot,
  replacement: string,
): Promise<DocumentSourceSnapshot | undefined> {
  const document = snapshot.document;
  if (
    snapshot.source !== snapshot.fullSource ||
    snapshot.selection ||
    !sameUri(document.uri, snapshot.uri) ||
    document.version !== snapshot.version ||
    document.getText() !== snapshot.fullSource ||
    document.languageId !== snapshot.languageId ||
    !isAnalysisContextCurrent(snapshot.analysisContext) ||
    vscode.workspace
      .getConfiguration("tclLsp", document.uri)
      .get<string>("dialect", DEFAULT_DIALECT) !== snapshot.resourceDialectHint
  ) {
    return undefined;
  }
  if (replacement === snapshot.fullSource) return snapshot;

  const editor = await vscode.window.showTextDocument(document);
  if (
    document.version !== snapshot.version ||
    document.getText() !== snapshot.fullSource ||
    document.languageId !== snapshot.languageId ||
    !isAnalysisContextCurrent(snapshot.analysisContext) ||
    vscode.workspace
      .getConfiguration("tclLsp", document.uri)
      .get<string>("dialect", DEFAULT_DIALECT) !== snapshot.resourceDialectHint ||
    editor.document !== document
  ) {
    return undefined;
  }
  const applied = await editor.edit((edit) => {
    edit.replace(new vscode.Range(0, 0, document.lineCount, 0), replacement);
  });
  if (
    !applied ||
    document.version !== snapshot.version + 1 ||
    !sameUri(document.uri, snapshot.uri) ||
    document.getText() !== replacement ||
    document.languageId !== snapshot.languageId ||
    !isAnalysisContextCurrent(snapshot.analysisContext) ||
    vscode.workspace
      .getConfiguration("tclLsp", document.uri)
      .get<string>("dialect", DEFAULT_DIALECT) !== snapshot.resourceDialectHint
  ) {
    return undefined;
  }
  return {
    ...snapshotDocumentSource(document, undefined, snapshot.sourceChannel),
    requestedAnalysisDialect: snapshot.requestedAnalysisDialect,
    analysisContext: snapshot.analysisContext,
    sourceChannel: snapshot.sourceChannel,
  };
}

export async function applyDocumentWrite(replacement: unknown, receipt: unknown): Promise<boolean> {
  if (
    typeof replacement !== "string" ||
    typeof receipt !== "object" ||
    receipt === null ||
    !("snapshotId" in receipt) ||
    typeof receipt.snapshotId !== "string" ||
    !("uri" in receipt) ||
    !isUriComponents(receipt.uri) ||
    !("version" in receipt) ||
    typeof receipt.version !== "number" ||
    !Number.isSafeInteger(receipt.version) ||
    receipt.version < 0 ||
    !("source" in receipt) ||
    typeof receipt.source !== "string" ||
    !("languageId" in receipt) ||
    typeof receipt.languageId !== "string" ||
    !("resourceDialectHint" in receipt) ||
    (typeof receipt.resourceDialectHint !== "string" &&
      receipt.resourceDialectHint !== undefined) ||
    !("requestedAnalysisDialect" in receipt) ||
    (typeof receipt.requestedAnalysisDialect !== "string" &&
      receipt.requestedAnalysisDialect !== null) ||
    !("sourceChannel" in receipt) ||
    typeof receipt.sourceChannel !== "string"
  ) {
    return false;
  }
  const sourceChannels = new Set([
    "file-reference",
    "location-reference",
    "editor-selection",
    "editor-document",
  ]);
  if (!sourceChannels.has(receipt.sourceChannel)) return false;

  const snapshot = pendingDocumentWrites.get(receipt.snapshotId);
  pendingDocumentWrites.delete(receipt.snapshotId);
  if (
    !snapshot ||
    !isAnalysisContextCurrent(snapshot.analysisContext) ||
    !sameUri(snapshot.uri, receipt.uri) ||
    snapshot.version !== receipt.version ||
    snapshot.fullSource !== receipt.source ||
    snapshot.languageId !== receipt.languageId ||
    snapshot.resourceDialectHint !== receipt.resourceDialectHint ||
    snapshot.requestedAnalysisDialect !== receipt.requestedAnalysisDialect ||
    snapshot.sourceChannel !== receipt.sourceChannel
  ) {
    return false;
  }
  return (await replaceDocumentFromSnapshot(snapshot, replacement)) !== undefined;
}

/**
 * Resolve two code sources for comparison without dropping document snapshots,
 * selected extents, or the whole-source bytes they were read from.
 */
export async function resolveTwoIruleSources(
  ctx: CommandContext,
): Promise<[ResolvedIruleSource, ResolvedIruleSource] | undefined> {
  const sources: ResolvedIruleSource[] = [];
  for (const ref of ctx.request.references) {
    const resolved = await resolveDocumentReference(ref);
    if (resolved) sources.push(resolved);
    if (sources.length >= 2) return [sources[0], sources[1]];
  }

  if (sources.length === 1) {
    const editor = vscode.window.activeTextEditor;
    if (editor && isTclLanguage(editor.document.languageId)) {
      const selection = editor.selection.isEmpty ? undefined : editor.selection;
      const channel = selection ? "editor-selection" : "editor-document";
      const snapshot = snapshotDocumentSource(editor.document, selection, channel);
      const activeSource: ResolvedIruleSource = {
        source: snapshot.source,
        snapshot,
        languageId: snapshot.languageId,
        resourceDialectHint: snapshot.resourceDialectHint,
        requestedAnalysisDialect: snapshot.requestedAnalysisDialect,
        sourceChannel: channel,
      };
      return [activeSource, sources[0]];
    }
  }

  const blocks = [...ctx.request.prompt.matchAll(/```(?:tcl|irule)?\s*\n([\s\S]+?)```/g)];
  if (blocks.length >= 2) {
    return [inlineSource(blocks[0][1].trimEnd()), inlineSource(blocks[1][1].trimEnd())];
  }
  return undefined;
}

/** Compatibility projection for callers that only need the two source strings. */
export async function resolveTwoCodeSources(
  ctx: CommandContext,
): Promise<[string, string] | undefined> {
  const sources = await resolveTwoIruleSources(ctx);
  return sources ? [sources[0].source, sources[1].source] : undefined;
}

/**
 * Open an untitled Tcl document with the given content, or update an existing one.
 */
export async function ensureDocumentOpen(
  code: string,
  existingDoc?: vscode.TextDocument,
  expectedSnapshot?: DocumentSourceSnapshot,
): Promise<vscode.TextDocument> {
  if (existingDoc) {
    if (!expectedSnapshot || expectedSnapshot.document !== existingDoc) {
      throw new Error("Existing document write requires its source snapshot");
    }
    const updated = await replaceDocumentFromSnapshot(expectedSnapshot, code);
    if (!updated) throw new Error("Document changed before the generated source could be applied");
    return existingDoc;
  }
  const doc = await vscode.workspace.openTextDocument({ language: "tcl", content: code });
  await vscode.window.showTextDocument(doc);
  return doc;
}
