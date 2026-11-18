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

//! Variable renames: declarations and reads, global aliases, braced and array references and reference replacement.

use super::*;

#[test]
fn rename_from_the_in_proc_global_alias_rewrites_the_callers_canonical_set_too() {
    // Triggering Rename from the in-proc `global tolComp` alias must
    // rewrite the caller's `set ::tolComp 0.05` too, not just the proc's
    // own 4 spans.  Leaving the caller behind decouples the two, so the
    // program silently falls back to `isEqual`'s hardcoded 0.01 default
    // instead of the caller-intended tolerance — a runtime break
    // introduced by a "successful" rename.
    let src =
        "proc use {} {\n    global tolComp\n    return $tolComp\n}\nset ::tolComp 0.05\nuse\n";
    let analysis = analyse(src);
    // Cursor on the `$tolComp` read inside the proc (line 2, col 14).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        14,
        "tolerance",
        &analysis,
        None,
    );
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(lines.contains(&1), "the `global tolComp` decl: {edits:?}");
    assert!(lines.contains(&2), "the in-proc $tolComp read: {edits:?}");
    assert!(
        lines.contains(&4),
        "the caller's canonical `set ::tolComp 0.05` must be rewritten too: {edits:?}"
    );
    let replacements: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(
        replacements.contains(&"::tolerance"),
        "the canonical cell's own `::` qualifier must be preserved; got {replacements:?}"
    );
}

#[test]
fn rename_from_the_callers_canonical_set_rewrites_every_in_proc_global_alias_too() {
    // The reverse direction of the test above:
    // triggering Rename from the caller's own `set ::tolComp` must
    // rewrite every in-proc `global tolComp` occurrence too, not just
    // the caller's own 2 spans.
    let src =
        "proc use {} {\n    global tolComp\n    return $tolComp\n}\nset ::tolComp 0.05\nuse\n";
    let analysis = analyse(src);
    // Cursor on the `set ::tolComp` declaration (line 4, col 8).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        4,
        8,
        "tolerance",
        &analysis,
        None,
    );
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(
        lines.contains(&4),
        "the caller's own decl must be rewritten: {edits:?}"
    );
    assert!(
        lines.contains(&1),
        "the in-proc `global tolComp` decl must be rewritten too: {edits:?}"
    );
    assert!(
        lines.contains(&2),
        "the in-proc $tolComp read must be rewritten too: {edits:?}"
    );
}

#[test]
fn rename_var_includes_decl_span() {
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    // Cursor inside `$x`.
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        7,
        "y",
        &analysis,
        None,
    );
    assert_ne!(edits, [] as [crate::rename::TextEdit; 0]);
    // Declaration replaces just `x` → `y`; reference
    // replaces `$x` → `$y` so the `$` prefix is preserved.
    let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(texts.contains(&"y"), "{texts:?}");
    assert!(texts.contains(&"$y"), "{texts:?}");
}

#[test]
fn rename_from_proc_param_bareword_declaration_rewrites_every_use() {
    // TP — renaming from a cursor placed directly on a proc parameter's
    // own bareword name (not a `$`-prefixed read) must rewrite both the
    // parameter's declaration and its `$name` read.  Answering *zero*
    // edits is worse than failing loudly: the user has no signal that
    // anything went wrong.
    let src = "proc greet {name} { return $name }\n";
    let analysis = analyse(src);
    // Cursor on `name` inside the parameter list (col 12-16).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        13,
        "label",
        &analysis,
        None,
    );
    assert_eq!(edits.len(), 2, "{edits:?}");
    // The declaration rewrites bare `name` -> `label`; the `$name` read
    // preserves its `$` prefix -> `$label` (mirrors
    // `rename_var_includes_decl_span`'s established convention).
    let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(texts.contains(&"label"), "{texts:?}");
    assert!(texts.contains(&"$label"), "{texts:?}");
    assert!(edits.iter().any(|e| e.range.start_line == 0), "{edits:?}");
}

