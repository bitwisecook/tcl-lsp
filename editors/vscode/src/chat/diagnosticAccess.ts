import * as vscode from "vscode";
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
 * Poll for diagnostics on the given URI until minCount are available or timeout.
 * Combines event listening with periodic polling for robustness.
 */
export async function waitForDiagnostics(
  uri: vscode.Uri,
  opts?: { timeout?: number; minCount?: number },
): Promise<vscode.Diagnostic[]> {
  const timeout = opts?.timeout ?? 5000;
  const minCount = opts?.minCount ?? 0;

  // Trigger LSP analysis by requesting hover (serialises behind didOpen/didChange)
  await vscode.commands.executeCommand(
    "vscode.executeHoverProvider",
    uri,
    new vscode.Position(0, 0),
  );

  const immediate = vscode.languages.getDiagnostics(uri);
  if (immediate.length >= minCount && minCount > 0) {
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

    const timer = setTimeout(() => {
      finish(vscode.languages.getDiagnostics(uri));
    }, timeout);

    const disposable = vscode.languages.onDidChangeDiagnostics((e) => {
      const changed = e.uris.some((u) => u.toString() === uri.toString());
      if (changed) {
        const diags = vscode.languages.getDiagnostics(uri);
        if (minCount === 0 || diags.length >= minCount) {
          finish(diags);
        }
      }
    });

    const poller = setInterval(() => {
      const diags = vscode.languages.getDiagnostics(uri);
      if (minCount === 0 || diags.length >= minCount) {
        finish(diags);
      }
    }, 500);
  });
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
