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

#[test]
fn a_colon_run_head_still_names_its_own_global_target() {
    // tclsh 8.6/9.0/9.1: `interp alias {} :::puts {} puts` aliases `::puts`
    // onto itself and deletes it; the next `puts` is `invalid command name`.
    assert!(super::global_heads_name_one_command("puts", ":::puts"));
    assert!(super::global_heads_name_one_command("::puts", "puts"));
    assert!(!super::global_heads_name_one_command("puts", ":puts"));
    assert!(!super::global_heads_name_one_command("puts", "ns::puts"));
}

#[test]
fn original_module_sink_candidates_keep_conditional_source_and_refuse_foreign_owners() {
    let environment = tcl_registry::model::ingress::resolve_environment("tcl8.6");
    let context = environment.default_context_registry();
    let registry = context.commands();
    let profile = environment.unit_profile();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let build = |source| {
        crate::compilation_unit::CompilationUnit::build_with_analysis_input(
            source,
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
        .with_interprocedural(registry, Some(profile))
    };
    let source =
        "proc pass {x} {return $x}; proc danger {} {set u [gets stdin]; set p [pass $u]; exec $p}";
    let unit = build(source);
    let (body, procedure) = unit
        .ir_module
        .original_source_procedure_bodies()
        .find(|(_, procedure)| {
            procedure
                .body_source
                .as_deref()
                .is_some_and(|source| source.contains("exec $p"))
        })
        .unwrap();
    assert_eq!(
        procedure.body_kind,
        crate::ir::ProcedureBodyKind::OriginalDeclaration
    );
    assert!(!unit.ir_module.procedures.contains_key(body));
    assert!(unit.ir_module.source_entry.native_entry.is_none());
    let warnings = |unit: &crate::compilation_unit::CompilationUnit, registry: &CommandRegistry| {
        let solved =
            crate::taint_interproc::solve_interprocedural_taints(unit, registry, Some(profile));
        let mut warnings = Vec::new();
        let _ = crate::compiler_checks::push_taint_and_module_checks(
            unit,
            registry,
            Some(profile),
            &solved,
            None,
            &mut warnings,
        );
        warnings
    };
    assert!(
        warnings(&unit, registry)
            .iter()
            .any(|warning| warning.code == DiagCode::T100)
    );
    let mut missing = unit.clone();
    missing.ir_module.retained_source_bindings = None;
    assert!(
        !warnings(&missing, registry)
            .iter()
            .any(|warning| warning.code == DiagCode::T100)
    );
    let mut changed = unit.clone();
    changed.ir_module.source = tcl_lexer::SourceImage::document("proc danger {} {return SAFE}");
    assert!(
        !warnings(&changed, registry)
            .iter()
            .any(|warning| warning.code == DiagCode::T100)
    );
    let foreign = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    assert!(
        !warnings(&unit, foreign)
            .iter()
            .any(|warning| warning.code == DiagCode::T100)
    );
    let shadowed =
        build("proc exec args {return SAFE}; proc danger {} {set u [gets stdin]; exec $u}");
    assert!(
        !warnings(&shadowed, registry)
            .iter()
            .any(|warning| warning.code == DiagCode::T100)
    );
}

use super::*;
use crate::sccp::SccpResult;
use tcl_dialect::model::{Family, SurfaceLayer};

#[test]
fn original_instance_receiver_names_preserve_head_kind_and_actual_grammar() {
    let classes = LocalInstanceClasses {
        at: HashMap::from([(
            42,
            HashMap::from([
                ("$literal".into(), HashSet::from(["LiteralClass".into()])),
                ("${literal}".into(), HashSet::from(["BraceClass".into()])),
                ("literal".into(), HashSet::from(["ReferenceClass".into()])),
                (String::new(), HashSet::from(["EmptyClass".into()])),
                ("arr".into(), HashSet::from(["ScalarClass".into()])),
                ("arr(key)".into(), HashSet::from(["CommandClass".into()])),
                ("a{b}c".into(), HashSet::from(["NestedClass".into()])),
            ]),
        )]),
    };
    for label in ["tcl8.6", "tcl9.1"] {
        let registry = tcl_registry::model::ingress::static_context_for(label).commands();
        let config = TaintSourceContext::standalone(registry, registry.profile()).config;
        for (source, expected) in [
            ("{$literal}", "LiteralClass"),
            ("{${literal}}", "BraceClass"),
            ("$literal", "ReferenceClass"),
            ("${$literal}", "LiteralClass"),
            ("{}", "EmptyClass"),
            ("${}", "EmptyClass"),
            ("arr(key)", "CommandClass"),
        ] {
            let word = crate::value_shapes::value_word_with_config(source, config).unwrap();
            assert_eq!(
                unique_instance_class(Some((&word, config)), Some(&classes), Some(42)),
                Some(expected),
                "{label}: {source}"
            );
            assert!(
                unique_instance_class(Some((&word, config)), Some(&classes), Some(43)).is_none()
            );
        }
        for source in [
            "$arr(key)",
            "${arr(key)}",
            "prefix$literal",
            "[pick]",
            "{*}{literal}",
            "${nul\0name}",
        ] {
            let word = crate::value_shapes::value_word_with_config(source, config).unwrap();
            assert!(
                unique_instance_class(Some((&word, config)), Some(&classes), Some(42)).is_none(),
                "{label}: {source}"
            );
        }
        let nested = crate::value_shapes::value_word_with_config("${a{b}c}", config).unwrap();
        assert_eq!(
            unique_instance_class(Some((&nested, config)), Some(&classes), Some(42)),
            if label == "tcl9.1" {
                Some("NestedClass")
            } else {
                None
            }
        );
    }
    assert!(unique_instance_class(None, Some(&classes), Some(42)).is_none());
}

#[test]
fn configured_word_taint_keeps_literal_reference_names_and_index_reads() {
    for label in ["tcl8.6", "tcl9.1"] {
        let registry = tcl_registry::model::ingress::static_context_for(label).commands();
        let cfg = crate::cfg::Function::new("model", "entry");
        let mut ssa = SsaFunction::trivial("model", cfg.entry, cfg.block_names().to_vec());
        let mut uses = HashMap::new();
        let mut taints = HashMap::new();
        for (name, tainted) in [
            ("literal", false),
            ("$literal", true),
            ("", true),
            ("arr", false),
            ("index", true),
            ("a{b", false),
            ("a{b}c", true),
        ] {
            let symbol = ssa.intern_var(name);
            uses.insert(symbol, 1);
            taints.insert(
                (symbol, 1),
                if tainted {
                    TaintLattice::tainted()
                } else {
                    TaintLattice::clean()
                },
            );
        }
        let ctx = TaintCtx {
            registry,
            ssa: &ssa,
            interproc: None,
            known_procs: None,
            dialect: registry.profile(),
            source: TaintSourceContext::standalone(registry, registry.profile()),
            taint_summaries: None,
            instance_classes: None,
            source_position: None,
            source_tokens: None,
            word_offset: None,
            source_word: None,
        };
        for (word, tainted) in [
            ("$literal", false),
            ("${$literal}", true),
            ("${}", true),
            ("prefix${$literal}", true),
            (r"prefix\$literal", false),
            ("$arr($index)", true),
            ("[missing $literal", true),
        ] {
            assert_eq!(
                word_taint(word, &uses, &taints, ctx).is_tainted(),
                tainted,
                "{label}: {word}"
            );
        }
        assert_eq!(
            word_taint("${a{b}c}", &uses, &taints, ctx).is_tainted(),
            label == "tcl9.1"
        );
        assert!(!word_taint(r"prefix\[gets stdin\]", &uses, &taints, ctx).is_tainted());
    }
}

#[test]
fn taint_escape_fragments_use_the_selected_release_escape_grammar() {
    let before = backslash_subst_in(r"\x4142", tcl_dialect::EscapeSyntax::Tcl84);
    let after = backslash_subst_in(r"\x4142", tcl_dialect::EscapeSyntax::Tcl86);
    assert_ne!(before, after);
    assert_eq!(after, "A42");
}

#[test]
fn mixed_source_word_reuses_the_retained_nested_command_invocation() {
    let environment = tcl_registry::model::ingress::resolve_environment("tcl8.6");
    let context = environment.default_context_registry();
    let registry = context.commands();
    let (_, unit) = retained_warning_unit("set payload \"prefix[string length $input]\"", &context);
    let module = &unit.ir_module;
    let tokens = module
        .top_level
        .statements
        .last()
        .and_then(Statement::tokens)
        .expect("source-backed assignment tokens");
    let word = tokens.words().get(2).expect("mixed source value word");
    let source = TaintSourceContext::for_module_function(registry, module, &unit.top_level);
    let ctx = TaintCtx {
        registry,
        ssa: &unit.top_level.ssa,
        interproc: None,
        known_procs: None,
        dialect: registry.profile(),
        source,
        taint_summaries: None,
        instance_classes: None,
        source_position: Some(tokens.argv[0].start()),
        source_tokens: Some(tokens),
        word_offset: Some(word.source().span.start()),
        source_word: Some(word),
    };
    assert!(
        !word_taint(&word.legacy_text(), &HashMap::new(), &HashMap::new(), ctx).is_tainted(),
        "the actual selected string-length sanitizer remains clean in a mixed word"
    );
    let unavailable = TaintCtx {
        source_tokens: None,
        ..ctx
    };
    assert!(
        word_taint(
            &word.legacy_text(),
            &HashMap::new(),
            &HashMap::new(),
            unavailable
        )
        .is_tainted(),
        "a source span without its retained parent token carrier is unavailable"
    );
}

#[test]
fn taint_source_word_requires_its_actual_retained_owner_and_span() {
    let environment = tcl_registry::model::ingress::resolve_environment("tcl8.6");
    let context = environment.default_context_registry();
    let registry = context.commands();
    let (_, unit) = retained_warning_unit("set input SAFE; set payload $input", &context);
    let module = &unit.ir_module;
    let tokens = unit
        .top_level
        .cfg
        .blocks
        .values()
        .flat_map(|block| &block.statements)
        .filter_map(Statement::tokens)
        .find(|tokens| {
            tokens
                .words()
                .iter()
                .any(|word| word.legacy_text() == "$input")
        })
        .expect("actual source statement tokens");
    let word = tokens.words().get(2).expect("original value word");
    let ctx = TaintCtx {
        registry,
        ssa: &unit.top_level.ssa,
        interproc: None,
        known_procs: None,
        dialect: registry.profile(),
        source: TaintSourceContext::for_module(registry, module),
        taint_summaries: None,
        instance_classes: None,
        source_position: None,
        source_tokens: Some(tokens),
        word_offset: Some(word.source().span.start()),
        source_word: Some(word),
    };
    let (image, span) =
        retained_word_source(word.legacy_text().as_str(), ctx).expect("same retained source owner");
    assert_eq!(
        image.bytes().get(span.as_range()),
        Some(b"$input".as_slice())
    );
    let read_site = word.source();
    let mut read_owner = None;
    for (block, data) in &unit.top_level.ssa.blocks {
        for index in 0..data.statements.len() {
            let point = crate::ssa::SsaSourceView::at_statement(&unit.top_level.ssa, *block, index);
            if let Some(read) = point.read_reference(read_site, "$input")
                && let Some(version) = read.version
            {
                read_owner = Some((read.symbol, version));
            }
        }
    }
    let (symbol, version) = read_owner.expect("the exact original variable read");
    let clean = HashMap::from([((symbol, version), TaintLattice::clean())]);
    let read_ctx = TaintCtx {
        source_position: Some(tokens.argv[0].start()),
        ..ctx
    };
    assert!(
        !original_component_read_taint(read_ctx, read_site.span, "$input", &clean,).is_tainted()
    );
    let tainted = HashMap::from([((symbol, version), TaintLattice::tainted())]);
    assert!(
        word_taint("plain_projection", &HashMap::new(), &tainted, read_ctx).is_tainted(),
        "the source read span, not a compatibility spelling, selects the SSA value"
    );
    let mut detached_tokens = tokens.clone();
    detached_tokens.variable_accesses.clear();
    let detached_ctx = TaintCtx {
        source_tokens: Some(&detached_tokens),
        ..read_ctx
    };
    assert!(
        original_component_read_taint(detached_ctx, read_site.span, "$input", &clean,).is_tainted()
    );
    let detached = TaintCtx {
        source_tokens: None,
        ..ctx
    };
    assert!(retained_word_source(word.legacy_text().as_str(), detached).is_none());
}

#[test]
fn incr_taint_uses_the_selected_definition_name_without_reparsing_data() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let cfg = crate::cfg::Function::new("model", "entry");
    let mut ssa = SsaFunction::trivial("model", cfg.entry, cfg.block_names().to_vec());
    let literal = ssa.intern_var("$literal");
    let stripped = ssa.intern_var("literal");
    let uses = HashMap::from([(literal, 1), (stripped, 1)]);
    let taints = HashMap::from([
        ((literal, 1), TaintLattice::tainted()),
        ((stripped, 1), TaintLattice::clean()),
    ]);
    let ctx = TaintCtx {
        registry,
        ssa: &ssa,
        interproc: None,
        known_procs: None,
        dialect: registry.profile(),
        source: TaintSourceContext::standalone(registry, registry.profile()),
        taint_summaries: None,
        instance_classes: None,
        source_position: None,
        source_tokens: None,
        word_offset: None,
        source_word: None,
    };
    let statement = Statement::Incr {
        span: Span::new(0, 0),
        name: "$literal".into(),
        name_braced: true,
        amount: None,
        amount_braced: false,
        safe_on_uninit: false,
    };
    assert!(evaluate_taint_def(&statement, literal, &uses, &taints, ctx, &ssa).is_tainted());
    assert!(
        evaluate_taint_def(&statement, literal, &HashMap::new(), &taints, ctx, &ssa).is_tainted()
    );
}

#[test]
fn unrepresented_variable_contents_have_no_clean_taint_proof() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for(
        "set represented SAFE",
        registry,
        false,
    );
    let symbol = unit.top_level.ssa.var_symbol("represented").unwrap();
    let uses = HashMap::from([(symbol, 0)]);
    let taints = HashMap::<ValueKey, TaintLattice>::new();
    assert!(var_taint("unrepresented", &uses, &taints, &unit.top_level.ssa).is_tainted());
    assert!(!var_taint("represented", &uses, &taints, &unit.top_level.ssa).is_tainted());
    assert!(var_taint("represented", &HashMap::new(), &taints, &unit.top_level.ssa).is_tainted());
    assert!(unit.top_level.ssa.var_symbol("unrepresented").is_none());
}

#[test]
fn original_operand_reference_names_keep_data_and_current_grammar_without_ssa_grants() {
    for label in ["tcl8.6", "tcl9.1"] {
        let registry = tcl_registry::model::ingress::static_context_for(label).commands();
        let source = TaintSourceContext::standalone(registry, registry.profile());
        let cfg = crate::cfg::Function::new("model", "entry");
        let ssa = SsaFunction::trivial("model", cfg.entry, cfg.block_names().to_vec());
        let uses = HashMap::new();
        let taints = HashMap::<ValueKey, TaintLattice>::new();
        let env = TaintScan {
            uses: &uses,
            taints: &taints,
            ssa: &ssa,
            quoted_uses: None,
            source_read: None,
            source,
        };
        for (reference, name) in [
            ("${$literal}", "$literal"),
            ("${}", ""),
            ("${arr($literal)}", "arr"),
        ] {
            let word =
                crate::value_shapes::value_word_with_config(reference, source.config).unwrap();
            assert!(
                operand_read_taint(&env, name, Some(&word)).is_some_and(TaintLattice::is_tainted)
            );
            assert!(operand_read_taint(&env, "donated", Some(&word)).is_none());
            assert!(ssa.var_symbol(name).is_none());
        }
        assert_eq!(
            reference_root_with_config("${a{b}c}", source.config),
            if label == "tcl9.1" {
                Some("a{b}c")
            } else {
                None
            }
        );
        for unavailable in [
            "prefix${literal}",
            r"\$literal",
            "${literal",
            "${literal\0tail}",
        ] {
            assert!(reference_root_with_config(unavailable, source.config).is_none());
        }
    }
}

#[test]
fn original_sink_read_keeps_missing_ssa_contents_unknown() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for (source, expected) in [
        ("regexp $missing test", true),
        ("regexp {$missing} test", false),
    ] {
        let unit = crate::compilation_unit::CompilationUnit::build_for(source, registry, false);
        let tokens = unit
            .top_level
            .cfg
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .find_map(Statement::tokens)
            .unwrap();
        let uses = HashMap::new();
        let taints = HashMap::<ValueKey, TaintLattice>::new();
        let env = TaintScan {
            uses: &uses,
            taints: &taints,
            ssa: &unit.top_level.ssa,
            quoted_uses: None,
            source_read: None,
            source: TaintSourceContext::for_function(registry, &unit.top_level),
        };
        assert_eq!(
            operand_read_taint(&env, "missing", tokens.words().get(1))
                .is_some_and(TaintLattice::is_tainted),
            expected,
            "{source}"
        );
        assert!(uses.is_empty());
    }
}

#[test]
fn native_byte_escapes_cannot_become_unicode_text_provenance() {
    use tcl_dialect::EscapeSyntax;
    assert_eq!(decoded_text_fragment(r"\xff", EscapeSyntax::Jim), None);
    assert_eq!(
        decoded_text_fragment(r"\x41", EscapeSyntax::Jim).as_deref(),
        Some("A")
    );
    assert_eq!(
        decoded_text_fragment(r"\xff", EscapeSyntax::Tcl90).as_deref(),
        Some("ÿ")
    );
}

#[test]
fn opaque_native_accesses_withdraw_receiver_and_clean_taint_proofs() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut unit =
        crate::compilation_unit::CompilationUnit::build_for("set value KNOWN", registry, false);
    let native = crate::ir::native_call_for_test(b"opaque \xff");
    unit.top_level
        .cfg
        .blocks
        .get_mut(&unit.top_level.cfg.entry)
        .unwrap()
        .statements
        .push(native.clone());
    let graph = TaintGraph::new(
        &unit.top_level.cfg,
        &unit.top_level.ssa,
        &unit.top_level.sccp,
    );
    let taints = propagate_taints(
        &graph,
        TaintPropagationInputs {
            registry,
            rendered_props: None,
            interproc: None,
            source: TaintSourceContext::standalone(registry, registry.profile()),
            param_taints: None,
            taint_summaries: None,
            instance_classes: &LocalInstanceClasses::default(),
        },
    );
    let keys: Vec<_> = unit
        .top_level
        .ssa
        .blocks
        .values()
        .flat_map(|block| &block.statements)
        .flat_map(|statement| {
            statement
                .defs
                .iter()
                .map(|(&symbol, &version)| (symbol, version))
        })
        .collect();
    assert_ne!(keys, [] as [(crate::ssa::Symbol, u32); 0]);
    assert!(
        keys.iter()
            .all(|key| taints.get(key) == Some(&TaintLattice::tainted()))
    );
    let mut classes = HashMap::from([("widget".into(), HashSet::from(["Entry".into()]))]);
    transfer_instance_statement(
        &mut classes,
        &native,
        registry,
        TaintSourceContext::for_input(registry, None, tcl_lexer::LexerConfig::default()),
        None,
    );
    assert!(classes.is_empty());
}

#[test]
fn alias_prefix_values_and_braced_literals_do_not_resubstitute_variables() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for source in [
        "interp alias {} collect {} list {$attacker}\nset value [collect]",
        "set value [list {$attacker}]",
    ] {
        let module = crate::lowering::lower_to_ir(source, registry);
        let (value, tokens) = module
            .top_level
            .statements
            .iter()
            .rev()
            .find_map(|statement| {
                if let Statement::AssignValue {
                    value,
                    tokens: Some(tokens),
                    ..
                } = statement
                {
                    Some((value, tokens))
                } else {
                    None
                }
            })
            .expect("assignment carrier");
        let cfg = crate::cfg::Function::new("::top", "entry");
        let mut ssa =
            crate::ssa::SsaFunction::trivial("::top", cfg.entry, cfg.block_names().to_vec());
        let attacker = ssa.intern_var("attacker");
        let ctx = TaintCtx {
            registry,
            ssa: &ssa,
            interproc: None,
            known_procs: None,
            dialect: registry.profile(),
            source: TaintSourceContext::standalone(registry, registry.profile()),
            taint_summaries: None,
            instance_classes: None,
            source_position: Some(tokens.argv[0].start()),
            source_tokens: Some(tokens),
            word_offset: tokens.argv.last().map(|span| span.start()),
            source_word: tokens.word_exprs.last(),
        };
        let uses = HashMap::from([(attacker, 1)]);
        let taints = HashMap::from([((attacker, 1), TaintLattice::tainted())]);
        assert!(
            !word_taint(value, &uses, &taints, ctx).is_tainted(),
            "{source}"
        );
    }
}

#[test]
fn nested_sanitiser_requires_the_actual_point_binding() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for (source, tainted) in [
        ("set value [string length [gets stdin]]", false),
        (
            "interp alias {} measure {} string length\nset value [measure [gets stdin]]",
            false,
        ),
        (
            "rename string core_string\nproc string args {return [lindex $args end]}\nset value [string length [gets stdin]]",
            true,
        ),
        (
            "interp alias {} measure {} string length\nrename string core_string\nproc string args {return [lindex $args end]}\nset value [measure [gets stdin]]",
            true,
        ),
        (
            "rename string core_string\nproc string args {return [lindex $args end]}\nset value \"[string length [gets stdin]]\"",
            true,
        ),
    ] {
        let module = crate::lowering::lower_to_ir(source, registry);
        let (value, tokens) = module
            .top_level
            .statements
            .iter()
            .rev()
            .find_map(|statement| {
                let tokens = statement.tokens()?;
                Some((tokens.argv_texts.last()?, tokens))
            })
            .expect("retained source value carrier");
        let cfg = crate::cfg::Function::new("::top", "entry");
        let ssa = crate::ssa::SsaFunction::trivial("::top", cfg.entry, cfg.block_names().to_vec());
        let ctx = TaintCtx {
            registry,
            ssa: &ssa,
            interproc: None,
            known_procs: None,
            dialect: registry.profile(),
            source: TaintSourceContext::standalone(registry, registry.profile()),
            taint_summaries: None,
            instance_classes: None,
            source_position: Some(tokens.argv[0].start()),
            source_tokens: Some(tokens),
            word_offset: tokens.argv.last().map(|span| span.start()),
            source_word: tokens.word_exprs.last(),
        };
        assert_eq!(
            word_taint(value, &HashMap::new(), &HashMap::new(), ctx).is_tainted(),
            tainted,
            "{source}"
        );
    }
}

