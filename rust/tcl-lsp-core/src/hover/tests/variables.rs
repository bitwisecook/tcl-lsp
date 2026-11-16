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

//! Variable cards: reads and declarations, inferred type and taint, special variables.

use super::*;

#[test]
fn hover_on_dollar_var_returns_var_text() {
    // Variable defined at top level, referenced via `$x`.
    let src = "set x 1\nset y $x\n";
    let analysis = analyse(src);
    let h = hover(src, 1, 7, &analysis, None);
    let h = h.expect("referenced variable should have hover");
    assert!(h.value.contains("Variable"), "{}", h.value);
    assert!(h.value.contains("`x`"), "{}", h.value);
}

#[test]
fn hover_on_proc_param_bareword_declaration_resolves() {
    // TP — a cursor on a proc parameter's own bareword name (not a
    // `$`-prefixed read) must hover, just as the same variable's `$name`
    // reads do.
    let src = "proc greet {name} { return $name }\n";
    let analysis = analyse(src);
    // Cursor on `name` inside the parameter list (col 12-16).
    let h = hover(src, 0, 13, &analysis, None).expect("hover");
    assert!(h.value.contains("**Variable** `name`"), "{}", h.value);
}

#[test]
fn hover_on_catch_resultvar_bareword_resolves() {
    // TP — the other bareword shape: a `catch script name`
    // result-var reuses an existing variable; its own bareword token
    // must still hover, surfacing the same variable.
    let src = "proc resolveSwitch {name def} {\n    catch {foo} name\n    return $name\n}\n";
    let analysis = analyse(src);
    // Cursor on the catch result-var `name` (line 1, col 16-20).
    let h = hover(src, 1, 17, &analysis, None).expect("hover");
    assert!(h.value.contains("**Variable** `name`"), "{}", h.value);
}

#[test]
fn hover_on_var_surfaces_inferred_intrep() {
    // `x` is assigned an integer literal; the compiler's
    // type-propagation pass should infer `int`, and the hover
    // (given a registry) should surface it.
    let src = "set x 1\nset y $x\n";
    let analysis = analyse(src);
    let registry = tcl_registry::CommandRegistry::build_default();
    let h = hover(src, 1, 7, &analysis, Some(&registry)).expect("hover");
    assert!(h.value.contains("**Variable** `x`"), "{}", h.value);
    assert!(h.value.contains("**Inferred intrep**: int"), "{}", h.value);
}

#[test]
fn hover_inference_keeps_same_named_variables_in_the_selected_source_body() {
    for source in [
        "set same 1\nproc local {} {set same TEXT; puts $same}\n",
        "set same [gets stdin]\nproc local {} {set same TEXT; puts $same}\n",
        "proc first {} {set same 1; puts $same}\nproc second {} {set same TEXT; puts $same}\n",
    ] {
        let analysis = analyse(source);
        let cursor = u32::try_from(source.rfind("$same").unwrap() + 1).unwrap();
        let position = tcl_lexer::LineIndex::new(source).position_at_utf16(cursor, source);
        let registry = tcl_registry::CommandRegistry::build_default();
        let selected = hover(
            source,
            position.line,
            position.character.get(),
            &analysis,
            Some(&registry),
        )
        .expect("actual selected local variable hover");
        assert!(
            selected.value.contains("**Inferred intrep**: string"),
            "{}",
            selected.value
        );
        assert!(
            !selected.value.contains("**Inferred intrep**: int"),
            "{}",
            selected.value
        );
        assert!(!selected.value.contains("**Taint**"), "{}", selected.value);
    }
}

#[test]
fn hover_inference_uses_the_represented_read_version_after_reassignment() {
    let source = "proc local {} {set same 123; puts $same; set same TEXT; puts $same}\n";
    let analysis = analyse(source);
    let registry = tcl_registry::CommandRegistry::build_default();
    let cursor = u32::try_from(source.rfind("$same").unwrap() + 1).unwrap();
    let position = tcl_lexer::LineIndex::new(source).position_at_utf16(cursor, source);
    let selected = hover(
        source,
        position.line,
        position.character.get(),
        &analysis,
        Some(&registry),
    )
    .expect("actual current version has its own source read");
    assert!(
        selected.value.contains("**Inferred intrep**: string"),
        "{}",
        selected.value
    );
    assert!(
        !selected.value.contains("**Inferred intrep**: int"),
        "{}",
        selected.value
    );
}

