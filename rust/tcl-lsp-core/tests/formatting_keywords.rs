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

//! Formatter keyword normalisation: abbreviation expansion (#1232) and the
//! canonical boolean form (#1233).
//!
//! tclsh ground truth (8.6.16): `string le abc` → `3`, so `string le` and
//! `string length` are the same call; `lsearch -noc {a b} a` → `0`;
//! `string l abc` → `unknown or ambiguous subcommand "l"`, so an ambiguous
//! word must never be rewritten into one of its candidates.

use tcl_lsp_core::formatting::config::{BooleanForm, FormatterConfig};
use tcl_lsp_core::formatting::engine::format_tcl;

fn fmt(src: &str, config: &FormatterConfig) -> String {
    let registry = tcl_registry::registry_for_dialect("tcl8.6");
    format_tcl(src, config, registry)
}

fn default_fmt(src: &str) -> String {
    fmt(src, &FormatterConfig::default())
}

#[test]
fn subcommand_abbreviations_expand_by_default() {
    assert!(default_fmt("string le $s\n").contains("string length $s"));
    assert!(default_fmt("string eq $a $b\n").contains("string equal $a $b"));
    assert!(default_fmt("info ex x\n").contains("info exists x"));
}

#[test]
fn option_abbreviations_expand_by_default() {
    let out = default_fmt("lsearch -noc -al $x $p\n");
    assert!(out.contains("lsearch -nocase -all $x $p"), "{out}");
}

#[test]
fn expansion_is_idempotent() {
    let once = default_fmt("string le $s\nlsearch -noc $x $p\n");
    let twice = default_fmt(&once);
    assert_eq!(once, twice);
    // A canonical spelling resolves to itself and is untouched.
    assert_eq!(
        default_fmt("string length $s\n"),
        default_fmt(&default_fmt("string length $s\n"))
    );
}

#[test]
fn ambiguous_and_unknown_words_are_left_byte_for_byte() {
    // `l` prefixes both `last` and `length`; the formatter never guesses.
    assert!(default_fmt("string l $s\n").contains("string l $s"));
    // An ambiguous option prefix likewise.
    assert!(default_fmt("lsearch -a $x $p\n").contains("lsearch -a $x $p"));
    // A word that prefixes nothing stays as written.
    assert!(default_fmt("string zzz $s\n").contains("string zzz $s"));
}

#[test]
fn dynamic_and_expanded_words_abstain() {
    for src in [
        "string $sub $s\n",
        "string [pick] $s\n",
        "string {*}$words $s\n",
    ] {
        let out = default_fmt(src);
        assert_eq!(out.trim_end(), src.trim_end(), "{src}");
    }
}

#[test]
fn command_names_are_never_expanded() {
    // Tcl does not prefix-match command names: `str` is a genuine unknown
    // command, not `string`.
    let out = default_fmt("str length $s\n");
    assert!(out.contains("str length"), "{out}");
    assert!(!out.contains("string length"), "{out}");
}

#[test]
fn a_braced_or_quoted_data_word_that_looks_like_a_keyword_is_untouched() {
    // The abbreviation is in a data position, not a keyword one.
    for src in ["set x {le}\n", "puts \"le\"\n", "lappend xs le\n"] {
        let out = default_fmt(src);
        assert!(out.contains("le"), "{src} -> {out}");
        assert!(!out.contains("length"), "{src} -> {out}");
    }
}

#[test]
fn expansion_can_be_turned_off() {
    let config = FormatterConfig {
        expand_abbreviations: false,
        ..FormatterConfig::default()
    };
    let out = fmt("string le $s\n", &config);
    assert!(out.contains("string le $s"), "{out}");
}

#[test]
fn boolean_form_defaults_to_true_false() {
    // `string is boolean -strict` takes no value, so the boolean rewrite is
    // exercised through an option whose declared value set *is* the boolean
    // vocabulary. The default form is true/false.
    assert_eq!(
        FormatterConfig::default().boolean_form,
        BooleanForm::TrueFalse
    );
    assert_eq!(BooleanForm::TrueFalse.pair(), Some(("true", "false")));
    assert_eq!(BooleanForm::YesNo.pair(), Some(("yes", "no")));
    assert_eq!(BooleanForm::OnOff.pair(), Some(("on", "off")));
    assert_eq!(BooleanForm::ZeroOne.pair(), Some(("1", "0")));
    assert_eq!(BooleanForm::Preserve.pair(), None);
    assert_eq!(BooleanForm::parse("yes/no"), BooleanForm::YesNo);
    assert_eq!(BooleanForm::parse("on/off"), BooleanForm::OnOff);
    assert_eq!(BooleanForm::parse("0/1"), BooleanForm::ZeroOne);
    assert_eq!(BooleanForm::parse("preserve"), BooleanForm::Preserve);
    assert_eq!(BooleanForm::parse("true/false"), BooleanForm::TrueFalse);
    // An unrecognised value falls back to the default.
    assert_eq!(BooleanForm::parse("nonsense"), BooleanForm::TrueFalse);
}