#[test]
fn original_cross_interp_script_reads_keep_the_tainted_command_word() {
    let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = context.commands();
    let profile = registry.profile().unwrap();
    for (source, expected) in [
        (
            "set raw [gets stdin]\ninterp eval $child [list $raw]\n",
            true,
        ),
        (
            "set raw [gets stdin]\ninterp eval $child [list puts $raw]\n",
            false,
        ),
        (
            "set raw [gets stdin]\nset child [interp create]\ninterp eval $child [list $raw]\n",
            true,
        ),
        (
            "set child [interp create]\nset raw [gets stdin]\ninterp eval $child [list $raw]\n",
            true,
        ),
        (
            "set child [interp create]\nset raw [gets stdin]\ninterp eval $child [list puts $raw]\n",
            false,
        ),
        (
            "set child [gets stdin]\ninterp eval $child {puts SAFE}\n",
            false,
        ),
        (
            "set child [interp create]\nset raw [gets stdin]\ninterp eval $child [list puts $raw] $raw\n",
            true,
        ),
    ] {
        let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            source, registry, false, profile,
        );
        let warnings = find_taint_warnings_for_cu(&unit, registry, Some(profile));
        let cross_interp: Vec<_> = warnings
            .iter()
            .filter(|warning| warning.code == DiagCode::T105)
            .collect();
        assert_eq!(
            !cross_interp.is_empty(),
            expected,
            "{source:?}: {warnings:?}"
        );
        assert!(
            cross_interp.iter().all(|warning| warning.variable == "raw"),
            "the interpreter path must not impersonate script data: {warnings:?}"
        );
    }
}

fn measured_source_taint(
    unit: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
) -> TaintLattice {
    let script = &unit.ir_module.top_level;
    let statement = script.statements.last().expect("source statement");
    let tokens = script
        .retained_source_tokens_for_statement(statement)
        .expect("genuine original source vector");
    let value = tokens.argv_texts.last().expect("original value operand");
    let source =
        TaintSourceContext::for_module_function(registry, &unit.ir_module, &unit.top_level);
    let ctx = TaintCtx {
        registry,
        ssa: &unit.top_level.ssa,
        interproc: None,
        known_procs: None,
        dialect: source.dialect,
        source,
        taint_summaries: None,
        instance_classes: None,
        source_position: tokens.argv.first().map(|span| span.start()),
        source_tokens: Some(tokens),
        word_offset: tokens.argv.last().map(|span| span.start()),
        source_word: tokens.word_exprs.last(),
    };
    word_taint(value, &HashMap::new(), &HashMap::new(), ctx)
}

fn retained_warning_unit(
    source: &str,
    context: &std::sync::Arc<tcl_registry::model::ContextRegistry>,
) -> (tcl_vm::Vm, crate::compilation_unit::CompilationUnit) {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let (owner, native) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let entry = crate::command_binding::SourceAnalysisEntry {
        native_entry: Some(std::sync::Arc::new(native)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    let unit = crate::compilation_unit::CompilationUnit::build_with_context_registry(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        std::sync::Arc::clone(context),
    );
    (owner, unit)
}

#[test]
fn branch_nested_sink_uses_retained_expression_and_read_owners() {
    let environment = tcl_registry::model::ingress::resolve_environment("tcl8.6");
    let context = environment.default_context_registry();
    let registry = context.commands();
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let source = "proc f {} { set x [gets stdin]; if {[exec $x] eq \"ok\"} { return 1 } }";
    let (_, unit) = retained_warning_unit(source, &context);
    let unit = unit.with_interprocedural(registry, Some(profile));
    let warnings = find_taint_warnings_for_cu(&unit, registry, Some(profile));
    assert!(
        warnings
            .iter()
            .any(|warning| warning.code == DiagCode::T100),
        "the retained branch expression and exact read occurrence select exec's tainted argument: {warnings:?}"
    );

    let mut detached = unit.clone();
    detached.ir_module.retained_source_bindings = None;
    let warnings = find_taint_warnings_for_cu(&detached, registry, Some(profile));
    assert!(
        warnings
            .iter()
            .all(|warning| warning.code != DiagCode::T100),
        "the same spelling without the retained evaluation/read owner cannot select the sink value: {warnings:?}"
    );

    let clean_source = "proc f {} { set x [gets stdin]; if {[string length $x] > 0} { return 1 } }";
    let (_, clean_unit) = retained_warning_unit(clean_source, &context);
    let clean_unit = clean_unit.with_interprocedural(registry, Some(profile));
    let warnings = find_taint_warnings_for_cu(&clean_unit, registry, Some(profile));
    assert!(
        warnings
            .iter()
            .all(|warning| warning.code != DiagCode::T100),
        "the actual selected sanitizer remains clean in a branch expression: {warnings:?}"
    );
}

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

fn source_regexp_warnings(
    unit: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
    source: TaintSourceContext<'_>,
) -> Vec<TaintWarning> {
    let statement = unit.ir_module.top_level.statements.last().unwrap();
    let tokens = statement.tokens().unwrap();
    let uses = HashMap::new();
    let taints: HashMap<ValueKey, TaintLattice> = HashMap::new();
    let scan = TaintScan {
        uses: &uses,
        taints: &taints,
        ssa: &unit.top_level.ssa,
        quoted_uses: None,
        source_read: None,
        source,
    };
    let mut warnings = Vec::new();
    emit_regexp_pattern_warnings(
        &scan,
        statement.span(),
        registry,
        &mut warnings,
        Some(tokens),
        source,
    );
    warnings
}

#[test]
fn whole_unit_warning_query_requires_its_retained_positioned_source_owner() {
    // naming.source.retained-positioned-command-realm
    // docs/design/analysis/name-resolution-proofs/retained-positioned-command-realm.md
    // Explicit source ingress supplies the tainted value; no external evaluation is performed.
    let current =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = current.commands();
    let (_owner, unit) = retained_warning_unit(
        "set payload [gets stdin]\nregexp -- $payload haystack",
        &current,
    );
    let positive = find_taint_warnings_for_cu_base(&unit, registry, None);
    assert!(
        positive
            .iter()
            .any(|warning| { warning.code == DiagCode::T103 && warning.variable == "payload" }),
        "the actual original source keeps its warning: {positive:?}"
    );
    let mut missing = unit.clone();
    missing.ir_module.retained_source_bindings = None;
    let mut changed = unit.clone();
    changed.ir_module.source = tcl_lexer::SourceImage::document("regexp -- fixed haystack");
    let mut missing_snapshot = unit.clone();
    missing_snapshot.ir_module.registry_snapshot = None;
    for refused in vec![missing, changed, missing_snapshot].into_boxed_slice() {
        assert_eq!(
            find_taint_warnings_for_cu_base(&refused, registry, None),
            [] as [crate::taint::TaintWarning; 0]
        );
    }
    assert_eq!(
        find_taint_warnings_for_cu_base(&unit, registry, None),
        positive
    );
}

#[test]
fn actual_regexp_warning_refuses_missing_and_foreign_source_metadata() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // A retained literal terminator establishes source pattern topology;
    // the original variable read remains unknown, with no observed value.
    let current =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = current.commands();
    let (_owner, unit) = retained_warning_unit("regexp -- $payload haystack", &current);
    let source =
        TaintSourceContext::for_module_function(registry, &unit.ir_module, &unit.top_level);
    assert!(
        source_regexp_warnings(&unit, registry, source)
            .iter()
            .any(|warning| warning.code == DiagCode::T103 && warning.variable == "payload")
    );
    let missing = TaintSourceContext::for_input(registry, None, source.config);
    assert_eq!(
        source_regexp_warnings(&unit, registry, missing),
        [] as [crate::taint::TaintWarning; 0]
    );
    let input = unit.top_level.source_metadata_input().unwrap();
    let foreign = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
        input.lexer_config(),
    );
    let foreign = TaintSourceContext::for_input(registry, Some(&foreign), source.config);
    assert_eq!(
        source_regexp_warnings(&unit, registry, foreign),
        [] as [crate::taint::TaintWarning; 0]
    );
    let mut stale = source.config;
    stale.strict_quoting = !stale.strict_quoting;
    let stale = TaintSourceContext::for_input(registry, Some(input), stale);
    assert_eq!(
        source_regexp_warnings(&unit, registry, stale),
        [] as [crate::taint::TaintWarning; 0]
    );
    let standalone = TaintSourceContext::standalone(registry, registry.profile());
    assert_ne!(
        source_regexp_warnings(&unit, registry, standalone),
        [] as [crate::taint::TaintWarning; 0]
    );
}

#[test]
fn constant_command_head_candidates_use_retained_escape_grammar() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // SSA source-value candidates only; no command lookup, target identity
    // or actual invocation follows from the decoded presentation value.
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    for (escapes, expected) in [
        (tcl_dialect::EscapeSyntax::Tcl84, "U0001f600"),
        (tcl_dialect::EscapeSyntax::Tcl90, "😀"),
    ] {
        let config = tcl_lexer::LexerConfig {
            escapes,
            ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
        };
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            config,
        );
        let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
            "set command \\U0001f600\n$command argument",
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
        let statement = unit.ir_module.top_level.statements.last().unwrap();
        let heads = resolve_taint_command_heads(
            "$command",
            statement,
            None,
            Some(CommandHeadContext {
                fu: &unit.top_level,
                module_traces: crate::compilation_unit::ModuleTraceFacts {
                    traced_variables: &unit.ir_module.traced_variables,
                    has_dynamic_variable_trace: unit.ir_module.has_dynamic_variable_trace,
                    deferred_writes: &crate::ir::NO_DEFERRED_WRITES,
                },
                identities: crate::realm::CommandBindingRealm::none(),
            }),
        );
        assert_eq!(
            heads
                .iter()
                .map(|head| head.command.as_str())
                .collect::<Vec<_>>(),
            [expected]
        );
    }
}

#[test]
fn actual_taint_lattice_keeps_availability_and_refuses_missing_source_owners() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Source-lattice result and owner refusals, with an independently kept
    // Native entry. This test does not execute gets or observe its result.
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let mut catalogue = baseline.commands().project_for_profile(profile);
    let mut descriptor = catalogue.get("string").unwrap().clone();
    descriptor.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    catalogue.insert(descriptor);
    let current = std::sync::Arc::new(baseline.with_command_store(std::sync::Arc::new(catalogue)));
    let registry = current.commands();
    let (_owner, native) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let entry = crate::command_binding::SourceAnalysisEntry {
        native_entry: Some(std::sync::Arc::new(native)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    let mut unit = crate::compilation_unit::CompilationUnit::build_with_context_registry(
        "set measured [string length [gets stdin]]",
        crate::compilation_unit::UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        std::sync::Arc::clone(&current),
    );
    assert!(!measured_source_taint(&unit, registry).is_tainted());
    let input = unit.ir_module.source_metadata_input.clone().unwrap();
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
        unit.ir_module.source_metadata_input = changed.clone();
        unit.top_level.source_metadata_input = changed;
        assert!(measured_source_taint(&unit, registry).is_tainted());
    }
    unit.top_level.source_metadata_input = Some(input.clone());
    assert!(
        measured_source_taint(&unit, registry).is_tainted(),
        "missing Module owner"
    );
    unit.ir_module.source_metadata_input = Some(input.clone());
    unit.ir_module.lexer_config.strict_quoting = !input.lexer_config().strict_quoting;
    assert!(
        measured_source_taint(&unit, registry).is_tainted(),
        "stale Module grammar"
    );
    unit.ir_module.lexer_config = input.lexer_config();
    unit.ir_module.dialect_profile = Some(tcl_dialect::DialectProfile::find("tcl9.1").unwrap());
    assert!(
        measured_source_taint(&unit, registry).is_tainted(),
        "stale Module profile"
    );
    unit.ir_module.dialect_profile = Some(input.unit_profile());
    assert!(!measured_source_taint(&unit, registry).is_tainted());
}

#[test]
fn actual_warning_wrapper_uses_the_retained_context_and_lexer() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // The sink's nested wrapper uses the same source-lattice owner as an
    // assigned value. No source-channel execution or mitigation is observed.
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let current =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = current.commands();
    let (_owner, native) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let entry = crate::command_binding::SourceAnalysisEntry {
        native_entry: Some(std::sync::Arc::new(native)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    let unit = crate::compilation_unit::CompilationUnit::build_with_context_registry(
        "set payload [gets stdin]; puts [string length $payload]",
        crate::compilation_unit::UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        std::sync::Arc::clone(&current),
    );
    let tokens = unit
        .ir_module
        .top_level
        .retained_source_tokens_for_statement(unit.ir_module.top_level.statements.last().unwrap())
        .unwrap();
    let args = vec![tokens.argv_texts.last().unwrap().clone()];
    let source =
        TaintSourceContext::for_module_function(registry, &unit.ir_module, &unit.top_level);
    let call = SinkCall {
        command: "puts",
        args: &args,
        registry,
        source,
        tokens: Some(tokens),
        effective: None,
        source_read: None,
        argument_offset: 0,
    };
    assert!(var_consumed_by_sanitiser(&call, "payload"));
    let missing = SinkCall {
        source: TaintSourceContext::for_input(registry, None, source.config),
        ..call
    };
    assert!(!var_consumed_by_sanitiser(&missing, "payload"));
    let mut stale = source.config;
    stale.strict_quoting = !stale.strict_quoting;
    let stale = SinkCall {
        source: TaintSourceContext::for_input(
            registry,
            unit.top_level.source_metadata_input(),
            stale,
        ),
        ..call
    };
    assert!(!var_consumed_by_sanitiser(&stale, "payload"));
}

#[test]
fn public_module_warning_projection_requires_matching_source_owners() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // These conditional source warnings retain metadata; no channel read
    // or output handler is executed by the source analysis.
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = context.commands();
    let (_owner, native) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let entry = crate::command_binding::SourceAnalysisEntry {
        native_entry: Some(std::sync::Arc::new(native)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    let mut unit = crate::compilation_unit::CompilationUnit::build_with_context_registry(
        "set payload [gets stdin]; puts $payload",
        crate::compilation_unit::UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        std::sync::Arc::clone(&context),
    );
    let input = unit.ir_module.source_metadata_input.clone().unwrap();
    let has_sink = |unit: &crate::compilation_unit::CompilationUnit| {
        let traces = crate::compilation_unit::ModuleTraceFacts {
            traced_variables: &unit.ir_module.traced_variables,
            has_dynamic_variable_trace: unit.ir_module.has_dynamic_variable_trace,
            deferred_writes: &crate::ir::NO_DEFERRED_WRITES,
        };
        find_taint_warnings_for_module_function(
            &unit.top_level,
            &unit.top_level.taints,
            &unit.top_level.sccp.executable_blocks,
            registry,
            &unit.ir_module,
            &HashSet::<String>::new(),
            traces,
        )
        .iter()
        .any(|warning| warning.code == DiagCode::T101)
    };
    assert!(has_sink(&unit));
    unit.ir_module.source_metadata_input = None;
    assert!(!has_sink(&unit), "missing module owner");
    unit.ir_module.source_metadata_input = Some(input.clone());
    unit.ir_module.lexer_config.strict_quoting = !input.lexer_config().strict_quoting;
    assert!(!has_sink(&unit), "stale module lexer");
    unit.ir_module.lexer_config = input.lexer_config();
    unit.ir_module.dialect_profile = Some(tcl_dialect::DialectProfile::find("tcl9.1").unwrap());
    assert!(!has_sink(&unit), "stale module profile");
    unit.ir_module.dialect_profile = Some(profile);
    assert!(has_sink(&unit));
}

#[test]
fn inline_sanitiser_mitigation_requires_its_retained_implementation() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let current =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = current.commands();
    for (body, warn) in [
        ("puts [string length $x]", false),
        (
            "rename string saved; proc string args {return [lindex $args end]}; puts [string length $x]",
            true,
        ),
        (
            "interp alias {} measure {} string length; puts [measure $x]",
            false,
        ),
        (
            "interp alias {} measure {} string length; rename string saved; proc string args {return [lindex $args end]}; puts [measure $x]",
            true,
        ),
    ] {
        let source = format!("set x [gets stdin]; {body}");
        let (_owner, native) =
            crate::environment_ingress::captured_native_entry_with_owner(profile);
        let entry = crate::command_binding::SourceAnalysisEntry {
            native_entry: Some(std::sync::Arc::new(native)),
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..Default::default()
        };
        let unit = crate::compilation_unit::CompilationUnit::build_with_context_registry(
            &source,
            crate::compilation_unit::UnitBuildOptions {
                registry,
                defer_top_level: false,
                config: tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            Some(&entry),
            std::sync::Arc::clone(&current),
        )
        .with_interprocedural(registry, Some(profile));
        let warnings = find_taint_warnings_for_cu(&unit, registry, Some(profile));
        assert_eq!(
            warnings
                .iter()
                .any(|warning| warning.code == DiagCode::T101),
            warn,
            "{source}: {warnings:?}"
        );
    }
}

#[test]
fn explicit_unknown_binding_cannot_recover_a_same_spelled_sink() {
    let source = "eval $x";
    let command = crate::segmenter::segment_commands(source).remove(0);
    let tokens = CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        tcl_lexer::LexerConfig::default(),
        &command,
    )
    .with_source_binding(crate::command_binding::SourceInvocationBinding::unknown());
    let mut statement = call_stmt("eval", &["$x"]);
    match &mut statement {
        Statement::Call { tokens: target, .. } => *target = Some(tokens),
        _ => panic!("call fixture"),
    }
    let warnings = warnings_for_tainted_sink(statement, &[("x", 1)]);
    assert!(
        warnings
            .iter()
            .all(|warning| warning.sink_command != "eval"),
        "{warnings:?}"
    );
}

#[test]
fn point_binding_supplies_sink_alias_prefix_once() {
    let registry = CommandRegistry::build_default();
    let module = crate::lowering::lower_to_ir(
        "interp alias {} invoke {} exec safe-program\ninvoke $x",
        &registry,
    );
    let tokens = module
        .top_level
        .statements
        .last()
        .unwrap()
        .tokens()
        .unwrap();
    let statement = call_stmt("invoke", &["$x"]);
    let heads = resolve_taint_command_heads("invoke", &statement, Some(tokens), None);
    assert_eq!(heads.len(), 1);
    assert_eq!(heads[0].command, "::exec");
    assert_eq!(
        heads[0].effective_args(&["$x".into()]),
        Some(vec!["safe-program".into(), "$x".into()])
    );
    let arguments = vec!["safe-program".into(), "$x".into()];
    let call = SinkCall {
        source: TaintSourceContext::standalone(&registry, registry.profile()),
        command: &heads[0].command,
        args: &arguments,
        registry: &registry,
        tokens: Some(tokens),
        effective: None,
        source_read: None,
        argument_offset: 0,
    };
    assert_eq!(sink_arg_span(&call, "x", statement.span()), tokens.argv[1]);
}

fn simple_sccp(blocks: &[BlockId]) -> SccpResult {
    SccpResult {
        required_math_invocations: Vec::new(),
        required_expression_preparations: Vec::new(),
        explanations: Vec::new(),
        route_tally: crate::value_transfer::RouteTally::default(),
        folded_types: HashMap::new(),
        preserved: HashMap::new(),
        raised: HashSet::default(),
        template_plans: Vec::new(),
        selections: Vec::new(),
        existence: HashMap::new(),
        existence_reads: HashMap::new(),
        existence_exits: HashMap::new(),
        existence_entries: HashMap::new(),
        refinements: Vec::new(),
        refinements_at: HashMap::new(),
        value_entries: HashMap::new(),
        query_places: Vec::new(),
        existence_guards: Vec::new(),
        loop_enumerations: Vec::new(),
        reads_module: false,
        completion: crate::sccp::RunCompletion::default(),
        values: HashMap::new(),
        executable_blocks: blocks.iter().copied().collect(),
        executable_edges: HashSet::new(),
        constant_branches: Vec::new(),
    }
}

/// Bare intra-procedural propagation with no optional inputs.
fn plain_taints(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    sccp: &SccpResult,
    registry: &CommandRegistry,
) -> HashMap<ValueKey, TaintLattice> {
    let instance_classes = local_instance_classes(cfg, registry);
    propagate_taints(
        &TaintGraph::new(cfg, ssa, sccp),
        TaintPropagationInputs {
            registry,
            rendered_props: None,
            interproc: None,
            source: TaintSourceContext::standalone(registry, None),
            param_taints: None,
            taint_summaries: None,
            instance_classes: &instance_classes,
        },
    )
}

#[test]
fn clean_and_tainted_constructors() {
    let c = TaintLattice::clean();
    assert!(!c.is_tainted());
    let t = TaintLattice::tainted();
    assert!(t.is_tainted());
}

fn incoming_global_unit(
    source: &str,
    context: &std::sync::Arc<tcl_registry::model::ContextRegistry>,
) -> (tcl_vm::Vm, crate::compilation_unit::CompilationUnit) {
    let registry = context.commands();
    let profile = registry.profile().unwrap();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(context),
        config,
    );
    let (owner, native) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let mut entry = crate::command_binding::SourceAnalysisEntry::for_supplied_source(
        registry,
        &input,
        config,
        Some(profile),
    );
    entry.native_entry = Some(std::sync::Arc::new(native));
    let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry,
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        &input,
    );
    (owner, unit)
}

#[test]
fn incoming_external_globals_keep_physical_identity_and_independent_provenance() {
    let context =
        crate::environment_ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = context.commands();
    let profile = registry.profile().unwrap();
    for (source, spelling, name, element) in [
        ("expr {$argv + 1}", "$argv", "argv", false),
        (
            "proc writer {} {set ::state 1}; expr {$argv + 1}; writer",
            "$argv",
            "argv",
            false,
        ),
        (
            "expr {$env(PATH) + 1}; proc writer {} {set ::state 1}; writer",
            "$env(PATH)",
            "env",
            true,
        ),
        ("expr {$::argv + 1}", "$::argv", "argv", false),
        (
            "upvar #0 argv argc; expr {$argc + 1}",
            "$argc",
            "argv",
            false,
        ),
    ] {
        let (_owner, unit) = incoming_global_unit(source, &context);
        let unit = unit.with_interprocedural(registry, Some(profile));
        let start = u32::try_from(source.find(spelling).unwrap()).unwrap();
        let span = Span::new(start, start + u32::try_from(spelling.len()).unwrap());
        let read = crate::ssa::original_source_read_at_unit(&unit, span, registry)
            .expect("actual retained top-level read");
        assert!(std::ptr::eq(read.function(), &raw const unit.top_level));
        let cell = read.function().ssa.cell_key(read.symbol()).unwrap();
        let global = cell
            .incoming_root_global(unit.ir_module.source_entry.native_entry.as_deref().unwrap())
            .expect("same-entry incoming root-global cell");
        assert_eq!(global.name().as_bytes(), name.as_bytes());
        assert_eq!(global.is_element(), element);
        let solved =
            crate::taint_interproc::solve_interprocedural_taints(&unit, registry, Some(profile));
        let incoming = solved.top_taints[&(read.symbol(), 0)];
        assert!(incoming.is_tainted(), "{source}: {incoming:?}");
        assert!(
            !incoming.is_conservative_global_seed_only(),
            "independent external source was lost: {source}: {incoming:?}"
        );
        // Unknown represented contents remain a separate warning source.
        assert!(
            find_taint_warnings_for_cu(&unit, registry, Some(profile))
                .iter()
                .any(|warning| warning.code == DiagCode::T100 && warning.span == span)
        );
    }
}

