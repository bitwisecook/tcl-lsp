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

import * as path from "path";
import * as vscode from "vscode";
import { isTclLanguage } from "../extension";
import {
  resolveIruleCode,
  resolveIruleSource,
  ensureDocumentOpen,
  type DocumentSourceSnapshot,
  snapshotDocumentSource,
  isDocumentSourceSnapshotCurrent,
} from "./codeUtils";
import {
  captureAnalysisContext,
  isAnalysisContextCurrent,
  resourceConfiguration,
  currentDiagnosticsReportMatches,
  validateCurrentDiagnosticsPublication,
  waitForDiagnosticsReport,
  type CurrentDiagnosticsReport,
  type SessionDialectOverrideLease,
} from "./diagnosticAccess";
import { buildPrompt } from "./promptLoader";
import { CommandContext } from "./types";

const DEFAULT_DIAGNOSTIC_TIMEOUT_MS = 2500;
const DEFAULT_MAX_DIAGNOSTICS = 8;
const DEFAULT_MAX_SYMBOLS = 15;
const DEFAULT_MAX_EVENTS = 6;

interface IruleEventInfo {
  event: string;
  known: boolean;
  /** The event-completion detail flag, not an F5 deprecation verdict. */
  deprecated: boolean;
  lifecycle?: { introduced: string | null; retired: string | null } | null;
  validCommandCount: number;
  sampleCommands: string[];
}

export interface SourceSymbolDefinition {
  kind: string;
  name: string;
  detail?: string;
  line: number;
}

export interface PromptContextOptions {
  code?: string;
  document?: vscode.TextDocument;
  /** Captured source language/version label; this is descriptive, not a dialect authority. */
  sourceContextLabel?: string;
  /** A completed report already obtained for this exact document and source. */
  completedDiagnosticsReport?: CurrentDiagnosticsReport;
  /** Recheck the exact source snapshot after awaits and before each model request. */
  isSourceCurrent?: () => boolean;
  allowAmbientContext?: boolean;
  includeDiagnostics?: boolean;
  includeSymbolDefinitions?: boolean;
  includeEventMetadata?: boolean;
  diagnosticsTimeoutMs?: number;
  maxDiagnostics?: number;
  maxSymbols?: number;
  maxEvents?: number;
}

/** Retained source/report composition for a user-invoked analysis operation.
 * Reporting and canonical proposal topics issue no source-write authority.
 */
export class DiagnosticSourceOperation {
  private report: CurrentDiagnosticsReport | undefined;

  private constructor(
    private snapshot: DocumentSourceSnapshot,
    private readonly configuration: string,
    private readonly origin: DocumentSourceSnapshot | undefined,
    private readonly originConfiguration: string | undefined,
    private readonly token: vscode.CancellationToken,
  ) {
    this.snapshot = Object.freeze({ ...snapshot });
  }

  get sourceSnapshot(): DocumentSourceSnapshot {
    return this.snapshot;
  }

  /** Preserve the handlers' active-full-document preference and attached extent.
   * A selected/inline source is analysed as an explicit detached document while
   * its authentic original selection remains a current dependency.
   */
  static async resolve(ctx: CommandContext): Promise<DiagnosticSourceOperation | undefined> {
    const initialContext = captureAnalysisContext();
    if (!isAnalysisContextCurrent(initialContext)) {
      throw new Error("No current analysis session is available for this operation");
    }
    const editor = vscode.window.activeTextEditor;
    const resolved =
      editor && isTclLanguage(editor.document.languageId)
        ? { source: editor.document.getText(), snapshot: snapshotDocumentSource(editor.document) }
        : await resolveIruleSource(ctx);
    if (!isAnalysisContextCurrent(initialContext)) {
      throw new Error("Analysis session changed while source was resolved");
    }
    if (!resolved?.source) return undefined;
    const origin = resolved.snapshot;
    const originConfiguration = origin ? resourceConfiguration(origin.document) : undefined;
    if (origin && (!isDocumentSourceSnapshotCurrent(origin) || originConfiguration === undefined)) {
      throw new Error("The original source or resource configuration is unavailable");
    }
    const document =
      origin &&
      isTclLanguage(origin.document.languageId) &&
      origin.source === origin.fullSource &&
      !origin.selection
        ? origin.document
        : await ensureDocumentOpen(resolved.source);
    if (
      !isAnalysisContextCurrent(initialContext) ||
      (origin &&
        (!isDocumentSourceSnapshotCurrent(origin) ||
          resourceConfiguration(origin.document) !== originConfiguration))
    ) {
      throw new Error("The original source changed while its analysis document was opened");
    }
    const snapshot =
      origin && origin.document === document ? origin : snapshotDocumentSource(document);
    const configuration = resourceConfiguration(document);
    if (configuration === undefined || snapshot.fullSource !== resolved.source) {
      throw new Error("The analysis document does not retain the requested full source/input");
    }
    const operation = new DiagnosticSourceOperation(
      snapshot,
      configuration,
      origin,
      originConfiguration,
      ctx.token,
    );
    if (!operation.isCurrent()) throw new Error("The analysis source is no longer current");
    return operation;
  }

