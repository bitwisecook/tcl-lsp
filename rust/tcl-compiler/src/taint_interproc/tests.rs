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

use super::*;
use crate::compilation_unit::CompilationUnit;
use tcl_core_types::DiagCode;
use tcl_registry::CommandRegistry;

fn warnings(src: &str) -> Vec<crate::taint::TaintWarning> {
    let reg = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(src, &reg, false).with_interprocedural(&reg, None);
    crate::taint::find_taint_warnings_for_cu(&cu, &reg, None)
}

#[test]
fn dynamic_original_head_retains_all_actual_summary_dependencies() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = context.commands().profile().unwrap();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let source = "proc target {} {return VALUE}\nproc other {} {return OTHER}\nproc caller {command x} {$command $x}\n";
    let cu = CompilationUnit::build_with_analysis_input(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
    );
    assert!(cu.ir_module.source_entry.native_entry.is_none());
    let known = cu
        .ir_module
        .original_source_procedure_bodies()
        .map(|(id, _)| *id)
        .collect::<HashSet<_>>();
    assert_eq!(known.len(), 3);
    let caller = *cu
        .ir_module
        .original_native_declaration_body_units
        .iter()
        .find(|(_, card)| card.name() == Some("caller"))
        .unwrap()
        .0;
    let function = cu.function_for_body(Some(caller)).unwrap();
    assert!(
        function
            .cfg
            .blocks
            .values()
            .flat_map(|block| block.statements.iter())
            .any(|statement| {
                statement.tokens().is_some_and(|tokens| {
                    original_head(tokens).is_none()
                        && tokens
                            .words()
                            .first()
                            .is_some_and(|head| head.legacy_text().contains("$command"))
                })
            })
    );
    let actual = resolved_callees(&cu, Some(caller), &known, context.commands())
        .into_iter()
        .collect::<HashSet<_>>();
    assert_eq!(actual, known);
}

#[test]
fn taint_basis_lattice_matches_expected_colours() {
    // Mutation guard for the basis-name -> lattice mapping: pins each
    // `TaintBasis` variant to its exact expected `TaintColour` set.
    // Swapping which colour two variants map to (e.g. giving `Path`
    // `NON_DASH_PREFIXED` and `NonDash` `PATH_PREFIXED`) flips two of
    // these assertions and fails the test — the mapping can drift without
    // breaking exhaustiveness, so this is the net exhaustiveness alone
    // does not provide.
    let t = TaintColour::TAINTED;
    let expected: [(TaintBasis, TaintColour); 17] = [
        (TaintBasis::Generic, t),
        (TaintBasis::Path, t | TaintColour::PATH_PREFIXED),
        (TaintBasis::NonDash, t | TaintColour::NON_DASH_PREFIXED),
        (TaintBasis::CrlfFree, t | TaintColour::CRLF_FREE),
        (TaintBasis::ShellAtom, t | TaintColour::SHELL_ATOM),
        (TaintBasis::ListCanonical, t | TaintColour::LIST_CANONICAL),
        (TaintBasis::RegexLiteral, t | TaintColour::REGEX_LITERAL),
        (TaintBasis::PathNormalised, t | TaintColour::PATH_NORMALISED),
        (
            TaintBasis::PathBounded,
            t | TaintColour::PATH_NORMALISED | TaintColour::PATH_BOUNDED,
        ),
        (
            TaintBasis::HeaderTokenSafe,
            t | TaintColour::HEADER_TOKEN_SAFE,
        ),
        (TaintBasis::HtmlEscaped, t | TaintColour::HTML_ESCAPED),
        (TaintBasis::UrlEncoded, t | TaintColour::URL_ENCODED),
        (TaintBasis::Ip, t | TaintColour::IP_ADDRESS),
        (TaintBasis::Port, t | TaintColour::PORT),
        (TaintBasis::Fqdn, t | TaintColour::FQDN),
        (TaintBasis::PathJoined, t | TaintColour::PATH_JOINED),
        (TaintBasis::Channel, t | TaintColour::CHANNEL),
    ];
    assert_eq!(expected.len(), TaintBasis::ALL.len());
    for (basis, colour) in expected {
        assert_eq!(
            basis.lattice(),
            TaintLattice::from_colours(colour),
            "{basis:?} mapped to an unexpected lattice"
        );
    }
    // Every non-generic basis is pairwise distinct — a swap between two
    // bases (rather than a wrong constant) still fails via `intersects`
    // returning true for both instead of one.
    for i in 1..TaintBasis::ALL.len() {
        for j in (i + 1)..TaintBasis::ALL.len() {
            let a = TaintBasis::ALL[i].lattice();
            let b = TaintBasis::ALL[j].lattice();
            assert_ne!(
                a,
                b,
                "{:?} and {:?} must map to distinct lattices",
                TaintBasis::ALL[i],
                TaintBasis::ALL[j]
            );
        }
    }
}

