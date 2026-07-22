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

//! `lreplace` — replace elements in a list with new elements.
//
// Manpage comparison across Tcl 8.4, 8.5, 8.6, 9.0, and 9.1
// (tcl-lang.org's TclCmd/lreplace.html, `.htm` for the 8.6 tree): the
// synopsis ("lreplace list first last ?element element ...?"), the
// "replace zero or more elements" description, and the "last < first
// -> pure insertion before first, nothing deleted" rule read the same
// in substance in every one of the five releases, and the first three
// EXAMPLES (replacing one element, replacing two elements with three,
// deleting the last element via `lreplace $var end end`) are
// word-for-word identical across all five.
//
// The one real behavioural difference (TIP 505, "Make [lreplace]
// Accept All Out-of-Range Index Values", landed in Tcl 8.6.9): Tcl 8.4
// and 8.5 require that, for a non-empty list, "the element indicated
// by first must exist" — a first index past the end of the list is a
// hard error there. From Tcl 8.6.9 onward (the tcl8.6 manpage tree
// here reflects the latest 8.6.x, i.e. post-8.6.9, matching the
// locally-pinned 8.6.16 reference in AGENTS.md) through 9.0 and 9.1,
// either first or last past the end of the list instead clamps to one
// past the last element, turning the call into a pure append; only
// the 8.6+/9.0/9.1 pages carry the confirming example
// (`lreplace $var 12345 end+2 f g h i`). Negative-index clamping
// (before the first element, enabling a pure prepend) is unchanged
// across all five releases — TIP 505 only loosened the past-the-end
// case. The 9.0/9.1 pages additionally gain a `lremove`-via-`lreplace`
// idiom example (Tcl 9.0's own `lremove` builtin postdates this
// idiom) and a much longer SEE ALSO list (`lassign`, `ledit`, `lmap`,
// `lpop`, `lremove`, `lrepeat`, `lreverse`, `lseq` alongside the 8.x
// set) — cross-reference growth from those sibling list commands not
// existing yet, not a change to `lreplace` itself. 9.0 and 9.1 are
// otherwise byte-similar (only the version banner differs), matching
// `lremove`'s own documented 9.0/9.1 parity.
//
// Not disabled or overridden in any modelled dialect: absent from
// `IRULES_DISABLED_COMMANDS` in tcl-dialect/src/profile.rs (the only
// disabled-commands table in that file), and no irules/iapps/tmsh/
// expect/eda*/tk/itcl spec file registers its own `lreplace` — so the
// command-level `dialects: None` (universal) is correct.
//
// `tcl-vm/src/cmd_list.rs::cmd_lreplace` requires exactly `list first
// last` plus any number of trailing elements — matching
// `Arity::at_least(3)` and the compiler's own inline-codegen guard
// (`args.len() >= 3` in `tcl-compiler/src/codegen/cmd_subst.rs`) — and
// reads/writes no variable: a flat function of its arguments with no
// eval fallback, so `FRAMELESS_RUNTIME`/`PURE` (already set) are
// correct; it already appears in
// `frameless_runtime_covers_the_audited_allow_list` in
// `tcl-registry/src/registry.rs`. Its minimum call, `lreplace {list}
// first {last}` (list a literal with no `$`/`[`, last braced), is the
// identical `HEAD NAME BRACED BRACED` four-token shape the proc-factory
// signature scanner
// (`tcl_compiler::signature_scan::handlers::maybe_record_factory_candidate`)
// misreads as a factory wrapper — same reasoning as `lrange`
// (commands/tcl/lrange.rs) and `lindex` (commands/tcl/lindex.rs) — so
// it needs `NOT_PROC_FACTORY` too. Deterministic given its arguments
// with no side effects, so a repeated call is worth flagging as
// redundant like its `lindex`/`linsert`/`lrange` siblings
// (`CSE_CANDIDATE`).
//
// No `ReturnElements` variant fits the result shape precisely: unlike
// `lrange`/`lremove` (always a sub-list of the source), `lreplace`
// splices the caller's own `element` arguments into the result, so it
// is not purely a sub-list of argument 0 — left unset rather than
// stamping the nearest-but-wrong `SubListOf`.
use crate::hooks::InlineCodegenHookId;
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lreplace list first last ?element element ...?",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lreplace",
        dialects: None,
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::PURE
            | Traits::CSE_CANDIDATE,
        arity: Arity::at_least(3),
        return_type: Some(TclType::List),
        inferred_storage_type: Some(StorageType::List),
        hover: Some(HoverSnippet {
            summary: "Replace a range of list elements with new elements.",
            synopsis: &["lreplace list first last ?element element ...?"],
            snippet: "Returns a new list formed from list by removing the inclusive first..last range of elements and splicing the element arguments (if any) into their place; list itself is left unmodified. first and last take the same index forms as string index — 0 is the first element and end the last. When last is less than first, no elements are removed and the given elements are simply inserted before first. An out-of-range index clamps rather than erroring: a negative first or last clamps to before the first element, so lreplace can prepend, and — from Tcl 8.6.9, and in 9.0/9.1 — an index past the last element clamps to one past the end, so lreplace can append regardless of how far out of range the index is. Tcl 8.4 and 8.5 (and Tcl 8.6 before 8.6.9) lack that append clamp: there, first must index an element that actually exists in a non-empty list, and a first past the end raises an error instead of appending. With no element arguments at all, the elements in the first..last range are simply deleted.",
            source: "Tcl lreplace(n)",
            examples: "lreplace {a b c d e} 1 1 foo                   ;# a foo c d e\nlreplace {a b c d e} 1 2 three more elements   ;# a three more elements d e\n\nset var {a b c d e}\nset var [lreplace $var end end]                ;# a b c d -- delete the last element\n\nset var {a b c d e}\nset var [lreplace $var 12345 end+2 f g h i]    ;# a b c d e f g h i -- out-of-range indices append (Tcl 8.6.9+, 9.0, 9.1)",
            return_value: "A new list: list with the first..last range removed and the element arguments, if any, spliced into their place.",
        }),
        inline_codegen_hook: Some(InlineCodegenHookId::Lreplace),
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
