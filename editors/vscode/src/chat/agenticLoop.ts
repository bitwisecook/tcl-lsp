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
import {
  DocumentSourceSnapshot,
  DocumentWriteReceipt,
  extractCodeBlock,
  ensureDocumentOpen,
  replaceDocumentFromSnapshot,
  snapshotDocumentSource,
} from "./codeUtils";
import { sendContextualRequest } from "./contextPack";
import { buildPrompt } from "./promptLoader";
import {
  acquireSessionDialectOverride,
  isAnalysisContextCurrent,
  waitForDiagnostics,
  waitForProposalDiagnostics,
  proposalDiagnosticsReportMatches,
  formatDiagnosticsForLLM,
  SessionDialectOverrideLease,
  ProposalDiagnosticsReport,
} from "./diagnosticAccess";

const MAX_ITERATIONS = 5;
const DIAGNOSTIC_WAIT_MS = 5000;

export interface AgenticResult {
  finalCode: string;
  iterations: number;
  remainingDiagnostics: vscode.Diagnostic[];
  clean: boolean;
  documentSnapshot?: DocumentSourceSnapshot;
  proposalReport?: ProposalDiagnosticsReport & {
    readonly sourceSnapshot: DocumentSourceSnapshot;
  };
}

export function proposalReportEvidence(
  report: AgenticResult["proposalReport"],
  sourceReceipt: DocumentWriteReceipt,
): unknown {
  if (!report) return undefined;
  // Chat command arguments are data-only. The actual source/session context is
  // retained by the existing checked write receipt owner, not marshalled as
  // live VS Code or transport objects.
  return {
    response: report.response,
    sourceReceipt,
  };
}

export interface AgenticLoopOptions {
  targetDialect?: string;
  sourceSnapshot?: DocumentSourceSnapshot;
  sessionOverrideLease?: SessionDialectOverrideLease;
  initialProposalReport?: ProposalDiagnosticsReport;
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
  const targetDialect = options.targetDialect;
  let currentCode = initialCode;
  let doc = existingDoc;
  const analyzeAsProposal = existingDoc !== undefined;
  let documentSnapshot = options.sourceSnapshot;
  if (doc && !documentSnapshot) documentSnapshot = snapshotDocumentSource(doc);
  if (
    doc &&
    (!documentSnapshot ||
      documentSnapshot.document !== doc ||
      documentSnapshot.source !== documentSnapshot.fullSource ||
      documentSnapshot.version !== doc.version ||
      doc.getText() !== documentSnapshot.fullSource ||
      !isAnalysisContextCurrent(documentSnapshot.analysisContext))
  ) {
    throw new Error("Document or analysis context changed before the source-fix action started");
  }

  let loopContext = ctx;
  if (options.initialProposalReport) {
    if (targetDialect || options.sessionOverrideLease) {
      throw new Error("A proposal report cannot be reused with a dialect override");
    }
    if (
      !doc ||
      !documentSnapshot ||
      !proposalDiagnosticsReportMatches(options.initialProposalReport, {
        document: doc,
        uri: documentSnapshot.uri,
        version: documentSnapshot.version,
        source: documentSnapshot.fullSource,
        proposal: initialCode,
      })
    ) {
      throw new Error(
        "The initial proposal report no longer matches its source document and session",
      );
    }
    // Bind both diagnostic selection and the dialect-specific system prompt to
    // the same completed server report; do not join a separate resource hint.
    loopContext = {
      ...ctx,
      systemPrompt: buildPrompt(options.initialProposalReport.response.dialectId),
    };
  }

  let overrideLease = options.sessionOverrideLease;
  const ownsOverrideLease = !overrideLease && !!targetDialect;
  if (!overrideLease && targetDialect)
    overrideLease = await acquireSessionDialectOverride(targetDialect);
  if (overrideLease && !isAnalysisContextCurrent(overrideLease.analysisContext)) {
    if (ownsOverrideLease) await overrideLease.restore();
    throw new Error("The source-fix dialect override is no longer current");
  }
  if (documentSnapshot && overrideLease) {
    documentSnapshot = {
      ...documentSnapshot,
      analysisContext: overrideLease.analysisContext,
      requestedAnalysisDialect: overrideLease.dialect,
    };
  }