#[test]
fn incoming_global_projection_refuses_foreign_nonroot_and_recreated_storage() {
    let context =
        crate::environment_ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = context.commands();
    let profile = registry.profile().unwrap();
    let source = "expr {$argv + 1}";
    let (_owner, unit) = incoming_global_unit(source, &context);
    let read = crate::ssa::original_source_read_at_unit(&unit, Span::new(6, 11), registry)
        .expect("actual retained incoming read");
    let cell = read.function().ssa.cell_key(read.symbol()).unwrap();
    let entry = unit.ir_module.source_entry.native_entry.as_deref().unwrap();
    let (_foreign_owner, foreign) =
        crate::environment_ingress::captured_native_entry_with_owner(profile);
    assert!(cell.incoming_root_global(&foreign).is_none());
    // Refusal-only wrappers around the genuine cell issue no SSA or lifetime proof.
    let recreated = crate::var_resolve::VariableCellKey::Lifetime {
        source: 1,
        cell: Box::new(cell.clone()),
    };
    assert!(recreated.incoming_root_global(entry).is_none());
    let member = crate::var_resolve::VariableCellKey::Element {
        cell: Box::new(recreated),
        index: "key".into(),
    };
    assert!(member.incoming_root_global(entry).is_none());
    assert!(
        crate::var_resolve::VariableCellKey::Authored("argv".to_owned())
            .incoming_root_global(entry)
            .is_none()
    );
    let source = "namespace eval box {variable argv}; upvar #0 box::argv argc; expr {$argc + 1}";
    let (_owner, unit) = incoming_global_unit(source, &context);
    let start = u32::try_from(source.find("$argc").unwrap()).unwrap();
    let read =
        crate::ssa::original_source_read_at_unit(&unit, Span::new(start, start + 5), registry)
            .expect("actual retained non-root read");
    assert!(
        read.function()
            .ssa
            .cell_key(read.symbol())
            .unwrap()
            .incoming_root_global(unit.ir_module.source_entry.native_entry.as_deref().unwrap())
            .is_none()
    );
}

#[test]
fn incoming_external_globals_use_retained_availability_and_entry() {
    let context =
        crate::environment_ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = context.commands();
    let profile = registry.profile().unwrap();
    let (_owner, unit) = incoming_global_unit("expr {$argv + 1}", &context);
    let read = crate::ssa::original_source_read_at_unit(&unit, Span::new(6, 11), registry)
        .expect("actual retained incoming read");
    let mut missing_origin = HashMap::new();
    seed_entry_taints(
        &mut missing_origin,
        &unit.top_level.ssa,
        &unit.top_level.cfg,
        None,
        None,
        registry,
        TaintSourceContext::for_function(registry, &unit.top_level),
    );
    assert!(!missing_origin.contains_key(&(read.symbol(), 0)));
    let retained =
        TaintSourceContext::for_module_function(registry, &unit.ir_module, &unit.top_level);
    for separate_dialect in [None, Some(tcl_dialect::DialectProfile::irules())] {
        let mut context = retained;
        context.dialect = separate_dialect;
        let mut incoming = HashMap::new();
        seed_entry_taints(
            &mut incoming,
            &unit.top_level.ssa,
            &unit.top_level.cfg,
            None,
            None,
            registry,
            context,
        );
        assert!(incoming[&(read.symbol(), 0)].is_tainted());
    }
    let mut unavailable = retained;
    unavailable.metadata = None;
    assert!(unavailable.incoming_native_global_context().is_none());
    assert!(
        TaintSourceContext::standalone(registry, Some(profile))
            .incoming_native_global_context()
            .is_none()
    );
    let mut detached = unit.clone();
    detached.ir_module.source_entry.native_entry = None;
    assert!(
            TaintSourceContext::for_module_function(
                registry,
                &detached.ir_module,
                &detached.top_level,
            )
            .incoming_native_global_context()
            .is_none()
        );
    detached.ir_module.source_entry = unit.ir_module.source_entry.clone();
    detached.ir_module.source_entry.unknown_entry = true;
    assert!(
            TaintSourceContext::for_module_function(
                registry,
                &detached.ir_module,
                &detached.top_level,
            )
            .incoming_native_global_context()
            .is_none()
        );
}

#[test]
fn incoming_global_metadata_cannot_replace_genuine_formal_or_gets_sources() {
    let context =
        crate::environment_ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = context.commands();
    let profile = registry.profile().unwrap();
    let (_owner, unit) = incoming_global_unit("proc p {argv} {expr {$argv + 1}}", &context);
    let (function, symbol) = unit
        .all_body_function_units()
        .find_map(|function| {
            function.ssa.cell_symbols().find_map(|(symbol, cell)| {
                matches!(cell, crate::var_resolve::VariableCellKey::Activation { simple, .. }
                        if simple.as_bytes() == b"argv")
                .then_some((function, symbol))
            })
        })
        .expect("genuine retained formal cell; no exact diagnostic read is claimed");
    let source = TaintSourceContext::for_module_function(registry, &unit.ir_module, function);
    let mut clean = HashMap::new();
    seed_entry_taints(
        &mut clean,
        &function.ssa,
        &function.cfg,
        None,
        None,
        registry,
        source,
    );
    assert!(!clean.contains_key(&(symbol, 0)));
    let mut explicit = HashMap::new();
    seed_entry_taints(
        &mut explicit,
        &function.ssa,
        &function.cfg,
        None,
        Some(&HashMap::from([(
            "argv".to_owned(),
            TaintLattice::tainted(),
        )])),
        registry,
        source,
    );
    assert!(explicit[&(symbol, 0)].is_tainted());
    assert!(!explicit[&(symbol, 0)].is_conservative_global_seed_only());

    for (source, spelling) in [
        ("set argc [gets stdin]; expr {$argc + 1}", "$argc"),
        (
            "set argv SAFE; unset argv; set argv [gets stdin]; expr {$argv + 1}",
            "$argv",
        ),
    ] {
        let (_owner, unit) = incoming_global_unit(source, &context);
        let unit = unit.with_interprocedural(registry, Some(profile));
        let start = u32::try_from(source.find(spelling).unwrap()).unwrap();
        let read = crate::ssa::original_source_read_at_unit(
            &unit,
            Span::new(start, start + u32::try_from(spelling.len()).unwrap()),
            registry,
        )
        .expect("actual retained gets/rebind read");
        let version = read.version().expect("represented later contents");
        assert!(version > 0);
        let solved =
            crate::taint_interproc::solve_interprocedural_taints(&unit, registry, Some(profile));
        let value = solved.top_taints[&(read.symbol(), version)];
        assert!(value.is_tainted());
        assert!(!value.is_conservative_global_seed_only());
    }
}

#[test]
fn version_zero_seed_provenance_does_not_use_display_name() {
    let cfg = crate::cfg::Function::new("::top", "entry");
    let mut ssa = SsaFunction::trivial("::top", cfg.entry, cfg.block_names().to_vec());
    let symbol = ssa.intern_var("::formal");
    let uses = HashMap::from([(symbol, 0)]);
    let global_seed = HashMap::from([((symbol, 0), TaintLattice::conservative_global_seed())]);
    let formal_entry = HashMap::from([((symbol, 0), TaintLattice::tainted())]);
    assert!(!join_uses(&uses, &global_seed).is_tainted());
    assert!(join_uses(&uses, &formal_entry).is_tainted());
    assert!(global_seed[&(symbol, 0)].is_conservative_global_seed_only());
    assert!(
        !global_seed[&(symbol, 0)]
            .join(TaintLattice::tainted())
            .is_conservative_global_seed_only()
    );
}

#[test]
fn source_formal_seed_requires_the_matching_retained_body() {
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
    let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
        "proc run {formal} {set local $formal}",
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
    );
    let function = unit.procedures.values().next().unwrap();
    let source = TaintSourceContext::for_module_function(registry, &unit.ir_module, function);
    let formal = function.ssa.var_symbol("formal").unwrap();
    let local = function.ssa.var_symbol("local").unwrap();
    assert!(is_actual_formal_entry(
        &function.ssa,
        formal,
        "formal",
        source
    ));
    assert!(!is_actual_formal_entry(
        &function.ssa,
        local,
        "local",
        source
    ));
    let foreign_body =
        TaintSourceContext::for_module_function(registry, &unit.ir_module, &unit.top_level);
    assert!(!is_actual_formal_entry(
        &function.ssa,
        formal,
        "formal",
        foreign_body
    ));
}

#[test]
fn join_propagates_taint_intersects_mitigations() {
    // Two TAINTED operands: taint unions, mitigations are must-have so only
    // the colour present on both survives the intersection.
    let a = TaintLattice::from_colours(
        TaintColour::TAINTED | TaintColour::CRLF_FREE | TaintColour::PATH_PREFIXED,
    );
    let b = TaintLattice::from_colours(
        TaintColour::TAINTED | TaintColour::CRLF_FREE | TaintColour::NON_DASH_PREFIXED,
    );
    let j = a.join(b);
    assert!(j.colours.contains(TaintColour::TAINTED));
    assert!(j.colours.contains(TaintColour::CRLF_FREE));
    assert!(!j.colours.contains(TaintColour::PATH_PREFIXED));
    assert!(!j.colours.contains(TaintColour::NON_DASH_PREFIXED));
}

#[test]
fn join_with_untainted_is_identity() {
    // A clean/untainted operand is the join identity: it contributes no
    // taint, so it must not dilute the tainted operand's mitigation
    // colours: joining with the annihilating empty set would strip
    // PATH_PREFIXED.
    let tainted = TaintLattice::from_colours(TaintColour::TAINTED | TaintColour::PATH_PREFIXED);
    assert_eq!(tainted.join(TaintLattice::clean()).colours, tainted.colours);
    assert_eq!(TaintLattice::clean().join(tainted).colours, tainted.colours);
    // An untainted operand that happens to carry colours is still the
    // identity — its taint contribution is nil, so it cannot remove a
    // mitigation from the tainted side.
    let untainted_coloured = TaintLattice::from_colours(TaintColour::CRLF_FREE);
    assert_eq!(tainted.join(untainted_coloured).colours, tainted.colours);
}

#[test]
fn with_and_sanitised() {
    let v = TaintLattice::tainted().with(TaintColour::CRLF_FREE);
    assert!(v.is_tainted());
    assert!(v.colours.contains(TaintColour::CRLF_FREE));
    let s = v.sanitised();
    assert!(!s.is_tainted());
    assert!(s.colours.contains(TaintColour::CRLF_FREE));
}

#[test]
fn t102_safe_mask_excludes_tainted() {
    assert!(!TaintColour::T102_SAFE.contains(TaintColour::TAINTED));
    assert!(TaintColour::T102_SAFE.contains(TaintColour::PATH_PREFIXED));
}

#[test]
fn crlf_safe_mask_composition() {
    // `CRLF_SAFE = CRLF_FREE | IP_ADDRESS | PORT | FQDN`.
    // HEADER_TOKEN_SAFE only suppresses IRULE3002 in the name position
    // (handled by `irule3002_name_position_safe`); HTML_ESCAPED / URL_ENCODED
    // do not prove CRLF-injection safety in the value position.
    assert!(TaintColour::CRLF_SAFE.contains(TaintColour::CRLF_FREE));
    assert!(TaintColour::CRLF_SAFE.contains(TaintColour::IP_ADDRESS));
    assert!(TaintColour::CRLF_SAFE.contains(TaintColour::PORT));
    assert!(TaintColour::CRLF_SAFE.contains(TaintColour::FQDN));
    assert!(!TaintColour::CRLF_SAFE.contains(TaintColour::HEADER_TOKEN_SAFE));
    assert!(!TaintColour::CRLF_SAFE.contains(TaintColour::HTML_ESCAPED));
    assert!(!TaintColour::CRLF_SAFE.contains(TaintColour::URL_ENCODED));
    assert!(!TaintColour::CRLF_SAFE.contains(TaintColour::TAINTED));
}

/// A deeply-nested `[a [a [a … x]]]` command-substitution word and a
/// deeply-nested `ExprNode` tree, plus a minimal [`TaintCtx`], for the
/// two ctx-taking recursion caps below.
fn deep_bracket_word(depth: usize) -> String {
    format!("{}x{}", "[a ".repeat(depth), "]".repeat(depth))
}

/// `word_taint` recurses once per nested `[cmd …]` command substitution
/// inside a single word's raw text (Tier 1B), and `expr_command_taint`
/// recurses once per `ExprNode` operator-tree level (Tier 1A) — both
/// genuinely unbounded axes, independent of any statement-tree cap.
/// Uncapped, each overflows the native stack (SIGABRT) in the low
/// thousands of levels
/// on a 2 MiB thread (`cargo test`'s default). 3000 is comfortably past
/// that crash range and past both caps (256); the assertion is that each
/// returns at all.
#[test]
fn deeply_nested_taint_walks_survive() {
    use crate::cfg::Function;
    use crate::ssa::SsaFunction;

    let registry = CommandRegistry::build_default();
    let cfg = Function::new("::top", "entry");
    let entry = cfg.entry;
    let ssa = SsaFunction::trivial("::top", entry, cfg.block_names().to_vec());
    let uses: HashMap<Symbol, u32> = HashMap::new();
    let taints: HashMap<ValueKey, TaintLattice> = HashMap::new();
    let ctx = TaintCtx {
        registry: &registry,
        ssa: &ssa,
        interproc: None,
        known_procs: None,
        dialect: None,
        source: TaintSourceContext::standalone(&registry, None),
        taint_summaries: None,
        instance_classes: None,
        source_position: None,
        source_tokens: None,
        word_offset: None,
        source_word: None,
    };

    // Tier 1B: nested `[a [a [a … x]]]` command-substitution text.
    let _ = word_taint(&deep_bracket_word(3000), &uses, &taints, ctx);

    // Tier 1A: a 3000-deep `ExprNode` tree (nested unary `!`).
    let mut node = ExprNode::Literal {
        text: "1".into(),
        start: 0,
        end: 1,
    };
    for _ in 0..3000 {
        node = ExprNode::Unary {
            op: crate::expr_ast::UnaryOp::Not,
            operand: Box::new(node),
        };
    }
    let _ = expr_command_taint(&node, &uses, &taints, ctx, None, 0);
}

#[test]
fn raw_coercion_references_use_current_grammar_and_keep_empty_name_data() {
    for label in ["tcl8.6", "tcl9.1"] {
        let registry = tcl_registry::model::ingress::static_context_for(label).commands();
        let source = TaintSourceContext::standalone(registry, registry.profile());
        let grammar = source
            .config
            .grammar_over(registry.profile().unwrap().grammar);
        for (text, name) in [
            ("${$literal}", "$literal"),
            ("${}", ""),
            ("$arr(key)", "arr"),
        ] {
            let names = HashSet::from([name.to_owned()]);
            assert!(holds_one_whole_var_ref(text, &names, source.config));
            assert!(!holds_one_whole_var_ref(
                text,
                &HashSet::from(["donated".to_owned()]),
                source.config
            ));
            let mut hits = Vec::new();
            collect_coercion_operands(
                &ExprNode::Raw {
                    text: text.to_owned(),
                },
                false,
                &mut hits,
                0,
                (source.config, grammar),
            );
            assert!(hits.is_empty(), "{label}: {text}");
        }
        let names = HashSet::from(["a{b}c".to_owned()]);
        assert_eq!(
            holds_one_whole_var_ref("${a{b}c}", &names, source.config),
            label == "tcl9.1"
        );
        for text in ["prefix$x", "$x + 1", "${x", r"\$x", "${x\0tail}"] {
            assert!(!holds_one_whole_var_ref(
                text,
                &HashSet::from(["x".to_owned()]),
                source.config
            ));
        }
        let mut hits = Vec::new();
        collect_coercion_operands(
            &ExprNode::Var {
                text: "${}".into(),
                name: String::new(),
                start: 0,
                end: 2,
            },
            true,
            &mut hits,
            0,
            (source.config, grammar),
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "");
        assert!(hits[0].variable.is_some());
        assert_eq!(hits[0].rel_span, Some((0, 3)));
    }
}

/// `extract_guard_var` and `collect_coercion_operands` each recurse once
/// per `ExprNode` level, so both need a depth cap. 3000-deep trees are
/// past the crash range and past `MAX_EXPR_NODE_DEPTH` (256); the
/// assertion is that each returns at all.
#[test]
fn deeply_nested_expr_guard_and_coercion_walks_survive() {
    // A 3000-deep nested-unary tree drives both walkers' recursion.
    let mut node = ExprNode::Var {
        text: "$x".into(),
        name: "x".into(),
        start: 0,
        end: 2,
    };
    for _ in 0..3000 {
        node = ExprNode::Unary {
            op: crate::expr_ast::UnaryOp::Not,
            operand: Box::new(node),
        };
    }
    let _ = extract_guard_var(&node, false, 0, tcl_lexer::LexerConfig::default());
    let mut out = Vec::new();
    collect_coercion_operands(
        &node,
        true,
        &mut out,
        0,
        (
            tcl_lexer::LexerConfig::default(),
            tcl_dialect::LexerGrammar::default(),
        ),
    );
}

#[test]
fn propagate_taints_gets_is_source() {
    use crate::cfg::{Function, Terminator};
    use crate::ssa::{SsaBlock, SsaFunction};
    use tcl_lexer::Span;

    let registry = CommandRegistry::build_default();

    // Minimal CFG: entry block with `set x [gets stdin]`.
    let stmt = Statement::AssignValue {
        span: Span::new(0, 20),
        name: "x".into(),
        name_braced: false,
        value: "[gets stdin]".into(),
        value_needs_backsubst: false,
        tokens: None,
    };
    let mut cfg = Function::new("::top", "entry");
    let entry = cfg.entry;
    cfg.blocks
        .get_mut(&entry)
        .unwrap()
        .statements
        .push(stmt.clone());
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
    let x = ssa.intern_var("x");
    let ssa_stmt = ssa_stmt(stmt, HashMap::new(), [(x, 1u32)].into_iter().collect());
    ssa.blocks.insert(
        entry,
        SsaBlock {
            name: "entry".into(),
            phis: Vec::new(),
            statements: vec![ssa_stmt],
            entry_versions: HashMap::new(),
            exit_versions: HashMap::new(),
        },
    );

    let sccp = simple_sccp(&[entry]);
    let taints = plain_taints(&cfg, &ssa, &sccp, &registry);
    assert!(
        taints.get(&(x, 1)).is_some_and(|t| t.is_tainted()),
        "gets stdin result should be tainted"
    );
}

