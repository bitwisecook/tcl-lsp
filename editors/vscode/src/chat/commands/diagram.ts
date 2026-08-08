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
import { openDiagramPanel } from "../../diagramPanel";

interface DiagramData {
  events: Array<{
    name: string;
    priority: number | null;
    multiplicity: string;
    flow: unknown[];
  }>;
  procedures: Array<{
    name: string;
    params: string[];
    flow: unknown[];
  }>;
  error?: string;
}

function buildDiagramPrompt(code: string, structuredData: string, userExtra: string): string {
  return `You are generating a Mermaid flowchart and explanation for an F5 BIG-IP iRule.

## Task
Produce:
1. A Mermaid flowchart diagram showing the iRule's logic flow
2. A brief explanation of what the iRule does

## Rules for the Mermaid diagram
- Use \`flowchart TD\` (top-down direction)
- Create a **subgraph** for each event handler, labeled with the event name (e.g., \`subgraph HTTP_REQUEST\`)
- Inside each subgraph:
  - Use **diamond shapes** \`{Decision}\` for \`switch\` and \`if\` decision points
  - Use **rectangle shapes** \`[Action]\` for commands like \`pool\`, \`HTTP::respond\`, \`HTTP::redirect\`, \`HTTP::header\`, \`log\`, etc.
  - Use **rounded rectangles** \`(Return)\` for \`return\` statements
  - Use **stadium shapes** \`([Loop])\` for loops
  - Connect decision points to their branches with labeled edges (the pattern or condition on the arrow)
- If procedures are called, show them as separate subgraphs linked from the call site
- Keep node labels concise (under 40 characters) — abbreviate long strings with "..."
- Use meaningful node IDs (e.g., \`hr_switch\` not \`A1\`)
- Show the event subgraphs in firing order (top to bottom)
- If events have a non-default priority (not 500), mention it in the subgraph label
- For switch statements, show the subject being switched on in the diamond, and label each outgoing edge with the match pattern
- Connect the last node of one event subgraph to the first node of the next event subgraph to show event ordering
- Use double-quoted strings for node labels containing special characters
- In node labels, use the word \`and\` instead of \`&&\`, \`or\` instead of \`||\`, and \`not\` instead of \`!\` — Mermaid treats \`&\` as an HTML entity
- Wrap data-group (class) names in single quotes in node labels (e.g., \`'AppCheck_trusted'\` not \`AppCheck_trusted\`)

## Rules for the explanation
- 2-4 paragraphs summarising what the iRule does
- Note the event firing order and any cross-event data flow (e.g. variables set in CLIENT_ACCEPTED used in HTTP_REQUEST)
- Highlight key decision points and routing logic
- Mention any security-relevant actions (respond, redirect, header manipulation)

## Structured flow data (authoritative — extracted from the compiler IR)
\`\`\`json
${structuredData}
\`\`\`

## Original iRule source (for reference)
\`\`\`tcl
${code}
\`\`\`
${userExtra}

Now produce the Mermaid diagram inside a \`\`\`mermaid code fence, followed by the explanation.`;
}

const MERMAID_START =
  /^\s*(?:flowchart|graph|sequenceDiagram|classDiagram|stateDiagram(?:-v2)?|erDiagram|journey|gantt|pie|mindmap|timeline|gitGraph)\b/;

/** Pull a Mermaid diagram out of a model response, tolerating the fence tag. */
function extractMermaid(text: string): string | undefined {
  let fallback: string | undefined;
  for (const match of text.matchAll(/```([A-Za-z0-9_.+-]*)[ \t]*\r?\n([\s\S]*?)```/g)) {
    const tag = (match[1] ?? "").toLowerCase();
    const body = match[2].trim();
    if (!body) continue;
    if (tag === "mermaid") return body;
    if (MERMAID_START.test(body)) fallback ??= body;
  }
  return fallback;
}

export async function handleDiagram(ctx: CommandContext): Promise<vscode.ChatResult> {
  const code = await resolveIruleCode(ctx);
  if (!code) {
    ctx.response.markdown(
      "Please open an iRule file, select iRule code, or attach a file with `#file` to generate a diagram.",
    );
    return {};
  }

  ctx.response.progress("Analysing iRule structure...");

  let diagramData: DiagramData | undefined;
  try {
    diagramData =
      ((await ctx.client.sendRequest("workspace/executeCommand", {
        command: "tcl-lsp.diagramData",
        arguments: [code],
      })) as DiagramData | null) ?? undefined;
  } catch {
    // Server might not support the command yet.
  }

  if (!diagramData || diagramData.error) {
    ctx.response.markdown(
      "Failed to extract structural data from the iRule. " +
        (diagramData?.error ?? "The server returned no data."),
    );
    return {};
  }

  if (diagramData.events.length === 0 && diagramData.procedures.length === 0) {
    ctx.response.markdown(
      "No `when` event handlers or procedures found in this code. " +
        "The diagram feature works with iRules that have `when EVENT { ... }` blocks.",
    );
    return {};
  }

  ctx.response.progress("Generating Mermaid diagram...");

  const structuredDataStr = JSON.stringify(diagramData, null, 2);
  const userExtra = ctx.request.prompt.trim()
    ? `\nThe user specifically asks: ${ctx.request.prompt}`
    : "";

  const prompt = buildDiagramPrompt(code, structuredDataStr, userExtra);

  const llmResponse = await sendContextualRequest(ctx, prompt, {
    code,
    allowAmbientContext: false,
    includeDiagnostics: false,
    includeSymbolDefinitions: false,
    includeEventMetadata: false,
  });

  let fullText = "";
  for await (const chunk of llmResponse.text) {
    ctx.response.markdown(chunk);
    fullText += chunk;
  }

  // Extract the Mermaid code block and open it in a webview tab.
  // Accept any fenced block whose body looks like Mermaid, not just one tagged
  // exactly ```mermaid — models label these inconsistently, and an untagged or
  // oddly-tagged fence used to mean no diagram panel at all.
  const mermaidSource = extractMermaid(fullText);
  if (mermaidSource) {
    openDiagramPanel(mermaidSource);
  } else {
    // Distinguish "the model said nothing" from "it answered but without a
    // recognisable diagram" — they need completely different fixes, and both
    // look identical in the chat view.
    console.error(
      `[tcl-lsp] /diagram produced no diagram: ${fullText.length} chars returned` +
        (fullText.length ? `, starting: ${JSON.stringify(fullText.slice(0, 200))}` : ""),
    );
    ctx.response.markdown(
      fullText.trim()
        ? "\n\nThe model replied without a Mermaid diagram, so no diagram panel was opened."
        : "\n\nThe model returned an empty response, so no diagram could be generated. " +
            "Try again, or pick a different model in the chat model picker.",
    );
  }

  return { metadata: { command: "diagram" } };
}
