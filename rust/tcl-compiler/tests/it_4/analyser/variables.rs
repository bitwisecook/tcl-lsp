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

//! Variable definitions and reads, and the constant values the analyser resolves through them.

// Variable definitions across scopes.
mod variable_analysis {
    use super::super::*;

    #[test]
    fn set_defines_var() {
        let r = Analyser::new().analyse("set x 42", D);
        assert!(r.global_scope.variables.contains_key("x"));
    }

    #[test]
    fn multiple_sets_define_each() {
        let r = Analyser::new().analyse("set x 1\nset y 2\nset z 3", D);
        for v in ["x", "y", "z"] {
            assert!(r.global_scope.variables.contains_key(v), "{v} missing");
        }
    }

    #[test]
    fn incr_defines_var() {
        let r = Analyser::new().analyse("incr counter", D);
        assert!(r.global_scope.variables.contains_key("counter"));
    }

    #[test]
    fn array_set_defines_base_name() {
        let r = Analyser::new().analyse("set arr(key) value", D);
        assert!(r.global_scope.variables.contains_key("arr"));
    }

    #[test]
    fn proc_local_not_in_global_scope() {
        let r = Analyser::new().analyse("proc foo {} { set local 1 }", D);
        assert!(!r.global_scope.variables.contains_key("local"));
        assert!(r.global_scope.children[0].variables.contains_key("local"));
    }

    #[test]
    fn set_one_arg_is_read_only() {
        // `set x` (1 arg) is a *read*, not a definition. C-Tcl: `set x` on an
        // undefined var errors `can't read "x"`, confirming it is a read.
        let r = Analyser::new().analyse("set x", D);
        assert!(!r.global_scope.variables.contains_key("x"));
    }

    #[test]
    fn variable_command_skips_value_word() {
        let r = Analyser::new().analyse("variable port 8080", D);
        assert!(r.global_scope.variables.contains_key("port"));
        assert!(!r.global_scope.variables.contains_key("8080"));
    }
}

// A registry `VarRead`-role name word is a reference site.
//
// A variable is read by more than `$name`. Any command whose spec puts an
// argument in `ArgRole::VarRead` reads the cell that word names, and tclsh
// agrees (9.0.4 and 8.6.16, byte-identical):
//
//   proc f {} {set m 1; puts [set m]};        f   -> 1
//   proc f {} {set m 1; puts [info exists m]}; f  -> 1
//   proc f {} {set m 1; return [set m]};      f   -> 1
//
// `VarDef::references` must include every one of these sites, not just
// `$m` and the *statement* form `set m` — otherwise Find References /
// document-highlight / the minifier's rename pass under-report them.
mod var_read_role_references {
    use super::super::*;

    /// The read spans recorded for `name` in the first proc scope, as the
    /// source text each one covers.
    fn read_texts(src: &str, name: &str) -> Vec<String> {
        fn walk(
            scope: &tcl_compiler::analyser::Scope,
            name: &str,
            src: &str,
        ) -> Option<Vec<String>> {
            if let Some(v) = scope.variables.get(name) {
                return Some(
                    v.references
                        .iter()
                        .map(|r| src[r.start() as usize..r.end() as usize].to_owned())
                        .collect(),
                );
            }
            scope.children.iter().find_map(|c| walk(c, name, src))
        }
        walk(&Analyser::new().analyse(src, D).global_scope, name, src).unwrap_or_default()
    }

    #[test]
    fn tp_a_substituted_set_read_is_recorded() {
        // FN before the fix: `refs` was empty.
        assert_eq!(
            read_texts("proc f {} {\n    set m 1\n    puts [set m]\n}\n", "m"),
            vec!["m".to_owned()],
            "`[set m]` is a read of `m`"
        );
    }

    #[test]
    fn tp_every_var_read_role_contributes_not_just_set() {
        // Registry-driven, so the fix is not about `set`: `info exists` and
        // `array get` declare `VarRead` positions too.
        for (src, name) in [
            ("proc f {} {\n    set m 1\n    info exists m\n}\n", "m"),
            (
                "proc f {} {\n    set m 1\n    puts [info exists m]\n}\n",
                "m",
            ),
            (
                "proc f {} {\n    array set a {k 1}\n    puts [array get a]\n}\n",
                "a",
            ),
        ] {
            assert_eq!(
                read_texts(src, name),
                vec![name.to_owned()],
                "a `VarRead`-role word must be a reference site: {src:?}"
            );
        }
    }

