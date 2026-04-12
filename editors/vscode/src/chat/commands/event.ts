import * as vscode from "vscode";
import { CommandContext } from "../types";
import { sendContextualRequest } from "../contextPack";

interface IruleEventInfo {
  event: string;
  known: boolean;
  deprecated: boolean;
  validCommandCount: number;
  sampleCommands: string[];
}

interface IruleCommandInfo {
  found: boolean;
  command: string;
  summary?: string;
  synopsis?: string[];
  switches?: string[];
  validEvents?: string[];
  anyEvent?: boolean;
  quality?: string;
  qualityNote?: string;
}

function extractAllCapsTokens(prompt: string): string[] {
  return [...prompt.matchAll(/\b([A-Z][A-Z0-9_]{2,})\b/g)].map((m) => m[1]);
}

export function detectQueryType(prompt: string): {
  type: "event" | "command" | "general";
  value: string;
} {
  const trimmed = prompt.trim();
  const commandMatch = trimmed.match(/\b([A-Z]+::\w+)\b/i);
  if (commandMatch) {
    return { type: "command", value: commandMatch[1] };
  }

  const capsTokens = extractAllCapsTokens(trimmed.toUpperCase());
  if (capsTokens.length > 0) {
    // Prefer explicit EVENT-ish token if one appears.
    const preferred = capsTokens.find(
      (tok) => tok.includes("REQUEST") || tok.includes("RESPONSE") || tok.includes("INIT"),
    );
    return { type: "event", value: preferred ?? capsTokens[0] };
  }

  return { type: "general", value: trimmed };
}

async function fetchEventInfo(
  ctx: CommandContext,
  eventName: string,
): Promise<IruleEventInfo | undefined> {
  try {
    const response = (await ctx.client.sendRequest("workspace/executeCommand", {
      command: "tcl-lsp.describeIruleEvent",
      arguments: [eventName],
    })) as IruleEventInfo | null;
    return response ?? undefined;
  } catch {
    return undefined;
  }
}

async function fetchCommandInfo(
  ctx: CommandContext,
  commandName: string,
): Promise<IruleCommandInfo | undefined> {
  try {
    const response = (await ctx.client.sendRequest("workspace/executeCommand", {
      command: "tcl-lsp.describeIruleCommand",
      arguments: [commandName],
    })) as IruleCommandInfo | null;
    return response ?? undefined;
  } catch {
    return undefined;
  }
}

function formatEventContext(info: IruleEventInfo | undefined): string {
  if (!info) return "No deterministic registry metadata available.";

  const lines: string[] = [];
  lines.push(`- Event known by registry: ${info.known ? "yes" : "no"}`);
  lines.push(`- Event deprecated: ${info.deprecated ? "yes" : "no"}`);
  lines.push(`- Commands marked valid: ${info.validCommandCount}`);
  if (info.sampleCommands.length > 0) {
    lines.push(`- Sample valid commands: ${info.sampleCommands.slice(0, 25).join(", ")}`);
  }
  return lines.join("\n");
}

function formatCommandContext(info: IruleCommandInfo | undefined): string {
  if (!info) return "No deterministic registry metadata available.";
  if (!info.found) return `- Command '${info.command}' not found in iRules registry.`;

  const lines: string[] = [];
  lines.push(`- Command found: ${info.command}`);
  if (info.summary) {
    lines.push(`- Summary: ${info.summary}`);
  }
  if ((info.synopsis ?? []).length > 0) {
    lines.push(`- Synopsis: ${(info.synopsis ?? []).slice(0, 3).join(" | ")}`);
  }
  if ((info.switches ?? []).length > 0) {
    lines.push(`- Switches: ${(info.switches ?? []).slice(0, 20).join(", ")}`);
  }
  if ((info.validEvents ?? []).length > 0 || info.anyEvent) {
    const eventText = info.anyEvent
      ? "Any event"
      : (info.validEvents ?? []).slice(0, 20).join(", ");
    lines.push(`- Valid events: ${eventText}`);
  }
  if (info.quality) {
    lines.push(`- Metadata quality: ${info.quality}`);
  }
  if (info.qualityNote) {
    lines.push(`- Quality note: ${info.qualityNote}`);
  }
  return lines.join("\n");
}

export async function handleEvent(ctx: CommandContext): Promise<vscode.ChatResult> {
  if (!ctx.request.prompt.trim()) {
    ctx.response.markdown(
      "Ask about an event or command. For example:\n\n" +
        "- `@irule /event HTTP_REQUEST` — What commands work in HTTP_REQUEST?\n" +
        "- `@irule /event HTTP::header` — Which events support HTTP::header?\n" +
        "- `@irule /event What events fire for SSL?` — General event questions\n",
    );
    return {};
  }

  const query = detectQueryType(ctx.request.prompt);

  let eventInfo: IruleEventInfo | undefined;
  let commandInfo: IruleCommandInfo | undefined;
  if (query.type === "event") {
    ctx.response.progress(`Loading registry metadata for ${query.value}...`);
    eventInfo = await fetchEventInfo(ctx, query.value);
  } else if (query.type === "command") {
    ctx.response.progress(`Loading registry metadata for ${query.value}...`);
    commandInfo = await fetchCommandInfo(ctx, query.value);
  }

  const eventContext = formatEventContext(eventInfo);
  const commandContext = formatCommandContext(commandInfo);

  let specificPrompt: string;
  switch (query.type) {
    case "event":
      specificPrompt =
        `Provide a practical iRules reference for event **${query.value}**:\n\n` +
        `1. When it fires\n` +
        `2. Common commands to use\n` +
        `3. Available request/response data\n` +
        `4. Performance and safety notes\n` +
        `5. Minimal useful example\n\n` +
        `Registry metadata (authoritative):\n${eventContext}`;
      break;
    case "command":
      specificPrompt =
        `Provide a practical iRules reference for command **${query.value}**:\n\n` +
        `1. Syntax and options\n` +
        `2. Valid/invalid events\n` +
        `3. Typical usage patterns\n` +
        `4. Common mistakes\n\n` +
        `Registry metadata (authoritative):\n${commandContext}`;
      break;
    default:
      specificPrompt =
        `Answer this iRules event/command question:\n${ctx.request.prompt}\n\n` +
        `Use deterministic registry facts where available and clearly separate facts from guidance.`;
      break;
  }

  ctx.response.markdown("## Event Reference\n");
  if (query.type === "event" && eventInfo) {
    ctx.response.markdown(`\n### Registry Facts\n\n${eventContext}\n`);
  }
  if (query.type === "command" && commandInfo) {
    ctx.response.markdown(`\n### Registry Facts\n\n${commandContext}\n`);
  }

  const llmResponse = await sendContextualRequest(ctx, specificPrompt, {
    includeEventMetadata: false,
  });
  for await (const chunk of llmResponse.text) {
    ctx.response.markdown(chunk);
  }

  return {
    metadata: { command: "event", queryType: query.type, queryValue: query.value },
  };
}
