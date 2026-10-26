// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

// Transform a document only while its captured source snapshot remains current.

import * as vscode from "vscode";
import {
  DocumentSourceSnapshot,
  isDocumentSourceSnapshotCurrent,
  replaceDocumentFromSnapshot,
  snapshotDocumentSource,
} from "./chat/codeUtils";
import { isAnalysisContextCurrent, resourceConfiguration } from "./chat/diagnosticAccess";

export type DocumentTransformCommand =
  | "tcl-lsp.optimiseDocument"
  | "tcl-lsp.minifyDocument"
  | "tcl-lsp.fixAllSafeIssues"
  | "tcl-lsp.insertPackageRequire";

export interface DocumentTransformRequest {
  uri: string;
  version: number;
  source: string;
}

export interface DocumentTransformReceipt extends DocumentTransformRequest {
  id: string;
  /** Original server ledger issuer, transported as lossless opaque bytes. */
  issuer: readonly number[];
}

export interface DocumentTransformSnapshot {
  readonly source: DocumentSourceSnapshot;
  readonly configuration: string;
}

export interface DocumentTransformResult {
  source: string;
  editReceipt: DocumentTransformReceipt;
}

export function captureDocumentTransform(
  document: vscode.TextDocument,
): DocumentTransformSnapshot | undefined {
  const source = snapshotDocumentSource(document);
  const configuration = resourceConfiguration(document);
  if (
    !source.analysisContext?.transport.requestTransform ||
    !source.analysisContext.transport.validateTransform ||
    configuration === undefined ||
    !isDocumentSourceSnapshotCurrent(source) ||
    !isAnalysisContextCurrent(source.analysisContext)
  )
    return undefined;
  return { source, configuration };
}

export function isDocumentTransformCurrent(snapshot: DocumentTransformSnapshot): boolean {
  return (
    isDocumentSourceSnapshotCurrent(snapshot.source) &&
    isAnalysisContextCurrent(snapshot.source.analysisContext) &&
    resourceConfiguration(snapshot.source.document) === snapshot.configuration
  );
}

function matchesReceipt(
  snapshot: DocumentTransformSnapshot,
  value: unknown,
): value is DocumentTransformResult {
  if (!value || typeof value !== "object") return false;
  const result = value as Partial<DocumentTransformResult>;
  const receipt = result.editReceipt;
  return (
    typeof result.source === "string" &&
    !!receipt &&
    typeof receipt.id === "string" &&
    receipt.id.length > 0 &&
    Array.isArray(receipt.issuer) &&
    receipt.issuer.length === 16 &&
    receipt.issuer.every(
      (byte) => typeof byte === "number" && Number.isInteger(byte) && byte >= 0 && byte <= 255,
    ) &&
    receipt.uri === snapshot.source.uri.toString() &&
    receipt.version === snapshot.source.version &&
    receipt.source === snapshot.source.fullSource
  );
}

export async function requestDocumentTransform<T extends { source: string }>(
  snapshot: DocumentTransformSnapshot,
  command: DocumentTransformCommand,
  options: readonly unknown[] = [],
): Promise<(T & DocumentTransformResult) | undefined> {
  const transport = snapshot.source.analysisContext?.transport;
  if (!isDocumentTransformCurrent(snapshot) || !transport?.requestTransform) return undefined;
  const request = {
    uri: snapshot.source.uri.toString(),
    version: snapshot.source.version,
    source: snapshot.source.fullSource,
  };
  try {
    const result = await transport.requestTransform(command, request, options);
    return isDocumentTransformCurrent(snapshot) && matchesReceipt(snapshot, result)
      ? (result as T & DocumentTransformResult)
      : undefined;
  } catch {
    return undefined;
  }
}

/** Recheck the same server result after UI awaits and immediately before editing. */
export async function applyDocumentTransform(
  snapshot: DocumentTransformSnapshot,
  result: DocumentTransformResult,
): Promise<boolean> {
  if (!isDocumentTransformCurrent(snapshot) || !matchesReceipt(snapshot, result)) return false;
  const transport = snapshot.source.analysisContext?.transport;
  if (!transport?.validateTransform) return false;
  const validateTransform = transport.validateTransform.bind(transport);
  try {
    return !!(await replaceDocumentFromSnapshot(snapshot.source, result.source, async () => {
      if (!isDocumentTransformCurrent(snapshot)) return false;
      const current = await validateTransform({
        ...result.editReceipt,
        replacement: result.source,
      });
      return current === true && isDocumentTransformCurrent(snapshot);
    }));
  } catch {
    return false;
  }
}
