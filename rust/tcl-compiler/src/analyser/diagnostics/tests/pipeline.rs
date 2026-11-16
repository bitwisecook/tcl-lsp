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

//! The diagnostics pipeline itself: the per-unit dispatcher and its memo, de-duplication and disabled codes.

use super::*;

#[test]
fn emit_variable_usage_diagnostics_is_a_noop() {
    // Hook is intentionally empty — running it must leave
    // the diagnostics list untouched.
    let mut a = Analyser::new();
    a.result
        .diagnostics
        .push(diag(DiagCode::W113, Span::new(0, 3), "x"));
    a.emit_variable_usage_diagnostics();
    assert_eq!(a.result.diagnostics.len(), 1);
}

#[test]
fn emit_cfg_ssa_diagnostics_runs_without_panicking_on_empty_source() {
    // Smoke test — the orchestrator handles empty input
    // gracefully (an empty CompilationUnit yields no
    // diagnostics).
    let mut a = Analyser::new();
    a.emit_document_cfg_ssa_diagnostics_for_test("");
    assert_eq!(
        a.result.diagnostics,
        [] as [crate::analyser::types::Diagnostic; 0]
    );
}

/// A memoised `CompilationUnit` (built via `build_for_memoized`
/// and fed through the `cu_override` seam) must yield **byte-identical**
/// diagnostics to the whole-file path — both on a cold cache (all misses,
/// proving the refactor) and a warm cache (all hits, proving the cache key
/// captures every lattice input).
const MEMOIZED_DIAGNOSTIC_SNIPPETS: [&str; 6] = [
    "proc a {x} {\n  if {$x} { set y 1 }\n  return $y\n}\nproc b {} { a 1 }\n",
    "set g 0\nproc inc {} { global g; incr g }\ninc\nputs $g\n",
    "proc f {n} {\n  set acc 0\n  for {set i 0} {$i < $n} {incr i} { set acc [expr {$acc + $i}] }\n  return $acc\n}\nproc g {} { set z 9; set z 10; return $z }\n",
    "namespace eval n {\n  proc p {a} { set b $a; return $b }\n}\nset r [n::p 3]\n",
    "oo::class create K {\n  method m {a} { set n $a; return $n }\n}\nproc top {} { set q 1; set q 2 }\n",
    // Constructor object typing through the memo: `set o [K new]` types
    // `o` as OBJECT(::K), so `$o gone` is validated (W308). Exercises
    // the `known_classes` thread on `LatticeRequest` + the memo key.
    "oo::class create K {\n  method m {} { return 1 }\n}\nproc top {} { set o [K new]; $o gone }\n",
];

fn memo_diagnostic_input() -> crate::analyser::ResolvedAnalysisInput {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_known_environment("tcl8.6")
            .expect("known Logical fixture metadata")
            .default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    )
}

#[derive(Default)]
struct MemoDiagnosticWorkerCache {
    entries: Vec<(
        crate::body_analysis_capsule::BodyLatticeInputs,
        crate::compilation_unit::FunctionUnit,
    )>,
    validated_hits: usize,
    rebuilds: usize,
}

impl MemoDiagnosticWorkerCache {
    pub(super) fn function(
        &mut self,
        request: &crate::compilation_unit::LatticeRequest<'_>,
        registry: &tcl_registry::CommandRegistry,
    ) -> crate::compilation_unit::FunctionUnit {
        use crate::body_analysis_capsule::{BodyLatticeCommandBindings, BodyLatticeInputs};
        let body = BodyLatticeInputs::retain_original_worker_request(request, registry)
            .expect("the actual current declaration issued this lattice request");
        if let Some((previous, cached)) = self
            .entries
            .iter()
            .find(|(_, cached)| cached.name == request.qname)
        {
            let mut current = cached.clone();
            if body.rebind_original_worker_result(previous, cached, &mut current) {
                self.validated_hits += 1;
                return current;
            }
        }
        let bindings = BodyLatticeCommandBindings::from(request.command_bindings.clone());
        let metadata = request.original_metadata_context(registry);
        let trust = request.command_trust.snapshot();
        let inputs = crate::compilation_unit::OriginalFunctionLatticeInputs {
            qname: request.qname,
            params: request.params,
            upvar_procs: request.upvar_procs,
            proc_params: request.proc_params,
            global_write_procs: request.global_write_procs,
            command_bindings: &bindings,
            procedure_summary_owners: request.procedure_summary_owners,
            lexer_config: request.lexer_config,
            profile: request.dialect.map(tcl_dialect::DialectProfile::cache_key),
            dialect: request.dialect.map_or("", |profile| profile.name),
            param_constants: request.param_constants,
            known_classes: request.known_classes,
            traced_variables: request.traced_variables,
            has_dynamic_variable_trace: request.has_dynamic_variable_trace,
            original_declaration_body: request
                .original_source_procedure()
                .filter(|procedure| {
                    procedure.body_kind == crate::ir::ProcedureBodyKind::OriginalDeclaration
                })
                .map(|_| request.body_id),
            plain_command_dispatch: request.plain_command_dispatch,
            metadata_context: &metadata,
            source_metadata_input: request.source_metadata_input,
            event_body: request.irules_event_body,
            command_trust: &trust,
            analysis_context: request.analysis_context,
        };
        let unit = body
            .original_function_lattice(&inputs, registry)
            .expect("the complete original worker accepts its unchanged request");
        self.rebuilds += 1;
        self.entries
            .retain(|(_, cached)| cached.name != request.qname);
        self.entries.push((body, unit.clone()));
        unit
    }
}

