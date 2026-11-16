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

//! Stored-callback replay: registration points, list builders, batching, name search and the mapping back to original offsets.

use super::*;

/// The fixture driver supplies Tk 8.6.13 before analysing registrations.
/// Its `tkWindow.c` entry/bind rows use `Tcl_CreateObjCommand`; neither
/// installs a compiler hook. This is actual loader provenance, separately
/// from the callback metadata queried by taint analysis.
fn loaded_tk_callback_unit(
    source: &str,
) -> (crate::compilation_unit::CompilationUnit, CommandRegistry) {
    let registry = CommandRegistry::build_default().project_for_profile(
        tcl_dialect::DialectProfile::find("tcl8.6").expect("selected native profile"),
    );
    let mut entry =
        crate::provider_fixtures::entry(&registry, &[crate::provider_fixtures::Provider::Tk])
            .as_ref()
            .clone();
    let loader = entry
        .trusted_package_loaders
        .first_mut()
        .expect("Tk provider");
    for command in ["entry", "bind"] {
        loader.compiler_hooks.insert(
            command.to_owned(),
            tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Absent,
        );
    }
    // The entry carries the document's complete source input, as an
    // analysed document's does; without it callback replay has no source
    // metadata to resolve against and correctly abstains.
    let profile = registry.profile().expect("selected native profile");
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let context = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for_profile(profile)
            .with_command_store(registry.snapshot().shared_registry()),
    );
    let input = crate::analyser::ResolvedAnalysisInput::new(profile, profile, context, config);
    entry.metadata_context =
        crate::registry_invocation::OwnedInvocationMetadataContext::for_source_input(Some(&input));
    let source = format!("package require Tk\n{source}");
    let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
        &source,
        crate::compilation_unit::UnitBuildOptions {
            registry: &registry,
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        &input,
    )
    .with_interprocedural(&registry, None);
    (unit, registry)
}

#[test]
fn command_substitution_callbacks_stop_before_replay_when_original_owner_is_refused() {
    // naming.source.retained-positioned-command-realm
    // docs/design/analysis/name-resolution-proofs/retained-positioned-command-realm.md
    // The loaded fixture supplies Tk provenance. These are compiler replay
    // controls, not claims about executing an external Tk event.
    let (unit, registry) = loaded_tk_callback_unit(
        "proc helper {value} {eval $value}\nentry .entry -validatecommand [list helper %P]",
    );
    reset_callback_replay_build_count();
    reset_callback_replay_name_search_count();
    let positive = find_callback_substitution_warnings(&unit, &registry, None, &[]);
    assert!(
        positive
            .iter()
            .any(|warning| warning.code == DiagCode::T100),
        "the owned list-prefix callback remains replayable: {positive:?}"
    );
    assert_eq!(callback_replay_build_count(), 1);
    assert_eq!(callback_replay_name_search_count(), 1);
    let mut missing = unit.clone();
    missing.ir_module.retained_source_bindings = None;
    let mut changed = unit.clone();
    changed.ir_module.source =
        tcl_lexer::SourceImage::document("entry .entry -validatecommand {};");
    for refused in vec![missing, changed].into_boxed_slice() {
        reset_callback_replay_build_count();
        reset_callback_replay_name_search_count();
        assert_eq!(
            find_callback_substitution_warnings(&refused, &registry, None, &[]),
            [] as [crate::taint::TaintWarning; 0]
        );
        assert_eq!(callback_replay_build_count(), 0);
        assert_eq!(callback_replay_name_search_count(), 0);
    }
}

#[test]
fn callback_list_replay_uses_the_selected_captured_builder_prefix() {
    let source = "interp alias {} mylist {} list helper\n\
                      proc helper {value} {eval $value}\n\
                      entry .entry -validatecommand [mylist %P]";
    let (unit, registry) = loaded_tk_callback_unit(source);
    let warnings = find_callback_substitution_warnings(&unit, &registry, None, &[]);
    assert!(
        warnings.iter().any(|warning| {
            warning.code == DiagCode::T100
                && warning.sink_command == "eval"
                && warning.variable == "%P"
        }),
        "the selected effective list result retains its captured `helper` prefix: {warnings:?}"
    );
}

#[test]
fn callback_list_replay_uses_the_selected_static_expansion_owner() {
    let source = "interp alias {} mylist {} list helper\n\
                      proc helper {value} {eval $value}\n\
                      entry .entry -validatecommand [mylist {*}{%P}]";
    let (unit, registry) = loaded_tk_callback_unit(source);
    let warnings = find_callback_substitution_warnings(&unit, &registry, None, &[]);
    assert!(
        warnings.iter().any(|warning| {
            warning.code == DiagCode::T100
                && warning.sink_command == "eval"
                && warning.variable == "%P"
        }),
        "the retained KnownExpansion operand preserves the selected list result: {warnings:?}"
    );
}

