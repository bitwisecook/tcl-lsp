import * as vscode from "vscode";
import { CommandContext } from "../types";
import { resolveIruleCode, ensureDocumentOpen } from "../codeUtils";
import { sendContextualRequest } from "../contextPack";
import {
  waitForDiagnostics,
  categoriseDiagnostics,
  formatDiagnosticsForLLM,
  renderDiagnosticSection,
} from "../diagnosticAccess";
import { setServerDialect, getActiveDialect, isTclLanguage } from "../../extension";

export async function handleReview(ctx: CommandContext): Promise<vscode.ChatResult> {
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
    ctx.response.markdown("Open an iRule file or attach one with `#file` for security review.");
    return {};
  }

  ctx.response.progress("Running security analysis...");

  // Ensure dialect is f5-irules
  const previousDialect = getActiveDialect();
  if (previousDialect !== "f5-irules") {
    await setServerDialect("f5-irules");
  }

  try {
    if (!doc) {
      doc = await ensureDocumentOpen(code);
    }

    // Step 1: Get LSP diagnostics
    const diagnostics = await waitForDiagnostics(doc.uri, { timeout: 5000 });
    const categorised = categoriseDiagnostics(diagnostics);

    // Step 2: Collect security-related diagnostics
    const securityDiags = [
      ...categorised.security,
      ...categorised.taint,
      ...categorised.threadSafety,
    ];

    const lspReport =
      securityDiags.length > 0
        ? formatDiagnosticsForLLM(securityDiags, code)
        : "No security issues detected by static analysis.";

    // Step 3: Ask LLM for deeper review
    ctx.response.progress("Running LLM security review...");

    // Step 4: Render combined report
    ctx.response.markdown("## Security Review\n");

    // Static analysis findings
    if (securityDiags.length > 0) {
      ctx.response.markdown("\n### Static Analysis Findings\n");
      renderDiagnosticSection(ctx.response, "Security", categorised.security, code);
      renderDiagnosticSection(ctx.response, "Taint Analysis", categorised.taint, code);
      renderDiagnosticSection(ctx.response, "Thread Safety", categorised.threadSafety, code);
    } else {
      ctx.response.markdown(
        "\n### Static Analysis\n\nNo security issues detected by static analysis.\n",
      );
    }

    // LLM analysis (streamed)
    ctx.response.markdown("\n### Deep Analysis\n\n");
    const llmResponse = await sendContextualRequest(
      ctx,
      `Perform a comprehensive security review of this iRule. ` +
        `The static analyser already found these issues:\n${lspReport}\n\n` +
        `Go beyond the static analysis. Check for:\n` +
        `- Input validation gaps\n` +
        `- Information leakage in logs or responses\n` +
        `- Race conditions with shared state\n` +
        `- Denial of service vectors (unbounded loops, expensive operations)\n` +
        `- Header injection possibilities\n` +
        `- Open redirect vulnerabilities\n` +
        `- Session handling issues\n\n` +
        `\`\`\`tcl\n${code}\n\`\`\``,
      { code, document: doc },
    );
    for await (const chunk of llmResponse.text) {
      ctx.response.markdown(chunk);
    }

    return { metadata: { command: "review", securityCount: securityDiags.length } };
  } finally {
    if (previousDialect !== "f5-irules") {
      await setServerDialect(previousDialect);
    }
  }
}
