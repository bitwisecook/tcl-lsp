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

//! Procedure cards: signatures, namespace-aware resolution, ensemble subcommands, docstrings and command heads.

use super::*;

#[test]
fn hover_on_proc_name_returns_signature() {
    let src = "proc greet {name} { puts $name }\n";
    let analysis = analyse(src);
    let h = hover(src, 0, 6, &analysis, None).expect("expected hover for proc name");
    assert_eq!(h.kind, HoverKind::Markdown);
    assert!(h.value.contains("proc ::greet"), "{}", h.value);
    assert!(h.value.contains("name"), "{}", h.value);
}

#[test]
fn hover_on_ensemble_subcommand_resolves_target_proc() {
    // TP — hover on an ensemble subcommand call
    // site surfaces the resolved target proc's own signature.
    let src = "namespace eval ::e {\n    namespace ensemble create -map {\n        foo ::e::Foo\n    }\n}\nproc ::e::Foo {args} { return \"foo: $args\" }\n\nputs [e foo bar]\n";
    let analysis = analyse(src);
    // Cursor on "foo" in `puts [e foo bar]` (0-based line 7, col 8).
    let h = hover(src, 7, 8, &analysis, None).expect("expected hover for ensemble subcommand");
    assert_eq!(h.kind, HoverKind::Markdown);
    assert!(h.value.contains("Ensemble subcommand"), "{}", h.value);
    assert!(h.value.contains("proc ::e::Foo"), "{}", h.value);
}

#[test]
fn hover_on_tk_ensemble_configure_splice_resolves_the_real_target() {
    // TP — `tk`'s built-in ensemble is extended at
    // runtime via `namespace ensemble configure tk -map [dict merge
    // [namespace ensemble configure tk -map] {systray ::tk::systray}]`
    // (the real `tk/library/systray.tcl` idiom). Hover on the call
    // site's "systray" must surface the real `::tk::systray` proc, not
    // abstain or resolve a same-tail-name decoy elsewhere.
    let src = "namespace eval ::decoy {\n    proc systray {args} { return \"DECOY\" }\n}\nproc ::tk::systray {args} { return \"real systray: $args\" }\nnamespace ensemble configure tk -map [dict merge [namespace ensemble configure tk -map] {systray ::tk::systray}]\ntk systray create -image book\n";
    let analysis = analyse(src);
    // Cursor on "systray" in `tk systray create ...` (0-based line 5).
    let h = hover(src, 5, 5, &analysis, None).expect("expected hover for tk systray splice");
    assert_eq!(h.kind, HoverKind::Markdown);
    assert!(h.value.contains("proc ::tk::systray"), "{}", h.value);
    assert!(!h.value.contains("DECOY"), "{}", h.value);
}

#[test]
fn hover_on_proc_qualified_name() {
    let src = "namespace eval ::ns { proc helper {} { return } }\n";
    let analysis = analyse(src);
    // Cursor on `helper` token at column ~28
    let h = hover(src, 0, 28, &analysis, None);
    // Either matches via simple name or qualified name; the
    // contract is that hover surfaces the proc when present.
    let h = h.expect("qualified proc declaration should have hover");
    assert!(h.value.contains("helper"), "{}", h.value);
}

// namespace-aware proc resolution (C Tcl `Tcl_FindCommand` order)

#[test]
fn hover_unqualified_call_prefers_callers_namespace_proc() {
    // Two namespaces each define `helper`; hovering the unqualified call
    // inside ::b must surface ::b::helper — the current namespace
    // resolves before global, never a sibling namespace.
    let src = "namespace eval a {\n    proc helper {x} { return 1 }\n}\nnamespace eval b {\n    proc helper {y} { return 2 }\n    helper 5\n}\n";
    let analysis = analyse(src);
    let h = hover(src, 5, 6, &analysis, None).expect("hover on namespaced call");
    assert!(h.value.contains("proc ::b::helper {y}"), "{}", h.value);
}

