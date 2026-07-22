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

//! `llength` — return the number of elements in a list.
//
// Manpage comparison across Tcl 8.4, 8.5, 8.6, 9.0, and 9.1
// (tcl-lang.org's TclCmd/llength.html, `.htm` for the 8.6 tree): the
// synopsis ("llength list"), the description paragraph, and the worked
// examples are identical word-for-word in all five releases. llength
// has taken exactly one list argument and returned its top-level
// element count as a decimal string, with no option, no alternate
// form, and no documented behaviour change anywhere across the whole
// 8.4-9.1 span. The 9.0 and 9.1 pages grow a longer "See Also" list
// (lassign, ledit, lmap, lpop, lremove, lrepeat, lreverse, lseq
// alongside the 8.x set) purely because those commands did not exist
// yet in 8.4-8.6 — cross-reference churn, not a change to llength
// itself. Not disabled or overridden in any modelled dialect: absent
// from `IRULES_DISABLED_COMMANDS` in tcl-dialect/src/profile.rs, and
// no irules/iapps/tmsh/expect/eda/itcl spec file named `llength`
// exists — so the command-level `dialects: None` (universal) is
// correct as-is.
use crate::hooks::CodegenHookId;
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "llength list",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "llength",
        dialects: None,
        const_fold: Some(crate::const_fold::fold_llength),
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::PURE
            | Traits::CSE_CANDIDATE,
        arity: Arity::exact(1),
        return_type: Some(TclType::Int),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::List),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        hover: Some(HoverSnippet {
            summary: "Count the number of elements in a list",
            synopsis: &["llength list"],
            snippet: "Treats list as a Tcl list and returns the count of its top-level elements as a decimal string. An element grouped with braces or quotes — including a nested sublist — counts as a single element; llength does not descend into it. An empty list is not necessarily an empty string: a value containing only whitespace has list length 0 even though its string length is nonzero. list must be syntactically valid as a Tcl list — a malformed value, such as one with an unmatched open brace, raises an error rather than being treated as a single-element list.",
            source: "Tcl man page llength.n",
            examples: "llength {a b c d e}      ;# 5\nllength {a b {c d} e}    ;# 4 -- the braced {c d} sublist counts as one element\nllength {}               ;# 0\n\nset s { }\nllength $s                ;# 0 -- a lone space is an empty list, though [string length $s] is 1\n\n# common idiom: test whether a list is empty\nif {[llength $items] == 0} {\n    puts \"no items\"\n}",
            return_value: "A decimal string giving the number of top-level elements in list.",
        }),
        codegen_hook: Some(CodegenHookId::Llength),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
