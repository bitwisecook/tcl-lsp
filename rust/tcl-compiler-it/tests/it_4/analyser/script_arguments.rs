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

//! Scripts passed as arguments: list-quoted scripts, `eval` and `uplevel` dispatch, lambdas, trace callbacks and `source` targets.

// A script argument *built* with `list` is walked as the command it
// provably is.
//
// tclsh 9.0.4 and 8.6.16 agree that the three spellings are functionally
// identical:
//
//   proc f {} {upvar #0 g l; set l 1}
//   proc f {} {uplevel #0 {upvar #0 g l}; …}
//   proc f {} {uplevel #0 [list upvar #0 g l]; …}
//
// `list` packs its already-substituted arguments into exactly one command,
// so the built form is not dynamic — yet every pass keyed on a literal
// `{…}` body skipped it.
mod list_quoted_script_arguments {
    use super::super::*;

    fn diag_codes(src: &str) -> Vec<String> {
        analyser_diags(src, D)
            .into_iter()
            .map(|(c, _, _)| c)
            .collect()
    }

    fn scope_named(
        scope: &tcl_compiler::analyser::Scope,
        kind: tcl_compiler::analyser::ScopeKind,
    ) -> Option<&tcl_compiler::analyser::Scope> {
        if scope.kind == kind {
            return Some(scope);
        }
        scope.children.iter().find_map(|c| scope_named(c, kind))
    }

    #[test]
    fn tp_a_list_built_uplevel_body_declares_in_the_uplevel_frame() {
        crate::install_surfaces();
        // tclsh8.6.18/9.0.4: `proc f {} {uplevel 1 [list set inner 1]};
        // proc h {} {f; return $inner}; h` -> 1.
        for src in [
            "proc f {} {\n    uplevel 1 [list set inner 1]\n}\nproc h {} {\n    f\n    return $inner\n}\nh\n",
            // The braced control — both must agree.
            "proc f {} {\n    uplevel 1 {set inner 1}\n}\nproc h {} {\n    f\n    return $inner\n}\nh\n",
        ] {
            let codes = diag_codes(src);
            assert!(!codes.iter().any(|c| c == "W210"), "{src:?}: {codes:?}");
        }
    }

    #[test]
    fn tp_a_list_built_namespace_body_declares_in_the_namespace() {
        crate::install_surfaces();
        // tclsh8.6.18/9.0.4: `namespace eval ::ns [list set v 1];
        // puts $::ns::v` -> 1.
        let src = "namespace eval ::ns [list set v 1]\nputs $::ns::v\n";
        let codes = diag_codes(src);
        assert!(!codes.iter().any(|c| c == "W210"), "{codes:?}");
    }

    #[test]
    fn fp_a_list_built_uplevel_body_does_not_claim_the_substitution_bytes() {
        crate::install_surfaces();
        let src = "proc p {file} {\n    uplevel #0 [list source [file join $file]]\n}\n";
        let r = Analyser::new().analyse(src, D);
        let proc_scope = &r.global_scope.children[0];
        let file_var = proc_scope.variables.get("file").expect("param");
        let read_off = u32::try_from(src.find("$file").unwrap()).unwrap();
        assert!(
            file_var.references.iter().any(|s| s.start() == read_off),
            "the $file read at {read_off} must attach to the proc param; refs {:?}",
            file_var.references
        );
        let up = scope_named(&r.global_scope, tcl_compiler::analyser::ScopeKind::Uplevel)
            .expect("uplevel scope opens");
        assert!(
            up.body_span.is_none(),
            "a list-built uplevel body must not claim the substitution bytes: {:?}",
            up.body_span
        );
    }

    #[test]
    fn fp_a_rewalked_built_body_emits_no_duplicate_diagnostics() {
        crate::install_surfaces();
        let src = "proc p {} {\n    namespace eval :: [list puts [string index abc 99]]\n}\n";
        let r = Analyser::new().analyse(src, D);
        let mut spans: Vec<_> = r
            .diagnostics
            .iter()
            .map(|d| (d.code.as_str().to_owned(), d.span.start(), d.span.end()))
            .collect();
        let n = spans.len();
        spans.sort();
        spans.dedup();
        eprintln!(
            "diags: {:?}",
            r.diagnostics
                .iter()
                .map(|d| (d.code.as_str(), d.span, d.message.clone()))
                .collect::<Vec<_>>()
        );
        assert_eq!(n, spans.len(), "duplicate diagnostics from the re-walk");
    }