#[test]
fn rename_from_catch_resultvar_bareword_rewrites_the_original_declaration_too() {
    // TP — the finding's other confirmed shape: a `catch script name`
    // result-var reuses an existing variable; renaming from a cursor
    // on its own bareword token must rewrite every occurrence,
    // including the original declaration elsewhere in the proc.
    let src = "proc resolveSwitch {name def} {\n    catch {error boom} name\n    return $name\n}\n";
    let analysis = analyse(src);
    // Cursor on the catch result-var `name` (line 1, col 23-27).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        24,
        "resolved",
        &analysis,
        None,
    );
    assert!(
        edits.iter().any(|e| e.range.start_line == 0),
        "original param declaration must be rewritten too: {edits:?}"
    );
    assert!(
        edits.iter().any(|e| e.range.start_line == 1),
        "the catch result-var site itself must be rewritten: {edits:?}"
    );
    assert!(
        edits.iter().any(|e| e.range.start_line == 2),
        "the later $name read must be rewritten: {edits:?}"
    );
    // The formal list and lexical root have independent edit extents;
    // applying the complete edit set preserves the read's original `$`.
    assert_eq!(
        apply_edits(src, &edits),
        "proc resolveSwitch {resolved def} {\n    catch {error boom} resolved\n    return $resolved\n}\n",
    );
}

// brace-ref escaping

#[test]
fn rename_var_preserves_braced_reference_form() {
    let src = "set x 1\nputs ${x}\n";
    let analysis = analyse(src);
    // Cursor inside `${x}` on the `x`.
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        7,
        "y",
        &analysis,
        None,
    );
    let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(texts.contains(&"y"), "{texts:?}");
    assert!(texts.contains(&"${y}"), "{texts:?}");
}

#[test]
fn var_ref_edit_span_extends_the_ordinary_braced_form() {
    let src = "${x}";
    // The lexer's own span for a non-degenerate name stops one byte
    // short of the closing `}` (see `var_ref_edit_span`'s doc).
    assert_eq!(
        var_ref_edit_span(src, tcl_lexer::Span::new(0, 3)),
        tcl_lexer::Span::new(0, 4)
    );
}

#[test]
fn var_ref_edit_span_leaves_the_degenerate_empty_name_span_untouched() {
    // `${}` immediately followed by a literal (unrelated) `}` — the
    // pathological case that would fool a naive "does the next byte
    // look like `}`" check. The lexer already extends `${}`'s own
    // span to include its closing brace, so `var_ref_edit_span` must
    // recognise the degenerate case and stop, not also swallow the
    // following literal `}` into the edit range.
    let src = "${}}";
    assert_eq!(
        var_ref_edit_span(src, tcl_lexer::Span::new(0, 3)),
        tcl_lexer::Span::new(0, 3)
    );
}

#[test]
fn var_ref_edit_span_extends_a_tcl9_nested_brace_name() {
    // `${a{b}}` names `a{b}` (Tcl 9's brace-nesting rule) — the span
    // stops before the *outer* `}`, one byte short, same as the
    // ordinary case.
    let src = "${a{b}}";
    assert_eq!(
        var_ref_edit_span(src, tcl_lexer::Span::new(0, 6)),
        tcl_lexer::Span::new(0, 7)
    );
}

#[test]
fn var_ref_edit_span_covers_a_braced_name_word() {
    // A braced *name* word (`set {a b} 1`) follows the same inner-end
    // convention as `${…}`, so an edit range that stops short leaves the
    // `}` orphaned after the replacement.  `word_span_at` covers every
    // delimited shape, so both are widened the same way.
    let src = "set {a b} 1";
    assert_eq!(&src[4..8], "{a b");
    assert_eq!(
        var_ref_edit_span(src, tcl_lexer::Span::new(4, 8)),
        tcl_lexer::Span::new(4, 9)
    );
}

