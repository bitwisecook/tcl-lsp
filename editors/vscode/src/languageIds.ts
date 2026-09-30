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
// is shared, other extensions' too. The version-pinned dialect
// ids are therefore undotted (`tcl84`, not `tcl8.4`); the *dialect* strings
// they map to keep their dots (`tcl8.4`), which is a different namespace.

// @generated:language-ids:begin -- cargo xtask gen-editor-extensions
export const TCL_LANGUAGE_IDS = new Set([
  "tcl",
  "tcl-expect",
  "tcl-bigip",
  "tcl-iapp",
  "tcl-irule",
  "tcl-tmsh",
  "tcl-jim",
  "tclspec",
  "sslictcl",
  "tcl84",
  "tcl85",
  "tcl86",
  "tcl90",
  "tcl91",
  "tcl-cadence",
  "tcl-quartus",
  "tcl-mentor",
  "tcl-microchip",
  "tcl-synopsys",
  "tcl-xilinx",
  "tcl-apl",
]);
// @generated:language-ids:end

export function isTclLanguage(languageId: string): boolean {
  return TCL_LANGUAGE_IDS.has(languageId);
}

// The dialect a language id implies — what the status bar shows for a document
// until the server has said which dialect it analyses the document under.
//
// Keys are *language ids* (undotted); values are *dialect* names, which keep
// their dots. The two namespaces are distinct: `tcl84` is what VS Code calls
// the language, `tcl8.4` is what the server calls the dialect. Every
// environment's editor identity is a key, and so is each spelling a client
// may send that selects an environment without being its identity.

// @generated:language-id-dialects:begin -- cargo xtask gen-editor-extensions
export const LANGUAGE_ID_DIALECTS: Record<string, string> = {
  "tcl-expect": "expect",
  "tcl-bigip": "f5-bigip",
  "tcl-iapp": "f5-iapps",
  "tcl-irule": "f5-irules",
  "tcl-tmsh": "f5-tmsh",
  "tcl-jim": "jim",
  tclspec: "spectcl",
  sslictcl: "sslictcl",
  tcl84: "tcl8.4",
  tcl85: "tcl8.5",
  tcl86: "tcl8.6",
  tcl90: "tcl9.0",
  tcl91: "tcl9.1",
  "tcl-cadence": "cadence-eda-tcl",
  "tcl-quartus": "intel-quartus-eda-tcl",
  "tcl-mentor": "mentor-eda-tcl",
  "tcl-microchip": "microchip-libero-eda-tcl",
  "tcl-synopsys": "synopsys-eda-tcl",
  "tcl-xilinx": "xilinx-eda-tcl",
  "tcl-apl": "f5-iapps",
  "tcl-bpf": "bpf",
  "tcl-libero": "microchip-libero-eda-tcl",
  "tcl-spec": "spectcl",
};
// @generated:language-id-dialects:end

// Which of our languages owns a given file extension (leading dot included, as
// `path.extname` and VS Code's own `files.associations` spell it), and which
// owns a given whole basename.
//
// Both are projections of the dialect catalogue's `file_extensions` /
// `filenames` axes — the same source `contributes.languages` above is built
// from, so the runtime can never claim an extension the manifest does not
// register. A hand-written switch answering this question would have to
// enumerate every one of the 25 registered extensions to avoid offering,
// say, plain `tcl` instead of `tcl-synopsys` for a `.sdc` file that lost
// its association.

// @generated:extension-language-ids:begin -- cargo xtask gen-editor-extensions
export const EXTENSION_LANGUAGE_IDS: Record<string, string> = {
  ".tcl": "tcl",
  ".tk": "tcl",
  ".itcl": "tcl",
  ".tm": "tcl",
  ".test": "tcl",
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
  ".tclspec": "tclspec",
  ".sslictcl": "sslictcl",
  ".globals": "tcl-cadence",
  ".qsf": "tcl-quartus",
  ".qpf": "tcl-quartus",
  ".qip": "tcl-quartus",
  ".do": "tcl-mentor",
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
  const byName = lookup(FILENAME_LANGUAGE_IDS, lower);
  if (byName) {
    return byName;
  }
  const dot = lower.lastIndexOf(".");
  return dot < 0 ? undefined : lookup(EXTENSION_LANGUAGE_IDS, lower.slice(dot));
}

/**
 * Read `key` from a generated map, ignoring everything the object inherits.
 *
 * The keys here come from a *filename*, so a plain `map[key]` answers for
 * `constructor` and `__proto__` with something off `Object.prototype` — a file
 * named `constructor` would resolve to a function, not a language id.
 */
function lookup(map: Record<string, string>, key: string): string | undefined {
  return Object.prototype.hasOwnProperty.call(map, key) ? map[key] : undefined;
}