fn memo_diagnostic_unit(
    source: &str,
    input: &crate::analyser::ResolvedAnalysisInput,
    settings: &std::sync::Arc<crate::analyser::OriginalAnalyserSourceSettings>,
    cache: &mut MemoDiagnosticWorkerCache,
) -> crate::compilation_unit::CompilationUnit {
    let context = input.context_registry();
    let registry = context.commands();
    let body_config = input.lexer_config().normalized();
    let body = |text: &str, namespace: &str| {
        crate::lowering::OriginalBodyLoweringTemplate::lower(
            text,
            namespace,
            registry,
            body_config,
            Some(input.unit_profile()),
        )
    };
    let syntax = |request: &crate::lowering::SourceBodySyntaxRequest| request.lower_neutral();
    let mut lattice =
        |request: &crate::compilation_unit::LatticeRequest<'_>| cache.function(request, registry);
    let unit = crate::compilation_unit::CompilationUnit::build_memoized_with_original_body_lowering_and_source_settings(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: input.lexer_config(),
            dialect: Some(input.unit_profile()),
            external_call_sites: None,
            declared_commands: None,
        },
        &mut lattice,
        &body,
        &syntax,
        None,
        (input, settings),
    );
    assert!(unit.original_lattice_callbacks_are_current());
    assert_eq!(unit.ir_module.source_metadata_input.as_ref(), Some(input));
    unit.with_interprocedural(registry, Some(input.unit_profile()))
}

fn analyse_memo_diagnostic_unit(
    source: &str,
    input: &crate::analyser::ResolvedAnalysisInput,
    cache: &mut MemoDiagnosticWorkerCache,
) -> crate::analyser::AnalysisResult {
    let mut analyser = Analyser::new().with_resolved_input(input.clone());
    let settings = analyser.source_compilation_settings();
    let unit = memo_diagnostic_unit(source, input, &settings, cache);
    analyser.set_cu_override(std::sync::Arc::new(unit));
    analyser.analyse(source, "tcl")
}

#[test]
fn memoized_compilation_unit_diagnostics_match_whole_file() {
    let input = memo_diagnostic_input();
    for source in MEMOIZED_DIAGNOSTIC_SNIPPETS {
        let want = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "tcl");
        let mut cache = MemoDiagnosticWorkerCache::default();
        let cold = analyse_memo_diagnostic_unit(source, &input, &mut cache);
        assert!(
            want.diagnostics == cold.diagnostics,
            "cold diagnostic equality: {source}"
        );
        assert!(
            !cache.entries.is_empty(),
            "no issued lattice requests: {source}"
        );
        let entries = cache.entries.len();
        let callbacks = cache.validated_hits + cache.rebuilds;
        let warm = analyse_memo_diagnostic_unit(source, &input, &mut cache);
        assert!(
            want.diagnostics == warm.diagnostics,
            "warm diagnostic equality: {source}"
        );
        assert_eq!(cache.entries.len(), entries);
        assert!(cache.validated_hits + cache.rebuilds > callbacks);
    }
}

#[test]
fn memoized_compilation_unit_shift_correctness() {
    let input = memo_diagnostic_input();
    let bodies = "proc a {} { return $undef }\n\
             proc b {} { set y 1; set y 2; return $y }\n\
             proc c {x} {\n  if {$x} { set z 1; set z 2 }\n  for {set i 0} {$i < 3} {incr i} { set w $q }\n  return $z\n}\n\
             proc d {n} {\n  while {$n} { catch { set r $undef2 } }\n  switch -- $n { 1 { set s 1; set s 2 } default { return $missing } }\n}\n";
    let shifted = format!("set top 0\nset top2 1\n# a comment line\n{bodies}");
    let mut cache = MemoDiagnosticWorkerCache::default();
    let _ = analyse_memo_diagnostic_unit(bodies, &input, &mut cache);
    let entries = cache.entries.len();
    let callbacks = cache.validated_hits + cache.rebuilds;
    let got = analyse_memo_diagnostic_unit(&shifted, &input, &mut cache);
    let want = Analyser::new()
        .with_resolved_input(input)
        .analyse(&shifted, "tcl");
    assert!(
        want.diagnostics == got.diagnostics,
        "shifted current-source diagnostic equality"
    );
    assert_eq!(cache.entries.len(), entries);
    assert!(cache.validated_hits + cache.rebuilds > callbacks);
    assert!(
        !want.diagnostics.is_empty(),
        "exercise positioned diagnostics"
    );
    // A Logical declaration does not certify source-independent neutral reuse.
    // A shifted request must pass the original worker rebind or rebuild; entry
    // count alone is never treated as a cache hit.
}

