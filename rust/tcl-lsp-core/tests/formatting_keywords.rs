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
use tcl_registry::prelude::DialectSet;

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
fn the_declared_role_reaches_the_whole_boolean_option_surface() {
    // A representative spread of the declared boolean options, across the
    // families that carry them. None of these was reachable by the old
    // closed-value-set inference: every one declares an *open* value, because
    // Tcl accepts unique prefixes there and a closed set would reject them.
    //
    // tclsh-proof (8.6.14) for the channel and clock cases:
    //   set c [open /dev/null r]
    //   chan configure $c -blocking yes ; chan configure $c -blocking  ;# 1
    //   chan configure $c -blocking off ; chan configure $c -blocking  ;# 0
    //   clock format 0 -gmt yes -format %Y                             ;# 1970
    for (src, want) in [
        // Channels: `fconfigure` and its `chan configure` spelling.
        ("chan configure $c -blocking no\n", "-blocking false"),
        // Interpreters.
        ("interp debug {} -frame yes\n", "-frame true"),
        // Namespaces.
        (
            "namespace ensemble create -prefixes no\n",
            "-prefixes false",
        ),
        // Tk geometry and fonts.
        ("pack configure .w -expand yes\n", "-expand true"),
        ("font configure f -underline yes\n", "-underline true"),
        // tcltest — the two options the pre-#1256 inference could see, still
        // rewritten now that the fact is declared rather than inferred.
        ("tcltest::configure -singleproc yes\n", "-singleproc true"),
        (
            "tcltest::configure -limitconstraints yes\n",
            "-limitconstraints true",
        ),
    ] {
        let out = default_fmt(src);
        assert!(out.contains(want), "{src} -> {out}");
    }
}

#[test]
fn an_irules_document_gets_the_same_rewrite() {
    // No iRules-specific option declares the boolean role today (TMM's own
    // commands take positional arguments, not `-option value` pairs), but an
    // iRule is Tcl: the core boolean options it can call are rewritten under
    // the vendor dialect exactly as under a core one, with no core version
    // range to widen over.
    let registry = tcl_registry::registry_for_dialect("f5-irules");
    let config = FormatterConfig {
        dialect: Some("f5-irules".to_owned()),
        target_range: tcl_registry::version_range::forward_range("f5-irules"),
        ..FormatterConfig::default()
    };
    let out = format_tcl("clock format $t -gmt yes\n", &config, registry);
    assert!(out.contains("-gmt true"), "{out}");
}

#[test]
fn an_abbreviated_boolean_word_resolves_through_the_boolean_table() {
    // The spelling is recognised by the built-in boolean table, which
    // reproduces `Tcl_GetBoolean` — including prefixes.
    //
    // tclsh-proof (8.6.14):
    //   chan configure $c -blocking tru ; chan configure $c -blocking  ;# 1
    //   string is boolean tru                                          ;# 1
    //   clock format 0 -gmt tru -format %Y                             ;# 1970
    for (src, want) in [
        ("fconfigure $c -blocking tru\n", "-blocking true"),
        ("fconfigure $c -blocking f\n", "-blocking false"),
        ("fconfigure $c -blocking ye\n", "-blocking true"),
        ("fconfigure $c -blocking of\n", "-blocking false"),
        // Both halves at once: an abbreviated option word and an abbreviated
        // boolean value.
        ("fconfigure $c -bl tru\n", "-blocking true"),
    ] {
        let out = default_fmt(src);
        assert!(out.contains(want), "{src} -> {out}");
    }
    // `o` is the one boolean prefix Tcl rejects as ambiguous, so it is not a
    // boolean word at all and keeps its bytes.
    //
    // tclsh-proof (8.6.14): `string is boolean o` -> 0.
    assert!(default_fmt("fconfigure $c -blocking o\n").contains("-blocking o"));
}