/// A replay is placed before the statement that registers it, so it is
/// analysed in that statement's command world; a later dynamic
/// registration cannot hide its definition. Tk 8.4–9.1 `-validatecommand`.
#[test]
fn callback_replay_is_defined_and_analysed_at_its_registration_point() {
    let source = "set before 1\n\
                      entry .one -validatecommand {eval %P}\n\
                      entry .two -validatecommand [gets stdin]\n\
                      set after 2";
    let (cu, registry) = loaded_tk_callback_unit(source);
    let mut candidates = Vec::new();
    collect_callback_replay_candidates(
        &cu,
        &registry,
        None,
        cu.ir_module.lexer_config,
        &mut candidates,
    );
    assert_eq!(
        candidates.len(),
        1,
        "only the static callback is replayable"
    );
    let candidate = &candidates[0];
    let replay = callback_replay_source(
        &candidate.callback,
        candidate.inputs,
        candidate.source_is_command_substitution,
        "input",
        "::replay_0",
        cu.ir_module.lexer_config,
        candidate.selected_builder.as_ref(),
    )
    .expect("static callback");
    let definition = replay.definition.clone();
    let (injected, spans, insertions) = insert_callback_replays(
        &cu.source,
        &cu.ir_module.top_level.statements,
        &[(replay, candidate.callback_span, String::new())],
    );
    let registration = cu
        .source
        .find("entry .one")
        .expect("registration statement");
    let before = &cu.source[..registration];
    assert_eq!(
        injected,
        format!("{before}{definition}\n{}", &cu.source[registration..])
    );
    let (synthetic, callback_span, _) = &spans[0];
    assert_eq!(
        &injected[synthetic.start() as usize..synthetic.end() as usize],
        definition
    );
    assert_eq!(*callback_span, candidate.callback_span);
    let after = u32::try_from(injected.find("set after").unwrap()).unwrap();
    let original_after = u32::try_from(cu.source.find("set after").unwrap()).unwrap();
    assert_eq!(
        insertions.original_span(Span::new(after, after + 9)),
        Some(Span::new(original_after, original_after + 9))
    );

    let replay_cu = callback_replay_unit(&cu, &registry, None, &injected).expect("source input");
    let replay_unit = replay_cu
        .analysable_body_function_units()
        .find(|fu| fu.name == "::replay_0")
        .expect("the replay definition is a procedure of the replay unit");
    assert!(
        replay_unit
            .cfg
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .any(|stmt| matches!(stmt, Statement::Call { command, .. } if command == "eval")),
        "the replayed callback body is lowered inside its own procedure"
    );
}

#[test]
fn callback_replays_batch_into_one_extra_interprocedural_build() {
    let source = "entry .one -validatecommand {eval %P}\n\
                      entry .two -validatecommand {eval %S}\n\
                      bind .two <Key> {eval %A}";
    let (cu, registry) = loaded_tk_callback_unit(source);
    reset_callback_replay_build_count();
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert_eq!(
        callback_replay_build_count(),
        1,
        "every statically recoverable callback must share one replay solve"
    );
    assert_eq!(
        warnings.len(),
        3,
        "each registered sink still warns: {warnings:?}"
    );
}

#[test]
fn callback_replay_batches_past_the_first_batch_without_truncating_later_sinks() {
    use std::fmt::Write as _;

    let mut source = String::new();
    for index in 0..=MAX_CALLBACK_TAINT_REPLAYS {
        writeln!(
            &mut source,
            "entry .benign{index} -validatecommand {{set value %P}}"
        )
        .expect("write to String");
    }
    source.push_str("entry .malicious -validatecommand {eval %P}");
    let (cu, registry) = loaded_tk_callback_unit(&source);
    let malicious_start = u32::try_from(
        cu.source
            .rfind("{eval %P}")
            .expect("malicious callback is present"),
    )
    .expect("test source fits in u32");

    reset_callback_replay_build_count();
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);

    assert_eq!(
        callback_replay_build_count(),
        2,
        "callbacks beyond one replay batch must receive a second bounded solve"
    );
    assert!(
        warnings.iter().any(|warning| {
            warning.code == DiagCode::T100 && warning.span.start() == malicious_start
        }),
        "the callback after 32 benign registrations must still warn: {warnings:?}"
    );
}

#[test]
fn unprovided_tk_callbacks_do_not_gain_replay_authority_from_catalogue() {
    let registry = CommandRegistry::build_default().project_for_profile(
        tcl_dialect::DialectProfile::find("tcl8.6").expect("selected native profile"),
    );
    let source = "package require Tk; entry .one -validatecommand {eval %P}";
    let cu =
        crate::compilation_unit::CompilationUnit::build_standalone_for(source, &registry, false)
            .with_interprocedural(&registry, None);
    reset_callback_replay_build_count();
    let _ = find_taint_warnings_for_cu(&cu, &registry, None);
    assert_eq!(callback_replay_build_count(), 0);
}

