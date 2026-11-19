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

//! Specifier tables for `clock format`, `format` and `binary format`.

use super::*;

fn format_profile() -> &'static tcl_dialect::DialectProfile {
    tcl_dialect::DialectProfile::find("tcl8.6").unwrap()
}

#[test]
fn binary_field_bytes_saturates_instead_of_overflowing() {
    // `d600000000` ⇒ 8 × 600000000 overflows u32; report unknown size,
    // never panic (debug) or wrap (release) (issue 182).
    assert_eq!(
        super::super::binary_field_bytes('d', 600_000_000, false),
        None
    );
    assert_eq!(super::super::binary_field_bytes('i', u32::MAX, false), None);
    // Ordinary sizes still compute.
    assert_eq!(super::super::binary_field_bytes('i', 4, false), Some(16));
}

// clock format hover

#[test]
fn scan_clock_specifiers_finds_each_specifier() {
    let s = scan_clock_specifiers("%Y-%m-%d %H:%M:%S");
    assert_eq!(s, vec!["%Y", "%m", "%d", "%H", "%M", "%S"]);
}

#[test]
fn scan_clock_specifiers_handles_locale_prefix() {
    let s = scan_clock_specifiers("%EY-%Om");
    assert_eq!(s, vec!["%EY", "%Om"]);
}

#[test]
fn scan_clock_specifiers_handles_literal_percent() {
    let s = scan_clock_specifiers("100%% complete");
    assert_eq!(s, vec!["%%"]);
}

#[test]
fn clock_format_hover_renders_specifier_table() {
    let text = clock_format_hover_text("%Y-%m-%d");
    assert!(text.contains("**Clock format string**"), "{text}");
    assert!(text.contains("| `%Y` | 4-digit year |"), "{text}");
    assert!(text.contains("| `%m` | Month (01–12) |"), "{text}");
    assert!(text.contains("| `%d` | Day of month (01–31) |"), "{text}");
}

#[test]
fn clock_format_hover_marks_locale_modified_specifiers() {
    let text = clock_format_hover_text("%EY");
    assert!(text.contains("(locale-modified)"), "{text}");
}

#[test]
fn clock_format_hover_handles_empty_format() {
    let text = clock_format_hover_text("no specifiers here");
    assert!(text.contains("No specifiers found"), "{text}");
}

#[test]
fn hover_fires_for_clock_format_specifier() {
    let src = "clock format $time -format {%Y-%m-%d}\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    // Cursor inside the format literal.
    let h = hover(src, 0, 30, &analysis, None).expect("hover");
    assert!(
        h.value.contains("Clock format string"),
        "expected clock hover, got: {value}",
        value = h.value,
    );
}

// sprintf format hover

#[test]
fn format_hover_uses_the_selected_complete_specifiers() {
    for (dialect, int64, long_long) in [
        ("tcl8.4", false, false),
        ("tcl8.6", false, true),
        ("tcl9.1", true, true),
        ("jimtcl", false, true),
    ] {
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
        assert_eq!(scan_sprintf_specifiers("%*d %ld", profile), ["%*d", "%ld"]);
        assert_eq!(!scan_sprintf_specifiers("%I64d", profile).is_empty(), int64);
        assert_eq!(
            !scan_sprintf_specifiers("%lld", profile).is_empty(),
            long_long
        );
        let hover = binary_format_hover_text(&binary_ctx("cs"), profile);
        assert!(hover.contains("2 fields, 3 bytes"), "{dialect}: {hover}");
    }
}

#[test]
fn scan_sprintf_specifiers_finds_basic_types() {
    let s = scan_sprintf_specifiers("%d - %s : %x", format_profile());
    assert_eq!(s, vec!["%d", "%s", "%x"]);
}

#[test]
fn scan_sprintf_specifiers_captures_width_and_precision() {
    let s = scan_sprintf_specifiers("%05d %-10s %.3f", format_profile());
    assert_eq!(s, vec!["%05d", "%-10s", "%.3f"]);
}

#[test]
fn scan_sprintf_specifiers_captures_positional() {
    let s = scan_sprintf_specifiers("%1$s %2$d", format_profile());
    assert_eq!(s, vec!["%1$s", "%2$d"]);
}

#[test]
fn scan_sprintf_specifiers_handles_literal_percent() {
    let s = scan_sprintf_specifiers("%% done", format_profile());
    assert_eq!(s, vec!["%%"]);
}

#[test]
fn sprintf_format_hover_renders_specifier_table() {
    let text = sprintf_format_hover_text("%d - %s", format_profile());
    assert!(text.contains("**Format string** (sprintf-style)"), "{text}");
    assert!(text.contains("| `%d` | Signed decimal integer |"), "{text}");
    assert!(text.contains("| `%s` | String |"), "{text}");
}

#[test]
fn sprintf_format_hover_handles_empty_format() {
    let text = sprintf_format_hover_text("no specifiers here", format_profile());
    assert!(text.contains("No specifiers found"), "{text}");
}

#[test]
fn hover_fires_for_sprintf_specifier() {
    let src = "format {%d items} $count\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    let h = hover(src, 0, 10, &analysis, None).expect("hover");
    assert!(
        h.value.contains("Format string"),
        "expected sprintf hover, got: {value}",
        value = h.value,
    );
}

// binary format hover

