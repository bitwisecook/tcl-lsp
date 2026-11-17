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

//! T102: option positions, `--` terminators, tight spans, fixes, and calls reached through aliases and renames.

use super::*;

fn source_option_context() -> std::sync::Arc<tcl_registry::model::ContextRegistry> {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let mut catalogue = baseline.commands().project_for_profile(profile);
    let mut descriptor = catalogue.get("regexp").unwrap().clone();
    let options = descriptor
        .options
        .iter()
        .cloned()
        .map(|mut option| {
            if option.name == "--" {
                option.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
            }
            option
        })
        .collect::<Vec<_>>();
    descriptor.options = Box::leak(options.into_boxed_slice());
    catalogue.insert(descriptor);
    std::sync::Arc::new(baseline.with_command_store(std::sync::Arc::new(catalogue)))
}

#[test]
fn actual_option_advice_keeps_selected_terminator_availability_and_source_vector() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Conditional descriptor/operand admission, independently of the kept
    // Native entry. No regexp handler or option value is executed here.
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let current = source_option_context();
    let registry = current.commands();
    let (_owner, unit) = retained_warning_unit("regexp $payload haystack", &current);
    let tokens = unit
        .ir_module
        .top_level
        .statements
        .last()
        .unwrap()
        .tokens()
        .unwrap();
    let source =
        TaintSourceContext::for_module_function(registry, &unit.ir_module, &unit.top_level);
    let (selected, _) =
        crate::registry_invocation::conditional_option_arguments_with_metadata_context(
            registry,
            source.metadata.unwrap(),
            tokens,
        )
        .expect("genuine selected source option layout");
    let args = selected
        .effective
        .argument_spellings(tokens.argv_texts.get(1..).unwrap())
        .unwrap();
    let call = SinkCall {
        source,
        command: &selected.facts.canonical_command,
        args: &args,
        registry,
        tokens: Some(tokens),
        effective: Some(&selected.effective),
        source_read: None,
        argument_offset: 0,
    };
    let (sink_spec, sink_command, sink_args) =
        selected_sink_schema(&call).expect("selected source sink schema");
    assert_eq!(sink_spec.name, "regexp");
    assert_eq!(sink_command, "regexp");
    assert_eq!(sink_args, args);
    assert_eq!(
        option_injection_arguments(&call, Some(profile)).unwrap().1,
        [0]
    );
    let input = unit.top_level.source_metadata_input().unwrap();
    let older = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(std::sync::Arc::clone(registry)),
    );
    let foreign =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    for context in [Some(older), Some(foreign), None] {
        let changed = context.map(|context| {
            crate::analyser::ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                context,
                input.lexer_config(),
            )
        });
        let unavailable = SinkCall {
            source: TaintSourceContext::for_input(registry, changed.as_ref(), source.config),
            ..call
        };
        assert!(option_injection_arguments(&unavailable, Some(profile)).is_none());
    }
    let mut stale = source.config;
    stale.strict_quoting = !stale.strict_quoting;
    let stale = SinkCall {
        source: TaintSourceContext::for_input(registry, Some(input), stale),
        ..call
    };
    assert!(option_injection_arguments(&stale, Some(profile)).is_none());
    let detached = SinkCall {
        effective: None,
        ..call
    };
    assert!(option_injection_arguments(&detached, Some(profile)).is_none());
    assert!(selected_sink_schema(&detached).is_none());
    let mismatched = SinkCall { args: &[], ..call };
    assert!(selected_sink_schema(&mismatched).is_none());
}

#[test]
fn actual_option_terminator_withdraws_possible_source_positions() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let current = source_option_context();
    let registry = current.commands();
    let (_terminated_owner, terminated) =
        retained_warning_unit("regexp -- $payload haystack", &current);
    let tokens = terminated
        .ir_module
        .top_level
        .statements
        .last()
        .unwrap()
        .tokens()
        .unwrap();
    let source = TaintSourceContext::for_module_function(
        registry,
        &terminated.ir_module,
        &terminated.top_level,
    );
    let (_, topology) =
        crate::registry_invocation::conditional_option_arguments_with_metadata_context(
            registry,
            source.metadata.unwrap(),
            tokens,
        )
        .unwrap();
    assert_eq!(topology.arguments, [] as [usize; 0]);
}

