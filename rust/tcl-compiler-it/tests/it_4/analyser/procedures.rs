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

//! Procedure definitions: recorded names and parameters, `tcl::OptProc` definers and unused parameters.

// Proc records in the semantic model.
mod proc_analysis {
    use super::super::*;

    #[test]
    fn simple_proc_records_name_and_params() {
        crate::install_surfaces();
        let r = Analyser::new().analyse("proc greet {name} { puts $name }", D);
        let proc = r.global_scope.procs.get("greet").expect("greet recorded");
        assert_eq!(proc.name, "greet");
        assert_eq!(proc.qualified_name, "::greet");
        assert_eq!(proc.params.len(), 1);
        assert_eq!(proc.params[0].name, "name");
    }

    #[test]
    fn multiple_params_in_order() {
        crate::install_surfaces();
        let r = Analyser::new().analyse("proc add {a b} { return [+ $a $b] }", D);
        let proc = &r.global_scope.procs["add"];
        assert_eq!(
            proc.params
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            ["a", "b"]
        );
    }

    #[test]
    fn default_param_value_captured() {
        crate::install_surfaces();
        let r = Analyser::new().analyse("proc greet {{name World}} { puts $name }", D);
        let p = &r.global_scope.procs["greet"].params[0];
        assert_eq!(p.name, "name");
        assert!(p.has_default);
        assert_eq!(p.default_value.as_deref(), Some("World"));
    }

    #[test]
    fn no_params_is_empty() {
        crate::install_surfaces();
        let r = Analyser::new().analyse("proc noop {} { }", D);
        assert_eq!(r.global_scope.procs["noop"].params.len(), 0);
    }

    #[test]
    fn proc_in_all_procs_qualified() {
        crate::install_surfaces();
        let r = Analyser::new().analyse("proc foo {} {}", D);
        assert!(r.all_procs.contains_key("::foo"));
    }

    #[test]
    fn doc_harvested_from_preceding_comment() {
        crate::install_surfaces();
        let src = "# Adds two numbers\nproc add {a b} { return [+ $a $b] }\n";
        let r = Analyser::new().analyse(src, D);
        assert!(r.global_scope.procs["add"].doc.contains("Adds two numbers"));
    }

    #[test]
    fn multiple_procs_each_recorded() {
        crate::install_surfaces();
        let r = Analyser::new().analyse("proc foo {} {}\nproc bar {x} { return $x }\n", D);
        assert!(r.global_scope.procs.contains_key("foo"));
        assert!(r.global_scope.procs.contains_key("bar"));
    }

    #[test]
    fn proc_creates_child_scope_with_param_and_local() {
        crate::install_surfaces();
        let r = Analyser::new().analyse("proc foo {x} { set y 1 }", D);
        assert_eq!(r.global_scope.children.len(), 1);
        let s = &r.global_scope.children[0];
        assert_eq!(s.kind, tcl_compiler::analyser::ScopeKind::Proc);
        assert_eq!(s.name, "foo"); // proc-scope is keyed by the simple name
        assert!(s.variables.contains_key("x"));
        assert!(s.variables.contains_key("y"));
    }
}

// `tcl::OptProc` — the `opt` package's automatic-option-parsing proc
// definer. Runtime mechanism (tclsh9.0/8.6-verified):
// installs `::proc $name args {...}` unconditionally — the real Tcl-level
// signature is always the single `args` catch-all, regardless of what
// `optlist` declares; `optlist`'s own descriptor words are bound as local
// variables in the body by `::tcl::OptKeyParse`, with a leading `-` on a
// flag descriptor stripped for the bound name.
mod opt_proc_definer {
    use super::super::*;

    #[test]
    fn records_proc_with_real_args_only_arity() {
        crate::install_surfaces();
        // TP — the analyser must record the *real* Tcl-level signature,
        // never `optlist`'s own descriptor words.
        let r = Analyser::new().analyse(
            "tcl::OptProc greet {child -use -display} { return $child }",
            D,
        );
        let proc = r.all_procs.get("::greet").expect("greet recorded");
        assert_eq!(
            proc.params
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            ["args"],
            "the real Tcl-level signature is always the single args catch-all"
        );
    }

