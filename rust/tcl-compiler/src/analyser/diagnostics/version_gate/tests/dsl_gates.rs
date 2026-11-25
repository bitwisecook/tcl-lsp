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

//! W137, W138, W200 and W202: `format`, `scan`, `binary` and `string is` forms gated by release.

use super::*;

/// Version-gate + argument-DSL codes for `source` under `dialect`.
fn dsl_diags(source: &str, dialect: &str) -> Vec<(String, String)> {
    Analyser::new()
        .analyse(source, dialect)
        .diagnostics
        .iter()
        .filter(|d| matches!(d.code.as_str(), "W137" | "W138" | "W200" | "W202"))
        .map(|d| (d.code.to_string(), d.message.clone()))
        .collect()
}

#[test]
fn original_format_gates_keep_alias_ordinals_and_written_template_anchors() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    // This checks readonly source metadata and diagnostic geometry, not
    // alias installation or successful execution in a native interpreter.
    for source in [
        "interp alias {} encode {} binary format\nencode cu 5\n",
        "rename binary encode\nencode format cu 5\n",
        "::binary format cu 5\n",
        "binary fo cu 5\n",
        "set output [binary format cu 5]\n",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.4");
        let found = analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::W200)
            .collect::<Vec<_>>();
        assert_eq!(found.len(), 1, "{source}: {found:?}");
        assert_eq!(&source[found[0].span.as_range()], "cu", "{source}");
    }
    // The effective template is a captured prefix. No invocation operand
    // can borrow its location from the alias declaration or a later value.
    assert_eq!(
        dsl_diags(
            "interp alias {} encode {} binary format cu\nencode 5\n",
            "tcl8.4",
        ),
        [] as [(std::string::String, std::string::String); 0]
    );
}

#[test]
fn original_format_gates_read_literal_values_without_promoting_dynamic_words() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    // These source-consumer controls do not claim native runtime results.
    for (source, template) in [
        (r"format \x25\u0062 5", b"%b".as_slice()),
        (r"binary format c\u0075 5", b"cu".as_slice()),
    ] {
        let result = Analyser::new().analyse(source, "tcl8.4");
        let config = result.body_lexer_config.unwrap();
        let segment =
            crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
        let words = crate::registry_invocation::source_structure::source_registry_words(
            source, &result, &segment,
        )
        .unwrap_or_else(|| panic!("source schema missing: {source}"));
        let context = result.resolved_input.as_ref().unwrap().context_registry();
        let formats = words
            .with_source_schema(
                &context,
                crate::analyser::diagnostic_registry::source_format_arguments,
            )
            .flatten()
            .unwrap_or_else(|| panic!("format schema missing: {source}"));
        assert_eq!(formats.len(), 1, "format role missing: {source}");
        assert_eq!(
            words.arguments()[formats[0].index].literal_bytes(),
            Some(template),
            "shared literal bytes: {source}"
        );
        assert_eq!(dsl_diags(source, "tcl8.4").len(), 1, "{source}");
    }
    // naming.source.original-static-escape-values
    // docs/design/analysis/name-resolution-proofs/original-source-static-escape-values.md
    // SourceEscapes173 records pre-8.6 greedy hex as '[' for this exact
    // spelling. It therefore contains no percent conversion to version-gate.
    for dialect in ["tcl8.4", "tcl8.5"] {
        assert!(
            dsl_diags(r"format \x25b 5", dialect).is_empty(),
            "{dialect}"
        );
    }
    for source in [
        r"format {\x25b} 5",
        r#"format "%b$part" 5"#,
        "format $template 5",
        "format [makeTemplate] 5",
        "binary {*}$selector cu 5",
        "binary $selector cu 5",
        "proc format args {}; format %b 5",
        "proc binary args {}; binary format cu 5",
        "rename format {}; format %b 5",
        "namespace eval n {proc binary args {}; binary format cu 5}",
        "clock format 0 -format {%b}",
    ] {
        assert!(dsl_diags(source, "tcl8.4").is_empty(), "{source}");
    }
    for dialect in ["tcl8.5", "tcl8.6"] {
        assert!(
            dsl_diags("format {*}$arguments %llu 5", dialect).is_empty(),
            "{dialect}: unknown expansion supplies no template ordinal"
        );
    }
}

#[test]
fn w202_gated_field_letter_names_the_letter_and_the_floor() {
    let diags = dsl_diags("binary format q 1.0\n", "tcl8.4");
    let (code, msg) = diags
        .iter()
        .find(|(c, _)| c == "W202")
        .expect("q is bad field specifier on tclsh 8.4.20");
    assert_eq!(code, "W202");
    assert!(msg.contains("'q'"), "{msg}");
    assert!(msg.contains("8.5"), "{msg}");
    // Clean once the floor is met.
    assert_eq!(
        dsl_diags("binary format q 1.0\n", "tcl8.6"),
        [] as [(std::string::String, std::string::String); 0]
    );
}

