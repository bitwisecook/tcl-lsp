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

//! O103: calls to pure procedures folded to their constant return, name resolution and the rebindings that withdraw the fold.

use super::*;

#[test]
fn resolve_proc_qname_walks_namespace_chain() {
    // Two procs in `::ns`; a bare `inner` call from `::ns` resolves to
    // `::ns::inner`, not the (absent) root `::inner`.
    let module = crate::lowering::lower_to_ir(
        "namespace eval ::ns {\n proc inner {} { return 1 }\n proc outer {} { inner }\n}",
        &registry(),
    );
    let ia = crate::interprocedural::build_interprocedural_analysis(
        &module,
        &registry(),
        None,
        crate::interprocedural::ObjectTypeMap::none(),
        crate::realm::CommandBindingRealm::none(),
        None,
    );
    assert!(
        fixture_id_if_present(&ia.procedures, "::ns::inner").is_some(),
        "expected ::ns::inner in IA, got {:?}",
        ia.procedures.keys().collect::<Vec<_>>(),
    );
    assert_eq!(
        resolve_proc_qname("inner", "::ns", &ia).as_deref(),
        Some("::ns::inner"),
        "bare call should resolve against the enclosing namespace",
    );
    // An absolute spelling is taken as-is.
    assert_eq!(
        resolve_proc_qname("::ns::inner", "::other", &ia).as_deref(),
        Some("::ns::inner"),
    );
    // A bare call with no matching namespace candidate fails to resolve.
    assert_eq!(resolve_proc_qname("missing", "::ns", &ia), None);
    // From the root namespace a bare `inner` does *not* reach `::ns::inner`.
    assert_eq!(resolve_proc_qname("inner", "::", &ia), None);
}

#[test]
fn resolve_proc_qname_does_not_walk_ancestor_namespaces() {
    // Real Tcl bareword resolution is exactly two levels — current
    // namespace, then global — absent an explicit `namespace path`.
    // `::a::b::c::caller`'s body calling bare `foo` must NOT resolve to
    // a `::a::foo` defined in a *grandparent* namespace: real tclsh
    // raises "invalid command name" there, it does not silently walk up
    // to find `::a::foo`.
    let module = crate::lowering::lower_to_ir(
        "namespace eval ::a {\n proc foo {} { return 1 }\n namespace eval b {\n  namespace eval c {\n   proc caller {} { foo }\n  }\n }\n}",
        &registry(),
    );
    let ia = crate::interprocedural::build_interprocedural_analysis(
        &module,
        &registry(),
        None,
        crate::interprocedural::ObjectTypeMap::none(),
        crate::realm::CommandBindingRealm::none(),
        None,
    );
    assert!(
        fixture_id_if_present(&ia.procedures, "::a::foo").is_some(),
        "expected ::a::foo in IA, got {:?}",
        ia.procedures.keys().collect::<Vec<_>>(),
    );
    assert_eq!(
        resolve_proc_qname("foo", "::a::b::c", &ia),
        None,
        "a grandparent-namespace proc must not resolve for a bare call",
    );
    // Control: the *direct* enclosing namespace still resolves.
    let module2 = crate::lowering::lower_to_ir(
        "namespace eval ::a::b::c {\n proc foo {} { return 1 }\n proc caller {} { foo }\n}",
        &registry(),
    );
    let ia2 = crate::interprocedural::build_interprocedural_analysis(
        &module2,
        &registry(),
        None,
        crate::interprocedural::ObjectTypeMap::none(),
        crate::realm::CommandBindingRealm::none(),
        None,
    );
    assert_eq!(
        resolve_proc_qname("foo", "::a::b::c", &ia2).as_deref(),
        Some("::a::b::c::foo"),
    );
}