    #[test]
    fn fp_a_nested_subst_in_a_built_body_is_not_rerecorded_in_the_target_scope() {
        crate::install_surfaces();
        let src = "namespace eval ::app {\n    proc helper {} { return 1 }\n    proc user {} { namespace eval :: [list puts [helper]] }\n}\n";
        let r = Analyser::new().analyse(src, D);
        let inv: Vec<_> = r
            .command_invocations
            .iter()
            .filter(|i| i.name.contains("helper"))
            .collect();
        eprintln!("helper invocations: {inv:?}");
        assert!(inv.len() <= 1, "double-recorded invocation: {inv:?}");
    }

    #[test]
    fn tn_a_dynamic_build_stays_an_opaque_barrier() {
        crate::install_surfaces();
        // `[list $cb inner 1]` names no statically known command, and
        // `[$build …]` is not a list build at all — both keep today's
        // behaviour rather than guessing.
        use tcl_compiler::analyser::ScopeKind;
        for src in [
            "proc f {} {\n    uplevel 1 [list $cb inner 1]\n}\n",
            "proc f {} {\n    uplevel 1 [$build inner]\n}\n",
        ] {
            let r = Analyser::new().analyse(src, D);
            assert!(
                scope_named(&r.global_scope, ScopeKind::Uplevel).is_none(),
                "an unresolvable body must not be walked: {src:?}"
            );
        }
    }

    #[test]
    fn tp_a_substituted_read_in_a_built_body_belongs_to_the_building_frame() {
        crate::install_surfaces();
        // `::tk::SourceLibFile`'s real shape. `$file` is the proc's own
        // parameter, substituted in the proc frame *before* `namespace eval`
        // enters `::` — so the read is the proc's, and the namespace scope
        // must not claim those bytes.
        let src = "proc ::tk::SourceLibFile {file} {\n    \
namespace eval :: [list source [file join $::tk_library $file.tcl]]\n\
}\n";
        let r = Analyser::new().analyse(src, D);
        let proc_scope = &r.global_scope.children[0];
        let file_var = &proc_scope.variables["file"];
        assert_eq!(
            file_var
                .references
                .iter()
                .map(|s| &src[s.start() as usize..s.end() as usize])
                .collect::<Vec<_>>(),
            vec!["$file"],
            "the parameter's read must be recorded"
        );
        let ns = proc_scope
            .children
            .iter()
            .find(|c| c.kind == tcl_compiler::analyser::ScopeKind::Namespace)
            .expect("the `namespace eval ::` scope exists");
        assert!(
            ns.body_span.is_none(),
            "a `[…]` body is evaluated in the calling frame, so the namespace \
scope must not own its bytes — owning them made the scope-chain lookup stop \
there and answer nothing for `$file`"
        );
    }

    #[test]
    fn tn_a_braced_namespace_body_still_owns_its_bytes() {
        crate::install_surfaces();
        // Control for the span above: a literal `{…}` body really does run in
        // the namespace frame, so it keeps its `body_span`.
        let src = "namespace eval ::ns {\n    variable v 1\n}\n";
        let r = Analyser::new().analyse(src, D);
        assert!(
            r.global_scope.children[0].body_span.is_some(),
            "a literal braced body is the namespace's own"
        );
    }
}

// Trace callbacks.
pub(super) mod trace_callbacks {
    use super::super::*;

    #[test]
    fn unrelated_proc_unused_param_still_flagged() {
        crate::install_surfaces();
        // Whatever the trace-callback handling, an *unrelated* proc's unused
        // param must still fire W214.
        let src = "proc watcher {name1 name2 op} { puts hello }\nproc other {a b} { puts $a }\ntrace add variable x write watcher\n";
        // Message shape: `Parameter 'b' of proc '::other' is unused`.
        let w214: Vec<_> = analyser_diags(src, D)
            .into_iter()
            .filter(|(c, m, _)| c == "W214" && m.contains("'b'") && m.contains("::other"))
            .collect();
        assert_eq!(
            w214.len(),
            1,
            "unrelated `other`'s unused `b` must fire W214"
        );
    }
}

