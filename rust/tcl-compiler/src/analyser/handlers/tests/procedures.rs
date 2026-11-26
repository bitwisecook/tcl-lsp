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

//! `proc` declarations, the body walk, parameter traits and the global `unknown` handler.

use super::*;

/// A document that does not parse is walked from the recovery
/// segmenter's stream, which no interpretation reaches; a procedure in
/// the recovered tail is still declared. tclsh runs none of it (`missing
/// close-bracket`), so this is an outline fact, not an execution one.
#[test]
fn a_procedure_after_an_unterminated_bracket_is_still_declared() {
    for dialect in ["tcl8.6", "tcl"] {
        let mut analyser = Analyser::new();
        let result = analyser.analyse("set x [foo\nproc recovered {} {}\n", dialect);
        assert!(
            result.all_procs.contains_key("::recovered"),
            "{dialect}: {:?}",
            result.all_procs.keys().collect::<Vec<_>>()
        );
    }
}

// handle_proc_command

#[test]
fn handle_proc_command_records_every_declaration_site_even_when_shadowed() {
    // TP: a proc declared twice
    // verbatim at different spans (plain Tcl's own "last redefinition
    // wins" semantics) must still leave BOTH declarations' own name
    // spans in `proc_declaration_sites`, even though `all_procs`
    // (keyed by qualified name) only ever retains the winner's.
    let mut a = Analyser::new();
    a.handle_proc_command(
        "proc",
        &["foo".to_string(), String::new(), "return ONE".to_string()],
        &[
            esc_tok(span(5, 8)),
            esc_tok(span(9, 9)),
            str_tok(span(10, 20)),
        ],
        &[],
        &[],
    );
    a.handle_proc_command(
        "proc",
        &["foo".to_string(), String::new(), "return TWO".to_string()],
        &[
            esc_tok(span(30, 33)),
            esc_tok(span(34, 34)),
            str_tok(span(35, 45)),
        ],
        &[],
        &[],
    );
    assert_eq!(a.result.all_procs["::foo"].body_span, span(35, 45));
    let sites: Vec<(&str, tcl_lexer::Span)> = a
        .result
        .proc_declaration_sites
        .iter()
        .map(|(q, s)| (q.as_str(), *s))
        .collect();
    assert_eq!(
        sites,
        vec![("::foo", span(5, 8)), ("::foo", span(30, 33))],
        "both declaration sites must be recorded, in source order"
    );
}

#[test]
fn handle_proc_records_proc_at_global() {
    let mut a = Analyser::new();
    let handled = a.handle_proc_command(
        "proc",
        &["foo".to_string(), "a b".to_string(), "set x $a".to_string()],
        &[
            esc_tok(span(5, 8)),
            esc_tok(span(9, 14)),
            str_tok(span(15, 25)),
        ],
        &[],
        &[],
    );
    assert!(handled);
    assert!(a.result.all_procs.contains_key("::foo"));
    let proc = &a.result.all_procs["::foo"];
    assert_eq!(proc.name, "foo");
    assert_eq!(proc.qualified_name, "::foo");
    assert_eq!(proc.params.len(), 2);
    assert_eq!(proc.params[0].name, "a");
    assert_eq!(proc.params[1].name, "b");
}

#[test]
fn handle_proc_qualifies_under_namespace() {
    use crate::analyser::types::{Scope, ScopeKind};
    let mut a = Analyser::new();
    a.result
        .global_scope
        .children
        .push(Scope::new(ScopeKind::Namespace, "ns1"));
    let handled = a.handle_proc_command(
        "proc",
        &["foo".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 8)),
            str_tok(span(9, 11)),
            str_tok(span(12, 14)),
        ],
        &[],
        &[0],
    );
    assert!(handled);
    assert!(a.result.all_procs.contains_key("::ns1::foo"));
}

