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
import { isDocumentSourceSnapshotCurrent, resolveTwoIruleSources } from "../codeUtils";
import { buildDialectNeutralSourcePrompt } from "../promptLoader";

export async function handleDiff(ctx: CommandContext): Promise<vscode.ChatResult> {
  const sources = await resolveTwoIruleSources(ctx);

  if (!sources) {
    ctx.response.markdown(
      "Attach two Tcl source files with `#file` to compare, " +
        "or have one open in the editor and attach the other with `#file`.\n\n" +
        "**Example:**\n" +
        "> `@irule /diff #file:old.tcl #file:new.tcl`",
    );
    return {};
  }

  const [sourceA, sourceB] = sources;
  const snapshots = [sourceA.snapshot, sourceB.snapshot].filter(
    (snapshot): snapshot is NonNullable<typeof snapshot> => snapshot !== undefined,
  );
  if (snapshots.some((snapshot) => !isDocumentSourceSnapshotCurrent(snapshot))) {
    ctx.response.markdown(
      "One of the selected sources changed before it could be compared. Please retry.",
    );
    return {};
  }
  const codeA = sourceA.source;
  const codeB = sourceB.source;
  const sourceContextLabel = [
    sourceA.snapshot
      ? `Version A: ${sourceA.snapshot.languageId} document version ${sourceA.snapshot.version}` +
        (sourceA.snapshot.selection ? " (selected extent)" : "")
      : undefined,
    sourceB.snapshot
      ? `Version B: ${sourceB.snapshot.languageId} document version ${sourceB.snapshot.version}` +
        (sourceB.snapshot.selection ? " (selected extent)" : "")
      : undefined,
  ]
    .filter((label): label is string => label !== undefined)
    .join("; ");
  const requestContext =
    snapshots.length > 0 ? { ...ctx, systemPrompt: buildDialectNeutralSourcePrompt() } : ctx;
  const isSourcesCurrent = () => snapshots.every(isDocumentSourceSnapshotCurrent);

  ctx.response.progress("Comparing source versions...");

  ctx.response.markdown("## Source Diff Analysis\n\n");
  const llmResponse = await sendContextualRequest(
    requestContext,
    `Compare these two source snapshots and explain the differences.\n\n` +
      `### Version A (original; language ${sourceA.languageId ?? "unspecified"})\n\`\`\`tcl\n${codeA}\n\`\`\`\n\n` +
      `### Version B (updated; language ${sourceB.languageId ?? "unspecified"})\n\`\`\`tcl\n${codeB}\n\`\`\`\n\n` +
      `Describe behavior changes rather than only line differences. Identify control-flow, command, variable, or compatibility changes supported by the source. Mention security or performance implications only where the code provides evidence, and call out dialect-specific uncertainty.\n\n` +
      (ctx.request.prompt.trim() ? `The user specifically asks: ${ctx.request.prompt}\n\n` : "") +
      `Be concise.`,
    {
      allowAmbientContext: false,
      code: codeB,
      document: sourceB.snapshot?.document,
      sourceContextLabel: sourceContextLabel || undefined,
      isSourceCurrent: snapshots.length > 0 ? isSourcesCurrent : undefined,
      includeDiagnostics: false,
      includeSymbolDefinitions: false,
      includeEventMetadata: false,
    },
  );
  for await (const chunk of llmResponse.text) {
    ctx.response.markdown(chunk);
  }

  return { metadata: { command: "diff" } };
}
