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

//! `global`, `variable`, `namespace upvar` and `upvar` links.

use super::*;

#[test]
fn handle_global_defines_each_name() {
    let a = analysed("global x y z\n");
    for name in ["x", "y", "z"] {
        assert!(a.result.global_scope.variables.contains_key(name));
        assert!(!a.result.global_scope.variables[name].warn_if_unused);
        assert_eq!(
            a.result.global_scope.variables[name].link_target.as_deref(),
            Some(format!("::{name}").as_str()),
        );
    }
}

#[test]
fn global_binds_the_tail_of_a_qualified_name_and_skips_a_computed_one() {
    let src = "namespace eval ns {variable v 1}\nset dyn other\nglobal ::ns::v $dyn\n";
    let a = analysed(src);
    let v = &a.result.global_scope.variables["v"];
    assert_eq!(v.link_target.as_deref(), Some("::ns::v"));
    // The declaration word names the cell, so a rename rewrites it.
    let word = u32::try_from(src.find("::ns::v").unwrap()).unwrap();
    assert_eq!(v.link_target_span, Some(span(word, word + 7)));
    assert!(!a.result.global_scope.variables.contains_key("other"));
    assert!(!a.result.global_scope.variables.contains_key("$dyn"));
}

#[test]
fn handle_variable_defines_only_names_skipping_values() {
    // `variable x 1 y 2 z` — names at 0, 2, 4; values at 1, 3.
    let a = analysed("variable x 1 y 2 z\n");
    for name in ["x", "y", "z"] {
        assert!(a.result.global_scope.variables.contains_key(name));
    }
    // Numbers should NOT be variable names.
    assert!(!a.result.global_scope.variables.contains_key("1"));
    assert!(!a.result.global_scope.variables.contains_key("2"));
}

#[test]
fn handle_variable_single_name_no_value() {
    let a = analysed("variable x\n");
    assert!(a.result.global_scope.variables.contains_key("x"));
    // At the top level the current namespace is the global one.
    assert_eq!(
        a.result.global_scope.variables["x"].link_target.as_deref(),
        Some("::x"),
    );
}

#[test]
fn declaration_aliases_use_selected_native_name_owner() {
    for version in tcl_dialect::TclVersion::ALL {
        let mut analyser = Analyser::new();
        analyser.source_analysis_entry = Some(std::sync::Arc::new(
            crate::command_binding::SourceAnalysisEntry {
                invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(version)),
                ..Default::default()
            },
        ));
        analyser.handle_global_command(&["ns:::v".to_owned()], &[esc_tok(span(0, 6))], &[]);
        assert!(analyser.result.global_scope.variables.contains_key("v"));
        assert!(!analyser.result.global_scope.variables.contains_key(":v"));
        assert_eq!(
            analyser.result.global_scope.variables["v"]
                .link_target
                .as_deref(),
            Some("::ns::v"),
        );
    }
    let mut analyser = Analyser::new();
    analyser.source_analysis_entry = Some(std::sync::Arc::new(
        crate::command_binding::SourceAnalysisEntry {
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(
                tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
            )),
            ..Default::default()
        },
    ));
    analyser.handle_global_command(&["::ns::v".to_owned()], &[esc_tok(span(0, 7))], &[]);
    assert!(!analyser.result.global_scope.variables.contains_key("v"));
    analyser.handle_variable_command(&["::ns:::v".to_owned()], &[esc_tok(span(0, 8))], &[]);
    assert!(analyser.result.global_scope.variables.contains_key("v"));
}

// handle_upvar_command — the `otherVar` link

#[test]
fn namespace_upvar_links_each_local_to_its_namespace_cell() {
    // `namespace upvar ns other local` — `other` resolved within `ns`
    // and kept whole (`b::c` is `::a::b::c`, not the tail-collapsed
    // `::a::c`), `ns` relative to the current namespace; the rename span
    // is the `other` word, not the local.
    let src = "namespace eval a {namespace eval b {variable c 1}; variable d 2}\n\
                   namespace upvar ::a b::c l d m\n";
    let a = analysed(src);
    let l = &a.result.global_scope.variables["l"];
    assert_eq!(l.link_target.as_deref(), Some("::a::b::c"));
    let other = u32::try_from(src.find("b::c l").unwrap()).unwrap();
    assert_eq!(l.link_target_span, Some(span(other, other + 4)));
    assert_eq!(
        a.result.global_scope.variables["m"].link_target.as_deref(),
        Some("::a::d"),
    );
}