#[test]
fn hover_namespace_proc_set_shadows_builtin_inside_namespace_only() {
    // A namespace proc named like a builtin: inside ::ns the proc wins;
    // at global scope only the builtin resolves — a proc in an unrelated
    // namespace must not hijack builtin hover.
    let src = "namespace eval ns {\n    proc set {key value} { return $value }\n    set x 1\n}\nset y 2\n";
    let analysis = analyse(src);
    let registry = tcl_registry::CommandRegistry::build_default();
    let inside = hover(src, 2, 5, &analysis, Some(&registry)).expect("hover inside ns");
    assert!(inside.value.contains("proc ::ns::set"), "{}", inside.value);
    let global = hover(src, 4, 1, &analysis, Some(&registry)).expect("hover at global");
    assert!(global.value.contains("built-in"), "{}", global.value);
    assert!(!global.value.contains("::ns::set"), "{}", global.value);
}

#[test]
fn hover_global_call_fallback_is_deterministic() {
    // No ::helper exists, so no candidate resolves; the lenient tail
    // fallback fires and must pick the lexicographically smallest
    // qualified name (::a::helper) on every repeat — never a
    // `HashMap`-iteration-order hijack.
    let src = "namespace eval z {\n    proc helper {a} { return 26 }\n}\nnamespace eval a {\n    proc helper {b} { return 1 }\n}\nhelper 1\n";
    let analysis = analyse(src);
    let registry = tcl_registry::CommandRegistry::build_default();
    let first = hover(src, 6, 2, &analysis, Some(&registry)).expect("hover on global call");
    assert!(first.value.contains("proc ::a::helper"), "{}", first.value);
    for attempt in 0..8 {
        let repeat = hover(src, 6, 2, &analysis, Some(&registry)).expect("hover on global call");
        assert_eq!(first, repeat, "attempt {attempt}: hover must be stable");
    }
}

#[test]
fn proc_hover_text_formats_default_param() {
    let src = "proc greet {{name world}} { puts $name }\n";
    let analysis = analyse(src);
    let proc_def = analysis.all_procs.values().next().unwrap();
    let text = proc_hover_text(proc_def);
    assert!(text.contains("{name world}"), "got: {text}");
}

// docstring formatting

#[test]
fn format_docstring_renders_brief_param_return_tags() {
    let raw = concat!(
        "@brief Greet someone\n",
        "Free-form description\n",
        "spanning two lines.\n",
        "@param name the person's name\n",
        "@param greeting optional greeting prefix\n",
        "@return the formatted greeting\n",
    );
    let rendered = format_docstring(raw);
    assert!(rendered.contains("Greet someone"), "{rendered}");
    assert!(rendered.contains("Free-form description"), "{rendered}");
    assert!(rendered.contains("**Parameters:**"), "{rendered}");
    assert!(
        rendered.contains("- **name** \u{2014} the person's name"),
        "{rendered}",
    );
    assert!(
        rendered.contains("- **greeting** \u{2014} optional greeting prefix"),
        "{rendered}",
    );
    assert!(
        rendered.contains("**Returns:** the formatted greeting"),
        "{rendered}",
    );
}

#[test]
fn format_docstring_drops_decoration_lines() {
    // Pure-decoration lines (`.....`, `-----`) shouldn't
    // pollute the description block.
    let raw = "..........\nA description.\n..........\n";
    let rendered = format_docstring(raw);
    assert_eq!(rendered, "A description.");
}

#[test]
fn format_docstring_passes_through_plain_text() {
    let raw = "Just a free-form description.\nNo tags here.\n";
    let rendered = format_docstring(raw);
    assert!(rendered.contains("Just a free-form description"));
    assert!(rendered.contains("No tags here"));
}

#[test]
fn format_docstring_handles_param_without_description() {
    let raw = "@param naked\n";
    let rendered = format_docstring(raw);
    assert!(
        rendered.contains("- **naked**"),
        "expected bare param entry; got {rendered}",
    );
    // No trailing em-dash since there's no description.
    assert!(!rendered.contains("**naked** \u{2014}"), "{rendered}");
}

