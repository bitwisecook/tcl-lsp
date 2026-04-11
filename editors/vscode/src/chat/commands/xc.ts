import * as vscode from "vscode";
import { CommandContext } from "../types";
import { resolveIruleCode } from "../codeUtils";
import { sendContextualRequest } from "../contextPack";

export async function handleXc(ctx: CommandContext): Promise<vscode.ChatResult> {
  const source = await resolveIruleCode(ctx);

  if (!source) {
    ctx.response.markdown(
      "Provide an iRule to translate to F5 XC configuration. You can:\n\n" +
        "- Have the iRule open in the editor\n" +
        "- Attach a file: `@irule /xc #file:my_irule.irul`\n" +
        "- Paste inline: `@irule /xc when HTTP_REQUEST { pool my_pool }`\n",
    );
    return {};
  }

  ctx.response.progress("Translating iRule to F5 XC configuration...");

  // Try the static translator via LSP custom command first
  const client = ctx.client;
  let staticResult: Record<string, unknown> | null = null;
  if (client) {
    try {
      staticResult = await client.sendRequest("workspace/executeCommand", {
        command: "tcl-lsp.xcTranslate",
        arguments: [source, "both"],
      });
    } catch {
      // LSP command not available — fall through to LLM
    }
  }

  if (staticResult && !staticResult.error) {
    const coverage = (staticResult.coverage_pct as number) ?? 0;
    const terraform = (staticResult.terraform as string) ?? "";
    const items = (staticResult.items as Array<Record<string, string>>) ?? [];

    // Render coverage summary
    const translatable = (staticResult.translatable_count as number) ?? 0;
    const untranslatable = (staticResult.untranslatable_count as number) ?? 0;
    ctx.response.markdown(
      `**Coverage: ${coverage.toFixed(1)}%** — ${translatable} translatable, ${untranslatable} untranslatable\n`,
    );

    // Show untranslatable items
    const untranslatableItems = items.filter((i) => i.status === "untranslatable");
    if (untranslatableItems.length > 0) {
      ctx.response.markdown("\n### Untranslatable Constructs\n");
      for (const item of untranslatableItems) {
        ctx.response.markdown(`- **\`${item.command}\`**: ${item.xc_description}`);
        if (item.note) {
          ctx.response.markdown(` — ${item.note}`);
        }
        ctx.response.markdown("\n");
      }
    }

    // Show advisory items
    const advisoryItems = items.filter((i) => i.status === "advisory");
    if (advisoryItems.length > 0) {
      ctx.response.markdown("\n### Advisory (Separate XC Features)\n");
      for (const item of advisoryItems) {
        ctx.response.markdown(`- **\`${item.command}\`**: ${item.xc_description}\n`);
      }
    }

    // Open scratch tabs for Terraform HCL and JSON API output
    if (terraform) {
      const hclDoc = await vscode.workspace.openTextDocument({
        content: terraform,
        language: "terraform",
      });
      await vscode.window.showTextDocument(hclDoc, {
        preview: false,
        viewColumn: vscode.ViewColumn.Beside,
        preserveFocus: true,
      });
      ctx.response.markdown("\nOpened **Terraform HCL** in a new tab.\n");
    }

    const jsonApi = staticResult.json_api;
    if (jsonApi) {
      const jsonStr = JSON.stringify(jsonApi, null, 2);
      const jsonDoc = await vscode.workspace.openTextDocument({
        content: jsonStr,
        language: "json",
      });
      await vscode.window.showTextDocument(jsonDoc, {
        preview: false,
        viewColumn: vscode.ViewColumn.Beside,
        preserveFocus: true,
      });
      ctx.response.markdown("Opened **JSON API** in a new tab.\n");
    }

    // If coverage is incomplete, invoke the LLM with the static results as
    // deep context so it can advise on untranslatable constructs and suggest
    // alternative XC features (App Stack, WAF tuning, bot defence, etc.)
    if (coverage < 100 && (untranslatableItems.length > 0 || advisoryItems.length > 0)) {
      ctx.response.progress("Consulting AI for untranslatable constructs...");
      const llmResponse = await _askLlmForGaps(ctx, source, staticResult, items);
      if (llmResponse) {
        ctx.response.markdown("\n---\n\n### AI Recommendations\n\n");
        for await (const chunk of llmResponse.text) {
          ctx.response.markdown(chunk);
        }
      }
    }

    return {
      metadata: {
        command: "xc",
        coverage,
        translatable,
        untranslatable,
      },
    };
  }

  // Fallback: use LLM for full translation when static translator unavailable
  ctx.response.progress("Static translator unavailable — using LLM...");

  const llmResponse = await sendContextualRequest(ctx, _buildFullLlmPrompt(source), {
    allowAmbientContext: false,
  });

  for await (const chunk of llmResponse.text) {
    ctx.response.markdown(chunk);
  }

  return { metadata: { command: "xc" } };
}