#[test]
fn namespace_upvar_with_a_computed_word_keeps_the_cell_as_written() {
    // A computed local binds nothing.
    let mut a = Analyser::new();
    dispatch_words(&mut a, &["namespace", "upvar", "::a", "x", "$local"]);
    assert!(!a.result.global_scope.variables.contains_key("local"));
    // Legacy alias reports preserve computed namespace/otherVar spellings.
    // Core candidates and rename coverage use typed source receipts instead.
    let mut a = Analyser::new();
    dispatch_words(&mut a, &["namespace", "upvar", "$ns", "x", "l"]);
    assert_eq!(
        a.result.global_scope.variables["l"].link_target.as_deref(),
        Some("::$ns::x"),
    );
    // The cell is named by the `otherVar` word, not the local.
    assert_eq!(
        a.result.global_scope.variables["l"].link_target_span,
        Some(span(12, 13)),
    );
    let mut a = Analyser::new();
    dispatch_words(&mut a, &["namespace", "upvar", "::mypkg", "$v", "l"]);
    assert_eq!(
        a.result.global_scope.variables["l"].link_target.as_deref(),
        Some("::mypkg::$v"),
    );
    // An absolute `otherVar` names its cell whatever the namespace word.
    let mut a = Analyser::new();
    dispatch_words(&mut a, &["namespace", "upvar", "$ns", "::abs", "l"]);
    assert_eq!(
        a.result.global_scope.variables["l"].link_target.as_deref(),
        Some("::abs"),
    );
}

#[test]
fn variable_links_keep_a_colon_named_namespace_and_collapse_written_runs() {
    // tclsh 8.6/9.0/9.1: with `set ::v glob; namespace eval : {variable v colon}`,
    // `proc a {} {namespace upvar : v l; namespace upvar ::: v m; global :::v; list $l $m $v}`
    // returns `colon glob glob`; `namespace which -variable v` in `:` is `:::::v`.
    // `upvar 1 :::x y; upvar #0 a:::b z` binds `::x` and `::a::b`.
    let a = analysed(
        "namespace upvar : v l\nnamespace upvar ::: v m\nglobal :::g\nupvar #0 :::x y\nupvar #0 a:::b z\n",
    );
    let variables = &a.result.global_scope.variables;
    let link = |name: &str| variables[name].link_target.clone();
    assert_eq!(link("l").as_deref(), Some(":::::v"));
    assert_eq!(link("m").as_deref(), Some("::v"));
    assert_eq!(link("g").as_deref(), Some("::g"));
    assert_eq!(link("y").as_deref(), Some("::x"));
    assert_eq!(link("z").as_deref(), Some("::a::b"));
}

// `upvar` — the `otherVar` link

/// The link target of `local` in the first procedure body of `src`,
/// analysed under Tcl 8.6.
fn proc_alias_link(src: &str, local: &str) -> Option<String> {
    let a = analysed(src);
    a.result
        .global_scope
        .children
        .iter()
        .find(|scope| scope.kind == crate::analyser::types::ScopeKind::Proc)
        .and_then(|scope| scope.variables.get(local))
        .and_then(|v| v.link_target.clone())
}

/// Dispatch `upvar <args>` and report the alias's link target, if it got
/// one.
fn upvar_link_of(args: &[&str], local: &str) -> Option<String> {
    let mut a = Analyser::new();
    let words: Vec<&str> = std::iter::once("upvar")
        .chain(args.iter().copied())
        .collect();
    dispatch_words(&mut a, &words);
    a.result
        .global_scope
        .variables
        .get(local)
        .and_then(|v| v.link_target.clone())
}

#[test]
fn upvar_links_a_fully_qualified_other_var_at_any_level() {
    // `upvar 1 ::myns::q g` binds `::myns::q` from any call depth
    // (tclsh 9.0.4 / 8.6.14: reached identically through `upvar 1` and
    // `upvar 2`), so the alias has a stable cell to link to.
    for src in [
        "namespace eval myns {variable q 1}\nproc p {} {upvar 1 ::myns::q g; return $g}\np\n",
        "namespace eval myns {variable q 1}\nproc p {} {upvar 2 ::myns::q g; return $g}\n\
             proc outer {} {p}\nouter\n",
    ] {
        assert_eq!(
            proc_alias_link(src, "g"),
            Some("::myns::q".to_string()),
            "{src}"
        );
    }
}

#[test]
fn upvar_links_an_array_other_var_to_its_base() {
    // tk.tcl:177 `upvar ::tk::FocusGrab($index) data` — the element key
    // is runtime data, the array is the named cell every sibling access
    // shares.
    assert_eq!(
        upvar_link_of(&["::tk::FocusGrab($index)", "data"], "data"),
        Some("::tk::FocusGrab".to_string()),
    );
}

#[test]
fn logical_upvar_targets_share_literal_scalar_and_element_geometry() {
    // naming.compiler.logical-upvar-target-element-geometry
    // docs/design/analysis/name-resolution-proofs/logical-upvar-target-element-geometry.md
    // These are readonly Logical source coordinates, not executed links.
    // Literal name parsing must preserve a scalar's unmatched parenthesis
    // and must not strip substitution sigils before the dynamic refusal.
    for (other, expected) in [
        ("a(b", Some("::a(b")),
        ("a(b)c", Some("::a(b)c")),
        ("a(b)", Some("::a")),
        ("π(β", Some("::π(β")),
        ("π(β)", Some("::π")),
        ("a($key)", Some("::a")),
        ("::ns::a(b", Some("::ns::a(b")),
        ("$computed", None),
    ] {
        let source = format!("upvar #0 {{{other}}} local");
        let analysis = Analyser::new().analyse(&source, "tcl");
        assert!(analysis.allows_retained_logical_declaration_advice());
        assert_eq!(
            analysis.global_scope.variables["local"]
                .link_target
                .as_deref(),
            expected,
            "{source}",
        );
    }
}