fn binary_ctx(text: &str) -> BinaryContext {
    BinaryContext {
        text: text.to_string(),
        subcmd: "format".to_string(),
        args: Vec::new(),
    }
}

#[test]
fn scan_binary_fields_finds_basic_types() {
    let fields = scan_binary_fields("a4 H2 i", format_profile());
    let fulls: Vec<&str> = fields.iter().map(|f| f.full.as_str()).collect();
    assert_eq!(fulls, vec!["a4", "H2", "i"]);
    assert_eq!(fields[0].byte_size, Some(4));
    assert_eq!(fields[1].byte_size, Some(1));
    assert_eq!(fields[2].byte_size, Some(4));
}

#[test]
fn scan_binary_fields_handles_star_count() {
    let fields = scan_binary_fields("a* I*", format_profile());
    assert!(fields[0].star);
    assert!(fields[1].star);
    assert_eq!(fields[0].byte_size, None);
    assert_eq!(fields[1].byte_size, None);
}

#[test]
fn binary_format_hover_renders_summary_and_detail_table() {
    let text = binary_format_hover_text(&binary_ctx("a4 i"), format_profile());
    assert!(text.contains("**binary format**"), "{text}");
    assert!(text.contains("2 fields"), "{text}");
    assert!(text.contains("8 bytes"), "{text}");
    // Detail table now has 4 columns and a Bytes column.
    assert!(
        text.contains("| Spec | Variable | Type | Bytes |"),
        "{text}"
    );
    assert!(
        text.contains("| `a4` | a4 | str (null-pad) | 4 |"),
        "{text}"
    );
    assert!(text.contains("| `i` | i | int32 LE | 4 |"), "{text}");
}

#[test]
fn binary_format_hover_renders_byte_ruler_diagram() {
    let text = binary_format_hover_text(&binary_ctx("c s i"), format_profile());
    // Diagram fenced in a code block.
    assert!(text.contains("```"), "{text}");
    // Box-drawing characters for the field boundaries.
    assert!(text.contains('┌'), "{text}");
    assert!(text.contains('┬'), "{text}");
    assert!(text.contains('┐'), "{text}");
    // Numeric ruler — 7 bytes total (1 + 2 + 4).
    assert!(text.contains("0   1"), "{text}");
}

#[test]
fn binary_format_hover_omits_diagram_when_total_exceeds_32_bytes() {
    // `d` is 8 bytes; five of them = 40 bytes — over the
    // 32-byte diagram budget.
    let text = binary_format_hover_text(&binary_ctx("d5"), format_profile());
    assert!(text.contains("**binary format**"), "{text}");
    assert!(!text.contains('┌'), "diagram should be skipped: {text}");
}

#[test]
fn binary_format_hover_omits_diagram_when_size_unknown() {
    // `a*` has unknown byte count.
    let text = binary_format_hover_text(&binary_ctx("a*"), format_profile());
    assert!(!text.contains('┌'), "{text}");
    // The Bytes column still renders `…` for star fields.
    assert!(
        text.contains("| `a*` | a* | str (null-pad) | … |"),
        "{text}"
    );
}

#[test]
fn binary_format_hover_labels_fields_with_arg_names() {
    let ctx = BinaryContext {
        text: "c i".to_string(),
        subcmd: "scan".to_string(),
        args: vec!["byte".to_string(), "word".to_string()],
    };
    let text = binary_format_hover_text(&ctx, format_profile());
    // Detail-table Variable column gets the real names.
    assert!(text.contains("| `c` | byte |"), "{text}");
    assert!(text.contains("| `i` | word |"), "{text}");
    // Ruler diagram labels also pick up the names.
    assert!(text.contains("byte"), "{text}");
    assert!(text.contains("word"), "{text}");
}

#[test]
fn binary_format_hover_renders_uint_modifier() {
    let text = binary_format_hover_text(&binary_ctx("iu"), format_profile());
    assert!(text.contains("uint32"), "{text}");
}

#[test]
fn binary_format_hover_no_specifiers_returns_friendly_message() {
    let text = binary_format_hover_text(&binary_ctx("ZZZ"), format_profile());
    assert!(text.contains("No specifiers found"), "{text}");
}

#[test]
fn hover_fires_for_binary_specifier() {
    let src = "binary format {a4 i} val\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    let h = hover(src, 0, 17, &analysis, None).expect("hover");
    assert!(h.value.contains("binary format"), "{}", h.value);
}

#[test]
fn binary_scan_hover_uses_variable_names_from_parsed_words() {
    let source = "binary scan $buf \"cI\" byte word\n";
    let analysis = tcl_compiler::analyser::Analyser::new()
        .analyse(source, "tcl8.6")
        .clone();
    let column = u32::try_from(source.find("cI").expect("format word")).unwrap();
    let hover = hover(source, 0, column, &analysis, None).expect("binary scan hover");
    assert!(hover.value.contains("binary scan"), "{hover:?}");
    assert!(hover.value.contains("byte"), "{hover:?}");
    assert!(hover.value.contains("word"), "{hover:?}");
}

#[test]
fn hover_fires_for_bare_binary_format_specifier() {
    // Tcl accepts an unquoted format word.  The registry identifies it as
    // the `FormatString`; hover must not require braces or quotes.
    let src = "binary format c2s value\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    let h = hover(src, 0, 15, &analysis, None).expect("hover");
    assert!(h.value.contains("binary format"), "{}", h.value);
}
