import * as vscode from "vscode";
import { getActiveDialect, getClient, isAiEnabled, isTclLanguage } from "../extension";
import { runAgenticLoop } from "./agenticLoop";
import { ensureDocumentOpen, extractCodeBlock, resolveIruleCode } from "./codeUtils";
import { sendContextualRequest } from "./contextPack";
import {
  categoriseDiagnostics,
  renderDiagnosticSection,
  waitForDiagnostics,
} from "./diagnosticAccess";
import { CommandContext } from "./types";
import { buildPrompt } from "./promptLoader";
import { handleHelp } from "./commands/help";

const PARTICIPANT_ID = "tcl-lsp.tcl";

export function registerTclParticipant(context: vscode.ExtensionContext): void {
  const participant = vscode.chat.createChatParticipant(PARTICIPANT_ID, tclRequestHandler);
  participant.iconPath = new vscode.ThemeIcon("symbol-function");
  participant.followupProvider = { provideFollowups: provideTclFollowups };
  context.subscriptions.push(participant);
}

const tclRequestHandler: vscode.ChatRequestHandler = async (request, context, response, token) => {
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
      return handleCreate(ctx);
    case "explain":
      return handleExplain(ctx);
    case "fix":
      return handleFix(ctx);
    case "validate":
      return handleValidate(ctx);
    case "optimise":
    case "optimize":
      return handleOptimise(ctx);
    case "help":
      return handleHelp(ctx, "tcl");
    default:
      return handleGeneral(ctx);
  }
};

async function handleGeneral(ctx: CommandContext): Promise<vscode.ChatResult> {
  if (!ctx.request.prompt.trim()) {
    ctx.response.markdown(
      "I can help with general Tcl development. Try one of these commands:\n\n" +
        "- `/create` — Create Tcl code from a description\n" +
        "- `/explain` — Explain Tcl code\n" +
        "- `/fix` — Fix issues found by the LSP\n" +
        "- `/validate` — Run LSP diagnostics\n" +
        "- `/optimise` (`/optimize`) — Apply LSP optimisations\n\n" +
        "Or just ask a Tcl question.",
    );
    return {};
  }

  const llmResponse = await sendContextualRequest(ctx, ctx.request.prompt);
  for await (const chunk of llmResponse.text) {
    ctx.response.markdown(chunk);
  }
  return { metadata: { command: "general" } };
}

async function handleCreate(ctx: CommandContext): Promise<vscode.ChatResult> {
  const description = ctx.request.prompt.trim();
  if (!description) {
    ctx.response.markdown("Describe the Tcl code you want to create.");
    return {};
  }

  ctx.response.progress("Generating Tcl code...");

  const llmResponse = await sendContextualRequest(
    ctx,
    `Create Tcl code for this request:\n\n${description}\n\n` +
      `Return ONLY the complete Tcl code in one \`\`\`tcl code block.`,
    { allowAmbientContext: false },
  );
  let responseText = "";
  for await (const chunk of llmResponse.text) {
    responseText += chunk;
  }

  const initialCode = extractCodeBlock(responseText);
  if (!initialCode) {
    ctx.response.markdown("Could not extract Tcl code from the model response.\n\n" + responseText);
    return {};
  }

  const result = await runAgenticLoop(ctx, initialCode, undefined, {
    targetDialect: getActiveDialect(),
  });

  ctx.response.markdown(`## Generated Tcl\n\n\`\`\`tcl\n${result.finalCode}\n\`\`\`\n`);
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

  return { metadata: { command: "create", clean: result.clean, iterations: result.iterations } };
}

async function handleExplain(ctx: CommandContext): Promise<vscode.ChatResult> {
  const code = await resolveIruleCode(ctx);
  if (!code) {
    ctx.response.markdown("Open a Tcl file, select code, or attach `#file` to explain.");
    return {};
  }

  ctx.response.progress("Analysing Tcl code...");
  const llmResponse = await sendContextualRequest(
    ctx,
    `Explain this Tcl code, including control flow and data flow:\n\n\`\`\`tcl\n${code}\n\`\`\``,
    { code, allowAmbientContext: false },
  );
  for await (const chunk of llmResponse.text) {
    ctx.response.markdown(chunk);
  }
  return { metadata: { command: "explain" } };
}

async function resolveDocForValidation(
  ctx: CommandContext,
): Promise<{ doc: vscode.TextDocument; code: string } | undefined> {
  const editor = vscode.window.activeTextEditor;
  if (editor && isTclLanguage(editor.document.languageId)) {
    return { doc: editor.document, code: editor.document.getText() };
  }
  const code = await resolveIruleCode(ctx);
  if (!code) {
    return undefined;
  }
  const doc = await ensureDocumentOpen(code);
  return { doc, code };
}

