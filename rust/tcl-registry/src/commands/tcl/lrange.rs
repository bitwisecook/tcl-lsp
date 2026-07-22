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

//! `lrange` — return a range of list elements.
//
// Manpage comparison across Tcl 8.4, 8.5, 8.6, 9.0, and 9.1
// (tcl-lang.org's TclCmd/lrange.html, `.htm` for the 8.6 tree): the
// synopsis ("lrange list first last"), the core description paragraph,
// the three out-of-range clamp rules (first < 0 -> treated as 0; last >=
// length -> treated as end; first > last -> empty string result), the
// lrange-vs-lindex note, and all four worked examples are identical
// word-for-word in every one of the five releases. The only prose
// difference is how the DESCRIPTION explains first/last: Tcl 8.4 says
// only "First or last may be end (or any abbreviation of it) to refer
// to the last element of the list", while 8.5 through 9.1 reword this
// to "interpreted the same as index values for the command string
// index, supporting simple index arithmetic and indices relative to
// the end of the list". Tcl 8.4's own EXAMPLES section already
// exercises `end-2` and `end-1` and documents the same output as every
// later version, so this reads as a documentation-precision rewrite
// rather than a functional change; the hover snippet below states the
// current index grammar as plain fact rather than claiming 8.4 lacked
// it. `tcl-cmd-core::index` (the parser this command, `lindex`, and
// `string index` all share) accepts a base (`end` or a signed integer,
// any radix) plus at most one trailing `+`/`-` connector and integer
// operand — verified against the tclsh 8.6/9.0 oracle in that module's
// own tests — not arbitrary chained arithmetic. The SEE ALSO list
// grows in the 9.0/9.1 pages (gaining lassign, ledit, lmap, lpop,
// lremove, lrepeat, lreverse, lseq alongside the 8.x set) purely
// because those sibling list commands did not exist before
// 8.5/8.6/9.0 — cross-reference churn, not a change to lrange itself.
// Not disabled or overridden in any modelled dialect: absent from
// `IRULES_DISABLED_COMMANDS` in tcl-dialect/src/profile.rs, and no
// irules/iapps/tmsh/expect/eda*/tk/itcl spec file names `lrange` — so
// the command-level `dialects: None` (universal) is correct as-is.
use crate::hooks::{CodegenHookId, InlineCodegenHookId};
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lrange list first last",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lrange",
        dialects: None,
        const_fold: Some(crate::const_fold::fold_lrange),
        // `lrange list first last` has fixed arity 3, so *every* call is a
        // `HEAD NAME BRACED BRACED` shape once first/last happen to be
        // brace-quoted (e.g. `lrange {a b c} 0 {2}`) — it needs
        // `NOT_PROC_FACTORY` to keep the proc-factory signature scanner
        // from mistaking such a call for a `proc`-defining wrapper. Same
        // reasoning as `lindex` (commands/tcl/lindex.rs) and `ledit`
        // (commands/tcl/ledit.rs), whose bare `listVar first last` form is
        // the identical collision shape.
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::PURE
            | Traits::CSE_CANDIDATE,
        arity: Arity::exact(3),
        return_type: Some(TclType::List),
        return_elements: Some(ReturnElements::SubListOf { container_arg: 0 }),
        hover: Some(HoverSnippet {
            summary: "Return one or more adjacent elements from a list.",
            synopsis: &["lrange list first last"],
            snippet: "Treats list as a Tcl list and returns a new list containing its elements from first through last, inclusive. first and last accept a plain (optionally signed, any Tcl integer radix) integer, end (the index of the last element), or a base of either kind followed by a single +/- offset such as end-2 or end+1 — the same index syntax as string index. first below 0 is treated as 0; last at or beyond the list's length is treated as end; when first is greater than last, the result is an empty string. list must be a syntactically valid Tcl list, and first/last must be syntactically valid indices — either failing raises an error rather than returning an empty result. Because the result is rebuilt through canonical list quoting rather than copied as a raw substring, `lrange list N N` is not always textually identical to `lindex list N` for an element containing whitespace or braces — it matches `list [lindex list N]` instead.",
            source: "Tcl lrange(n)",
            examples: "set colors {red green blue yellow orange}\nlrange $colors 0 1        ;# red green\nlrange $colors end-2 end  ;# blue yellow orange\nlrange $colors 1 end-1    ;# green blue yellow\n\n# a single-element range is still a *list*, unlike lindex's bare element\nset var {some {elements to} select}\nlindex $var 1              ;# elements to\nlrange $var 1 1            ;# {elements to}",
            return_value: "A new list built from the elements in the (clamped) inclusive range; an empty string when the range selects no elements.",
        }),
        codegen_hook: Some(CodegenHookId::Lrange),
        inline_codegen_hook: Some(InlineCodegenHookId::Lrange),
        forms: FORMS,
        arg_types: &[
            (
                0,
                ArgTypeHint {
                    expected: Some(TclType::List),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
            (
                2,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
        ],
        ..CommandSpec::DEFAULT
    }
}
