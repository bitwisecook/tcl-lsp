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

//! The `scan` match predicate: each conversion, suppression, width and the decline paths behind W210.

use super::*;

// scan_predicate.rs — every conversion specifier + the decline paths
//
// `scan_provably_no_match(FMT, INPUT)` returns `true` iff the first conversion
// is statically guaranteed not to consume input (so the first output var stays
// unset).  Every expected verdict below is pinned to tclsh: a `true` means
// tclsh leaves the output unset; a `false` means tclsh either fills it or the
// predicate conservatively declines (noted inline).

#[test]
fn scan_decimal_int_match_and_no_match() {
    // tclsh (8.6, 9.0): `scan 42 %d n` → n=42 (match); `scan abc %d n` → unset.
    assert!(!scan_provably_no_match("%d", "42"));
    assert!(scan_provably_no_match("%d", "abc"));
    // Signed forms still match (leading +/-).
    // tclsh (8.6, 9.0): `scan -5 %d n` → n=-5; `scan +5 %i n` → n=5.
    assert!(!scan_provably_no_match("%d", "-5"));
    assert!(!scan_provably_no_match("%i", "+5"));
    assert!(!scan_provably_no_match("%u", "5"));
    // A lone sign with no following digit can't satisfy the conversion.
    assert!(scan_provably_no_match("%d", "-"));
}

#[test]
fn scan_octal_int_match_and_no_match() {
    // tclsh (8.6, 9.0): `scan 777 %o n` → n=511 (octal); `scan 8 %o n` → unset
    // (8 is not an octal digit); `scan 9 %o n` → unset.
    assert!(!scan_provably_no_match("%o", "777"));
    assert!(scan_provably_no_match("%o", "8"));
    assert!(scan_provably_no_match("%o", "9"));
}

#[test]
fn scan_binary_int_match_and_no_match() {
    // tclsh (8.6, 9.0): `scan 101 %b n` → n=5; `scan 2 %b n` → unset.
    assert!(!scan_provably_no_match("%b", "101"));
    assert!(scan_provably_no_match("%b", "2"));
}

#[test]
fn scan_hex_int_match_and_no_match() {
    // tclsh (8.6, 9.0): `scan 0x1f %x n` → n=31 (leading 0 is a hex digit);
    // `scan g %x n` → unset (g is not a hex digit).
    assert!(!scan_provably_no_match("%x", "0x1f"));
    assert!(!scan_provably_no_match("%X", "1f"));
    assert!(scan_provably_no_match("%x", "g"));
}

#[test]
fn scan_char_conversion_consumes_any_byte() {
    // `%c` reads ANY single byte (no leading-whitespace skip).
    // tclsh (8.6, 9.0): `scan abc %c n` → n=97; `scan "" %c n` → unset.
    assert!(!scan_provably_no_match("%c", "abc"));
    assert!(!scan_provably_no_match("%c", " ")); // a space is a byte for %c
    assert!(scan_provably_no_match("%c", "")); // empty input — nothing to read
}

#[test]
fn scan_string_conversion_needs_nonspace() {
    // `%s` skips leading whitespace then needs a non-space char.
    // tclsh (8.6, 9.0): `scan "   " %s n` → unset (all whitespace);
    // `scan abc %s n` → n=abc.
    assert!(!scan_provably_no_match("%s", "abc"));
    assert!(!scan_provably_no_match("%s", "  x")); // leading ws skipped, x remains
    assert!(scan_provably_no_match("%s", "   "));
    assert!(scan_provably_no_match("%s", ""));
}

