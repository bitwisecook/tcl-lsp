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
import { sendContextualRequest } from "../contextPack";
import { isDocumentSourceSnapshotCurrent, resolveIruleSource } from "../codeUtils";
import { buildDialectNeutralSourcePrompt } from "../promptLoader";

export async function handleExplain(ctx: CommandContext): Promise<vscode.ChatResult> {
  const resolved = await resolveIruleSource(ctx);
  if (!resolved) {
    ctx.response.markdown(
      "Open a Tcl source file, select source, or attach a file with `#file` to explain.",
    );
    return {};
  }

  const snapshot = resolved.snapshot;
  if (snapshot && !isDocumentSourceSnapshotCurrent(snapshot)) {
    ctx.response.markdown(
      "The selected source changed before it could be explained. Please retry.",
    );
    return {};
  }
  const code = resolved.source;
  const sourceContextLabel = snapshot
    ? `${snapshot.languageId} document version ${snapshot.version}${snapshot.selection ? " (selected extent)" : ""}`
    : undefined;
  const requestContext = snapshot
    ? { ...ctx, systemPrompt: buildDialectNeutralSourcePrompt() }
    : ctx;

  ctx.response.progress("Analysing source...");

  const llmResponse = await sendContextualRequest(
    requestContext,
    `Explain the captured source. Describe its structure and data flow, and note security or compatibility concerns only where the text supports them.\n\n` +
      `\`\`\`tcl\n${code}\n\`\`\`\n` +
      (ctx.request.prompt.trim() ? `\nThe user specifically asks: ${ctx.request.prompt}` : ""),
    {
      code,
      document: snapshot?.document,
      sourceContextLabel,
      isSourceCurrent: snapshot ? () => isDocumentSourceSnapshotCurrent(snapshot) : undefined,
      allowAmbientContext: false,
      includeDiagnostics: false,
      includeSymbolDefinitions: false,
      includeEventMetadata: false,
    },
  );
  for await (const chunk of llmResponse.text) {
    ctx.response.markdown(chunk);
  }

  return { metadata: { command: "explain" } };
}
