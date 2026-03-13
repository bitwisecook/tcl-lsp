import * as vscode from "vscode";
import { CommandContext } from "../types";
import { resolveIruleCode, extractCodeBlock } from "../codeUtils";
import { sendContextualRequest } from "../contextPack";
import { runAgenticLoop } from "../agenticLoop";
import { renderDiagnosticSection } from "../diagnosticAccess";

type SourceFormat = "nginx" | "apache" | "haproxy" | "unknown";

function detectSourceFormat(config: string): SourceFormat {
  const lower = config.toLowerCase();

  // nginx patterns
  if (
    /\bserver\s*\{/.test(lower) ||
    /\blocation\s+[/~]/.test(lower) ||
    /\bproxy_pass\b/.test(lower) ||
    /\bnginx\b/.test(lower)
  ) {
    return "nginx";
  }

  // Apache patterns
  if (
    /\brewriterule\b/.test(lower) ||
    /\brewritecond\b/.test(lower) ||
    /\bproxypass\b/.test(lower) ||
    /\b<virtualhost\b/.test(lower) ||
    /\b<directory\b/.test(lower) ||
    /\bapache\b/.test(lower)
  ) {
    return "apache";
  }

  // HAProxy patterns
  if (
    /\bfrontend\b/.test(lower) ||
    /\bbackend\b/.test(lower) ||
    /\buse_backend\b/.test(lower) ||
    /\bacl\s+\w+/.test(lower) ||
    /\bhaproxy\b/.test(lower)
  ) {
    return "haproxy";
  }

  return "unknown";
}

const FORMAT_LABELS: Record<SourceFormat, string> = {
  nginx: "nginx",
  apache: "Apache",
  haproxy: "HAProxy",
  unknown: "unknown",
};

export async function handleMigrate(ctx: CommandContext): Promise<vscode.ChatResult> {
  // Try to get config from inline code, file reference, or active editor
  let config = await resolveIruleCode(ctx);

  // Also check for inline code blocks in the prompt itself
  if (!config) {
    const extracted = extractCodeBlock(ctx.request.prompt);
    if (extracted) {
      config = extracted;
    }
  }

  // If prompt has substantial text (not just a command), treat it as inline config
  if (!config && ctx.request.prompt.trim().length > 20) {
    config = ctx.request.prompt.trim();
  }

  if (!config) {
    ctx.response.markdown(
      "Provide a configuration to convert to an iRule. You can:\n\n" +
        "- Paste the config inline: `@irule /migrate server { listen 80; ... }`\n" +
        "- Attach a file: `@irule /migrate #file:nginx.conf`\n" +
        "- Have the config file open in the editor\n\n" +
        "Supported formats: **nginx**, **Apache**, **HAProxy**",
    );
    return {};
  }

  const format = detectSourceFormat(config);

  ctx.response.progress(`Detected ${FORMAT_LABELS[format]} configuration. Converting to iRule...`);

  const migrationGuidance =
    format === "nginx"
      ? `Map nginx constructs:\n` +
        `- location → switch on [HTTP::uri] or [HTTP::path]\n` +
        `- proxy_pass → pool selection\n` +
        `- rewrite → HTTP::uri / HTTP::redirect\n` +
        `- add_header → HTTP::header insert\n` +
        `- return 301/302 → HTTP::redirect\n` +
        `- if ($host) → string match or class match on [HTTP::host]\n`
      : format === "apache"
        ? `Map Apache constructs:\n` +
          `- RewriteRule → HTTP::uri / HTTP::redirect\n` +
          `- RewriteCond → if/switch conditions\n` +
          `- ProxyPass → pool selection\n` +
          `- Header set → HTTP::header insert/replace\n` +
          `- <VirtualHost> → switch on [HTTP::host]\n` +
          `- <Location> → switch on [HTTP::path]\n`
        : format === "haproxy"
          ? `Map HAProxy constructs:\n` +
            `- acl → if conditions or class match\n` +
            `- use_backend → pool selection\n` +
            `- http-request redirect → HTTP::redirect\n` +
            `- http-request set-header → HTTP::header replace\n` +
            `- http-request add-header → HTTP::header insert\n` +
            `- frontend bind → handled by virtual server (note in comments)\n`
          : `Auto-detect the configuration format and map constructs appropriately.\n`;

  const llmResponse = await sendContextualRequest(
    ctx,
    `Convert this ${FORMAT_LABELS[format]} configuration to an F5 BIG-IP iRule.\n\n` +
      `Source configuration:\n\`\`\`\n${config}\n\`\`\`\n\n` +
      `${migrationGuidance}\n` +
      `General requirements:\n` +
      `- Use appropriate events (typically HTTP_REQUEST for routing, HTTP_RESPONSE for headers)\n` +
      `- Use data-groups (class match) for large lookup tables instead of inline switch\n` +
      `- Follow iRules security best practices\n` +
      `- Add comments explaining the mapping from the original config\n` +
      `- Note anything that cannot be directly translated (e.g., backend health checks)\n\n` +
      `Return ONLY the complete iRule in a single \`\`\`tcl code block.`,
    { allowAmbientContext: false },
  );
  let responseText = "";
  for await (const chunk of llmResponse.text) {
    responseText += chunk;
  }

  const initialCode = extractCodeBlock(responseText);
  if (!initialCode) {
    ctx.response.markdown("Failed to generate iRule. Here is the raw response:\n\n" + responseText);
    return {};
  }

  // Run agentic loop to validate the generated iRule
  const result = await runAgenticLoop(ctx, initialCode, undefined, { targetDialect: "f5-irules" });

  ctx.response.markdown(
    `## Converted iRule (from ${FORMAT_LABELS[format]})\n\n\`\`\`tcl\n${result.finalCode}\n\`\`\`\n`,
  );

  if (result.clean) {
    ctx.response.markdown(`\nConverted and validated clean in ${result.iterations} iteration(s).`);
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

  return {
    metadata: { command: "migrate", format, clean: result.clean },
  };
}