#[test]
fn scan_float_conversion_variants() {
    // Every float leader: a digit, a leading-sign+digit, a `.digit`, and Inf.
    // tclsh (8.6, 9.0): `scan .5 %f n` → 0.5; `scan -.5 %f n` → -0.5;
    // `scan Inf %f n` → Inf; `scan x %f n` → unset; `scan . %f n` → unset.
    assert!(!scan_provably_no_match("%f", "3"));
    assert!(!scan_provably_no_match("%e", "+3"));
    assert!(!scan_provably_no_match("%g", ".5"));
    assert!(!scan_provably_no_match("%E", "-.5"));
    assert!(!scan_provably_no_match("%G", "Inf"));
    assert!(!scan_provably_no_match("%f", "infinity")); // case-insensitive prefix
    assert!(scan_provably_no_match("%f", "x"));
    assert!(scan_provably_no_match("%f", ".")); // a lone dot is not a float
    assert!(scan_provably_no_match("%f", "+")); // a lone sign is not a float
    assert!(scan_provably_no_match("%f", "")); // empty input
}

#[test]
fn scan_float_nan_is_conservative_not_provable() {
    // SOUNDNESS NOTE (not a bug): tclsh (8.6, 9.0) `scan nan %g n` leaves `n`
    // UNSET — this build's `scan "nan" %g n` does not fill the var.  The
    // predicate, however, treats a leading `nan` prefix as "might match" and
    // returns `false` (declines to prove no-match).  That is a deliberate
    // OVER-approximation: returning `false` means no W210 fires, so a real
    // unset is merely not flagged — never a false positive.  The predicate is
    // sound (it only ever asserts a *provable* no-match), just imprecise here.
    assert!(
        !scan_provably_no_match("%g", "nan"),
        "the nan-prefix float path is a sound over-approximation (returns false)"
    );
}

#[test]
fn scan_percent_n_always_succeeds() {
    // `%n` writes the consumed-character count and never consumes input, so it
    // can never be a provable no-match — even on empty input.
    // tclsh (8.6, 9.0): `scan "" %n n` → n=0 (always assigned).
    assert!(!scan_provably_no_match("%n", ""));
    assert!(!scan_provably_no_match("%n", "anything"));
}

#[test]
fn scan_literal_percent_directive() {
    // `%%` matches a literal `%` in the input; only THEN does the conversion
    // run.  tclsh (8.6, 9.0): `scan "%5" "%%%d" n` → n=5 (the %% eats the %,
    // %d reads 5); `scan "X5" "%%%d" n` → unset (X is not a literal %).
    assert!(!scan_provably_no_match("%%%d", "%5"));
    assert!(scan_provably_no_match("%%%d", "X5"));
    // A `%%` that runs off the end of the input mismatches.
    assert!(scan_provably_no_match("%%%d", ""));
}

#[test]
fn scan_assignment_suppression_star() {
    // `%*d` consumes-and-discards; the predicate models the FIRST conversion,
    // which is the suppressed `d`.  `scan "5" "%*d%d" n` matches the first %*d
    // against `5`, so the predicate returns `false` (might consume).  tclsh
    // (8.6, 9.0) leaves `n` UNSET there (the SECOND %d has no input left) — a
    // sound over-approximation: `false` means no false W210.
    assert!(
        !scan_provably_no_match("%*d%d", "5"),
        "suppressed first conversion consumes — predicate declines to prove no-match"
    );
    // But if the suppressed conversion itself can't match, it IS a no-match.
    assert!(scan_provably_no_match("%*d%d", "abc"));
}

#[test]
fn scan_width_and_size_modifiers_are_skipped() {
    // A width prefix (`%2d`) and a size modifier (`%ld`, `%hd`, `%lld`) are
    // skipped before the conversion char is read.
    // tclsh (8.6, 9.0): `scan "12 34" "%2d" n` → n=12; `scan 42 "%ld" n` → 42.
    assert!(!scan_provably_no_match("%2d", "12"));
    assert!(!scan_provably_no_match("%ld", "42"));
    assert!(!scan_provably_no_match("%lld", "42"));
    assert!(!scan_provably_no_match("%hd", "42"));
    // …and they don't rescue a genuine mismatch.
    assert!(scan_provably_no_match("%2d", "ab"));
    assert!(scan_provably_no_match("%ld", "zz"));
}