  const assertCurrent = (): void => {
    if (!documentSnapshot || !isAnalysisContextCurrent(documentSnapshot.analysisContext)) {
      throw new Error("Analysis context changed while the source-fix action was running");
    }
    if (
      doc &&
      (documentSnapshot.document !== doc ||
        doc.version !== documentSnapshot.version ||
        doc.getText() !== documentSnapshot.fullSource)
    ) {
      throw new Error("Document changed while the source-fix action was running");
    }
  };

  let result: AgenticResult | undefined;
  try {
    result = await (async (): Promise<AgenticResult> => {
      let iteration = 0;
      let previousCount = Infinity;
      let proposalReport: AgenticResult["proposalReport"];
      let nextProposalReport = options.initialProposalReport;

      for (iteration = 0; iteration < MAX_ITERATIONS; iteration++) {
        if (ctx.token.isCancellationRequested) break;

        if (doc && !analyzeAsProposal) {
          if (!documentSnapshot) throw new Error("Existing document has no source snapshot");
          const updated = await replaceDocumentFromSnapshot(documentSnapshot, currentCode);
          if (!updated) throw new Error("Document changed while the source-fix action was running");
          documentSnapshot = updated;
          doc = updated.document;
        } else if (!doc) {
          doc = await ensureDocumentOpen(currentCode);
          if (doc.getText() !== currentCode) {
            throw new Error("New document changed before analysis could start");
          }
          const openedSnapshot = snapshotDocumentSource(doc);
          documentSnapshot = {
            ...openedSnapshot,
            requestedAnalysisDialect: overrideLease?.dialect ?? null,
            analysisContext: overrideLease?.analysisContext ?? openedSnapshot.analysisContext,
          };
        }
        assertCurrent();

        ctx.response.progress(
          `Iteration ${iteration + 1}/${MAX_ITERATIONS}: analysing with LSP...`,
        );
        let diagnostics: readonly vscode.Diagnostic[];
        if (analyzeAsProposal) {
          if (!documentSnapshot) throw new Error("Proposal analysis has no source snapshot");
          const report =
            nextProposalReport ??
            (await waitForProposalDiagnostics(doc.uri, currentCode, {
              timeout: DIAGNOSTIC_WAIT_MS,
              expectedSource: documentSnapshot.fullSource,
            }));
          nextProposalReport = undefined;
          if (report.response.proposal !== currentCode) {
            throw new Error("Proposal diagnostics do not match the current Tcl source");
          }
          if (
            proposalReport &&
            (report.response.dialectId !== proposalReport.response.dialectId ||
              report.response.inputsEpoch !== proposalReport.response.inputsEpoch)
          ) {
            throw new Error("The resource analysis context changed during the source-fix action");
          }
          proposalReport = Object.freeze({ ...report, sourceSnapshot: documentSnapshot });
          diagnostics = report.diagnostics;
        } else {
          diagnostics = await waitForDiagnostics(doc.uri, {
            timeout: DIAGNOSTIC_WAIT_MS,
            expectedSource: currentCode,
          });
        }
        assertCurrent();
        const actionable = diagnostics.filter(
          (d) => d.severity <= vscode.DiagnosticSeverity.Warning,
        );

        if (actionable.length === 0) {
          ctx.response.progress("Clean -- no errors or warnings.");
          return {
            finalCode: currentCode,
            iterations: iteration + 1,
            remainingDiagnostics: [],
            clean: true,
            documentSnapshot,
            proposalReport,
          };
        }
        if (actionable.length >= previousCount && iteration > 0) {
          ctx.response.progress(`Issue count not decreasing (${actionable.length}). Stopping.`);
          break;
        }
        previousCount = actionable.length;

        ctx.response.progress(`Found ${actionable.length} issue(s). Asking LLM to fix...`);
        const diagnosticReport = formatDiagnosticsForLLM(actionable, currentCode);
        const selectedDialect = proposalReport?.response.dialectId;
        if (analyzeAsProposal && !selectedDialect) {
          throw new Error("The server did not provide the selected resource dialect");
        }
        const llmResponse = await sendContextualRequest(
          loopContext,
          `The language server selected resource dialect ${selectedDialect ?? "Tcl"}.\n` +
            `Here is the current Tcl code:
\`\`\`tcl\n${currentCode}\n\`\`\`\n\n` +
            `The Tcl LSP found these issues:\n${diagnosticReport}\n\n` +
            `Fix all the issues listed above. Return ONLY the complete corrected Tcl code in a single \`\`\`tcl code block. Do not include any explanation outside the code block.`,
          {
            allowAmbientContext: false,
            code: currentCode,
            document: doc,
            includeDiagnostics: false,
          },
        );
        assertCurrent();

        let responseText = "";
        for await (const chunk of llmResponse.text) responseText += chunk;
        assertCurrent();
        const extractedCode = extractCodeBlock(responseText);
        if (!extractedCode) {
          ctx.response.progress("LLM did not return a code block. Stopping.");
          break;
        }
        currentCode = extractedCode;
      }

      if (doc) {
        if (!documentSnapshot) throw new Error("Existing document has no source snapshot");
        if (!analyzeAsProposal) {
          const updated = await replaceDocumentFromSnapshot(documentSnapshot, currentCode);
          if (!updated) throw new Error("Document changed before the final source check");
          documentSnapshot = updated;
          doc = updated.document;
        }
        let finalDiags: readonly vscode.Diagnostic[];
        if (analyzeAsProposal) {
          const report = await waitForProposalDiagnostics(doc.uri, currentCode, {
            timeout: DIAGNOSTIC_WAIT_MS,
            expectedSource: documentSnapshot.fullSource,
          });
          if (
            proposalReport &&
            (report.response.dialectId !== proposalReport.response.dialectId ||
              report.response.inputsEpoch !== proposalReport.response.inputsEpoch)
          ) {
            throw new Error("The resource analysis context changed during the source-fix action");
          }
          proposalReport = Object.freeze({ ...report, sourceSnapshot: documentSnapshot });
          finalDiags = report.diagnostics;
        } else {
          finalDiags = await waitForDiagnostics(doc.uri, {
            timeout: DIAGNOSTIC_WAIT_MS,
            expectedSource: currentCode,
          });
        }
        assertCurrent();
        const remaining = finalDiags.filter((d) => d.severity <= vscode.DiagnosticSeverity.Warning);
        return {
          finalCode: currentCode,
          iterations: iteration,
          remainingDiagnostics: remaining,
          clean: remaining.length === 0,
          documentSnapshot,
          proposalReport,
        };
      }
      return {
        finalCode: currentCode,
        iterations: iteration,
        remainingDiagnostics: [],
        clean: false,
      };
    })();
  } finally {
    if (overrideLease && ownsOverrideLease) {
      const writeContext = await overrideLease.restore();
      if (!writeContext) {
        throw new Error("The source-fix dialect override was superseded before restoration");
      }
      if (
        result?.documentSnapshot &&
        doc &&
        doc.version === result.documentSnapshot.version &&
        doc.getText() === result.documentSnapshot.fullSource
      ) {
        result.documentSnapshot = {
          ...result.documentSnapshot,
          analysisContext: writeContext,
        };
      } else if (result?.documentSnapshot) {
        throw new Error("Document changed before the source-fix result could be published");
      }
    }
  }
  if (!result) throw new Error("Source-fix analysis did not produce a result");
  if (
    result.documentSnapshot &&
    !isAnalysisContextCurrent(result.documentSnapshot.analysisContext)
  ) {
    throw new Error("Analysis context changed before the source-fix result could be published");
  }
  return result;
}