#[test]
fn resolve_proc_qname_relative_dotted_word_prefers_current_namespace() {
    // A relative *dotted* word (`ns2::inner`, containing `::` but not
    // starting with it) must resolve against the current namespace
    // first, not be rooted straight at global — the same rule
    // `bareword_resolution_candidates` documents (confirmed against
    // tclsh 9.0.4). Two procs of this shape exist here, one nested
    // under `::ns` and one at the root; a bare `ns2::inner` call from
    // inside `::ns` must reach the nearer one.
    let module = crate::lowering::lower_to_ir(
        "proc ns2::inner {} { return 1 }\n\
             namespace eval ::ns {\n proc ns2::inner {} { return 2 }\n proc caller {} { ns2::inner }\n}",
        &registry(),
    );
    let ia = crate::interprocedural::build_interprocedural_analysis(
        &module,
        &registry(),
        None,
        crate::interprocedural::ObjectTypeMap::none(),
        crate::realm::CommandBindingRealm::none(),
        None,
    );
    assert!(fixture_id_if_present(&ia.procedures, "::ns2::inner").is_some());
    assert!(fixture_id_if_present(&ia.procedures, "::ns::ns2::inner").is_some());
    assert_eq!(
        resolve_proc_qname("ns2::inner", "::ns", &ia).as_deref(),
        Some("::ns::ns2::inner"),
        "a relative dotted call must prefer the current-namespace proc \
             over the root one",
    );
    // Falls back to the root proc when there is no current-namespace
    // candidate.
    assert_eq!(
        resolve_proc_qname("ns2::inner", "::other", &ia).as_deref(),
        Some("::ns2::inner"),
    );
}

#[test]
fn static_proc_call_folds_to_constant_return() {
    // ::answer returns 42 unconditionally and is pure — a
    // call to ::answer can be folded.
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let cu =
        CompilationUnit::build_for("proc ::answer {} { return 42 }\n::answer", &registry, false)
            .with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations.iter().any(|o| o.code == DiagCode::O103),
        "expected O103 static-proc fold, got {:?}",
        ctx.optimisations,
    );
}

#[test]
fn o103_cmd_subst_fires_applicable_rewrite() {
    // `[::answer]` inside an argv → applicable O103 with the
    // argv span covering `[::answer]` and the constant return
    // as replacement.
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let source = "proc ::answer {} { return 42 }\nputs [::answer]";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    let o103s: Vec<_> = ctx
        .optimisations
        .iter()
        .filter(|o| o.code == DiagCode::O103)
        .collect();
    assert!(
        o103s.iter().any(|o| !o.hint_only
            && o.replacement == "42"
            && &source[o.span.start() as usize..o.span.end() as usize] == "[::answer]"),
        "expected applicable O103 spanning `[::answer]`, got {o103s:?}",
    );
}

#[test]
fn o103_folds_multi_word_string_return_list_quoted() {
    // A pure proc returning a multi-word
    // string folds in a cmd-sub position, list-quoted as one word
    // (a bare `a b c` is not a safe single word).
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let source = "proc ::greet {} { return \"a b c\" }\nputs [::greet]";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    let o103s: Vec<_> = ctx
        .optimisations
        .iter()
        .filter(|o| o.code == DiagCode::O103)
        .collect();
    assert!(
        o103s
            .iter()
            .any(|o| !o.hint_only && o.replacement == "{a b c}"),
        "expected applicable O103 folding [::greet] to {{a b c}}, got {o103s:?}",
    );
}

#[test]
fn o103_cmd_subst_fires_on_set_value_target() {
    // The value-position cmd-sub folds must also
    // fire on a `set TARGET [cmd-sub]` target. `set` lowers to
    // `AssignValue`, which `walk_statement` did not visit for cmd-sub
    // folds, so `set y [::answer]` missed the O103 fold that the same
    // `[::answer]` gets in a command-argument position (`puts
    // [::answer]`). With the AssignValue arm wired, the set value
    // folds identically.
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let source = "proc ::answer {} { return 42 }\nproc ::f {} { set y [::answer]\nputs $y }";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    let o103s: Vec<_> = ctx
        .optimisations
        .iter()
        .filter(|o| o.code == DiagCode::O103)
        .collect();
    assert!(
        o103s.iter().any(|o| !o.hint_only
            && o.replacement == "42"
            && &source[o.span.start() as usize..o.span.end() as usize] == "[::answer]"),
        "expected applicable O103 spanning the `[::answer]` set value, got {o103s:?}",
    );
}

