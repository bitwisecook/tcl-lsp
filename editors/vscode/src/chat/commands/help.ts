import * as vscode from "vscode";
import { CommandContext } from "../types";

/** Feature catalogue rendered as markdown for the /help chat command. */
export async function handleHelp(
  ctx: CommandContext,
  participant: "irule" | "tcl" | "tk",
): Promise<vscode.ChatResult> {
  const topic = ctx.request.prompt.trim().toLowerCase();

  // Try the KCS help database via the LSP server first
  try {
    const lspResult = await queryHelpDb(ctx, topic);
    if (lspResult) {
      ctx.response.markdown(lspResult);
      return { metadata: { command: "help" } };
    }
  } catch {
    // Fall through to hard-coded catalogue
  }

  if (topic) {
    return handleTopicHelp(ctx, participant, topic);
  }

  ctx.response.markdown(buildOverview(participant));
  return { metadata: { command: "help" } };
}

interface HelpResult {
  catalogue?: Record<string, HelpDbFeature[]>;
  results?: HelpDbFeature[];
  screenshots?: Record<string, { data: string; mime_type: string }>;
}

interface HelpDbFeature {
  name: string;
  summary: string;
  surface: string;
  category?: string;
  how_to_use: string;
  file: string;
}

async function queryHelpDb(ctx: CommandContext, topic: string): Promise<string | null> {
  const args: Record<string, unknown> = {};
  if (topic) {
    args.query = topic;
  }

  const result = (await ctx.client.sendRequest("workspace/executeCommand", {
    command: "tcl-lsp.searchHelp",
    arguments: [args.query || "", false],
  })) as HelpResult | null;

  if (!result) {
    return null;
  }

  if (result.catalogue) {
    return renderCatalogue(result.catalogue);
  }

  if (result.results && result.results.length > 0) {
    return renderSearchResults(result.results);
  }

  return null;
}

function renderCatalogue(catalogue: Record<string, HelpDbFeature[]>): string {
  const lines = ["## tcl-lsp Feature Guide\n"];
  for (const [category, features] of Object.entries(catalogue)) {
    lines.push(`### ${category}\n`);
    for (const feat of features) {
      lines.push(`- **${feat.name}** — ${feat.summary}`);
    }
    lines.push("");
  }
  lines.push(
    "_The same analysis engine powers all surfaces: " +
      "the LSP server, MCP tools, Claude Code skills, and VS Code chat._\n",
  );
  lines.push("Ask `/help <topic>` to drill into a specific area.");
  return lines.join("\n");
}

function renderSearchResults(results: HelpDbFeature[]): string {
  const lines: string[] = [];
  for (const feat of results) {
    lines.push(`### ${feat.name}\n`);
    lines.push(feat.summary + "\n");
    if (feat.how_to_use) {
      lines.push(feat.how_to_use + "\n");
    }
  }
  return lines.join("\n");
}

// Fallback: hard-coded feature catalogue

function handleTopicHelp(
  ctx: CommandContext,
  participant: "irule" | "tcl" | "tk",
  topic: string,
): vscode.ChatResult {
  const sections = allSections(participant);
  const matched: string[] = [];

  for (const section of sections) {
    if (
      section.title.toLowerCase().includes(topic) ||
      section.items.some(
        (item) =>
          item.name.toLowerCase().includes(topic) || item.description.toLowerCase().includes(topic),
      )
    ) {
      matched.push(renderSection(section));
    }
  }

  if (matched.length > 0) {
    ctx.response.markdown(matched.join("\n"));
  } else {
    ctx.response.markdown(
      `No features match **${topic}**. Try \`/help\` to see everything, ` +
        `or ask about: \`lsp\`, \`commands\`, \`mcp\`, \`skills\`, ` +
        `\`editors\`, \`dialects\`, \`formatting\`.`,
    );
  }
  return { metadata: { command: "help" } };
}

interface HelpItem {
  name: string;
  description: string;
}

