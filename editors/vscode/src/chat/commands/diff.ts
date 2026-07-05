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
import { resolveTwoCodeSources } from "../codeUtils";

export async function handleDiff(ctx: CommandContext): Promise<vscode.ChatResult> {
  const sources = await resolveTwoCodeSources(ctx);

  if (!sources) {
    ctx.response.markdown(
      "Attach two iRule files with `#file` to compare, " +
        "or have one open in the editor and attach the other with `#file`.\n\n" +
        "**Example:**\n" +
        "> `@irule /diff #file:old_irule.tcl #file:new_irule.tcl`",
    );
    return {};
  }

  const [codeA, codeB] = sources;

  ctx.response.progress("Comparing iRule versions...");

  ctx.response.markdown("## iRule Diff Analysis\n\n");
  const llmResponse = await sendContextualRequest(
    ctx,
    `Compare these two versions of an iRule and explain the differences.\n\n` +
      `### Version A (original)\n\`\`\`tcl\n${codeA}\n\`\`\`\n\n` +
      `### Version B (updated)\n\`\`\`tcl\n${codeB}\n\`\`\`\n\n` +
      `Provide:\n` +
      `1. **Semantic changes** — What changed in behaviour (not just line diffs)?\n` +
      `2. **Events** — Any events added, removed, or reordered?\n` +
      `3. **Security implications** — Do the changes introduce or fix security issues?\n` +
      `4. **Performance implications** — Any changes to hot-path efficiency?\n` +
      `5. **Breaking changes** — Could these changes affect traffic handling?\n\n` +
      (ctx.request.prompt.trim() ? `The user specifically asks: ${ctx.request.prompt}\n\n` : "") +
      `Focus on what matters operationally. Be concise.`,
    {
      allowAmbientContext: false,
      code: codeB,
    },
  );
  for await (const chunk of llmResponse.text) {
    ctx.response.markdown(chunk);
  }

  return { metadata: { command: "diff" } };
}