// tcl::OptProc — the `opt` package's automatic-option-parsing proc
// definer: hover on either the declaration or a
// call site must show the real `args`-only signature, never
// `optlist`'s own literal text.

#[test]
fn opt_proc_declaration_and_call_site_both_hover_the_real_args_only_signature() {
    let src = "::tcl::OptProc greet {child -use -display} { return $child }\ngreet foo\n";
    let analysis = analyse(src);
    // Line 0 — cursor on "greet" right after `::tcl::OptProc` (col 15).
    let decl = hover(src, 0, 15, &analysis, None).expect("hover on decl");
    assert!(
        decl.value.contains("greet") && decl.value.contains("args"),
        "{}",
        decl.value
    );
    // Line 1 — cursor on the `greet foo` call site (col 0).
    let call = hover(src, 1, 0, &analysis, None).expect("hover on call site");
    assert_eq!(decl.value, call.value, "decl and call site must agree");
}

// Hover shares `definition()`'s command-resolution seam, so it shares
// both of its gates — the command-position gate and the
// resolved-indirect-head preference.

#[test]
fn fp_argument_word_sharing_a_procs_name_does_not_hover_that_proc() {
    // FP guard — `dump` is `anotherproc`'s first argument, never a
    // command, so rendering `proc ::dump`'s signature there is a wrong
    // answer.
    let src = "proc anotherproc {a b} { return $a }\nproc dump {x} { return $x }\nproc caller {} {\n    return [anotherproc dump 5]\n}\n";
    let analysis = analyse(src);
    let h = hover(src, 3, 24, &analysis, None);
    assert!(
        h.is_none_or(|hv| !hv.value.contains("proc ::dump")),
        "an argument word must not hover as a command"
    );
}

#[test]
fn tp_the_command_head_beside_that_argument_still_hovers() {
    // TP — the gate must not cost the real head its hover.
    let src = "proc anotherproc {a b} { return $a }\nproc dump {x} { return $x }\nproc caller {} {\n    return [anotherproc dump 5]\n}\n";
    let analysis = analyse(src);
    let h = hover(src, 3, 12, &analysis, None).expect("hover on the call head");
    assert!(h.value.contains("proc ::anotherproc"), "{}", h.value);
}

#[test]
fn consumed_procedure_name_hover_uses_its_lookup_receipt() {
    let source = "proc target {} {return OK}\ninfo args target\nrename target moved\nmoved\n";
    let analysis = tcl_compiler::analyser::Analyser::new()
        .analyse(source, "tcl8.6")
        .clone();
    for (written, prefix) in [
        ("info args target", "info args "),
        ("rename target", "rename "),
    ] {
        let offset = u32::try_from(source.find(written).unwrap() + prefix.len()).unwrap();
        assert!(!crate::definition::offset_is_command_head(
            &analysis, offset
        ));
        let result = proc_hover_at(
            &analysis,
            source,
            offset,
            "target",
            crate::definition::CallResolution::document_only(),
        )
        .expect("hover on the consumed source procedure");
        assert!(result.value.contains("proc ::target"), "{}", result.value);
    }
}

#[test]
fn tp_resolved_indirect_head_hovers_the_command_it_reaches() {
    // TP — hover's half: the `${ns}::setdef` head carries
    // no written command name, so only the analyser's own resolution
    // can answer, and it must not lose to a same-named decoy.
    let src = "namespace eval ::tc { proc setdef {a} { return $a } }\nnamespace eval ::other { proc setdef {b c} { return $b } }\nset ns ::tc\n${ns}::setdef\n";
    let analysis = analyse(src);
    let h = hover(src, 3, 7, &analysis, None).expect("hover on the indirect head");
    assert!(
        h.value.contains("::tc::setdef"),
        "must describe ::tc::setdef, not the ::other decoy: {}",
        h.value
    );
}