#[test]
fn o103_scoped_call_withdraws_later_callee_changes_and_unknown_entry() {
    for source in [
        "proc answer {} {return 42}; proc f {} {set y [answer]}; rename answer saved; proc answer {} {return 99}",
        "proc answer {} {return 42}; proc f {} {set y [answer]}; unknown_future_entry",
        "proc answer {} {return 42}; proc f {} {set y [answer]}; trace add execution answer enter callback",
    ] {
        let opts = run_pass_with_command_mutations(source);
        assert!(
            opts.iter()
                .all(|optimisation| optimisation.code != DiagCode::O103 || optimisation.hint_only),
            "{source}: {opts:?}"
        );
    }
}

#[test]
fn o103_bare_call_stays_hint_only() {
    // Top-level `::answer` — the call result is discarded, so
    // folding it as a statement would leave an invalid `42`
    // command. Diagnostic must stay hint-only.
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let cu =
        CompilationUnit::build_for("proc ::answer {} { return 42 }\n::answer", &registry, false)
            .with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    let o103s: Vec<_> = ctx
        .optimisations
        .iter()
        .filter(|o| o.code == DiagCode::O103)
        .collect();
    assert!(!o103s.is_empty(), "expected at least one O103");
    assert!(
        o103s.iter().all(|o| o.hint_only),
        "bare-call form must stay hint-only, got {o103s:?}",
    );
}

#[test]
fn o103_folds_arg_sensitive_passthrough_cmd_subst() {
    // `::not_const {x} { return $x }` has no argument-*independent*
    // constant return, but with the call's constant arg bound it folds:
    // `[::not_const 1]` → `1` (the passthrough folds). The applicable
    // rewrite fires only for the CMD-subst form; the bare-call form
    // stays hint-only.
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let cu = CompilationUnit::build_for(
        "proc ::not_const {x} { return $x }\nputs [::not_const 1]",
        &registry,
        false,
    )
    .with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations
            .iter()
            .any(|o| o.code == DiagCode::O103 && o.replacement == "1" && !o.hint_only),
        "expected applicable O103 folding [::not_const 1] to 1, got {:?}",
        ctx.optimisations,
    );
}

#[test]
fn o103_folds_arg_sensitive_passthrough_with_braced_multiword_literal() {
    // `parse_static_call_args` shares `literal_words`'s proper Tcl-aware
    // tokeniser rather than a naive `split_whitespace`, so a braced
    // multi-word call argument (`{a b}` — one clean literal argument in
    // real Tcl) folds too.
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let cu = CompilationUnit::build_for(
        "proc ::passthrough {x} { return $x }\nputs [::passthrough {a b}]",
        &registry,
        false,
    )
    .with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations
            .iter()
            .any(|o| o.code == DiagCode::O103 && o.replacement == "{a b}" && !o.hint_only),
        "expected applicable O103 folding [::passthrough {{a b}}] to {{a b}}, got {:?}",
        ctx.optimisations,
    );
}

#[test]
fn o103_folds_variadic_args_proc_returning_args_directly() {
    // TP: `args` is seeded as a canonical Tcl list (see
    // `seed_params_from_args`), not skipped outright — `proc ::foo {a
    // args} { return $args }` called as `[::foo 1 2]` binds `a=1`,
    // `args={2}` (a one-element list, which renders bare as `2` with
    // no braces needed), so `return $args` folds to `2`. Confirmed
    // against tclsh 8.6: `puts [::foo 1 2]` prints `2`.
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let cu = CompilationUnit::build_for(
        "proc ::foo {a args} { return $args }\nputs [::foo 1 2]",
        &registry,
        false,
    )
    .with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations
            .iter()
            .any(|o| o.code == DiagCode::O103 && o.replacement == "2" && !o.hint_only),
        "expected applicable O103 folding [::foo 1 2] to 2 (args seeded as a list), got {:?}",
        ctx.optimisations,
    );
}