  /** Only an actual acknowledged lease may advance this operation's override.
   * Original source/resource/session identity must survive its own async await.
   */
  adoptOverride(lease: SessionDialectOverrideLease): void {
    const before = this.sourceSnapshot.analysisContext;
    if (
      this.report ||
      !before ||
      !this.sourceUnchanged() ||
      !isAnalysisContextCurrent(lease.analysisContext) ||
      before.transport.sessionIdentity !== lease.analysisContext.transport.sessionIdentity
    ) {
      throw new Error("The source operation cannot adopt this analysis override");
    }
    this.snapshot = Object.freeze({
      ...this.sourceSnapshot,
      analysisContext: lease.analysisContext,
      requestedAnalysisDialect: lease.dialect,
    });
  }

  /** Pure original-source/settings check, also used across an owned restore. */
  sourceUnchanged(): boolean {
    return (
      isDocumentSourceSnapshotCurrent(this.sourceSnapshot) &&
      resourceConfiguration(this.sourceSnapshot.document) === this.configuration &&
      (!this.origin ||
        (isDocumentSourceSnapshotCurrent(this.origin) &&
          resourceConfiguration(this.origin.document) === this.originConfiguration))
    );
  }

  isCurrent(): boolean {
    const snapshot = this.sourceSnapshot;
    return (
      !this.token.isCancellationRequested &&
      this.sourceUnchanged() &&
      isAnalysisContextCurrent(snapshot.analysisContext) &&
      (!this.report ||
        currentDiagnosticsReportMatches(this.report, {
          document: snapshot.document,
          uri: snapshot.uri,
          version: snapshot.version,
          source: snapshot.fullSource,
        }))
    );
  }

  async readReport(timeout = 5000): Promise<CurrentDiagnosticsReport> {
    if (this.report || !this.isCurrent()) throw new Error("The analysis operation is not current");
    const snapshot = this.sourceSnapshot;
    const report = await waitForDiagnosticsReport(snapshot.uri, {
      timeout,
      expectedSource: snapshot.fullSource,
    });
    if (
      !this.isCurrent() ||
      report.analysisContext.overrideGeneration !== snapshot.analysisContext?.overrideGeneration ||
      report.sessionIdentity !== snapshot.analysisContext?.transport.sessionIdentity ||
      !currentDiagnosticsReportMatches(report, {
        document: snapshot.document,
        uri: snapshot.uri,
        version: snapshot.version,
        source: snapshot.fullSource,
      })
    ) {
      throw new Error("The completed report does not belong to this original source operation");
    }
    this.report = report;
    return report;
  }

  async assertPublished(): Promise<void> {
    const snapshot = this.sourceSnapshot;
    if (
      !this.isCurrent() ||
      !this.report ||
      !(await validateCurrentDiagnosticsPublication(this.report, {
        document: snapshot.document,
        uri: snapshot.uri,
        version: snapshot.version,
        source: snapshot.fullSource,
      })) ||
      !this.isCurrent()
    ) {
      throw new Error("The source operation's completed publication is no longer current");
    }
  }

