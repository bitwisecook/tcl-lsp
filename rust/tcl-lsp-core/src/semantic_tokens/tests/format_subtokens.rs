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

//! Sub-tokens of `format`, `scan`, `clock` and `binary` specifications and backslash escapes.

use super::*;

#[test]
fn escape_subtoken_width_follows_the_selected_release_grammar() {
    let source = "puts \"\\x4142\"\n";
    let tcl84 = tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile();
    let tcl90 = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let escape_width = |profile| {
        full(source, profile, &reg())
            .data
            .chunks(5)
            .find(|token| token[3] == TokenKind::Escape as u32)
            .map(|token| token[2])
            .expect("the string contains an escape sub-token")
    };

    // Tcl 8.4 consumes every hex digit; Tcl 9.0 consumes exactly two.
    // The source slice and token geometry therefore depend on the
    // original document's selected escape configuration.
    assert_eq!(escape_width(tcl84), 6);
    assert_eq!(escape_width(tcl90), 4);
}

#[test]
fn escape_before_multibyte_char_does_not_panic() {
    // Slicing a fixed 2 bytes at the backslash of a `\<non-ASCII>` escape
    // inside a string lands inside the multi-byte char and panics the whole
    // semantic-tokens request. The escape must span the backslash plus the
    // full UTF-8 char.
    for src in [
        "puts \"\\é\"\n",
        "puts \"a\\你b\"\n",
        "puts \"\\€\"\n",
        "puts \"x\\é\\你\"\n",
    ] {
        let ks = kinds(src, tcl(), &reg());
        assert!(
            ks.contains(&(TokenKind::Escape as u32)),
            "expected an Escape sub-token for {src:?}, got {ks:?}",
        );
    }
}

#[test]
fn sprintf_format_spec_subtokens() {
    // `format {%d}` → `%` percent, `d` spec.
    let ks = kinds("format {%d} $n\n", tcl(), &reg());
    assert!(ks.contains(&(TokenKind::FormatPercent as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::FormatSpec as u32)), "{ks:?}");
}

#[test]
fn format_binary_specifier_is_release_gated() {
    let old = kinds(
        "format {%b} 1\n",
        tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile(),
        &reg(),
    );
    assert!(!old.contains(&(TokenKind::FormatSpec as u32)), "{old:?}");
    let modern = kinds(
        "format {%b} 1\n",
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &reg(),
    );
    assert!(
        modern.contains(&(TokenKind::FormatSpec as u32)),
        "{modern:?}"
    );
    let tcl9 = kinds(
        "format {%b} 1\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &reg(),
    );
    assert!(tcl9.contains(&(TokenKind::FormatSpec as u32)), "{tcl9:?}");
}

#[test]
fn sprintf_flags_and_width_subtokens() {
    // `%-5.2f` → percent, `-` flag, `5` width, `.` flag, `2` width,
    // `f` spec.
    let ks = kinds("format {%-5.2f} $x\n", tcl(), &reg());
    assert!(ks.contains(&(TokenKind::FormatFlag as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::FormatWidth as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::FormatSpec as u32)), "{ks:?}");
}

#[test]
fn scan_format_arg_subtokenised() {
    // `scan`'s format string is arg 2.
    let ks = kinds("scan $s {%n%[a-z]} a b\n", tcl(), &reg());
    assert!(ks.contains(&(TokenKind::FormatPercent as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::FormatSpec as u32)), "{ks:?}");
}

#[test]
fn invalid_scan_conversion_has_no_format_tokens() {
    let ks = kinds("scan 1 {%#d} a\n", tcl(), &reg());
    assert!(!ks.contains(&(TokenKind::FormatSpec as u32)), "{ks:?}");
}

#[test]
fn format_without_specifiers_stays_string() {
    let ks = kinds("format {plain} $x\n", tcl(), &reg());
    assert!(!ks.contains(&(TokenKind::FormatPercent as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::String as u32)), "{ks:?}");
}

#[test]
fn format_literal_percent_stays_string() {
    let ks = kinds(
        "format {100%%} 1\n",
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &reg(),
    );
    assert!(!ks.contains(&(TokenKind::FormatPercent as u32)), "{ks:?}");
    assert!(!ks.contains(&(TokenKind::FormatSpec as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::String as u32)), "{ks:?}");
}