    #[test]
    fn fully_qualified_spelling_also_registers() {
        crate::install_surfaces();
        // TP — real corpus code commonly writes this fully qualified;
        // `resolve_analyser_hook` must resolve it identically to the bare
        // spelling.
        let r = Analyser::new().analyse(
            "::tcl::OptProc greet {child -use -display} { return $child }",
            D,
        );
        assert!(r.all_procs.contains_key("::greet"), "{:?}", r.all_procs);
    }

    #[test]
    fn call_with_any_arity_draws_no_wrong_arg_count_diagnostic() {
        crate::install_surfaces();
        // TP: every real call must not report "wrong number of arguments" —
        // the stub proc's `{}`-arity `ProcDef` must be overwritten with the
        // real signature.
        let src = "tcl::OptProc greet {child -use -display} { return $child }\ngreet a b c d\n";
        assert!(!fires(src, D, "E003"), "{:?}", codes(src, D));
    }

    /// The **accepted trade-off**, pinned as a choice rather than left to
    /// read as an oversight.
    ///
    /// C Tcl does enforce a positional-argument bound for an `OptProc`, at
    /// call time, from the parsed optlist — tclsh 9.0.4 and 8.6.16 agree
    /// exactly:
    ///
    /// ```text
    /// ::tcl::OptProc ::safe::loadTk {{child -tkAppName} {-use …} {-display …}} {…}
    /// ::safe::loadTk interp1                       → OK (one positional arg)
    /// ::safe::loadTk interp1 extraPositionalArg    → too many arguments
    ///                                                (unexpected argument(s): …)
    /// ```
    ///
    /// The definer installs `::proc $name args {…}`, so nothing in the
    /// *declaration* bounds the call; reproducing the bound would mean
    /// modelling `::tcl::OptKeyParse`'s runtime dispatch over the optlist
    /// (flag descriptors, defaults, `-…` terminators).  Until that exists,
    /// the choice is between a false E003 on every legitimate call — the
    /// defect the audit found, `::safe::loadTk interp1` reported as "too
    /// many arguments: expected at most 0" — and silence on a genuine
    /// overflow.  Silence is the abstaining direction this whole cluster
    /// takes, so the overflow goes unreported **on purpose**.
    ///
    /// The scope of that silence is what keeps it honest, so it is asserted
    /// here too: an ordinary proc is still arity-checked
    /// (`unrelated_proc_still_gets_real_arity_checked`), and this is a
    /// definer-scoped abstention, not a retreat from arity checking.
    #[test]
    fn a_genuine_opt_proc_arity_overflow_is_deliberately_unreported() {
        crate::install_surfaces();
        let src = "tcl::OptProc greet {child -use -display} { return $child }\n\
                   greet interp1 extraPositionalArg\n";
        assert!(
            !fires(src, D, "E002") && !fires(src, D, "E003"),
            "the overflow is real in C Tcl but deliberately not modelled here: {:?}",
            codes(src, D)
        );
        // …and the legitimate call is silent for the same reason, which is
        // the half the audit was actually about.
        let ok = "tcl::OptProc greet {child -use -display} { return $child }\ngreet interp1\n";
        assert!(
            !fires(ok, D, "E002") && !fires(ok, D, "E003"),
            "a legitimate OptProc call must never draw arity noise: {:?}",
            codes(ok, D)
        );
    }

    #[test]
    fn optlist_flag_descriptor_dash_is_stripped_for_the_bound_local() {
        crate::install_surfaces();
        // TP — `::tcl::OptKeyParse` binds `-use`/`-display` as `use`/
        // `display`, never with the leading dash.
        let src =
            "tcl::OptProc greet {child -use -display} { return \"$use $display\" }\ngreet a\n";
        assert!(
            !fires(src, D, "W210"),
            "use/display must resolve as bound locals, not read-before-set: {:?}",
            codes(src, D)
        );
    }

    #[test]
    fn args_catch_all_is_readable_in_the_body() {
        crate::install_surfaces();
        // TP — `$args` still holds the whole original argument list after
        // `OptKeyParse` runs, so a body reference to it (inspecting
        // leftovers) is legitimate, not a false read-before-set.
        let src = "tcl::OptProc greet {child} { return $args }\ngreet a\n";
        assert!(!fires(src, D, "W210"), "{:?}", codes(src, D));
    }

