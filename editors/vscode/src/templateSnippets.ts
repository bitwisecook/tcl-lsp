import { readFileSync } from "fs";
import * as path from "path";

interface RawSnippetDefinition {
  prefix?: string;
  description?: string;
  body?: string | string[];
}

export interface TemplateSnippet {
  name: string;
  prefix: string;
  description: string;
  body: string[];
}

function normaliseBody(body: string | string[] | undefined): string[] {
  if (typeof body === "string") {
    return [body];
  }
  if (Array.isArray(body)) {
    return body;
  }
  return [];
}

export function parseTemplateSnippetCatalog(raw: string): TemplateSnippet[] {
  const parsed = JSON.parse(raw) as Record<string, RawSnippetDefinition>;
  const snippets: TemplateSnippet[] = [];

  for (const [name, definition] of Object.entries(parsed)) {
    if (!definition || typeof definition !== "object") {
      continue;
    }
    if (!definition.prefix || typeof definition.prefix !== "string") {
      continue;
    }

    const body = normaliseBody(definition.body);
    if (body.length === 0) {
      continue;
    }

    snippets.push({
      name,
      prefix: definition.prefix,
      description: definition.description || "",
      body,
    });
  }

  snippets.sort((left, right) => left.name.localeCompare(right.name));
  return snippets;
}

export function loadTemplateSnippets(extensionPath: string): TemplateSnippet[] {
  const snippetsPath = path.join(extensionPath, "snippets", "tcl.code-snippets");
  const raw = readFileSync(snippetsPath, "utf8");
  return parseTemplateSnippetCatalog(raw);
}

export function renderTemplateSnippet(snippet: TemplateSnippet): string {
  return snippet.body.join("\n");
}