#[test]
fn summary_dependencies_read_original_static_heads_under_source_grammar() {
    // naming.compiler.original-summary-dependency-source-heads
    // docs/design/analysis/name-resolution-proofs/compiler-original-summary-dependency-source-heads.md
    // Lexical memo dependencies supply no selected command or execution.
    let known: HashSet<_> = ["::foo", "::café", "::ns::local", "::unused"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    let config = tcl_lexer::LexerConfig::for_profile(tcl_dialect::DialectProfile::find("tcl8.6"));
    let dependencies = command_subst_callees_with_config(
        r#"return ["foo"] [{café}] [\u0066oo] [local]"#,
        "::ns::caller",
        &known,
        config,
    );
    assert_eq!(dependencies, ["::café", "::foo", "::ns::local"]);
    assert_eq!(
        command_subst_callees_with_config("return [foo; café]", "::ns::caller", &known, config),
        ["::café", "::foo"],
    );
    assert_eq!(
        command_subst_callees_with_config("return []", "::ns::caller", &known, config),
        [] as [std::string::String; 0]
    );
}

#[test]
fn summary_dependencies_keep_original_bracket_data_inert() {
    let known = HashSet::from(["::foo".to_owned(), "::unused".to_owned()]);
    let config = tcl_lexer::LexerConfig::for_profile(tcl_dialect::DialectProfile::find("tcl8.6"));
    assert_eq!(
        command_subst_callees_with_config(
            "while {$value} {incr value}",
            "::caller",
            &known,
            config,
        ),
        [] as [std::string::String; 0]
    );
    assert_eq!(
        command_subst_callees_with_config(
            "puts {[foo]}; return {[unused]}",
            "::caller",
            &known,
            config,
        ),
        [] as [std::string::String; 0]
    );
}

#[test]
fn summary_dependencies_keep_unresolved_and_dialect_residuals() {
    // naming.compiler.original-summary-dependency-source-heads
    // docs/design/analysis/name-resolution-proofs/compiler-original-summary-dependency-source-heads.md
    let known: HashSet<_> = ["::foo", "::other"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    let config = tcl_lexer::LexerConfig::for_profile(tcl_dialect::DialectProfile::find("tcl8.6"));
    for source in ["return [$head]", "return [installed_alias]", "return [foo"] {
        assert_eq!(
            command_subst_callees_with_config(source, "::caller", &known, config),
            ["::foo", "::other"],
            "unresolved source dependency: {source}",
        );
    }
    assert_eq!(
        command_subst_callees_with_config(r"return [\U00000066oo]", "::caller", &known, config),
        ["::foo"],
    );
    let older = tcl_lexer::LexerConfig::for_profile(tcl_dialect::DialectProfile::find("tcl8.4"));
    assert_eq!(
        command_subst_callees_with_config(r"return [\U00000066oo]", "::caller", &known, older),
        ["::foo", "::other"],
    );
}

#[test]
fn cross_proc_entry_taint_into_sink_warns() {
    // A tainted argument flowing into a proc parameter and then into a
    // sink inside that proc is reported (cross-proc entry-taint) — the
    // gap interprocedural taint solving closes.
    let src = "proc s {v} { eval $v }\nset x [gets stdin]\ns $x\n";
    let w = warnings(src);
    assert_eq!(w.len(), 1, "expected one cross-proc sink warning: {w:?}");
    assert_eq!(w[0].code, DiagCode::T100);
    assert_eq!(w[0].variable, "v");
    assert_eq!(w[0].sink_command, "eval");
}

#[test]
fn clean_argument_into_proc_does_not_warn() {
    // A clean (literal) argument flowing into the same proc must NOT warn.
    let src = "proc s {v} { eval $v }\ns hello\n";
    assert_eq!(warnings(src), [] as [crate::taint::TaintWarning; 0]);
}

#[test]
fn original_return_substitution_flows_arguments_into_the_actual_callee() {
    let source = "proc sink {argument} {eval $argument}\n\
            proc wrapper {value} {return [sink $value]}\n\
            set input [gets stdin]\nwrapper $input\n";
    let findings = warnings(source);
    assert!(
        findings.iter().any(|finding| {
            finding.code == DiagCode::T100
                && finding.variable == "argument"
                && finding.sink_command == "eval"
        }),
        "the original returned invocation must taint its actual callee's parameter: {findings:?}"
    );
}

#[test]
fn return_summary_carries_taint_through_callee() {
    // A proc returning its tainted parameter taints the caller's value,
    // which then reaches a sink: exercises the return-summary half.
    let src = "proc id {v} { return $v }\nset x [gets stdin]\neval [id $x]\n";
    let w = warnings(src);
    assert!(
        w.iter()
            .any(|w| w.code == DiagCode::T100 && w.sink_command == "eval"),
        "expected eval injection via return summary: {w:?}"
    );
}

#[test]
fn original_return_lookup_without_a_target_cannot_supply_a_clean_summary() {
    let (context, cu) = crate::interprocedural::logical_completion_unit(
        "proc wrapper {} {return [missing]}",
        "tcl8.6",
    );
    let body = *cu.ir_module.procedures.keys().next().unwrap();
    assert!(cu.procedures[&body].cfg.blocks.values().any(|block| {
        matches!(
            &block.terminator,
            Some(Terminator::Return {
                tokens: Some(_),
                value_word: Some(_),
                ..
            })
        )
    }));
    let summary = infer_proc_summary(
        &cu,
        body,
        context.commands(),
        cu.interproc.as_ref(),
        None,
        &HashSet::from([body]),
        &HashMap::new(),
    )
    .expect("actual original wrapper declaration");
    assert_eq!(summary.return_base, TaintLattice::tainted());
}

#[test]
fn source_summary_reborrows_formals_from_its_actual_module_body() {
    let (context, cu) = crate::interprocedural::logical_completion_unit(
        "proc defaulted {{value fallback} args} { return $value }",
        "tcl8.6",
    );
    let registry = context.commands();
    let body = *cu.ir_module.procedures.keys().next().unwrap();
    let formals = cu
        .ir_module
        .original_source_formal_topology(body, registry)
        .expect("the exact retained source procedure supplies its selected ParamList");
    assert_eq!(formals.len(), 2);
    let crate::signature_scan::formal_parameters::SourceProcedureFormalTopology::Logical(logical) =
        &formals
    else {
        panic!("Logical source procedure keeps its separate selected topology");
    };
    assert_eq!(logical.parameters()[0].name, "value");
    assert_eq!(logical.parameters()[0].default.as_deref(), Some("fallback"));
    assert_eq!(logical.parameters()[1].name, "args");
    assert_eq!(formals.argument_count_shape().minimum, 0);
    assert_eq!(formals.argument_count_shape().maximum, None);

    let summary = infer_proc_summary(
        &cu,
        body,
        registry,
        cu.interproc.as_ref(),
        None,
        &HashSet::from([body]),
        &HashMap::new(),
    )
    .expect("the Module-owned procedure summary is available");
    assert_eq!(summary.source_formals.as_ref(), Some(&formals));
    assert!(summary.source_formals_required);
    assert_eq!(
        apply_proc_return_summary(&summary, &[TaintLattice::tainted(), TaintLattice::clean()],),
        TaintLattice::tainted(),
        "the actual first formal carries its tainted argument"
    );

    let mut foreign = cu.clone();
    foreign.ir_module.procedures.get_mut(&body).unwrap().params[0] =
        "renamed_without_source_change".to_owned();
    assert!(
        foreign
            .ir_module
            .original_source_formal_topology(body, registry)
            .is_none(),
        "a changed body projection cannot reuse the retained source ParamList"
    );
}

#[test]
fn self_recursive_proc_taint_summary_converges() {
    // A directly self-recursive proc — `fib` calling itself inside
    // `[expr {[fib …]}]` — has no self-call edge in `direct_calls` (it is
    // buried in a braced `expr`). A worklist that does not re-queue a proc
    // on its own summary change therefore under-converges the interproc
    // taint fixpoint, the debug-only convergence guard panics, and the LSP
    // diagnostic worker publishes nothing. Re-queueing on a proc's own
    // change is what lets the fixpoint settle and the guard hold.
    let reg = CommandRegistry::build_default();
    let src = "proc fib {n} {\n    if {$n < 2} { return $n }\n    return [expr {[fib [expr {$n - 1}]] + [fib [expr {$n - 2}]]}]\n}\nputs \"fib(10) = [fib 10]\"\n";
    let cu = CompilationUnit::build_for(src, &reg, false).with_interprocedural(&reg, None);
    // Exercises `converge_summaries_with` and its debug fixpoint guard.
    let _ = solve_interprocedural_taints(&cu, &reg, None);
    // The full taint pass over the same source must also complete cleanly.
    let _ = warnings(src);
}

fn fixture_body(cu: &CompilationUnit, display: &str) -> ProcedureId {
    *cu.ir_module
        .procedures
        .unique_display(display)
        .expect("fixture has one independently retained declaration")
        .0
}

fn summaries_for_unit(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
) -> HashMap<ProcedureId, ProcTaintSummary> {
    converge_summaries_with(
        cu,
        registry,
        cu.interproc.as_ref(),
        None,
        &mut |unit, id, interproc, known, summaries| {
            infer_proc_summary(unit, id, registry, interproc, None, known, summaries)
                .expect("actual fixture body")
        },
    )
}

fn command_subst_callees_with_config(
    body: &str,
    caller: &str,
    known: &HashSet<String>,
    config: tcl_lexer::LexerConfig,
) -> Vec<String> {
    use std::fmt::Write as _;
    let mut declarations: Vec<_> = known.iter().collect();
    declarations.sort();
    let mut source = String::from("namespace eval ::ns {}\n");
    for name in declarations {
        let _ = writeln!(source, "proc {name} {{}} {{}}");
    }
    let _ = writeln!(source, "proc {caller} {{}} {{{body}}}");
    let registry = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for_with_config(&source, &registry, false, config);
    let known_ids: HashSet<_> = known
        .iter()
        .filter_map(|name| {
            cu.ir_module
                .procedures
                .unique_display(name)
                .map(|(id, _)| *id)
        })
        .collect();
    assert_eq!(
        known_ids.len(),
        known.len(),
        "every dependency fixture retains its actual declaration"
    );
    let ids = if let Some((id, _)) = cu.ir_module.procedures.unique_display(caller) {
        command_subst_callees(&cu, *id, &known_ids, &registry)
    } else {
        known_ids.iter().copied().collect()
    };
    let mut displays: Vec<_> = ids
        .into_iter()
        .map(|id| {
            cu.ir_module
                .procedures
                .get(&id)
                .unwrap()
                .qualified_name
                .clone()
        })
        .collect();
    displays.sort();
    displays
}

/// Build the whole-module summary map the way the solver does, so a test
/// can compare summaries rather than only the diagnostics they produce.
fn summaries_for(src: &str) -> HashMap<String, ProcTaintSummary> {
    let reg = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(src, &reg, false).with_interprocedural(&reg, None);
    let mut shown = HashMap::new();
    for (id, summary) in summaries_for_unit(&cu, &reg) {
        let body = cu
            .ir_module
            .procedures
            .get(&id)
            .expect("actual taint fixture body");
        assert!(
            shown.insert(body.qualified_name.clone(), summary).is_none(),
            "authored taint fixture labels must be unique"
        );
    }
    shown
}

#[test]
fn summary_scenarios_reuse_one_instance_class_solve() {
    // `id` requires the clean solve plus every taint-basis scenario for
    // its live parameter. Receiver typing is independent of those seeds,
    // so it must be solved once for the whole summary, not once per
    // propagation run (the old cost was 1 + TaintBasis::ALL.len()).
    let reg = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let cu = CompilationUnit::build_for("proc id {v} { return $v }\n", &reg, false)
        .with_interprocedural(&reg, None);
    let interproc = cu.interproc.as_ref();
    let id = fixture_body(&cu, "::id");
    let known = HashSet::from([id]);
    crate::taint::reset_instance_class_solve_count();

    let summary = infer_proc_summary(&cu, id, &reg, interproc, None, &known, &HashMap::new())
        .expect("actual fixture body");

    assert_eq!(
        summary.return_by_param_basis[0].1.len(),
        TaintBasis::ALL.len()
    );
    assert_eq!(crate::taint::instance_class_solve_count(), 1);
}

/// The unpruned reference: re-derive one procedure's summary without the
/// prunes, by seeding every
/// `(parameter, basis)` pair with a full dataflow solve.  A test can then
/// assert the pruned result is bit-identical rather than merely plausible.
fn summary_without_prunes(src: &str, target: &str) -> ProcTaintSummary {
    let reg = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(src, &reg, false).with_interprocedural(&reg, None);
    let interproc = cu.interproc.as_ref();
    let summaries = summaries_for_unit(&cu, &reg);
    let known: HashSet<_> = summaries.keys().copied().collect();
    let id = fixture_body(&cu, target);
    let proc = cu.ir_module.procedures.get(&id).unwrap();
    let fu = &cu.procedures[&id];

    let graph = TaintGraph::new(&fu.cfg, &fu.ssa, &fu.sccp);
    let instance_classes = local_instance_classes(&fu.cfg, &reg);
    let base = run_propagation(
        &graph,
        fu,
        &instance_classes,
        &reg,
        interproc,
        TaintSourceContext::for_module_function(&reg, &cu.ir_module, fu),
        None,
        &summaries,
    );
    let ctx = return_ctx(
        fu,
        &reg,
        interproc,
        TaintSourceContext::for_module_function(&reg, &cu.ir_module, fu),
        &known,
        &summaries,
        &instance_classes,
    );
    let return_base = collect_return_taint(fu, &base, ctx);
    let mut by_param_basis = Vec::new();
    for param in &proc.params {
        let mut values = Vec::new();
        for basis in TaintBasis::ALL {
            let mut seed = HashMap::new();
            seed.insert(param.clone(), basis.lattice());
            let seeded = run_propagation(
                &graph,
                fu,
                &instance_classes,
                &reg,
                interproc,
                TaintSourceContext::for_module_function(&reg, &cu.ir_module, fu),
                Some(&seed),
                &summaries,
            );
            values.push(collect_return_taint(fu, &seeded, ctx));
        }
        by_param_basis.push((param.clone(), values));
    }
    ProcTaintSummary {
        qualified_name: target.to_owned(),
        params: proc.params.clone(),
        arity: crate::interprocedural::arity_from_names(&proc.params),
        return_base,
        return_by_param_basis: by_param_basis,
        source_formals: None,
        source_formals_required: false,
    }
}

/// Assert every summary the solver produces for `src` is bit-identical to
/// the unpruned computation.
fn prunes_are_bit_identical(src: &str) {
    let pruned = summaries_for(src);
    assert!(!pruned.is_empty(), "fixture defines no procedures");
    for (qname, summary) in &pruned {
        let reference = summary_without_prunes(src, qname);
        assert_eq!(
            *summary, reference,
            "pruned summary for `{qname}` differs from the unpruned solve"
        );
    }
}

#[test]
fn prune_keeps_summaries_bit_identical_across_the_matrix() {
    // TP — direct and transitive passthrough, several parameters, and a
    // basis-bearing (sanitised) path, so the pruned and unpruned solves
    // must agree on non-trivial colour values, not just on `clean`.
    prunes_are_bit_identical("proc id {v} { return $v }\n");
    prunes_are_bit_identical("proc pick {a b c} { return $b }\n");
    prunes_are_bit_identical(
        "proc inner {v} { return $v }\nproc outer {v} { return [inner $v] }\n",
    );
    prunes_are_bit_identical("proc norm {p} { return [file normalize $p] }\n");
    prunes_are_bit_identical("proc joined {a b} { return \"$a/$b\" }\n");
    // `args` — the variadic binding the summary applier special-cases.
    prunes_are_bit_identical("proc varargs {a args} { return $args }\n");
    // FP — a sanitised value, and parameters that never reach the return.
    prunes_are_bit_identical("proc clean {v} { return [string length $v] }\n");
    prunes_are_bit_identical("proc unused {a b} { return literal }\n");
    prunes_are_bit_identical("proc partly {a b} { set t $a\n return $t }\n");
    // TN — no return value at all, and a dynamic/unresolved call.
    prunes_are_bit_identical("proc silent {v} { puts $v }\n");
    prunes_are_bit_identical("proc dynamic {v} { return [$v run] }\n");
    // FN — a callee reached only through a nested command substitution,
    // and mutual recursion (an SCC the worklist must settle).
    prunes_are_bit_identical(
        "proc leaf {v} { return $v }\nproc nest {t v} { return [lindex [leaf $v] 0] }\n",
    );
    prunes_are_bit_identical(
        "proc ping {n} { if {$n < 1} { return $n }\n return [pong $n] }\n\
             proc pong {n} { return [ping $n] }\n",
    );
}

#[test]
fn opaque_native_call_cannot_produce_a_clean_constant_return_summary() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut unit = crate::compilation_unit::CompilationUnit::build_for(
        "proc p {value} {return LITERAL}",
        registry,
        false,
    );
    let id = fixture_body(&unit, "::p");
    let function = unit.procedures.get_mut(&id).unwrap();
    function
        .cfg
        .blocks
        .get_mut(&function.cfg.entry)
        .unwrap()
        .statements
        .push(crate::ir::native_call_for_test(b"opaque \xff"));
    let summary = infer_proc_summary(
        &unit,
        id,
        registry,
        None,
        registry.profile(),
        &HashSet::from([id]),
        &HashMap::new(),
    )
    .expect("actual fixture body");
    assert_eq!(summary.return_base, TaintLattice::tainted());
    assert_eq!(
        apply_proc_return_summary(&summary, &[TaintLattice::clean()]),
        TaintLattice::tainted()
    );
}

#[test]
fn constant_return_prune_yields_the_untainted_summary() {
    // A procedure whose every executable return value is substitution-free
    // cannot carry taint out, so the whole summary is the clean one — with
    // no dataflow solve run at all.
    let summaries = summaries_for("proc lit {a b} { return ok }\n");
    let s = summaries.get("::lit").expect("::lit summarised");
    assert_eq!(*s, ProcTaintSummary::untainted("::lit", &s.params));
    assert_eq!(s.params, vec!["a".to_owned(), "b".to_owned()]);
}

#[test]
fn unread_parameter_scenarios_equal_the_base_return() {
    // `b` is never read, so seeding it cannot change the return: every one
    // of its 17 basis scenarios is exactly `return_base`.
    let summaries = summaries_for("proc half {a b} { return [string cat $a x] }\n");
    let s = summaries.get("::half").expect("::half summarised");
    let (name, values) = s
        .return_by_param_basis
        .iter()
        .find(|(p, _)| p == "b")
        .expect("parameter b recorded");
    assert_eq!(name, "b");
    assert_eq!(values.len(), TaintBasis::ALL.len());
    assert!(
        values.iter().all(|v| *v == s.return_base),
        "unread parameter must reproduce the base return, got {values:?}"
    );
}

#[test]
fn pruning_preserves_cross_proc_taint_diagnostics() {
    // The end-to-end guarantee: the diagnostics a user sees are unchanged.
    // A passthrough that must still warn…
    let w = warnings("proc id {v} { return $v }\nset x [gets stdin]\neval [id $x]\n");
    assert!(
        w.iter()
            .any(|w| w.code == DiagCode::T100 && w.sink_command == "eval"),
        "passthrough taint must survive the prunes: {w:?}"
    );
    // …and a procedure the constant-return prune skips *entirely* (zero
    // solves) must still produce exactly what the unpruned solver did.
    // The original `eval` operand contains only the selected procedure's
    // clean result. An input read inside that procedure call cannot supply
    // bytes to the outer sink independently of the returned value.
    let src = "proc lit {v} { return safe }\nset x [gets stdin]\neval [lit $x]\n";
    let w = warnings(src);
    assert!(w.is_empty(), "constant result must remain clean: {w:?}");
    let w = warnings("proc lit {v} { return safe }\nset x [gets stdin]\neval [lit $x]$x\n");
    assert_eq!(w.len(), 1, "{w:?}");
    assert_eq!(w[0].code, DiagCode::T100);
    assert_eq!(w[0].variable, "x");
    // The summary itself is the clean one — the prune's actual claim.
    let summaries = summaries_for(src);
    let lit = summaries.get("::lit").expect("::lit summarised");
    assert_eq!(*lit, ProcTaintSummary::untainted("::lit", &lit.params));
}

#[test]
fn untainted_param_summary_is_clean() {
    let reg = CommandRegistry::build_default();
    let summary = ProcTaintSummary::untainted("::p", &["a".to_owned(), "b".to_owned()]);
    let _ = &reg;
    // Untainted args → untainted return.
    let out = apply_proc_return_summary(&summary, &[TaintLattice::clean(), TaintLattice::clean()]);
    assert!(!out.is_tainted());
    // Wrong arity → untainted.
    let out = apply_proc_return_summary(&summary, &[TaintLattice::tainted()]);
    assert!(!out.is_tainted());
}