async function handleValidate(ctx: CommandContext): Promise<vscode.ChatResult> {
  const resolved = await resolveDocForValidation(ctx);
  if (!resolved) {
    ctx.response.markdown("Open a Tcl file or attach one with `#file` to validate.");
    return {};
  }

  ctx.response.progress("Running Tcl LSP analysis...");
  const diagnostics = await waitForDiagnostics(resolved.doc.uri, { timeout: 5000 });
  const categorised = categoriseDiagnostics(diagnostics);

  ctx.response.markdown("## Tcl Validation Report\n");
  if (categorised.all.length === 0) {
    ctx.response.markdown("\nNo issues found.\n");
    return { metadata: { command: "validate", count: 0 } };
  }

  renderDiagnosticSection(ctx.response, "Errors", categorised.errors, resolved.code);
  renderDiagnosticSection(ctx.response, "Security", categorised.security, resolved.code);
  renderDiagnosticSection(ctx.response, "Taint", categorised.taint, resolved.code);
  renderDiagnosticSection(ctx.response, "Performance", categorised.performance, resolved.code);
  renderDiagnosticSection(ctx.response, "Style", categorised.style, resolved.code);
  renderDiagnosticSection(ctx.response, "Optimiser", categorised.optimiser, resolved.code);

  ctx.response.markdown(`\n**Summary**: ${categorised.all.length} issue(s).\n`);
  return { metadata: { command: "validate", count: categorised.all.length } };
}

async function handleFix(ctx: CommandContext): Promise<vscode.ChatResult> {
  const editor = vscode.window.activeTextEditor;
  if (!editor || !isTclLanguage(editor.document.languageId)) {
    ctx.response.markdown("Open a Tcl file to use `/fix`.");
    return {};
  }

  const code = editor.document.getText();
  const diagnostics = await waitForDiagnostics(editor.document.uri, { timeout: 5000 });
  const actionable = diagnostics.filter((d) => d.severity <= vscode.DiagnosticSeverity.Warning);

  if (actionable.length === 0) {
    ctx.response.markdown("No errors or warnings found.");
    return { metadata: { command: "fix", clean: true, iterations: 0 } };
  }

  const result = await runAgenticLoop(ctx, code, editor.document, {
    targetDialect: getActiveDialect(),
  });

  ctx.response.markdown(`## Fixed Tcl\n\n\`\`\`tcl\n${result.finalCode}\n\`\`\`\n`);
  if (!result.clean) {
    renderDiagnosticSection(
      ctx.response,
      "Remaining Issues",
      result.remainingDiagnostics,
      result.finalCode,
    );
  }

  ctx.response.button({
    command: "tclLsp.applyFix",
    title: "Apply fixes",
    arguments: [result.finalCode, editor.document.uri.toString()],
  });

  return { metadata: { command: "fix", clean: result.clean, iterations: result.iterations } };
}

async function handleOptimise(ctx: CommandContext): Promise<vscode.ChatResult> {
  const editor = vscode.window.activeTextEditor;
  if (!editor || !isTclLanguage(editor.document.languageId)) {
    ctx.response.markdown("Open a Tcl file to run optimisations.");
    return {};
  }

  const uri = editor.document.uri.toString();

  ctx.response.progress("Running optimiser...");

  const result = (await ctx.client.sendRequest("workspace/executeCommand", {
    command: "tcl-lsp.optimiseDocument",
    arguments: [uri],
  })) as { optimisations: Array<Record<string, unknown>>; source: string } | null;

  if (!result || !result.optimisations || result.optimisations.length === 0) {
    ctx.response.markdown("No optimisations found.");
    return { metadata: { command: "optimise", count: 0 } };
  }

  // Apply immediately.
  await vscode.commands.executeCommand("tclLsp.applyFix", result.source, uri);

  ctx.response.markdown(
    `Applied **${result.optimisations.length}** optimisation(s):\n\n` +
      result.optimisations
        .map((opt) => {
          const code = opt.code as string;
          const message = opt.message as string;
          const line = (opt.startLine as number) + 1;
          return `- **${code}** (line ${line}): ${message}`;
        })
        .join("\n") +
      "\n",
  );

  return { metadata: { command: "optimise", count: result.optimisations.length } };
}

async function provideTclFollowups(
  result: vscode.ChatResult,
  _context: vscode.ChatContext,
  _token: vscode.CancellationToken,
): Promise<vscode.ChatFollowup[]> {
  const metadata = result.metadata as Record<string, unknown> | undefined;
  const command = metadata?.command as string | undefined;
  switch (command) {
    case "create":
      return [
        { prompt: "Validate this Tcl code", command: "validate" },
        { prompt: "Explain this Tcl code", command: "explain" },
      ];
    case "validate":
      return [
        { prompt: "Fix the issues", command: "fix" },
        { prompt: "Optimise the file", command: "optimise" },
      ];
    default:
      return [
        { prompt: "Validate current Tcl file", command: "validate" },
        { prompt: "Fix current Tcl file", command: "fix" },
      ];
  }
}