#[test]
fn a_value_definition_site_keeps_its_bytes() {
    // `set flag yes` is a definition, not a boolean consumption site: `$flag`
    // may later meet `eq "yes"`, a `switch` arm, or a log line, and `true`
    // and `yes` are different strings. Never rewritten, under any form.
    for form in [
        BooleanForm::TrueFalse,
        BooleanForm::YesNo,
        BooleanForm::OnOff,
        BooleanForm::ZeroOne,
    ] {
        let config = FormatterConfig {
            boolean_form: form,
            ..FormatterConfig::default()
        };
        let out = fmt("set flag yes\n", &config);
        assert!(out.contains("set flag yes"), "{form:?} -> {out}");
    }
}

#[test]
fn preserve_leaves_every_boolean_word_alone() {
    let config = FormatterConfig {
        boolean_form: BooleanForm::Preserve,
        ..FormatterConfig::default()
    };
    for src in ["set flag yes\n", "set flag 1\n", "set flag on\n"] {
        let out = fmt(src, &config);
        assert_eq!(out.trim_end(), src.trim_end(), "{src}");
    }
}

#[test]
fn formatting_never_changes_a_range_it_was_not_asked_about() {
    // Range formatting only rewrites words wholly inside the range: the
    // engine formats the extracted slice, so a keyword outside it is never
    // seen. Proven by formatting a one-line slice of a two-line document.
    use tcl_lsp_core::definition::LspRange;
    use tcl_lsp_core::formatting::range_formatting;
    let registry = tcl_registry::registry_for_dialect("tcl8.6");
    let src = "string le $a\nstring le $b\n";
    let edits = range_formatting(
        src,
        LspRange {
            start_line: 0,
            start_character: 0,
            end_line: 0,
            end_character: 100,
        },
        &FormatterConfig::default(),
        registry,
    );
    let text = edits.iter().map(|e| e.new_text.clone()).collect::<String>();
    assert!(text.contains("string length $a"), "{text}");
    assert!(
        !text.contains("$b"),
        "the second line is outside the range: {text}"
    );
}

// ---------------------------------------------------------------------------
// Issue #1256 — the boolean consumption site is a declared registry fact
// (`ArgRole::Boolean`), not something inferred from an option's value set.
//
// tclsh-proof (8.6.16 / 9.0.4): the value's bytes are consumed and discarded,
// so every spelling of the same truth value is interchangeable —
//   set c [open /dev/null]
//   chan configure $c -blocking yes ; chan configure $c -blocking   ;# -> 1
//   chan configure $c -blocking off ; chan configure $c -blocking   ;# -> 0
// ---------------------------------------------------------------------------

#[test]
fn a_declared_boolean_option_value_normalises() {
    // Before the fact existed, the inference reached exactly two options in
    // the whole registry (`tcltest::configure -limitconstraints`/
    // `-singleproc`); every core boolean option was invisible to it.
    for (src, want) in [
        ("fconfigure $c -blocking yes\n", "-blocking true"),
        ("fconfigure $c -blocking 0\n", "-blocking false"),
        ("socket -server $cb -reuseaddr on 0\n", "-reuseaddr true"),
        ("clock format $t -gmt 1\n", "-gmt true"),
    ] {
        let out = default_fmt(src);
        assert!(out.contains(want), "{src} -> {out}");
    }
}

#[test]
fn the_configured_form_reaches_a_declared_boolean_option() {
    for (form, want) in [
        (BooleanForm::YesNo, "-blocking yes"),
        (BooleanForm::OnOff, "-blocking on"),
        (BooleanForm::ZeroOne, "-blocking 1"),
    ] {
        let config = FormatterConfig {
            boolean_form: form,
            ..FormatterConfig::default()
        };
        let out = fmt("fconfigure $c -blocking true\n", &config);
        assert!(out.contains(want), "{form:?} -> {out}");
    }
}

#[test]
fn a_non_boolean_option_value_is_never_rewritten() {
    // TN: `-translation` takes an enum, `-buffersize` a count. Neither is a
    // boolean consumption site, so `binary`/`1` keep their bytes.
    let out = default_fmt("fconfigure $c -translation binary -buffersize 1\n");
    assert!(out.contains("-translation binary"), "{out}");
    assert!(out.contains("-buffersize 1"), "{out}");
}

#[test]
fn boolean_option_rewriting_is_idempotent() {
    let once = default_fmt("fconfigure $c -blocking yes\n");
    assert_eq!(once, default_fmt(&once));
}

#[test]
fn a_dynamic_boolean_option_value_abstains() {
    // FP guard: the value is a substitution, so there are no bytes to
    // canonicalise.
    let out = default_fmt("fconfigure $c -blocking $flag\n");
    assert!(out.contains("-blocking $flag"), "{out}");
}