#[test]
fn handle_proc_keys_scope_procs_by_simple_name() {
    // ``scope.procs`` is keyed by the simple name so per-scope
    // lookups and shadowing rules work locally. The
    // qualified name lives on ``ProcDef.qualified_name`` and
    // is the key for ``result.all_procs``.
    use crate::analyser::types::{Scope, ScopeKind};
    let mut a = Analyser::new();
    a.result
        .global_scope
        .children
        .push(Scope::new(ScopeKind::Namespace, "ns1"));
    a.handle_proc_command(
        "proc",
        &["foo".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 8)),
            str_tok(span(9, 11)),
            str_tok(span(12, 14)),
        ],
        &[],
        &[0],
    );
    let scope = &a.result.global_scope.children[0];
    // Scope's procs map keyed by simple name, NOT qualified.
    assert!(
        scope.procs.contains_key("foo"),
        "scope.procs should be keyed by simple `foo`, got keys: {:?}",
        scope.procs.keys().collect::<Vec<_>>(),
    );
    assert!(
        !scope.procs.contains_key("::ns1::foo"),
        "scope.procs must not be keyed by qualified name",
    );
    // The qualified name is still on the ProcDef.
    assert_eq!(scope.procs["foo"].qualified_name, "::ns1::foo");
    // ...and result.all_procs is keyed by qualified name.
    assert!(a.result.all_procs.contains_key("::ns1::foo"));
}

#[test]
fn handle_proc_absolute_name_rebases() {
    use crate::analyser::types::{Scope, ScopeKind};
    let mut a = Analyser::new();
    a.result
        .global_scope
        .children
        .push(Scope::new(ScopeKind::Namespace, "outer"));
    let handled = a.handle_proc_command(
        "proc",
        &["::other::foo".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 17)),
            str_tok(span(18, 20)),
            str_tok(span(21, 23)),
        ],
        &[],
        &[0],
    );
    assert!(handled);
    // Absolute name rebases — does NOT nest under outer.
    assert!(a.result.all_procs.contains_key("::other::foo"));
    assert!(!a.result.all_procs.contains_key("::outer::other::foo"));
}

#[test]
fn handle_proc_consumes_last_comment_as_doc() {
    let mut a = Analyser::new();
    a.last_comment = "doc string".to_string();
    a.handle_proc_command(
        "proc",
        &["foo".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(0, 3)),
            str_tok(span(4, 6)),
            str_tok(span(7, 9)),
        ],
        &[],
        &[],
    );
    assert_eq!(a.result.all_procs["::foo"].doc, "doc string");
    // last_comment is consumed.
    assert_eq!(a.last_comment, "");
}

#[test]
fn handle_proc_too_few_args_returns_false() {
    let mut a = Analyser::new();
    let handled = a.handle_proc_command(
        "proc",
        &["foo".to_string()],
        &[esc_tok(span(0, 3))],
        &[],
        &[],
    );
    assert!(!handled);
    assert_eq!(a.result.all_procs.len(), 0);
}

// handle_proc_command body recursion

#[test]
fn handle_proc_creates_proc_scope_for_braced_body() {
    // ``proc foo {} {}`` — empty braced body still opens a
    // proc scope so subsequent body-walking handlers have a
    // place to record locals.
    let mut a = Analyser::new();
    a.handle_proc_command(
        "proc",
        &["foo".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 8)),
            str_tok(span(9, 11)),
            str_tok(span(12, 14)),
        ],
        &[],
        &[],
    );
    assert_eq!(a.result.global_scope.children.len(), 1);
    let proc_scope = &a.result.global_scope.children[0];
    assert_eq!(proc_scope.kind, crate::analyser::types::ScopeKind::Proc);
    assert_eq!(proc_scope.name, "foo");
    assert_eq!(proc_scope.body_span, Some(span(12, 14)));
}

