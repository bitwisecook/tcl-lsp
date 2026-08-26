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
 * Selection transforms that rewrite the editor's own text — Tcl escaping and
 * the generic multi-selection edit both entries drive them with.
 *
 * Node-free on purpose: both the node and the browser entry register these
 * commands. Base64 is deliberately NOT here — the node entry keeps its
 * `Buffer`-based implementation byte-for-byte, and the browser entry uses the
 * platform's own `TextEncoder`/`btoa`.
 */

import { window } from "vscode";
import { isTclLanguage } from "./languageIds";

const TCL_ESCAPE_MAP: Record<string, string> = {
  "\\": "\\\\",
  "\n": "\\n",
  "\r": "\\r",
  "\t": "\\t",
  "\b": "\\b",
  "\f": "\\f",
  "\v": "\\v",
  '"': '\\"',
  $: "\\$",
  "[": "\\[",
  "]": "\\]",
};

const TCL_UNESCAPE_MAP: Record<string, string> = {
  "\\": "\\",
  n: "\n",
  r: "\r",
  t: "\t",
  b: "\b",
  f: "\f",
  v: "\v",
  '"': '"',
  $: "$",
  "[": "[",
  "]": "]",
};

export function escapeTclText(text: string): string {
  return text.replace(/[\\\n\r\t\b\f\v"$\[\]]/g, (char) => TCL_ESCAPE_MAP[char]);
}

export function unescapeTclText(text: string): string {
  return text.replace(
    /\\([\\nrtbfv"\$\[\]])/g,
    (_match, escaped: string) => TCL_UNESCAPE_MAP[escaped],
  );
}

export async function transformSelection(
  transform: (text: string) => string,
  pastTenseAction: string,
  infinitiveAction: string,
): Promise<void> {
  const editor = window.activeTextEditor;
  if (!editor || !isTclLanguage(editor.document.languageId)) {
    window.showWarningMessage("Open a Tcl file to transform a selection.");
    return;
  }

  const selections = editor.selections.filter((selection) => !selection.isEmpty);
  if (selections.length === 0) {
    window.showWarningMessage("Select text in a Tcl file first.");
    return;
  }

  const applied = await editor.edit((editBuilder) => {
    for (const selection of selections) {
      const input = editor.document.getText(selection);
      editBuilder.replace(selection, transform(input));
    }
  });

  if (!applied) {
    window.showWarningMessage(`Failed to ${infinitiveAction} selection.`);
    return;
  }

  window.showInformationMessage(
    `${pastTenseAction} ${selections.length} selection${selections.length === 1 ? "" : "s"}.`,
  );
}