// LLM helpers

/**
 * Ask the LLM to help with constructs the static translator could not handle,
 * providing the static results as deep context.
 */
async function _askLlmForGaps(
  ctx: CommandContext,
  source: string,
  staticResult: Record<string, unknown>,
  items: Array<Record<string, string>>,
): Promise<Awaited<ReturnType<typeof sendContextualRequest>> | null> {
  const untranslatable = items.filter((i) => i.status === "untranslatable");
  const advisory = items.filter((i) => i.status === "advisory");
  const partial = items.filter((i) => i.status === "partial");
  const coverage = (staticResult.coverage_pct as number) ?? 0;

  // Build a rich prompt with the static translation as context
  const gapList = [...untranslatable, ...partial, ...advisory]
    .map(
      (i) => `- **${i.command}** (${i.status}): ${i.xc_description}${i.note ? ` — ${i.note}` : ""}`,
    )
    .join("\n");

  const prompt =
    `The static iRule→XC translator produced ${coverage.toFixed(1)}% coverage. ` +
    `Below is the original iRule and the items that could not be fully translated.\n\n` +
    `## Original iRule\n\`\`\`tcl\n${source}\n\`\`\`\n\n` +
    `## Items needing attention\n${gapList}\n\n` +
    `For each untranslatable or partial item, provide:\n` +
    `1. **Why** it cannot be statically translated to XC\n` +
    `2. **Recommended XC alternative** — choose from:\n` +
    `   - App Stack (vK8s with custom code for complex procedural logic)\n` +
    `   - XC WAF exclusion rules (for ASM bypass patterns)\n` +
    `   - XC WAF detection control tuning (exclude specific attack types, rule IDs, or bot names)\n` +
    `   - XC Rate Limiting (for rate-based deny rules)\n` +
    `   - XC Bot Defence (for bot detection replacing ASM bot signatures)\n` +
    `   - XC Service Policy with custom match criteria (IP, geo, headers, TLS fingerprint)\n` +
    `   - XC API Discovery / API Definition (for API-specific routing)\n` +
    `   - XC Client-Side Defence (for JavaScript injection patterns)\n` +
    `   - Manual Terraform configuration (provide a template snippet)\n` +
    `3. A **concrete Terraform snippet** or JSON API fragment when possible\n\n` +
    `Keep recommendations practical and actionable. ` +
    `Do not repeat constructs that were already successfully translated.`;

  try {
    return await sendContextualRequest(ctx, prompt, { allowAmbientContext: false });
  } catch {
    return null;
  }
}

/**
 * Build the full LLM prompt for when the static translator is unavailable.
 */
function _buildFullLlmPrompt(source: string): string {
  return (
    `Translate this F5 BIG-IP iRule to F5 Distributed Cloud (XC) configuration.\n\n` +
    `\`\`\`tcl\n${source}\n\`\`\`\n\n` +
    `Generate:\n` +
    `1. Terraform HCL using the volterra provider (volterra_http_loadbalancer, volterra_origin_pool, volterra_service_policy)\n` +
    `2. A coverage summary listing which constructs are translatable and which are not\n\n` +
    `For each untranslatable construct, explain why and suggest an XC alternative (e.g., App Stack, XC WAF, XC bot defence).\n\n` +
    `Map these iRule patterns:\n` +
    `- pool → volterra_origin_pool\n` +
    `- switch on HTTP::path/uri → L7 routes with path matching (prefix, suffix, exact, regex)\n` +
    `- switch on HTTP::host → L7 routes with domain matching\n` +
    `- HTTP::redirect → redirect_route\n` +
    `- HTTP::respond 403/401 → volterra_service_policy deny rule\n` +
    `- HTTP::respond 200 → direct_response_route\n` +
    `- HTTP::header insert/replace/remove → load balancer header processing\n` +
    `- HTTP::header value/exists conditions → route or policy header matching\n` +
    `- HTTP::cookie conditions → route or policy cookie matching\n` +
    `- HTTP::query conditions → query parameter matching\n` +
    `- IP::client_addr conditions → service policy client source matching\n` +
    `- ASM::disable → WAF exclusion rules with app_firewall_detection_control\n` +
    `- RULE_INIT, CLIENT_ACCEPTED, LB_FAILED → no XC equivalent\n` +
    `- CLIENTSSL_HANDSHAKE → XC TLS settings\n` +
    `- ASM events → XC App Firewall\n`
  );
}
