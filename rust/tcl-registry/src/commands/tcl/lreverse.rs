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

//! `lreverse` — reverse a list.
//
// Manpage comparison across Tcl 8.4, 8.5, 8.6, 9.0, and 9.1
// (tcl-lang.org's TclCmd/lreverse.html, `.htm` for the 8.6 tree): Tcl 8.4
// has no lreverse page at all — the site's own "URL Not Found" page, not a
// stale mirror — and `generic/tclCmdIL.c` in the 8.4.20 source tree has no
// `Tcl_LreverseObjCmd`, confirming the command was genuinely added in 8.5
// (TIP 272) rather than merely undocumented at 8.4. From 8.5 through 9.1
// the synopsis ("lreverse list"), the description paragraph, and both
// worked examples are byte-for-byte identical; only the "See Also" list
// grows in the 9.0/9.1 pages (lassign, ledit, lmap, lpop, lremove,
// lrepeat, lseq, … alongside the 8.x set) purely because those sibling
// list commands did not exist yet in earlier releases.
//
// `generic/tclCmdIL.c` across the five fetched source trees shows two real
// behavioural facts the manpage prose never states, captured as
// version-qualified prose in the hover snippet below rather than a
// dialects gate (the command itself is unchanged since 8.5): an
// empty-list argument is returned unchanged rather than rebuilt (stable
// 8.5 through 9.1), and from 9.0 a value that is an "abstract list"
// providing its own reverse operation (e.g. the lazy sequence `lseq`
// returns) is reversed directly through that operation instead of first
// being shimmered into an ordinary list. Tcl 9.1 further refactors the
// command to delegate to a new public `Tcl_ListObjReverse()` API
// (`tcl.decls`), with no observable script-level behaviour change from
// 9.0. `generic/tclBasic.c`'s `builtInCmds[]` table (fetched at each of the
// `core-8-5-19` / `core-8-6-16` / `core-9-0-4` / `core-9-1-b0` release
// tags) shows `lreverse` has a NULL `compileProc` in every release that
// has the command at all — unlike its `lindex` / `lrange` / `llength` /
// `lassign` / `lreplace` siblings, no `TclCompileLreverseCmd` exists — so,
// unlike those siblings, this spec does not carry `Traits::BYTE_COMPILED`.
//
// Not disabled or overridden in any modelled dialect: no irules/iapps/
// tmsh/expect/eda*/tk/itcl spec file references `lreverse`, and it is
// absent from `IRULES_DISABLED_COMMANDS` in tcl-dialect/src/profile.rs.
// `f5-irules`'s `availability_mask` is the bare `IRULES` bit (the
// subtractive model — iRules embeds a real Tcl 8.4.6, so a plain
// version-gated spec need not itself carry the `IRULES` bit to resolve
// there), so the existing `TCL85_PLUS` gate already and correctly
// excludes iRules by the same bare-bit intersection rule — matching the
// real product, whose embedded 8.4.6 predates TIP 272 — with no explicit
// exclusion needed. `f5-iapps` / `f5-tmsh` / xilinx / quartus / mentor
// (`TCL85 | vendor` masks) and cadence / synopsys / expect (`TCL86 |
// vendor`) and bpf (`TCL90 | vendor`) all correctly retain `lreverse`,
// since every one of those masks intersects `TCL85_PLUS` through its
// shared Tcl-version bit.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lreverse list",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lreverse",
        const_fold: Some(crate::const_fold::fold_lreverse),
        traits: Traits::FRAMELESS_RUNTIME | Traits::PURE | Traits::CSE_CANDIDATE,
        // Added in Tcl 8.5 (TIP 272) — absent from the 8.4 manpage tree and
        // from `generic/tclCmdIL.c` in the 8.4.20 source; see the module
        // doc comment above for the full cross-version/cross-dialect audit.
        dialects: Some(DialectSet::TCL85_PLUS),
        arity: Arity::exact(1),
        return_type: Some(TclType::List),
        return_elements: Some(ReturnElements::SubListOf { container_arg: 0 }),
        hover: Some(HoverSnippet {
            summary: "Reverse the order of a list",
            synopsis: &["lreverse list"],
            snippet: "Treats list as a Tcl list and returns a new list with the same elements in reverse order; list must be syntactically valid as a Tcl list, and a malformed value (such as one with an unmatched open brace) raises an error rather than being treated as a single-element list. Only the element order changes — each element's own value and internal representation is left untouched, so a nested sublist such as {c d} still travels as a single element. An empty list argument is returned unchanged. From Tcl 9.0, when list is an abstract list such as the result of lseq, lreverse reverses it directly through the type's own reverse operation instead of first expanding it into an ordinary list.",
            source: "Tcl lreverse(n)",
            examples: "lreverse {a a b c}          ;# c b a a\nlreverse {a b {c d} e f}    ;# f e {c d} b a -- the nested {c d} sublist stays intact as one element\nlreverse {}                  ;# {} -- an empty list is returned unchanged",
            return_value: "A new list containing the same elements as list, in reverse order; an empty list argument is returned unchanged.",
        }),
        forms: FORMS,
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::List),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        ..CommandSpec::DEFAULT
    }
}