#[test]
fn handle_proc_defines_params_in_proc_scope() {
    // ``proc foo {a b} {}`` — a, b become locals in the
    // proc scope, not in the outer scope.
    let mut a = Analyser::new();
    a.handle_proc_command(
        "proc",
        &["foo".to_string(), "a b".to_string(), String::new()],
        &[
            esc_tok(span(5, 8)),
            esc_tok(span(9, 14)),
            str_tok(span(15, 17)),
        ],
        &[],
        &[],
    );
    let proc_scope = &a.result.global_scope.children[0];
    assert!(proc_scope.variables.contains_key("a"));
    assert!(proc_scope.variables.contains_key("b"));
    // Outer scope must be untouched.
    assert!(!a.result.global_scope.variables.contains_key("a"));
}

#[test]
fn handle_proc_walks_body_set_defines_local() {
    // The body walk dispatches `set x 1` against the procedure scope, so
    // the local lands there and not in the global scope.
    let a = analysed("proc foo {} {set x 1}\n");
    let proc_scope = &a.result.global_scope.children[0];
    assert!(
        proc_scope.variables.contains_key("x"),
        "body walk should land 'x' in proc scope; vars: {:?}",
        proc_scope.variables.keys().collect::<Vec<_>>()
    );
    assert!(!a.result.global_scope.variables.contains_key("x"));
}

#[test]
fn handle_proc_walks_body_global_falls_into_proc_scope() {
    // `global a b` inside a procedure body binds both names in the
    // procedure scope, so the body's later reads and writes resolve.
    let a = analysed("proc foo {} {global a b}\n");
    let proc_scope = &a.result.global_scope.children[0];
    assert!(proc_scope.variables.contains_key("a"));
    assert!(proc_scope.variables.contains_key("b"));
}

#[test]
fn handle_proc_nested_proc_creates_nested_scopes() {
    // The body walk dispatches `proc` inside the body, creating a nested
    // procedure scope under the outer one.
    let a = analysed("proc outer {} {proc inner {} {}}\n");
    // Outer proc registered.
    assert!(a.result.all_procs.contains_key("::outer"));
    // A nested definition homes to the enclosing proc's *defining*
    // namespace (`command_resolution_namespace` /
    // `advance_command_resolution_namespace`), not to a namespace named
    // after the proc. ``outer`` is unqualified, so its defining namespace
    // is ``::`` and the nested ``inner`` qualifies as ``::inner``.
    assert!(a.result.all_procs.contains_key("::inner"));
    // Outer's proc scope holds the nested proc scope as a child.
    let outer_scope = &a.result.global_scope.children[0];
    assert_ne!(
        outer_scope.children,
        [] as [crate::analyser::types::Scope; 0]
    );
    assert_eq!(
        outer_scope.children[0].kind,
        crate::analyser::types::ScopeKind::Proc,
    );
    assert_eq!(outer_scope.children[0].name, "inner");
}

#[test]
fn handle_proc_dynamic_body_skips_walk() {
    // ``proc foo {} $body`` — the body is a Var token, not a
    // Str token; we cannot statically re-segment a dynamic
    // body, so the body walk is skipped. The proc record
    // itself still lands so downstream signature consumers see
    // the proc; only the inner walk is gated.
    let mut a = Analyser::new();
    let var_tok = Token::new(TokenType::Var, span(13, 18));
    a.handle_proc_command(
        "proc",
        &["foo".to_string(), String::new(), "$body".to_string()],
        &[esc_tok(span(5, 8)), str_tok(span(9, 11)), var_tok],
        &[],
        &[],
    );
    assert!(a.result.all_procs.contains_key("::foo"));
    // No proc scope opened — Str gate failed.
    assert_eq!(
        a.result.global_scope.children,
        [] as [crate::analyser::types::Scope; 0]
    );
}

#[test]
fn handle_proc_body_walk_increments_body_depth_temporarily() {
    // ``body_depth`` is bumped for the duration of the body
    // walk and restored on exit — top-level-only command
    // checks rely on the depth being zero outside any body.
    let mut a = Analyser::new();
    assert_eq!(a.body_depth, 0);
    a.handle_proc_command(
        "proc",
        &["foo".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 8)),
            str_tok(span(9, 11)),
            str_tok(span(12, 14)),
        ],
        &[],
        &[],
    );
    assert_eq!(a.body_depth, 0);
}

