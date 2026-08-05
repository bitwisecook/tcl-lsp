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
import { CommandContext } from "./types";
import { extractCodeBlock, ensureDocumentOpen } from "./codeUtils";
import { sendContextualRequest } from "./contextPack";
import { waitForDiagnostics, formatDiagnosticsForLLM } from "./diagnosticAccess";
import { setSessionDialectOverride, getActiveDialect } from "../extension";

const MAX_ITERATIONS = 5;
const DIAGNOSTIC_WAIT_MS = 5000;

export interface AgenticResult {
  finalCode: string;
  iterations: number;
  remainingDiagnostics: vscode.Diagnostic[];
  clean: boolean;
}

export interface AgenticLoopOptions {
  targetDialect?: string;
}

/**
 * Run the generate-validate-iterate loop.
 *
 * Writes code to a document, waits for LSP diagnostics, feeds issues back to
 * the LLM for correction, and repeats until clean or MAX_ITERATIONS reached.
 */
export async function runAgenticLoop(
  ctx: CommandContext,
  initialCode: string,
  existingDoc?: vscode.TextDocument,
  options: AgenticLoopOptions = {},
): Promise<AgenticResult> {
  // Keep caller control over dialect. iRules workflows pin to f5-irules,
  // while general Tcl workflows keep the active dialect. Pinned as a *session
  // override* rather than a configuration push: a push is undone by the next
  // `workspace/configuration` pull, which can happen at any time (issue #1217).
  const targetDialect = options.targetDialect;
  const switchedDialect = !!targetDialect && getActiveDialect() !== targetDialect;
  if (switchedDialect) {
    await setSessionDialectOverride(targetDialect);
  }

  let currentCode = initialCode;
  let doc = existingDoc;
  let iteration = 0;
  let previousCount = Infinity;

  try {
    for (iteration = 0; iteration < MAX_ITERATIONS; iteration++) {
      if (ctx.token.isCancellationRequested) break;

      // Write current code to document
      doc = await ensureDocumentOpen(currentCode, doc);

      // Wait for LSP diagnostics
      ctx.response.progress(`Iteration ${iteration + 1}/${MAX_ITERATIONS}: analysing with LSP...`);
      const diagnostics = await waitForDiagnostics(doc.uri, {
        timeout: DIAGNOSTIC_WAIT_MS,
      });

      // Filter to actionable issues (errors and warnings only)
      const actionable = diagnostics.filter((d) => d.severity <= vscode.DiagnosticSeverity.Warning);

      if (actionable.length === 0) {
        ctx.response.progress("Clean -- no errors or warnings.");
        return {
          finalCode: currentCode,
          iterations: iteration + 1,
          remainingDiagnostics: [],
          clean: true,
        };
      }

      // Early exit if diagnostics are not decreasing (stuck)
      if (actionable.length >= previousCount && iteration > 0) {
        ctx.response.progress(`Issue count not decreasing (${actionable.length}). Stopping.`);
        break;
      }
      previousCount = actionable.length;

      // Format diagnostics for the LLM
      ctx.response.progress(`Found ${actionable.length} issue(s). Asking LLM to fix...`);
      const diagnosticReport = formatDiagnosticsForLLM(actionable, currentCode);

      // Ask LLM to fix
      const llmResponse = await sendContextualRequest(
        ctx,
        `Here is the current Tcl code:\n\`\`\`tcl\n${currentCode}\n\`\`\`\n\n` +
          `The Tcl LSP found these issues:\n${diagnosticReport}\n\n` +
          `Fix all the issues listed above. Return ONLY the complete corrected Tcl code in a single \`\`\`tcl code block. Do not include any explanation outside the code block.`,
        {
          allowAmbientContext: false,
          code: currentCode,
          document: doc,
          includeDiagnostics: false,
        },
      );

      let responseText = "";
      for await (const chunk of llmResponse.text) {
        responseText += chunk;
      }

      const extractedCode = extractCodeBlock(responseText);
      if (!extractedCode) {
        ctx.response.progress("LLM did not return a code block. Stopping.");
        break;
      }

      currentCode = extractedCode;
    }
  } finally {
    // Clearing the override restores whatever the configuration resolves to,
    // so there is no previous value to remember.
    if (switchedDialect) {
      await setSessionDialectOverride(null);
    }
  }

  // Final diagnostic check
  if (doc) {
    doc = await ensureDocumentOpen(currentCode, doc);
    const finalDiags = await waitForDiagnostics(doc.uri, { timeout: DIAGNOSTIC_WAIT_MS });
    const remaining = finalDiags.filter((d) => d.severity <= vscode.DiagnosticSeverity.Warning);
    return {
      finalCode: currentCode,
      iterations: iteration,
      remainingDiagnostics: remaining,
      clean: remaining.length === 0,
    };
  }

  return {
    finalCode: currentCode,
    iterations: iteration,
    remainingDiagnostics: [],
    clean: false,
  };
}
