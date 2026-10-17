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

import type * as vscode from "vscode";
import { CommandContext } from "../types";
import { documentWriteReceipt, extractCodeBlock } from "../codeUtils";
import { DiagnosticSourceOperation, sendContextualRequest } from "../contextPack";
import {
  acquireSessionDialectOverride,
  isAnalysisContextCurrent,
  diagnosticConversionTopics,
  formatDiagnosticsForLLM,
  renderDiagnosticSection,
  SessionDialectOverrideLease,
} from "../diagnosticAccess";
import { proposalReportEvidence, runAgenticLoop } from "../agenticLoop";

export async function handleFindLegacy(ctx: CommandContext): Promise<vscode.ChatResult> {
  const operation = await DiagnosticSourceOperation.resolve(ctx);
  const doc = operation?.sourceSnapshot.document;
  const code = operation?.sourceSnapshot.fullSource;

  if (!operation || !doc || !code) {
    ctx.response.markdown("Open an iRule file or attach one with `#file` to modernise.");
    return {};
  }

  ctx.response.progress("Scanning for legacy patterns...");

  // Ensure dialect is f5-irules.
  // Pinned as a *session override*, not a configuration push: a push is
  // re-applied away by the next `workspace/configuration` pull, which can
  // land at any time, so a push's effect would last an unpredictable length
  // of time.
  // This command explicitly requests an iRule analysis operation. An ambient
  // display dialect cannot stand in for its acknowledged source/session lease.
  let overrideLease: SessionDialectOverrideLease | undefined;
  if (!operation.isCurrent()) throw new Error("The source changed before legacy scanning");
  overrideLease = await acquireSessionDialectOverride("f5-irules");

  try {
    if (overrideLease) operation.adoptOverride(overrideLease);
    const report = await operation.readReport(5000);
    await operation.assertPublished();
    // Canonical metadata selects authored proposal topics, never edit authority.
    const topics = diagnosticConversionTopics(report.diagnostics);
    const convertible = topics.map((topic) => topic.diagnostic);
    const sourceSnapshot = operation.sourceSnapshot;

    if (convertible.length === 0) {
      ctx.response.markdown("No canonical legacy-conversion topics were reported for this source.");
      return { metadata: { command: "find-legacy", count: 0 } };
    }

    ctx.response.progress(`Found ${convertible.length} legacy pattern(s). Modernising...`);

    const diagnosticReport = formatDiagnosticsForLLM(convertible, code);

    // Ask LLM to modernise
    const llmResponse = await sendContextualRequest(
      ctx,
      `Modernise this iRule by converting all legacy patterns identified below.\n\n` +
        `Current code:\n\`\`\`tcl\n${code}\n\`\`\`\n\n` +
        `Legacy patterns found by static analysis:\n${diagnosticReport}\n\n` +
        `Consider these canonical conversion topics (proposals, not automatic edits):\n` +
        [...new Set(topics.map((topic) => topic.description))]
          .map((description) => `- ${description}`)
          .join("\n") +
        "\n\n" +
        `Return ONLY the complete modernised iRule in a single \`\`\`tcl code block.`,
      operation.promptOptions(),
    );
    let responseText = "";
    for await (const chunk of llmResponse.text) {
      responseText += chunk;
    }
    await operation.assertPublished();

    const modernised = extractCodeBlock(responseText);
    if (!modernised) {
      ctx.response.markdown(
        "Failed to extract modernised code from LLM response.\n\n" + responseText,
      );
      return {};
    }

    // The existing checked proposal path validates authored text; diagnostic
    // topic membership itself grants neither edits nor successful execution.
    // Run agentic loop to validate
    const result = await runAgenticLoop(ctx, modernised, doc, {
      // Reuse the operation's actual owned lease; never start a second
      // override merely from the ambient display dialect.
      targetDialect: overrideLease?.dialect,
      sourceSnapshot,
      sessionOverrideLease: overrideLease,
    });

    await operation.assertPublished();
    ctx.response.markdown(`## Modernised iRule\n\n\`\`\`tcl\n${result.finalCode}\n\`\`\`\n`);

    if (result.proposalReport) {
      const report = result.proposalReport.response;
      ctx.response.markdown(
        `\nDiagnostics were captured for the proposal at ${report.uri}, source version ${report.version}.`,
      );
    }
    if (result.clean) {
      ctx.response.markdown(
        `\nProposed updates for ${convertible.length} legacy topic(s). No proposal diagnostics were reported after ${result.iterations} iteration(s).`,
      );
    } else {
      ctx.response.markdown(
        `\nProposed updates for ${convertible.length} legacy topic(s). After ${result.iterations} iteration(s), ${result.remainingDiagnostics.length} issue(s) remain:\n`,
      );
      renderDiagnosticSection(
        ctx.response,
        "Remaining Issues",
        result.remainingDiagnostics,
        result.finalCode,
      );
    }

    if (!result.documentSnapshot)
      throw new Error("Modernisation result has no current document snapshot");
    const operationContext = result.documentSnapshot.analysisContext;
    if (!isAnalysisContextCurrent(operationContext)) {
      throw new Error("Analysis context changed before the modernisation result was ready");
    }
    if (overrideLease) {
      const writeContext = await overrideLease.restore();
      if (!writeContext || !isAnalysisContextCurrent(writeContext) || !operation.sourceUnchanged())
        throw new Error("The modernisation source or dialect override was superseded");
      result.documentSnapshot = { ...result.documentSnapshot, analysisContext: writeContext };
      overrideLease = undefined;
    }
    if (ctx.token.isCancellationRequested || !operation.sourceUnchanged())
      throw new Error("The modernisation source changed before its action was published");
    const writeReceipt = documentWriteReceipt(result.documentSnapshot);
    ctx.response.button({
      command: "tclLsp.applyFix",
      title: "Apply modernised code",
      arguments: [
        result.finalCode,
        writeReceipt,
        proposalReportEvidence(result.proposalReport, writeReceipt),
      ],
    });

    return {
      metadata: { command: "find-legacy", count: convertible.length, clean: result.clean },
    };
  } finally {
    if (overrideLease) await overrideLease.restore();
  }
}