// Dynamic proc names + the foreach rename-and-reinstall idiom:
// `tk/library/accessibility.tcl`'s `foreach wtype {...} {
// rename ::$wtype ::tk::accessible::orig_$wtype ; proc ::$wtype {args}
// {...} }` renames each classic widget command away and reinstalls a
// wrapper under the same original name.

#[test]
fn handle_proc_command_dynamic_name_resolves_via_constant_fold() {
    // TP — no loop at all: a plain `set` constant. `proc ::$wtype {...}` must
    // constant-fold its name, as `rename` does, rather than register under
    // the literal garbled text.
    let mut a = Analyser::new();
    let src = "set wtype button\nproc ::$wtype {} {return ok}\n";
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.all_procs.contains_key("::button"),
        "{:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
    assert!(
        !r.all_procs.keys().any(|k| k.contains('$')),
        "{:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
}

#[test]
fn handle_proc_command_unresolvable_dynamic_name_keeps_raw_text() {
    // TN — a genuinely dynamic name (no constant to fold against) keeps the
    // raw-text fallback behaviour; only the resolvable case folds.
    let mut a = Analyser::new();
    let src = "proc ::$wtype {} {return ok}\n";
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.all_procs.keys().any(|k| k.contains("wtype")),
        "{:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
}

#[test]
fn analyse_unknown_proc_records_dispatch_targets_end_to_end() {
    // End-to-end: a ``proc unknown {cmd args} {...}`` with
    // an exact-match switch should populate
    // ``result.unknown_proc_info`` with the arm labels as
    // dispatch targets.  This is what gates W123.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "proc unknown {cmd args} { switch -exact $cmd { foo { return 1 } bar { return 2 } } }",
        "tcl",
    );
    let info = r.unknown_proc_info.expect("unknown_proc_info populated");
    assert!(!info.empty_stub);
    assert!(info.dispatch_targets.contains("foo"));
    assert!(info.dispatch_targets.contains("bar"));
}

#[test]
fn analyse_without_unknown_proc_leaves_unknown_proc_info_none() {
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("proc foo {} { return 1 }", "tcl");
    assert!(r.unknown_proc_info.is_none());
}

#[test]
fn analyse_unknown_proc_with_empty_body_marks_empty_stub() {
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("proc unknown {cmd args} {}", "tcl");
    let info = r.unknown_proc_info.expect("unknown_proc_info populated");
    assert!(info.empty_stub);
}

/// A `proc unknown` nested inside `namespace eval`
/// is an ordinary namespace proc, not the interpreter's handler, so it
/// must not seed `unknown_proc_info` and suppress W123 file-wide.
///
/// The body is the dynamic (`exec`-dispatching) shape that *would*
/// suppress W123 file-wide if this were the global handler, so the test
/// isolates the scope question rather than the body-shape one.
///
/// Oracle (tclsh8.6.14 and tclsh9.0.4): with
/// `namespace eval ::mylib { proc unknown {args} { return handled } }`,
/// calling `totallyBogusCommand` still fails
/// `invalid command name "totallyBogusCommand"` — only a *global*
/// `proc unknown` makes it return `handled`.
#[test]
fn analyse_namespace_local_unknown_proc_does_not_seed_the_global_handler() {
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "namespace eval ::mylib { proc unknown {cmd args} { exec $cmd {*}$args } }\n\
             totallyBogusCommand\n",
        "tcl9.0",
    );
    assert!(
        r.unknown_proc_info.is_none(),
        "a namespace-local proc named unknown is not the global handler",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W123),
        "so the bogus command is still unresolved; got {:?}",
        r.diagnostics,
    );
}

