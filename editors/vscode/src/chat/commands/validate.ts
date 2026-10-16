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
import { resolveIruleCode, ensureDocumentOpen, snapshotDocumentSource } from "../codeUtils";
import {
  acquireSessionDialectOverride,
  captureAnalysisContext,
  isAnalysisContextCurrent,
  waitForDiagnostics,
  waitForProposalDiagnostics,
  proposalDiagnosticsReportMatches,
  ProposalDiagnosticsReport,
  SessionDialectOverrideLease,
  categoriseDiagnostics,
  renderDiagnosticSection,
} from "../diagnosticAccess";
import { getActiveDialect, isTclLanguage } from "../../extension";

export async function handleValidate(ctx: CommandContext): Promise<vscode.ChatResult> {
  // Try active editor first, then resolve from references
  let doc: vscode.TextDocument | undefined;
  let code: string | undefined;

  const editor = vscode.window.activeTextEditor;
  const sourceSnapshot =
    editor && isTclLanguage(editor.document.languageId)
      ? snapshotDocumentSource(editor.document)
      : undefined;
  if (sourceSnapshot) {
    doc = sourceSnapshot.document;
    code = sourceSnapshot.fullSource;
  } else {
    code = await resolveIruleCode(ctx);
  }

  if (!code) {
    ctx.response.markdown("Open an iRule file or attach one with `#file` to validate.");
    return {};
  }

  ctx.response.progress("Running LSP analysis...");

  // Attachments still use the legacy path until their resolver provides an
  // admitted original document snapshot. An actual open document is analysed
  // in its resource context and does not acquire a session-wide dialect pin.
  const pinnedDialect = !sourceSnapshot && getActiveDialect() !== "f5-irules";
  let overrideLease: SessionDialectOverrideLease | undefined;
  if (pinnedDialect) overrideLease = await acquireSessionDialectOverride("f5-irules");

  try {
    if (!doc) {
      doc = await ensureDocumentOpen(code);
    }
    const analysisContext = sourceSnapshot
      ? sourceSnapshot.analysisContext
      : (overrideLease?.analysisContext ?? captureAnalysisContext());
    if (!analysisContext || !isAnalysisContextCurrent(analysisContext)) {
      throw new Error("Analysis context changed before validation started");
    }

    let proposalReport: ProposalDiagnosticsReport | undefined;
    let diagnostics: vscode.Diagnostic[];
    if (sourceSnapshot) {
      proposalReport = await waitForProposalDiagnostics(doc.uri, code, {
        timeout: 5000,
        expectedSource: sourceSnapshot.fullSource,
      });
      if (
        !proposalDiagnosticsReportMatches(proposalReport, {
          document: sourceSnapshot.document,
          uri: sourceSnapshot.uri,
          version: sourceSnapshot.version,
          source: sourceSnapshot.fullSource,
          proposal: code,
        })
      ) {
        throw new Error("The validation report no longer matches the captured document source");
      }
      diagnostics = [...proposalReport.diagnostics];
    } else {
      diagnostics = await waitForDiagnostics(doc.uri, { timeout: 5000, expectedSource: code });
    }
    if (!isAnalysisContextCurrent(analysisContext)) {
      throw new Error("Analysis context changed while validation was running");
    }
    const categorised = categoriseDiagnostics(diagnostics);
    const selectedDialect = proposalReport?.response.dialectId;
    const sourceVersion = proposalReport?.response.version;
    const reportContext = selectedDialect
      ? ` (${selectedDialect}, source version ${sourceVersion})`
      : "";

    ctx.response.markdown(
      `## ${proposalReport ? "Resource" : "iRule"} Validation Report${reportContext}\n`,
    );

    if (categorised.all.length === 0) {
      ctx.response.markdown(
        proposalReport
          ? `\nNo issues found in the completed ${selectedDialect} analysis.\n`
          : "\nNo issues found. The iRule looks clean.\n",
      );
      return {
        metadata: {
          command: "validate",
          count: 0,
          ...(proposalReport && {
            dialectId: proposalReport.response.dialectId,
            inputsEpoch: proposalReport.response.inputsEpoch,
            sourceVersion: proposalReport.response.version,
            resultId: proposalReport.response.resultId,
          }),
        },
      };
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

    return {
      metadata: {
        command: "validate",
        count: categorised.all.length,
        ...(proposalReport && {
          dialectId: proposalReport.response.dialectId,
          inputsEpoch: proposalReport.response.inputsEpoch,
          sourceVersion: proposalReport.response.version,
          resultId: proposalReport.response.resultId,
        }),
      },
    };
  } finally {
    if (overrideLease && !(await overrideLease.restore())) {
      throw new Error("The iRule analysis dialect override was superseded");
    }
  }
}
