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
// 8.5 through 9.1 at the value level — 9.1's `Tcl_ListObjReverse`
// actually `Tcl_DuplicateObj`s rather than returning the identical
// `Tcl_Obj` for a 0- or 1-element input, but the *value* is still
// `{}`/unchanged, which is all a script can ever observe), and from 9.0
// a value that is an "abstract list" providing its own reverse
// operation (e.g. the lazy sequence `lseq` returns — confirmed via
// `TclArithSeriesObjReverse` in `generic/tclArithSeries.c` at both
// `core-9-0-4` and `core-9-1-b0`; `lseq` itself is absent from
// `tclBasic.c`'s `builtInCmds[]` before 9.0) is reversed directly
// through that operation instead of first being shimmered into an
// ordinary list.
//
// Tcl 9.1 moves the command's body behind a new public
// `Tcl_ListObjReverse()` API (declared in `tcl.decls`, stub slot 693) —
// and this is more than the refactor it looks like, a fact the previous
// pass's "no observable script-level behaviour change from 9.0" framing
// missed: the new `generic/tclListTypes.c` (confirmed absent — HTTP 404
// — from both the `core-9-0-4` and `core-8-6-16` trees; first appears at
// `core-9-1-b0`) gives that function a second, independent abstract-list
// fast path alongside the 9.0 reverseProc-delegation one above. Past the
// reverseProc check, `Tcl_ListObjReverse` takes a lazy path — a
// `"reversedList"` view mapping index `i` to `len-1-i` on the original
// object, instead of eagerly copying — whenever the value has 100 or
// more elements (`LREVERSE_LENGTH_THRESHOLD`) *or* its type simply isn't
// the canonical list type, even for a short list: this second condition
// concretely fires for 9.1's own `lrepeat`/`lrange` lazy results
// (`lrepeatType`/`lrangeType`, same file, both with a NULL reverseProc),
// so e.g. `lreverse [lrepeat 5 x]` takes the lazy path in 9.1 regardless
// of length. The view's length/index/stringify/`in` hooks
// (`LreverseTypeLength`/`LreverseTypeIndex`/`TclAbstractListUpdateString`/
// `LreverseTypeInOper`) reproduce the eager result's value exactly, so
// this is a genuine, version-precise 9.1-only *performance* behaviour
// with no *value* difference from 9.0 — captured as its own sentence in
// the hover snippet below rather than folded into the 9.0 one.
//
// `generic/tclBasic.c`'s `builtInCmds[]` table (fetched at each of the
// `core-8-5-19` / `core-8-6-16` / `core-9-0-4` / `core-9-1-b0` release
// tags) shows `lreverse` has a NULL `compileProc` in every release that
// has the command at all — unlike its `lindex` / `lrange` / `llength` /
// `lassign` / `lreplace` siblings, no `TclCompileLreverseCmd` exists — so,
// unlike those siblings, this spec does not carry `Traits::BYTE_COMPILED`.
//
// Not disabled or overridden in any modelled dialect: no irules/iapps/
// tmsh/expect/eda*/tk/itcl spec file references `lreverse`, and there is
// no disable list for it to appear in (a sandbox-banned command would
// instead carry a bare `ALL_TCL` group lacking the `IRULES` bit —
// `lreverse` is version-gated, not banned). `f5-irules`'s
// `availability_mask` is the bare `IRULES` bit, and under the
// fully-explicit model a spec resolves under iRules only if its own
// `dialects` group carries that bit — iRules embeds a real Tcl 8.4.6, so
// this plain version-gated spec (whose `TCL85_PLUS` group lacks the
// `IRULES` bit) simply does not intersect the mask, so the existing
// `TCL85_PLUS` gate already and correctly excludes iRules by that same
// bare-bit intersection rule — matching the real product, whose embedded
// 8.4.6 predates TIP 272 — with no explicit exclusion needed. `f5-iapps` / `f5-tmsh` / xilinx / quartus / mentor
// (`TCL85 | vendor` masks) and cadence / synopsys / expect (`TCL86 |
// vendor`) and bpf (`TCL90 | vendor`) all correctly retain `lreverse`,
// since every one of those masks intersects `TCL85_PLUS` through its
// shared Tcl-version bit.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "lreverse list",
    ..FormSpec::DEFAULT
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
            snippet: "Treats list as a Tcl list and returns a new list with the same elements in reverse order; list must be syntactically valid as a Tcl list, and a malformed value (such as one with an unmatched open brace) raises an error rather than being treated as a single-element list. Only the element order changes — each element's own value and internal representation is left untouched, so a nested sublist such as {c d} still travels as a single element. An empty list argument is returned unchanged. From Tcl 9.0, when list is an abstract list such as the result of lseq, lreverse reverses it directly through the type's own reverse operation instead of first expanding it into an ordinary list. From Tcl 9.1, reversing a list of 100 or more elements -- or any value already backed by another lazy list view, such as a large lrepeat or lrange result -- similarly skips the up-front copy: the result is a lightweight view over the original value that indexes, measures, and prints exactly like a fully-reversed list.",
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
        ..CommandSpec::CLOSED_REFERENTIALLY_TRANSPARENT
    }
}