#[test]
fn hover_on_pure_literal_surfaces_first_used_as() {
    // `l` is a pure list-shaped literal whose only typed read commits the
    // list intrep — the def-site pushback of the committed-intrep
    // dataflow surfaces "first used as: list" at the creation site.
    let src = "set l {1 2 3}\nllength $l\n";
    let analysis = analyse(src);
    let registry = tcl_registry::CommandRegistry::build_default();
    let h = hover(src, 1, 9, &analysis, Some(&registry)).expect("hover");
    assert!(h.value.contains("**Variable** `l`"), "{}", h.value);
    assert!(
        h.value
            .contains("**Inferred intrep**: string (first used as: list)"),
        "{}",
        h.value
    );
}

#[test]
fn hover_committed_producer_has_no_first_used_as() {
    // A committed producer (`[list …]`) is not "first used as" anything —
    // its intrep is set at the def, so the pushback annotation must not
    // appear.
    let src = "set l [list 1 2 3]\nllength $l\n";
    let analysis = analyse(src);
    let registry = tcl_registry::CommandRegistry::build_default();
    let h = hover(src, 1, 9, &analysis, Some(&registry)).expect("hover");
    assert!(
        !h.value.contains("first used as"),
        "committed producers must not carry the pushback label: {}",
        h.value
    );
}

#[test]
fn var_hover_text_renders_reference_count() {
    let src = "set x 1\nset y $x\nset z $x\n";
    let analysis = analyse(src);
    let var_def = analysis
        .global_scope
        .variables
        .get("x")
        .expect("x recorded");
    let text = var_hover_text(var_def, None, None, None);
    assert!(text.contains("**Variable** `x`"), "{}", text);
    assert!(text.contains("reference"), "{}", text);
}

#[test]
fn var_hover_text_appends_intrep_and_taint() {
    let var_def = VarDef {
        name: "x".to_owned(),
        definition_span: tcl_lexer::Span::new(0, 1),
        references: Vec::new(),
        warn_if_unused: false,
        array_indices: std::collections::BTreeSet::new(),
        link_target: None,
        link_target_span: None,
    };
    let text = var_hover_text(&var_def, Some("int"), Some("tainted (from I/O)"), None);
    assert!(text.contains("**Inferred intrep**: int"), "{text}");
    assert!(text.contains("**Taint**: tainted (from I/O)"), "{text}");
}

#[test]
fn var_hover_text_appends_proven_format_info() {
    let var_def = VarDef {
        name: "fmt".to_owned(),
        definition_span: tcl_lexer::Span::new(0, 1),
        references: Vec::new(),
        warn_if_unused: false,
        array_indices: std::collections::BTreeSet::new(),
        link_target: None,
        link_target_span: None,
    };
    let text = var_hover_text(&var_def, None, None, Some("**Format string**"));
    assert!(text.contains("**Variable** `fmt`"), "{text}");
    assert!(text.contains("**Format string**"), "{text}");
}

#[test]
fn special_var_hover_documents_auto_path() {
    // `$auto_path` has no user definition, but the special-variable
    // registry provides documentation on hover.
    let src = "puts $auto_path\n";
    let analysis = analyse(src);
    let registry = tcl_registry::CommandRegistry::build_default();
    let h = hover(src, 0, 8, &analysis, Some(&registry)).expect("hover");
    assert!(h.value.contains("special variable"), "{}", h.value);
    assert!(h.value.contains("auto-loader"), "{}", h.value);
}

#[test]
fn special_var_hover_is_dialect_aware() {
    let analysis = analyse_for("puts $auto_path\n", "f5-irules");
    let mut registry = CommandRegistry::build_default();
    registry.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    // iRules provides no `auto_path`, so no special-var hover fires there.
    assert!(
        hover_with_profile(
            "puts $auto_path\n",
            0,
            8,
            &analysis,
            Some(&registry),
            tcl_dialect::DialectProfile::irules(),
        )
        .is_none()
    );

    // `tcl_platform` exists under iRules and is CMP-unsafe as a plain
    // global — the hover surfaces the `static::` guidance.
    let src = "set x $tcl_platform(os)\n";
    let analysis = analyse_for(src, "f5-irules");
    let h = hover_with_profile(
        src,
        0,
        10,
        &analysis,
        Some(&registry),
        tcl_dialect::DialectProfile::irules(),
    )
    .expect("hover");
    assert!(h.value.contains("special array"), "{}", h.value);
    assert!(h.value.contains("CMP"), "{}", h.value);
    assert!(h.value.contains("static::tcl_platform"), "{}", h.value);
}