#[test]
fn o103_does_not_fold_recursive_or_loop_proc() {
    // A proc whose return SCCP cannot determine under the bound args must
    // not fold — a loop-carried `return $total` is a phi (Overdefined at
    // exit), even though a pre-loop `set total 0` leaves a stale Const(0).
    // Mis-folding it would be a miscompile.
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default();
    let src = "proc ::sum {ns} {\n  set total 0\n  foreach n $ns { set total [expr {$total + $n}] }\n  return $total\n}\nputs [::sum {1 2 3}]\n";
    let cu =
        CompilationUnit::build_for(src, &registry, false).with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations
            .iter()
            .all(|o| o.code != DiagCode::O103 || o.hint_only),
        "loop-carried return must not fold, got {:?}",
        ctx.optimisations,
    );
}

#[test]
fn o103_folds_implicit_return_proc_cmd_subst() {
    // TP: the KCS O103 doc's own canonical example — a proc with NO
    // explicit `return`, whose result is Tcl's "value of the last
    // command executed" (a bare `expr {…}`) — must fold. Before the
    // fall-through fix, `resolve_return_constant` only looked at
    // `Terminator::Return` blocks, so a pure proc that relies entirely
    // on implicit return never folded at all (`[double 21]` stayed
    // unrewritten even though tclsh evaluates it to `42`).
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let src = "proc double {n} { expr {$n * 2} }\nset x [double 21]\n";
    let cu =
        CompilationUnit::build_for(src, &registry, false).with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations
            .iter()
            .any(|o| o.code == DiagCode::O103 && o.replacement == "42" && !o.hint_only),
        "expected applicable O103 folding [double 21] to 42 (implicit return), got {:?}",
        ctx.optimisations,
    );
}

#[test]
fn o103_folds_implicit_return_zero_arg_proc() {
    // TP: a zero-parameter implicit-return proc (`set` as the trailing
    // statement — Tcl's `set` returns the value it just assigned, so
    // this is exactly like `return $pi`) also folds via the CMD-subst
    // path.
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let src = "proc pi {} { set p 3 }\nputs [pi]\n";
    let cu =
        CompilationUnit::build_for(src, &registry, false).with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations
            .iter()
            .any(|o| o.code == DiagCode::O103 && o.replacement == "3" && !o.hint_only),
        "expected applicable O103 folding [pi] to 3 (implicit `set` return), got {:?}; {}",
        ctx.optimisations,
        proc_fold_context(&cu, "::pi"),
    );
}

#[test]
fn o103_does_not_fold_when_return_and_fallthrough_disagree() {
    // FN / miscompile-guard: `helper`'s result is opaque to intraprocedural
    // SCCP (it's a call, not folded interprocedurally), so `g`'s `if`
    // condition can't be resolved even after `n` is seeded to 5 — both the
    // `return 1` branch AND the fall-through `expr {99}` stay executable.
    // tclsh 9.0.4 ground truth: `g 5` calls `helper 5` (== 6), `6 > 100` is
    // false, so `g 5` falls through and returns `99` — NOT `1`. Before the
    // fall-through fix, the return reading silently ignored the
    // reachable non-`Return` exit and confidently (wrongly) folded to the
    // `return 1` branch's value — a real miscompile, not just an
    // over-eager fold.
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default();
    let src = "proc helper {n} { expr {$n + 1} }\nproc g {n} {\n    if {[helper $n] > 100} {\n        return 1\n    }\n    expr {99}\n}\nputs [g 5]\n";
    let cu =
        CompilationUnit::build_for(src, &registry, false).with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations
            .iter()
            .all(|o| o.code != DiagCode::O103 || o.hint_only),
        "disagreeing return/fall-through exits must not produce an applicable fold, got {:?}",
        ctx.optimisations,
    );
}

