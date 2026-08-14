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
// word-for-word in every one of the five releases. The DESCRIPTION's
// wording for first/last does change at 8.5 ("First or last may be end
// (or any abbreviation of it)" in 8.4, vs. "interpreted the same as
// index values for the command string index, supporting simple index
// arithmetic and indices relative to the end of the list" from 8.5 on)
// — and this is a REAL grammar extension, not just a documentation
// reword, even though 8.4's own EXAMPLES already exercise `end-2`/
// `end-1`. Verified directly against Tcl's own C source
// (generic/tclUtil.c, tcltk/tcl tags `core-8-4-20` and `core-8-5-19`):
// 8.4's `SetEndOffsetFromAny` only special-cases `bytes[3] == '-'` (a
// literal `end-N`), falling through to `bad index "...": must be
// integer or end?-integer?` for `end+N`, and its `TclGetIntForIndex`
// has no arithmetic path at all for a plain-integer base — only a
// bare, whole `Tcl_GetIntFromObj`-parseable integer, or `end`/`end-N`.
// 8.5 adds `bytes[3] == '+'` to `SetEndOffsetFromAny` and a new
// `integer[+-]integer` path to `TclGetIntForIndex`, with the error
// text changing to match (`end?[+-]integer?`) — exactly, and only
// since 8.5, the grammar `tcl-cmd-core::index` models. The hover
// snippet below states the 8.4-vs-8.5+ split explicitly, matching the
// precedent already established in `lindex.rs`/`lset.rs`/
// `lsearch_.rs` ("Tcl 8.4 supports only a bare integer or the fixed
// end-integer form"); an earlier pass here had stated the fuller
// arithmetic grammar (including `end+1`) as if true unconditionally,
// which is wrong for 8.4 by the C source above. `tcl-cmd-core::index`
// (the parser this command, `lindex`, and `string index` all share)
// accepts a base (`end` or a signed integer, any radix) plus at most
// one trailing `+`/`-` connector and integer operand — verified
// against the tclsh 8.6/9.0 oracle in that module's own tests — not
// arbitrary chained arithmetic; it has no per-dialect branch, so it
// models the 8.5+ grammar unconditionally and the 8.4 restriction is
// documentation-only here, same as its sibling index-consuming
// commands. (Separately, 8.4-8.6's "or any abbreviation of it" — a
// bare `e`/`en` also meaning `end`, from a truncated-length `strncmp`
// in `SetEndOffsetFromAny`/`GetEndOffsetFromObj` — is confirmed
// dropped in 9.0/9.1's rewritten parser, which requires the literal
// 3-byte "end". Not modelled by `tcl-cmd-core::index` at any version
// and not added to the hover text below: it is shared-parser
// behaviour affecting every index-consuming command identically, not
// something specific to lrange, and no sibling file documents it
// either.) The SEE ALSO list grows in the 9.0/9.1 pages (gaining
// lassign, ledit, lmap, lpop, lremove, lrepeat, lreverse, lseq
// alongside the 8.x set) purely because those sibling list commands
// did not exist before 8.5/8.6/9.0 — cross-reference churn, not a
// change to lrange itself. Not disabled or overridden in any modelled
// dialect: `lrange` carries the `IRULES` bit explicitly (its `dialects`
// group is `ALL_TCL.union(IRULES)`, resolving under the bare `IRULES`
// mask), and no irules/iapps/tmsh/expect/eda*/tk/itcl spec file names
// `lrange` — so the command-level group is correct as-is.
use crate::hooks::{CodegenHookId, InlineCodegenHookId};
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "lrange list first last",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lrange",
        dialects: Some(DialectSet::ALL_TCL.union(DialectSet::IRULES)),
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
            snippet: "Treats list as a Tcl list and returns a new list containing its elements from first through last, inclusive. first and last accept a plain (optionally signed, any Tcl integer radix) integer, end (the index of the last element), or end-integer (e.g. end-2); Tcl 8.5 and later also accept the fuller index-arithmetic syntax of string index — a base of either kind followed by a single +/- offset, such as end+1 or 2+1 — which Tcl 8.4 does not: end+1 is a \"bad index\" error there, and arithmetic on a plain-integer base isn't recognized at all. first below 0 is treated as 0; last at or beyond the list's length is treated as end; when first is greater than last, the result is an empty string. list must be a syntactically valid Tcl list, and first/last must be syntactically valid indices — either failing raises an error rather than returning an empty result. Because the result is rebuilt through canonical list quoting rather than copied as a raw substring, `lrange list N N` is not always textually identical to `lindex list N` for an element containing whitespace or braces — it matches `list [lindex list N]` instead.",
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
        ..CommandSpec::CLOSED_REFERENTIALLY_TRANSPARENT
    }
}