#[test]
fn var_ref_edit_span_leaves_an_empty_braced_word_untouched() {
    // The universal discriminator: an empty `{}` whose span already
    // covers its own closer, with the enclosing body's `}` right after.
    let src = "proc p {} {}";
    assert_eq!(
        var_ref_edit_span(src, tcl_lexer::Span::new(7, 9)),
        tcl_lexer::Span::new(7, 9)
    );
}

#[test]
fn var_ref_edit_span_leaves_non_braced_forms_untouched() {
    let src = "$x";
    let span = tcl_lexer::Span::new(0, 2);
    assert_eq!(var_ref_edit_span(src, span), span);
}

#[test]
fn var_ref_edit_span_leaves_an_unterminated_reference_untouched() {
    // No closing brace anywhere in the source to extend to.
    let src = "${x";
    let span = tcl_lexer::Span::new(0, 3);
    assert_eq!(var_ref_edit_span(src, span), span);
}

#[test]
fn rename_array_variable_applying_the_braced_index_edit_does_not_duplicate_the_closing_brace() {
    let src = "set arr(0) 1\nputs ${arr(0)}\n";
    let analysis = analyse(src);
    // Cursor on `arr` inside `${arr(0)}` (line 1, col 7).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        7,
        "data",
        &analysis,
        None,
    );
    assert_eq!(apply_edits(src, &edits), "set data(0) 1\nputs ${data(0)}\n");
}

#[test]
fn rename_var_applying_the_braced_reference_edit_does_not_duplicate_the_closing_brace() {
    // The `Var` token's own lexer span for a
    // non-degenerate `${name}` form stops one byte short of the
    // closing `}` (`${a{b}}` names `a{b}`, whose content can itself
    // legitimately end in `}` — see `var_ref_edit_span`'s doc), so
    // using that raw span as the *edit range* (rather than just for
    // matching/reference bookkeeping) left the source's own original
    // `}` sitting right after the replacement once applied, silently
    // corrupting `${x}` into `${y}}`. `rename_var_preserves_braced_reference_form`
    // above already pins `new_text`; this applies the edit for real.
    let src = "set x 1\nputs ${x}\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        7,
        "y",
        &analysis,
        None,
    );
    assert_eq!(apply_edits(src, &edits), "set y 1\nputs ${y}\n");
}

#[test]
fn rename_var_applying_the_dir_view_idiom_edit_produces_valid_tcl() {
    // A one-byte-short range would apply `${dir}view` as `${direction}}view`,
    // which tclsh 8.6/9.0 cannot parse. tclsh 8.6.18 and 9.0.4: [p y] is
    // yview before and after. In the `tk.tcl` form `$w ${dir}view ...` the
    // computed head may read `dir` through `upvar 1` (a peek proc gives y
    // before and can't read "d" after), so that rename refuses.
    let src = "proc p {dir} {\n    return ${dir}view\n}\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        1,
        14,
        "direction",
        &analysis,
        None,
    );
    assert_eq!(
        apply_edits(src, &edits),
        "proc p {direction} {\n    return ${direction}view\n}\n"
    );
    let src = "proc m {w dir} {\n    $w ${dir}view scroll 1 units\n}\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        1,
        9,
        "direction",
        &analysis,
        None,
    );
    assert_eq!(edits, [] as [TextEdit; 0]);
}

#[test]
fn rename_array_variable_preserves_element_index() {
    // Renaming the base array variable `arr` must keep each
    // element index intact: `$arr(0)` → `$data(0)`, not `$data`.
    let src = "set arr(0) 1\nputs $arr(0)\n";
    let analysis = analyse(src);
    // Cursor on `arr` in the `set` target (line 0, col 4).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        6,
        "data",
        &analysis,
        None,
    );
    assert!(!edits.is_empty(), "expected array rename edits");
    let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(
        texts.contains(&"$data(0)"),
        "reference edit must preserve the array index, got {texts:?}",
    );
}