#[test]
fn callback_free_units_skip_replay_name_search_and_solve() {
    use crate::compilation_unit::CompilationUnit;

    let registry = CommandRegistry::build_default();
    for source in [
        "set value fixed\nputs $value",
        // This is a callback registration, but its builder is dynamic and
        // therefore deliberately has no statically recoverable replay.
        "entry .entry -validatecommand [format {eval %P}]",
    ] {
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        reset_callback_replay_build_count();
        reset_callback_replay_name_search_count();
        let _warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        assert_eq!(
            callback_replay_build_count(),
            0,
            "no recoverable callback may compile an empty replay: {source}"
        );
        assert_eq!(
            callback_replay_name_search_count(),
            0,
            "no recoverable callback may scan source for replay names: {source}"
        );
    }
}

#[test]
fn callback_replay_name_search_is_single_pass_under_collision_stress() {
    use std::fmt::Write as _;

    let mut source = String::new();
    for suffix in 0..(MAX_CALLBACK_TAINT_REPLAYS + 8) {
        let input = if suffix == 0 {
            CALLBACK_REPLAY_INPUT_VAR_BASE.to_owned()
        } else {
            format!("{CALLBACK_REPLAY_INPUT_VAR_BASE}_{suffix}")
        };
        let proc = if suffix == 0 {
            CALLBACK_REPLAY_PROC_BASE.to_owned()
        } else {
            format!("{CALLBACK_REPLAY_PROC_BASE}_{suffix}")
        };
        writeln!(
            &mut source,
            "set {input} safe\nproc ::{proc}_0 {{}} {{ return }}"
        )
        .expect("write to String");
    }
    source.push_str("entry .entry -validatecommand {eval %P}");

    let (cu, registry) = loaded_tk_callback_unit(&source);
    reset_callback_replay_build_count();
    reset_callback_replay_name_search_count();
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings
            .iter()
            .any(|warning| warning.code == DiagCode::T100),
        "collision avoidance must preserve the callback warning: {warnings:?}"
    );
    assert_eq!(callback_replay_build_count(), 1);
    assert_eq!(
        callback_replay_name_search_count(),
        1,
        "even a collision-rich source gets one linear synthetic-name scan"
    );
}

/// Release-independent offset bookkeeping (8.4–9.1).
#[test]
fn callback_replay_insertions_map_back_to_original_offsets() {
    let insertions = CallbackReplayInsertions(vec![(4, 3), (10, 5)]);
    assert_eq!(
        insertions.original_span(Span::new(0, 4)),
        Some(Span::new(0, 4))
    );
    assert_eq!(insertions.original_span(Span::new(5, 6)), None);
    assert_eq!(
        insertions.original_span(Span::new(7, 9)),
        Some(Span::new(4, 6))
    );
    assert_eq!(insertions.original_span(Span::new(13, 14)), None);
    assert_eq!(
        insertions.original_span(Span::new(11, 13)),
        Some(Span::new(8, 10))
    );
    assert_eq!(
        insertions.original_span(Span::new(18, 20)),
        Some(Span::new(10, 12))
    );
    assert!(!insertions.shifts(Span::new(0, 2)));
    assert!(insertions.shifts(Span::new(7, 8)));
}

#[test]
fn callback_unavailable_spans_require_bounded_original_correspondence() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let (_owner, unit) = retained_warning_unit("set value 1", &context);
    let end = u32::try_from(unit.source.len()).unwrap();
    let registration = unit.ir_module.top_level.statements.first().unwrap().span();
    let replay = Span::new(end + 1, end + 21);
    let spans = [(replay, registration, "input".to_owned())];
    let insertions = CallbackReplayInsertions(vec![(end, 21)]);
    let residual = |span| UnavailableTaintReplay {
        span,
        reason: UnavailableTaintReplayReason::SourceMetadata,
        source: None,
    };
    let original = residual(Span::new(0, end));
    assert_eq!(
        original_callback_replay_residuals(&original, &unit, &spans, &insertions)[0].span,
        original.span
    );
    for span in [
        Span::new(end + 2, end + 3),
        Span::new(end - 1, end + 2),
        Span::new(end + 30, end + 31),
    ] {
        let mapped =
            original_callback_replay_residuals(&residual(span), &unit, &spans, &insertions);
        assert_eq!(mapped.len(), 1);
        assert_eq!(mapped[0].span, registration);
        assert!(
            mapped[0]
                .source
                .as_ref()
                .unwrap()
                .matches_module(&unit.ir_module, context.commands())
        );
    }
    let out_of_bounds = residual(Span::new(end + 30, end + 31));
    let unmapped = original_callback_replay_residuals(
        &out_of_bounds,
        &unit,
        &[],
        &CallbackReplayInsertions::default(),
    );
    assert_eq!(unmapped[0].span, out_of_bounds.span);
    assert!(unmapped[0].source.is_none());
}
