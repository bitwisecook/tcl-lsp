import * as vscode from "vscode";
import { CommandContext } from "../types";
import { resolveIruleCode, ensureDocumentOpen, extractCodeBlock } from "../codeUtils";
import { sendContextualRequest } from "../contextPack";
import {
  waitForDiagnostics,
  formatDiagnosticsForLLM,
  renderDiagnosticSection,
} from "../diagnosticAccess";
import { runAgenticLoop } from "../agenticLoop";
import { setServerDialect, getActiveDialect, isTclLanguage } from "../../extension";

/** Diagnostic codes that represent convertible legacy patterns. */
const CONVERTIBLE_CODES = new Set([
  "W100", // unbraced expr
  "W104", // string concat for list building
  "W110", // == for strings instead of eq/ne
  "W304", // missing option terminator --
  "IRULE2001", // deprecated matchclass
  "IRULE5001", // ungated log in hot event
]);

function diagnosticCode(d: vscode.Diagnostic): string {
  if (typeof d.code === "object" && d.code !== null && "value" in d.code) {
    return String(d.code.value);
  }
  return String(d.code ?? "");
}

export async function handleConvert(ctx: CommandContext): Promise<vscode.ChatResult> {
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
    ctx.response.markdown("Open an iRule file or attach one with `#file` to modernise.");
    return {};
  }

  ctx.response.progress("Scanning for legacy patterns...");

  // Ensure dialect is f5-irules
  const previousDialect = getActiveDialect();
  if (previousDialect !== "f5-irules") {
    await setServerDialect("f5-irules");
  }

  try {
    if (!doc) {
      doc = await ensureDocumentOpen(code);
    }

    // Get diagnostics and filter to convertible patterns
    const diagnostics = await waitForDiagnostics(doc.uri, { timeout: 5000 });
    const convertible = diagnostics.filter((d) => CONVERTIBLE_CODES.has(diagnosticCode(d)));

    if (convertible.length === 0) {
      ctx.response.markdown(
        "No legacy patterns detected. The iRule already follows current best practices.",
      );
      return { metadata: { command: "convert", count: 0 } };
    }

    ctx.response.progress(`Found ${convertible.length} legacy pattern(s). Modernising...`);

    const diagnosticReport = formatDiagnosticsForLLM(convertible, code);

    // Ask LLM to modernise
    const llmResponse = await sendContextualRequest(
      ctx,
      `Modernise this iRule by converting all legacy patterns identified below.\n\n` +
        `Current code:\n\`\`\`tcl\n${code}\n\`\`\`\n\n` +
        `Legacy patterns found by static analysis:\n${diagnosticReport}\n\n` +
        `Apply these conversions:\n` +
        `- matchclass → class match (IRULE2001)\n` +
        `- Unbraced expr → braced expr (W100)\n` +
        `- String concat for lists → lappend (W104)\n` +
        `- == / != for strings → eq / ne (W110)\n` +
        `- Missing -- option terminator → add -- (W304)\n` +
        `- Ungated log → add CLIENT_ACCEPTED { set debug 0 } and wrap with if {$debug} (IRULE5001)\n\n` +
        `Return ONLY the complete modernised iRule in a single \`\`\`tcl code block.`,
      { code, document: doc },
    );
    let responseText = "";
    for await (const chunk of llmResponse.text) {
      responseText += chunk;
    }

    const modernised = extractCodeBlock(responseText);
    if (!modernised) {
      ctx.response.markdown(
        "Failed to extract modernised code from LLM response.\n\n" + responseText,
      );
      return {};
    }

    // Run agentic loop to validate
    const result = await runAgenticLoop(ctx, modernised, doc, { targetDialect: "f5-irules" });

    ctx.response.markdown(`## Modernised iRule\n\n\`\`\`tcl\n${result.finalCode}\n\`\`\`\n`);

    if (result.clean) {
      ctx.response.markdown(
        `\nConverted ${convertible.length} legacy pattern(s). Validated clean in ${result.iterations} iteration(s).`,
      );
    } else {
      ctx.response.markdown(
        `\nConverted ${convertible.length} legacy pattern(s). After ${result.iterations} iteration(s), ${result.remainingDiagnostics.length} issue(s) remain:\n`,
      );
      renderDiagnosticSection(
        ctx.response,
        "Remaining Issues",
        result.remainingDiagnostics,
        result.finalCode,
      );
    }

    ctx.response.button({
      command: "tclLsp.applyFix",
      title: "Apply modernised code",
      arguments: [result.finalCode, doc.uri.toString()],
    });

    return { metadata: { command: "convert", count: convertible.length, clean: result.clean } };
  } finally {
    if (previousDialect !== "f5-irules") {
      await setServerDialect(previousDialect);
    }
  }
}