#[test]
fn the_boolean_rewrite_can_be_turned_off_on_its_own() {
    // TN: `preserve` stops the boolean rewrite while abbreviation expansion
    // carries on — the two settings are independent.
    let config = FormatterConfig {
        boolean_form: BooleanForm::Preserve,
        ..FormatterConfig::default()
    };
    let out = fmt("fconfigure $c -bl yes\n", &config);
    assert!(out.contains("-blocking yes"), "{out}");
    // And with both off, the bytes are untouched.
    let config = FormatterConfig {
        boolean_form: BooleanForm::Preserve,
        expand_abbreviations: false,
        ..FormatterConfig::default()
    };
    let out = fmt("fconfigure $c -bl yes\n", &config);
    assert!(out.contains("-bl yes"), "{out}");
}

#[test]
fn a_dynamic_boolean_option_value_abstains() {
    // FP guard: the value is a substitution, so there are no bytes to
    // canonicalise.
    let out = default_fmt("fconfigure $c -blocking $flag\n");
    assert!(out.contains("-blocking $flag"), "{out}");
}

// ---------------------------------------------------------------------------
// Issue #1257 — the formatter config carries the document's dialect and target
// version range, so a version-range-aware rewrite can apply it.
//
// tclsh ground truth: `string c` is unique in 8.5 (only `compare` starts with
// `c`) but ambiguous in 8.6+, where `string cat` arrived —
//   tclsh8.5: string c abc abc  -> 0
//   tclsh8.6: string c abc abc  -> ambiguous subcommand "c": must be ...
// so expanding `string c` to `string compare` in a file that may be run on
// 8.6 changes what a newer interpreter does with the source.
// ---------------------------------------------------------------------------

/// Format `src` against `dialect`'s registry, with the target range set to
/// that release and every later one.
fn fmt_over_range(src: &str, dialect: &str) -> String {
    let registry = tcl_registry::registry_for_dialect(dialect);
    let config = FormatterConfig {
        dialect: Some(dialect.to_owned()),
        target_range: tcl_registry::version_range::forward_range(dialect),
        ..FormatterConfig::default()
    };
    format_tcl(src, &config, registry)
}

/// Format `src` against `dialect`'s registry with no declared range — the
/// pre-#1257 behaviour, kept as the control.
fn fmt_no_range(src: &str, dialect: &str) -> String {
    let registry = tcl_registry::registry_for_dialect(dialect);
    format_tcl(src, &FormatterConfig::default(), registry)
}

#[test]
fn the_default_config_declares_no_range() {
    let cfg = FormatterConfig::default();
    assert_eq!(cfg.dialect, None);
    assert!(cfg.target_range.is_empty());
    // No dialect and no range: every declared keyword stays a candidate, the
    // pre-#1257 conservative direction. `string c` is ambiguous under that
    // rule (8.6's `cat` is in the table whatever the target), so it is left
    // alone — unchanged behaviour.
    assert!(fmt_no_range("string c $a $b\n", "tcl8.5").contains("string c $a $b"));
}

#[test]
fn a_prefix_ambiguous_later_in_the_range_is_not_expanded() {
    // `info e` is unique in 8.4 — `exists` is the only `e…` subcommand — but
    // `info errorstack` (9.0) collides at `e`. Declaring the range makes the
    // 8.4 table precise *and* keeps the expansion off, because the whole
    // range must agree.
    //
    // tclsh ground truth:
    //   tclsh8.4: info e x        -> 0          (resolves to `info exists`)
    //   tclsh9.0: info e x        -> ambiguous option "e": must be args, ...
    let out = fmt_over_range("info e x\n", "tcl8.4");
    assert!(out.contains("info e x"), "{out}");
    assert!(!out.contains("info exists"), "{out}");
}