#[test]
fn scan_plain_literal_prefix_must_match() {
    // A non-`%`, non-whitespace literal in the format must match the input
    // verbatim before the conversion.  tclsh (8.6, 9.0): `scan abc "lit%d" n`
    // → unset (no `lit` prefix); a matching literal lets the conversion run.
    assert!(scan_provably_no_match("lit%d", "abc"));
    assert!(scan_provably_no_match("x%d", "y3")); // literal x vs y
    assert!(!scan_provably_no_match("x%d", "x3")); // literal matches, %d reads 3
}

#[test]
fn scan_format_whitespace_directive_skips_input_run() {
    // A whitespace char in the format skips a run of input whitespace before
    // the conversion.  tclsh (8.6, 9.0): `scan "  42" "%d" n` → 42 (leading ws
    // auto-skipped for %d); a leading-ws skip lets a following digit match.
    assert!(!scan_provably_no_match(" %d", "   42"));
    assert!(!scan_provably_no_match("%d", "  42"));
}

#[test]
fn scan_decline_paths_return_false() {
    // Every "outside the modelled subset" path returns `false` (never
    // proves a no-match):
    // 1. A char-set conversion `%[...]` is unmodelled.
    assert!(!scan_provably_no_match("%[abc]", "x"));
    // 2. A format that ends right after `%` is malformed → None → false.
    assert!(!scan_provably_no_match("%", "abc"));
    // 3. A format with no conversion at all has nothing to write → None → false.
    assert!(!scan_provably_no_match("plain", "plain"));
    assert!(!scan_provably_no_match("", ""));
    // 4. A `\`, `$`, or `[` in either arg bails immediately (raw-source slices,
    //    no substitution applied).
    assert!(!scan_provably_no_match("\\r%d", " 123"));
    assert!(!scan_provably_no_match("%d", "$x"));
    assert!(!scan_provably_no_match("[cmd]%d", "abc"));
    // 5. An unmodelled conversion char (`%p`) → None → false.
    assert!(!scan_provably_no_match("%p", "abc"));
    // 6. A `%*` that runs off the end of the format → None → false.
    assert!(!scan_provably_no_match("%*", "abc"));
    // 7. Width digits that consume the rest of the format → None → false.
    assert!(!scan_provably_no_match("%12", "abc"));
    // 8. A size modifier that ends the format → None → false.
    assert!(!scan_provably_no_match("%l", "abc"));
}

#[test]
fn scan_predicate_w210_end_to_end_fires_and_silent() {
    // End-to-end through the read-before-set surface, the Tcl-observable proof:
    // a provable no-match leaves the output var unset, so the later read is a
    // real read-before-set (W210); a match must stay silent.
    // tclsh (8.6, 9.0): `scan 777 %o n` → n=511 (no W210);
    //                    `scan 8 %o n`   → n unset (W210 on `puts $n`).
    let silent = "proc f {} { scan 777 %o n\n puts $n }";
    let fires_src = "proc f {} { scan 8 %o n\n puts $n }";
    assert!(
        !fires(silent, D, "W210"),
        "octal match must not fire W210; emitted {:?}",
        codes(silent, D)
    );
    assert!(
        fires(fires_src, D, "W210"),
        "octal no-match read-before-set must fire W210; emitted {:?}",
        codes(fires_src, D)
    );
}

#[test]
fn scan_predicate_w210_hex_and_binary() {
    // tclsh (8.6, 9.0): `scan 0x1f %x n` → 31 (silent); `scan g %x n` → unset
    // (fires); `scan 101 %b n` → 5 (silent); `scan 2 %b n` → unset (fires).
    assert!(!fires("proc f {} { scan 0x1f %x n\n puts $n }", D, "W210"));
    assert!(fires("proc f {} { scan g %x n\n puts $n }", D, "W210"));
    assert!(!fires("proc f {} { scan 101 %b n\n puts $n }", D, "W210"));
    assert!(fires("proc f {} { scan 2 %b n\n puts $n }", D, "W210"));
}
