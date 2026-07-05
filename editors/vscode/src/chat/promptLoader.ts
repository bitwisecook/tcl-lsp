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
 * Dialect-aware system prompt composition.
 *
 * Gathers all prompt fragments whose dialect set includes the active dialect,
 * then appends any user-configured extra prompts from the
 * `tclLsp.ai.extraPrompts` setting.
 *
 * Source of truth: ai/prompts/manifest.json + ai/prompts/*.md
 * (copied to src/chat/canonical/ at build time by the Makefile).
 */

import * as vscode from "vscode";
import manifest from "./canonical/manifest.json";
import irulesMd from "./canonical/irules_system.md";
import tclMd from "./canonical/tcl_system.md";
import tkMd from "./canonical/tk_system.md";

/** Map from prompt filename to its inlined content. */
const PROMPT_CONTENT: Readonly<Record<string, string>> = {
  "irules_system.md": irulesMd,
  "tcl_system.md": tclMd,
  "tk_system.md": tkMd,
};

interface ExtraPrompt {
  text: string;
  dialects: string[];
}

/**
 * Build the composite system prompt for the given dialect.
 *
 * 1. Selects all canonical prompts whose manifest entry includes `dialect`.
 * 2. Appends any user-configured extra prompts whose dialect list includes
 *    `dialect` or the wildcard `"*"`.
 */
export function buildPrompt(dialect: string): string {
  const parts: string[] = [];

  for (const entry of manifest.prompts) {
    if (entry.dialects.includes(dialect)) {
      const content = PROMPT_CONTENT[entry.file];
      if (content) {
        parts.push(content);
      }
    }
  }

  const extras = vscode.workspace
    .getConfiguration("tclLsp.ai")
    .get<ExtraPrompt[]>("extraPrompts", []);

  for (const extra of extras) {
    if (extra.dialects.includes("*") || extra.dialects.includes(dialect)) {
      parts.push(extra.text);
    }
  }

  return parts.join("\n\n");
}