    #[test]
    fn tn_the_write_form_records_no_read() {
        // `set m 1` is a definition, not a read — the two-argument form has no
        // `VarRead` role at all, so nothing new appears.
        assert!(
            read_texts("proc f {} {\n    set m 1\n}\n", "m").is_empty(),
            "a plain write must not become its own reference"
        );
    }

    #[test]
    fn tn_a_computed_name_records_nothing() {
        // `set $n` reads whatever *value* `n` holds; which cell that is cannot
        // be known statically, so no read is attributed to `m`.
        assert!(
            read_texts(
                "proc f {} {\n    set m 1\n    set n m\n    puts [set $n]\n}\n",
                "m"
            )
            .is_empty(),
            "a `$`-computed name must not be credited to a same-spelled cell"
        );
    }

    #[test]
    fn fp_the_statement_form_is_recorded_exactly_once() {
        // `set m` is recorded by `set`'s own handler *and* by the generic
        // role pass; the sink dedupes by span, so the site appears once. A
        // duplicate would make hover's "N reference(s)" count read double.
        assert_eq!(
            read_texts("proc f {} {\n    set m 1\n    set m\n}\n", "m"),
            vec!["m".to_owned()],
            "one source location is one reference"
        );
    }

    #[test]
    fn tp_a_brace_quoted_name_word_reads_the_literal_cell() {
        // The cell read through the role path: `{$n}` names the
        // variable *called* `$n`. tclsh 9.0.4 / 8.6.16: `set {$n} v; set {$n}`
        // -> v, while `info exists n` -> 0.
        let src = "proc f {} {\n    set {$n} 1\n    puts [set {$n}]\n}\n";
        // The read's range follows the same convention as the `{$n}`
        // declaration's range, whatever that convention is.
        let r = Analyser::new().analyse(src, D);
        let cell = r.global_scope.children[0]
            .variables
            .get("$n")
            .expect("the literal cell `$n` is declared");
        let text = |start: u32, end: u32| src[start as usize..end as usize].to_owned();
        assert_eq!(
            cell.references
                .iter()
                .map(|r| text(r.start(), r.end()))
                .collect::<Vec<_>>(),
            vec![text(cell.definition_span.start(), cell.definition_span.end())],
            "the literal cell owns the read"
        );
        assert!(
            read_texts(src, "n").is_empty(),
            "the unrelated plain cell must own nothing"
        );
    }
}

// `set` dual-shape role resolution, observed through behaviour.
mod set_dual_shape {
    use super::super::*;

    #[test]
    fn set_two_arg_is_write_one_arg_is_read() {
        // 2-arg `set x 1` is a write: defines `x`, no W210.
        assert!(!fires("set x 1\nputs $x", D, "W210"));
        assert!(
            Analyser::new()
                .analyse("set x 1", D)
                .global_scope
                .variables
                .contains_key("x")
        );
        // 1-arg `set x` is a read: does NOT define `x`.
        assert!(
            !Analyser::new()
                .analyse("set x", D)
                .global_scope
                .variables
                .contains_key("x")
        );
    }
}

// Constant command-substitution `set` RHS folding.
//
// The analyser's constant lattice folds `set VAR [cmd …]` through the
// registry `const_fold` / frame-fact engine (`crate::analyser::const_subst`), so the
// `${ns}::setdef` navigation chain resolves when `ns` was assigned a
// constant command substitution rather than a bare literal.
//
// C-Tcl ground truth (tclsh 9.0.4, run while authoring): `namespace
// qualifiers ::tc::X` → `::tc`; inside an instance method of `::tc::Chart`
// the full chain really dispatches (`set ns [namespace qualifiers [self
// class]]; ${ns}::setdef x y` → `::tc::setdef` runs); inside a `self
// method`, `self class` raises `method not defined by a class`; inside a
// `classmethod` it answers the internal `::oo::ObjN:: oo ::delegate` class,
// never the written one.
mod const_cmd_subst_set_rhs {
    use super::super::*;
    use tcl_compiler::analyser::AnalysisResult;

