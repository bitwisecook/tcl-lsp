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
