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
import {
  waitForDiagnostics,
  categoriseDiagnostics,
  renderDiagnosticSection,
} from "../diagnosticAccess";
import { setSessionDialectOverride, getActiveDialect, isTclLanguage } from "../../extension";

export async function handleValidate(ctx: CommandContext): Promise<vscode.ChatResult> {
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
    ctx.response.markdown("Open an iRule file or attach one with `#file` to validate.");
    return {};
  }

  ctx.response.progress("Running LSP analysis...");

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

    const diagnostics = await waitForDiagnostics(doc.uri, { timeout: 5000 });
    const categorised = categoriseDiagnostics(diagnostics);

    ctx.response.markdown("## iRule Validation Report\n");

    if (categorised.all.length === 0) {
      ctx.response.markdown("\nNo issues found. The iRule looks clean.\n");
      return { metadata: { command: "validate", count: 0 } };
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

    return { metadata: { command: "validate", count: categorised.all.length } };
  } finally {
    if (pinnedDialect) {
      // Clearing restores whatever the configuration resolves to.
      await setSessionDialectOverride(null);
    }
  }
}
