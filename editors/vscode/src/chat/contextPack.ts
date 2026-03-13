import * as path from "path";
import * as vscode from "vscode";
import { isTclLanguage } from "../extension";
import { resolveIruleCode } from "./codeUtils";
import { waitForDiagnostics } from "./diagnosticAccess";
import { IRULES_EVENT_PATTERN } from "./canonicalDiagnostics";
import { CommandContext } from "./types";

const DEFAULT_DIAGNOSTIC_TIMEOUT_MS = 2500;
const DEFAULT_MAX_DIAGNOSTICS = 8;
const DEFAULT_MAX_SYMBOLS = 15;
const DEFAULT_MAX_EVENTS = 6;

interface IruleEventInfo {
  event: string;
  known: boolean;
  deprecated: boolean;
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
  allowAmbientContext?: boolean;
  includeDiagnostics?: boolean;
  includeSymbolDefinitions?: boolean;
  includeEventMetadata?: boolean;
  diagnosticsTimeoutMs?: number;
  maxDiagnostics?: number;
  maxSymbols?: number;
  maxEvents?: number;
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

function activeDialect(): string {
  return vscode.workspace.getConfiguration("tclLsp").get<string>("dialect", "tcl8.6");
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

export function detectIruleEvents(source: string, maxEvents = DEFAULT_MAX_EVENTS): string[] {
  if (!source.trim()) {
    return [];
  }

  const events: string[] = [];
  const seen = new Set<string>();

  IRULES_EVENT_PATTERN.lastIndex = 0;
  for (const match of source.matchAll(IRULES_EVENT_PATTERN)) {
    const eventName = match[1];
    if (seen.has(eventName)) {
      continue;
    }
    seen.add(eventName);
    events.push(eventName);
    if (events.length >= maxEvents) {
      break;
    }
  }

  return events;
}

export function extractSourceSymbolDefinitions(
  source: string,
  maxSymbols = DEFAULT_MAX_SYMBOLS,
): SourceSymbolDefinition[] {
  const definitions: SourceSymbolDefinition[] = [];
  const lines = source.split("\n");

  for (let idx = 0; idx < lines.length; idx++) {
    const line = lines[idx];

    const procMatch = line.match(/^\s*proc\s+([^\s{}]+)\s+\{([^}]*)\}/);
    if (procMatch) {
      definitions.push({
        kind: "Function",
        name: procMatch[1],
        detail: `{${procMatch[2].trim()}}`,
        line: idx + 1,
      });
      if (definitions.length >= maxSymbols) {
        break;
      }
      continue;
    }

    const namespaceMatch = line.match(/^\s*namespace\s+eval\s+([^\s{}]+)/);
    if (namespaceMatch) {
      definitions.push({
        kind: "Namespace",
        name: namespaceMatch[1],
        line: idx + 1,
      });
      if (definitions.length >= maxSymbols) {
        break;
      }
      continue;
    }

    const eventMatch = line.match(/^\s*when\s+([A-Z][A-Z0-9_]{2,})\b/);
    if (eventMatch) {
      definitions.push({
        kind: "Event",
        name: eventMatch[1],
        line: idx + 1,
      });
      if (definitions.length >= maxSymbols) {
        break;
      }
    }
  }

  return definitions;
}

async function collectSymbolDefinitions(
  document: vscode.TextDocument | undefined,
  source: string,
  maxSymbols: number,
): Promise<SourceSymbolDefinition[]> {
  if (document) {
    try {
      const symbols = await vscode.commands.executeCommand<
        ReadonlyArray<vscode.DocumentSymbol | vscode.SymbolInformation> | undefined
      >("vscode.executeDocumentSymbolProvider", document.uri);

      if (symbols && symbols.length > 0) {
        const flattened: SourceSymbolDefinition[] = [];
        if (isDocumentSymbol(symbols[0])) {
          flattenDocumentSymbols(symbols as readonly vscode.DocumentSymbol[], flattened);
        } else {
          for (const symbol of symbols as readonly vscode.SymbolInformation[]) {
            const kindLabel = (vscode.SymbolKind[symbol.kind] ?? "Symbol") as string;
            flattened.push({
              kind: kindLabel,
              name: symbol.name,
              line: symbol.location.range.start.line + 1,
            });
          }
        }

        const unique: SourceSymbolDefinition[] = [];
        const seen = new Set<string>();
        for (const entry of flattened) {
          const key = `${entry.kind}:${entry.name}:${entry.line}`;
          if (seen.has(key)) {
            continue;
          }
          seen.add(key);
          unique.push(entry);
          if (unique.length >= maxSymbols) {
            break;
          }
        }

        if (unique.length > 0) {
          return unique;
        }
      }
    } catch {
      // Fall back to source-based extraction below.
    }
  }

  return extractSourceSymbolDefinitions(source, maxSymbols);
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
  maxDiagnostics: number,
  timeoutMs: number,
): Promise<string> {
  if (!document) {
    return "Diagnostics: unavailable (no open Tcl document URI).";
  }

  const diagnostics = await waitForDiagnostics(document.uri, {
    timeout: timeoutMs,
  });

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
    return "Diagnostics: no errors/warnings.";
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

  return [`Diagnostics (${actionable.length}):`, ...lines].join("\n");
}

function buildSymbolSection(symbols: readonly SourceSymbolDefinition[]): string {
  if (symbols.length === 0) {
    return "Symbol definitions: none detected.";
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
    lines.push(
      `- ${eventName}: known=${info.known ? "yes" : "no"}, deprecated=${info.deprecated ? "yes" : "no"}, validCommands=${info.validCommandCount}` +
        (sample ? `, sample=${sample}` : ""),
    );
  }

  return [`iRules event metadata (${events.length}):`, ...lines].join("\n");
}

export async function buildPromptContextBlock(
  ctx: CommandContext,
  options: PromptContextOptions = {},
): Promise<string> {
  const allowAmbientContext = options.allowAmbientContext ?? true;
  const includeDiagnostics = options.includeDiagnostics ?? true;
  const includeSymbolDefinitions = options.includeSymbolDefinitions ?? true;
  const includeEventMetadata = options.includeEventMetadata ?? true;

  const document = await resolveContextDocument(ctx, options.document, allowAmbientContext);
  const code = await resolveContextCode(ctx, options.code, document, allowAmbientContext);
  if (!document && !code) {
    return "";
  }

  const summary: string[] = ["Deterministic context (Tcl LSP):", `- Dialect: ${activeDialect()}`];
  if (document) {
    summary.push(`- File: ${formatDocumentLabel(document)}`);
  }
  if (code) {
    summary.push(`- Code lines: ${code.split("\n").length}`);
  }

  const sections: string[] = [summary.join("\n")];

  if (includeDiagnostics) {
    sections.push(
      await buildDiagnosticsSection(
        document,
        options.maxDiagnostics ?? DEFAULT_MAX_DIAGNOSTICS,
        options.diagnosticsTimeoutMs ?? DEFAULT_DIAGNOSTIC_TIMEOUT_MS,
      ),
    );
  }

  if (includeSymbolDefinitions && code) {
    const symbolDefs = await collectSymbolDefinitions(
      document,
      code,
      options.maxSymbols ?? DEFAULT_MAX_SYMBOLS,
    );
    sections.push(buildSymbolSection(symbolDefs));
  }

  if (includeEventMetadata && code) {
    const events = detectIruleEvents(code, options.maxEvents ?? DEFAULT_MAX_EVENTS);
    if (events.length > 0) {
      const metadata = await collectEventMetadata(ctx, events);
      sections.push(buildEventSection(events, metadata));
    }
  }

  return [
    "Use this deterministic project/editor context when generating the answer:",
    ...sections,
  ].join("\n\n");
}

export async function buildContextualMessages(
  ctx: CommandContext,
  prompt: string,
  options: PromptContextOptions = {},
): Promise<vscode.LanguageModelChatMessage[]> {
  const packedContext = await buildPromptContextBlock(ctx, options);
  const userPrompt = packedContext ? `${prompt.trimEnd()}\n\n${packedContext}` : prompt;

  return [
    vscode.LanguageModelChatMessage.User(ctx.systemPrompt),
    vscode.LanguageModelChatMessage.User(userPrompt),
  ];
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

  // Preferred model not in the concrete list (e.g. id is "auto").
  // Pick the first available model.
  return available[0];
}

export async function sendContextualRequest(
  ctx: CommandContext,
  prompt: string,
  options: PromptContextOptions = {},
): Promise<vscode.LanguageModelChatResponse> {
  const messages = await buildContextualMessages(ctx, prompt, options);
  const sendOpts = {
    justification:
      "The iRule Assistant uses the language model to generate, explain, and analyse iRule code.",
  };

  const model = await resolveModel(ctx.request.model);

  try {
    return await model.sendRequest(messages, sendOpts, ctx.token);
  } catch (firstErr) {
    if (ctx.token.isCancellationRequested) {
      throw firstErr;
    }
    // Try every other available model in turn.
    const models = await vscode.lm.selectChatModels();
    for (const candidate of models) {
      if (candidate.id === model.id) continue;
      try {
        return await candidate.sendRequest(messages, sendOpts, ctx.token);
      } catch {
        continue;
      }
    }
    throw firstErr;
  }
}
