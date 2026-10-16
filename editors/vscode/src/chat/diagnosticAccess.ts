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
import type { Diagnostic as LspDiagnostic } from "vscode-languageclient";
import { DiagnosticCategory } from "./types";
import { CODE_TO_CATEGORY, CATEGORY_PREFIX_RULES, DEFAULT_CATEGORY } from "./canonicalDiagnostics";

function diagnosticCode(d: vscode.Diagnostic): string {
  if (typeof d.code === "object" && d.code !== null && "value" in d.code) {
    return String(d.code.value);
  }
  return String(d.code ?? "");
}

export function categorizeDiagnostic(code: string): DiagnosticCategory {
  const mapped = CODE_TO_CATEGORY[code];
  if (mapped) return mapped as DiagnosticCategory;
  for (const rule of CATEGORY_PREFIX_RULES) {
    if (code.startsWith(rule.prefix)) return rule.category as DiagnosticCategory;
  }
  return DEFAULT_CATEGORY as DiagnosticCategory;
}

export interface CategorisedDiagnostics {
  errors: vscode.Diagnostic[];
  security: vscode.Diagnostic[];
  taint: vscode.Diagnostic[];
  performance: vscode.Diagnostic[];
  threadSafety: vscode.Diagnostic[];
  controlFlow: vscode.Diagnostic[];
  style: vscode.Diagnostic[];
  optimiser: vscode.Diagnostic[];
  all: vscode.Diagnostic[];
}

export function categoriseDiagnostics(diagnostics: vscode.Diagnostic[]): CategorisedDiagnostics {
  const result: CategorisedDiagnostics = {
    errors: [],
    security: [],
    taint: [],
    performance: [],
    threadSafety: [],
    controlFlow: [],
    style: [],
    optimiser: [],
    all: [...diagnostics],
  };

  for (const d of diagnostics) {
    const code = diagnosticCode(d);
    const cat = categorizeDiagnostic(code);
    switch (cat) {
      case DiagnosticCategory.Error:
        result.errors.push(d);
        break;
      case DiagnosticCategory.Security:
        result.security.push(d);
        break;
      case DiagnosticCategory.Taint:
        result.taint.push(d);
        break;
      case DiagnosticCategory.Performance:
        result.performance.push(d);
        break;
      case DiagnosticCategory.ThreadSafety:
        result.threadSafety.push(d);
        break;
      case DiagnosticCategory.ControlFlow:
        result.controlFlow.push(d);
        break;
      case DiagnosticCategory.Optimiser:
        result.optimiser.push(d);
        break;
      default:
        result.style.push(d);
        break;
    }
  }

  return result;
}

/**
 * Format diagnostics for inclusion in an LLM prompt, with source context.
 */
export function formatDiagnosticsForLLM(diagnostics: vscode.Diagnostic[], source: string): string {
  const lines = source.split("\n");
  return diagnostics
    .map((d) => {
      const code = diagnosticCode(d);
      const line = d.range.start.line;
      const sourceLine = lines[line] ?? "";
      return `Line ${line + 1} [${code}]: ${d.message}\n  > ${sourceLine.trimEnd()}`;
    })
    .join("\n\n");
}

/**
 * A client-neutral transport installed by the Node or browser entrypoint.
 * The converter is the active LanguageClient's protocol-to-VS Code adapter,
 * so all standard LSP diagnostic fields keep their normal client semantics.
 */
export interface CurrentDiagnosticsTransport {
  /** Stable opaque identity for one actual client/server session generation. */
  readonly sessionIdentity: object;
  request(params: { uri: string; version: number; source: string }): Promise<unknown>;
  convert(diagnostics: LspDiagnostic[]): Promise<vscode.Diagnostic[]>;
  setSessionDialectOverride?(dialect: string | null): Promise<void>;
  isCurrent(): boolean;
}

export type CurrentDiagnosticsTransportProvider = () => CurrentDiagnosticsTransport | undefined;