    fn analysis(src: &str, dialect: &str) -> AnalysisResult {
        Analyser::new().analyse(src, dialect)
    }

    fn resolutions_of(r: &AnalysisResult, head_contains: &str) -> Vec<String> {
        r.command_invocations
            .iter()
            .filter(|inv| inv.name.contains(head_contains))
            .filter_map(|inv| inv.resolved_qualified_name.clone())
            .collect()
    }

    #[test]
    fn a_constant_namespace_qualifiers_rhs_folds_and_resolves_the_head() {
        // TP — the probe shape: zero OO involvement, a
        // plain proc, a constant `[namespace qualifiers …]` RHS.
        let src = concat!(
            "namespace eval tc { proc setdef {a b} { return 1 } }\n",
            "proc user {} {\n",
            "    set ns [namespace qualifiers ::tc::X]\n",
            "    ${ns}::setdef x y\n",
            "}\n",
        );
        let r = analysis(src, "tcl8.6");
        assert_eq!(
            resolutions_of(&r, "${ns}"),
            ["::tc::setdef"],
            "the folded constant must drive head resolution"
        );
    }

    #[test]
    fn a_vendor_profile_folds_a_version_sensitive_constant_only_where_its_release_was_measured() {
        // The version-sensitive `format %d 010` is 8 under Tcl 8.4 (a leading
        // zero is octal) and 10 under 9.0. iRules is an 8.4 fork whose release
        // was measured, so it folds as 8.4 does; `expect` models an 8.6 base that
        // nobody measured, so a fold under it stays unresolved rather than being
        // folded as one release's semantics.
        let src = concat!(
            "namespace eval tc { proc setdef {a b} { return 1 } }\n",
            "proc user {} {\n",
            "    set value [format %d 010]\n",
            "    ${value}::setdef x y\n",
            "}\n",
        );
        let measured = analysis(src, "f5-irules");
        assert_eq!(
            resolutions_of(&measured, "${value}"),
            ["::8::setdef"],
            "a measured vendor release folds as that release"
        );
        let unmeasured = analysis(src, "expect");
        assert_eq!(
            resolutions_of(&unmeasured, "${value}"),
            ["::${value}::setdef"],
            "a vendor base nothing measured must abstain"
        );
    }

    #[test]
    fn a_self_class_chain_folds_inside_an_instance_method() {
        // TP — the ticklecharts idiom one level removed: `set ns
        // [namespace qualifiers [self class]]` inside an instance method of
        // `::tc::Chart`. Every registry dependency is exact, so the
        // runtime-selected receiver namespace cannot change the fold.
        let src = concat!(
            "namespace eval tc { proc setdef {a b} { return 1 } }\n",
            "oo::class create ::tc::Chart {\n",
            "    method go {} {\n",
            "        ::set ns [::namespace qualifiers [::oo::Helpers::self class]]\n",
            "        ${ns}::setdef x y\n",
            "    }\n",
            "}\n",
        );
        let r = analysis(src, "tcl9.0");
        assert_eq!(
            resolutions_of(&r, "${ns}"),
            ["::tc::setdef"],
            "the frame-fact fold must feed the constant lattice"
        );
    }

    #[test]
    fn a_class_side_method_abstains_from_the_self_class_fold() {
        // FP guard: `self class` never
        // answers the written class in a class-side frame (tclsh 9.0.4:
        // raises in a `self method`; answers the internal delegate class
        // in a `classmethod`) — folding it would invent a value. The head
        // must stay unresolved-as-written.
        for member in ["classmethod go {}", "self method go {}"] {
            let src = format!(
                concat!(
                    "namespace eval tc {{ proc setdef {{a b}} {{ return 1 }} }}\n",
                    "oo::class create ::tc::Chart {{\n",
                    "    {} {{\n",
                    "        set ns [namespace qualifiers [self class]]\n",
                    "        ${{ns}}::setdef x y\n",
                    "    }}\n",
                    "}}\n",
                ),
                member
            );
            let r = analysis(&src, "tcl9.0");
            assert_eq!(
                resolutions_of(&r, "${ns}"),
                ["::${ns}::setdef"],
                "{member}: a class-side frame must not fold [self class]"
            );
        }
    }

