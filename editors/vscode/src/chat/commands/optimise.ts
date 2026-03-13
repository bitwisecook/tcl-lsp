import * as vscode from "vscode";
import { CommandContext } from "../types";
import { sendContextualRequest } from "../contextPack";
import { isTclLanguage } from "../../extension";

interface OptEntry {
  code: string;
  message: string;
  startLine: number;
  replacement: string;
  group?: number;
}

export async function handleOptimise(ctx: CommandContext): Promise<vscode.ChatResult> {
  const editor = vscode.window.activeTextEditor;
  if (!editor || !isTclLanguage(editor.document.languageId)) {
    ctx.response.markdown("Open a Tcl or iRule file to run optimisations.");
    return {};
  }

  const uri = editor.document.uri.toString();

  ctx.response.progress("Running LSP optimiser...");

  const result = (await ctx.client.sendRequest("workspace/executeCommand", {
    command: "tcl-lsp.optimiseDocument",
    arguments: [uri],
  })) as { optimisations: OptEntry[]; source: string } | null;

  if (!result || !result.optimisations || result.optimisations.length === 0) {
    ctx.response.markdown("No optimisations found. The code is already well-optimised.");
    return { metadata: { command: "optimise", count: 0 } };
  }

  const opts = result.optimisations;

  // Apply optimisations immediately.
  ctx.response.progress("Applying optimisations...");
  await vscode.commands.executeCommand("tclLsp.applyFix", result.source, uri);

  // Group optimisations for display.
  const ELIM_CODES = new Set(["O107", "O108", "O109"]);
  const groups = new Map<number, OptEntry[]>();
  const ungrouped: OptEntry[] = [];
  for (const opt of opts) {
    if (opt.group !== undefined && opt.group !== null) {
      const list = groups.get(opt.group) ?? [];
      list.push(opt);
      groups.set(opt.group, list);
    } else {
      ungrouped.push(opt);
    }
  }

  // Build display items: one per group + ungrouped.
  interface DisplayItem {
    primary: OptEntry;
    members: OptEntry[];
  }
  const displayItems: DisplayItem[] = [];
  for (const [, members] of [...groups.entries()].sort((a, b) => a[0] - b[0])) {
    const primary = members.find((m) => !ELIM_CODES.has(m.code)) ?? members[0];
    displayItems.push({ primary, members });
  }
  for (const opt of ungrouped) {
    displayItems.push({ primary: opt, members: [opt] });
  }

  // Show summary of what was applied.
  ctx.response.markdown(`## Applied ${displayItems.length} Optimisation(s)\n\n`);

  for (const item of displayItems) {
    const { primary, members } = item;
    const startLine = primary.startLine + 1;
    if (members.length > 1) {
      const elimCount = members.filter((m) => ELIM_CODES.has(m.code)).length;
      const suffix = elimCount
        ? ` (+${elimCount} dead store${elimCount > 1 ? "s" : ""} eliminated)`
        : "";
      ctx.response.markdown(
        `- **${primary.code}** (line ${startLine}): ${primary.message}${suffix}\n`,
      );
      for (const m of members) {
        if (m !== primary) {
          ctx.response.markdown(`  - ${m.code} (line ${m.startLine + 1}): ${m.message}\n`);
        }
      }
    } else {
      ctx.response.markdown(`- **${primary.code}** (line ${startLine}): ${primary.message}\n`);
    }
  }

  // Ask LLM to briefly explain the optimisations.
  ctx.response.progress("Generating explanations...");

  const optSummary = displayItems
    .map((item) => {
      const { primary, members } = item;
      if (members.length > 1) {
        const sub = members
          .map(
            (m) =>
              `  - ${m.code} (line ${m.startLine + 1}): ${m.message} → \`${m.replacement || "(remove)"}\``,
          )
          .join("\n");
        return `- Group: ${primary.message} (${members.length} rewrites)\n${sub}`;
      }
      return `- ${primary.code} (line ${primary.startLine + 1}): ${primary.message} → \`${primary.replacement}\``;
    })
    .join("\n");

  ctx.response.markdown("\n## Explanations\n\n");
  const llmResponse = await sendContextualRequest(
    ctx,
    `The LSP optimiser applied these optimisations to a Tcl/iRule file:\n\n` +
      `${optSummary}\n\n` +
      `For each optimisation (or group), explain in 1-2 sentences why it is safe and what benefit it provides. ` +
      `When a group combines constant propagation/folding with dead store elimination, explain them as one logical transformation.`,
    { document: editor.document },
  );
  for await (const chunk of llmResponse.text) {
    ctx.response.markdown(chunk);
  }

  return { metadata: { command: "optimise", count: displayItems.length } };
}
