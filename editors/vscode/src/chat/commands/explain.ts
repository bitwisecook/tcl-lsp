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
