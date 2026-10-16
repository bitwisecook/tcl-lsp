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
import {
  documentWriteReceipt,
  resolveIruleCode,
  ensureDocumentOpen,
  extractCodeBlock,
  snapshotDocumentSource,
  DocumentSourceSnapshot,
} from "../codeUtils";
import { sendContextualRequest } from "../contextPack";
import {
  acquireSessionDialectOverride,
  isAnalysisContextCurrent,
  waitForDiagnostics,
  formatDiagnosticsForLLM,
  renderDiagnosticSection,
  SessionDialectOverrideLease,
} from "../diagnosticAccess";
import { runAgenticLoop } from "../agenticLoop";
import { getActiveDialect, isTclLanguage } from "../../extension";

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

export async function handleFindLegacy(ctx: CommandContext): Promise<vscode.ChatResult> {
  // Try active editor first, then resolve from references
  let doc: vscode.TextDocument | undefined;
  let code: string | undefined;
  let sourceSnapshot: DocumentSourceSnapshot | undefined;

  const editor = vscode.window.activeTextEditor;
  if (editor && isTclLanguage(editor.document.languageId)) {
    doc = editor.document;
    sourceSnapshot = snapshotDocumentSource(doc);
    code = sourceSnapshot.fullSource;
  } else {
    code = await resolveIruleCode(ctx);
  }

  if (!code) {
    ctx.response.markdown("Open an iRule file or attach one with `#file` to modernise.");
    return {};
  }

  ctx.response.progress("Scanning for legacy patterns...");

  // Ensure dialect is f5-irules.
  // Pinned as a *session override*, not a configuration push: a push is
  // re-applied away by the next `workspace/configuration` pull, which can
  // land at any time, so a push's effect would last an unpredictable length
  // of time.
  const pinnedDialect = getActiveDialect() !== "f5-irules";
  let overrideLease: SessionDialectOverrideLease | undefined;
  if (pinnedDialect) overrideLease = await acquireSessionDialectOverride("f5-irules");

  try {
    if (!doc) {
      doc = await ensureDocumentOpen(code);
      if (doc.getText() !== code) throw new Error("Source document changed before scanning");
      sourceSnapshot = snapshotDocumentSource(doc);
    }
    if (overrideLease && sourceSnapshot) {
      sourceSnapshot = {
        ...sourceSnapshot,
        analysisContext: overrideLease.analysisContext,
        requestedAnalysisDialect: overrideLease.dialect,
      };
    }
    let operationContext = overrideLease?.analysisContext ?? sourceSnapshot?.analysisContext;
    if (!isAnalysisContextCurrent(operationContext)) {
      throw new Error("Analysis context changed before legacy scanning started");
    }

    // Get diagnostics and filter to convertible patterns
    const diagnostics = await waitForDiagnostics(doc.uri, { timeout: 5000, expectedSource: code });
    if (!isAnalysisContextCurrent(operationContext)) {
      throw new Error("Analysis context changed during legacy scanning");
    }
    const convertible = diagnostics.filter((d) => CONVERTIBLE_CODES.has(diagnosticCode(d)));

    if (convertible.length === 0) {
      ctx.response.markdown(
        "No legacy patterns detected. The iRule already follows current best practices.",
      );
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
    if (!isAnalysisContextCurrent(operationContext)) {
      throw new Error("Analysis context changed while legacy code was being modernised");
    }

    const modernised = extractCodeBlock(responseText);
    if (!modernised) {
      ctx.response.markdown(
        "Failed to extract modernised code from LLM response.\n\n" + responseText,
      );
      return {};
    }

    // Run agentic loop to validate
    const result = await runAgenticLoop(ctx, modernised, doc, {
      targetDialect: "f5-irules",
      sourceSnapshot,
      sessionOverrideLease: overrideLease,
    });

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

    if (!result.documentSnapshot)
      throw new Error("Modernisation result has no current document snapshot");
    operationContext = result.documentSnapshot.analysisContext;
    if (!isAnalysisContextCurrent(operationContext)) {
      throw new Error("Analysis context changed before the modernisation result was ready");
    }
    if (overrideLease) {
      const writeContext = await overrideLease.restore();
      if (!writeContext) throw new Error("The modernisation dialect override was superseded");
      result.documentSnapshot = { ...result.documentSnapshot, analysisContext: writeContext };
      overrideLease = undefined;
    }
    ctx.response.button({
      command: "tclLsp.applyFix",
      title: "Apply modernised code",
      arguments: [result.finalCode, documentWriteReceipt(result.documentSnapshot)],
    });

    return {
      metadata: { command: "find-legacy", count: convertible.length, clean: result.clean },
    };
  } finally {
    if (overrideLease) await overrideLease.restore();
  }
}
