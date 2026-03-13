import * as vscode from "vscode";
import { CommandContext } from "../types";
import { extractCodeBlock } from "../codeUtils";
import { sendContextualRequest } from "../contextPack";
import { runAgenticLoop } from "../agenticLoop";
import { renderDiagnosticSection } from "../diagnosticAccess";

function buildFallbackCreateIrule(description: string): string {
  const summary = description.replace(/\s+/g, " ").trim();
  return [
    "when HTTP_REQUEST {",
    "    # Fallback starter generated because the model response was malformed.",
    `    # Request: ${summary || "Create a secure iRule."}`,
    '    if {[HTTP::uri] eq "/old"} {',
    '        HTTP::redirect "https://[HTTP::host]/new"',
    "        return",
    "    }",
    '    log local0.notice "request [HTTP::method] [HTTP::host][HTTP::uri]"',
    "}",
    "",
  ].join("\n");
}

export async function handleCreate(ctx: CommandContext): Promise<vscode.ChatResult> {
  const description = ctx.request.prompt;
  if (!description.trim()) {
    ctx.response.markdown(
      "Please describe the iRule you want to create. For example:\n\n" +
        "> `@irule /create Redirect HTTP to HTTPS preserving the path and query string`",
    );
    return {};
  }

  ctx.response.progress("Generating iRule...");

  // Step 1: Ask LLM to generate initial iRule
  const llmResponse = await sendContextualRequest(
    ctx,
    `Create an F5 BIG-IP iRule that does the following:\n\n${description}\n\n` +
      `Requirements:\n` +
      `- Use appropriate event handlers (when blocks)\n` +
      `- Follow security best practices (braced expressions, option terminators, no eval with user data)\n` +
      `- Include comments explaining the logic\n` +
      `- Return ONLY the complete iRule code in a \`\`\`tcl code block`,
    { allowAmbientContext: false },
  );
  let responseText = "";
  for await (const chunk of llmResponse.text) {
    responseText += chunk;
  }

  let initialCode = extractCodeBlock(responseText);
  if (!initialCode) {
    const trimmed = responseText.trim();
    if (trimmed.startsWith("when ") || trimmed.includes("\nwhen ")) {
      initialCode = trimmed;
    }
  }
  if (!initialCode) {
    ctx.response.markdown(
      "The model response did not contain a valid iRule code block. " +
        "Using a safe fallback template and validating it now.",
    );
    initialCode = buildFallbackCreateIrule(description);
  }

  // Step 2: Run agentic validation loop
  const result = await runAgenticLoop(ctx, initialCode, undefined, { targetDialect: "f5-irules" });

  // Step 3: Present result
  ctx.response.markdown(`## Generated iRule\n\n\`\`\`tcl\n${result.finalCode}\n\`\`\`\n`);

  if (result.clean) {
    ctx.response.markdown(
      `\nValidated clean in ${result.iterations} iteration(s). No errors or warnings.`,
    );
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

  // Step 4: Offer to insert into editor
  ctx.response.button({
    command: "tclLsp.insertIrule",
    title: "Insert into new file",
    arguments: [result.finalCode],
  });

  return {
    metadata: { command: "create", iterations: result.iterations, clean: result.clean },
  };
}
