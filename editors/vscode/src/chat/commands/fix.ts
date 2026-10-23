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
import { proposalReportEvidence, runAgenticLoop } from "../agenticLoop";
import { documentWriteReceipt, snapshotDocumentSource } from "../codeUtils";
import { waitForProposalDiagnostics, renderDiagnosticSection } from "../diagnosticAccess";
import { isTclLanguage } from "../../extension";

export async function handleFix(ctx: CommandContext): Promise<vscode.ChatResult> {
  const editor = vscode.window.activeTextEditor;
  if (!editor || !isTclLanguage(editor.document.languageId)) {
    ctx.response.markdown("Open an iRule or Tcl file to use `/fix`.");
    return {};
  }

  const sourceSnapshot = snapshotDocumentSource(editor.document);
  const code = sourceSnapshot.fullSource;
  const uri = editor.document.uri;

  // Check current diagnostics
  ctx.response.progress("Checking current diagnostics...");
  const initialReport = await waitForProposalDiagnostics(uri, code, {
    timeout: 5000,
    expectedSource: code,
  });
  const actionable = initialReport.diagnostics.filter(
    (d) => d.severity <= vscode.DiagnosticSeverity.Warning,
  );

  if (actionable.length === 0) {
    ctx.response.markdown("No errors or warnings found in the current document.");
    return { metadata: { command: "fix", iterations: 0, clean: true } };
  }

  ctx.response.markdown(`Found **${actionable.length}** issue(s). Starting fix loop...\n`);

  // Run agentic loop using the existing document
  const result = await runAgenticLoop(ctx, code, editor.document, {
    sourceSnapshot,
    initialProposalReport: initialReport,
  });

  // Present result
  ctx.response.markdown(
    `## Fixed Tcl source (${initialReport.response.dialectId})\n\n\`\`\`tcl\n${result.finalCode}\n\`\`\`\n`,
  );

  if (result.proposalReport) {
    const report = result.proposalReport.response;
    ctx.response.markdown(
      `\nDiagnostics were captured for the proposal at ${report.uri}, source version ${report.version}.`,
    );
  }
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

  // Offer to apply only against the completed document snapshot.
  if (!result.documentSnapshot) throw new Error("Fix result has no current document snapshot");
  const writeReceipt = documentWriteReceipt(result.documentSnapshot);
  ctx.response.button({
    command: "tclLsp.applyFix",
    title: "Apply fixes to document",
    arguments: [
      result.finalCode,
      writeReceipt,
      proposalReportEvidence(result.proposalReport, writeReceipt),
    ],
  });

  return {
    metadata: { command: "fix", iterations: result.iterations, clean: result.clean },
  };
}