#[test]
fn o103_does_not_fold_through_empty_if_body() {
    // FP guard / miscompile-guard (reported in code review): a trailing
    // `if {$c} {}` with an empty body (and no `else`) is itself a real
    // Tcl command whose result is `""` — whether the condition is true
    // (the empty body's own result) or false (no branch ran, no
    // `else`). tclsh 9.0.4 ground truth: `f 0` and `f 1` both return
    // `""`, NOT `1` from the preceding `set x 1`. An earlier, more
    // permissive fall-through resolver walked straight through the
    // empty `if`-body block back to `set x 1` and wrongly folded `[f
    // 0]` to `1` — treating "0 statements" as "no command ran here"
    // rather than "this command's own result is empty". The fold must
    // decline entirely rather than inherit a stale prior value.
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default();
    let src = "proc f {c} { set x 1\nif {$c} {} }\nputs [f 0]\n";
    let cu =
        CompilationUnit::build_for(src, &registry, false).with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations
            .iter()
            .all(|o| o.code != DiagCode::O103 || o.hint_only),
        "empty if-body fall-through must not produce an applicable fold, got {:?}",
        ctx.optimisations,
    );
}

#[test]
fn o103_folds_variadic_args_proc_ignoring_trailing_args() {
    // TP: a proc with a trailing `args` parameter folds on its fixed
    // parameter when the call supplies MORE arguments than the proc has
    // fixed params — the extra args collect into `args`, unused by this
    // body. Before the `args` fix, `evaluate_proc_with_constants`
    // required `params.len() == args.len()` exactly, so a call
    // supplying more than one trailing argument (`[f 5 x y z]` for
    // `proc f {a args}`) never even reached the fold attempt.
    use tcl_registry::CommandRegistry;
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let src = "proc f {a args} { expr {$a * 2} }\nputs [f 5 x y z]\n";
    let cu =
        CompilationUnit::build_for(src, &registry, false).with_interprocedural(&registry, None);
    let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    ctx.registry = Some(&registry);
    run(&mut ctx, &cu);
    assert!(
        ctx.optimisations
            .iter()
            .any(|o| o.code == DiagCode::O103 && o.replacement == "10" && !o.hint_only),
        "expected applicable O103 folding [f 5 x y z] to 10, got {:?}",
        ctx.optimisations,
    );
}

/// Run the propagation pass with `ctx.command_mutations` populated —
/// the piece `optimise_raw` (used by the O129 trust-gate test) never
/// wires (it doesn't set `cu.interproc` either, so it can't drive
/// O103 at all) and a bare `PassContext::new` + `run` leaves at its
/// all-trusting `Default`. Mirrors the whole-module scan every real
/// production entry point (`optimise_unit_raw`, `find_dead_stores`,
/// `optimise_by_pass`) performs before running passes.
fn run_pass_with_command_mutations(source: &str) -> Vec<Optimisation> {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    let cu = CompilationUnit::build_for_profile(source, &registry, false, profile)
        .with_interprocedural(&registry, Some(profile));
    let mut ctx = crate::optimiser::manager::build_pass_context(&cu, &registry, Some(profile));
    run(&mut ctx, &cu);
    ctx.optimisations
}

#[test]
fn o103_does_not_fold_call_to_proc_renamed_over() {
    // The selected moved allocation cannot borrow the replaced body's summary.
    // Original C9.0.4: double 21 returns 63 after this rename.
    let src = "proc double {n} { expr {$n * 2} }\nproc triple {n} { expr {$n * 3} }\nrename triple double\nputs [double 21]\n";
    let opts = run_pass_with_command_mutations(src);
    assert!(
        opts.iter().all(|o| o.code != DiagCode::O103 || o.hint_only),
        "call to a proc name later `rename`d over must not fold, got {opts:?}",
    );
}