/// T102: tainted variable passed to `regexp` without `--` terminator.
#[test]
fn t102_emitted_for_tainted_regexp_pattern() {
    use crate::cfg::{Function, Terminator};
    use crate::ssa::{SsaBlock, SsaFunction};
    use tcl_lexer::Span;

    let registry = CommandRegistry::build_default();

    // set pattern [gets stdin] ; regexp $pattern $haystack
    let assign = Statement::AssignValue {
        span: Span::new(0, 25),
        name: "pattern".into(),
        name_braced: false,
        value: "[gets stdin]".into(),
        value_needs_backsubst: false,
        tokens: None,
    };
    let regexp_call = Statement::Call {
        span: Span::new(26, 50),
        command: "regexp".into(),
        canonical_command: None,
        args: vec!["$pattern".into(), "haystack_value".into()],
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    };

    let mut cfg = Function::new("::top", "entry");
    let entry = cfg.entry;
    cfg.blocks
        .get_mut(&entry)
        .unwrap()
        .statements
        .push(assign.clone());
    cfg.blocks
        .get_mut(&entry)
        .unwrap()
        .statements
        .push(regexp_call.clone());
    cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });

    let mut ssa = SsaFunction::trivial("::top", entry, cfg.block_names().to_vec());
    let pattern = ssa.intern_var("pattern");
    let ssa_assign = ssa_stmt(
        assign,
        HashMap::new(),
        [(pattern, 1u32)].into_iter().collect(),
    );
    let ssa_regexp = ssa_stmt(
        regexp_call,
        [(pattern, 1u32)].into_iter().collect(),
        HashMap::new(),
    );

    ssa.blocks.insert(
        entry,
        SsaBlock {
            name: "entry".into(),
            phis: Vec::new(),
            statements: vec![ssa_assign, ssa_regexp],
            entry_versions: HashMap::new(),
            exit_versions: HashMap::new(),
        },
    );

    let sccp = simple_sccp(&[entry]);
    let taints = plain_taints(&cfg, &ssa, &sccp, &registry);
    let warnings = find_taint_warnings(
        &cfg,
        &ssa,
        &taints,
        &sccp.executable_blocks,
        &registry,
        None,
        &HashSet::new(),
    );

    assert!(
        warnings
            .iter()
            .any(|w| w.code == DiagCode::T102 && w.variable == "pattern"),
        "expected T102 for tainted $pattern passed to regexp, got {warnings:?}"
    );
}

/// T102 is suppressed when a `--` terminator precedes the tainted argument.
#[test]
fn t102_suppressed_with_terminator() {
    use crate::cfg::{Function, Terminator};
    use crate::ssa::{SsaBlock, SsaFunction};
    use tcl_lexer::Span;

    let registry = CommandRegistry::build_default();

    let assign = Statement::AssignValue {
        span: Span::new(0, 25),
        name: "pattern".into(),
        name_braced: false,
        value: "[gets stdin]".into(),
        value_needs_backsubst: false,
        tokens: None,
    };
    // regexp -- $pattern $haystack  (safe: -- terminates option parsing)
    let regexp_call = Statement::Call {
        span: Span::new(26, 55),
        command: "regexp".into(),
        canonical_command: None,
        args: vec!["--".into(), "$pattern".into(), "haystack_value".into()],
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    };

    let mut cfg = Function::new("::top", "entry");
    let entry = cfg.entry;
    cfg.blocks
        .get_mut(&entry)
        .unwrap()
        .statements
        .push(assign.clone());
    cfg.blocks
        .get_mut(&entry)
        .unwrap()
        .statements
        .push(regexp_call.clone());
    cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });

    let mut ssa = SsaFunction::trivial("::top", entry, cfg.block_names().to_vec());
    let pattern = ssa.intern_var("pattern");
    let ssa_assign = ssa_stmt(
        assign,
        HashMap::new(),
        [(pattern, 1u32)].into_iter().collect(),
    );
    let ssa_regexp = ssa_stmt(
        regexp_call,
        [(pattern, 1u32)].into_iter().collect(),
        HashMap::new(),
    );

    ssa.blocks.insert(
        entry,
        SsaBlock {
            name: "entry".into(),
            phis: Vec::new(),
            statements: vec![ssa_assign, ssa_regexp],
            entry_versions: HashMap::new(),
            exit_versions: HashMap::new(),
        },
    );

    let sccp = simple_sccp(&[entry]);
    let taints = plain_taints(&cfg, &ssa, &sccp, &registry);
    let warnings = find_taint_warnings(
        &cfg,
        &ssa,
        &taints,
        &sccp.executable_blocks,
        &registry,
        None,
        &HashSet::new(),
    );

    let t102: Vec<_> = warnings
        .iter()
        .filter(|w| w.code == DiagCode::T102)
        .collect();
    assert_eq!(
        t102.len(),
        0,
        "expected no T102 when '--' terminator present, got {t102:?}"
    );
}

