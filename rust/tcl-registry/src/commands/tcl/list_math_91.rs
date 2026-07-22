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

//! Tcl 9.1 list-returning math commands: `divmod` / `frexp` / `modf` /
//! `remquo`.
//!
//! Each is a pure, deterministic numeric operation that returns a two-element
//! list.  Verified against C Tcl 9.1b0 `generic/tclBasic.c` (the built-in
//! command table: `DivModObjCmd` / `FracExpObjCmd` / `ModFObjCmd` /
//! `RemQuoObjCmd`, all `CMD_IS_SAFE`) and `doc/{divmod,frexp,modf,remquo}.n`.
//! Confirmed absent (404) from the Tcl 8.4, 8.5, 8.6, and 9.0 manual trees —
//! all four are new in 9.1 (TIP 745's multi-value C99 math functions land as
//! these standalone commands rather than as `tcl::mathfunc` entries, so none
//! of them can be called from inside `expr`).  None of the vendor dialects
//! (F5 iRules/iApps/tmsh, Expect, the EDA consoles, Tk, incr Tcl) declare an
//! override for any of these four names, so the plain `TCL91` command-level
//! gate is the whole story.

use crate::prelude::*;

/// Build a pure 9.1 math command returning a two-element list.  `args` is
/// the exact argument count after the command word (`divmod x y` ⇒ 2,
/// `frexp value` ⇒ 1).  `hover` and `forms` are supplied per command so each
/// gets its own accurate synopsis/snippet/examples instead of a shared,
/// generic description.
fn make(
    name: &'static str,
    args: u16,
    hover: HoverSnippet,
    forms: &'static [FormSpec],
) -> CommandSpec {
    CommandSpec {
        name,
        // Deterministic and side-effect-free ⇒ constant-foldable / CSE-able,
        // like `lseq`.  Not `BYTE_COMPILED`: that trait marks the curated
        // former minifier `_BUILTIN_SKIP` list (`set`/`if`/`string`/…), and
        // these four commands were never part of it — same as `lseq`.
        traits: Traits::PURE.union(Traits::CSE_CANDIDATE),
        dialects: Some(DialectSet::TCL91),
        arity: Arity::exact(args),
        return_type: Some(TclType::List),
        hover: Some(hover),
        forms,
        ..CommandSpec::DEFAULT
    }
}

/// `divmod x y` — integer quotient and remainder as a two-element list.
pub fn divmod() -> CommandSpec {
    const FORMS: &[FormSpec] = &[FormSpec {
        kind: FormKind::Default,
        synopsis: "divmod x y",
        dialects: None,
    }];
    make(
        "divmod",
        2,
        HoverSnippet {
            summary: "Divide an integer by another and report the quotient and remainder as a two-element list (Tcl 9.1).",
            synopsis: &["divmod x y"],
            snippet: "Returns a two-element list {quotient remainder} such that remainder + quotient*y == x. The quotient and remainder follow the same floor-division rules as Tcl's / and % operators, not C's truncating division: the remainder always takes the sign of y (never x), with magnitude smaller than abs(y). divmod is a standalone command, not a tcl::mathfunc entry, so it cannot be called from inside expr the way floor() or int() can.",
            source: "Tcl man page divmod.n (Tcl 9.1)",
            examples: "lassign [divmod 123 50] q r\nputs \"quotient=$q remainder=$r\"\n# quotient=2 remainder=23\nlassign [divmod -57 10] q r\nputs \"quotient=$q remainder=$r\"\n# quotient=-6 remainder=3",
            return_value: "A two-element list: the integer quotient of x divided by y, then the remainder.",
        },
        FORMS,
    )
}

/// `frexp value` — fraction and exponent parts of a float as a list.
pub fn frexp() -> CommandSpec {
    const FORMS: &[FormSpec] = &[FormSpec {
        kind: FormKind::Default,
        synopsis: "frexp value",
        dialects: None,
    }];
    make(
        "frexp",
        1,
        HoverSnippet {
            summary: "Split a floating-point number into a normalized fraction and a base-2 exponent (Tcl 9.1).",
            synopsis: &["frexp value"],
            snippet: "Returns a two-element list {fraction exponent} such that value == fraction * 2**exponent. fraction is signed, with magnitude between 0.5 and 1.0. When value is infinite, fraction is also infinite, with the same sign as value. Pairs with the ldexp() math function added in the same release: ldexp(fraction, exponent) reconstructs value, subject to floating-point rounding. Like the other Tcl 9.1 list-math commands, frexp is a standalone command, not a tcl::mathfunc entry usable inside expr.",
            source: "Tcl man page frexp.n (Tcl 9.1)",
            examples: "set value 123.456\nlassign [frexp $value] frac exp\nset recovered [expr {ldexp($frac, $exp)}]\nputs \"$value -> $frac*2**$exp -> $recovered\"\n# 123.456 -> 0.9645*2**7 -> 123.456",
            return_value: "A two-element list: the fractional part of value (magnitude between 0.5 and 1.0), then its base-2 exponent.",
        },
        FORMS,
    )
}

/// `modf value` — integer and fraction parts of a float as a list.
pub fn modf() -> CommandSpec {
    const FORMS: &[FormSpec] = &[FormSpec {
        kind: FormKind::Default,
        synopsis: "modf value",
        dialects: None,
    }];
    make(
        "modf",
        1,
        HoverSnippet {
            summary: "Split a floating-point number into its integer and fractional parts (Tcl 9.1).",
            synopsis: &["modf value"],
            snippet: "Returns a two-element list {whole fractional} where whole is the integer part of value, still represented as a float, and fractional is what remains; both carry the sign of value. Floating-point rounding means fractional can show trailing digits that look arbitrary — they cancel out again once whole and fractional are added back together. modf is a standalone command, not a tcl::mathfunc entry usable inside expr.",
            source: "Tcl man page modf.n (Tcl 9.1)",
            examples: "set value 123.456\nlassign [modf $value] whole part\nset sum [expr {$whole + $part}]\nputs \"$value -> $whole $part -> $sum\"\n# 123.456 -> 123.0 0.45600000000000307 -> 123.456",
            return_value: "A two-element list: the whole-number part of value as a float, then the fractional remainder; both share the sign of value.",
        },
        FORMS,
    )
}

/// `remquo x y` — floating-point remainder and quadrant determinant as a list.
pub fn remquo() -> CommandSpec {
    const FORMS: &[FormSpec] = &[FormSpec {
        kind: FormKind::Default,
        synopsis: "remquo x y",
        dialects: None,
    }];
    make(
        "remquo",
        2,
        HoverSnippet {
            summary: "Compute a floating-point remainder together with quadrant/octant information (Tcl 9.1).",
            synopsis: &["remquo x y"],
            snippet: "Returns a two-element list {remainder quo}. remainder is the IEEE remainder of x/y — the round-to-nearest (ties-to-even) definition also used by the remainder() math function added in the same release, not the floor-based rule that divmod and % use, and not fmod's truncating rule. quo is an integer carrying the sign of x/y together with enough low-order bits to identify which of eight octants the true quotient falls into, mirroring what C's remquo() provides for trigonometric range reduction. remquo is a standalone command, not a tcl::mathfunc entry.",
            source: "Tcl man page remquo.n (Tcl 9.1)",
            examples: "lassign [remquo 12.3 3.89] rem quo\nputs \"rem=$rem, quo=$quo\"\n# rem=0.6300000000000003, quo=3",
            return_value: "A two-element list: the floating-point remainder of x/y, then an integer carrying the sign and octant-determining bits of the quotient.",
        },
        FORMS,
    )
}