/// The argument-DSL gates run against the command a head *is*, not the
/// word it is spelled with (#2065).
///
/// Both reproductions shadow a built-in from inside a `namespace eval`
/// body, where C Tcl resolves the bare head in that namespace before the
/// global table — tclsh 8.6.18 / 9.0.4, byte-identical:
///
/// ```tcl
/// namespace eval n {proc format {args} {return F}; puts [format %b 5]}
/// namespace eval m {proc binary {args} {return B}
///                   puts [binary format q 1.0]}      ;# F, then B
/// ```
#[test]
fn a_shadowed_head_is_not_gated_as_the_builtin() {
    for (src, code) in [
        (
            "namespace eval n { proc format {args} {}\nformat %b 5 }\n",
            "W138",
        ),
        (
            "namespace eval n { proc binary {args} {}\nbinary format q 1.0 }\n",
            "W202",
        ),
        // The document-level shadow reads the same way.
        ("proc format {args} {}\nformat %b 5\n", "W138"),
        // As does a name the document renamed the built-in away from.
        ("rename format origfmt\nformat %b 5\n", "W138"),
    ] {
        assert_eq!(
            dsl_diags(src, "tcl8.4").len(),
            0,
            "{code} must not gate a head the document rebound: {src}"
        );
    }
    // The shadow is not a licence to stop gating everywhere: a call
    // outside the namespace still runs the built-in, and the rename's
    // target still *is* the built-in.
    assert!(
        dsl_diags(
            "namespace eval n { proc format {args} {} }\nformat %b 5\n",
            "tcl8.4"
        )
        .iter()
        .any(|(c, _)| c == "W138"),
        "a namespace-local shadow must not silence the global call"
    );
    assert!(
        dsl_diags("rename format origfmt\norigfmt %b 5\n", "tcl8.4")
            .iter()
            .any(|(c, m)| c == "W138" && m.contains("origfmt")),
        "the gate follows a proven rename onto its new name"
    );
}

#[test]
fn w138_format_binary_conversion_is_86_gated() {
    // Oracle-verified surface: `%b` was added in Tcl 8.6.
    let src = "format %b 5\n";
    // TP: 8.4/8.5-era runtimes (incl. iRules' embedded 8.4.6).
    for d in ["tcl8.4", "tcl8.5", "f5-irules", "f5-iapps", "f5-tmsh"] {
        let diags = dsl_diags(src, d);
        assert!(
            diags.iter().any(|(c, m)| c == "W138" && m.contains("%b")),
            "{d}: %b needs 8.6, got {diags:?}"
        );
    }
    // TN: 8.6+ runtimes.
    for d in ["tcl8.6", "tcl9.0", "expect", "bpf", "synopsys-eda-tcl"] {
        assert_eq!(dsl_diags(src, d).len(), 0, "{d}: %b is real on 8.6+");
    }
    // FP-guard: `%%b` is a literal percent + `b`, not the conversion;
    // a dynamic format string abstains.
    assert_eq!(
        dsl_diags("format %%b 5\n", "tcl8.4"),
        [] as [(std::string::String, std::string::String); 0]
    );
    assert_eq!(
        dsl_diags("format $fmt 5\n", "tcl8.4"),
        [] as [(std::string::String, std::string::String); 0]
    );
    // The permissive fallback abstains entirely (§8).
    assert_eq!(
        dsl_diags(src, "tcl"),
        [] as [(std::string::String, std::string::String); 0]
    );
}

#[test]
fn w138_format_unsigned_bignum_is_90_gated() {
    // Oracle-verified: tclsh8.6 raises "unsigned bignum format is
    // invalid" for %llu; tclsh9.0.4 renders it.
    let src = "format %llu 5\n";
    let diags = dsl_diags(src, "tcl8.6");
    assert!(
        diags.iter().any(|(c, m)| c == "W138" && m.contains("%llu")),
        "tcl8.6: %llu needs 9.0, got {diags:?}"
    );
    assert_eq!(dsl_diags(src, "tcl9.0").len(), 0, "9.0 renders %llu");
    // Plain %lld is fine everywhere the ladder models.
    assert_eq!(
        dsl_diags("format %lld 5\n", "tcl8.6"),
        [] as [(std::string::String, std::string::String); 0]
    );
}

#[test]
fn w138_scan_binary_conversion_is_86_gated() {
    let src = "scan 101 %b x\n";
    let diags = dsl_diags(src, "tcl8.5");
    assert!(
        diags.iter().any(|(c, m)| c == "W138" && m.contains("%b")),
        "tcl8.5: scan %b needs 8.6, got {diags:?}"
    );
    assert_eq!(
        dsl_diags(src, "tcl8.6"),
        [] as [(std::string::String, std::string::String); 0]
    );
}