#[test]
fn rename_array_variable_preserves_index_in_braced_reference() {
    // The `${arr(0)}` braced form is recorded against base `arr`,
    // so renaming must rewrite it to `${data(0)}` (index kept).
    let src = "set arr(0) 1\nputs ${arr(0)}\n";
    let analysis = analyse(src);
    // Cursor on `arr` inside `${arr(0)}` (line 1, col 7).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        7,
        "data",
        &analysis,
        None,
    );
    assert!(!edits.is_empty(), "expected braced array rename edits");
    let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(
        texts.contains(&"${data(0)}"),
        "braced reference edit must preserve the array index, got {texts:?}",
    );
}

#[test]
fn build_var_ref_replacement_handles_all_forms() {
    // Pure helper exercise — bare `$x`, braced `${x}`, and
    // qualified forms all produce the right replacement text.
    // Namespace-qualified refs preserve the prefix so e.g.
    // `$ns::z` → `$ns::c` even if the analyser doesn't yet
    // surface namespace-scoped variable lookups (the helper
    // is the building block; the lookup side isn't wired
    // yet).
    let src = "  $x  ${y}  $ns::z  ${ns::w}  ";
    let span_x = tcl_lexer::Span::new(2, 4);
    let span_braced_y = tcl_lexer::Span::new(6, 10);
    let span_qualified_z = tcl_lexer::Span::new(12, 18);
    let span_braced_w = tcl_lexer::Span::new(20, 28);
    assert_eq!(
        build_var_ref_replacement(src, span_x, "a", tcl_lexer::LexerConfig::default()).unwrap(),
        "$a"
    );
    assert_eq!(
        build_var_ref_replacement(src, span_braced_y, "b", tcl_lexer::LexerConfig::default())
            .unwrap(),
        "${b}"
    );
    assert_eq!(
        build_var_ref_replacement(
            src,
            span_qualified_z,
            "c",
            tcl_lexer::LexerConfig::default()
        )
        .unwrap(),
        "$ns::c"
    );
    assert_eq!(
        build_var_ref_replacement(src, span_braced_w, "d", tcl_lexer::LexerConfig::default())
            .unwrap(),
        "${ns::d}"
    );
}

#[test]
fn build_var_ref_replacement_preserves_array_index() {
    // Renaming the base array variable must keep the element
    // index: `$arr(idx)` → `$data(idx)`, not `$data`.
    let src = "$arr(idx)  $arr($i)  $ns::a(k)";
    assert_eq!(
        build_var_ref_replacement(
            src,
            tcl_lexer::Span::new(0, 9),
            "data",
            tcl_lexer::LexerConfig::default()
        )
        .unwrap(),
        "$data(idx)"
    );
    // A substituted index is copied verbatim (the inner `$i`
    // reference is renamed on its own if `i` is renamed).
    assert_eq!(
        build_var_ref_replacement(
            src,
            tcl_lexer::Span::new(11, 19),
            "data",
            tcl_lexer::LexerConfig::default()
        )
        .unwrap(),
        "$data($i)"
    );
    // Namespace prefix + array index together.
    assert_eq!(
        build_var_ref_replacement(
            src,
            tcl_lexer::Span::new(21, 30),
            "b",
            tcl_lexer::LexerConfig::default()
        )
        .unwrap(),
        "$ns::b(k)"
    );
}

#[test]
fn build_var_ref_replacement_preserves_array_index_in_braced_form() {
    // `${arr(idx)}` is recorded against base `arr`, so renaming it
    // must keep the index inside the braces: `${data(idx)}`.
    let src = "${arr(idx)}  ${ns::a(k)}";
    assert_eq!(
        build_var_ref_replacement(
            src,
            tcl_lexer::Span::new(0, 11),
            "data",
            tcl_lexer::LexerConfig::default()
        )
        .unwrap(),
        "${data(idx)}"
    );
    assert_eq!(
        build_var_ref_replacement(
            src,
            tcl_lexer::Span::new(13, 23),
            "b",
            tcl_lexer::LexerConfig::default()
        )
        .unwrap(),
        "${ns::b(k)}"
    );
}