interface HelpSection {
  title: string;
  items: HelpItem[];
}

function renderSection(section: HelpSection): string {
  const lines = [`### ${section.title}\n`];
  for (const item of section.items) {
    lines.push(`- **${item.name}** — ${item.description}`);
  }
  lines.push("");
  return lines.join("\n");
}

function buildOverview(participant: "irule" | "tcl" | "tk"): string {
  const sections = allSections(participant);
  const lines = ["## tcl-lsp Feature Guide\n"];
  for (const section of sections) {
    lines.push(renderSection(section));
  }
  lines.push(
    "_The same analysis engine powers all surfaces: " +
      "the LSP server, MCP tools, Claude Code skills, and VS Code chat._\n",
  );
  lines.push("Ask `/help <topic>` to drill into a specific area.");
  return lines.join("\n");
}

function allSections(participant: "irule" | "tcl" | "tk"): HelpSection[] {
  return [
    chatCommandsSection(participant),
    lspFeaturesSection(),
    editorCommandsSection(),
    mcpToolsSection(),
    claudeSkillsSection(),
    dialectsSection(),
    editorsSection(),
  ];
}

function chatCommandsSection(participant: "irule" | "tcl" | "tk"): HelpSection {
  const common: HelpItem[] = [
    { name: "/create", description: "Generate code from a description" },
    { name: "/explain", description: "Explain what the code does" },
    { name: "/fix", description: "Fix issues using LSP diagnostics" },
    { name: "/validate", description: "Run LSP diagnostics" },
    { name: "/optimise", description: "Apply LSP optimisations" },
    { name: "/help", description: "Show this feature guide" },
  ];

  const iruleExtra: HelpItem[] = [
    { name: "/review", description: "Security and safety review" },
    { name: "/convert", description: "Modernise legacy patterns" },
    { name: "/scaffold", description: "Generate iRule skeleton from events" },
    { name: "/datagroup", description: "Suggest data-group extraction" },
    { name: "/diff", description: "Compare two iRule versions" },
    { name: "/event", description: "Event and command reference" },
    { name: "/diagram", description: "Generate Mermaid flowchart" },
    { name: "/migrate", description: "Convert nginx/Apache/HAProxy to iRule" },
    { name: "/xc", description: "Translate to F5 XC config" },
  ];

  const tkExtra: HelpItem[] = [{ name: "/preview", description: "Open the Tk Preview pane" }];

  let items = [...common];
  if (participant === "irule") {
    items = [...common.slice(0, -1), ...iruleExtra, common[common.length - 1]];
  } else if (participant === "tk") {
    items = [...common.slice(0, -1), ...tkExtra, common[common.length - 1]];
  }

  const title =
    participant === "irule"
      ? "@irule Chat Commands"
      : participant === "tk"
        ? "@tk Chat Commands"
        : "@tcl Chat Commands";

  return { title, items };
}

function lspFeaturesSection(): HelpSection {
  return {
    title: "LSP Features (all editors)",
    items: [
      { name: "Diagnostics", description: "Errors, warnings, security, taint, style checks" },
      { name: "Completions", description: "Commands, variables, switches, procs (Ctrl+Space)" },
      { name: "Hover", description: "Command help, signatures, taint status" },
      { name: "Go to Definition", description: "Jump to definitions (F12)" },
      { name: "Find References", description: "All references to a symbol (Shift+F12)" },
      { name: "Document Symbols", description: "Outline of procs and events (Ctrl+Shift+O)" },
      { name: "Formatting", description: "Configurable indent, brace style, line length" },
      { name: "Code Actions", description: "Quick fixes for diagnostics (Ctrl+.)" },
      { name: "Rename", description: "Rename procs and variables (F2)" },
      { name: "Signature Help", description: "Parameter hints while typing" },
      { name: "Semantic Tokens", description: "Rich highlighting for regex, format strings, etc." },
      { name: "Call Hierarchy", description: "Incoming/outgoing calls for a proc" },
      { name: "Inlay Hints", description: "Inline type and value information" },
      { name: "Folding", description: "Code folding for procs, namespaces, events" },
    ],
  };
}