/// TP guard — the global handler still seeds the info (and still
/// suppresses W123), which is the behaviour the C2 fix must not disturb.
#[test]
fn analyse_global_unknown_proc_still_seeds_the_handler() {
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "proc unknown {cmd args} { exec $cmd {*}$args }\ntotallyBogusCommand\n",
        "tcl9.0",
    );
    assert!(
        r.unknown_proc_info.is_some(),
        "a global proc unknown is the handler",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W123),
        "and it suppresses the unresolved-command warning; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_qualified_unknown_proc_also_populates_info() {
    // ``::tcl::unknown`` (the canonical fully-qualified
    // name) should trigger detection too.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("proc ::tcl::unknown {cmd args} { exec $cmd }", "tcl");
    let info = r.unknown_proc_info.expect("unknown_proc_info populated");
    assert!(info.has_exec);
}

#[test]
fn analyse_w123_suppressed_when_package_require_seen() {
    // W123 is suppressed when any package require is on
    // file — package may load arbitrary commands.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("package require Foo\nbogus_command arg", "tcl");
    assert!(!r.diagnostics.iter().any(|d| d.code == DiagCode::W123));
}

#[test]
fn analyse_w123_suppressed_when_unknown_proc_chains_original() {
    // ``proc unknown`` that chains the original handler is
    // a *dynamic* shape — W123 is suppressed entirely
    // because runtime can resolve any command name.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "proc unknown {cmd args} { _original_unknown $cmd {*}$args }\nbogus_command arg",
        "tcl",
    );
    assert!(!r.diagnostics.iter().any(|d| d.code == DiagCode::W123));
}

#[test]
fn analyse_w123_suppressed_for_every_chain_spelling_of_the_handler() {
    // `chains_original` is the registry's
    // `Traits::UNRESOLVED_COMMAND_HANDLER`, so every spelling of the
    // handler counts.  The old `CHAIN_TARGETS` list named the `tcl::`
    // pair by hand and omitted `unknown` / `::unknown` entirely, so a
    // handler chaining through the plain spelling read as
    // self-contained and every unresolved command in the file collected
    // a spurious W123.
    for chain in ["unknown", "::unknown", "::tcl::unknown", "_orig_unknown"] {
        let mut a = crate::analyser::Analyser::new();
        let src = format!("proc unknown {{cmd args}} {{ {chain} $cmd {{*}}$args }}\nbogus_cmd arg");
        let r = a.analyse(&src, "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
            "chaining through {chain} should suppress W123",
        );
    }
}

#[test]
fn analyse_w123_suppressed_when_unknown_proc_calls_exec() {
    // ``exec $cmd`` inside ``unknown`` is a dynamic shape;
    // any command may be a real binary on PATH.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "proc unknown {cmd args} { exec $cmd {*}$args }\nbogus_command arg",
        "tcl",
    );
    assert!(!r.diagnostics.iter().any(|d| d.code == DiagCode::W123));
}

#[test]
fn analyse_w123_quiet_outside_explicit_dispatch_targets() {
    // A global ``proc unknown`` receives every unresolved word, so
    // ``bogus_command`` runs it and returns normally (tclsh 8.4.20, 8.6
    // and 9.0: `catch` gives 0): nothing is proved unresolved.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "proc unknown {cmd args} { switch -exact $cmd { foo { return 1 } } }\nbogus_command arg",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "the global unknown handler runs ``bogus_command``; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w123_suppressed_for_explicit_dispatch_target() {
    // ``foo`` is in the explicit dispatch_targets — even
    // for the non-dynamic shape, the per-invocation loop
    // suppresses W123 for it.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "proc unknown {cmd args} { switch -exact $cmd { foo { return 1 } } }\nfoo arg",
        "tcl",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W123 && d.message.contains("'foo'")),
        "W123 should not fire for command listed in dispatch_targets; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w123_quiet_for_empty_unknown_stub() {
    // An empty global ``unknown`` still receives the call, which then
    // returns an empty result (tclsh 8.4.20, 8.6 and 9.0).
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("proc unknown {cmd args} {}\nbogus_command arg", "tcl8.6");
    assert!(!r.diagnostics.iter().any(|d| d.code == DiagCode::W123));
}