let currentDiagnosticsTransportProvider: CurrentDiagnosticsTransportProvider | undefined;

export function setCurrentDiagnosticsTransportProvider(
  provider: CurrentDiagnosticsTransportProvider | undefined,
): vscode.Disposable {
  const previous = currentDiagnosticsTransportProvider;
  currentDiagnosticsTransportProvider = provider;
  return {
    dispose: () => {
      if (currentDiagnosticsTransportProvider === provider) {
        currentDiagnosticsTransportProvider = previous;
      }
    },
  };
}

export type CurrentDiagnosticsUnavailableReason =
  | "pending"
  | "stale"
  | "context"
  | "providerError"
  | "cancelled"
  | "disabled"
  | "excluded"
  | "unsupported"
  | "notOpen"
  | "timeout"
  | "clientUnavailable"
  | "malformedResponse";

export class CurrentDiagnosticsUnavailableError extends Error {
  constructor(readonly reason: CurrentDiagnosticsUnavailableReason) {
    super(`Current diagnostics unavailable: ${reason}`);
    this.name = "CurrentDiagnosticsUnavailableError";
  }
}

export class CurrentDiagnosticsSuppressedError extends Error {
  constructor(
    readonly reason: "disabled" | "excluded",
    readonly receipt: {
      uri: string;
      version: number;
      source: string;
      revision: string;
      inputsEpoch: string;
      resultId: string;
    },
  ) {
    super(`Current diagnostics were published as suppressed: ${reason}`);
    this.name = "CurrentDiagnosticsSuppressedError";
  }
}

export interface AnalysisContextSnapshot {
  readonly transport: CurrentDiagnosticsTransport;
  readonly overrideGeneration: number;
}

export interface SessionDialectOverrideLease {
  readonly dialect: string;
  readonly analysisContext: AnalysisContextSnapshot;
  restore(): Promise<AnalysisContextSnapshot | undefined>;
}

let analysisOverrideGeneration = 0;
let nextOverrideOwner = 1;
let activeOverrideOwner: number | undefined;
let acknowledgedLeaseOverride:
  { owner: number; sessionIdentity: object; transport: CurrentDiagnosticsTransport } | undefined;
let sessionOverrideOperationGeneration = 0;
let pendingOverrideTransition: { operation: number; sessionIdentity: object } | undefined;
let sessionOverrideQueue: Promise<void> = Promise.resolve();

function enqueueSessionOverride<T>(operation: () => Promise<T>): Promise<T> {
  const result = sessionOverrideQueue.then(operation, operation);
  sessionOverrideQueue = result.then(
    () => undefined,
    () => undefined,
  );
  return result;
}

export function captureAnalysisContext(): AnalysisContextSnapshot | undefined {
  const transport = currentDiagnosticsTransportProvider?.();
  if (
    !transport ||
    !transport.isCurrent() ||
    pendingOverrideTransition?.sessionIdentity === transport.sessionIdentity
  ) {
    return undefined;
  }
  return { transport, overrideGeneration: analysisOverrideGeneration };
}

export function invalidateAnalysisContext(): void {
  analysisOverrideGeneration += 1;
}

export function isAnalysisContextCurrent(
  context: AnalysisContextSnapshot | undefined,
): context is AnalysisContextSnapshot {
  return (
    context !== undefined &&
    context.overrideGeneration === analysisOverrideGeneration &&
    context.transport.isCurrent()
  );
}