#[test]
fn find_taint_warnings_eval_with_tainted_var() {
    use crate::cfg::{Function, Terminator};
    use crate::ssa::{SsaBlock, SsaFunction};
    use tcl_lexer::Span;

    let registry = CommandRegistry::build_default();

    // Statements: set x [gets stdin]; eval $x
    let assign = Statement::AssignValue {
        span: Span::new(0, 12),
        name: "x".into(),
        name_braced: false,
        value: "[gets stdin]".into(),
        value_needs_backsubst: false,
        tokens: None,
    };
    let eval_call = Statement::Call {
        span: Span::new(13, 20),
        command: "eval".into(),
        canonical_command: None,
        args: vec!["$x".into()],
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
        .push(eval_call.clone());
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
    let x = ssa.intern_var("x");
    let ssa_assign = ssa_stmt(assign, HashMap::new(), [(x, 1u32)].into_iter().collect());
    let ssa_eval = ssa_stmt(eval_call, [(x, 1u32)].into_iter().collect(), HashMap::new());

    ssa.blocks.insert(
        entry,
        SsaBlock {
            name: "entry".into(),
            phis: Vec::new(),
            statements: vec![ssa_assign, ssa_eval],
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
            .any(|w| w.code == DiagCode::T100 && w.variable == "x"),
        "expected T100 for tainted $x passed to eval, got {warnings:?}"
    );
}

/// Wire `set x [gets stdin]` (a taint source) followed by `sink`
/// (which uses `$x`) and return the taint warnings.  Shared by the
/// T104 / T105 sink tests.
fn warnings_for_tainted_sink(sink: Statement, sink_uses: &[(&str, u32)]) -> Vec<TaintWarning> {
    use crate::cfg::{Function, Terminator};
    use crate::ssa::{SsaBlock, SsaFunction};
    use tcl_lexer::Span;

    let registry = CommandRegistry::build_default();
    let assign = Statement::AssignValue {
        span: Span::new(0, 12),
        name: "x".into(),
        name_braced: false,
        value: "[gets stdin]".into(),
        value_needs_backsubst: false,
        tokens: None,
    };
    let mut cfg = Function::new("::top", "entry");
    let entry = cfg.entry;
    {
        let b = cfg.blocks.get_mut(&entry).unwrap();
        b.statements.push(assign.clone());
        b.statements.push(sink.clone());
        b.terminator = Some(Terminator::Return {
            expr_base: None,
            tokens: None,
            value: None,
            value_word: None,
            span: None,
            expr: None,
            braced: false,
        });
    }
    let mut ssa = SsaFunction::trivial("::top", entry, cfg.block_names().to_vec());
    let ssa_assign = ssa_stmt(
        assign,
        HashMap::new(),
        [(ssa.intern_var("x"), 1u32)].into_iter().collect(),
    );
    let ssa_sink = ssa_stmt(
        sink,
        sink_uses
            .iter()
            .map(|&(n, v)| (ssa.intern_var(n), v))
            .collect(),
        HashMap::new(),
    );
    ssa.blocks.insert(
        entry,
        SsaBlock {
            name: "entry".into(),
            phis: Vec::new(),
            statements: vec![ssa_assign, ssa_sink],
            entry_versions: HashMap::new(),
            exit_versions: HashMap::new(),
        },
    );
    let sccp = simple_sccp(&[entry]);
    let taints = plain_taints(&cfg, &ssa, &sccp, &registry);
    find_taint_warnings(
        &cfg,
        &ssa,
        &taints,
        &sccp.executable_blocks,
        &registry,
        None,
        &HashSet::new(),
    )
}

fn call_stmt(command: &str, args: &[&str]) -> Statement {
    Statement::Call {
        span: tcl_lexer::Span::new(13, 30),
        command: command.into(),
        canonical_command: None,
        args: args.iter().map(|a| (*a).to_string()).collect(),
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    }
}

#[test]
fn t104_ssrf_for_tainted_socket_address() {
    // `socket $x 80` with tainted `$x` → SSRF (T104).
    let sink = call_stmt("socket", &["$x", "80"]);
    let w = warnings_for_tainted_sink(sink, &[("x", 1)]);
    assert!(
        w.iter()
            .any(|w| w.code == DiagCode::T104 && w.variable == "x"),
        "expected T104 for tainted socket address; got {w:?}"
    );
}

#[test]
fn t105_cross_interp_for_tainted_interp_eval() {
    // `interp eval $child $x` with tainted `$x` → cross-interp (T105).
    let sink = call_stmt("interp", &["eval", "$child", "$x"]);
    let w = warnings_for_tainted_sink(sink, &[("x", 1)]);
    let t105 = w
        .iter()
        .find(|w| w.code == DiagCode::T105 && w.variable == "x");
    assert!(t105.is_some(), "expected T105 for interp eval; got {w:?}");
    assert_eq!(t105.unwrap().sink_command, "interp eval");
}

#[test]
fn t105_cross_interp_for_tainted_console_eval() {
    // `console eval $x` with tainted `$x` → cross-interp (T105): the
    // script runs in the separate console interpreter, the same
    // category as `interp eval`.
    let sink = call_stmt("console", &["eval", "$x"]);
    let w = warnings_for_tainted_sink(sink, &[("x", 1)]);
    let t105 = w
        .iter()
        .find(|w| w.code == DiagCode::T105 && w.variable == "x");
    assert!(t105.is_some(), "expected T105 for console eval; got {w:?}");
    assert_eq!(t105.unwrap().sink_command, "console eval");
}

#[test]
fn t105_cross_interp_for_tainted_consoleinterp_eval_and_record() {
    // `consoleinterp eval $x` / `consoleinterp record $x` with tainted
    // `$x` → cross-interp (T105): both cross back into the interpreter
    // the console is attached to.
    for sub in ["eval", "record"] {
        let sink = call_stmt("consoleinterp", &[sub, "$x"]);
        let w = warnings_for_tainted_sink(sink, &[("x", 1)]);
        let t105 = w
            .iter()
            .find(|w| w.code == DiagCode::T105 && w.variable == "x");
        assert!(
            t105.is_some(),
            "expected T105 for consoleinterp {sub}; got {w:?}"
        );
        assert_eq!(t105.unwrap().sink_command, format!("consoleinterp {sub}"));
    }
}

/// A hand-built SSA statement with no may-defs, quoted uses or name-only
/// uses — the shape every fixture in this module wants.
fn ssa_stmt(
    statement: Statement,
    uses: HashMap<crate::ssa::Symbol, u32>,
    defs: HashMap<crate::ssa::Symbol, u32>,
) -> crate::ssa::SsaStatement {
    crate::ssa::SsaStatement {
        statement,
        uses,
        defs,
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    }
}

#[test]
fn destruction_versions_preserve_predecessor_taint_without_sanitising() {
    use crate::cfg::Function;
    use crate::ssa::{SsaBlock, SsaFunction};
    let registry = CommandRegistry::build_default();
    let cfg = Function::new("::top", "entry");
    let mut ssa = SsaFunction::trivial("::top", cfg.entry, cfg.block_names().to_vec());
    let symbol = ssa.intern_var("payload");
    let ctx = TaintCtx {
        registry: &registry,
        ssa: &ssa,
        interproc: None,
        known_procs: None,
        dialect: None,
        source: TaintSourceContext::standalone(&registry, None),
        taint_summaries: None,
        instance_classes: None,
        source_position: None,
        source_tokens: None,
        word_offset: None,
        source_word: None,
    };
    for conditional in [false, true] {
        for predecessor in [
            TaintLattice::clean(),
            TaintLattice::tainted(),
            TaintLattice::tainted().with(TaintColour::CRLF_FREE),
        ] {
            let mut statement = ssa_stmt(
                call_stmt("unset", &["payload"]),
                HashMap::from([(symbol, 1)]),
                HashMap::from([(symbol, 2)]),
            );
            statement.destruction_defs.insert(symbol);
            if conditional {
                statement.may_defs.insert(symbol);
            }
            let block = SsaBlock {
                name: "entry".into(),
                phis: Vec::new(),
                statements: vec![statement],
                entry_versions: HashMap::new(),
                exit_versions: HashMap::new(),
            };
            let mut taints = HashMap::from([((symbol, 1), predecessor)]);
            propagate_statement_taints(&mut taints, cfg.entry, &block, ctx, &ssa, None);
            assert_eq!(
                taints[&(symbol, 2)],
                predecessor,
                "conditional={conditional}: unset supplies neither cleaning nor new mitigation"
            );
            let uses = HashMap::from([(symbol, 2)]);
            assert_eq!(
                word_taint("$payload", &uses, &taints, ctx),
                predecessor,
                "the downstream script operand keeps its possible predecessor influence"
            );
        }
    }
}

#[test]
fn t106_double_encode_through_uri_encode() {
    // `set x [URI::encode $tainted]` stamps URL_ENCODED on x; passing
    // x back through `URI::encode` double-encodes → T106.
    use crate::cfg::{Function, Terminator};
    use crate::ssa::{SsaBlock, SsaFunction};
    use tcl_lexer::Span;

    let mut registry = CommandRegistry::build_default();
    registry.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));

    // set x [gets stdin]      (taint source)
    // set y [URI::encode $x]  (x tainted → y URL_ENCODED)
    // URI::encode $y          (y already URL_ENCODED → T106)
    let s0 = Statement::AssignValue {
        span: Span::new(0, 12),
        name: "x".into(),
        name_braced: false,
        value: "[gets stdin]".into(),
        value_needs_backsubst: false,
        tokens: None,
    };
    let s1 = Statement::AssignValue {
        span: Span::new(13, 35),
        name: "y".into(),
        name_braced: false,
        value: "[URI::encode $x]".into(),
        value_needs_backsubst: false,
        tokens: None,
    };
    let s2 = call_stmt("URI::encode", &["$y"]);

    let mut cfg = Function::new("::top", "entry");
    let entry = cfg.entry;
    {
        let b = cfg.blocks.get_mut(&entry).unwrap();
        b.statements.push(s0.clone());
        b.statements.push(s1.clone());
        b.statements.push(s2.clone());
        b.terminator = Some(Terminator::Return {
            expr_base: None,
            tokens: None,
            value: None,
            value_word: None,
            span: None,
            expr: None,
            braced: false,
        });
    }
    let mut ssa = SsaFunction::trivial("::top", entry, cfg.block_names().to_vec());
    let x = ssa.intern_var("x");
    let y = ssa.intern_var("y");
    let ssa_s0 = ssa_stmt(s0, HashMap::new(), [(x, 1u32)].into_iter().collect());
    let ssa_s1 = ssa_stmt(
        s1,
        [(x, 1u32)].into_iter().collect(),
        [(y, 1u32)].into_iter().collect(),
    );
    let ssa_s2 = ssa_stmt(s2, [(y, 1u32)].into_iter().collect(), HashMap::new());
    ssa.blocks.insert(
        entry,
        SsaBlock {
            name: "entry".into(),
            phis: Vec::new(),
            statements: vec![ssa_s0, ssa_s1, ssa_s2],
            entry_versions: HashMap::new(),
            exit_versions: HashMap::new(),
        },
    );
    let sccp = simple_sccp(&[entry]);
    let taints = plain_taints(&cfg, &ssa, &sccp, &registry);
    // The transform colour must have propagated to y.
    assert!(
        taints
            .get(&(y, 1))
            .is_some_and(|t| t.colours.intersects(TaintColour::URL_ENCODED)),
        "y should carry URL_ENCODED after URI::encode; got {:?}",
        taints.get(&(y, 1))
    );
    let warnings = find_taint_warnings(
        &cfg,
        &ssa,
        &taints,
        &sccp.executable_blocks,
        &registry,
        Some(tcl_dialect::DialectProfile::irules()),
        &HashSet::new(),
    );
    let t106 = warnings
        .iter()
        .find(|w| w.code == DiagCode::T106 && w.variable == "y");
    assert!(
        t106.is_some(),
        "expected T106 double-encode; got {warnings:?}"
    );
    assert!(
        t106.unwrap().message.contains("already URL-encoded"),
        "{:?}",
        t106.unwrap().message
    );
}

/// Run W313 over a single-block function whose SSA statements are built by
/// `build` against the function's variable interner.
fn w313_warnings(
    build: impl FnOnce(&mut SsaFunction) -> Vec<crate::ssa::SsaStatement>,
) -> Vec<TaintWarning> {
    use crate::cfg::{Function, Terminator};
    use crate::ssa::{SsaBlock, SsaFunction};

    let registry = CommandRegistry::build_default();
    let mut cfg = Function::new("::top", "entry");
    let entry = cfg.entry;
    let mut ssa = SsaFunction::trivial("::top", entry, cfg.block_names().to_vec());
    let ssa_stmts = build(&mut ssa);
    {
        let b = cfg.blocks.get_mut(&entry).unwrap();
        for s in &ssa_stmts {
            b.statements.push(s.statement.clone());
        }
        b.terminator = Some(Terminator::Return {
            expr_base: None,
            tokens: None,
            value: None,
            value_word: None,
            span: None,
            expr: None,
            braced: false,
        });
    }
    ssa.blocks.insert(
        entry,
        SsaBlock {
            name: "entry".into(),
            phis: Vec::new(),
            statements: ssa_stmts,
            entry_versions: HashMap::new(),
            exit_versions: HashMap::new(),
        },
    );
    let sccp = simple_sccp(&[entry]);
    let taints = plain_taints(&cfg, &ssa, &sccp, &registry);
    find_destructive_file_warnings(
        &cfg,
        &ssa,
        &taints,
        &sccp.executable_blocks,
        &registry,
        None,
    )
}

fn file_call(args: &[&str]) -> Statement {
    Statement::Call {
        span: tcl_lexer::Span::new(0, 20),
        command: "file".into(),
        canonical_command: Some("::file".into()),
        args: args.iter().map(|a| (*a).to_string()).collect(),
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    }
}

#[test]
fn w313_variable_path_to_file_delete() {
    use crate::ssa::SsaStatement;
    let w = w313_warnings(|ssa| {
        vec![SsaStatement {
            statement: file_call(&["delete", "$p"]),
            uses: [(ssa.intern_var("p"), 1u32)].into_iter().collect(),
            defs: HashMap::new(),
            may_defs: std::collections::HashSet::new(),
            destruction_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
            name_only_uses: std::collections::HashSet::new(),
        }]
    });
    let d = w.iter().find(|w| w.code == DiagCode::W313).expect("W313");
    assert_eq!(d.variable, "p");
    assert!(
        d.message
            .contains("file delete with a variable path ($p) risks path-traversal"),
        "{}",
        d.message
    );
}

#[test]
fn w313_normalised_path_gets_softer_message() {
    use crate::ssa::SsaStatement;
    use tcl_lexer::Span;
    // set p [file normalize $base]; file delete $p
    let assign = Statement::AssignValue {
        span: Span::new(0, 25),
        name: "p".into(),
        name_braced: false,
        value: "[file normalize $base]".into(),
        value_needs_backsubst: false,
        tokens: None,
    };
    let w = w313_warnings(|ssa| {
        let base = ssa.intern_var("base");
        let p = ssa.intern_var("p");
        let s0 = ssa_stmt(
            assign,
            [(base, 0u32)].into_iter().collect(),
            [(p, 1u32)].into_iter().collect(),
        );
        let s1 = SsaStatement {
            statement: file_call(&["delete", "$p"]),
            uses: [(p, 1u32)].into_iter().collect(),
            defs: HashMap::new(),
            may_defs: std::collections::HashSet::new(),
            destruction_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
            name_only_uses: std::collections::HashSet::new(),
        };
        vec![s0, s1]
    });
    let d = w.iter().find(|w| w.code == DiagCode::W313).expect("W313");
    assert!(
        d.message.contains("file delete with normalised path ($p)"),
        "{}",
        d.message
    );
    // The `[string match …]` example must interpolate the variable
    // name (`${p}`), not render the literal placeholder `${name}`.
    assert!(
        d.message.contains("[string match \"$base/*\" ${p}]"),
        "{}",
        d.message
    );
    assert!(!d.message.contains("${name}"), "{}", d.message);
}

#[test]
fn w313_silent_for_literal_path() {
    use crate::ssa::SsaStatement;
    // `file delete /tmp/foo` has no variable path argument.
    assert_eq!(
        w313_warnings(|_ssa| {
            vec![SsaStatement {
                statement: file_call(&["delete", "/tmp/foo"]),
                uses: HashMap::new(),
                defs: HashMap::new(),
                may_defs: std::collections::HashSet::new(),
                destruction_defs: std::collections::HashSet::new(),
                quoted_uses: std::collections::HashSet::new(),
                name_only_uses: std::collections::HashSet::new(),
            }]
        })
        .len(),
        0
    );
}

#[test]
fn standalone_guard_names_follow_original_operand_kind_and_supplied_config() {
    for label in ["tcl8.6", "tcl9.1"] {
        let registry = tcl_registry::model::ingress::static_context_for(label).commands();
        let config = TaintSourceContext::standalone(registry, registry.profile()).config;
        for (source, expected) in [
            ("[string match SAFE ${$literal}]", Some("$literal")),
            ("[string match SAFE ${}]", Some("")),
            ("[string match SAFE {$literal}]", None),
            ("[string match SAFE {${literal}}]", None),
            (
                "[string first SAFE ${qualified::literal}]",
                Some("qualified::literal"),
            ),
            ("[string first SAFE $arr(key)]", None),
            ("[string equal SAFE ${arr(key)}]", None),
            ("[string equal SAFE prefix$x]", None),
            ("[string match SAFE {*}$x]", None),
        ] {
            assert_eq!(
                guard_var_from_string_command(source, config).as_deref(),
                expected,
                "{label}: {source}"
            );
        }
        assert_eq!(
            guard_var_from_string_command("[string match SAFE ${a{b}c}]", config).as_deref(),
            if label == "tcl9.1" {
                Some("a{b}c")
            } else {
                None
            }
        );
    }
}

#[test]
fn w313_helpers_parse_vars_and_guards() {
    assert!(arg_var_names("$p", tcl_dialect::BracedVarStyle::default()).contains("p"));
    assert!(arg_var_names("${dir}/$file", tcl_dialect::BracedVarStyle::default()).contains("dir"));
    assert!(arg_var_names("${dir}/$file", tcl_dialect::BracedVarStyle::default()).contains("file"));
    assert_eq!(
        extract_var_name("$p", tcl_lexer::LexerConfig::default()).as_deref(),
        Some("p")
    );
    assert_eq!(
        extract_var_name("${p}", tcl_lexer::LexerConfig::default()).as_deref(),
        Some("p")
    );
    assert_eq!(
        guard_var_from_string_command(
            "[string match \"$base/*\" $p]",
            tcl_lexer::LexerConfig::default()
        )
        .as_deref(),
        Some("p")
    );
    assert_eq!(
        guard_var_from_string_command(
            "[string equal -length 4 $a $p]",
            tcl_lexer::LexerConfig::default()
        )
        .as_deref(),
        Some("a")
    );
}

#[test]
fn classify_network_interp_sinks_maps_socket_and_interp() {
    let reg = CommandRegistry::build_default();
    assert_eq!(
        classify_network_interp_sinks(&reg, "socket", &["host".into(), "80".into()]),
        vec![(DiagCode::T104, "socket".to_owned())]
    );
    assert_eq!(
        classify_network_interp_sinks(&reg, "interp", &["eval".into(), "$c".into()]),
        vec![(DiagCode::T105, "interp eval".to_owned())]
    );
    // A non-eval interp subcommand and a plain command map to nothing.
    assert_eq!(
        classify_network_interp_sinks(&reg, "interp", &["share".into()]).len(),
        0
    );
    assert_eq!(
        classify_network_interp_sinks(&reg, "puts", &["hi".into()]).len(),
        0
    );
}