#[test]
fn clock_format_subtokens() {
    // `clock format $t -format {%Y-%m-%d}` → %/letter pairs.
    let ks = kinds("clock format $t -format {%Y-%m-%d}\n", tcl(), &reg());
    assert!(ks.contains(&(TokenKind::ClockPercent as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::ClockSpec as u32)), "{ks:?}");
}

#[test]
fn clock_locale_modifier_uses_shared_grammar() {
    // Tcl accepts `%Ey`; the shared clock grammar owns both the modifier
    // and the conversion instead of leaving an editor-only table here.
    let ks = kinds("clock scan $s -format {%Ey}\n", tcl(), &reg());
    assert!(ks.contains(&(TokenKind::ClockPercent as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::ClockModifier as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::ClockSpec as u32)), "{ks:?}");
}

#[test]
fn unsupported_clock_g_specifier_stays_string() {
    let ks = kinds(
        "clock scan $s -format {%g}\n",
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &reg(),
    );
    assert!(!ks.contains(&(TokenKind::ClockPercent as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::String as u32)), "{ks:?}");
}

#[test]
fn clock_format_without_specifiers_stays_string() {
    let ks = kinds("clock format $t -format {plain}\n", tcl(), &reg());
    assert!(!ks.contains(&(TokenKind::ClockPercent as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::String as u32)), "{ks:?}");
}

#[test]
fn binary_format_spec_and_count_subtokens() {
    // `binary format a3 $d` (arg 2) → spec `a`, count `3`.
    let ks = kinds("binary format a3 $d\n", tcl(), &reg());
    assert!(ks.contains(&(TokenKind::BinarySpec as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::BinaryCount as u32)), "{ks:?}");
}

#[test]
fn binary_scan_signed_modifier_and_star() {
    // `binary scan $d su r` (arg 3) → spec `s`, modifier `u`.
    let ks = kinds(
        "binary scan $d su r\n",
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &reg(),
    );
    assert!(ks.contains(&(TokenKind::BinarySpec as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::BinaryFlag as u32)), "{ks:?}");
    // `c*` → spec `c`, `*` flag.
    let ks = kinds(
        "binary format c* $l\n",
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &reg(),
    );
    assert!(ks.contains(&(TokenKind::BinaryFlag as u32)), "{ks:?}");
}

#[test]
fn binary_signed_modifier_suppressed_in_tcl84() {
    // The `u` suffix is 8.5+ (TIP 275), so under tcl8.4 the `u` is not a
    // binaryFlag.
    let ks = kinds(
        "binary scan $d su r\n",
        tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
        &reg(),
    );
    assert!(ks.contains(&(TokenKind::BinarySpec as u32)), "{ks:?}");
    assert!(!ks.contains(&(TokenKind::BinaryFlag as u32)), "{ks:?}");
}

/// `insert_format_overrides` marks a format/clock/binary-role
/// argument by its registry-declared *position*, independent of
/// whether that word is literal — so a computed word (`$fmt`) reaches
/// the sub-tokeniser exactly as a literal one does, and each
/// sub-tokeniser already "falls back to the default classification"
/// (its own doc comment, `ArgOverride`) when it finds no specifier in
/// the token's own bytes. `$fmt`'s own text is `"$fmt"`, never a `%`
/// specifier, so it renders as a plain `variable` token — a computed
/// pattern is explained at its use through hover and inlay hints
/// (`hover.rs`/`inlay_hints.rs`), never painted here at a token
/// range it does not have.
#[test]
fn a_computed_format_word_falls_back_to_its_plain_classification() {
    let names = legend_token_types();
    let kind_of = |src: &str, needle: &str| {
        decode_words(src, &reg())
            .into_iter()
            .find(|(_, _, _, _, word)| word == needle)
            .map(|(_, _, _, kind, _)| names[kind as usize])
    };
    assert_eq!(
        kind_of("set fmt \"%-20s %d\"\nformat $fmt a 1\n", "$fmt"),
        Some("variable"),
        "a computed format argument is an ordinary variable token, not formatSpec"
    );
    assert_eq!(
        kind_of(
            "set fmt \"%Y-%m-%d\"\nclock format 0 -format $fmt\n",
            "$fmt"
        ),
        Some("variable")
    );
    assert_eq!(
        kind_of("set fmt \"a3 i\"\nbinary format $fmt foo 1\n", "$fmt"),
        Some("variable")
    );
}
