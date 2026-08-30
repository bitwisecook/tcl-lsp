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

//! `llookup` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "llookup",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Returns a list of values corresponding to the given key in a multimap.",
            synopsis: &["llookup MMAP KEY"],
            snippet: "A *multimap* is a flat Tcl list of `{key value}` pairs — the same structure returned by `[ASM::violation details]`.  Because the same key can appear more than once, `llookup` returns **a list** of every value whose key matches *KEY*.\n\nReturns an empty string when *KEY* is absent or *MMAP* is not a properly structured multimap.\n\nEquivalent Tcl (what `llookup` replaces):\n```tcl\nset r {}\nforeach pair $mmap {\n    if {[lindex $pair 0] eq $key} {\n        lappend r [lindex $pair 1]\n    }\n}\n```",
            source: "https://clouddocs.f5.com/api/irules/llookup.html",
            examples: "# Iterate violations in parallel using llookup\nwhen ASM_REQUEST_DONE {\n    set details [ASM::violation details]\n    foreach viol_name       [llookup $details viol_name] \\\n            sanity_status   [llookup $details http_sanity_checks_status] \\\n            sub_viol_status [llookup $details http_sub_violation_status] {\n        log local0.info \"$viol_name $sanity_status $sub_viol_status\"\n    }\n}",
            return_value: "A Tcl list of values matching *KEY*.  When used with `[ASM::violation details]`, binary values such as `http_sub_violation` and `sig_data.kw_data.buffer` are base64-encoded.",
        }),
        forms: &[FormSpec {
            synopsis: "llookup MMAP KEY",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            reads: true,
            connection_side: ConnectionSide::Global,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
