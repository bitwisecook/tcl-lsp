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
import { resolveIruleCode, ensureDocumentOpen } from "../codeUtils";
import { sendContextualRequest } from "../contextPack";
import {
  waitForDiagnostics,
  categoriseDiagnostics,
  formatDiagnosticsForLLM,
} from "../diagnosticAccess";
import { setSessionDialectOverride, getActiveDialect, isTclLanguage } from "../../extension";

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
  // Pinned as a *session override*, not a configuration push: a push is
  // re-applied away by the next `workspace/configuration` pull, which can land
  // at any time, so the pin's lifetime was arbitrary (issue #1217).
  const pinnedDialect = getActiveDialect() !== "f5-irules";
  if (pinnedDialect) {
    await setSessionDialectOverride("f5-irules");
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
    if (pinnedDialect) {
      // Clearing restores whatever the configuration resolves to.
      await setSessionDialectOverride(null);
    }
  }
}