// EvalUplevelIndirectDispatch — a bare `$var` body of an
// `ArgRole::Body`-marked argument (`eval $cmd`, `uplevel #0 $cmd …`)
// dynamically evaluates $var's value as a script at runtime, whose first
// word is the command actually dispatched. `command_invocations` must
// record it: hover/go-to-definition reach it via an independent
// cursor-token walk, but references/rename read `command_invocations`
// directly, so it must appear there too.
mod eval_uplevel_indirect_dispatch {
    use super::super::*;

    fn invocations(
        src: &str,
    ) -> Vec<tcl_compiler::signature_scan::types::SignatureCommandInvocation> {
        Analyser::new().analyse(src, D).command_invocations.clone()
    }

    #[test]
    fn eval_of_a_list_computed_var_resolves_the_real_head() {
        crate::install_surfaces();
        // TP — the finding's own minimal repro: `eval $cmdD` where `$cmdD`
        // is built via `[list greetD World]` — a computed value, not a
        // simple lexical literal the *lowering*-phase's simpler const-map
        // can fold (see `try_lower_eval_static`'s `const_map_lookup`), so
        // only the analyser's own flow-sensitive SSA-based
        // `settle_const_dispatches` can resolve it. Real tclsh9.0/8.6-
        // verified: `eval $cmdD` dispatches to `greetD World`.
        let src = "proc greetD {n} {puts \"D $n\"}\nset cmdD [list greetD World]\neval $cmdD\n";
        let invs = invocations(src);
        assert!(
            invs.iter()
                .any(|i| i.indirect && i.resolved_qualified_name.as_deref() == Some("::greetD")),
            "expected an indirect invocation resolving to ::greetD: {invs:?}"
        );
        // The `greetD` word inside `[list greetD World]` is also a
        // separate, *direct*, rename-writable invocation at its own span
        // (byte 45..51, `&src[45..51] == "greetD"`) — the actual command
        // name as written, not just the `$cmdD` dispatch anchor.
        assert!(
            invs.iter().any(|i| !i.indirect
                && i.rename_safe
                && i.range == tcl_lexer::Span::new(45, 51)
                && i.resolved_qualified_name.as_deref() == Some("::greetD")),
            "expected a direct, rename-safe invocation anchored on greetD's own word: {invs:?}"
        );
    }

    #[test]
    fn uplevel_hash_zero_of_a_var_headed_script_resolves_the_real_head() {
        crate::install_surfaces();
        // TP — the real `tk/library/tearoff.tcl` `MenuDup`-analogue shape:
        // `uplevel #0 $cmd [list $w $newMenu]`. `$cmd`'s own value is the
        // command name; the trailing `[list …]` argument supplies its
        // arguments (uplevel concatenates all its post-level arguments into
        // one script). tclsh9.0/8.6-verified: dispatches to `target`.
        let src =
            "proc target {a b} {puts \"$a-$b\"}\nset cmd target\nuplevel #0 $cmd [list x y]\n";
        let invs = invocations(src);
        assert!(
            invs.iter()
                .any(|i| i.indirect && i.resolved_qualified_name.as_deref() == Some("::target")),
            "expected an indirect invocation resolving to ::target: {invs:?}"
        );
    }

    #[test]
    fn plain_eval_of_a_braced_literal_body_is_unaffected() {
        crate::install_surfaces();
        // TN — control check: the ordinary literal-body `eval {…}` shape
        // (handled entirely by `analyse_body`'s existing `Str`-token path)
        // must keep resolving exactly as before; the new `Var`-token branch
        // is additive, not a replacement.
        let src = "proc target {} { return hi }\neval { target }\n";
        let invs = invocations(src);
        assert!(
            invs.iter()
                .any(|i| !i.indirect && i.resolved_qualified_name.as_deref() == Some("::target")),
            "expected a direct (non-indirect) invocation resolving to ::target: {invs:?}"
        );
    }

    #[test]
    fn genuinely_dynamic_eval_body_stays_unresolved() {
        crate::install_surfaces();
        // TN — regression guard: a value with no provable constant origin
        // (piped through `gets`) must still abstain — no invocation
        // recorded, matching the analyser's existing conservative default.
        let src = "proc target {} { return hi }\nset cmd [gets stdin]\neval $cmd\n";
        let invs = invocations(src);
        assert!(
            !invs
                .iter()
                .any(|i| i.resolved_qualified_name.as_deref() == Some("::target")),
            "a genuinely dynamic eval body must not resolve: {invs:?}"
        );
    }