// deep_param_traits plumbing
//
// These tests pin the contract that flipping
// `Analyser::deep_param_traits` actually changes the
// `ProcDef.param_traits` surface: the shallow pass alone
// misses traits hidden inside braced bodies, the deep pass
// catches them, and the union of both is what the analyser
// exposes.  The call-graph / symbol-graph / dataflow-graph /
// semantic-graph builders flip this on.

#[test]
fn analyse_with_deep_param_traits_surfaces_nested_eval() {
    // `$body` is buried inside a `foreach` body — the
    // shallow pass walks only top-level commands and misses
    // it.  Flipping `deep_param_traits` on must surface
    // `Eval`.
    let source = "proc f {items body} {\n  foreach item $items {\n    uplevel 1 $body\n  }\n}";
    let mut shallow = crate::analyser::Analyser::new();
    let shallow_r = shallow.analyse(source, "tcl");
    let shallow_proc = shallow_r.all_procs.get("::f").expect("::f proc registered");
    let shallow_body_traits = shallow_proc.param_traits.get("body");
    assert!(
        !shallow_body_traits.is_some_and(|s| s.contains(&crate::analyser::ProcArgTrait::Eval)),
        "shallow pass should miss nested Eval, got {shallow_body_traits:?}",
    );

    let mut deep = crate::analyser::Analyser::new();
    deep.deep_param_traits = true;
    let deep_r = deep.analyse(source, "tcl");
    let deep_proc = deep_r.all_procs.get("::f").expect("::f proc registered");
    let deep_body_traits = deep_proc
        .param_traits
        .get("body")
        .expect("body traits present with deep_param_traits on");
    assert!(
        deep_body_traits.contains(&crate::analyser::ProcArgTrait::Eval),
        "deep pass should surface nested Eval, got {deep_body_traits:?}",
    );
}

#[test]
fn deep_lambda_traits_follow_selected_original_words_and_document_offsets() {
    for prefix in ["", "# λ shifts the original byte offsets\n", "\u{feff}"] {
        let source = format!("{prefix}proc f {{body}} {{apply {{x {{eval $x}}}} $body}}");
        let mut analyser = crate::analyser::Analyser::new();
        analyser.deep_param_traits = true;
        let result = analyser.analyse(&source, "tcl9.0");
        let traits = result
            .all_procs
            .get("::f")
            .and_then(|procedure| procedure.param_traits.get("body"));
        assert!(
            traits.is_some_and(|traits| traits.contains(&crate::analyser::ProcArgTrait::Eval)),
            "selected original lambda source should carry Eval through its actual argument; prefix={prefix:?}, got {traits:?}",
        );
    }
}

#[test]
fn analyse_with_deep_param_traits_off_matches_shallow() {
    // For procs without nested-body usage, deep + shallow
    // produce the same trait map.  This pins that the deep
    // pass doesn't accidentally lose any shallow trait.
    let source = "proc g {body} { uplevel 1 $body }";
    let mut shallow = crate::analyser::Analyser::new();
    let shallow_r = shallow.analyse(source, "tcl");

    let mut deep = crate::analyser::Analyser::new();
    deep.deep_param_traits = true;
    let deep_r = deep.analyse(source, "tcl");

    let shallow_traits = shallow_r
        .all_procs
        .get("::g")
        .expect("::g registered")
        .param_traits
        .clone();
    let deep_traits = deep_r
        .all_procs
        .get("::g")
        .expect("::g registered")
        .param_traits
        .clone();
    assert_eq!(
        shallow_traits, deep_traits,
        "deep + shallow should match for top-level-only bodies",
    );
}