  promptOptions(): PromptContextOptions {
    if (!this.report || !this.isCurrent()) throw new Error("No current completed source report");
    return {
      code: this.sourceSnapshot.fullSource,
      document: this.sourceSnapshot.document,
      completedDiagnosticsReport: this.report,
      isSourceCurrent: () => this.isCurrent(),
      allowAmbientContext: false,
      ...(this.origin && this.origin.document !== this.sourceSnapshot.document
        ? {
            sourceContextLabel:
              "Original attached extent analysed as an explicit detached source document",
          }
        : {}),
    };
  }
}

function diagnosticCode(diagnostic: vscode.Diagnostic): string {
  if (
    typeof diagnostic.code === "object" &&
    diagnostic.code !== null &&
    "value" in diagnostic.code
  ) {
    return String(diagnostic.code.value);
  }
  return String(diagnostic.code ?? "");
}

function diagnosticSeverity(severity: vscode.DiagnosticSeverity): string {
  if (severity === vscode.DiagnosticSeverity.Error) {
    return "error";
  }
  if (severity === vscode.DiagnosticSeverity.Warning) {
    return "warning";
  }
  if (severity === vscode.DiagnosticSeverity.Information) {
    return "info";
  }
  return "hint";
}

function formatDocumentLabel(document: vscode.TextDocument): string {
  if (document.uri.scheme !== "file") {
    return document.uri.toString(true);
  }

  const workspaceFolder = vscode.workspace.getWorkspaceFolder(document.uri);
  if (!workspaceFolder) {
    return document.uri.fsPath;
  }

  const relative = path.relative(workspaceFolder.uri.fsPath, document.uri.fsPath);
  return relative || document.uri.fsPath;
}

function isDocumentSymbol(
  symbol: vscode.DocumentSymbol | vscode.SymbolInformation,
): symbol is vscode.DocumentSymbol {
  return "selectionRange" in symbol;
}

function flattenDocumentSymbols(
  symbols: readonly vscode.DocumentSymbol[],
  into: SourceSymbolDefinition[] = [],
): SourceSymbolDefinition[] {
  for (const symbol of symbols) {
    const kindLabel = (vscode.SymbolKind[symbol.kind] ?? "Symbol") as string;
    into.push({
      kind: kindLabel,
      name: symbol.name,
      detail: symbol.detail || undefined,
      line: symbol.selectionRange.start.line + 1,
    });
    if (symbol.children.length > 0) {
      flattenDocumentSymbols(symbol.children, into);
    }
  }
  return into;
}

async function resolveContextDocument(
  ctx: CommandContext,
  explicitDoc?: vscode.TextDocument,
  allowAmbientContext = true,
): Promise<vscode.TextDocument | undefined> {
  if (explicitDoc) {
    return explicitDoc;
  }

  if (!allowAmbientContext) {
    return undefined;
  }

  for (const ref of ctx.request.references) {
    try {
      if (ref.value instanceof vscode.Uri) {
        const doc = await vscode.workspace.openTextDocument(ref.value);
        if (isTclLanguage(doc.languageId)) {
          return doc;
        }
      }
      if (ref.value instanceof vscode.Location) {
        const doc = await vscode.workspace.openTextDocument(ref.value.uri);
        if (isTclLanguage(doc.languageId)) {
          return doc;
        }
      }
    } catch {
      // Ignore unreadable references and continue scanning.
    }
  }

  const editor = vscode.window.activeTextEditor;
  if (editor && isTclLanguage(editor.document.languageId)) {
    return editor.document;
  }

  return undefined;
}

async function resolveContextCode(
  ctx: CommandContext,
  explicitCode: string | undefined,
  document: vscode.TextDocument | undefined,
  allowAmbientContext: boolean,
): Promise<string | undefined> {
  if (typeof explicitCode === "string") {
    return explicitCode;
  }

  if (document) {
    return document.getText();
  }

  if (!allowAmbientContext) {
    return undefined;
  }

  return resolveIruleCode(ctx);
}

export function detectIruleEvents(
  definitions: readonly SourceSymbolDefinition[],
  maxEvents = DEFAULT_MAX_EVENTS,
): string[] {
  return [
    ...new Set(definitions.filter((entry) => entry.kind === "Event").map((entry) => entry.name)),
  ].slice(0, Math.max(0, maxEvents));
}