#[test]
fn o103_folds_the_actual_procedure_selected_by_interp_alias() {
    // The selected alias target returns 99 on C Tcl 8.4–9.1.
    let src = "proc answer {} { return 42 }\nproc other {} { return 99 }\ninterp alias {} answer {} other\nputs [answer]\n";
    let opts = run_pass_with_command_mutations(src);
    assert!(
        opts.iter()
            .any(|o| o.code == DiagCode::O103 && !o.hint_only && o.replacement == "99"),
        "the exact alias target must fold to its own result, got {opts:?}",
    );
    // jimsh 0.84 rejects the four-word `interp alias` (`wrong # args`), so
    // the call never reaches either body.
    let jim_registry = tcl_registry::model::ingress::static_context_for("jim").commands();
    let jim = crate::optimiser::optimise_raw(src, jim_registry, Some("jim"));
    assert!(jim.iter().all(|o| o.code != DiagCode::O103 || o.hint_only));
}

#[test]
fn o103_still_folds_unrelated_proc_despite_unrelated_rename_elsewhere() {
    // TN control: `trusts_proc_binding` must be per-name, not a blanket
    // "any rename anywhere disables all O103 folds" — an unrelated
    // rename of a *different* proc must not block folding calls to
    // procs it never touched.
    let src = "proc double {n} { expr {$n * 2} }\nproc triple {n} { expr {$n * 3} }\nrename triple somethingElse\nputs [double 21]\n";
    let opts = run_pass_with_command_mutations(src);
    assert!(
        opts.iter()
            .any(|o| o.code == DiagCode::O103 && o.replacement == "42" && !o.hint_only),
        "unrelated proc must still fold despite an unrelated rename, got {opts:?}",
    );
}

#[test]
fn o103_does_not_fold_call_to_proc_renamed_over_via_namespace_relative_rename() {
    // FP guard / miscompile-guard (reported in code review): `rename
    // triple double` inside `proc ::ns::doit` resolves both bare names
    // relative to `::ns` (the proc's own namespace) — it renames
    // `::ns::triple` onto `::ns::double`, NOT the (nonexistent)
    // top-level `::triple`/`::double`. tclsh 9.0.4 ground truth: after
    // `::ns::doit` runs, `::ns::double 21` executes `triple`'s body
    // (21*3 == 63), not `double`'s original body (21*2 == 42). An
    // earlier version of `collect_proc_rebindings` always rooted a bare
    // rename argument at the GLOBAL namespace (`::double`/`::triple`),
    // so it never marked `::ns::double`/`::ns::triple` as rebound and
    // O103 could fold `[::ns::double 21]` to the wrong `42`.
    let src = "namespace eval ::ns {\n    proc double {n} { expr {$n * 2} }\n    proc triple {n} { expr {$n * 3} }\n}\nproc ::ns::doit {} { rename triple double }\n::ns::doit\nputs [::ns::double 21]\n";
    let opts = run_pass_with_command_mutations(src);
    assert!(
        opts.iter().all(|o| o.code != DiagCode::O103 || o.hint_only),
        "call to a proc renamed over via a namespace-relative rename must not fold, got {opts:?}",
    );
}

#[test]
fn parse_cmd_subst_head_rejects_metachars() {
    assert_eq!(parse_cmd_subst_head("::answer"), Some("::answer"));
    assert_eq!(parse_cmd_subst_head("::answer 1 2"), Some("::answer"));
    assert_eq!(parse_cmd_subst_head("  ::answer"), Some("::answer"));
    assert_eq!(parse_cmd_subst_head(""), None);
    // Leading `$` / `[` / `{` / `"` → would re-substitute.
    assert_eq!(parse_cmd_subst_head("$cmd"), None);
    assert_eq!(parse_cmd_subst_head("[cmd]"), None);
}