#[test]
fn nested_def_in_qualified_encloser_does_not_overwrite_global() {
    // `proc a::outer { proc helper ... }`: the nested `helper` homes to the
    // encloser's *defining* namespace (`::a::helper`), not the lexical
    // global — so the real global `proc helper` is preserved rather than
    // overwritten in `all_procs`.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
            "proc helper {} { puts global }\nnamespace eval a {}\nproc a::outer {} {\n    proc helper {} { puts nested }\n}\n",
            "tcl8.6",
        );
    assert!(
        r.all_procs.contains_key("::a::helper"),
        "nested helper must home to ::a::helper: {:?}",
        r.all_procs.keys().collect::<Vec<_>>(),
    );
    let global = r
        .all_procs
        .get("::helper")
        .expect("global ::helper preserved");
    assert_eq!(
        global.name_span.start(),
        5,
        "::helper must remain the global declaration (line 0), not the nested one",
    );
    // The same for a nested class.
    let mut b = crate::analyser::Analyser::new();
    let rc = b.analyse(
        "oo::class create Widget {}\nnamespace eval a {}\nproc a::outer {} {\n    oo::class create Widget {}\n}\n",
        "tcl8.6",
    );
    assert!(
        rc.all_classes.contains_key("::a::Widget"),
        "nested class homes to ::a::Widget: {:?}",
        rc.all_classes.keys().collect::<Vec<_>>(),
    );
    assert!(
        rc.all_classes.contains_key("::Widget"),
        "global ::Widget preserved",
    );
}

// stub-overlay end-to-end
//
// These tests pin the contract that the stub overlay built
// from `# tcl-lsp: stub` directives during `analyse()`
// propagates into the per-proc `param_traits` map.

#[test]
fn analyse_with_declared_stub_propagates_role_to_param_traits() {
    // The source declares a `# tcl-lsp: stub my_eval
    // {script:body}` directive, then defines a proc that
    // invokes `my_eval $body`.  The body arg's role flows
    // from the stub overlay → `param_traits["body"]
    // .contains(Body)`.
    let source = "\
# tcl-lsp: stubs-begin\n\
# tcl-lsp: stub my_eval {script:body}\n\
# tcl-lsp: stubs-end\n\
proc runs {body} {\n\
    my_eval $body\n\
}\n";
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(source, "tcl8.6");
    let proc = r.all_procs.get("::runs").expect("::runs registered");
    let body_traits = proc
        .param_traits
        .get("body")
        .expect("body param has traits");
    assert!(
        body_traits.contains(&crate::analyser::ProcArgTrait::Body),
        "expected Body trait via stub overlay, got {body_traits:?}",
    );
}

#[test]
fn analyse_without_stub_directive_leaves_body_untyped() {
    // Same proc, no stub directive — without the overlay
    // entry, `my_eval` isn't a known command, so the body
    // arg has no recorded role and `param_traits` is empty
    // for `body`.  This pins that the stub directive (not
    // some background heuristic) is what gives the
    // parameter its `Body` trait.
    let source = "proc runs {body} { my_eval $body }";
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(source, "tcl");
    let proc = r.all_procs.get("::runs").expect("::runs registered");
    assert!(
        !proc
            .param_traits
            .get("body")
            .is_some_and(|s| s.contains(&crate::analyser::ProcArgTrait::Body)),
        "expected NO Body trait without stub directive, got {:?}",
        proc.param_traits.get("body"),
    );
}

/// The declaration carries each formal's script contract, and the reported
/// traits keep their established vocabulary.
#[test]
fn a_procedure_records_the_frame_it_runs_a_script_parameter_in() {
    use crate::analyser::{ProcArgTrait, ScriptParameterFrame};
    let result = crate::analyser::Analyser::new().analyse(
        "proc t {label script} {uplevel 1 $script}\nproc g {s} {uplevel #0 $s}\n",
        "tcl8.6",
    );
    let t = result.all_procs.get("::t").expect("::t");
    assert_eq!(
        t.script_parameters.get("script").map(|c| c.frame),
        Some(ScriptParameterFrame::Caller(1))
    );
    assert!(!t.script_parameters.contains_key("label"));
    assert!(
        !t.param_traits["script"]
            .iter()
            .any(|item| matches!(item, ProcArgTrait::Script(_)))
    );
    let g = result.all_procs.get("::g").expect("::g");
    assert_eq!(
        g.script_parameters.get("s").map(|c| c.frame),
        Some(ScriptParameterFrame::Global)
    );
}