export async function acquireSessionDialectOverride(
  dialect: string,
): Promise<SessionDialectOverrideLease> {
  const transport = currentDiagnosticsTransportProvider?.();
  if (!transport?.setSessionDialectOverride || !transport.isCurrent()) {
    throw new CurrentDiagnosticsUnavailableError("clientUnavailable");
  }
  const owner = nextOverrideOwner++;
  activeOverrideOwner = owner;
  const requestOperation = ++sessionOverrideOperationGeneration;
  pendingOverrideTransition = {
    operation: requestOperation,
    sessionIdentity: transport.sessionIdentity,
  };
  const requestGeneration = ++analysisOverrideGeneration;
  let attempted = false;
  let acknowledged = false;
  try {
    await enqueueSessionOverride(async () => {
      if (
        activeOverrideOwner !== owner ||
        sessionOverrideOperationGeneration !== requestOperation ||
        analysisOverrideGeneration !== requestGeneration ||
        !transport.isCurrent()
      ) {
        throw new CurrentDiagnosticsUnavailableError("stale");
      }
      attempted = true;
      await transport.setSessionDialectOverride!(dialect);
      acknowledgedLeaseOverride = { owner, sessionIdentity: transport.sessionIdentity, transport };
      acknowledged = true;
      if (
        activeOverrideOwner !== owner ||
        sessionOverrideOperationGeneration !== requestOperation ||
        analysisOverrideGeneration !== requestGeneration ||
        !transport.isCurrent()
      ) {
        throw new CurrentDiagnosticsUnavailableError("stale");
      }
    });
    if (
      activeOverrideOwner !== owner ||
      sessionOverrideOperationGeneration !== requestOperation ||
      analysisOverrideGeneration !== requestGeneration ||
      !transport.isCurrent()
    ) {
      throw new CurrentDiagnosticsUnavailableError("stale");
    }
    if (pendingOverrideTransition?.operation === requestOperation) {
      pendingOverrideTransition = undefined;
    }
  } catch (error) {
    if (activeOverrideOwner === owner) {
      const installedPredecessor =
        acknowledgedLeaseOverride?.sessionIdentity === transport.sessionIdentity;
      if ((acknowledged || attempted || installedPredecessor) && transport.isCurrent()) {
        const cleanupOperation = ++sessionOverrideOperationGeneration;
        activeOverrideOwner = undefined;
        pendingOverrideTransition = {
          operation: cleanupOperation,
          sessionIdentity: transport.sessionIdentity,
        };
        ++analysisOverrideGeneration;
        try {
          let cleanupAcknowledged = false;
          await enqueueSessionOverride(async () => {
            if (
              activeOverrideOwner !== undefined ||
              sessionOverrideOperationGeneration !== cleanupOperation ||
              !transport.isCurrent()
            ) {
              return;
            }
            await transport.setSessionDialectOverride!(null);
            if (acknowledgedLeaseOverride?.sessionIdentity === transport.sessionIdentity) {
              acknowledgedLeaseOverride = undefined;
            }
            cleanupAcknowledged = true;
          });
          if (cleanupAcknowledged && pendingOverrideTransition?.operation === cleanupOperation) {
            pendingOverrideTransition = undefined;
          }
        } catch {
          // Preserve the acquisition failure; cleanup grants no analysis context.
        }
      } else {
        activeOverrideOwner = undefined;
        sessionOverrideOperationGeneration += 1;
        if (pendingOverrideTransition?.operation === requestOperation) {
          pendingOverrideTransition = undefined;
        }
        analysisOverrideGeneration += 1;
      }
    }
    throw error;
  }
  const analysisContext: AnalysisContextSnapshot = {
    transport,
    overrideGeneration: requestGeneration,
  };
  let restored = false;
  return {
    dialect,
    analysisContext,
    restore: async () => {
      if (restored || activeOverrideOwner !== owner || !transport.isCurrent()) {
        return undefined;
      }
      const leaseWasCurrent = isAnalysisContextCurrent(analysisContext);
      restored = true;
      activeOverrideOwner = undefined;
      const clearOperation = ++sessionOverrideOperationGeneration;
      pendingOverrideTransition = {
        operation: clearOperation,
        sessionIdentity: transport.sessionIdentity,
      };
      const clearGeneration = ++analysisOverrideGeneration;
      await enqueueSessionOverride(async () => {
        if (
          activeOverrideOwner !== undefined ||
          sessionOverrideOperationGeneration !== clearOperation ||
          !transport.isCurrent()
        ) {
          throw new CurrentDiagnosticsUnavailableError("stale");
        }
        await transport.setSessionDialectOverride!(null);
        if (acknowledgedLeaseOverride?.sessionIdentity === transport.sessionIdentity) {
          acknowledgedLeaseOverride = undefined;
        }
        if (
          activeOverrideOwner !== undefined ||
          sessionOverrideOperationGeneration !== clearOperation ||
          !transport.isCurrent()
        ) {
          throw new CurrentDiagnosticsUnavailableError("stale");
        }
      });
      if (
        activeOverrideOwner !== undefined ||
        sessionOverrideOperationGeneration !== clearOperation ||
        !transport.isCurrent()
      ) {
        return undefined;
      }
      if (pendingOverrideTransition?.operation === clearOperation) {
        pendingOverrideTransition = undefined;
      }
      if (analysisOverrideGeneration !== clearGeneration) return undefined;
      return leaseWasCurrent ? { transport, overrideGeneration: clearGeneration } : undefined;
    },
  };
}

