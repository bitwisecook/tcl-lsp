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
import { resolveIruleCode } from "../codeUtils";

export async function handleExplain(ctx: CommandContext): Promise<vscode.ChatResult> {
  const code = await resolveIruleCode(ctx);
  if (!code) {
    ctx.response.markdown(
      "Please open an iRule file, select iRule code, or attach a file with `#file` to explain.",
    );
    return {};
  }

  ctx.response.progress("Analysing iRule...");

  const llmResponse = await sendContextualRequest(
    ctx,
    `Explain what this iRule does. Break down each event handler, ` +
      `describe the data flow, note any security concerns, and summarise the overall purpose.\n\n` +
      `\`\`\`tcl\n${code}\n\`\`\`\n` +
      (ctx.request.prompt.trim() ? `\nThe user specifically asks: ${ctx.request.prompt}` : ""),
    { code, allowAmbientContext: false },
  );
  for await (const chunk of llmResponse.text) {
    ctx.response.markdown(chunk);
  }

  return { metadata: { command: "explain" } };
}
