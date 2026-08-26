// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Parsing and rendering for the bundled `snippets/tcl.code-snippets` catalogue.
 *
 * Split out of `./templateSnippets` (which keeps the `fs` reader) so the
 * browser entry can read the same bundled file through
 * `vscode.workspace.fs` and share the parser rather than duplicating it.
 */

/** The bundled catalogue's path, relative to the extension root. */
export const TEMPLATE_SNIPPET_RELATIVE_PATH = ["snippets", "tcl.code-snippets"] as const;

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

export function renderTemplateSnippet(snippet: TemplateSnippet): string {
  return snippet.body.join("\n");
}