#[test]
fn logical_alias_source_declarations_keep_typed_targets_and_unknown_obligations() {
    let source = "upvar #0 shared local; puts $local";
    let analysis = Analyser::new().analyse(source, "tcl");
    assert_eq!(
        analysis.global_scope.variables["local"]
            .link_target
            .as_deref(),
        Some("::shared")
    );
    let span = tcl_lexer::Span::new(9, 15);
    let target = analysis
        .original_variable_symbols
        .iter()
        .find(|site| site.span() == span)
        .expect("actual original target occurrence");
    assert!(target.is_declaration());
    assert_eq!(target.original_name_input().bytes(), b"shared");
    assert_eq!(
        target.symbol().checked_c_namespace_key().as_deref(),
        Some("::shared")
    );
    assert_eq!(
        analysis.original_variable_alias_receipts,
        [] as [crate::signature_scan::variable_symbol::OriginalVariableAliasReceipt; 0]
    );
    assert_eq!(
        analysis.original_variable_alias_operands,
        [] as [(
            u32,
            tcl_lexer::Span,
            tcl_lexer::Span,
            crate::registry_invocation::DeclarationVariableAliasPurpose
        ); 0]
    );

    let source = "upvar #0 $unknown local";
    let analysis = Analyser::new().analyse(source, "tcl");
    assert!(
        analysis
            .global_scope
            .variables
            .get("local")
            .is_none_or(|variable| variable.link_target.is_none())
    );
    assert_ne!(
        analysis.original_variable_name_unknowns,
        [] as [tcl_lexer::Span; 0]
    );
    assert_eq!(
        analysis.original_variable_alias_receipts,
        [] as [crate::signature_scan::variable_symbol::OriginalVariableAliasReceipt; 0]
    );
}

#[test]
fn upvar_at_level_hash_zero_links_to_the_global_cell() {
    // tclsh 9.0.4 / 8.6.14: `upvar #0 counter c` reaches `::counter`
    // however deep the stack is.
    assert_eq!(
        proc_alias_link("proc p {} {upvar #0 counter c; set c 1}\np\n", "c"),
        Some("::counter".to_string()),
    );
}

#[test]
fn upvar_does_not_link_a_frame_relative_other_var() {
    // TN — `upvar 1 x y` names whatever local the *caller* happens to
    // have; there is no namespace path for it, so inventing one would
    // unify unrelated cells.
    assert_eq!(upvar_link_of(&["1", "x", "y"], "y"), None);
    assert_eq!(upvar_link_of(&["x", "y"], "y"), None);
    // A computed target has no readable name at all.
    assert_eq!(upvar_link_of(&["1", "$name", "obj"], "obj"), None);
    // A computed *level* could be `#0`, but could equally be `1` — the
    // alias must not be linked on a guess.
    assert_eq!(upvar_link_of(&["$lvl", "counter", "c"], "c"), None);
}

#[test]
fn upvar_level_word_presence_follows_argument_count_parity() {
    // `upvar 1 b` is TWO words, so there is no level word and `1` is the
    // caller-side variable name — the alias is `b` (tclsh 9.0.4 /
    // 8.6.14: with `set 1 ONE` in the caller, the callee reads `ONE`).
    let a = analysed("proc p {} {upvar 1 b; return $b}\nset 1 ONE\np\n");
    let body = &a.result.global_scope.children[0];
    assert!(body.variables.contains_key("b"));
    assert!(!body.variables.contains_key("1"));
}

#[test]
fn variable_global_upvar_skip_dynamic_names() {
    // A `variable` / `global` / `upvar` / `namespace upvar` whose name word
    // is computed (`$dyn` / `[f]`) is not a static declaration — the literal
    // substitution text must not be recorded as a variable.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
            "namespace eval ns {\n    variable $dyn\n    variable realvar 1\n}\nproc p {} {\n    global $g\n    upvar 1 other $loc\n}\n",
            "tcl8.6",
        );
    let ns = r
        .global_scope
        .children
        .iter()
        .find(|c| c.name == "ns")
        .expect("ns scope");
    assert!(
        ns.variables.contains_key("realvar"),
        "a static `variable` name is still recorded",
    );
    assert!(
        !ns.variables.contains_key("dyn"),
        "`variable $dyn` must not record `dyn`: {:?}",
        ns.variables.keys().collect::<Vec<_>>(),
    );
    let p = r
        .global_scope
        .children
        .iter()
        .find(|c| c.name == "p")
        .expect("p scope");
    assert!(
        !p.variables.contains_key("g") && !p.variables.contains_key("loc"),
        "dynamic `global`/`upvar` names must not be recorded: {:?}",
        p.variables.keys().collect::<Vec<_>>(),
    );
}