    #[test]
    fn a_later_rename_of_the_folding_head_blocks_the_fold() {
        // FP guard: the trust oracle is
        // whole-module — a `rename` AFTER the `set`, buried inside a proc
        // body, still unbinds `namespace` from its builtin semantics
        // before some later call can run. The mid-walk `renamed_commands`
        // map cannot see it; the fold must still decline.
        let src = concat!(
            "namespace eval tc { proc setdef {a b} { return 1 } }\n",
            "proc user {} {\n",
            "    set ns [namespace qualifiers ::tc::X]\n",
            "    ${ns}::setdef x y\n",
            "}\n",
            "proc sabotage {} { rename namespace nsx }\n",
        );
        let r = analysis(src, "tcl8.6");
        assert_eq!(
            resolutions_of(&r, "${ns}"),
            ["::${ns}::setdef"],
            "a whole-module rename of the head must block the fold"
        );
    }

    #[test]
    fn a_shadowing_proc_definition_replaces_the_builtin_fold() {
        // A user `proc namespace …` shadows the builtin for every call made
        // after the file is loaded, so `[namespace qualifiers …]` is whatever
        // that procedure returns, never `::tc`. tclsh 8.4.20 and 9.0.4 raise
        // `invalid command name "::evil::setdef"` for `user` after this
        // source.
        let src = concat!(
            "namespace eval tc { proc setdef {a b} { return 1 } }\n",
            "proc user {} {\n",
            "    set ns [namespace qualifiers ::tc::X]\n",
            "    ${ns}::setdef x y\n",
            "}\n",
            "proc namespace {args} { return ::evil }\n",
        );
        let r = analysis(src, "tcl8.6");
        assert_eq!(
            resolutions_of(&r, "${ns}"),
            ["::evil::setdef"],
            "the head is built from what the shadowing procedure returns"
        );
    }

    #[test]
    fn a_branch_dependent_argument_abstains() {
        // TN — the substitution's `$q` argument has no dominating constant
        // (two branch values), so the fold must abstain rather than pick
        // the last-written branch.
        let src = concat!(
            "namespace eval a { proc go {} { return 1 } }\n",
            "namespace eval b { proc go {} { return 1 } }\n",
            "proc user {c} {\n",
            "    set q ::a::X\n",
            "    if {$c} { set q ::b::X }\n",
            "    set ns [namespace qualifiers $q]\n",
            "    ${ns}::go\n",
            "}\n",
        );
        let r = analysis(src, "tcl8.6");
        assert_eq!(
            resolutions_of(&r, "${ns}"),
            ["::${ns}::go"],
            "a branch-dependent argument must not fold"
        );
    }

    #[test]
    fn per_item_path_folds_and_abstains_identically_to_the_whole_file_walk() {
        // The per-item (incremental) path analyses each proc body in
        // ISOLATION — it cannot see a `rename` elsewhere in the file, so
        // the shell attaches a whole-file trust snapshot to every deferred
        // body with a fold candidate (`DeferredBody::command_trust`).
        // Both directions must match the whole-file walk byte-for-byte:
        // the TP still folds, the FP still abstains.
        let tp = concat!(
            "namespace eval tc { proc setdef {a b} { return 1 } }\n",
            "proc user {} {\n",
            "    set ns [namespace qualifiers ::tc::X]\n",
            "    ${ns}::setdef x y\n",
            "}\n",
        );
        let fp = concat!(
            "namespace eval tc { proc setdef {a b} { return 1 } }\n",
            "proc user {} {\n",
            "    set ns [namespace qualifiers ::tc::X]\n",
            "    ${ns}::setdef x y\n",
            "}\n",
            "proc sabotage {} { rename namespace nsx }\n",
        );
        for (src, expected) in [(tp, "::tc::setdef"), (fp, "::${ns}::setdef")] {
            let whole = analysis(src, "tcl8.6");
            let per_item = Analyser::new().analyse_per_item(src, "tcl8.6");
            assert_eq!(
                resolutions_of(&whole, "${ns}"),
                [expected],
                "whole-file walk for {src:?}"
            );
            assert_eq!(
                resolutions_of(&per_item, "${ns}"),
                resolutions_of(&whole, "${ns}"),
                "per-item must match the whole-file walk for {src:?}"
            );
        }
    }

