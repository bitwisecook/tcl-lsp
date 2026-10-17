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
import { DiagnosticSourceOperation, sendContextualRequest } from "../contextPack";
import {
  acquireSessionDialectOverride,
  SessionDialectOverrideLease,
  categoriseDiagnostics,
  formatDiagnosticsForLLM,
  renderDiagnosticSection,
} from "../diagnosticAccess";

export async function handleReview(ctx: CommandContext): Promise<vscode.ChatResult> {
  const operation = await DiagnosticSourceOperation.resolve(ctx);
  const doc = operation?.sourceSnapshot.document;
  const code = operation?.sourceSnapshot.fullSource;

  if (!operation || !doc || !code) {
    ctx.response.markdown("Open an iRule file or attach one with `#file` for security review.");
    return {};
  }

  ctx.response.progress("Running security analysis...");

  // Ensure dialect is f5-irules.
  // Pinned as a *session override*, not a configuration push: a push is
  // re-applied away by the next `workspace/configuration` pull, which can
  // land at any time, so a push's effect would last an unpredictable length
  // of time.
  // This command explicitly requests an iRule analysis operation. An ambient
  // display dialect cannot stand in for its acknowledged source/session lease.
  let overrideLease: SessionDialectOverrideLease | undefined;
  if (!operation.isCurrent()) throw new Error("The source changed before analysis started");
  overrideLease = await acquireSessionDialectOverride("f5-irules");

  try {
    if (overrideLease) operation.adoptOverride(overrideLease);
    const report = await operation.readReport(5000);
    const diagnostics = [...report.diagnostics];
    await operation.assertPublished();
    const categorised = categoriseDiagnostics(diagnostics);

    const securityDiags = [
      ...categorised.security,
      ...categorised.taint,
      ...categorised.threadSafety,
    ];

    const lspReport =
      securityDiags.length > 0
        ? formatDiagnosticsForLLM(securityDiags, code)
        : "No security issues detected by static analysis.";

    ctx.response.progress("Running LLM security review...");

    ctx.response.markdown("## Security Review\n");

    // Static analysis findings
    if (securityDiags.length > 0) {
      ctx.response.markdown("\n### Static Analysis Findings\n");
      renderDiagnosticSection(ctx.response, "Security", categorised.security, code);
      renderDiagnosticSection(ctx.response, "Taint Analysis", categorised.taint, code);
      renderDiagnosticSection(ctx.response, "Thread Safety", categorised.threadSafety, code);
    } else {
      ctx.response.markdown(
        "\n### Static Analysis\n\nNo security issues detected by static analysis.\n",
      );
    }

    // LLM analysis (streamed)
    ctx.response.markdown("\n### Deep Analysis\n\n");
    const llmResponse = await sendContextualRequest(
      ctx,
      `Perform a comprehensive security review of this iRule. ` +
        `The static analyser already found these issues:\n${lspReport}\n\n` +
        `Go beyond the static analysis. Check for:\n` +
        `- Input validation gaps\n` +
        `- Information leakage in logs or responses\n` +
        `- Race conditions with shared state\n` +
        `- Denial of service vectors (unbounded loops, expensive operations)\n` +
        `- Header injection possibilities\n` +
        `- Open redirect vulnerabilities\n` +
        `- Session handling issues\n\n` +
        `\`\`\`tcl\n${code}\n\`\`\``,
      operation.promptOptions(),
    );
    let responseText = "";
    for await (const chunk of llmResponse.text) responseText += chunk;
    await operation.assertPublished();
    ctx.response.markdown(responseText);

    return { metadata: { command: "review", securityCount: securityDiags.length } };
  } finally {
    if (overrideLease && !(await overrideLease.restore())) {
      throw new Error("The iRule analysis dialect override was superseded");
    }
  }
}
