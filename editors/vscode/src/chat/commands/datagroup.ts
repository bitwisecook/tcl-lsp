import * as vscode from "vscode";
import { CommandContext } from "../types";
import { resolveIruleCode, ensureDocumentOpen } from "../codeUtils";
import { sendContextualRequest } from "../contextPack";
import {
  waitForDiagnostics,
  categoriseDiagnostics,
  formatDiagnosticsForLLM,
} from "../diagnosticAccess";
import { setServerDialect, getActiveDialect, isTclLanguage } from "../../extension";

export async function handleDatagroup(ctx: CommandContext): Promise<vscode.ChatResult> {
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
    ctx.response.markdown(
      "Open an iRule file or attach one with `#file` to analyse for data-group opportunities.",
    );
    return {};
  }

  ctx.response.progress("Analysing for data-group extraction opportunities...");

  // Ensure dialect is f5-irules
  const previousDialect = getActiveDialect();
  if (previousDialect !== "f5-irules") {
    await setServerDialect("f5-irules");
  }

  try {
    if (!doc) {
      doc = await ensureDocumentOpen(code);
    }

    // Get diagnostics — look for performance-related issues
    const diagnostics = await waitForDiagnostics(doc.uri, { timeout: 5000 });
    const categorised = categoriseDiagnostics(diagnostics);
    const perfDiags = [...categorised.performance, ...categorised.style];

    let lspContext = "";
    if (perfDiags.length > 0) {
      lspContext =
        `\n\nThe LSP also found these related issues:\n` + formatDiagnosticsForLLM(perfDiags, code);
    }

    // Ask LLM for data-group analysis
    ctx.response.markdown("## Data-Group Analysis\n\n");
    const llmResponse = await sendContextualRequest(
      ctx,
      `Analyse this iRule for opportunities to extract inline lookup patterns into data-groups.\n\n` +
        `\`\`\`tcl\n${code}\n\`\`\`${lspContext}\n\n` +
        `Look for:\n` +
        `1. Large switch statements mapping strings to values — replace with \`class match\` or \`class lookup\`\n` +
        `2. Chains of if/elseif matching string patterns — replace with data-group + \`class match\`\n` +
        `3. Repeated regexp patterns for classification — replace with data-group + \`class match -glob\`\n` +
        `4. Inline IP address lists — replace with address-type data-groups\n` +
        `5. Any matchclass usage — modernise to \`class match\`\n\n` +
        `For each candidate:\n` +
        `- Show the current inline code\n` +
        `- Show the replacement using \`class match\` / \`class lookup\`\n` +
        `- Provide the TMSH command to create the data-group: \`tmsh create ltm data-group internal <name> type <type> records add { ... }\`\n` +
        `- Explain the performance benefit\n\n` +
        `If no data-group opportunities exist, say so and explain why the current approach is acceptable.`,
      { code, document: doc },
    );
    for await (const chunk of llmResponse.text) {
      ctx.response.markdown(chunk);
    }

    return { metadata: { command: "datagroup" } };
  } finally {
    if (previousDialect !== "f5-irules") {
      await setServerDialect(previousDialect);
    }
  }
}