/** Explicit ownerless session change for the user-facing dialect command. */
export async function setUnleasedSessionDialectOverride(dialect: string | null): Promise<void> {
  const transport = currentDiagnosticsTransportProvider?.();
  if (!transport?.setSessionDialectOverride || !transport.isCurrent()) return;
  activeOverrideOwner = undefined;
  const operationGeneration = ++sessionOverrideOperationGeneration;
  pendingOverrideTransition = {
    operation: operationGeneration,
    sessionIdentity: transport.sessionIdentity,
  };
  ++analysisOverrideGeneration;
  await enqueueSessionOverride(async () => {
    if (
      activeOverrideOwner !== undefined ||
      sessionOverrideOperationGeneration !== operationGeneration
    ) {
      return;
    }
    if (!transport.isCurrent()) {
      if (pendingOverrideTransition?.operation === operationGeneration) {
        pendingOverrideTransition = undefined;
      }
      return;
    }
    await transport.setSessionDialectOverride!(dialect);
    acknowledgedLeaseOverride = undefined;
    if (
      activeOverrideOwner !== undefined ||
      sessionOverrideOperationGeneration !== operationGeneration ||
      !transport.isCurrent()
    ) {
      return;
    }
    if (pendingOverrideTransition?.operation === operationGeneration) {
      pendingOverrideTransition = undefined;
    }
  });
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function openedDocument(uri: vscode.Uri): vscode.TextDocument | undefined {
  const key = uri.toString();
  return vscode.workspace.textDocuments.find((document) => document.uri.toString() === key);
}

function snapshotMatches(
  uri: vscode.Uri,
  expectedDocument: vscode.TextDocument,
  version: number,
  source: string,
): boolean {
  const current = openedDocument(uri);
  return (
    current === expectedDocument && current.version === version && current.getText() === source
  );
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function withinDeadline<T>(promise: Promise<T>, deadline: number): Promise<T> {
  const remaining = deadline - Date.now();
  if (remaining <= 0) {
    return Promise.reject(new CurrentDiagnosticsUnavailableError("timeout"));
  }
  let timer: ReturnType<typeof setTimeout> | undefined;
  return Promise.race([
    promise,
    new Promise<never>((_resolve, reject) => {
      timer = setTimeout(
        () => reject(new CurrentDiagnosticsUnavailableError("timeout")),
        remaining,
      );
    }),
  ]).finally(() => {
    if (timer !== undefined) clearTimeout(timer);
  });
}

function isRevision(value: unknown): value is string {
  return typeof value === "string" && /^[0-9]+$/.test(value);
}

function isPosition(value: unknown): value is { line: number; character: number } {
  return (
    isRecord(value) &&
    Number.isInteger(value.line) &&
    (value.line as number) >= 0 &&
    Number.isInteger(value.character) &&
    (value.character as number) >= 0
  );
}

function isRange(value: unknown): boolean {
  if (!isRecord(value) || !isPosition(value.start) || !isPosition(value.end)) {
    return false;
  }
  const start = value.start as { line: number; character: number };
  const end = value.end as { line: number; character: number };
  return start.line < end.line || (start.line === end.line && start.character <= end.character);
}

function isDiagnostic(value: unknown): value is LspDiagnostic {
  if (
    !isRecord(value) ||
    !isRange(value.range) ||
    typeof value.message !== "string" ||
    (value.severity !== undefined &&
      value.severity !== 1 &&
      value.severity !== 2 &&
      value.severity !== 3 &&
      value.severity !== 4) ||
    (value.code !== undefined &&
      typeof value.code !== "string" &&
      typeof value.code !== "number") ||
    (value.source !== undefined && typeof value.source !== "string") ||
    (value.codeDescription !== undefined &&
      (!isRecord(value.codeDescription) || typeof value.codeDescription.href !== "string")) ||
    (value.tags !== undefined &&
      (!Array.isArray(value.tags) || !value.tags.every((tag) => tag === 1 || tag === 2))) ||
    (value.relatedInformation !== undefined &&
      (!Array.isArray(value.relatedInformation) ||
        !value.relatedInformation.every(
          (item) =>
            isRecord(item) &&
            typeof item.message === "string" &&
            isRecord(item.location) &&
            typeof item.location.uri === "string" &&
            isRange(item.location.range),
        )))
  ) {
    return false;
  }
  return true;
}

function validDiagnostics(value: unknown): value is LspDiagnostic[] {
  return Array.isArray(value) && value.every(isDiagnostic);
}

/**
 * Read diagnostics for the current open document from the server's completed
 * push-publication ledger. This request never triggers analysis. Server-reported
 * pending/stale states are retried within the caller's original budget; a local
 * document change and all terminal unavailable states remain explicit errors.
 */
export async function waitForDiagnostics(
  uri: vscode.Uri,
  opts?: { timeout?: number; minCount?: number; expectedSource?: string },
): Promise<vscode.Diagnostic[]> {
  const timeout = opts?.timeout ?? 5000;
  const minCount = opts?.minCount ?? 0;
  const deadline = Date.now() + timeout;
  const document = openedDocument(uri);
  if (!document) {
    throw new CurrentDiagnosticsUnavailableError("notOpen");
  }
  const source = opts?.expectedSource ?? document.getText();
  if (document.getText() !== source) {
    throw new CurrentDiagnosticsUnavailableError("stale");
  }
  const request = {
    uri: document.uri.toString(),
    version: document.version,
    source,
  };

  while (Date.now() < deadline) {
    if (!snapshotMatches(uri, document, request.version, request.source)) {
      throw new CurrentDiagnosticsUnavailableError("stale");
    }
    const transport = currentDiagnosticsTransportProvider?.();
    if (!transport) {
      throw new CurrentDiagnosticsUnavailableError("clientUnavailable");
    }
    if (!transport.isCurrent()) {
      await delay(Math.min(20, Math.max(1, deadline - Date.now())));
      continue;
    }

    let raw: unknown;
    try {
      raw = await withinDeadline(transport.request(request), deadline);
    } catch (error) {
      if (error instanceof CurrentDiagnosticsUnavailableError) throw error;
      throw new CurrentDiagnosticsUnavailableError("providerError");
    }
    if (!transport.isCurrent()) {
      continue;
    }

    // A response for an older editor snapshot is never returned or displayed.
    if (!snapshotMatches(uri, document, request.version, request.source)) {
      continue;
    }
    if (!isRecord(raw)) {
      throw new CurrentDiagnosticsUnavailableError("malformedResponse");
    }

    if (raw.status === "suppressed") {
      const reason = raw.reason;
      if (
        (reason !== "disabled" && reason !== "excluded") ||
        raw.uri !== request.uri ||
        raw.version !== request.version ||
        raw.source !== request.source ||
        !isRevision(raw.revision) ||
        !isRevision(raw.inputsEpoch) ||
        typeof raw.resultId !== "string" ||
        !validDiagnostics(raw.diagnostics)
      ) {
        throw new CurrentDiagnosticsUnavailableError("malformedResponse");
      }
      if (!snapshotMatches(uri, document, request.version, request.source)) {
        continue;
      }
      throw new CurrentDiagnosticsSuppressedError(reason, {
        uri: request.uri,
        version: request.version,
        source: request.source,
        revision: raw.revision,
        inputsEpoch: raw.inputsEpoch,
        resultId: raw.resultId,
      });
    }

    if (raw.status === "unavailable") {
      const reason = raw.reason;
      if (
        reason === "pending" ||
        reason === "stale" ||
        reason === "context" ||
        reason === "providerError" ||
        reason === "cancelled" ||
        reason === "disabled" ||
        reason === "excluded" ||
        reason === "unsupported" ||
        reason === "notOpen"
      ) {
        if (reason === "pending" || reason === "stale") {
          await delay(Math.min(100, Math.max(1, deadline - Date.now())));
          continue;
        }
        throw new CurrentDiagnosticsUnavailableError(reason);
      }
      throw new CurrentDiagnosticsUnavailableError("malformedResponse");
    }

    if (
      raw.status !== "complete" ||
      raw.uri !== request.uri ||
      raw.version !== request.version ||
      raw.source !== request.source ||
      !isRevision(raw.revision) ||
      !isRevision(raw.inputsEpoch) ||
      typeof raw.resultId !== "string" ||
      !validDiagnostics(raw.diagnostics)
    ) {
      throw new CurrentDiagnosticsUnavailableError("malformedResponse");
    }

    let diagnostics: vscode.Diagnostic[];
    try {
      if (Date.now() >= deadline) {
        throw new CurrentDiagnosticsUnavailableError("timeout");
      }
      diagnostics = await withinDeadline(transport.convert(raw.diagnostics), deadline);
      // A blocked event loop can let both the converter and timeout timer
      // resume after the deadline; recheck wall-clock time after conversion.
      if (Date.now() >= deadline) {
        throw new CurrentDiagnosticsUnavailableError("timeout");
      }
    } catch (error) {
      if (error instanceof CurrentDiagnosticsUnavailableError) throw error;
      throw new CurrentDiagnosticsUnavailableError("providerError");
    }
    if (
      !transport.isCurrent() ||
      !snapshotMatches(uri, document, request.version, request.source)
    ) {
      continue;
    }
    if (diagnostics.length >= minCount) {
      return diagnostics;
    }

    // minCount keeps its existing meaning, but only completed results can
    // satisfy it; a complete empty result is still distinct from unavailable.
    await delay(Math.min(100, Math.max(1, deadline - Date.now())));
  }

  throw new CurrentDiagnosticsUnavailableError("timeout");
}

/**
 * Render a section of diagnostics into the chat response stream.
 */
export function renderDiagnosticSection(
  response: vscode.ChatResponseStream,
  title: string,
  diagnostics: vscode.Diagnostic[],
  source: string,
): void {
  if (diagnostics.length === 0) return;

  const lines = source.split("\n");
  response.markdown(`\n### ${title} (${diagnostics.length})\n\n`);

  for (const d of diagnostics) {
    const code = diagnosticCode(d);
    const line = d.range.start.line;
    const sourceLine = lines[line] ?? "";
    const severity =
      d.severity === vscode.DiagnosticSeverity.Error
        ? "Error"
        : d.severity === vscode.DiagnosticSeverity.Warning
          ? "Warning"
          : "Info";
    response.markdown(
      `- **${code}** (${severity}, line ${line + 1}): ${d.message}\n  \`${sourceLine.trim()}\`\n`,
    );
  }
}