function editorCommandsSection(): HelpSection {
  return {
    title: "Editor Commands (Command Palette)",
    items: [
      { name: "Select Dialect", description: "Switch between Tcl versions and iRules" },
      { name: "Apply All Optimisations", description: "Apply optimiser suggestions (Ctrl+Alt+O)" },
      { name: "Apply Safe Quick Fixes", description: "Apply all safe fixes at once" },
      { name: "Open Compiler Explorer", description: "Interactive bytecode explorer (Ctrl+Alt+E)" },
      { name: "Open Tk Preview", description: "Live Tk GUI preview pane" },
      { name: "Insert iRule Event Skeleton", description: "Pick events and generate skeleton" },
      { name: "Insert Tcl Template Snippet", description: "Built-in templates" },
      { name: "Translate iRule to F5 XC", description: "Convert to Distributed Cloud config" },
      { name: "Run Runtime Validation", description: "Validate against real tclsh" },
      { name: "Escape/Unescape Selection", description: "Tcl escape transforms" },
      { name: "Base64 Encode/Decode", description: "Base64 transforms" },
      { name: "Extract iRules from Config", description: "Extract from BIG-IP config files" },
    ],
  };
}

function mcpToolsSection(): HelpSection {
  return {
    title: "MCP Tools (AI agents)",
    items: [
      { name: "analyze", description: "Full analysis: diagnostics + symbols + events" },
      { name: "validate", description: "Categorised validation report" },
      { name: "review", description: "Security-focused analysis" },
      { name: "optimize", description: "Optimisation suggestions + rewritten source" },
      { name: "hover / complete", description: "Hover info and completions at a position" },
      { name: "goto_definition / find_references", description: "Navigate symbols" },
      { name: "symbols / code_actions", description: "Document structure and quick fixes" },
      { name: "format_source / rename", description: "Format and rename" },
      { name: "event_info / command_info", description: "iRules registry lookups" },
      { name: "diagram / call_graph", description: "Control flow and call graph data" },
      { name: "xc_translate", description: "iRule to F5 XC translation" },
      { name: "help", description: "This feature catalogue" },
    ],
  };
}

function claudeSkillsSection(): HelpSection {
  return {
    title: "Claude Code Skills",
    items: [
      {
        name: "/irule-*",
        description: "14 iRules skills (create, explain, fix, validate, review, ...)",
      },
      { name: "/tcl-*", description: "5 Tcl skills (create, explain, fix, validate, optimise)" },
      { name: "/tk-create", description: "Create Tk GUI applications" },
      { name: "/ai-help", description: "This feature guide" },
    ],
  };
}

function dialectsSection(): HelpSection {
  return {
    title: "Supported Dialects",
    items: [
      { name: "tcl8.4 / tcl8.5 / tcl8.6 / tcl9.0", description: "Standard Tcl versions" },
      { name: "f5-irules", description: "F5 BIG-IP iRules" },
      { name: "f5-iapps / f5-bigip", description: "F5 iApps and BIG-IP config" },
      { name: "eda-tools", description: "EDA tool Tcl dialects" },
    ],
  };
}

function editorsSection(): HelpSection {
  return {
    title: "Supported Editors",
    items: [
      {
        name: "VS Code",
        description: "Full extension with AI chat, compiler explorer, Tk preview",
      },
      { name: "Neovim", description: "Built-in LSP client (0.8+)" },
      { name: "Emacs", description: "eglot or lsp-mode" },
      { name: "Zed", description: "Extension with slash commands and MCP" },
      { name: "Helix", description: "Built-in LSP via languages.toml" },
      { name: "Sublime Text", description: "LSP package with snippets" },
      { name: "JetBrains", description: "IntelliJ Platform LSP plugin" },
    ],
  };
}