    #[test]
    fn dash_stripped_local_does_not_collide_with_its_own_optlist_declaration() {
        crate::install_surfaces();
        // FN guard: a naive anchor for the synthetic `args` binding (this
        // idiom writes no literal `args` word anywhere) could collide with
        // another symbol's own span and silently hide it — every
        // optlist-derived local must still resolve to its own name, not
        // `args` (anchoring `args` to the whole `optlist` word instead
        // would swallow every one of its own descriptor sub-spans).
        let r = Analyser::new().analyse(
            "tcl::OptProc greet {child -use -display} { return $child }",
            D,
        );
        let scope = &r.global_scope.children[0];
        assert!(
            scope.variables.contains_key("child"),
            "{:?}",
            scope.variables
        );
        assert!(scope.variables.contains_key("use"), "{:?}", scope.variables);
        assert!(
            scope.variables.contains_key("display"),
            "{:?}",
            scope.variables
        );
        assert!(
            scope.variables.contains_key("args"),
            "{:?}",
            scope.variables
        );
    }

    #[test]
    fn unrelated_proc_still_gets_real_arity_checked() {
        crate::install_surfaces();
        // FP guard: the fix must not loosen arity checking for ordinary
        // procs elsewhere in the same document.
        let src = "proc real {a b} { return [+ $a $b] }\nreal 1\n";
        assert!(fires(src, D, "E002"), "{:?}", codes(src, D));
    }

    #[test]
    fn plain_double_proc_redefinition_still_last_definition_wins() {
        crate::install_surfaces();
        // TN — control: an ordinary double-`proc` redefinition (unrelated
        // to `tcl::OptProc`) is unaffected by any of this fix's changes.
        let r = Analyser::new().analyse(
            "proc greet {} { return 1 }\nproc greet {x} { return $x }\n",
            D,
        );
        let proc = r.all_procs.get("::greet").expect("greet recorded");
        assert_eq!(
            proc.params
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            ["x"],
            "last definition wins: {:?}",
            proc.params
        );
    }
}

// Unused proc parameters (W214) — pure static heuristic.
mod unused_proc_parameters {
    use super::super::*;

    fn w214(src: &str) -> Vec<String> {
        analyser_diags(src, D)
            .into_iter()
            .filter(|(c, _, _)| c == "W214")
            .map(|(_, m, _)| m)
            .collect()
    }

    #[test]
    fn unused_param_detected() {
        crate::install_surfaces();
        let w = w214("proc foo {x y} { puts $x }");
        assert_eq!(w.len(), 1);
        assert!(w[0].contains('y'));
        // Severity is Hint.
        assert!(
            analyser_diags("proc foo {x y} { puts $x }", D)
                .iter()
                .any(|(c, _, s)| c == "W214" && *s == Severity::Hint)
        );
    }

    #[test]
    fn all_params_used_no_warning() {
        crate::install_surfaces();
        assert_eq!(
            w214("proc foo {a b} { puts $a; puts $b }"),
            [] as [std::string::String; 0]
        );
    }

    #[test]
    fn args_param_not_flagged() {
        crate::install_surfaces();
        // `args` is the variadic catch-all and is never "unused".
        assert_eq!(
            w214("proc foo {args} { puts hello }"),
            [] as [std::string::String; 0]
        );
    }

    #[test]
    fn underscore_prefixed_param_is_still_flagged_rust_behaviour() {
        crate::install_surfaces();
        // A leading `_` is not treated as a deliberate "unused" marker —
        // `_unused` still fires W214.
        let w = w214("proc foo {_unused x} { puts $x }");
        assert_eq!(w.len(), 1);
        assert!(w[0].contains("_unused"));
    }

    #[test]
    fn multiple_unused_params_each_flagged() {
        crate::install_surfaces();
        assert_eq!(w214("proc foo {a b c} { puts hello }").len(), 3);
    }

    #[test]
    fn param_used_in_return_or_branch_condition() {
        crate::install_surfaces();
        assert_eq!(
            w214("proc foo {x} { return $x }"),
            [] as [std::string::String; 0]
        );
        assert_eq!(
            w214("proc foo {x} { if {$x > 0} { puts yes } }"),
            [] as [std::string::String; 0]
        );
    }

    #[test]
    fn param_used_in_expr_alias_counts_as_read() {
        crate::install_surfaces();
        // interp alias for expr — refs inside the aliased expr arg are reads.
        let src = "interp alias {} = {} expr\nproc foo {x y} {\n    set result [= {$x + $y}]\n    return $result\n}\n";
        assert_eq!(w214(src), [] as [std::string::String; 0]);
    }