    #[test]
    fn a_dominating_constant_argument_folds_through_the_lattice() {
        // TP — the substitution reads a `$q` that IS a dominating
        // constant; the fold chains through the constant lattice.
        let src = concat!(
            "namespace eval tc { proc setdef {a b} { return 1 } }\n",
            "proc user {} {\n",
            "    set q ::tc::X\n",
            "    set ns [namespace qualifiers $q]\n",
            "    ${ns}::setdef x y\n",
            "}\n",
        );
        let r = analysis(src, "tcl8.6");
        assert_eq!(resolutions_of(&r, "${ns}"), ["::tc::setdef"]);
    }
}

// A constant-dominated computed `namespace eval` target.
//
// `set ns ::app; namespace eval $ns { … }` creates `::app` on every run, so
// the block's procs really do home to `::app::…`.  The word is settled by the
// same identity-resolution helper the command head, `source`,
// `rename`, and `oo::define`'s target already use, so its dominance rule —
// a branch-conditional binding proves nothing — applies here unchanged.
// Anything it cannot settle keeps the per-site `@dynns@` domain.
mod const_dominated_namespace_eval {
    use super::super::Analyser;

    fn proc_names(src: &str) -> Vec<String> {
        let mut names: Vec<String> = Analyser::new()
            .analyse(src, "tcl9.0")
            .all_procs
            .keys()
            .cloned()
            .collect();
        names.sort();
        names
    }

    fn scope_names(src: &str) -> Vec<String> {
        Analyser::new()
            .analyse(src, "tcl9.0")
            .global_scope
            .children
            .iter()
            .map(|c| c.name.clone())
            .collect()
    }

    #[test]
    fn tp_a_constant_target_homes_the_blocks_procs_to_the_real_namespace() {
        let src = "set ns ::app\nnamespace eval $ns {\n    proc go {} { return 1 }\n}\n";
        assert_eq!(proc_names(src), ["::app::go"]);
        assert_eq!(scope_names(src), ["::app"]);
    }

    #[test]
    fn tp_the_word_is_recorded_as_a_declaring_occurrence() {
        // Navigation's half: the `$ns` word is where `::app` is created, so
        // it is the answer a `namespace children ::app` elsewhere jumps to.
        let src = "set ns ::app\nnamespace eval $ns { proc go {} { return 1 } }\n";
        let r = Analyser::new().analyse(src, "tcl9.0").clone();
        let decls: Vec<&str> = r
            .namespace_refs
            .iter()
            .filter(|n| n.declares && n.qualified_name == "::app")
            .map(|n| &src[n.span.start() as usize..n.span.end() as usize])
            .collect();
        assert_eq!(decls, ["$ns"], "{:?}", r.namespace_refs);
    }

    #[test]
    fn fp_a_branch_conditional_binding_keeps_the_synthetic_domain() {
        // FP guard — the dominance rule.  Which namespace this creates is a
        // run-time question, so claiming either would be a wrong answer.
        let src = "set ns ::a\nif {$c} { set ns ::b }\nnamespace eval $ns {\n    proc go {} { return 1 }\n}\n";
        assert!(
            proc_names(src).iter().all(|n| n.contains("@dynns@")),
            "{:?}",
            proc_names(src),
        );
    }

    #[test]
    fn tn_a_parameter_target_keeps_the_synthetic_domain() {
        // TN — the irc.tcl per-connection idiom the synthetic domain exists
        // for: nothing constant reaches `$n`, so each occurrence stays its
        // own scope.
        let src =
            "proc mk {n} {\n    namespace eval $n {\n        proc go {} { return 1 }\n    }\n}\n";
        let names = proc_names(src);
        assert!(names.iter().any(|n| n.contains("@dynns@")), "{names:?}");
    }

    #[test]
    fn tn_a_literal_target_is_unchanged() {
        let src = "namespace eval ::app {\n    proc go {} { return 1 }\n}\n";
        assert_eq!(proc_names(src), ["::app::go"]);
        assert_eq!(scope_names(src), ["::app"]);
    }
}