    #[test]
    fn resolves_identically_inside_a_proc_body() {
        crate::install_surfaces();
        // TN (confirms a hypothesis in the finding's own root-cause hint is
        // moot): the auditor flagged "worth checking whether per-proc
        // FunctionUnits reach settle_one_site with usable SSA/def-use data
        // the same way the top-level unit does" as a second, compounding
        // gap. Empirically it isn't one — `function_unit_at`/
        // `const_contributors` have no top-level-vs-proc special-casing,
        // and this resolves identically to the top-level repro above.
        let src = "proc greetD {n} {puts \"D $n\"}\nproc caller {} {\n    set cmdD [list greetD World]\n    eval $cmdD\n}\n";
        let invs = invocations(src);
        assert!(
            invs.iter()
                .any(|i| i.indirect && i.resolved_qualified_name.as_deref() == Some("::greetD")),
            "expected an indirect invocation resolving to ::greetD even inside a proc body: {invs:?}"
        );
    }
}

// `apply [list {params} {body} ns]` — the list-constructor lambda idiom.
// Each list element must reach the body walk with its list delimiters
// removed, exactly as a literal braced lambda does.
mod apply_list_lambda {
    use super::super::*;

    #[test]
    fn apply_list_constructor_body_element_is_delimiter_stripped() {
        crate::install_surfaces();
        // `resolve_dynamic_apply_lambda`'s `[list …]` path must strip each
        // element's list delimiters (zipped from the segmenter's `texts`,
        // the same shape the literal-lambda path uses) rather than slicing
        // raw source spans — otherwise a `{frobnicate arg}` body element
        // keeps its braces, the body re-segments as a single braced word,
        // and the real `frobnicate` call is never seen. With delimiters
        // stripped, the body walks as `frobnicate arg` and records
        // `frobnicate` as its own command invocation.
        let invs = Analyser::new()
            .analyse("apply [list {} {frobnicate arg} ::myns]\n", D)
            .command_invocations;
        assert!(
            invs.iter().any(|i| i.name == "frobnicate"),
            "apply [list ...] body must segment to the real `frobnicate` call: {:?}",
            invs.iter().map(|i| &i.name).collect::<Vec<_>>()
        );
    }
}

// `source` command extraction.
pub(super) mod source_targets {
    use super::super::*;

    #[test]
    fn literal_source_recorded() {
        crate::install_surfaces();
        let r = Analyser::new().analyse("source lib/utils.tcl", D);
        assert_eq!(r.source_targets.len(), 1);
        assert_eq!(r.source_targets[0].raw_path, "lib/utils.tcl");
        assert!(r.source_targets[0].is_literal);
    }

    #[test]
    fn variable_and_cmd_subst_sources_not_literal() {
        crate::install_surfaces();
        let r = Analyser::new().analyse("source $dir/utils.tcl", D);
        assert_eq!(r.source_targets.len(), 1);
        assert!(!r.source_targets[0].is_literal);
        let r2 = Analyser::new().analyse(
            "source [file join [file dirname [info script]] helper.tcl]",
            D,
        );
        assert_eq!(r2.source_targets.len(), 1);
        assert!(!r2.source_targets[0].is_literal);
    }

    #[test]
    fn source_with_encoding_option() {
        crate::install_surfaces();
        let r = Analyser::new().analyse("source -encoding utf-8 myfile.tcl", D);
        assert_eq!(r.source_targets.len(), 1);
        assert_eq!(r.source_targets[0].raw_path, "myfile.tcl");
        assert!(r.source_targets[0].is_literal);
    }

    #[test]
    fn multiple_sources_mixed_literal() {
        crate::install_surfaces();
        let r = Analyser::new().analyse("source a.tcl\nsource b.tcl\nsource $c", D);
        assert_eq!(r.source_targets.len(), 3);
        assert!(r.source_targets[0].is_literal);
        assert!(r.source_targets[1].is_literal);
        assert!(!r.source_targets[2].is_literal);
    }
}
