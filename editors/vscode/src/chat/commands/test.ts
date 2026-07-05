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
import { resolveIruleCode } from "../codeUtils";
import { sendContextualRequest } from "../contextPack";
import { isTclLanguage } from "../../extension";

export async function handleTest(ctx: CommandContext): Promise<vscode.ChatResult> {
  // Resolve the iRule source code
  let code: string | undefined;

  const editor = vscode.window.activeTextEditor;
  if (editor && isTclLanguage(editor.document.languageId)) {
    code = editor.document.getText();
  } else {
    code = await resolveIruleCode(ctx);
  }

  if (!code) {
    ctx.response.markdown(
      "Open an iRule file or attach one with `#file` to generate tests.\n\n" +
        "Usage: `@irule /test` — generates a test script for the current iRule\n\n" +
        "The generated test uses the iRule Event Orchestrator framework to:\n" +
        "- Fire events in correct order based on profiles\n" +
        "- Assert on pool selection, header manipulation, logging\n" +
        "- Test edge cases (empty values, missing headers)\n" +
        "- Test keep-alive scenarios for multi-request iRules\n",
    );
    return {};
  }

  ctx.response.progress("Analyzing iRule and generating test scenarios...");

  const prompt =
    `Generate a comprehensive test script for this iRule using the Event Orchestrator test framework.\n\n` +
    `The test framework provides structured test cases (like tcltest):\n\n` +
    `\`\`\`tcl\n` +
    `::orch::configure_tests \\\n` +
    `    -profiles {TCP HTTP} \\\n` +
    `    -irule { when HTTP_REQUEST { pool web_pool } } \\\n` +
    `    -setup { ::orch::add_pool web_pool {10.0.0.1:80} }\n\n` +
    `::orch::test "name-1.0" "description" -body {\n` +
    `    ::orch::run_http_request -host example.com\n` +
    `    ::orch::assert_that pool_selected equals web_pool\n` +
    `}\n\n` +
    `exit [::orch::done]\n` +
    `\`\`\`\n\n` +
    `Key APIs:\n` +
    `- \`::orch::configure_tests -profiles -irule -setup\` — set defaults\n` +
    `- \`::orch::test "name" "desc" -body {...}\` — isolated test case (auto reset)\n` +
    `- \`::orch::run_http_request -host -uri -method\` — simulate request\n` +
    `- \`::orch::run_next_request\` — keep-alive follow-up\n` +
    `- \`::orch::assert_that <subject> <verb> <value>\` — fluent assertions\n` +
    `- \`::orch::done\` — print summary, return exit code\n\n` +
    `Subjects: pool_selected, http_uri, http_host, http_path, http_method, http_status,\n` +
    `  http_header "Name", decision <cat> <action>, log, event, var <name>\n\n` +
    `Verbs: equals, not_equals, contains, starts_with, ends_with, matches,\n` +
    `  was_called, was_called_with, was_not_called\n\n` +
    `Generate 3-5 named test cases covering:\n` +
    `1. Normal operation (happy path)\n` +
    `2. Edge cases (empty values, missing headers)\n` +
    `3. Error/rejection paths\n` +
    `4. Keep-alive if HTTP_RESPONSE is handled\n\n` +
    `iRule source:\n\`\`\`tcl\n${code}\n\`\`\``;

  const llmResponse = await sendContextualRequest(ctx, prompt);

  for await (const chunk of llmResponse.text) {
    ctx.response.markdown(chunk);
  }

  return { metadata: { command: "test" } };
}