/** Names come from the shared provider for the exact requested source. */
export async function extractSourceSymbolDefinitions(
  source: string,
  maxSymbols = DEFAULT_MAX_SYMBOLS,
  document?: vscode.TextDocument,
  languageId?: string,
): Promise<SourceSymbolDefinition[] | undefined> {
  try {
    const language =
      languageId ?? (document && isTclLanguage(document.languageId) ? document.languageId : "tcl");
    if (!isTclLanguage(language)) {
      return undefined;
    }
    const original =
      document?.getText() === source &&
      isTclLanguage(document.languageId) &&
      document.languageId === language
        ? document
        : undefined;
    const owner =
      original ?? (await vscode.workspace.openTextDocument({ language, content: source }));
    const version = owner.version;
    const symbols = await vscode.commands.executeCommand<
      ReadonlyArray<vscode.DocumentSymbol | vscode.SymbolInformation> | undefined
    >("vscode.executeDocumentSymbolProvider", owner.uri);
    if (
      !symbols ||
      owner.isClosed ||
      owner.version !== version ||
      owner.getText() !== source ||
      !isTclLanguage(owner.languageId) ||
      owner.languageId !== language
    ) {
      return undefined;
    }
    const definitions: SourceSymbolDefinition[] = [];
    for (const symbol of symbols) {
      if (isDocumentSymbol(symbol)) {
        flattenDocumentSymbols([symbol], definitions);
      } else {
        definitions.push({
          kind: (vscode.SymbolKind[symbol.kind] ?? "Symbol") as string,
          name: symbol.name,
          line: symbol.location.range.start.line + 1,
        });
      }
    }
    return definitions.slice(0, Math.max(0, maxSymbols));
  } catch {
    return undefined;
  }
}

async function collectEventMetadata(
  ctx: CommandContext,
  events: readonly string[],
): Promise<Array<IruleEventInfo | undefined>> {
  return Promise.all(
    events.map(async (eventName) => {
      try {
        const response = (await ctx.client.sendRequest("workspace/executeCommand", {
          command: "tcl-lsp.describeIruleEvent",
          arguments: [eventName],
        })) as IruleEventInfo | null;
        return response ?? undefined;
      } catch {
        return undefined;
      }
    }),
  );
}

async function buildDiagnosticsSection(
  document: vscode.TextDocument | undefined,
  expectedSource: string | undefined,
  maxDiagnostics: number,
  timeoutMs: number,
): Promise<{ text: string; report?: CurrentDiagnosticsReport }> {
  if (!document) {
    return { text: "Diagnostics: unavailable (no open Tcl document URI)." };
  }
  if (expectedSource === undefined) {
    return { text: "Diagnostics: unavailable (no resolved source snapshot)." };
  }

  const report = await waitForDiagnosticsReport(document.uri, {
    timeout: timeoutMs,
    expectedSource,
  });
  const diagnostics = [...report.diagnostics];

  const actionable = diagnostics
    .filter((diagnostic) => diagnostic.severity <= vscode.DiagnosticSeverity.Warning)
    .sort((left, right) => {
      const lineDiff = left.range.start.line - right.range.start.line;
      if (lineDiff !== 0) {
        return lineDiff;
      }
      return left.range.start.character - right.range.start.character;
    });

  if (actionable.length === 0) {
    return {
      text: `Diagnostics: no errors/warnings (completed ${report.response.dialectId} report for source version ${report.response.version}).`,
      report,
    };
  }

  const visible = actionable.slice(0, maxDiagnostics);
  const lines = visible.map((diagnostic) => {
    const code = diagnosticCode(diagnostic) || "unknown";
    return (
      `- [${code}] line ${diagnostic.range.start.line + 1} (${diagnosticSeverity(diagnostic.severity)}): ` +
      diagnostic.message.replace(/\s+/g, " ").trim()
    );
  });

  if (actionable.length > visible.length) {
    lines.push(`- ... and ${actionable.length - visible.length} more issue(s).`);
  }

  return {
    text: [
      `Diagnostics (${actionable.length}; completed ${report.response.dialectId} report, source version ${report.response.version}):`,
      ...lines,
    ].join("\n"),
    report,
  };
}