#[test]
fn logical_for_condition_keeps_its_used_parameter_reference() {
    let source = MEMOIZED_DIAGNOSTIC_SNIPPETS[2];
    let input = memo_diagnostic_input();
    let whole = Analyser::new()
        .with_resolved_input(input.clone())
        .analyse(source, "tcl");
    let mut cache = MemoDiagnosticWorkerCache::default();
    let cold = analyse_memo_diagnostic_unit(source, &input, &mut cache);
    let warm = analyse_memo_diagnostic_unit(source, &input, &mut cache);
    for (kind, result) in [("whole", whole), ("cold", cold), ("warm", warm)] {
        assert!(
            !result.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == DiagCode::W214 && diagnostic.message.contains("Parameter 'n'")
            }),
            "{kind}: the original for condition references n",
        );
    }
}

#[test]
fn dedupe_drops_exact_duplicates() {
    // Same code + span + message + severity → kept once.
    let mut a = Analyser::new();
    a.source = "set x 1".to_string().into();
    a.result
        .diagnostics
        .push(diag(DiagCode::W210, Span::new(4, 5), "x not set"));
    a.result
        .diagnostics
        .push(diag(DiagCode::W210, Span::new(4, 5), "x not set"));
    a.dedupe_diagnostics();
    assert_eq!(a.result.diagnostics.len(), 1);
}

#[test]
fn dedupe_keeps_distinct_diagnostics_at_different_spans() {
    let mut a = Analyser::new();
    a.source = "set x 1\nset y 2".to_string().into();
    a.result
        .diagnostics
        .push(diag(DiagCode::W210, Span::new(4, 5), "x"));
    a.result
        .diagnostics
        .push(diag(DiagCode::W210, Span::new(12, 13), "y"));
    a.dedupe_diagnostics();
    assert_eq!(a.result.diagnostics.len(), 2);
}

#[test]
fn dedupe_drops_e002_on_e101_line() {
    // E101 fires on a line; any E002 on the same line is
    // a false positive (arity check confused by the
    // recovered switch) and gets dropped.
    let mut a = Analyser::new();
    a.source = "switch $x { foo {puts foo}".to_string().into();
    let switch_span = Span::new(0, 6);
    a.result
        .diagnostics
        .push(diag(DiagCode::E101, switch_span, "missing open brace"));
    a.result
        .diagnostics
        .push(diag(DiagCode::E002, switch_span, "too few args"));
    a.dedupe_diagnostics();
    assert!(
        a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::E101)
    );
    assert!(
        !a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::E002)
    );
}

#[test]
fn dedupe_keeps_e002_on_unrelated_line() {
    // E101 on line 0, E002 on line 1 — different lines, so
    // the suppression rule doesn't fire.
    let mut a = Analyser::new();
    a.source = "switch $x {\nset y 1".to_string().into();
    a.result
        .diagnostics
        .push(diag(DiagCode::E101, Span::new(0, 6), "missing brace"));
    a.result
        .diagnostics
        .push(diag(DiagCode::E002, Span::new(12, 15), "too few args"));
    a.dedupe_diagnostics();
    assert!(
        a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::E002)
    );
}

#[test]
fn apply_disabled_diagnostics_removes_listed_codes() {
    let mut a =
        Analyser::with_disabled_diagnostics(["W113"].iter().map(|s| (*s).to_string()).collect());
    a.result
        .diagnostics
        .push(diag(DiagCode::W113, Span::new(0, 3), "shadows"));
    a.result
        .diagnostics
        .push(diag(DiagCode::W210, Span::new(0, 3), "unset"));
    a.apply_disabled_diagnostics();
    assert!(
        !a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W113)
    );
    assert!(
        a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W210)
    );
}

#[test]
fn apply_disabled_diagnostics_no_op_when_empty() {
    let mut a = Analyser::new();
    a.result
        .diagnostics
        .push(diag(DiagCode::W113, Span::new(0, 3), "x"));
    a.apply_disabled_diagnostics();
    assert_eq!(a.result.diagnostics.len(), 1);
}