#[test]
fn const_assignment_is_not_tainted() {
    use crate::cfg::{Function, Terminator};
    use crate::ssa::{SsaBlock, SsaFunction};
    use tcl_lexer::Span;

    let registry = CommandRegistry::build_default();

    let stmt = Statement::AssignConst {
        span: Span::new(0, 10),
        name: "x".into(),
        name_braced: false,
        value: "hello".into(),
        value_span: None,
    };
    let mut cfg = Function::new("::top", "entry");
    let entry = cfg.entry;
    cfg.blocks
        .get_mut(&entry)
        .unwrap()
        .statements
        .push(stmt.clone());
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
    let x = ssa.intern_var("x");
    let ssa_stmt = ssa_stmt(stmt, HashMap::new(), [(x, 1u32)].into_iter().collect());
    ssa.blocks.insert(
        entry,
        SsaBlock {
            name: "entry".into(),
            phis: Vec::new(),
            statements: vec![ssa_stmt],
            entry_versions: HashMap::new(),
            exit_versions: HashMap::new(),
        },
    );

    let sccp = simple_sccp(&[entry]);
    let taints = plain_taints(&cfg, &ssa, &sccp, &registry);
    assert!(
        taints.get(&(x, 1)).is_none_or(|t| !t.is_tainted()),
        "constant assignment should not be tainted"
    );
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

/// Taint propagates through an interpolated-string word that embeds a
/// tainted variable: `set out "prefix_${x}_suffix"` where `$x` is tainted.
#[test]
fn interpolated_string_word_propagates_taint() {
    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::CommandRegistry;

    let registry = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(
        "set x [gets stdin]\nset out \"prefix_${x}_suffix\"",
        &registry,
        false,
    );
    let fu = cu.function("::top").unwrap();
    let out_tainted = fu
        .taints
        .iter()
        .any(|((name, _ver), t)| fu.ssa.var_name(*name) == "out" && t.is_tainted());
    assert!(
        out_tainted,
        "expected 'out' to be tainted via interpolated string embedding tainted $x"
    );
}

/// Actual F5 getter properties survive an optional diagnostic dialect
/// filter; an unloaded catalogue spelling supplies no security properties.
#[test]
fn irules_http_uri_is_a_dialect_agnostic_source() {
    use crate::compilation_unit::CompilationUnit;

    let profile = tcl_dialect::DialectProfile::irules();
    let mut registry = CommandRegistry::build_default();
    registry.load_irules();
    let registry = registry.project_for_profile(profile);
    for dialect in [None, Some(profile)] {
        let cu = CompilationUnit::build_for_profile("set u [HTTP::uri]", &registry, false, profile)
            .with_interprocedural(&registry, dialect);
        let fu = cu.function("::top").unwrap();
        let u = fu
            .taints
            .iter()
            .find(|((n, _), _)| fu.ssa.var_name(*n) == "u")
            .map(|(_, t)| *t);
        assert!(
            u.is_some_and(TaintLattice::is_tainted),
            "HTTP::uri should be a taint source (dialect={dialect:?})",
        );
        assert!(
            u.is_some_and(|t| t.colours.contains(TaintColour::PATH_PREFIXED)),
            "HTTP::uri getter should carry PATH_PREFIXED (dialect={dialect:?})",
        );
    }
    let plain = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for("set u [HTTP::uri]", &plain, false);
    assert!(
        unit.top_level
            .taints
            .values()
            .all(|taint| { !taint.colours.contains(TaintColour::PATH_PREFIXED) }),
        "an unloaded F5 name must not donate getter safety properties"
    );
}

/// Inter-procedural: `proc id {x} { return $x }` + tainted actual
/// should taint the call's result because the passthrough parameter
/// forwards the tainted value through the proc.
#[test]
fn interprocedural_passthrough_transfers_taint() {
    use crate::compilation_unit::CompilationUnit;

    let registry = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(
        "proc ::id {x} { return $x }\n\
             set tainted [gets stdin]\n\
             set out [::id $tainted]",
        &registry,
        false,
    )
    .with_interprocedural(&registry, None);
    let fu = cu.function("::top").unwrap();
    assert!(
        fu.taints
            .iter()
            .any(|((n, _), t)| fu.ssa.var_name(*n) == "out" && t.is_tainted()),
        "expected 'out' to be tainted via passthrough proc: {:?}",
        fu.taints,
    );
}

/// Global-write seeding must be scoped to procs actually reachable
/// from the current function. An unrelated helper that writes to
/// `::state` should not taint `::other_global` in a function that
/// never calls it.
#[test]
fn global_write_seeding_is_scoped_to_reachable_callees() {
    use crate::compilation_unit::CompilationUnit;

    let registry = CommandRegistry::build_default();
    // `::writer` writes ::state. `::top` never calls ::writer, so
    // reads of other globals here must stay clean.
    let cu = CompilationUnit::build_for(
        "proc ::writer {} { set ::state 1 }\n\
             set local $::safe",
        &registry,
        false,
    )
    .with_interprocedural(&registry, None);
    let fu = cu.function("::top").unwrap();
    let local_tainted = fu
        .taints
        .iter()
        .any(|((n, _), t)| fu.ssa.var_name(*n) == "local" && t.is_tainted());
    assert!(
        !local_tainted,
        "`local` must stay clean — `::writer` is unreachable from top-level: {:?}",
        fu.taints,
    );
}

/// Rendered-property colouring: a value starting with `/` should
/// pick up `PATH_PREFIXED` (implying `NON_DASH_PREFIXED`).
#[test]
fn rendered_props_colour_slash_value_path_prefixed() {
    use crate::compilation_unit::CompilationUnit;

    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let cu = CompilationUnit::build_for("set path /etc/hosts", &registry, false);
    let fu = cu.function("::top").unwrap();
    let entry = fu
        .taints
        .iter()
        .find(|((n, _), _)| fu.ssa.var_name(*n) == "path")
        .expect("path taint entry");
    assert!(
        entry.1.colours.contains(TaintColour::PATH_PREFIXED),
        "expected PATH_PREFIXED colour on /-prefixed literal",
    );
}

// IRULE3001–3004 sink classifier + end-to-end detection

fn irules_warnings_for(source: &str) -> Vec<TaintWarning> {
    use crate::compilation_unit::CompilationUnit;
    // Mirror production (tcl-lsp-db): build_default + load the iRules
    // dialect, so the iRules command *specs* (with their subcommands and
    // taint_output_sink_subcommands) are present for the registry-driven,
    // prefix-aware sink classification.
    let mut registry = CommandRegistry::build_default();
    registry.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    let profile = tcl_dialect::DialectProfile::irules();
    let registry = registry.project_for_profile(profile);
    let cu = CompilationUnit::build_for_profile(source, &registry, false, profile)
        .with_interprocedural(&registry, Some(profile));
    let mut out: Vec<TaintWarning> = Vec::new();
    for fu in cu.analysable_functions() {
        out.extend(find_taint_warnings(
            &fu.cfg,
            &fu.ssa,
            &fu.taints,
            &fu.sccp.executable_blocks,
            &registry,
            Some(tcl_dialect::DialectProfile::irules()),
            &HashSet::new(),
        ));
    }
    out
}

#[test]
fn redirect_safe_mask_includes_path_prefixed() {
    assert!(TaintColour::REDIRECT_SAFE.contains(TaintColour::PATH_PREFIXED));
    assert!(TaintColour::REDIRECT_SAFE.contains(TaintColour::PATH_NORMALISED));
    // Must not spill into unrelated colours.
    assert!(!TaintColour::REDIRECT_SAFE.contains(TaintColour::TAINTED));
    assert!(!TaintColour::REDIRECT_SAFE.contains(TaintColour::CRLF_FREE));
}

/// The compiler and registry colour domains are a bijection.
/// Keep both directions pinned so a future colour cannot be silently
/// dropped by a permissive bitflags conversion.
#[test]
fn registry_taint_colour_bridge_preserves_every_bit_in_both_directions() {
    let registry_all = tcl_registry::TaintColour::all();
    let compiler_all = TaintColour::all();
    assert_eq!(reg_colour(registry_all), compiler_all);
    assert_eq!(compiler_colour(compiler_all), registry_all);
    for colour in [
        tcl_registry::TaintColour::TAINTED,
        tcl_registry::TaintColour::PATH_PREFIXED,
        tcl_registry::TaintColour::NON_DASH_PREFIXED,
        tcl_registry::TaintColour::CRLF_FREE,
        tcl_registry::TaintColour::SHELL_ATOM,
        tcl_registry::TaintColour::LIST_CANONICAL,
        tcl_registry::TaintColour::REGEX_LITERAL,
        tcl_registry::TaintColour::PATH_NORMALISED,
        tcl_registry::TaintColour::PATH_BOUNDED,
        tcl_registry::TaintColour::HEADER_TOKEN_SAFE,
        tcl_registry::TaintColour::HTML_ESCAPED,
        tcl_registry::TaintColour::URL_ENCODED,
        tcl_registry::TaintColour::IP_ADDRESS,
        tcl_registry::TaintColour::PORT,
        tcl_registry::TaintColour::FQDN,
        tcl_registry::TaintColour::PATH_JOINED,
        tcl_registry::TaintColour::CHANNEL,
    ] {
        let compiler = reg_colour(colour);
        assert_eq!(compiler_colour(compiler), colour, "round-trip {colour:?}");
    }
}

#[test]
fn file_join_transform_reaches_the_live_cfg_taint_lattice() {
    use crate::compilation_unit::CompilationUnit;

    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let colour_for = |source: &str, name: &str| {
        let cu = CompilationUnit::build_for(source, &registry, false);
        let fu = cu.function("::top").expect("top-level CFG built");
        fu.taints
            .iter()
            .find(|((symbol, _), _)| fu.ssa.var_name(*symbol) == name)
            .map(|(_, taint)| *taint)
            .expect("assignment has a live SSA taint value")
    };

    let joined = colour_for(
        "set raw [gets stdin]\nset joined [file join /base $raw]",
        "joined",
    );
    assert!(joined.is_tainted(), "the source taint must reach file join");
    assert!(
        joined.colours.contains(TaintColour::PATH_JOINED),
        "file join's registry transform must reach the CFG taint lattice: {joined:?}"
    );

    // Mutation control: replacing the transform-bearing `file join` with
    // a same-shape non-transform command removes the only PATH_JOINED
    // producer. If `word_taint_at` stopped applying registry transforms,
    // this and the live vector above would become indistinguishable.
    let without_transform = colour_for(
        "set raw [gets stdin]\nset joined [file tail $raw]",
        "joined",
    );
    assert!(without_transform.is_tainted());
    assert!(
        !without_transform.colours.contains(TaintColour::PATH_JOINED),
        "the mutation control must not inherit file join's transform: {without_transform:?}"
    );
}

#[test]
fn classify_irules_sink_http_respond() {
    let reg = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let hit = classify_sink(
        reg,
        "HTTP::respond",
        &[],
        Some(tcl_dialect::DialectProfile::irules()),
    );
    assert_eq!(hit.as_ref().map(|(c, _)| *c), Some(DiagCode::Irule3001));
}

#[test]
fn classify_irules_sink_http_header_insert() {
    let reg = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let hit = classify_sink(
        reg,
        "HTTP::header",
        &["insert".to_owned(), "X-Foo".to_owned(), "bar".to_owned()],
        Some(tcl_dialect::DialectProfile::irules()),
    );
    assert_eq!(hit.as_ref().map(|(c, _)| *c), Some(DiagCode::Irule3002));
    assert_eq!(hit.as_ref().unwrap().1, "HTTP::header insert");
}

#[test]
fn classify_irules_sink_http_cookie_replace() {
    let reg = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let hit = classify_sink(
        reg,
        "HTTP::cookie",
        &["replace".to_owned(), "sid".to_owned(), "val".to_owned()],
        Some(tcl_dialect::DialectProfile::irules()),
    );
    assert_eq!(hit.as_ref().map(|(c, _)| *c), Some(DiagCode::Irule3002));
    assert_eq!(hit.as_ref().unwrap().1, "HTTP::cookie replace");
}

#[test]
fn classify_irules_sink_prefix_abbreviation() {
    // `HTTP::cookie ins` is a legal abbreviation of `insert`
    // and must still classify as the IRULE3002 output sink.
    let reg = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let hit = classify_sink(
        reg,
        "HTTP::cookie",
        &["ins".to_owned(), "sid".to_owned(), "val".to_owned()],
        Some(tcl_dialect::DialectProfile::irules()),
    );
    assert_eq!(hit.as_ref().map(|(c, _)| *c), Some(DiagCode::Irule3002));
    // The label reports the canonical subcommand name.
    assert_eq!(hit.as_ref().unwrap().1, "HTTP::cookie insert");
}

#[test]
fn classify_irules_sink_http_header_remove_is_none() {
    let reg = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let hit = classify_sink(
        reg,
        "HTTP::header",
        &["remove".to_owned(), "X-Foo".to_owned()],
        Some(tcl_dialect::DialectProfile::irules()),
    );
    assert!(hit.is_none(), "remove subcommand must not emit IRULE3002");
}

#[test]
fn classify_irules_sink_log_and_redirect() {
    let reg = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    assert_eq!(
        classify_sink(
            reg,
            "log",
            &["local0.info".to_owned(), "x".to_owned()],
            Some(tcl_dialect::DialectProfile::irules())
        )
        .as_ref()
        .map(|(c, _)| *c),
        Some(DiagCode::Irule3003),
    );
    assert_eq!(
        classify_sink(
            reg,
            "HTTP::redirect",
            &["https://evil".to_owned()],
            Some(tcl_dialect::DialectProfile::irules())
        )
        .as_ref()
        .map(|(c, _)| *c),
        Some(DiagCode::Irule3004),
    );
}

#[test]
fn classify_sink_skips_irules_without_dialect() {
    let registry = CommandRegistry::build_default();
    let hit = classify_sink(&registry, "HTTP::respond", &["body".to_owned()], None);
    assert!(hit.is_none(), "no dialect → no IRULE3001, got {hit:?}");
}

#[test]
fn classify_sink_skips_irules_without_dialect_even_when_specs_loaded() {
    // Stronger than `classify_sink_skips_irules_without_dialect`: the
    // registry here *does* have the iRules specs loaded (as a shared
    // multi-dialect registry might), so `registry.get("HTTP::respond")`
    // succeeds. The IRULE3001 classification must still be withheld
    // because the active *document* dialect (`None`) isn't iRules —
    // mirrors `find_setter_constraint_warnings`'s explicit
    // `is_irules_dialect` gate for IRULE3101.
    let mut registry = CommandRegistry::build_default();
    registry.load_irules();
    assert!(registry.get("HTTP::respond").is_some());
    let hit = classify_sink(&registry, "HTTP::respond", &["body".to_owned()], None);
    assert!(
        hit.is_none(),
        "no dialect → no IRULE3001 even with iRules specs loaded, got {hit:?}"
    );
}

#[test]
fn classify_sink_exec_stays_t100_under_irules_dialect() {
    // `exec` is universal spec data (`surface: None`); under f5-irules
    // it is *banned* by the profile's §9 disable list, not by its own
    // surface. T100
    // classification must not route through profile availability —
    // it's a plain trait fact, independent of the active document
    // dialect — or `exec` would lose its T100 sink status in exactly
    // the dialect where flagging it matters most.
    let mut registry = CommandRegistry::build_default();
    registry.load_irules();
    let hit = classify_sink(
        &registry,
        "exec",
        &["$cmd".to_owned()],
        Some(tcl_dialect::DialectProfile::irules()),
    );
    assert_eq!(hit.as_ref().map(|(c, _)| *c), Some(DiagCode::T100));
}

#[test]
fn irule3001_fires_on_tainted_respond_body() {
    let w = irules_warnings_for("set u [HTTP::uri]\nHTTP::respond 200 content $u");
    assert!(
        w.iter().any(|x| x.code == DiagCode::Irule3001),
        "expected IRULE3001, got {w:?}"
    );
}

#[test]
fn irule3001_no_warning_for_literal_body() {
    let w = irules_warnings_for("HTTP::respond 200 content \"hello\"");
    assert!(
        w.iter().all(|x| x.code != DiagCode::Irule3001),
        "expected no IRULE3001 on literal body, got {w:?}"
    );
}

#[test]
fn irule3002_fires_on_tainted_header_value() {
    let w = irules_warnings_for("set v [HTTP::header X-Src]\nHTTP::header insert X-Echo $v");
    assert!(
        w.iter().any(|x| x.code == DiagCode::Irule3002),
        "expected IRULE3002, got {w:?}"
    );
}

#[test]
fn irule3002_fires_in_command_sub_form() {
    // Regression: `set _ [HTTP::header insert X-Foo $v]` — sink is
    // inside a command substitution, so the AssignValue branch must
    // preserve the subcommand args so classify_irules_sink sees
    // "insert" at arg-index 0.
    let w =
        irules_warnings_for("set v [HTTP::header X-Src]\nset _ [HTTP::header insert X-Echo $v]");
    assert!(
        w.iter().any(|x| x.code == DiagCode::Irule3002),
        "expected IRULE3002 inside command-sub sink, got {w:?}"
    );
}

#[test]
fn irule3002_skipped_on_remove_subcommand() {
    let w = irules_warnings_for("set v [HTTP::header X-Src]\nHTTP::header remove $v");
    assert!(
        w.iter().all(|x| x.code != DiagCode::Irule3002),
        "remove subcommand must not fire IRULE3002, got {w:?}"
    );
}

#[test]
fn irule3003_fires_on_tainted_log() {
    let w = irules_warnings_for("set u [HTTP::uri]\nlog local0.info $u");
    assert!(
        w.iter().any(|x| x.code == DiagCode::Irule3003),
        "expected IRULE3003, got {w:?}"
    );
}

#[test]
fn irule3004_fires_on_tainted_redirect() {
    let w = irules_warnings_for("set target [HTTP::header Location]\nHTTP::redirect $target");
    assert!(
        w.iter().any(|x| x.code == DiagCode::Irule3004),
        "expected IRULE3004, got {w:?}"
    );
}

#[test]
fn irule3004_redirect_safe_suppresses_via_lattice() {
    // Direct lattice check: tainted + REDIRECT_SAFE must suppress.
    // (Latent as an end-to-end test until iRules sources are tagged
    // with PATH_PREFIXED from their `taint_hints` — `HTTP::path`
    // carries PATH_PREFIXED on the getter form.)
    let lat = TaintLattice::tainted().with(TaintColour::PATH_PREFIXED);
    assert!(sink_colour_mitigated(DiagCode::Irule3004, lat));
    let lat = TaintLattice::tainted().with(TaintColour::PATH_NORMALISED);
    assert!(sink_colour_mitigated(DiagCode::Irule3004, lat));
    // Plain tainted should not suppress.
    assert!(!sink_colour_mitigated(
        DiagCode::Irule3004,
        TaintLattice::tainted()
    ));
}

#[test]
fn t102_uses_ensemble_subcommand_label_and_t103_for_regexp() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
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
        Some(tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()),
        &HashSet::new(),
    );
    let t102 = warnings.iter().find(|w| w.code == DiagCode::T102);
    assert!(
        t102.is_some_and(|w| w.sink_command == "file delete"),
        "expected a T102 with ensemble label 'file delete', got {warnings:?}",
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.code == DiagCode::T103 && w.sink_command == "regexp"),
        "expected T103 for tainted regexp pattern, got {warnings:?}",
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
        "regexp warnings must be ordered T103 then T102",
    );
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
fn irule_sinks_do_not_fire_without_dialect() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    // Without `with_interprocedural(Some(tcl_dialect::DialectProfile::irules()))`, no dialect
    // is active, and HTTP commands aren't sinks.
    let cu = CompilationUnit::build_for(
        "set u [HTTP::uri]\nHTTP::respond 200 content $u",
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
        None,
        &HashSet::new(),
    );
    assert!(
        warnings
            .iter()
            .all(|w| !w.code.as_str().starts_with("IRULE")),
        "no IRULE warnings without dialect, got {warnings:?}"
    );
}

#[test]
fn sink_colour_mitigated_html_escaped_only_mitigates_3001() {
    let tainted_html = TaintLattice::tainted().with(TaintColour::HTML_ESCAPED);
    // IRULE3001 (HTTP response body) — HTML_ESCAPED directly mitigates.
    assert!(sink_colour_mitigated(DiagCode::Irule3001, tainted_html));
    // IRULE3002/3003 (header / log) — HTML_ESCAPED does NOT prove
    // CRLF-injection safety (the escape rewrites `<`/`>`/`&` but
    // leaves raw CR/LF untouched). The CRLF-safe mask excludes
    // `HTML_ESCAPED`.
    assert!(!sink_colour_mitigated(DiagCode::Irule3002, tainted_html));
    assert!(!sink_colour_mitigated(DiagCode::Irule3003, tainted_html));
    // IRULE3004 (redirect) — also not mitigated by HTML_ESCAPED.
    assert!(!sink_colour_mitigated(DiagCode::Irule3004, tainted_html));

    // `CRLF_FREE` does suppress IRULE3002/3003 (the one mitigation
    // accepted in the value position).
    let tainted_crlf_free = TaintLattice::tainted().with(TaintColour::CRLF_FREE);
    assert!(sink_colour_mitigated(
        DiagCode::Irule3002,
        tainted_crlf_free
    ));
    assert!(sink_colour_mitigated(
        DiagCode::Irule3003,
        tainted_crlf_free
    ));
}

#[test]
fn original_scalar_reference_consumers_keep_literal_words_and_current_closers() {
    for label in ["tcl8.6", "tcl9.1"] {
        let registry = tcl_registry::model::ingress::static_context_for(label).commands();
        let config = TaintSourceContext::standalone(registry, registry.profile()).config;
        for (spelling, expected) in [
            ("${$literal}", Some("$literal")),
            ("${}", Some("")),
            ("{$literal}", None),
            ("{${literal}}", None),
            ("$arr(key)", None),
            ("${arr(key)}", None),
            ("prefix$x", None),
            (r"\$x", None),
        ] {
            let word = crate::value_shapes::value_word_with_config(spelling, config).unwrap();
            assert_eq!(
                original_scalar_reference_name(&word, config),
                expected,
                "{label}: {spelling}"
            );
        }
        let cfg = crate::cfg::Function::new("model", "entry");
        let mut ssa = SsaFunction::trivial("model", cfg.entry, cfg.block_names().to_vec());
        let symbol = ssa.intern_var("$literal");
        let statement = ssa_stmt(
            Statement::Incr {
                span: Span::new(0, 0),
                name: "$literal".into(),
                name_braced: true,
                amount: None,
                amount_braced: false,
                safe_on_uninit: false,
            },
            HashMap::from([(symbol, 1)]),
            HashMap::new(),
        );
        let taints = HashMap::from([((symbol, 1), TaintLattice::tainted())]);
        for (spelling, expected) in [("${$literal}", 1), ("{$literal}", 0)] {
            let word = crate::value_shapes::value_word_with_config(spelling, config).unwrap();
            let mut warnings = Vec::new();
            emit_dynamic_command_head_warning(
                Some((&word, config)),
                &statement,
                &taints,
                &mut warnings,
                &ssa,
            );
            assert_eq!(warnings.len(), expected, "{label}: {spelling}");
            if let Some(warning) = warnings.first() {
                assert_eq!(warning.variable, "$literal");
            }
        }
        let mut warnings = Vec::new();
        emit_dynamic_command_head_warning(None, &statement, &taints, &mut warnings, &ssa);
        assert_eq!(warnings, [] as [crate::taint::TaintWarning; 0]);
        let word = crate::value_shapes::value_word_with_config("${a{b}c}", config).unwrap();
        assert_eq!(
            original_scalar_reference_name(&word, config),
            if label == "tcl9.1" {
                Some("a{b}c")
            } else {
                None
            }
        );
    }
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let config = TaintSourceContext::standalone(registry, registry.profile()).config;
    let lat = TaintLattice::tainted().with(TaintColour::HEADER_TOKEN_SAFE);
    for (spelling, name, expected) in [
        ("${$literal}", "$literal", true),
        ("${}", "", true),
        ("{$literal}", "literal", false),
    ] {
        let args = vec![
            "insert".to_owned(),
            spelling.to_owned(),
            "$value".to_owned(),
        ];
        let word = crate::value_shapes::value_word_with_config(spelling, config).unwrap();
        assert_eq!(
            irule3002_name_position_safe(
                registry,
                "HTTP::header",
                &args,
                name,
                lat,
                Some(&word),
                config
            ),
            expected
        );
        assert!(!irule3002_name_position_safe(
            registry,
            "HTTP::header",
            &args,
            name,
            lat,
            None,
            config
        ));
    }
}

#[test]
fn irule3002_header_token_safe_name_position_suppresses() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let args = vec!["insert".to_owned(), "$name".to_owned(), "$value".to_owned()];
    let config = TaintSourceContext::standalone(registry, registry.profile()).config;
    let name_word = crate::value_shapes::value_word_with_config(&args[1], config).unwrap();
    let lat = TaintLattice::tainted().with(TaintColour::HEADER_TOKEN_SAFE);
    // Var `name` occupies arg-index 1 (name position) → suppressed.
    assert!(irule3002_name_position_safe(
        registry,
        "HTTP::header",
        &args,
        "name",
        lat,
        Some(&name_word),
        config,
    ));
    // Var `value` occupies arg-index 2 (value position) → not suppressed.
    assert!(!irule3002_name_position_safe(
        registry,
        "HTTP::header",
        &args,
        "value",
        lat,
        Some(&name_word),
        config,
    ));
    // Without HEADER_TOKEN_SAFE colour: never suppressed, even at name position.
    let plain = TaintLattice::tainted();
    assert!(!irule3002_name_position_safe(
        registry,
        "HTTP::header",
        &args,
        "name",
        plain,
        Some(&name_word),
        config,
    ));
    // Wrong subcommand: not suppressed.
    let rm_args = vec!["remove".to_owned(), "$name".to_owned()];
    assert!(!irule3002_name_position_safe(
        registry,
        "HTTP::header",
        &rm_args,
        "name",
        lat,
        Some(&name_word),
        config,
    ));
}

// IRULE3101 — setter-constraint violations

/// Default helper: run the setter check under the `f5-irules` dialect
/// (which is the only dialect that can surface IRULE3101 post-internal-gate).
fn setter_warnings_for(source: &str) -> Vec<TaintWarning> {
    setter_warnings_for_dialect(source, Some(tcl_dialect::DialectProfile::irules()))
}

