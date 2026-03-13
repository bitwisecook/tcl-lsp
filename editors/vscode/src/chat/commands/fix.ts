import * as vscode from "vscode";
import { CommandContext } from "../types";
import { runAgenticLoop } from "../agenticLoop";
import { waitForDiagnostics, renderDiagnosticSection } from "../diagnosticAccess";
import { isTclLanguage } from "../../extension";

export async function handleFix(ctx: CommandContext): Promise<vscode.ChatResult> {
  const editor = vscode.window.activeTextEditor;
  if (!editor || !isTclLanguage(editor.document.languageId)) {
    ctx.response.markdown("Open an iRule or Tcl file to use `/fix`.");
    return {};
  }

  const code = editor.document.getText();
  const uri = editor.document.uri;

  // Check current diagnostics
  ctx.response.progress("Checking current diagnostics...");
  const diagnostics = await waitForDiagnostics(uri, { timeout: 5000 });
  const actionable = diagnostics.filter((d) => d.severity <= vscode.DiagnosticSeverity.Warning);

  if (actionable.length === 0) {
    ctx.response.markdown("No errors or warnings found in the current document.");
    return { metadata: { command: "fix", iterations: 0, clean: true } };
  }

  ctx.response.markdown(`Found **${actionable.length}** issue(s). Starting fix loop...\n`);

  // Run agentic loop using the existing document
  const result = await runAgenticLoop(ctx, code, editor.document, { targetDialect: "f5-irules" });

  // Present result
  ctx.response.markdown(`## Fixed iRule\n\n\`\`\`tcl\n${result.finalCode}\n\`\`\`\n`);

  if (result.clean) {
    ctx.response.markdown(`\nAll issues resolved in ${result.iterations} iteration(s).`);
  } else {
    ctx.response.markdown(
      `\nAfter ${result.iterations} iteration(s), ${result.remainingDiagnostics.length} issue(s) remain:\n`,
    );
    renderDiagnosticSection(
      ctx.response,
      "Remaining Issues",
      result.remainingDiagnostics,
      result.finalCode,
    );
  }

  // Offer to apply
  ctx.response.button({
    command: "tclLsp.applyFix",
    title: "Apply fixes to document",
    arguments: [result.finalCode, uri.toString()],
  });

  return {
    metadata: { command: "fix", iterations: result.iterations, clean: result.clean },
  };
}
