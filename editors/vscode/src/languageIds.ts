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

// @generated:language-ids:begin -- cargo xtask gen-editor-extensions
export const TCL_LANGUAGE_IDS = new Set([
  "tcl",
  "tcl-cadence",
  "tcl-expect",
  "tcl-bigip",
  "tcl-iapp",
  "tcl-irule",
  "tcl-tmsh",
  "tcl-quartus",
  "tcl-mentor",
  "tcl-microchip",
  "tclspec",
  "tcl-synopsys",
  "tcl84",
  "tcl85",
  "tcl86",
  "tcl90",
  "tcl91",
  "tcl-xilinx",
  "tcl-apl",
]);
// @generated:language-ids:end

export function isTclLanguage(languageId: string): boolean {
  return TCL_LANGUAGE_IDS.has(languageId);
}

// Which of our languages owns a given file extension (leading dot included, as
// `path.extname` and VS Code's own `files.associations` spell it), and which
// owns a given whole basename.
//
// Both are projections of the dialect catalogue's `file_extensions` /
// `filenames` axes — the same source `contributes.languages` above is built
// from, so the runtime can never claim an extension the manifest does not
// register. Two hand-written switches used to answer this question, one
// covering 6 of the 25 extensions and the other 4, which is how a `.sdc` file
// that lost its association was offered plain `tcl` instead of `tcl-synopsys`
// (issue #1625).

// @generated:extension-language-ids:begin -- cargo xtask gen-editor-extensions
export const EXTENSION_LANGUAGE_IDS: Record<string, string> = {
  ".tcl": "tcl",
  ".tk": "tcl",
  ".itcl": "tcl",
  ".tm": "tcl",
  ".test": "tcl",
  ".globals": "tcl-cadence",
  ".exp": "tcl-expect",
  ".expect": "tcl-expect",
  ".scf": "tcl-bigip",
  ".iapp": "tcl-iapp",
  ".iappimpl": "tcl-iapp",
  ".impl": "tcl-iapp",
  ".irul": "tcl-irule",
  ".irule": "tcl-irule",
  ".irules": "tcl-irule",
  ".tmsh": "tcl-tmsh",
  ".qsf": "tcl-quartus",
  ".qpf": "tcl-quartus",
  ".qip": "tcl-quartus",
  ".do": "tcl-mentor",
  ".tclspec": "tclspec",
  ".sdc": "tcl-synopsys",
  ".upf": "tcl-synopsys",
  ".xdc": "tcl-xilinx",
  ".apl": "tcl-apl",
};
// @generated:extension-language-ids:end

// @generated:filename-language-ids:begin -- cargo xtask gen-editor-extensions
export const FILENAME_LANGUAGE_IDS: Record<string, string> = {
  "bigip.conf": "tcl-bigip",
  "bigip_base.conf": "tcl-bigip",
  "bigip_gtm.conf": "tcl-bigip",
  "bigip_script.conf": "tcl-bigip",
  "bigip_user.conf": "tcl-bigip",
  presentation: "tcl-apl",
};
// @generated:filename-language-ids:end

/**
 * The most specific Tcl language id for a file, by whole basename first and
 * extension second — the order the server's own `dialect_from_extension`
 * resolves in, since a file claimed by name (`bigip.conf`) has no extension
 * worth claiming. `undefined` when we register neither.
 */
export function tclLanguageIdForPath(path: string): string | undefined {
  const basename = path.split(/[/\\]/).pop() ?? path;
  const lower = basename.toLowerCase();
  const byName = FILENAME_LANGUAGE_IDS[lower];
  if (byName) {
    return byName;
  }
  const dot = lower.lastIndexOf(".");
  return dot < 0 ? undefined : EXTENSION_LANGUAGE_IDS[lower.slice(dot)];
}