fn setter_warnings_for_dialect(
    source: &str,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Vec<TaintWarning> {
    use crate::compilation_unit::CompilationUnit;
    let mut registry = CommandRegistry::build_default();
    // The IRULE3101 setter constraints live on the iRules specs, so the
    // registry must have them loaded for the dialect-gated check to see
    // anything.
    registry.load_irules();
    let mut cu = CompilationUnit::build_for(source, &registry, false);
    if dialect.is_some() {
        cu = cu.with_interprocedural(&registry, dialect);
    }
    let mut out: Vec<TaintWarning> = Vec::new();
    for fu in cu.analysable_functions() {
        out.extend(find_setter_constraint_warnings(
            &registry,
            &fu.cfg,
            &fu.ssa,
            &fu.taints,
            Some(&fu.sccp.values),
            &fu.sccp.executable_blocks,
            dialect,
        ));
    }
    out
}

#[test]
fn irule3101_literal_missing_slash_warns() {
    let w = setter_warnings_for("HTTP::uri foo");
    assert_eq!(w.len(), 1);
    assert_eq!(w[0].code, DiagCode::Irule3101);
    assert!(w[0].message.contains("HTTP::uri value must start"));
}

#[test]
fn irule3101_literal_with_slash_clean() {
    let w = setter_warnings_for("HTTP::uri /foo");
    assert_eq!(w.len(), 0, "literal /foo must be clean, got {w:?}");
}

#[test]
fn irule3101_http_path_literal_variants() {
    let bad = setter_warnings_for("HTTP::path bar");
    assert_eq!(bad.len(), 1);
    assert_eq!(bad[0].code, DiagCode::Irule3101);
    let good = setter_warnings_for("HTTP::path /bar");
    assert_eq!(good.len(), 0);
}

#[test]
fn irule3101_generic_taint_warns() {
    // `HTTP::header X-Foo` is a tainted source; reusing it as a path
    // has generic taint (no PATH_PREFIXED / _NORMALISED / _BOUNDED).
    let w = setter_warnings_for("set v [HTTP::header X-Foo]\nHTTP::uri $v");
    assert!(
        w.iter().any(|x| x.code == DiagCode::Irule3101),
        "generic taint must fire IRULE3101, got {w:?}"
    );
}

#[test]
fn irule3101_pure_var_ref_always_warns_without_safe_colour() {
    // A plain `$p` setter value (no taint + no provable path colour)
    // that the lattice cannot pin — two arms set two values — cannot be
    // proved `/`-prefixed by the static analyser, so IRULE3101 fires.
    // Latent suppression paths via tainted-with-PATH_PREFIXED /
    // _NORMALISED / _BOUNDED colours will light up once iRules source
    // `taint_hints` reach the lattice. A value the lattice proves is
    // checked as a literal is (#2055): `set p /safe` is clean.
    let w = setter_warnings_for(
        "if {[HTTP::has_responded]} { set p /safe } else { set p safe }\nHTTP::uri $p",
    );
    assert!(
        w.iter().any(|x| x.code == DiagCode::Irule3101),
        "pure var-ref setter value must warn without tainted-safe-colour, got {w:?}"
    );
    let w = setter_warnings_for("set p /safe\nHTTP::uri $p");
    assert!(
        !w.iter().any(|x| x.code == DiagCode::Irule3101),
        "a proven `/` path is clean, got {w:?}"
    );
}

#[test]
fn irule3101_dynamic_command_sub_warns() {
    // RHS is a command sub `[foo]` → hits the dynamic branch, which
    // always warns. The literal-check also bails on `[`.
    let w = setter_warnings_for("HTTP::uri [something]");
    assert!(
        w.iter().any(|x| x.code == DiagCode::Irule3101),
        "command-sub setter value must warn, got {w:?}"
    );
}

#[test]
fn irule3101_internally_gated_on_dialect() {
    // Defence-in-depth: even if a caller invokes
    // `find_setter_constraint_warnings` directly (bypassing
    // `run_all_checks`), IRULE3101 must not fire under a non-iRules
    // dialect against user-defined commands named `HTTP::uri` /
    // `HTTP::path`.
    let under_irules =
        setter_warnings_for_dialect("HTTP::uri foo", Some(tcl_dialect::DialectProfile::irules()));
    assert_eq!(under_irules.len(), 1);
    assert_eq!(under_irules[0].code, DiagCode::Irule3101);

    let under_none = setter_warnings_for_dialect("HTTP::uri foo", None);
    assert_eq!(
        under_none.len(),
        0,
        "no IRULE3101 under None dialect, got {under_none:?}"
    );

    let under_tcl = setter_warnings_for_dialect(
        "HTTP::uri foo",
        Some(tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile()),
    );
    assert_eq!(
        under_tcl.len(),
        0,
        "no IRULE3101 under tcl dialect, got {under_tcl:?}"
    );
}

// registry-driven source / sink / setter-constraint coverage

/// The Tcl-core source classification flows from the registry's
/// [`Traits::TAINT_SOURCE`] flag: registry-side query and
/// end-to-end taint-warning emission must agree on `gets` /
/// `read` / `exec` / `socket`.
#[test]
fn arch3_tcl_core_source_is_registry_driven() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    // Registry-side: TAINT_SOURCE trait is stamped on these.
    for cmd in ["gets", "read", "exec", "socket"] {
        let spec = registry.get(cmd).unwrap_or_else(|| panic!("{cmd} missing"));
        assert!(
            spec.traits.contains(tcl_registry::Traits::TAINT_SOURCE),
            "{cmd} must carry TAINT_SOURCE in the registry",
        );
    }
    // Same fact via the registry-side taint helper.
    assert!(tcl_registry::taint::is_taint_source(
        &registry,
        "gets",
        &["stdin"],
        None,
    ));

    // End-to-end: `gets` → `eval` raises T100. The fact reaches
    // the diagnostic via the registry, not via a compiler-side
    // command-name table.
    let cu = CompilationUnit::build_for("set x [gets stdin]\neval $x", &registry, false)
        .with_interprocedural(&registry, None);
    let mut warnings: Vec<TaintWarning> = Vec::new();
    for fu in cu.analysable_functions() {
        warnings.extend(find_taint_warnings(
            &fu.cfg,
            &fu.ssa,
            &fu.taints,
            &fu.sccp.executable_blocks,
            &registry,
            None,
            &HashSet::new(),
        ));
    }
    assert!(
        warnings.iter().any(|w| w.code == DiagCode::T100),
        "expected T100 from gets→eval with registry-driven source, got {warnings:?}",
    );
}

/// The iRules sink classification is registry-driven (via
/// `TAINT_SINK` and `EVALUATES_CODE` traits): tainted data into
/// `expr` (which carries `TAINT_SINK`) raises T100. The trait
/// is the single source of truth — flipping it removes the
/// classification.
#[test]
fn arch3_irules_sink_is_registry_driven() {
    let registry = CommandRegistry::build_default();
    let expr_spec = registry.get("expr").expect("expr in registry");
    assert!(
        expr_spec.traits.contains(tcl_registry::Traits::TAINT_SINK),
        "expr must carry TAINT_SINK in the registry",
    );

    // End-to-end: tainted iRules data (`HTTP::uri`) into expr
    // surfaces T100 under the iRules dialect.
    let w = irules_warnings_for("set u [HTTP::uri]\nexpr $u");
    assert!(
        w.iter().any(|d| d.code == DiagCode::T100),
        "expected T100 on tainted expr sink under iRules, got {w:?}",
    );
}

/// IRULE3101 setter-constraint violations are gated by the
/// iRules dialect filter — the diagnostic must not fire under
/// the plain Tcl dialect because the setter constraint comes
/// from the iRules-only registry entry.
#[test]
fn arch3_setter_constraint_is_dialect_driven() {
    // `HTTP::uri foo` (no leading slash) under iRules produces
    // an IRULE3101 setter-constraint violation.
    let irules =
        setter_warnings_for_dialect("HTTP::uri foo", Some(tcl_dialect::DialectProfile::irules()));
    assert!(
        irules.iter().any(|w| w.code == DiagCode::Irule3101),
        "expected IRULE3101 under iRules dialect, got {irules:?}",
    );

    // The same source under plain Tcl: a user-defined proc named
    // `HTTP::uri` is legal and the setter check must stay silent.
    let plain = setter_warnings_for_dialect("HTTP::uri foo", None);
    assert!(
        plain.iter().all(|w| w.code != DiagCode::Irule3101),
        "no IRULE3101 outside iRules, got {plain:?}",
    );
}

#[test]
fn taint_warnings_emitted_in_deterministic_proc_order() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    // `bbb` is defined *before* `aaa` in source. Per-proc warning order
    // must follow qualified name (aaa, then bbb) — not source order and
    // not the (random) HashMap order — so the output is reproducible.
    let source = "proc bbb {p} { set v [exec $p]\n eval $v }\n\
                      proc aaa {p} { set v [exec $p]\n eval $v }\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert_eq!(warnings.len(), 2, "one eval sink per proc: {warnings:?}");
    // `aaa` is defined later in source (larger span) yet sorts first.
    assert!(
        warnings[0].span.start() > warnings[1].span.start(),
        "expected name order (aaa before bbb) regardless of source order: {warnings:?}",
    );
}

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

/// TP: `interp alias {} myEval {} eval; myEval $x` still raises T100 —
/// `classify_sink` dispatches on `Statement::canonical_command_or_source()`,
/// so the alias resolves to `eval` even though the source spells the call
/// `myEval`, which is not itself a registered command.
#[test]
fn t100_fires_through_interp_alias_indirection() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let source = "interp alias {} myEval {} eval\n\
                       set x [gets stdin]\n\
                       myEval $x\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().any(|w| w.code == DiagCode::T100),
        "expected T100 through the `myEval` alias for `eval`, got {warnings:?}",
    );
}

/// TP: `rename eval myEval; myEval $x` also raises T100 — the same
/// `CommandAliasMap` mechanism `interp alias` uses now also recognises
/// a static `rename oldName newName` (`detect_rename`), so a renamed
/// built-in resolves through `canonical_command_or_source()` exactly
/// like an `interp alias`.
#[test]
fn t100_fires_through_rename_indirection() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let source = "rename eval myEval\n\
                       set x [gets stdin]\n\
                       myEval $x\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().any(|w| w.code == DiagCode::T100),
        "expected T100 through the `myEval` rename of `eval`, got {warnings:?}",
    );
}

/// TN: without the alias, a plain user command named `myEval` is not a
/// sink at all — proves the alias fix above isn't matching on the bare
/// name.
#[test]
fn t100_silent_for_unaliased_lookalike_command() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let source = "proc myEval {x} { return $x }\n\
                       set x [gets stdin]\n\
                       myEval $x\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().all(|w| w.code != DiagCode::T100),
        "a user proc named myEval must not be treated as the eval sink, got {warnings:?}",
    );
}

/// TP: `puts $tainted` still raises T101 (not T100) end-to-end, now via
/// the registry-driven `taint_output_sink` dispatch in `classify_sink`
/// rather than a hardcoded `command == "puts"` match.
#[test]
fn t101_fires_for_puts_via_registry_driven_dispatch() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let source = "set x [gets stdin]\nputs $x\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().any(|w| w.code == DiagCode::T101),
        "expected T101 for tainted puts, got {warnings:?}",
    );
    assert!(
        warnings.iter().all(|w| w.code != DiagCode::T100),
        "puts must not also raise T100, got {warnings:?}",
    );
}

/// TP: `open $tainted` raises T100. A `fileName` argument whose first
/// character is `|` runs the rest of the word as a command pipeline —
/// the same command-injection hazard `exec` carries — so tainted data
/// reaching `open`'s first argument is a code-execution sink exactly
/// like `exec`, not merely a file-I/O one.
#[test]
fn t100_fires_for_open_tainted_filename() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert!(
        registry
            .get("open")
            .expect("open in registry")
            .traits
            .contains(tcl_registry::Traits::TAINT_SINK),
        "open must carry TAINT_SINK in the registry",
    );
    let source = "set x [gets stdin]\nopen $x r\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().any(|w| w.code == DiagCode::T100),
        "expected T100 for tainted open fileName, got {warnings:?}",
    );
}

/// FP fix / TN: `open /tmp/out $mode` — a *literal* safe fileName with a
/// tainted access-mode argument must not raise T100. Only arg 0 selects a
/// `|`-pipeline; a tainted access mode or octal permissions can never
/// turn a literal path into a command, so `open`'s `taint_code_sink_args
/// = Some(&[0])` gates the sink to the fileName slot alone.
#[test]
fn t100_silent_for_open_tainted_access_mode() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let source = "set mode [gets stdin]\nopen /tmp/out $mode\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().all(|w| w.code != DiagCode::T100),
        "a tainted open access mode with a literal fileName must not raise T100, got {warnings:?}",
    );
}

/// TP: `apply $lambda ...` with a tainted lambda (arg 0) raises T100 — the
/// lambda body is executed verbatim, so tainted code in the `func` slot
/// is a genuine code-injection sink.
#[test]
fn t100_fires_for_apply_tainted_lambda() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let source = "set lambda [gets stdin]\napply $lambda 1 2\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().any(|w| w.code == DiagCode::T100),
        "expected T100 for a tainted apply lambda (arg 0), got {warnings:?}",
    );
}

/// FP fix / TN: `apply {{x} {return $x}} $tainted` — a tainted value
/// passed as an ordinary lambda *argument* (bound to a formal parameter,
/// never re-evaluated as script) must not raise T100. `apply`'s
/// `taint_code_sink_args = Some(&[0])` gates the sink to the `func` slot.
#[test]
fn t100_silent_for_apply_tainted_argument() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let source = "set x [gets stdin]\napply {{a} {return $a}} $x\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().all(|w| w.code != DiagCode::T100),
        "a tainted apply argument (not the lambda) must not raise T100, got {warnings:?}",
    );
}

/// FP fix / TN: `subst -nocommands $tainted` — the exact mitigation
/// `subst`'s own hover snippet recommends — must not raise T100, since
/// `-nocommands` disables the only hazard (command substitution) the
/// code names.
#[test]
fn t100_silent_for_subst_nocommands() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let source = "set x [gets stdin]\nsubst -nocommands $x\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().all(|w| w.code != DiagCode::T100),
        "subst -nocommands must not raise T100, got {warnings:?}",
    );
}

/// TP regression: plain `subst $tainted` (no flags) keeps raising T100 —
/// the new sink gate must not suppress the common, genuinely dangerous
/// default-options call.
#[test]
fn t100_fires_for_subst_without_nocommands() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let source = "set x [gets stdin]\nsubst $x\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().any(|w| w.code == DiagCode::T100),
        "expected T100 for bare `subst $x`, got {warnings:?}",
    );
}

/// TN: the Tcl 9.1 positive option form without `-commands` is equally
/// safe — `-variables` alone never runs command substitution.
#[test]
fn t100_silent_for_subst_positive_form_without_commands() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let source = "set x [gets stdin]\nsubst -variables $x\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().all(|w| w.code != DiagCode::T100),
        "subst -variables (Tcl 9.1 positive form) must not raise T100, got {warnings:?}",
    );
}

/// TP: the Tcl 9.1 positive form *with* `-commands` present stays a sink.
#[test]
fn t100_fires_for_subst_positive_form_with_commands() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let source = "set x [gets stdin]\nsubst -variables -commands $x\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().any(|w| w.code == DiagCode::T100),
        "expected T100 for `subst -variables -commands $x`, got {warnings:?}",
    );
}

/// FN fix — TP: a tainted variable used directly in an `if` branch
/// condition is evaluated exactly like any other braced `expr`, but
/// conditions live on `Terminator::Branch`, not in `ssa_block.statements`,
/// so the per-statement scan alone never reaches them —
/// `emit_branch_condition_warnings` is what covers them.
#[test]
fn t100_fires_for_tainted_if_condition() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let source = "proc f {} {\n  set x [gets stdin]\n  if {$x + 1 > 5} {\n    puts big\n  }\n}\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    let hit = warnings
        .iter()
        .find(|w| w.code == DiagCode::T100)
        .unwrap_or_else(|| panic!("expected T100 for tainted if-condition, got {warnings:?}"));
    // Tight span: just the `{...}` condition text, not the whole
    // `if {...} { puts big }` statement (which also spans the body).
    let text = &source[hit.span.start() as usize..hit.span.end() as usize];
    assert!(
        text == "$x",
        "expected the exact coercing variable reference, got {text:?}",
    );
}

/// TP: the same hazard through `while` and `for` conditions.
#[test]
fn t100_fires_for_tainted_while_and_for_conditions() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    for source in [
        "proc f {} {\n  set x [gets stdin]\n  while {$x < 10} {\n    incr x\n  }\n}\n",
        "proc f {} {\n  set x [gets stdin]\n  for {set i 0} {$i < $x} {incr i} {\n    puts $i\n  }\n}\n",
    ] {
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        assert!(
            warnings.iter().any(|w| w.code == DiagCode::T100),
            "expected T100 for tainted loop condition in {source:?}, got {warnings:?}",
        );
    }
}

/// FP fix / TN: `if {$tainted eq "admin"}` raises nothing — `eq` is a
/// pure string comparison with no numeric coercion, so neither the
/// "numeric coercion" message nor any other T100 variant applies. This
/// also proves the branch-condition pass reuses the same
/// `collect_coercion_operands` operator-context filter as the
/// `AssignExpr`/`ExprEval` path, not a re-implementation that flags
/// every variable indiscriminately.
#[test]
fn t100_silent_for_string_only_if_condition() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let source =
        "proc f {} {\n  set x [gets stdin]\n  if {$x eq \"admin\"} {\n    puts ok\n  }\n}\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().all(|w| w.code != DiagCode::T100),
        "eq is a pure string compare, expected no T100, got {warnings:?}",
    );
}

/// FP fix / TN: a `switch` on a tainted value raises no T100 in any of
/// its matching modes.
///
/// An exact `switch` is flattened into a chain of `StrEq` branch
/// terminators whose subject operand is an `ExprNode::Raw` holding the
/// normalised variable reference
/// (`cfg_builder::cfg_lower::switch_subject_operand`), so an arm's
/// coercion context is the one that `StrEq` gives its operands: none.
///
/// Every string mode is covered because none of them is an exception:
/// `-exact`, `-glob` and `-regexp` all match text, so there is no arm
/// spelling the message would fit. Tcl 9.1's `-integer` is the one numeric
/// mode; see `t100_for_a_tainted_switch_integer_subject`.
///
/// tclsh 8.6.18 and 9.0.4 both print `other` for `set c 0x10; switch -- $c
/// {16 {puts hex} default {puts other}}` — the subject is compared as the
/// string `0x10`, never coerced to 16.
#[test]
fn t100_silent_for_switch_arms_on_a_tainted_subject() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    for mode in ["--", "-exact --", "-glob --", "-regexp --"] {
        let source = format!(
            "proc f {{}} {{\n  set cmd [gets stdin]\n  switch {mode} $cmd {{\n    \
                 status {{ puts ok }}\n    version {{ puts 1 }}\n    \
                 default {{ puts none }}\n  }}\n}}\n"
        );
        let cu = CompilationUnit::build_for(&source, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        assert!(
            warnings.iter().all(|w| w.code != DiagCode::T100),
            "`switch {mode}` matches as a string, expected no T100, got {warnings:?}",
        );
    }
}

fn switch_integer_t100s(release: &str, body: &str) -> Vec<TaintWarning> {
    use crate::compilation_unit::CompilationUnit;
    let registry = tcl_registry::model::ingress::static_context_for(release).commands();
    let profile = tcl_dialect::DialectProfile::find(release).expect("catalogue profile");
    let source = format!("proc f {{}} {{\n  set cmd [gets stdin]\n  set ok 5\n  {body}\n}}\n");
    let cu = CompilationUnit::build_for_profile(&source, registry, false, profile)
        .with_interprocedural(registry, Some(profile));
    find_taint_warnings_for_cu(&cu, registry, Some(profile))
        .into_iter()
        .filter(|w| w.code == DiagCode::T100)
        .collect()
}

/// TP: Tcl 9.1's `switch -integer` (TIP 730) reads its subject with
/// `Tcl_GetWideIntFromObj`, so a tainted subject is the same coercion
/// hazard as a braced `expr` operand. tclsh 9.1.0 runs the `16` arm for
/// `switch -integer -- 0x10 {16 {puts hex} default {puts other}}` and
/// raises `expected integer but got "abc"` for a subject of `abc`.
#[test]
fn t100_for_a_tainted_switch_integer_subject() {
    for call in [
        "switch -integer -- $cmd {1 { puts one } default { puts none }}",
        "switch -int $cmd {1 { puts one }}",
        "switch -integer -- $cmd 1 { puts one } default { puts none }",
        "if {$ok} { switch -integer -- $cmd {1 { puts one }} }",
    ] {
        let hits = switch_integer_t100s("tcl9.1", call);
        assert_eq!(hits.len(), 1, "{call}: {hits:?}");
        assert_eq!(hits[0].variable, "cmd", "{call}");
        assert_eq!(hits[0].sink_command, "switch -integer operand", "{call}");
        assert!(
            hits[0].message.contains("numeric coercion"),
            "{call}: {}",
            hits[0].message
        );
    }
    // The squiggle is the coerced word, not the whole `switch`.
    let call = "switch -integer -- $cmd {1 { puts one }}";
    let hits = switch_integer_t100s("tcl9.1", call);
    let source = format!("proc f {{}} {{\n  set cmd [gets stdin]\n  set ok 5\n  {call}\n}}\n");
    let span = hits[0].span;
    assert_eq!(&source[span.start() as usize..span.end() as usize], "$cmd");
    // A tainted inline pattern is read as an integer too.
    let hits = switch_integer_t100s("tcl9.1", "switch -integer -- $ok $cmd { puts one }");
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0].variable, "cmd");
}

/// TN: no coercion reaches the tainted value — the option does not exist
/// before 9.1 (tclsh 9.0.4 rejects `-integer` as a bad option), the value
/// is brace-quoted text, a sanitiser hands over an integer it produced, the
/// subject is clean, the mode compares text (including when `-integer` is
/// only another option's value), or C rejects the option combination.
#[test]
fn t100_silent_for_switch_integer_without_a_coerced_tainted_word() {
    for (release, call) in [
        ("tcl9.0", "switch -integer -- $cmd {1 { puts one }}"),
        ("tcl8.6", "switch -integer -- $cmd {1 { puts one }}"),
        ("tcl9.1", "switch -integer -- {$cmd} {1 { puts one }}"),
        (
            "tcl9.1",
            "switch -integer -- [string length $cmd] {1 { puts one }}",
        ),
        ("tcl9.1", "switch -integer -- $ok {1 { puts one }}"),
        ("tcl9.1", "switch -glob -- $cmd {1* { puts one }}"),
        // `-integer` is `-matchvar`'s value here: a regexp switch.
        (
            "tcl9.1",
            "switch -matchvar -integer -regexp -- $cmd {1 { puts one }}",
        ),
        // C rejects the call before reading the subject.
        ("tcl9.1", "switch -integer -nocase -- $cmd {1 { puts one }}"),
        (
            "tcl9.1",
            "if {[scan $cmd %d n] == 1} { switch -integer -- $n {1 { puts one }} }",
        ),
    ] {
        let hits = switch_integer_t100s(release, call);
        assert_eq!(hits.len(), 0, "{release} `{call}`: {hits:?}");
    }
}

/// The same silence for an array element as a `switch` subject.
///
/// `$a(k)` normalises to a `${a(k)}` subject, whose reconstruction has to
/// match the array name the expr scan reports plus the index it does not.
/// It is one reference to one element, so the arm's `StrEq` context is the
/// whole answer here too.
#[test]
fn t100_silent_for_a_switch_on_a_tainted_array_element() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let source = "proc f {} {\n  array set a [list k [gets stdin]]\n  \
                      switch -- $a(k) {\n    status { puts ok }\n    \
                      default { puts none }\n  }\n}\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().all(|w| w.code != DiagCode::T100),
        "an array-element switch subject matches as a string too, got {warnings:?}",
    );
}

/// TP guard for the test above: the `Raw` arm's whole-variable narrowing
/// must not reach a `Raw` that really is unparsed expression source with a
/// coercing operator inside it.
#[test]
fn t100_still_fires_for_a_tainted_operand_in_an_unparsed_expr() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let source = "proc f {} {\n  set x [gets stdin]\n  expr {$x + 1}\n}\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().any(|w| w.code == DiagCode::T100),
        "expected T100 for a numeric-coercing operand, got {warnings:?}",
    );
}