#[test]
fn t102_uses_ensemble_subcommand_label_and_t103_for_regexp() {
    use crate::compilation_unit::CompilationUnit;
    for (release, registry) in release_registries() {
        // `file delete $u` → T102 labelled "file delete" (ensemble
        // subcommand, via resolve_option_terminator). `regexp $u …` →
        // T103 (regex injection) then T102 (regexp also takes `--`).
        let cu = CompilationUnit::build_for(
            "set u [gets stdin]\nfile delete $u\nregexp $u \"abc\"",
            &registry,
            false,
        );
        let fu = cu.function("::top").unwrap();
        let warnings = find_taint_warnings(
            &fu.cfg,
            &fu.ssa,
            &fu.taints,
            &fu.sccp.executable_blocks,
            &registry,
            Some(tcl_registry::model::ingress::resolve_environment(release).analyser_profile()),
            &HashSet::new(),
        );
        let t102 = warnings.iter().find(|w| w.code == DiagCode::T102);
        assert!(
            t102.is_some_and(|w| w.sink_command == "file delete"),
            "{release}: expected a T102 with ensemble label 'file delete', got {warnings:?}",
        );
        assert!(
            warnings
                .iter()
                .any(|w| w.code == DiagCode::T103 && w.sink_command == "regexp"),
            "{release}: expected T103 for tainted regexp pattern, got {warnings:?}",
        );
        // T103 must precede the regexp T102 (the pattern check is
        // emitted before the sink loop).
        let regexp_codes: Vec<&str> = warnings
            .iter()
            .filter(|w| w.sink_command == "regexp")
            .map(|w| w.code.as_str())
            .collect();
        assert_eq!(
            regexp_codes,
            vec!["T103", "T102"],
            "{release}: regexp warnings must be ordered T103 then T102",
        );
    }
}

#[test]
fn t102_suppressed_after_double_dash_terminator() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    // `file delete -- $u`: the `--` terminator protects the path
    // position, so no T102 (W313 still fires elsewhere).
    let cu = CompilationUnit::build_for("set u [gets stdin]\nfile delete -- $u", &registry, false);
    let fu = cu.function("::top").unwrap();
    let warnings = find_taint_warnings(
        &fu.cfg,
        &fu.ssa,
        &fu.taints,
        &fu.sccp.executable_blocks,
        &registry,
        Some(tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()),
        &HashSet::new(),
    );
    assert!(
        warnings.iter().all(|w| w.code != DiagCode::T102),
        "no T102 when `--` precedes the tainted path, got {warnings:?}",
    );
}

/// FN regression (see `emit_statement_warnings`): T102 must fire for
/// a `set matched [regexp $pattern $s]` capture exactly as it does for
/// the bare `regexp $pattern $s` call it wraps — the earlier code only
/// ran the option-injection check on `Statement::Call`, silently
/// skipping the (dominant, in real Tcl) `set x [cmd ...]` idiom.
#[test]
fn t102_fires_for_assign_value_wrapped_call() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let cu = CompilationUnit::build_for(
        "set haystack text\nset pattern [gets stdin]\nset result [regexp $pattern $haystack]",
        &registry,
        false,
    );
    let fu = cu.function("::top").unwrap();
    let warnings = find_taint_warnings(
        &fu.cfg,
        &fu.ssa,
        &fu.taints,
        &fu.sccp.executable_blocks,
        &registry,
        Some(tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()),
        &HashSet::new(),
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.code == DiagCode::T102 && w.variable == "pattern"),
        "expected T102 for tainted $pattern inside `set result [regexp ...]`, got {warnings:?}"
    );
}

