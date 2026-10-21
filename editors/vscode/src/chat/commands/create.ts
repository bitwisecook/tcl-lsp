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
import { CommandContext } from "../types";
import { extractCodeBlock } from "../codeUtils";
import { sendContextualRequest } from "../contextPack";
import { AgenticResult, runAgenticLoop } from "../agenticLoop";
import { renderDiagnosticSection } from "../diagnosticAccess";

/**
 * A new authored starter for when the model returns no code: a fixed HTTP
 * template whose commands are an unvalidated recommendation, not a check
 * against the user's BIG-IP release. It names no existing rule or handler.
 */
function buildFallbackCreateIrule(description: string): string {
  const summary = description.replace(/\s+/g, " ").trim();
  return [
    "when HTTP_REQUEST {",
    "    # Fallback starter generated because the model response was malformed.",
    "    # Authored template: confirm these commands against your BIG-IP release.",
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

/**
 * What a clean loop result establishes: the completed analysis of this exact
 * source reported no errors or warnings. Static analysis only — no BIG-IP ran
 * it — and without a completed report nothing is claimed.
 */
export function cleanReportLine(result: AgenticResult): string {
  const dialect = result.finalDiagnosticsReport?.response.dialectId;
  if (!dialect) {
    return `No errors or warnings were reported after ${result.iterations} iteration(s), but no completed analysis report confirms it; review the source before use.`;
  }
  return (
    `The language server's completed ${dialect} analysis of this exact source reported ` +
    `no errors or warnings after ${result.iterations} iteration(s). This is static ` +
    `analysis only; the iRule has not been run on a BIG-IP.`
  );
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

  const result = await runAgenticLoop(ctx, initialCode, undefined, { targetDialect: "f5-irules" });

  ctx.response.markdown(`## Generated iRule\n\n\`\`\`tcl\n${result.finalCode}\n\`\`\`\n`);

  if (result.clean) {
    ctx.response.markdown(`\n${cleanReportLine(result)}`);
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

  ctx.response.button({
    command: "tclLsp.insertIrule",
    title: "Insert into new file",
    arguments: [result.finalCode],
  });

  return {
    metadata: { command: "create", iterations: result.iterations, clean: result.clean },
  };
}
