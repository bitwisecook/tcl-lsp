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

export const HOOK_HOST_PACK = `speclib hosthooks 2.0 {
    command host_hook_guard {
        dialects tcl8.6
        arity 1
        context_gate -inputs {words} {words ctx} {
            if {[lindex $words 0] eq {blocked}} { reject {host-hook-ran} }
        }
    }
}
`;

/** A word-sensitive body must run, then answer the changed content correctly. */
export async function runHookHostProof(session, uri, check, wait) {
    const first = "host_hook_guard blocked\nhost_hook_guard allowed\n";
    session.notify("textDocument/didOpen", {
        textDocument: { uri, languageId: "tcl", version: 1, text: first },
    });
    const reportsAt = (line) => {
        const diagnostics = session.diagnostics.get(uri);
        if (!Array.isArray(diagnostics)) return null;
        const reports = diagnostics.filter((d) => d.code === "W142" && d.message.includes("host-hook-ran"));
        return reports.length === 1 && reports[0].range.start.line === line ? reports : null;
    };
    const cold = await wait(() => reportsAt(0));
    check("a non-tabulated SpecTcl hook runs on the actual host", Boolean(cold), JSON.stringify(cold));

    const changed = "host_hook_guard allowed\nhost_hook_guard blocked\n";
    session.notify("textDocument/didChange", {
        textDocument: { uri, version: 2 },
        contentChanges: [{ text: changed }],
    });
    const warm = await wait(() => reportsAt(1));
    check("the warm SpecTcl host answers changed word content", Boolean(warm), JSON.stringify(warm));
}