#[test]
fn original_reference_replacement_preserves_closed_elements_and_scalar_parentheses() {
    // Implementation contract: naming.core.selected-variable-cursor-syntax
    // docs/design/analysis/name-resolution-proofs/selected-variable-cursor-syntax.md
    for style in [
        tcl_dialect::BracedVarStyle::FirstClose,
        tcl_dialect::BracedVarStyle::Tcl9Nesting,
    ] {
        let config = tcl_lexer::LexerConfig {
            braced_var: style,
            ..Default::default()
        };
        for (source, expected) in [
            ("${scalar(open}", "${new}"),
            ("${scalar(open)tail}", "${new}"),
            ("${arr(first(second)}", "${new(first(second)}"),
            ("${cash$name}", "${new}"),
            ("${café🙂}", "${new}"),
            ("$arr($idx)", "$new($idx)"),
            ("${ns::arr(k)}", "${ns::new(k)}"),
        ] {
            let span = tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap());
            assert_eq!(
                build_var_ref_replacement(source, span, "new", config).as_deref(),
                Some(expected)
            );
        }
        for source in ["${missing", "$a tail", "${a}(tail)", "not_a_reference"] {
            let span = tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap());
            assert!(build_var_ref_replacement(source, span, "new", config).is_none());
        }
        assert!(
            build_var_ref_replacement("$a", tcl_lexer::Span::new(0, 9), "new", config).is_none()
        );
    }
    assert_eq!(split_array_suffix("scalar(open"), ("scalar(open", ""));
    assert_eq!(
        split_array_suffix("scalar(open)tail"),
        ("scalar(open)tail", "")
    );
}

#[test]
fn original_logical_rename_keeps_scalar_parentheses_and_literal_dollar_names() {
    // Implementation contract: naming.core.selected-variable-cursor-syntax
    // docs/design/analysis/name-resolution-proofs/selected-variable-cursor-syntax.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let context = tcl_registry::model::ingress::context_for_profile(profile);
    for name in [
        "scalar(open",
        "scalar(open)tail",
        "cash$name",
        "$n",
        "café🙂",
    ] {
        let source = format!("set {{{name}}} 1\nputs ${{{name}}}\n");
        let input = ResolvedAnalysisInput::new(profile, profile, context.clone(), config);
        let analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(&source, profile.name)
            .clone();
        assert_eq!(analysis.resolved_input.as_ref(), Some(&input));
        assert!(analysis.allows_lexical_declaration_advice());
        assert!(
            rename_with_diagnosis(&source, profile, 1, 7, "new", &analysis, None).is_err(),
            "{name}"
        );
        let proposal = conditional_variable_rename_with_diagnosis(&source, 1, 7, "new", &analysis)
            .unwrap_or_else(|error| panic!("{name}: {error:?}"));
        let width = u32::try_from(name.encode_utf16().count()).unwrap();
        assert_eq!(
            proposal.edits,
            vec![
                TextEdit {
                    range: LspRange {
                        start_line: 0,
                        start_character: 4,
                        end_line: 0,
                        end_character: 6 + width,
                    },
                    new_text: "{new}".to_owned(),
                },
                TextEdit {
                    range: LspRange {
                        start_line: 1,
                        start_character: 7,
                        end_line: 1,
                        end_character: 7 + width,
                    },
                    new_text: "new".to_owned(),
                },
            ],
            "{name}"
        );
        assert_eq!(
            proposal.generated_source, "set {new} 1\nputs ${new}\n",
            "{name}"
        );
        assert_eq!(
            apply_edits(&source, &proposal.edits),
            proposal.generated_source
        );
        assert_eq!(
            rename(
                &(source.clone() + " "),
                profile,
                1,
                7,
                "new",
                &analysis,
                None
            ),
            [] as [crate::rename::TextEdit; 0]
        );
        assert!(
            conditional_variable_rename_with_diagnosis(
                &(source.clone() + " "),
                1,
                7,
                "new",
                &analysis,
            )
            .is_err(),
            "{name}"
        );
    }
}

