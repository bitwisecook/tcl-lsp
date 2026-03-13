import * as vscode from "vscode";
import * as fs from "fs";
import * as path from "path";
import { getClient, isAiEnabled } from "../extension";
import { sendContextualRequest } from "./contextPack";
import { buildPrompt } from "./promptLoader";
import { CommandContext } from "./types";
import { handleCreate } from "./commands/create";
import { handleExplain } from "./commands/explain";
import { handleFix } from "./commands/fix";
import { handleValidate } from "./commands/validate";
import { handleReview } from "./commands/review";
import { handleConvert } from "./commands/convert";
import { handleOptimise } from "./commands/optimise";
import { handleScaffold } from "./commands/scaffold";
import { handleDatagroup } from "./commands/datagroup";
import { handleDiff } from "./commands/diff";
import { handleEvent } from "./commands/event";
import { handleMigrate } from "./commands/migrate";
import { handleDiagram } from "./commands/diagram";
import { handleTest } from "./commands/test";
import { handleXc } from "./commands/xc";
import { handleHelp } from "./commands/help";

const PARTICIPANT_ID = "tcl-lsp.irule";

export function registerIruleParticipant(context: vscode.ExtensionContext): void {
  const participant = vscode.chat.createChatParticipant(PARTICIPANT_ID, iruleRequestHandler);
  participant.iconPath = new vscode.ThemeIcon("symbol-event");
  participant.followupProvider = { provideFollowups: provideIruleFollowups };
  context.subscriptions.push(participant);
}

/** Signal the screenshot pipeline when an AI request starts/succeeds/completes. */
function signalAiMarker(fileName: ".ai-started" | ".ai-success" | ".ai-done"): void {
  const dir = process.env.SCREENSHOT_OUTPUT_DIR;
  if (!dir) return;
  try {
    fs.writeFileSync(path.join(dir, fileName), "", "utf8");
  } catch {}
}

function signalAiStarted(): void {
  signalAiMarker(".ai-started");
}

function signalAiDone(): void {
  signalAiMarker(".ai-done");
}

function signalAiSuccess(): void {
  signalAiMarker(".ai-success");
}

const iruleRequestHandler: vscode.ChatRequestHandler = async (
  request,
  context,
  response,
  token,
) => {
  signalAiStarted();
  let successful = false;

  try {
    if (!isAiEnabled()) {
      response.markdown(
        "AI features are disabled. Enable them in settings (`tclLsp.ai.enabled`) " +
          "or run the **Tcl: Toggle AI Features** command.",
      );
      return {};
    }

    const systemPrompt = buildPrompt("f5-irules");
    const ctx: CommandContext = {
      request,
      context,
      response,
      token,
      systemPrompt,
      client: getClient(),
    };

    let result: vscode.ChatResult;
    switch (request.command) {
      case "create":
        result = await handleCreate(ctx);
        break;
      case "explain":
        result = await handleExplain(ctx);
        break;
      case "fix":
        result = await handleFix(ctx);
        break;
      case "validate":
        result = await handleValidate(ctx);
        break;
      case "review":
        result = await handleReview(ctx);
        break;
      case "convert":
        result = await handleConvert(ctx);
        break;
      case "optimise":
      case "optimize":
        result = await handleOptimise(ctx);
        break;
      case "scaffold":
        result = await handleScaffold(ctx);
        break;
      case "datagroup":
        result = await handleDatagroup(ctx);
        break;
      case "diff":
        result = await handleDiff(ctx);
        break;
      case "event":
        result = await handleEvent(ctx);
        break;
      case "migrate":
        result = await handleMigrate(ctx);
        break;
      case "diagram":
        result = await handleDiagram(ctx);
        break;
      case "test":
        result = await handleTest(ctx);
        break;
      case "xc":
        result = await handleXc(ctx);
        break;
      case "help":
        result = await handleHelp(ctx, "irule");
        break;
      default:
        result = await handleGeneral(ctx);
        break;
    }

    successful = true;
    return result;
  } finally {
    if (successful) {
      signalAiSuccess();
    }
    signalAiDone();
  }
};

