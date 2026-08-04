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

// Tcl language identifiers contributed by this extension. Kept vscode-free so
// it can be imported by lightweight/unit-testable modules without pulling in
// the language client.
//
// A language id must NEVER contain a `.`. VS Code splits a
// `configurationDefaults` override key on `.` while building the
// default-configuration value tree, so a `"[tcl8.4]": {…}` block throws and
// aborts every remaining override in that block — ours and, because the tree
// is shared, other extensions' too (issue #1122). The version-pinned dialect
// ids are therefore undotted (`tcl84`, not `tcl8.4`); the *dialect* strings
// they map to keep their dots (`tcl8.4`), which is a different namespace.

export const TCL_LANGUAGE_IDS = new Set([
  "tcl",
  "tcl-irule",
  "tcl-iapp",
  "tcl-apl",
  "tcl-bigip",
  "tcl84",
  "tcl85",
  "tcl90",
  "tcl91",
  "tcl-synopsys",
  "tcl-cadence",
  "tcl-xilinx",
  "tcl-quartus",
  "tcl-mentor",
  "tcl-expect",
]);

export function isTclLanguage(languageId: string): boolean {
  return TCL_LANGUAGE_IDS.has(languageId);
}