    #[test]
    fn param_used_in_dict_for_and_dict_map_bodies() {
        crate::install_surfaces();
        // dict for/map bodies are lowered into real CFG blocks in the analysis
        // build, so a param read nested in the body is a first-class SSA
        // use, with the deep body scan / text fallback as belt-and-braces.
        let used = [
            "proc f {but} {\n    dict for {k v} $d {\n        if {$but ne \"\"} { puts $but }\n    }\n}\n",
            "proc f {scale} {\n    dict map {k v} $d { expr {$v * $scale} }\n}\n",
            "proc foo {targetKey} {\n    set d [dict create]\n    dict for {k v} $d {\n        if {[dict exists $v $targetKey]} { puts hi }\n    }\n}\n",
        ];
        for src in used {
            assert!(w214(src).is_empty(), "unexpected W214 for {src:?}");
        }
    }

    #[test]
    fn unused_param_alongside_used_in_dict_for_still_flagged() {
        crate::install_surfaces();
        let src = "proc f {{count 0} other} {\n    dict for {k v} $d {\n        if {$count>0} { incr count }\n    }\n}\n";
        let w = w214(src);
        assert_eq!(w.len(), 1);
        assert!(w[0].contains("other"));
    }

    #[test]
    fn param_read_inside_catch_body() {
        crate::install_surfaces();
        assert_eq!(
            w214("proc f {x} {\n    catch { puts $x }\n}\n"),
            [] as [std::string::String; 0]
        );
    }

    #[test]
    fn param_used_only_by_dict_with_or_update() {
        crate::install_surfaces();
        // dict with / dict update mark the dict var VAR_READ+VAR_WRITE; the read
        // must survive the defs filter.
        assert_eq!(
            w214("proc f {pdata} {\n    dict with pdata {}\n}\n"),
            [] as [std::string::String; 0]
        );
        assert_eq!(
            w214("proc f {d} {\n    dict update d k v {}\n}\n"),
            [] as [std::string::String; 0]
        );
    }

    #[test]
    fn pure_write_to_param_inside_opaque_body_still_flags() {
        crate::install_surfaces();
        let src = "proc f {x} {\n    dict for {k v} $d { set x 1 }\n}\n";
        let w = w214(src);
        assert_eq!(w.len(), 1);
        assert!(w[0].contains('x'));
    }

    #[test]
    fn unrelated_namespaced_for_does_not_get_dict_body_fallback() {
        crate::install_surfaces();
        // Only ::tcl::dict::for|map get the synthetic body-arg fallback.
        let src = "proc f {x} {\n    ::my::for a b { set x 1 }\n}\n";
        let w = w214(src);
        assert_eq!(w.len(), 1);
        assert!(w[0].contains('x'));
    }

    #[test]
    fn return_expr_and_cmd_subst_do_not_false_flag() {
        crate::install_surfaces();
        for src in [
            "proc foo {x} { return [expr {$x + 1}] }",
            "proc foo {x} { return [string length $x] }",
        ] {
            assert!(w214(src).is_empty(), "unexpected W214 for {src:?}");
        }
        // …but a genuinely-unused param still warns alongside.
        let w = w214("proc foo {x y} { return [expr {$x + 1}] }");
        assert_eq!(w.len(), 1);
        assert!(w[0].contains('y'));
    }

    #[test]
    fn dict_for_braced_data_word_is_not_a_use() {
        crate::install_surfaces();
        // tclsh: braces inhibit substitution, so `{$unused}` never reads
        // `unused`; once `dict for` is proved the builtin its body is scanned
        // as written and the parameter is reported.
        let src = "proc foo {used unused} {\n    set d [dict create]\n    dict for {k v} $d {\n        set msg {$unused}\n        puts $used\n    }\n}\n";
        let w = w214(src);
        assert_eq!(w.len(), 1, "{w:?}");
        assert!(w[0].contains("unused"), "{w:?}");
    }

    // NOTE — trace-callback W214 (see `trace_callbacks` module): the analyser +
    // run_all_checks surface does NOT special-case trace callbacks, so W214
    // fires on a `trace add variable … watcher` callback's `name1 name2 op`
    // params. The control test `unrelated_proc_unused_param_still_flagged`
    // (which fires either way) covers this.
}