#[test]
fn rename_var_from_definition_site_resolves_without_dollar() {
    // Cursor on the `x` in `set x 42` (no `$`) — the def-site
    // resolver must find the variable and rewrite decl + reads.
    let src = "set x 42\nputs $x\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        4,
        "newvar",
        &analysis,
        None,
    );
    let texts: std::collections::HashSet<&str> =
        edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(texts.contains("newvar"), "decl edit missing: {edits:?}");
    assert!(texts.contains("$newvar"), "ref edit missing: {edits:?}");
}

#[test]
fn rename_var_preserves_namespace_qualifier_at_decl() {
    let src = "set myns::count 0\nputs $myns::count\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        10,
        "total",
        &analysis,
        None,
    );
    let texts: std::collections::HashSet<&str> =
        edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(texts.contains("myns::total"), "decl ns lost: {edits:?}");
    assert!(texts.contains("$myns::total"), "ref ns lost: {edits:?}");
}

#[test]
fn rename_var_rejected_on_same_scope_collision() {
    let src = "proc demo {} {\n    set x 1\n    set y 2\n    puts $x\n}\n";
    let analysis = analyse(src);
    // Rename `x` (read site, line 3) to `y` which already exists in scope.
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        3,
        10,
        "y",
        &analysis,
        None,
    );
    assert!(edits.is_empty(), "collision not rejected: {edits:?}");
}

#[test]
fn logical_rename_preserves_original_separator_runs_unicode_and_array_extent() {
    // naming.editor.logical-written-component-geometry
    // docs/design/analysis/name-resolution-proofs/logical-written-component-geometry.md
    // Exact source geometry only; edit permission stays with the caller.
    let config =
        tcl_lexer::LexerConfig::from_grammar(tcl_dialect::DialectProfile::plain_tcl().grammar);
    for (written, expected) in [
        ("${::::茶:::箱(槽)}", "${::::茶:::total(槽)}"),
        ("$::::lib:::box(槽)", "$::::lib:::total(槽)"),
        ("${茶:箱}", "${total}"),
    ] {
        let span = tcl_lexer::Span::new(0, u32::try_from(written.len()).unwrap());
        assert_eq!(
            build_var_ref_replacement(written, span, "total", config).as_deref(),
            Some(expected)
        );
        assert!(
            build_var_ref_replacement(written, tcl_lexer::Span::new(0, 1), "total", config)
                .is_none()
        );
    }
    let bare_unicode = "$::::茶:::箱(槽)";
    let bare_unicode_span = tcl_lexer::Span::new(0, u32::try_from(bare_unicode.len()).unwrap());
    assert!(build_var_ref_replacement(bare_unicode, bare_unicode_span, "total", config).is_none());
    for (written, expected) in [
        ("::::茶:::méthode", "::::茶:::changed"),
        ("茶::::", "茶::::changed"),
        ("茶:méthode", "changed"),
    ] {
        assert_eq!(
            invocation_replacement("unused", "unused", "changed", written),
            expected
        );
    }
    let source = "set 茶:::箱(槽) VALUE";
    let analysis = Analyser::new().analyse(source, "tcl");
    let start = source.find("茶").unwrap();
    let end = source.find(" VALUE").unwrap();
    let span = tcl_lexer::Span::new(u32::try_from(start).unwrap(), u32::try_from(end).unwrap());
    assert_eq!(
        build_var_replacement_in_analysis(source, &analysis, span, "total")
            .unwrap()
            .1,
        "茶:::total(槽)"
    );
    assert!(
        build_var_replacement_in_analysis("set 茶:::箱(槽) OTHER", &analysis, span, "total")
            .is_none()
    );
}