#[test]
fn w137_string_is_class_follows_the_effective_version() {
    // `string is dict` — oracle-verified 9.0-only (tclsh8.6: bad
    // class; tclsh9.0: works).
    let src = "string is dict {a 1}\n";
    for d in ["tcl8.4", "tcl8.6", "f5-iapps", "f5-tmsh"] {
        let diags = dsl_diags(src, d);
        assert!(
            diags
                .iter()
                .any(|(c, m)| c == "W137" && m.contains("'dict'")),
            "{d}: string is dict needs 9.0, got {diags:?}"
        );
    }
    for d in ["tcl9.0", "tcl9.1", "bpf"] {
        assert_eq!(dsl_diags(src, d).len(), 0, "{d}: dict class is real");
    }
    // entier is 8.6+; wideinteger is 8.5+.
    assert!(
        dsl_diags("string is entier 5\n", "tcl8.5")
            .iter()
            .any(|(c, _)| c == "W137"),
        "entier needs 8.6"
    );
    assert_eq!(
        dsl_diags("string is entier 5\n", "tcl8.6"),
        [] as [(std::string::String, std::string::String); 0]
    );
    assert!(
        dsl_diags("string is wideinteger 5\n", "tcl8.4")
            .iter()
            .any(|(c, _)| c == "W137"),
        "wideinteger needs 8.5"
    );
    assert_eq!(
        dsl_diags("string is wideinteger 5\n", "tcl8.5"),
        [] as [(std::string::String, std::string::String); 0]
    );
    // FP-guards: an always-available class, a dynamic class, and the
    // unique-prefix abbreviation of an ungated class stay silent.
    assert_eq!(
        dsl_diags("string is alpha abc\n", "tcl8.4"),
        [] as [(std::string::String, std::string::String); 0]
    );
    assert_eq!(
        dsl_diags("string is $cls abc\n", "tcl8.4"),
        [] as [(std::string::String, std::string::String); 0]
    );
    assert_eq!(
        dsl_diags("string is xd abc\n", "tcl8.4"),
        [] as [(std::string::String, std::string::String); 0]
    );
}

#[test]
fn dsl_gates_honour_a_package_require_tcl_floor() {
    // §6.1: `package require Tcl 9.0` raises the effective version
    // above the ambient tcl8.6 dialect — the file validates as 9.0.
    let src = "package require Tcl 9.0\nformat %llu 5\nstring is dict {a 1}\n";
    assert_eq!(
        dsl_diags(src, "tcl8.6").len(),
        0,
        "a 9.0 core floor admits 9.0 DSL features"
    );
}

#[test]
fn a_prerelease_package_require_tcl_sets_its_release_floor() {
    // `8.6b1` and `9.0b1` name the 8.6 and 9.0 lines (item 147): the floor
    // they set is the one their release sets.
    for (release, ambient, src) in [
        ("8.6", "tcl8.5", "format %b 5\n"),
        ("9.0", "tcl8.6", "format %llu 5\nstring is dict {a 1}\n"),
    ] {
        let floored =
            |version: &str| dsl_diags(&format!("package require Tcl {version}\n{src}"), ambient);
        assert!(
            !dsl_diags(src, ambient).is_empty(),
            "{release} feature gated under {ambient}"
        );
        assert_eq!(
            floored(&format!("{release}b1")),
            floored(release),
            "{release}b1"
        );
        assert!(
            floored(&format!("{release}b1")).is_empty(),
            "{release}b1 admits {release}"
        );
    }
}

#[test]
fn dsl_gates_preserve_older_alternative_package_require_versions() {
    let source = "package require Tcl 9.0 8.6\nformat %llu 5\n";
    assert!(
        dsl_diags(source, "tcl8.6")
            .iter()
            .any(|(code, _)| code == "W138")
    );
}

#[test]
fn w200_binary_modifiers_follow_the_effective_version() {
    // TIP 275: binary format/scan u/s modifiers are 8.5+. The F5
    // trunk profiles all ride the fork of Tcl at 8.4.6 — measured,
    // bigip-irule-parser-measurements.md §4a: iApps/tmsh fail every
    // 8.5 discriminator, so the modifiers flag there too.
    let src = "binary format cu 5\n";
    for d in ["tcl8.4", "f5-irules", "f5-iapps", "f5-tmsh"] {
        let diags = dsl_diags(src, d);
        assert!(
            diags.iter().any(|(c, _)| c == "W200"),
            "{d}: binary u modifier needs 8.5, got {diags:?}"
        );
    }
    for d in ["tcl8.5", "tcl8.6"] {
        assert_eq!(
            dsl_diags(src, d).len(),
            0,
            "{d}: binary u modifier is real on 8.5+"
        );
    }
}