#[test]
fn a_subcommand_a_later_release_removes_is_not_expanded() {
    // `trace vd` is `vdelete` in 8.4/8.5 and the subcommand is gone in 9.0,
    // so no spelling of it survives the whole range. Not expanded.
    //
    // tclsh ground truth:
    //   tclsh8.5: trace vdelete x w handler   -> ok (deprecated form)
    //   tclsh9.0: trace vdelete ...           -> unknown option "vdelete"
    let out = fmt_over_range("trace vd x w handler\n", "tcl8.5");
    assert!(out.contains("trace vd x w handler"), "{out}");
    assert!(!out.contains("vdelete"), "{out}");
}

#[test]
fn a_prefix_unique_across_the_whole_range_still_expands() {
    // TN: `le` is `length` in every release from 8.5 on, so the range check
    // does not cost the ordinary expansion.
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0"] {
        let out = fmt_over_range("string le $s\n", dialect);
        assert!(out.contains("string length $s"), "{dialect} -> {out}");
    }
}

#[test]
fn an_option_prefix_is_checked_over_the_range_too() {
    // The same rule applies to option words, not just subcommands.
    let out = fmt_over_range("lsearch -noc $x $p\n", "tcl8.6");
    assert!(out.contains("lsearch -nocase $x $p"), "{out}");
}

/// Format `src` against `dialect`'s registry with an explicit range — what a
/// document that must keep working on more than one release declares.
fn fmt_in_range(src: &str, dialect: &str, range: DialectSet) -> String {
    let registry = tcl_registry::registry_for_dialect(dialect);
    let config = FormatterConfig {
        dialect: Some(dialect.to_owned()),
        target_range: range,
        ..FormatterConfig::default()
    };
    format_tcl(src, &config, registry)
}

#[test]
fn a_boolean_option_a_release_in_the_range_lacks_is_not_normalised() {
    // `clock scan -validate` is Tcl 9.0+ (TIP 688). Under 9.0 the value is a
    // declared boolean and normalises — but a document that must also run on
    // 8.6 has no such option there, so nothing in the range vouches for what
    // an interpreter does with those bytes and they are left alone.
    //
    // tclsh-proof (8.6.14):
    //   clock scan 2026-01-01 -format %Y-%m-%d -validate yes
    //     -> bad option "-validate", must be -base, -format, -gmt, -locale
    //        or -timezone
    let src = "clock scan $s -format %Y -validate yes\n";
    let out = fmt_in_range(src, "tcl9.0", DialectSet::TCL86 | DialectSet::TCL90);
    assert!(out.contains("-validate yes"), "{out}");
    // TP control: with the range confined to the releases that have it, the
    // same value normalises.
    let out = fmt_in_range(
        src,
        "tcl9.0",
        tcl_registry::version_range::forward_range("tcl9.0"),
    );
    assert!(out.contains("-validate true"), "{out}");
}

#[test]
fn a_boolean_option_present_across_the_range_still_normalises() {
    // TN: `-gmt` is a boolean on `clock scan` in every release, so the range
    // check costs the ordinary rewrite nothing.
    //
    // tclsh-proof (8.6.14): `clock format 0 -gmt yes -format %Y` -> 1970.
    let out = fmt_in_range(
        "clock scan $s -gmt yes\n",
        "tcl9.0",
        DialectSet::TCL86 | DialectSet::TCL90,
    );
    assert!(out.contains("-gmt true"), "{out}");
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0"] {
        let out = fmt_over_range("fconfigure $c -blocking tru\n", dialect);
        assert!(out.contains("-blocking true"), "{dialect} -> {out}");
    }
}

#[test]
fn a_vendor_dialect_has_no_core_range_and_behaves_as_before() {
    // `f5-irules` names a runtime, not a core release: the forward range is
    // empty, so the handed registry stays the whole story.
    let cfg_range = tcl_registry::version_range::forward_range("f5-irules");
    assert!(cfg_range.is_empty());
    let out = fmt_over_range("string le $s\n", "f5-irules");
    assert!(out.contains("string length $s"), "{out}");
}