function buildSymbolSection(symbols: readonly SourceSymbolDefinition[] | undefined): string {
  if (!symbols) {
    return "Symbol definitions: unavailable.";
  }
  if (symbols.length === 0) {
    return "Symbol definitions: none reported.";
  }

  const lines = symbols.map((symbol) => {
    const detail = symbol.detail ? ` ${symbol.detail}` : "";
    return `- ${symbol.kind} ${symbol.name}${detail} (line ${symbol.line})`;
  });

  return [`Symbol definitions (${symbols.length}):`, ...lines].join("\n");
}

function buildEventSection(
  events: readonly string[],
  metadata: readonly (IruleEventInfo | undefined)[],
): string {
  if (events.length === 0) {
    return "";
  }

  const lines: string[] = [];
  for (let idx = 0; idx < events.length; idx++) {
    const eventName = events[idx];
    const info = metadata[idx];
    if (!info) {
      lines.push(`- ${eventName}: metadata unavailable.`);
      continue;
    }

    const sample = info.sampleCommands.slice(0, 6).join(", ");
    const lifecycle = info.lifecycle
      ? `, declaredBigipLifecycle=${info.lifecycle.introduced ?? "-"}..${info.lifecycle.retired ?? "-"}`
      : "";
    lines.push(
      `- ${eventName}: known=${info.known ? "yes" : "no"}${lifecycle}, validCommands=${info.validCommandCount}` +
        (sample ? `, sample=${sample}` : ""),
    );
  }

  return [
    `iRules event metadata from the standalone registry catalogue (${events.length}; not checked against the document's BIG-IP release):`,
    ...lines,
  ].join("\n");
}