/// T102's diagnostic span must cover only the offending argument
/// (`$pattern`), not the whole statement — tight enough that the
/// editor squiggle sits on the actual problem, matching what W304
/// already does for the same position.
#[test]
fn t102_span_is_tight_around_the_tainted_argument_bare_call() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let src = "set pattern [gets stdin]\nregexp $pattern $haystack";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let fu = cu.function("::top").unwrap();
    let warnings = find_taint_warnings(
        &fu.cfg,
        &fu.ssa,
        &fu.taints,
        &fu.sccp.executable_blocks,
        &registry,
        Some(tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()),
        &HashSet::new(),
    );
    let t102 = warnings
        .iter()
        .find(|w| w.code == DiagCode::T102)
        .expect("expected a T102 warning");
    let slice = &src[t102.span.start() as usize..t102.span.end() as usize];
    assert_eq!(
        slice, "$pattern",
        "T102 span should cover exactly the tainted argument, got {slice:?} from {t102:?}"
    );
}

/// Same tight-span requirement for the `set x [cmd ...]` shape: the
/// embedded command's argument has no token of its own on the
/// `AssignValue` statement, so the span is re-derived from the
/// bracketed value text — this pins that derivation to the exact
/// same byte range as the bare-call case above.
#[test]
fn t102_span_is_tight_around_the_tainted_argument_assign_value() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let src = "set haystack text\nset pattern [gets stdin]\nset result [regexp $pattern $haystack]";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let fu = cu.function("::top").unwrap();
    let warnings = find_taint_warnings(
        &fu.cfg,
        &fu.ssa,
        &fu.taints,
        &fu.sccp.executable_blocks,
        &registry,
        Some(tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()),
        &HashSet::new(),
    );
    let t102 = warnings
        .iter()
        .find(|w| w.code == DiagCode::T102)
        .expect("expected a T102 warning");
    let slice = &src[t102.span.start() as usize..t102.span.end() as usize];
    assert_eq!(
        slice, "$pattern",
        "T102 span should cover exactly the tainted argument, got {slice:?} from {t102:?}"
    );
}

/// T102 must carry a `--`-insertion fix anchored immediately before
/// the tainted argument, so applying it turns the flagged call into
/// the documented safe form (`regexp -- $pattern ...`).
#[test]
fn t102_carries_insert_terminator_fix_at_the_argument_start() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let src = "set pattern [gets stdin]\nregexp $pattern $haystack";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let fu = cu.function("::top").unwrap();
    let warnings = find_taint_warnings(
        &fu.cfg,
        &fu.ssa,
        &fu.taints,
        &fu.sccp.executable_blocks,
        &registry,
        Some(tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()),
        &HashSet::new(),
    );
    let t102 = warnings
        .iter()
        .find(|w| w.code == DiagCode::T102)
        .expect("expected a T102 warning");
    assert_eq!(t102.fixes.len(), 1, "expected exactly one fix: {t102:?}");
    let fix = &t102.fixes[0];
    assert_eq!(
        fix.span.start(),
        fix.span.end(),
        "fix must be a pure insertion"
    );
    assert_eq!(fix.span.start(), t102.span.start());
    assert_eq!(fix.new_text, "-- ");
    let mut fixed = src.to_owned();
    fixed.insert_str(fix.span.start() as usize, &fix.new_text);
    assert_eq!(
        fixed,
        "set pattern [gets stdin]\nregexp -- $pattern $haystack"
    );
}

