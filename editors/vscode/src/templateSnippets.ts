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

import { readFileSync } from "fs";
import * as path from "path";
import {
  parseTemplateSnippetCatalog,
  TEMPLATE_SNIPPET_RELATIVE_PATH,
  type TemplateSnippet,
} from "./templateSnippetsCatalog";

// The parser and the snippet type live in `./templateSnippetsCatalog` (which
// imports no node builtin) so the browser entry can share them; this module is
// the node-side reader. Re-exported so existing importers keep working.
export {
  parseTemplateSnippetCatalog,
  renderTemplateSnippet,
  TEMPLATE_SNIPPET_RELATIVE_PATH,
  type TemplateSnippet,
} from "./templateSnippetsCatalog";

export function loadTemplateSnippets(extensionPath: string): TemplateSnippet[] {
  const snippetsPath = path.join(extensionPath, ...TEMPLATE_SNIPPET_RELATIVE_PATH);
  const raw = readFileSync(snippetsPath, "utf8");
  return parseTemplateSnippetCatalog(raw);
}