async function buildPromptContextWithReport(
  ctx: CommandContext,
  options: PromptContextOptions = {},
): Promise<{
  block: string;
  diagnosticsReport?: CurrentDiagnosticsReport;
  assertCurrent?: () => void;
}> {
  const diagnosticDeadline =
    Date.now() + (options.diagnosticsTimeoutMs ?? DEFAULT_DIAGNOSTIC_TIMEOUT_MS);
  const remainingDiagnosticBudget = () => Math.max(0, diagnosticDeadline - Date.now());
  const allowAmbientContext = options.allowAmbientContext ?? true;
  const includeDiagnostics = options.includeDiagnostics ?? true;
  const includeSymbolDefinitions = options.includeSymbolDefinitions ?? true;
  const includeEventMetadata = options.includeEventMetadata ?? true;

  const document = await resolveContextDocument(ctx, options.document, allowAmbientContext);
  // Retain the actual document image and resource input before any provider await.
  // A detached code proposal remains source-only; a configured dialect is a hint,
  // while only a completed resource report supplies an analysed dialect.
  const snapshot = document ? snapshotDocumentSource(document) : undefined;
  const analysisContext = snapshot?.analysisContext ?? captureAnalysisContext();
  const configuration = document ? resourceConfiguration(document) : undefined;
  const assertCurrent = () => {
    if (
      (snapshot && !isDocumentSourceSnapshotCurrent(snapshot)) ||
      (document && resourceConfiguration(document) !== configuration) ||
      (analysisContext && !isAnalysisContextCurrent(analysisContext)) ||
      (options.isSourceCurrent && !options.isSourceCurrent())
    ) {
      throw new Error("The captured prompt source or analysis input changed");
    }
  };
  const code = await resolveContextCode(ctx, options.code, document, allowAmbientContext);
  assertCurrent();
  if (!document && !code) {
    return { block: "" };
  }

  let diagnosticsReport = options.completedDiagnosticsReport;
  let diagnosticsText: string | undefined;
  if (diagnosticsReport) {
    if (
      !document ||
      code === undefined ||
      !currentDiagnosticsReportMatches(diagnosticsReport, {
        document,
        uri: document.uri,
        version: document.version,
        source: code,
      })
    ) {
      throw new Error("The supplied completed diagnostics report does not match this source");
    }
  }
  if (includeDiagnostics && !diagnosticsReport) {
    const diagnosticSection = await buildDiagnosticsSection(
      document,
      code,
      options.maxDiagnostics ?? DEFAULT_MAX_DIAGNOSTICS,
      remainingDiagnosticBudget(),
    );
    diagnosticsReport = diagnosticSection.report;
    diagnosticsText = diagnosticSection.text;
  }

  const summary: string[] = ["Deterministic context (Tcl LSP):"];
  summary.push(
    diagnosticsReport
      ? `- Completed resource analysis: ${diagnosticsReport.response.dialectId} (source version ${diagnosticsReport.response.version})`
      : options.sourceContextLabel
        ? `- Captured source: ${options.sourceContextLabel}`
        : snapshot
          ? `- Captured resource dialect hint (not completed analysis): ${snapshot.resourceDialectHint}`
          : "- Detached source: no resource analysis dialect is available",
  );
  if (document) {
    summary.push(`- File: ${formatDocumentLabel(document)}`);
  }
  if (code) {
    summary.push(`- Code lines: ${code.split("\n").length}`);
  }

  const sections: string[] = [summary.join("\n")];

  if (includeDiagnostics && diagnosticsReport) {
    if (diagnosticsText !== undefined) {
      sections.push(diagnosticsText);
    } else {
      const diagnostics = [...diagnosticsReport.diagnostics];
      const actionable = diagnostics
        .filter((diagnostic) => diagnostic.severity <= vscode.DiagnosticSeverity.Warning)
        .sort((left, right) => {
          const lineDiff = left.range.start.line - right.range.start.line;
          return lineDiff !== 0
            ? lineDiff
            : left.range.start.character - right.range.start.character;
        });
      if (actionable.length === 0) {
        sections.push(
          `Diagnostics: no errors/warnings (completed ${diagnosticsReport.response.dialectId} report for source version ${diagnosticsReport.response.version}).`,
        );
      } else {
        const visible = actionable.slice(0, options.maxDiagnostics ?? DEFAULT_MAX_DIAGNOSTICS);
        const lines = visible.map((diagnostic) => {
          const code = diagnosticCode(diagnostic) || "unknown";
          return (
            `- [${code}] line ${diagnostic.range.start.line + 1} (${diagnosticSeverity(diagnostic.severity)}): ` +
            diagnostic.message.replace(/\s+/g, " ").trim()
          );
        });
        if (actionable.length > visible.length) {
          lines.push(`- ... and ${actionable.length - visible.length} more issue(s).`);
        }
        sections.push([`Diagnostics (${actionable.length} actionable):`, ...lines].join("\n"));
      }
    }
  } else if (includeDiagnostics && diagnosticsText !== undefined) {
    sections.push(diagnosticsText);
  }

  const sourceSymbols =
    code && (includeSymbolDefinitions || includeEventMetadata)
      ? await extractSourceSymbolDefinitions(code, Number.POSITIVE_INFINITY, document)
      : undefined;
  if (includeSymbolDefinitions && code) {
    sections.push(
      buildSymbolSection(sourceSymbols?.slice(0, options.maxSymbols ?? DEFAULT_MAX_SYMBOLS)),
    );
  }

  if (includeEventMetadata && code && !sourceSymbols) {
    sections.push("Event metadata: unavailable.");
  }
  if (includeEventMetadata && sourceSymbols) {
    const events = detectIruleEvents(sourceSymbols, options.maxEvents ?? DEFAULT_MAX_EVENTS);
    if (events.length > 0) {
      const metadata = await collectEventMetadata(ctx, events);
      sections.push(buildEventSection(events, metadata));
    }
  }

  assertCurrent();
  if (
    diagnosticsReport &&
    !(await validateCurrentDiagnosticsPublication(
      diagnosticsReport,
      {
        document: diagnosticsReport.document,
        uri: diagnosticsReport.document.uri,
        version: diagnosticsReport.response.version,
        source: diagnosticsReport.response.source,
      },
      remainingDiagnosticBudget(),
    ))
  ) {
    throw new Error("The completed report was retired while prompt context was packed");
  }
  assertCurrent();
  return {
    assertCurrent,
    block: [
      "Use this deterministic project/editor context when generating the answer:",
      ...sections,
    ].join("\n\n"),
    diagnosticsReport,
  };
}

export async function buildPromptContextBlock(
  ctx: CommandContext,
  options: PromptContextOptions = {},
): Promise<string> {
  return (await buildPromptContextWithReport(ctx, options)).block;
}