/// An *escaped* bracket writes a literal `[`, so the quoter never runs and
/// the raw value lands in the pattern.
///
/// tclsh 8.6.18 and 9.0.4 both leave `"\[regex::quote $p]"` as the text
/// `[regex::quote <value of p>]` — the command is not invoked — so the
/// wrapper the mitigation would credit does not exist. Segmentation is the
/// lexer's for exactly this reason; a bracket count reads a call here.
#[test]
fn t103_not_cleared_by_an_escaped_bracket_that_never_calls_the_quoter() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let source = "proc f {} {\n  set p [gets stdin]\n  \
                      regexp -- \"\\[regex::quote $p]\" $line\n}\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().any(|w| w.code == DiagCode::T103),
        "an escaped bracket is not a call, so T103 must stand: {warnings:?}",
    );
}

/// T103's `[regex::quote …]` mitigation reads the same whether the quote
/// wraps the value in place or through a variable.
///
/// The quoting is what makes the pattern trusted, and it happens either
/// way; only the spelling differs. Since wrapping in place is what T103's
/// quick fix writes, that spelling is the one an applied fix has to clear.
///
/// The scope is the pattern argument alone: `$p` used as the *subject*
/// too is not in a pattern position, so it cannot withdraw the proof the
/// wrapped pattern carries.
#[test]
fn t103_cleared_by_an_inline_regex_quote_in_the_pattern_slot() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let cases: [(&str, bool); 5] = [
        ("regexp -- $p $line", true),
        ("set q [regex::quote $p]\n  regexp -- $q $line", false),
        ("regexp -- [regex::quote $p] $line", false),
        ("regexp -- [regex::quote $p] $p", false),
        // `string range` carries no transform, so it proves nothing.
        ("regexp -- [string range $p 0 5] $line", true),
    ];
    for (tail, expected) in cases {
        let source = format!("proc f {{}} {{\n  set p [gets stdin]\n  {tail}\n}}\n");
        let cu = CompilationUnit::build_for(&source, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        let got = warnings.iter().any(|w| w.code == DiagCode::T103);
        assert_eq!(got, expected, "for {tail:?}, got {warnings:?}");
    }
}

/// The T101 mitigation chain end to end: the CR/LF-stripping `string map`
/// its KCS page prescribes — and its quick fix writes — clears the
/// diagnostic, in both the wrap-a-variable and wrap-in-place spellings,
/// and no other mapping does.
///
/// tclsh 8.6.18 and 9.0.4 both give `string map {"\n" "" "\r" ""}
/// "a\nb\rc"` a length of 3 (`abc`), so the mapping really does delete
/// both characters — the element `"\n"` is list-parsed with its backslash
/// honoured, giving the newline itself as the key rather than a
/// two-character sequence. Dropping the `"\r"` pair leaves length 4 on
/// both, which is the half-pair case below.
#[test]
fn t101_cleared_only_by_a_mapping_that_deletes_crlf() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let crlf = r#"{"\n" "" "\r" ""}"#;
    let cases: &[(&str, bool)] = &[
        ("puts $x", true),
        (&format!("puts [string map {crlf} $x]"), false),
        (&format!("set s [string map {crlf} $x]\n  puts $s"), false),
        ("puts [string map {a b} $x]", true),
        // Only half the pair: an LF is deleted but a CR still survives.
        (r#"puts [string map {"\n" ""} $x]"#, true),
        // The mapping is dynamic and the *subject* is the braced word, so
        // the rewriting is exactly what cannot be seen. tclsh 8.6.18 and
        // 9.0.4: `set m [list X "a\nb"]; string map $m {"\n" "" "\r" ""
        // X}` yields a string containing a newline.
        (&format!("puts [string map $x {crlf}]"), true),
        // A later `string map` puts a newline back, so the earlier proof
        // no longer describes what reaches `puts` — in either spelling.
        (
            &format!("set s [string map {crlf} $x]\n  puts [string map {{\"|\" \"\\n\"}} $s]"),
            true,
        ),
        (
            &format!(
                "set s [string map {crlf} $x]\n  \
                     set t [string map {{\"|\" \"\\n\"}} $s]\n  puts $t"
            ),
            true,
        ),
    ];
    for &(tail, expected) in cases {
        let source = format!("proc f {{}} {{\n  set x [gets stdin]\n  {tail}\n}}\n");
        let cu = CompilationUnit::build_for(&source, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        let got = warnings.iter().any(|w| w.code == DiagCode::T101);
        assert_eq!(got, expected, "for {tail:?}, got {warnings:?}");
    }
}

/// FP fix — TN: a tainted variable that only appears inside a nested
/// `[cmd $x]` command substitution within a braced expr must not be
/// flagged as an "expr operand: numeric coercion" hit — it flows into
/// that nested call's own argument, a completely different risk
/// category (code execution, if `cmd` is itself dangerous) which that
/// call's own sink classification already covers independently.
#[test]
fn t100_expr_operand_message_not_raised_for_var_only_in_nested_command_sub() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    // `string length` is a sanitiser (fixed numeric return) and not a
    // sink, so the nested call itself raises nothing either — isolating
    // the "expr operand" false positive this test targets.
    let source = "proc f {} {\n  set x [gets stdin]\n  if {[string length $x] > 5} {\n    puts big\n  }\n}\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().all(|w| w.code != DiagCode::T100),
        "$x only reaches a nested command-sub argument, not a direct \
             expr operand; expected no T100, got {warnings:?}",
    );
}

/// TP: a tainted variable nested through a nested command sub still
/// raises T100 when that nested command is *itself* a sink.
#[test]
fn t100_fires_when_nested_command_sub_is_itself_a_sink() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let source =
        "proc f {} {\n  set x [gets stdin]\n  if {[exec $x] eq \"ok\"} {\n    puts ok\n  }\n}\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().any(|w| w.code == DiagCode::T100),
        "expected T100 from the nested `exec $x`, got {warnings:?}",
    );
}

/// TP: `$tainted` inside a ternary branch still flags when the ternary
/// itself sits in a numeric-coercion position — the branch's value
/// stands in for the ternary as a whole in the parent operator.
#[test]
fn t100_fires_for_tainted_var_through_ternary_branch_in_coercing_context() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let source = "proc f {c} {\n  set x [gets stdin]\n  if {($c ? $x : 0) + 1 > 5} {\n    puts big\n  }\n}\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().any(|w| w.code == DiagCode::T100),
        "expected T100 for $x flowing through a coerced ternary branch, got {warnings:?}",
    );
}

/// TP: the ternary *condition* is always boolean-coerced, regardless of
/// the ternary's own surrounding context.
#[test]
fn t100_fires_for_tainted_var_in_ternary_condition() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let source =
        "proc f {} {\n  set x [gets stdin]\n  if {$x ? \"a\" : \"b\"} {\n    puts ok\n  }\n}\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().any(|w| w.code == DiagCode::T100),
        "expected T100 for tainted ternary condition, got {warnings:?}",
    );
}

/// Precision: `eval "prefix" $x "suffix"` underlines only the `$x`
/// argument word (2 characters), not the whole `eval "prefix" $x
/// "suffix"` statement — `tight_arg_span` narrows the highlight to the
/// one argument word that actually carries the tainted value, using the
/// real per-word source spans a compiled `Statement::Call` carries.
#[test]
fn t100_span_is_the_tainted_argument_word_not_the_whole_statement() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let source = "proc f {} {\n  set x [gets stdin]\n  eval \"prefix\" $x \"suffix\"\n}\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    let hit = warnings
        .iter()
        .find(|w| w.code == DiagCode::T100)
        .unwrap_or_else(|| panic!("expected T100, got {warnings:?}"));
    let text = &source[hit.span.start() as usize..hit.span.end() as usize];
    assert_eq!(
        text, "$x",
        "expected the tight `$x` argument span, got {text:?} from {hit:?}",
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

/// The `${…}` closer comes from the shared owner, so the name taint is
/// keyed on is the one the lexer spanned.
///
/// Taint is name-keyed: a scan that recovers `a{b` where the document's
/// lexer spanned `a{b}c` looks up a cell nothing ever writes, so the
/// tainted value's colour is lost and the sink diagnostics that depend on
/// it (T1xx, W313) go quiet on a flow they should flag:
/// `set {a{b}c} 7; subst {${a{b}c}}` is `7` on tclsh 9.0.4 and
/// `can't read "a{b"` on 8.6.16.
#[test]
fn original_argument_candidates_use_full_source_config_and_nested_index_inventory() {
    let context = tcl_registry::model::ingress::static_context_for("tcl9.1");
    let registry = context.commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for(
        "puts [list exec $arr($index) ${a{b}c} $tail $tail]",
        registry,
        false,
    );
    let tokens = unit.ir_module.top_level.statements[0].tokens().unwrap();
    let source =
        TaintSourceContext::for_module_function(registry, &unit.ir_module, &unit.top_level);
    assert_eq!(source.config, unit.ir_module.lexer_config);
    let names = arg_var_names_with_config(&tokens.argv_texts[1], source.config);
    assert!(!names.has_unknown_residual());
    assert_eq!(names.names(), ["arr", "index", "a{b}c", "tail", "tail"]);
}

#[test]
fn standalone_list_wrapper_uses_original_grouped_head_values_and_exact_extent() {
    let registry = CommandRegistry::build_default();
    let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
    let dialect = registry
        .profile()
        .map(tcl_registry::InvocationDialect::of_profile);
    for original in [
        "[list puts $value]",
        "[list {puts} $value]",
        "[::list \"puts\" $value]",
    ] {
        assert_eq!(
            standalone_list_wrapper_head(original, &registry, config, dialect).as_deref(),
            Some("puts")
        );
    }
    for original in [
        "[list $command $value]",
        "[list {$command} $value]",
        "[list {} $value]",
        "[list {*}{puts value}]",
        "[list puts $value; list puts $value]",
        "[list puts $value]suffix",
        "[list puts $value]; missing",
        "[list puts $value",
        "[string cat puts $value]",
    ] {
        assert!(
            standalone_list_wrapper_head(original, &registry, config, dialect).is_none(),
            "{original}"
        );
    }
    let args = vec!["[list {puts} $value]".to_owned()];
    let call = SinkCall {
        source: TaintSourceContext::standalone(&registry, registry.profile()),
        command: "eval",
        args: &args,
        registry: &registry,
        tokens: None,
        effective: None,
        source_read: None,
        argument_offset: 0,
    };
    assert!(list_wrapped_arg_command_is_literal(&call, "value"));
    let mut unavailable = call;
    unavailable.source = TaintSourceContext::for_input(&registry, None, config);
    assert!(!list_wrapped_arg_command_is_literal(&unavailable, "value"));
}

#[test]
fn unavailable_argument_inventory_keeps_known_reads_and_refuses_mitigations() {
    let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = context.commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for(
        "set value SAFE; puts {[missing $value}",
        registry,
        false,
    );
    let source =
        TaintSourceContext::for_module_function(registry, &unit.ir_module, &unit.top_level);
    let tokens = unit
        .ir_module
        .top_level
        .statements
        .last()
        .unwrap()
        .tokens()
        .unwrap();
    let args = tokens.argv_texts[1..].to_vec();
    let call = SinkCall {
        source,
        command: "puts",
        args: &args,
        registry,
        tokens: Some(tokens),
        effective: None,
        source_read: None,
        argument_offset: 0,
    };
    assert!(arg_var_names_with_config(&args[0], source.config).has_unknown_residual());
    assert!(
        all_argument_candidates(&args[0], source.config, &unit.top_level.ssa)
            .iter()
            .any(|name| name == "value")
    );
    assert!(!var_consumed_by_sanitiser(&call, "value"));
    assert!(!list_wrapped_arg_command_is_literal(&call, "value"));
    assert!(!sink_var_position_safe(&call, DiagCode::T100, "value"));
    assert!(var_wrapper_colours(registry, source, &args, "value", Some(tokens), 0).is_none());
}

#[test]
fn arg_var_names_follows_the_release_close_rule() {
    use tcl_dialect::BracedVarStyle::{FirstClose, Tcl9Nesting};
    assert_eq!(
        arg_var_names("/tmp/${a{b}c}", Tcl9Nesting),
        HashSet::from(["a{b}c".to_owned()])
    );
    assert_eq!(
        arg_var_names("/tmp/${a{b}c}", FirstClose),
        HashSet::from(["a{b".to_owned()])
    );
    // The scan resumes past the reference under either rule, so a second
    // ordinary reference in the same word is still collected.
    assert_eq!(
        arg_var_names("${a{b}c}/$tail", Tcl9Nesting),
        HashSet::from(["a{b}c".to_owned(), "tail".to_owned()])
    );
}

/// Shared substitution inventories preserve name and lexical boundaries.
mod arg_var_names_vs_vars_in_word {
    use super::*;

    /// TP: a bare `$name` reference — both scanners agree.
    #[test]
    fn agree_on_bare_var() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            arg_var_names("$p", tcl_dialect::BracedVarStyle::default()),
            HashSet::from(["p".to_owned()])
        );
        assert_eq!(
            crate::var_refs::vars_in_word("$p", &registry)
                .into_iter()
                .collect::<HashSet<_>>(),
            HashSet::from(["p".to_owned()])
        );
    }

    /// TP: `${braced}` next to a bare `$var` in the same word — both
    /// scanners agree, including across a non-variable separator (`/`).
    #[test]
    fn agree_on_braced_and_bare_in_one_word() {
        let registry = CommandRegistry::build_default();
        let want = HashSet::from(["dir".to_owned(), "file".to_owned()]);
        assert_eq!(
            arg_var_names("${dir}/$file", tcl_dialect::BracedVarStyle::default()),
            want
        );
        assert_eq!(
            crate::var_refs::vars_in_word("${dir}/$file", &registry)
                .into_iter()
                .collect::<HashSet<_>>(),
            want
        );
    }

    /// TP (command substitution): a `$var` nested inside `[cmd …]` —
    /// both scanners recurse into the substitution and agree.
    #[test]
    fn agree_on_var_inside_command_substitution() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            arg_var_names("[foo $x]", tcl_dialect::BracedVarStyle::default()),
            HashSet::from(["x".to_owned()])
        );
        assert_eq!(
            crate::var_refs::vars_in_word("[foo $x]", &registry)
                .into_iter()
                .collect::<HashSet<_>>(),
            HashSet::from(["x".to_owned()])
        );
    }

    /// The array root and its substituted index are separate dependencies.
    #[test]
    fn agree_on_var_nested_in_array_index() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            arg_var_names("$arr($cmd)", tcl_dialect::BracedVarStyle::default()),
            HashSet::from(["arr".to_owned(), "cmd".to_owned()]),
            "arg_var_names must keep catching the nested index variable"
        );
        assert_eq!(
            crate::var_refs::vars_in_word("$arr($cmd)", &registry)
                .into_iter()
                .collect::<HashSet<_>>(),
            HashSet::from(["arr".to_owned(), "cmd".to_owned()]),
            "the shared inventory includes the substituted index dependency"
        );
    }

    /// FP guard: a backslash-escaped `\$name` is literal text (Tcl
    /// backslash substitution turns `\$` into a literal `$`, so no
    /// variable is referenced). A byte-scan that does not know about
    /// backslash escaping reads it as a reference to `notavar`;
    /// `arg_var_names` must agree with `vars_in_word`, which goes through
    /// the real lexer.
    #[test]
    fn escaped_dollar_is_not_a_reference() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            arg_var_names("\\$notavar", tcl_dialect::BracedVarStyle::default()).len(),
            0,
            "an escaped \\$ must not be read as a variable reference"
        );
        assert_eq!(
            crate::var_refs::vars_in_word("\\$notavar", &registry).len(),
            0
        );
    }

    /// FP fixed, doubled escape: `\\$foo` is an escaped backslash
    /// (literal `\`) followed by a live, unescaped `$foo` — the
    /// variable reference still must be found.
    #[test]
    fn double_backslash_then_live_var_is_still_found() {
        assert_eq!(
            arg_var_names("\\\\$foo", tcl_dialect::BracedVarStyle::default()),
            HashSet::from(["foo".to_owned()])
        );
    }

    /// TN: plain literal text with no `$` sigil at all.
    #[test]
    fn no_dollar_sigil_is_empty() {
        assert_eq!(
            arg_var_names("hello world", tcl_dialect::BracedVarStyle::default()).len(),
            0
        );
    }
}

/// `interp alias` / `rename` command-name resolution feeding sink
/// classification. Sink checks key off
/// `Statement::canonical_command_or_source()`, so a call through a
/// renamed or aliased name resolves to whatever it now denotes.
mod rename_and_alias_taint {
    use super::*;
    use crate::compilation_unit::CompilationUnit;

    fn taint_warnings_for(source: &str) -> Vec<TaintWarning> {
        let registry = CommandRegistry::build_default().project_for_profile(
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        );
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        find_taint_warnings_for_cu(&cu, &registry, None)
    }

    /// TP (regression guard): tainted data reaching the real `puts` sink
    /// through an `interp alias` name. This was silently missed before
    /// sink classification started reading `canonical_command` — the
    /// alias table was populated but never consulted here.
    #[test]
    fn tainted_data_through_interp_alias_is_flagged() {
        let w = taint_warnings_for("set u [gets stdin]\ninterp alias {} myputs {} puts\nmyputs $u");
        assert!(
            w.iter().any(|d| d.code == DiagCode::T101),
            "expected T101 through the `myputs` alias, got {w:?}"
        );
    }

    /// TP: tainted data reaching the real `puts` sink through a renamed
    /// command name (`rename puts myputs`) — the scenario this
    /// investigation was specifically scoped to cover.
    #[test]
    fn tainted_data_through_renamed_sink_is_flagged() {
        let w = taint_warnings_for("set u [gets stdin]\nrename puts myputs\nmyputs $u");
        assert!(
            w.iter().any(|d| d.code == DiagCode::T101),
            "expected T101 through the renamed `myputs`, got {w:?}"
        );
    }

    /// TN: `rename someSafeProc puts` shadows the builtin — the bare
    /// name `puts` now denotes the user's own proc, not the real
    /// output sink, so a subsequent `puts $tainted` must NOT be flagged.
    /// Guards the false-positive direction of the same fix (a renamed
    /// binding can point *away* from a builtin just as easily as onto
    /// one).
    #[test]
    fn rename_shadowing_a_builtin_suppresses_the_sink() {
        let w = taint_warnings_for(
            "proc mysafeputs {x} { return }\n\
                 set u [gets stdin]\n\
                 rename mysafeputs puts\n\
                 puts $u",
        );
        assert!(
            w.iter().all(|d| d.code != DiagCode::T101),
            "puts is shadowed by a user proc after the rename, got {w:?}"
        );
    }

    /// A substituted word whose value is known names the missing command;
    /// its failed rename stops this script before the later sink.
    #[test]
    fn known_missing_rename_stops_before_the_builtin_sink() {
        let w =
            taint_warnings_for("set old somecmd\nset u [gets stdin]\nrename $old puts\nputs $u");
        assert!(
            w.iter().all(|d| d.code != DiagCode::T101),
            "a failed rename must not invent a reached puts sink, got {w:?}"
        );
    }

    /// TN control: an *unrelated* rename must not disturb ordinary
    /// `puts` sink detection.
    #[test]
    fn unrelated_rename_does_not_affect_puts_sink() {
        let w = taint_warnings_for("rename string mystr\nset u [gets stdin]\nputs $u");
        assert!(
            w.iter().any(|d| d.code == DiagCode::T101),
            "expected T101 for a plain puts call, got {w:?}"
        );
    }

    /// TN: the deletion form `rename puts {}` creates no new binding —
    /// nothing to alias — so it must not be mistaken for a rename that
    /// resolves some other name to `puts`.
    #[test]
    fn rename_deletion_form_creates_no_alias() {
        let w = taint_warnings_for("set u [gets stdin]\nrename puts {}\nputs $u");
        assert!(
            w.iter().all(|d| d.code != DiagCode::T101),
            "a deleted command cannot regain stock puts metadata: {w:?}"
        );
    }
}

/// Unit tests for the pure [`shadowed_builtin_names`] helper, isolated
/// from the end-to-end taint-warning plumbing tested by
/// `namespace_shadowed_builtin_taint` below.
mod shadowed_builtin_names_tests {
    use super::*;

    #[test]
    fn retained_body_scope_ignores_display_and_refuses_detached_units() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let mut unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            "namespace eval ns {proc puts {args} {}; proc run {value} {puts $value}}",
            &registry,
            false,
            profile,
        );
        let id =
            crate::interprocedural::test_body_identity::fixture_id(&unit.procedures, "::ns::run");
        let function = unit.procedures.get_mut(&id).unwrap();
        function.name = "::counterfactual::run".to_owned();
        assert_eq!(
            shadowed_builtin_names_for_module_function(
                unit.procedures.get(&id).unwrap(),
                &unit,
                &registry
            ),
            Some(HashSet::from(["puts".to_owned()]))
        );
        let detached_top = unit.top_level.clone();
        assert!(
            shadowed_builtin_names_for_module_function(&detached_top, &unit, &registry).is_none()
        );
        let detached_body = unit.procedures.get(&id).unwrap().clone();
        assert!(
            shadowed_builtin_names_for_module_function(&detached_body, &unit, &registry).is_none()
        );
        let foreign_unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            "proc foreign {} {}",
            &registry,
            false,
            profile,
        );
        let foreign = crate::interprocedural::test_body_identity::fixture_id(
            &foreign_unit.procedures,
            "::foreign",
        );
        unit.procedures.get_mut(&id).unwrap().body_id = Some(foreign);
        assert!(
            shadowed_builtin_names_for_module_function(
                unit.procedures.get(&id).unwrap(),
                &unit,
                &registry
            )
            .is_none()
        );
    }

    #[test]
    fn namespace_scoped_proc_shadows_only_its_own_namespace() {
        let registry = CommandRegistry::build_default();
        let procs = ["::myns::puts", "::myns::helper"];
        let shadowed = shadowed_builtin_names("::myns", procs.iter().copied(), &registry);
        assert_eq!(shadowed, HashSet::from(["puts".to_owned()]));
        // A different namespace sees no shadow at all.
        assert_eq!(
            shadowed_builtin_names("::other", procs.iter().copied(), &registry).len(),
            0
        );
    }

    #[test]
    fn global_proc_shadows_every_namespace() {
        let registry = CommandRegistry::build_default();
        let procs = ["::puts"];
        for ns in ["::", "::myns", "::a::b"] {
            assert_eq!(
                shadowed_builtin_names(ns, procs.iter().copied(), &registry),
                HashSet::from(["puts".to_owned()]),
                "namespace {ns} must see the global shadow"
            );
        }
    }

    #[test]
    fn nested_child_proc_does_not_shadow_its_parent() {
        let registry = CommandRegistry::build_default();
        let procs = ["::a::b::puts"];
        assert_eq!(
            shadowed_builtin_names("::a", procs.iter().copied(), &registry).len(),
            0
        );
    }

    #[test]
    fn non_registry_proc_name_is_not_shadowing() {
        // A proc named after something the registry doesn't know is not a
        // "shadow" in this sense — there's no builtin sink to suppress.
        let registry = CommandRegistry::build_default();
        let procs = ["::myns::totallyMadeUpName"];
        assert_eq!(
            shadowed_builtin_names("::myns", procs.iter().copied(), &registry).len(),
            0
        );
    }
}

