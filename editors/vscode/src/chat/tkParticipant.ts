import * as vscode from "vscode";
import { getActiveDialect, getClient, isAiEnabled, isTclLanguage } from "../extension";
import { runAgenticLoop } from "./agenticLoop";
import { extractCodeBlock, resolveIruleCode } from "./codeUtils";
import { sendContextualRequest } from "./contextPack";
import { renderDiagnosticSection } from "./diagnosticAccess";
import { CommandContext } from "./types";
import { buildPrompt } from "./promptLoader";
import { handleHelp } from "./commands/help";

const PARTICIPANT_ID = "tcl-lsp.tk";

export function registerTkParticipant(context: vscode.ExtensionContext): void {
  const participant = vscode.chat.createChatParticipant(PARTICIPANT_ID, tkRequestHandler);
  participant.iconPath = new vscode.ThemeIcon("window");
  participant.followupProvider = { provideFollowups: provideTkFollowups };
  context.subscriptions.push(participant);
}

const tkRequestHandler: vscode.ChatRequestHandler = async (request, context, response, token) => {
  if (!isAiEnabled()) {
    response.markdown(
      "AI features are disabled. Enable them in settings (`tclLsp.ai.enabled`) " +
        "or run the **Tcl: Toggle AI Features** command.",
    );
    return {};
  }

  const systemPrompt = buildPrompt(getActiveDialect());
  const ctx: CommandContext = {
    request,
    context,
    response,
    token,
    systemPrompt,
    client: getClient(),
  };

  switch (request.command) {
    case "create":
      return handleTkCreate(ctx);
    case "explain":
      return handleTkExplain(ctx);
    case "preview":
      return handleTkPreview(ctx);
    case "help":
      return handleHelp(ctx, "tk");
    default:
      return handleTkGeneral(ctx);
  }
};

async function handleTkGeneral(ctx: CommandContext): Promise<vscode.ChatResult> {
  if (!ctx.request.prompt.trim()) {
    ctx.response.markdown(
      "I can help with Tk GUI development. Try one of these commands:\n\n" +
        "- `/create` — Create a Tk GUI from a description\n" +
        "- `/explain` — Explain a Tk GUI's widget hierarchy and layout\n" +
        "- `/preview` — Open the Tk Preview pane for the current file\n\n" +
        "Or just ask a Tk question.",
    );
    return {};
  }

  const llmResponse = await sendContextualRequest(
    ctx,
    `This is a Tk GUI question. ${ctx.request.prompt}`,
  );
  for await (const chunk of llmResponse.text) {
    ctx.response.markdown(chunk);
  }
  return { metadata: { command: "general" } };
}

async function handleTkCreate(ctx: CommandContext): Promise<vscode.ChatResult> {
  const description = ctx.request.prompt.trim();
  if (!description) {
    ctx.response.markdown("Describe the Tk GUI you want to create.");
    return {};
  }

  ctx.response.progress("Generating Tk GUI code...");

  const llmResponse = await sendContextualRequest(
    ctx,
    `Create a Tk GUI application for this request:\n\n${description}\n\n` +
      `Requirements:\n` +
      `- Always start with \`package require Tk\`\n` +
      `- Use ttk:: themed widgets where available\n` +
      `- Prefer grid geometry manager for layout\n` +
      `- Never mix pack and grid in the same parent\n` +
      `- Use proper widget pathname hierarchy (.parent.child)\n` +
      `- Include wm title and wm geometry for the main window\n` +
      `- Add event bindings where appropriate\n\n` +
      `Return ONLY the complete Tcl/Tk code in one \`\`\`tcl code block.`,
    { allowAmbientContext: false },
  );
  let responseText = "";
  for await (const chunk of llmResponse.text) {
    responseText += chunk;
  }

  const initialCode = extractCodeBlock(responseText);
  if (!initialCode) {
    ctx.response.markdown("Could not extract Tk code from the model response.\n\n" + responseText);
    return {};
  }

  const result = await runAgenticLoop(ctx, initialCode, undefined, {
    targetDialect: getActiveDialect(),
  });

  ctx.response.markdown(`## Generated Tk GUI\n\n\`\`\`tcl\n${result.finalCode}\n\`\`\`\n`);
  if (result.clean) {
    ctx.response.markdown(`\nValidated clean in ${result.iterations} iteration(s).`);
  } else {
    ctx.response.markdown(
      `\nAfter ${result.iterations} iteration(s), ${result.remainingDiagnostics.length} issue(s) remain:\n`,
    );
    renderDiagnosticSection(
      ctx.response,
      "Remaining Issues",
      result.remainingDiagnostics,
      result.finalCode,
    );
  }

  ctx.response.button({
    command: "tclLsp.insertIrule",
    title: "Insert into new file",
    arguments: [result.finalCode],
  });

  ctx.response.button({
    command: "tclLsp.openTkPreview",
    title: "Open Tk Preview",
  });

  return { metadata: { command: "create", clean: result.clean, iterations: result.iterations } };
}

async function handleTkExplain(ctx: CommandContext): Promise<vscode.ChatResult> {
  const code = await resolveIruleCode(ctx);
  if (!code) {
    ctx.response.markdown("Open a Tk file, select code, or attach `#file` to explain.");
    return {};
  }

  ctx.response.progress("Analysing Tk GUI layout...");
  const llmResponse = await sendContextualRequest(
    ctx,
    `Explain this Tk GUI code. Cover:\n` +
      `- Widget hierarchy (parent-child relationships)\n` +
      `- Geometry manager usage (grid/pack/place layout)\n` +
      `- Event bindings and callbacks\n` +
      `- Any potential issues (mixed geometry managers, missing parents)\n\n` +
      `\`\`\`tcl\n${code}\n\`\`\``,
    { code, allowAmbientContext: false },
  );
  for await (const chunk of llmResponse.text) {
    ctx.response.markdown(chunk);
  }
  return { metadata: { command: "explain" } };
}

async function handleTkPreview(ctx: CommandContext): Promise<vscode.ChatResult> {
  const editor = vscode.window.activeTextEditor;
  if (!editor || !isTclLanguage(editor.document.languageId)) {
    ctx.response.markdown("Open a Tcl/Tk file to preview.");
    return {};
  }

  const source = editor.document.getText();
  if (!source.includes("package require Tk") && !source.includes("package require tk")) {
    ctx.response.markdown(
      "The current file does not contain `package require Tk`. " +
        "The Tk Preview pane only works with Tk applications.",
    );
    return {};
  }

  await vscode.commands.executeCommand("tclLsp.openTkPreview");
  ctx.response.markdown(
    "Opened the Tk Preview pane. The preview updates automatically as you edit.",
  );
  return { metadata: { command: "preview" } };
}

async function provideTkFollowups(
  result: vscode.ChatResult,
  _context: vscode.ChatContext,
  _token: vscode.CancellationToken,
): Promise<vscode.ChatFollowup[]> {
  const metadata = result.metadata as Record<string, unknown> | undefined;
  const command = metadata?.command as string | undefined;
  switch (command) {
    case "create":
      return [
        { prompt: "Preview this Tk GUI", command: "preview" },
        { prompt: "Explain this Tk GUI", command: "explain" },
      ];
    default:
      return [
        { prompt: "Create a Tk GUI", command: "create" },
        { prompt: "Preview current Tk file", command: "preview" },
      ];
  }
}
