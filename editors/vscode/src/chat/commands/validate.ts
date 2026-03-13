import * as vscode from "vscode";
import { CommandContext } from "../types";
import { resolveIruleCode, ensureDocumentOpen } from "../codeUtils";
import {
  waitForDiagnostics,
  categoriseDiagnostics,
  renderDiagnosticSection,
} from "../diagnosticAccess";
import { setServerDialect, getActiveDialect, isTclLanguage } from "../../extension";

export async function handleValidate(ctx: CommandContext): Promise<vscode.ChatResult> {
  // Try active editor first, then resolve from references
  let doc: vscode.TextDocument | undefined;
  let code: string | undefined;

  const editor = vscode.window.activeTextEditor;
  if (editor && isTclLanguage(editor.document.languageId)) {
    doc = editor.document;
    code = doc.getText();
  } else {
    code = await resolveIruleCode(ctx);
  }

  if (!code) {
    ctx.response.markdown("Open an iRule file or attach one with `#file` to validate.");
    return {};
  }

  ctx.response.progress("Running LSP analysis...");

  // Ensure dialect is f5-irules
  const previousDialect = getActiveDialect();
  if (previousDialect !== "f5-irules") {
    await setServerDialect("f5-irules");
  }

  try {
    if (!doc) {
      doc = await ensureDocumentOpen(code);
    }

    const diagnostics = await waitForDiagnostics(doc.uri, { timeout: 5000 });
    const categorised = categoriseDiagnostics(diagnostics);

    ctx.response.markdown("## iRule Validation Report\n");

    if (categorised.all.length === 0) {
      ctx.response.markdown("\nNo issues found. The iRule looks clean.\n");
      return { metadata: { command: "validate", count: 0 } };
    }

    renderDiagnosticSection(ctx.response, "Errors", categorised.errors, code);
    renderDiagnosticSection(ctx.response, "Security", categorised.security, code);
    renderDiagnosticSection(ctx.response, "Taint Analysis", categorised.taint, code);
    renderDiagnosticSection(ctx.response, "Thread Safety", categorised.threadSafety, code);
    renderDiagnosticSection(ctx.response, "Control Flow", categorised.controlFlow, code);
    renderDiagnosticSection(ctx.response, "Performance", categorised.performance, code);
    renderDiagnosticSection(ctx.response, "Style & Best Practice", categorised.style, code);
    renderDiagnosticSection(ctx.response, "Optimiser", categorised.optimiser, code);

    ctx.response.markdown(`\n**Summary**: ${categorised.all.length} issue(s) found.\n`);

    return { metadata: { command: "validate", count: categorised.all.length } };
  } finally {
    if (previousDialect !== "f5-irules") {
      await setServerDialect(previousDialect);
    }
  }
}