/// A user proc shadowing a builtin's bare name: `proc ::myns::puts {args}
/// {…}` (or a plain top-level `proc puts {…}`) makes a bare `puts` call
/// from the shadowing scope dispatch to the user's own proc, not the
/// output-sink builtin.
mod namespace_shadowed_builtin_taint {
    use super::*;
    use crate::compilation_unit::CompilationUnit;

    fn taint_warnings_for(source: &str) -> Vec<TaintWarning> {
        // These fixtures mean C Tcl 8.6: its own profile, not the lenient
        // all-release `tcl` profile, beside its 8.6 command context.
        let environment = tcl_registry::model::ingress::resolve_environment("tcl8.6");
        let context = environment.default_context_registry();
        let profile = environment.analyser_profile();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            environment.unit_profile(),
            std::sync::Arc::clone(&context),
            config,
        );
        let registry = context.commands();
        let cu = CompilationUnit::build_with_analysis_input(
            source,
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
        find_taint_warnings_for_cu(&cu, registry, Some(profile))
    }

    /// FP fixed: a namespace-scoped proc of the same name as a builtin
    /// must suppress the builtin sink classification for bare calls made
    /// from that namespace — this was the false positive the
    /// investigation's example (`proc ::myns::puts`) called out.
    #[test]
    fn namespace_scoped_proc_shadowing_builtin_suppresses_sink() {
        let w = taint_warnings_for(
            "namespace eval ::myns {\n\
                 \x20 proc puts {x} { return }\n\
                 \x20 proc run {} {\n\
                 \x20   set u [gets stdin]\n\
                 \x20   puts $u\n\
                 \x20 }\n\
                 }",
        );
        assert!(
            w.iter().all(|d| d.code != DiagCode::T101),
            "puts is shadowed by ::myns::puts, must not be a sink, got {w:?}"
        );
    }

    /// FP fixed: the same shadowing effect for a plain top-level proc
    /// redefinition (no namespace involved at all).
    #[test]
    fn top_level_proc_shadowing_builtin_suppresses_sink() {
        let w = taint_warnings_for("proc puts {x} { return }\nset u [gets stdin]\nputs $u");
        assert!(
            w.iter().all(|d| d.code != DiagCode::T101),
            "puts is shadowed by a top-level proc, got {w:?}"
        );
    }

    /// FP fixed: a *global*-level shadowing proc suppresses the sink for
    /// bare calls made from *any* namespace, not just the global one —
    /// Tcl's unqualified-name lookup always falls back to global.
    #[test]
    fn global_proc_shadows_calls_from_other_namespaces_too() {
        let w = taint_warnings_for(
            "proc puts {x} { return }\n\
                 namespace eval ::myns {\n\
                 \x20 proc run {} {\n\
                 \x20   set u [gets stdin]\n\
                 \x20   puts $u\n\
                 \x20 }\n\
                 }",
        );
        assert!(
            w.iter().all(|d| d.code != DiagCode::T101),
            "the global puts proc must shadow calls from ::myns too, got {w:?}"
        );
    }

    /// TN control: an unshadowed bare `puts` inside a namespace is still
    /// the real builtin sink.
    #[test]
    fn unshadowed_namespace_call_is_still_a_sink() {
        let w = taint_warnings_for(
            "namespace eval ::myns {\n\
                 \x20 proc run {} {\n\
                 \x20   set u [gets stdin]\n\
                 \x20   puts $u\n\
                 \x20 }\n\
                 }",
        );
        assert!(
            w.iter().any(|d| d.code == DiagCode::T101),
            "expected T101 for an unshadowed puts call, got {w:?}"
        );
    }

    /// TN control: a shadowing proc in an unrelated namespace must not
    /// suppress a sink in a *different* namespace.
    #[test]
    fn shadow_in_unrelated_namespace_does_not_suppress_sink() {
        let w = taint_warnings_for(
            "namespace eval ::other {\n\
                 \x20 proc puts {x} { return }\n\
                 }\n\
                 namespace eval ::myns {\n\
                 \x20 proc run {} {\n\
                 \x20   set u [gets stdin]\n\
                 \x20   puts $u\n\
                 \x20 }\n\
                 }",
        );
        assert!(
            w.iter().any(|d| d.code == DiagCode::T101),
            "::other's shadow must not affect ::myns, got {w:?}"
        );
    }

    /// TN control: an explicit fully-qualified `::puts` call bypasses
    /// namespace resolution entirely, so it must still be flagged even
    /// when the bare name is locally shadowed.
    #[test]
    fn fully_qualified_call_bypasses_the_shadow() {
        let w = taint_warnings_for(
            "namespace eval ::myns {\n\
                 \x20 proc puts {x} { return }\n\
                 \x20 proc run {} {\n\
                 \x20   set u [gets stdin]\n\
                 \x20   ::puts $u\n\
                 \x20 }\n\
                 }",
        );
        assert!(
            w.iter()
                .any(|d| matches!(d.code, DiagCode::T100 | DiagCode::T101)),
            "an explicit ::puts must still be classified as a sink, got {w:?}"
        );
    }
}

/// `trace add variable` / `trace variable` conservatively forcing a
/// traced name to fully tainted (`apply_module_variable_traces`).
mod variable_trace_taint {
    use super::*;
    use crate::compilation_unit::CompilationUnit;

    fn taint_warnings_for(source: &str) -> Vec<TaintWarning> {
        let registry = CommandRegistry::build_default().project_for_profile(
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        );
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        find_taint_warnings_for_cu(&cu, &registry, None)
    }

    /// FN guard: installing a `write` trace on an already-tainted
    /// variable must not clear its taint. The trace-add call's own
    /// literal argument words — `add`, `variable`, `x`, `write`,
    /// `{handler}` — are all clean, so a generic "propagate from
    /// arguments" fallback would treat the trace-add's synthetic write to
    /// `x` as deriving from them.
    #[test]
    fn write_trace_does_not_clear_prior_taint() {
        let w = taint_warnings_for(
            "set x [gets stdin]\ntrace add variable x write {sanitize}\nputs $x",
        );
        assert!(
            w.iter().any(|d| d.code == DiagCode::T101),
            "trace installation must not launder x's existing taint, got {w:?}"
        );
    }

    /// FN direction: a variable set to a literal (clean) value is
    /// conservatively treated as tainted once *any* trace (even a
    /// `read` trace) is attached — the handler could inject
    /// attacker-controlled content on every future access.
    #[test]
    fn traced_literal_value_is_conservatively_tainted() {
        // The handler must exist: an undefined one makes the read raise
        // in C Tcl, so `puts` never runs.
        let w = taint_warnings_for(
            "proc inject {name args} { upvar 1 $name v; set v [gets stdin] }\n\
                 set x \"safe-literal\"\ntrace add variable x read {inject}\nputs $x",
        );
        assert!(
            w.iter().any(|d| d.code == DiagCode::T101),
            "a traced variable must be treated as tainted even from a literal set, got {w:?}"
        );
    }

    /// Cross-proc: a trace installed by one proc still taints a read of
    /// the same global variable in a different function (the module-wide
    /// `Module::traced_variables` over-approximation, not a per-function
    /// one).
    #[test]
    fn trace_installed_in_another_proc_still_taints_this_read() {
        let w = taint_warnings_for(
            "proc install_trace {} { trace add variable ::x write {cb} }\n\
                 set x [gets stdin]\n\
                 install_trace\n\
                 puts $x",
        );
        assert!(
            w.iter().any(|d| d.code == DiagCode::T101),
            "expected T101 despite the trace living in a different proc, got {w:?}"
        );
    }

    /// A dynamic trace target (`trace add variable $name …`) can't be
    /// resolved to a literal name, so — mirroring
    /// `Module::has_dynamic_variable_trace` — every name in the module is
    /// conservatively treated as traced (tainted).
    #[test]
    fn dynamic_trace_target_taints_every_name() {
        let w = taint_warnings_for(
            "set x \"safe-literal\"\ntrace add variable $somevar write {cb}\nputs $x",
        );
        assert!(
            w.iter().any(|d| d.code == DiagCode::T101),
            "a dynamic trace target must conservatively taint every name, got {w:?}"
        );
    }

    /// TN control: an untraced variable behaves exactly as before — a
    /// clean literal stays clean.
    #[test]
    fn untraced_literal_value_stays_clean() {
        let w = taint_warnings_for("set x hello\nputs $x");
        assert_eq!(w.len(), 0, "no trace in scope, expected no warnings: {w:?}");
    }

    /// TN control: an untraced tainted variable is unaffected by this
    /// fix — still flagged exactly as the pre-existing behaviour.
    #[test]
    fn untraced_tainted_value_still_flagged() {
        let w = taint_warnings_for("set x [gets stdin]\nputs $x");
        assert!(
            w.iter().any(|d| d.code == DiagCode::T101),
            "expected T101 for a plain untraced tainted puts call, got {w:?}"
        );
    }
}

/// Unit tests for the pure [`apply_module_variable_traces`] helper,
/// isolated from the end-to-end plumbing tested by
/// `variable_trace_taint` above.
mod apply_module_variable_traces_tests {
    use super::*;
    use crate::compilation_unit::ModuleTraceFacts;

    #[test]
    fn forces_existing_entries_for_a_traced_name() {
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "proc p {} { trace add variable t write cb\nset t 1\nputs $t }",
            &CommandRegistry::build_default()
                .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap()),
            false,
        );
        assert!(cu.ir_module.traced_variables.contains("t"));
        let traces = ModuleTraceFacts {
            traced_variables: &cu.ir_module.traced_variables,
            has_dynamic_variable_trace: cu.ir_module.has_dynamic_variable_trace,
            deferred_writes: &cu.ir_module.deferred_writes,
        };

        let fu = cu.function("::p").unwrap();
        let patched = apply_module_variable_traces((*fu.taints).clone(), &fu.ssa, traces);
        let first_definitions: Vec<_> = patched
            .iter()
            .filter(|((symbol, version), _)| *version == 1 && fu.ssa.var_name(*symbol) == "t")
            .collect();
        assert!(
            !first_definitions.is_empty(),
            "the traced variable must have a represented first definition: {patched:?}"
        );
        assert!(
            first_definitions
                .iter()
                .all(|(_, taint)| taint.is_tainted()),
            "every canonical t#1 definition must be forced tainted: {patched:?}"
        );
    }

    #[test]
    fn leaves_untraced_names_unchanged() {
        let registry = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "proc p {} { set t 1\nputs $t }",
            &registry,
            false,
        );
        assert!(!cu.ir_module.traced_variables.contains("t"));
        assert!(!cu.ir_module.has_dynamic_variable_trace);
        let traces = ModuleTraceFacts {
            traced_variables: &cu.ir_module.traced_variables,
            has_dynamic_variable_trace: cu.ir_module.has_dynamic_variable_trace,
            deferred_writes: &cu.ir_module.deferred_writes,
        };
        let fu = cu.function("::p").unwrap();
        let patched = apply_module_variable_traces((*fu.taints).clone(), &fu.ssa, traces);
        assert_eq!(
            patched, *fu.taints,
            "no trace in scope — map must be unchanged"
        );
    }
}
/// An iRules event handler is a body with no command slot. A unit that
/// holds one still answers which builtins its procedures shadow, for the
/// file level, the procedure and the handler alike.
#[test]
fn an_event_handler_body_shadows_no_builtin() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for(
        "proc log {args} {}\nwhen HTTP_REQUEST {set x 1}\n",
        registry,
        false,
    );
    let mut handler = false;
    for function in unit.all_body_function_units() {
        let shadowed = shadowed_builtin_names_for_module_function(function, &unit, registry)
            .unwrap_or_else(|| panic!("{} has no answer", function.name));
        assert!(shadowed.contains("log"), "{}: {shadowed:?}", function.name);
        handler |= function.name.contains("HTTP_REQUEST");
    }
    assert!(handler, "the handler body is one of the unit's functions");
}

#[test]
fn hosted_taint_diagnostics_use_current_source_metadata_without_native_handler_claims() {
    // Implementation contract: naming.consumer.hosted-source-taint-descriptor
    // docs/design/analysis/name-resolution-proofs/hosted-source-taint-descriptor.md
    let context =
        tcl_registry::model::ingress::resolve_environment("f5-irules").default_context_registry();
    let registry = context.commands();
    let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
    let source = "when HTTP_REQUEST {set x [HTTP::query]; HTTP::respond 200 content $x}";
    let options = crate::compilation_unit::UnitBuildOptions {
        registry,
        defer_top_level: false,
        config,
        dialect: Some(tcl_dialect::DialectProfile::irules()),
        external_call_sites: None,
        declared_commands: None,
    };
    let cu = crate::compilation_unit::CompilationUnit::build_with_context_registry(
        source,
        options,
        None,
        std::sync::Arc::clone(&context),
    )
    .with_interprocedural(registry, Some(tcl_dialect::DialectProfile::irules()));
    let diagnostics = crate::compiler_checks::run_all_checks(
        &cu,
        registry,
        Some(tcl_dialect::DialectProfile::irules()),
    );
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagCode::Irule3001)
        .expect("genuine conditional hosted sink");
    assert_eq!(
        diagnostic.taint_subject.as_ref().unwrap().sink_command(),
        "HTTP::respond"
    );
    assert_eq!(
        diagnostic.source_context.as_ref().unwrap().image().bytes(),
        source.as_bytes()
    );
    for negative in [
        "when HTTP_REQUEST {HTTP::respond 200 content literal}",
        "when HTTP_REQUEST {HTTP::respond 200 content {$x}}",
        "when HTTP_REQUEST {HTTP::respond 200 content \"$x$tail\"}",
        "proc HTTP::respond {args} {}; when HTTP_REQUEST {HTTP::respond 200 content $x}",
    ] {
        let unit = crate::compilation_unit::CompilationUnit::build_with_context_registry(
            negative,
            options,
            None,
            std::sync::Arc::clone(&context),
        );
        let warnings = find_taint_warnings_for_cu(
            &unit,
            registry,
            Some(tcl_dialect::DialectProfile::irules()),
        );
        assert!(
            warnings
                .iter()
                .all(|warning| warning.code != DiagCode::Irule3001),
            "{negative}: {warnings:?}"
        );
    }
    // An unavailable TMM transition does not prove that a later event's
    // command is deleted. Its conditional hosted descriptor remains advice.
    let uncertain = "rename HTTP::respond {}; when HTTP_REQUEST {HTTP::respond 200 content $x}";
    let unit = crate::compilation_unit::CompilationUnit::build_with_context_registry(
        uncertain,
        options,
        None,
        std::sync::Arc::clone(&context),
    );
    let warnings =
        find_taint_warnings_for_cu(&unit, registry, Some(tcl_dialect::DialectProfile::irules()));
    assert!(
        warnings
            .iter()
            .any(|warning| warning.code == DiagCode::Irule3001)
    );
}

/// A name an opaque `switch`'s arm may overwrite keeps the taint it held
/// when no arm runs: the arm need not run, so `puts $u` still sees data
/// read from the channel.
#[test]
fn a_tainted_name_an_opaque_switch_arm_may_overwrite_stays_tainted() {
    let registry = CommandRegistry::build_default();
    let source = "proc p {s} {\n set u [gets stdin]\n switch -glob -- $s { q* { set u clean } }\n puts $u\n}\n";
    let cu = crate::compilation_unit::CompilationUnit::build_for(source, &registry, false)
        .with_interprocedural(&registry, None);
    let found = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        found.iter().any(|w| w.code == DiagCode::T101),
        "expected T101 for the sink after the switch, got {found:?}"
    );
}
#[test]
fn required_replay_empty_success_differs_from_withdrawn_source_owner() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = context.commands();
    let (_owner, unit) = retained_warning_unit("set value 1", &context);
    let current = taint_warning_report_for_cu(&unit, registry, None);
    assert_eq!(current.warnings, [] as [crate::taint::TaintWarning; 0]);
    assert!(current.replays.is_complete());
    let mut withdrawn = unit.clone();
    withdrawn.ir_module.retained_source_bindings = None;
    let missing = taint_warning_report_for_cu(&withdrawn, registry, None);
    assert_eq!(missing.warnings, [] as [crate::taint::TaintWarning; 0]);
    assert!(!missing.replays.is_complete());
    assert!(matches!(
        missing.replays,
        TaintReplayAnalysis::Unavailable { .. }
    ));
}

#[test]
fn unavailable_nested_replay_retains_original_site_and_independent_warnings() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = context.commands();
    let source = "set payload [gets stdin]\nregexp $payload haystack\n[eval $payload]";
    let (_owner, unit) = retained_warning_unit(source, &context);
    let proven = taint_warning_report_for_cu(&unit, registry, None).warnings;
    assert!(proven.iter().any(|warning| warning.code == DiagCode::T103));
    let statement = unit
        .top_level
        .ssa
        .blocks
        .values()
        .flat_map(|block| &block.statements)
        .find(|statement| {
            statement
                .statement
                .tokens()
                .and_then(|tokens| tokens.word_exprs.first())
                .is_some_and(|word| {
                    let mut children = Vec::new();
                    command_substitutions_in_word(word, &mut children);
                    !children.is_empty()
                })
        })
        .expect("genuine original command-head substitution");
    let word = statement
        .statement
        .tokens()
        .unwrap()
        .word_exprs
        .first()
        .unwrap();
    let mut children = Vec::new();
    command_substitutions_in_word(word, &mut children);
    let (spelling, span) = children.first().unwrap();
    let script = spelling
        .strip_prefix('[')
        .unwrap()
        .strip_suffix(']')
        .unwrap();
    let unavailable = std::cell::RefCell::new(Vec::new());
    let shadowed = HashSet::new();
    let sink = SinkWarningContext {
        registry,
        source: TaintSourceContext::for_input(registry, None, unit.ir_module.lexer_config),
        dialect: unit.ir_module.dialect_profile,
        shadowed_builtins: &shadowed,
        original_module: Some(&unit.ir_module),
        unavailable_replays: Some(&unavailable),
    };
    let mut warnings = proven.clone();
    emit_isolated_script_warnings(
        script,
        *span,
        statement,
        &unit.top_level.taints,
        &mut warnings,
        &unit.top_level.ssa,
        &sink,
        None,
    );
    assert_eq!(warnings, proven);
    let unavailable = unavailable.into_inner();
    assert_eq!(unavailable.len(), 1);
    assert_eq!(unavailable[0].span, *span);
    assert_eq!(
        unavailable[0].reason,
        UnavailableTaintReplayReason::SourceMetadata
    );
    let captured = unavailable[0].source.as_ref().unwrap();
    assert!(captured.matches_module(&unit.ir_module, registry));
    let mut changed = unit.ir_module.clone();
    changed.source = tcl_lexer::SourceImage::document("set payload fixed");
    assert!(!captured.matches_module(&changed, registry));
    let mut changed_entry = unit.ir_module.clone();
    changed_entry.source_entry.unknown_entry = !changed_entry.source_entry.unknown_entry;
    assert!(!captured.matches_module(&changed_entry, registry));
    let mut changed_namespace = unit.ir_module.clone();
    changed_namespace.top_level_namespace = "::different_replay_source".to_owned();
    assert!(!captured.matches_module(&changed_namespace, registry));
    let mut foreign = tcl_registry::CommandRegistry::build_default();
    foreign.insert(tcl_registry::CommandSpec {
        name: "__foreign_replay_store",
        ..tcl_registry::CommandSpec::DEFAULT
    });
    assert!(!captured.matches_module(&unit.ir_module, &foreign));
}

#[test]
fn scaffold_constructors_require_actual_selected_descriptor_identity() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = context.commands();
    let actual = ScaffoldConstructors::required(registry, None, true, true, true).unwrap();
    assert_eq!(actual.rebinding_priority(registry, None, "proc"), 2);
    assert_eq!(actual.rebinding_priority(registry, None, "::proc"), 2);
    assert_eq!(actual.rebinding_priority(registry, None, "interp"), 1);
    assert_eq!(actual.rebinding_priority(registry, None, "ordinary"), 0);
    let mut replacement = tcl_registry::CommandRegistry::build_default();
    replacement.insert(registry.get("proc").unwrap().clone());
    assert_eq!(actual.rebinding_priority(&replacement, None, "proc"), 0);
    assert!(matches!(
        ScaffoldConstructors::required(registry, None, false, true, false),
        Err(UnavailableTaintReplayReason::SourceMetadata)
    ));
    assert!(ScaffoldConstructors::required(registry, None, false, false, false).is_ok());
    let mut unavailable = tcl_registry::CommandRegistry::build_default();
    unavailable.insert(tcl_registry::CommandSpec {
        state_transitions: Some(tcl_registry::StateTransitionDescriptor::EMPTY),
        ..registry.get("proc").unwrap().clone()
    });
    assert!(matches!(
        ScaffoldConstructors::required(&unavailable, None, true, true, false),
        Err(UnavailableTaintReplayReason::ProcedureConstructor)
    ));
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
#[test]
fn public_report_keeps_constructor_refusal_beside_an_independent_warning() {
    let baseline =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let mut supplied = tcl_registry::CommandRegistry::build_default();
    supplied.insert(tcl_registry::CommandSpec {
        state_transitions: Some(tcl_registry::StateTransitionDescriptor::EMPTY),
        ..baseline.commands().get("proc").unwrap().clone()
    });
    let context = std::sync::Arc::new(baseline.with_command_store(std::sync::Arc::new(supplied)));
    let registry = context.commands();
    let source = "set payload [gets stdin]\nregexp $payload haystack\nrename puts replacement\n[puts $payload]";
    let (_owner, unit) = retained_warning_unit(source, &context);
    let report = taint_warning_report_for_cu(&unit, registry, None);
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.code == DiagCode::T103)
    );
    assert!(!report.replays.is_complete());
    let refusal = report
        .replays
        .unavailable_replays()
        .iter()
        .find(|residual| residual.reason == UnavailableTaintReplayReason::ProcedureConstructor)
        .expect("actual required scaffold constructor is unavailable");
    assert!(
        refusal
            .source
            .as_ref()
            .unwrap()
            .matches_module(&unit.ir_module, registry)
    );
    assert_eq!(
        unit.ir_module
            .source
            .bytes()
            .get(refusal.span.as_range())
            .unwrap(),
        b"[puts $payload]"
    );
}