/** Handle general iRules Q&A when no slash command is given. */
async function handleGeneral(ctx: CommandContext): Promise<vscode.ChatResult> {
  if (!ctx.request.prompt.trim()) {
    ctx.response.markdown(
      "I can help with F5 BIG-IP iRules. Try one of these commands:\n\n" +
        "- `/create` — Create a new iRule from a description\n" +
        "- `/explain` — Explain what an iRule does\n" +
        "- `/fix` — Fix issues found by the LSP\n" +
        "- `/validate` — Run LSP diagnostics\n" +
        "- `/review` — Security and safety review\n" +
        "- `/convert` — Modernise legacy patterns (matchclass, unbraced expr, etc.)\n" +
        "- `/optimise` (`/optimize`) — Apply LSP optimisations with explanations\n" +
        "- `/scaffold` — Generate an iRule skeleton from events\n" +
        "- `/datagroup` — Suggest data-group extraction for inline lookups\n" +
        "- `/diff` — Compare two iRule versions\n" +
        "- `/event` — Event/command reference\n" +
        "- `/diagram` — Generate a Mermaid flowchart of an iRule's logic\n" +
        "- `/migrate` — Convert nginx/Apache/HAProxy config to an iRule\n" +
        "- `/test` — Generate test script using the Event Orchestrator framework\n" +
        "- `/xc` — Translate an iRule to F5 XC routes and service policies\n\n" +
        "Or just ask a question about iRules!",
    );
    return {};
  }

  const llmResponse = await sendContextualRequest(ctx, ctx.request.prompt);
  for await (const chunk of llmResponse.text) {
    ctx.response.markdown(chunk);
  }

  return { metadata: { command: "general" } };
}

/** Provide context-aware follow-up suggestions. */
async function provideIruleFollowups(
  result: vscode.ChatResult,
  _context: vscode.ChatContext,
  _token: vscode.CancellationToken,
): Promise<vscode.ChatFollowup[]> {
  const metadata = result.metadata as Record<string, unknown> | undefined;
  const command = metadata?.command as string | undefined;

  switch (command) {
    case "create":
      return [
        { prompt: "Validate this iRule", command: "validate" },
        { prompt: "Review for security issues", command: "review" },
        { prompt: "Explain what this iRule does", command: "explain" },
      ];
    case "explain":
      return [
        { prompt: "Suggest improvements", label: "Suggest improvements" },
        { prompt: "Review for security issues", command: "review" },
      ];
    case "fix":
      if (metadata?.clean) {
        return [{ prompt: "Review the fixed iRule for security", command: "review" }];
      }
      return [
        { prompt: "Try fixing the remaining issues", command: "fix" },
        { prompt: "Explain the remaining issues", command: "explain" },
      ];
    case "validate":
      if (((metadata?.count as number) ?? 0) > 0) {
        return [
          { prompt: "Fix the issues found", command: "fix" },
          { prompt: "Explain these issues", command: "explain" },
        ];
      }
      return [{ prompt: "Review for security concerns", command: "review" }];
    case "review":
      return [
        { prompt: "Fix the security issues", command: "fix" },
        {
          prompt: "Create a more secure version",
          command: "create",
          label: "Rewrite securely",
        },
      ];
    case "convert":
      return [
        { prompt: "Validate the modernised iRule", command: "validate" },
        { prompt: "Review for security issues", command: "review" },
      ];
    case "optimise":
      return [
        { prompt: "Validate after optimisation", command: "validate" },
        { prompt: "Review for security issues", command: "review" },
      ];
    case "scaffold":
      return [
        { prompt: "Fill in the skeleton", command: "create", label: "Fill in skeleton" },
        { prompt: "Generate tests", command: "test" },
        { prompt: "Validate the skeleton", command: "validate" },
      ];
    case "datagroup":
      return [
        { prompt: "Apply data-group suggestions", command: "fix", label: "Apply suggestions" },
        { prompt: "Validate the iRule", command: "validate" },
      ];
    case "diff":
      return [
        { prompt: "Review the new version", command: "review" },
        { prompt: "Explain the new version", command: "explain" },
      ];
    case "event":
      return [
        { prompt: "Create an iRule for this event", command: "create" },
        { prompt: "Scaffold with this event", command: "scaffold" },
      ];
    case "migrate":
      return [
        { prompt: "Validate the converted iRule", command: "validate" },
        { prompt: "Fix any issues", command: "fix" },
        { prompt: "Security review", command: "review" },
      ];
    case "diagram":
      return [
        { prompt: "Explain this iRule in detail", command: "explain" },
        { prompt: "Review for security issues", command: "review" },
        { prompt: "Validate this iRule", command: "validate" },
      ];
    case "test":
      return [
        { prompt: "Review the iRule for issues", command: "review" },
        { prompt: "Validate the iRule", command: "validate" },
      ];
    case "xc":
      return [
        { prompt: "Explain what couldn't be translated", command: "explain" },
        { prompt: "Review security implications", command: "review" },
        { prompt: "Validate the original iRule", command: "validate" },
      ];
    default:
      return [
        { prompt: "Create a new iRule", command: "create", label: "Create iRule" },
        { prompt: "Validate current iRule", command: "validate", label: "Validate" },
      ];
  }
}