async function buildContextualMessagesWithReport(
  ctx: CommandContext,
  prompt: string,
  options: PromptContextOptions = {},
): Promise<{
  messages: vscode.LanguageModelChatMessage[];
  diagnosticsReport?: CurrentDiagnosticsReport;
  assertCurrent?: () => void;
}> {
  const packed = await buildPromptContextWithReport(ctx, options);
  const userPrompt = packed.block ? `${prompt.trimEnd()}\n\n${packed.block}` : prompt;
  const systemPrompt = packed.diagnosticsReport
    ? buildPrompt(packed.diagnosticsReport.response.dialectId)
    : ctx.systemPrompt;
  return {
    messages: [
      vscode.LanguageModelChatMessage.User(systemPrompt),
      vscode.LanguageModelChatMessage.User(userPrompt),
    ],
    diagnosticsReport: packed.diagnosticsReport,
    assertCurrent: packed.assertCurrent,
  };
}

export async function buildContextualMessages(
  ctx: CommandContext,
  prompt: string,
  options: PromptContextOptions = {},
): Promise<vscode.LanguageModelChatMessage[]> {
  return (await buildContextualMessagesWithReport(ctx, prompt, options)).messages;
}

/**
 * Resolve a concrete language model.
 *
 * The chat-panel model (`request.model`) may be a synthetic "auto" selector
 * that has no real backend endpoint.  When that happens `sendRequest` throws
 * "Endpoint not found for model auto".  We avoid this by checking whether
 * the preferred model appears in the list returned by
 * `vscode.lm.selectChatModels()`.  If it doesn't, we pick the first
 * concrete model that is available.
 */
async function resolveModel(
  preferred: vscode.LanguageModelChat,
): Promise<vscode.LanguageModelChat> {
  const available = await vscode.lm.selectChatModels();
  if (available.length === 0) {
    // No models enumerated — return preferred and let the caller handle
    // any access-consent dialog that may appear.
    return preferred;
  }

  // If the preferred model is among the concrete models, use it.
  const exact = available.find((m) => m.id === preferred.id);
  if (exact) {
    return exact;
  }

  // Preferred model not in the concrete list (e.g. id is "auto"). The order of
  // `selectChatModels()` is not specified, so taking [0] can land on an old,
  // small model while a far more capable one is right there. Prefer the
  // largest context window instead: it is a provider-agnostic proxy for the
  // newer model, and it does not rot the way a hardcoded model name would.
  const byCapacity = [...available].sort(
    (a, b) => (b.maxInputTokens ?? 0) - (a.maxInputTokens ?? 0),
  );
  return byCapacity[0] ?? available[0];
}

export async function sendContextualRequest(
  ctx: CommandContext,
  prompt: string,
  options: PromptContextOptions = {},
): Promise<vscode.LanguageModelChatResponse> {
  const built = await buildContextualMessagesWithReport(ctx, prompt, options);
  const messages = built.messages;
  const assertSourceCurrent = async () => {
    built.assertCurrent?.();
    if (options.isSourceCurrent && !options.isSourceCurrent()) {
      throw new Error("The captured source changed before the model request could be sent");
    }
    const report = built.diagnosticsReport;
    if (
      report &&
      !(await validateCurrentDiagnosticsPublication(report, {
        document: report.document,
        uri: report.document.uri,
        version: report.response.version,
        source: report.response.source,
      }))
    ) {
      throw new Error(
        "The completed diagnostics report is no longer current for its source/session",
      );
    }
    // The publisher check itself awaits: rejoin the original host image/input.
    built.assertCurrent?.();
  };
  await assertSourceCurrent();
  const sendOpts = {
    justification:
      "The iRule Assistant uses the language model to generate, explain, and analyse iRule code.",
  };

  const model = await resolveModel(ctx.request.model);
  await assertSourceCurrent();

  try {
    const response = await model.sendRequest(messages, sendOpts, ctx.token);
    await assertSourceCurrent();
    return response;
  } catch (firstErr) {
    if (ctx.token.isCancellationRequested) {
      throw firstErr;
    }
    // Try every other available model in turn.
    const models = await vscode.lm.selectChatModels();
    for (const candidate of models) {
      if (candidate.id === model.id) continue;
      await assertSourceCurrent();
      try {
        const response = await candidate.sendRequest(messages, sendOpts, ctx.token);
        await assertSourceCurrent();
        return response;
      } catch {
        await assertSourceCurrent();
        continue;
      }
    }
    throw firstErr;
  }
}