/// `interp alias {} myregexp {} regexp` makes `myregexp` behave
/// exactly like `regexp` at runtime (tclsh 9.0.4-verified), including
/// its option parsing — so a tainted argument reaching `myregexp`
/// must trip T102 just as it would through the bare `regexp` name.
/// `emit_statement_warnings`'s `canonical_command_or_source()` dispatch
/// (shared by the whole sink family, see the `t100_fires_through_*`
/// tests) resolves the alias for registry classification, while the
/// message and span still anchor on the alias call the source
/// actually wrote.
#[test]
fn t102_fires_through_an_interp_alias() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let src =
        "interp alias {} myregexp {} regexp\nset pattern [gets stdin]\nmyregexp $pattern $haystack";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let fu = cu.function("::top").unwrap();
    let warnings = find_taint_warnings(
        &fu.cfg,
        &fu.ssa,
        &fu.taints,
        &fu.sccp.executable_blocks,
        &registry,
        Some(tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()),
        &HashSet::new(),
    );
    let t102 = warnings
        .iter()
        .find(|w| w.code == DiagCode::T102)
        .unwrap_or_else(|| panic!("expected a T102 warning through the alias, got {warnings:?}"));
    assert_eq!(t102.variable, "pattern");
    assert!(
        t102.message.contains("'regexp'"),
        "message should name the resolved target command, got {:?}",
        t102.message
    );
    // The span/fix still anchor on the alias call's own argument,
    // not some fabricated position for the aliased-to command.
    assert_eq!(t102.fixes.len(), 1);
    let fix = &t102.fixes[0];
    let mut fixed = src.to_owned();
    fixed.insert_str(fix.span.start() as usize, &fix.new_text);
    assert_eq!(
        fixed,
        "interp alias {} myregexp {} regexp\nset pattern [gets stdin]\nmyregexp -- $pattern $haystack"
    );
}

/// `rename regexp myregexp` resolves through the same `CommandAliasMap`
/// mechanism as `interp alias` (`detect_rename`, see the T100 sibling
/// tests `t100_fires_through_rename_indirection` /
/// `t100_silent_for_unaliased_lookalike_command`), so T102 must also
/// fire through a renamed command name, not just an `interp alias`.
#[test]
fn t102_fires_through_a_rename() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let src = "rename regexp myregexp\nset pattern [gets stdin]\nmyregexp $pattern $haystack";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let fu = cu.function("::top").unwrap();
    let warnings = find_taint_warnings(
        &fu.cfg,
        &fu.ssa,
        &fu.taints,
        &fu.sccp.executable_blocks,
        &registry,
        Some(tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()),
        &HashSet::new(),
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.code == DiagCode::T102 && w.variable == "pattern"),
        "expected T102 for tainted $pattern through the `myregexp` rename, got {warnings:?}"
    );
}

#[test]
fn native_source_option_warnings_keep_original_schema_and_terminator() {
    let context =
        crate::environment_ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = context.commands();
    let profile = registry.profile().unwrap();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    for (command, expected) in [
        ("exec $value", true),
        ("exec -- $value", false),
        ("exec {$value}", false),
        ("exec {*}$value", false),
    ] {
        let source = format!("proc run {{value}} {{{command}}}");
        let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
            &source,
            crate::compilation_unit::UnitBuildOptions {
                registry,
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        )
        .with_interprocedural(registry, Some(profile));
        assert!(unit.ir_module.source_entry.native_entry.is_none());
        assert_eq!(
            unit.ir_module.original_native_declaration_body_units.len(),
            1
        );
        let warnings = find_taint_warnings_for_cu(&unit, registry, Some(profile));
        assert_eq!(
            warnings.iter().any(|warning| warning.code == DiagCode::T102
                && warning.variable == "value"
                && warning.sink_command == "exec"),
            expected,
            "{command}: {warnings:?}"
        );
        if expected {
            let mut missing = unit.clone();
            missing.ir_module.retained_source_bindings = None;
            assert!(
                find_taint_warnings_for_cu(&missing, registry, Some(profile))
                    .iter()
                    .all(|warning| warning.code != DiagCode::T102)
            );
            let foreign = crate::environment_ingress::resolve_environment("tcl9.0")
                .default_context_registry();
            assert!(
                find_taint_warnings_for_cu(&unit, foreign.commands(), Some(profile))
                    .iter()
                    .all(|warning| warning.code != DiagCode::T102)
            );
        }
    }
}