// A brace-quoted word's *elements* are literal too.
//
// The IR/analyser word arrives with its braces already stripped, so scanning
// the element text alone reports "dynamic" for content Tcl never substitutes.
// The whole-word question for `namespace path` is already answered; these
// are the two remaining places that ask it per element with the token in
// hand.
//
// tclsh-proof (8.6.16 / 9.0.4):
//   namespace eval x {}
//   namespace eval {::$ns} {}
//   namespace eval n { namespace path {::$ns ::x} ; namespace path }
//     ->  {::$ns} ::x        (an entry literally named `::$ns`, no var read)
//   foreach n {aa {$b} cc} { puts $n }
//     ->  aa / $b / cc       (three literal iterations, no read of `b`)
mod braced_word_elements_are_literal {
    use super::super::*;

    /// Namespace-reference source texts recorded for `src`.
    fn ns_ref_texts(src: &str) -> Vec<String> {
        let r = Analyser::new().analyse(src, "tcl9.0").clone();
        let mut out: Vec<String> = r
            .namespace_refs
            .iter()
            .map(|n| src[n.span.start() as usize..n.span.end() as usize].to_owned())
            .collect();
        out.sort();
        out
    }

    /// Diagnostic codes emitted for `src`.
    fn diag_codes(src: &str) -> Vec<String> {
        Analyser::new()
            .analyse(src, "tcl9.0")
            .diagnostics
            .iter()
            .map(|d| d.code.to_string())
            .collect()
    }

    #[test]
    fn tp_namespace_path_records_a_braced_dollar_element_as_a_reference() {
        // The whole-word gate already lets the path be *recorded*; the
        // element-reference walk must not skip the same word — both must
        // see `::$ns`, or the function disagrees with itself.
        let src = "namespace eval n { namespace path {::$ns ::x} }\n";
        assert!(
            ns_ref_texts(src).iter().any(|t| t == "::$ns"),
            "the braced element is a namespace literally named `::$ns`; got {:?}",
            ns_ref_texts(src)
        );
    }

    #[test]
    fn fp_namespace_path_still_skips_a_substituting_element() {
        // FP guard: an unbraced path word does substitute, so its `$ns` names
        // nothing static and must stay unrecorded.
        let src = "namespace eval n { namespace path ::$ns }\n";
        assert!(
            !ns_ref_texts(src).iter().any(|t| t.contains('$')),
            "a substituting path word must record no element reference; got {:?}",
            ns_ref_texts(src)
        );
    }

    #[test]
    fn tn_namespace_path_literal_elements_unchanged() {
        let src = "namespace eval n { namespace path {::x ::y} }\n";
        let refs = ns_ref_texts(src);
        assert!(refs.iter().any(|t| t == "::x"), "{refs:?}");
        assert!(refs.iter().any(|t| t == "::y"), "{refs:?}");
    }

    #[test]
    fn tp_foreach_simulation_survives_a_braced_dollar_element() {
        // One odd element must not abstain the whole simulation — otherwise
        // every proc the loop installs goes missing and each call raises
        // W123.
        let src = "foreach n {aa {$b} cc} { proc $n {} {} }\naa\ncc\n";
        assert!(
            !diag_codes(src).iter().any(|c| c == "W123"),
            "the braced element must not abstain the whole simulation; got {:?}",
            diag_codes(src)
        );
    }

    #[test]
    fn fp_foreach_simulation_still_abstains_on_a_substituting_list() {
        // FP guard: a quoted list word *does* substitute, so its elements are
        // not knowable and the simulation must still decline — declining is
        // no claim, never a claim of absence. tclsh8.6.18/9.0.4: with b
        // bound, the loop defines aa and the call runs.
        let src = "set b bb\nforeach n \"aa $b cc\" { proc $n {} {} }\naa\n";
        assert!(
            !diag_codes(src).iter().any(|c| c == "W123"),
            "a substituting list word must still abstain; got {:?}",
            diag_codes(src)
        );
    }

    #[test]
    fn tn_foreach_simulation_literal_elements_unchanged() {
        let src = "foreach n {aa cc} { proc $n {} {} }\naa\ncc\n";
        assert!(
            !diag_codes(src).iter().any(|c| c == "W123"),
            "{:?}",
            diag_codes(src)
        );
    }
}
