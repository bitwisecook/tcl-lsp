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
use tcl_names::place::{Index, LOCAL_NS, PlaceKind, array_elem, overlap, scalar};

fn registry() -> CommandRegistry {
    CommandRegistry::build_default()
}

#[test]
fn replacement_ends_store_exposure_before_later_opaque_reads() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let source = "proc f {} {set x OLD; set x NEW; unknown_reader}";
    let cu =
        CompilationUnit::build_for_profile(source, registry, false, registry.profile().unwrap());
    let function = cu.function("::f").unwrap();
    let points = function.ssa.point_contexts.as_ref().unwrap();
    let stores: Vec<_> = function
        .cfg
        .blocks
        .iter()
        .flat_map(|(&block, body)| {
            body.statements
                .iter()
                .enumerate()
                .filter_map(move |(index, statement)| {
                    def_places(statement, points.before_statement(block, index), registry)
                        .iter()
                        .any(|place| place.name == "x")
                        .then_some((block, index, statement))
                })
        })
        .collect();
    assert_eq!(stores.len(), 2);
    for (ordinal, &(block, index, statement)) in stores.iter().enumerate() {
        let written = def_places(statement, points.before_statement(block, index), registry);
        assert_eq!(written.len(), 1);
        assert_eq!(
            write_observed_by_unknown_access(
                &function.cfg,
                block,
                index,
                &written[0],
                points,
                registry,
            ),
            ordinal == 1,
            "store {ordinal}"
        );
    }
}

#[test]
fn element_observers_follow_original_renamed_and_aliased_array_reads() {
    // Native dispatch controls: naming.variable.original-array-observer-renamed-alias-controls
    // docs/design/analysis/name-resolution-proofs/original-array-observer-renamed-alias-controls.md
    // naming.variable.original-point-element-observers
    // docs/design/analysis/name-resolution-proofs/variable-original-point-element-observers.md
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_dialect::DialectProfile::find(dialect).unwrap();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let (_owner, captured) =
            crate::environment_ingress::captured_native_entry_with_owner(profile);
        let entry = crate::command_binding::SourceAnalysisEntry {
            native_entry: Some(std::sync::Arc::new(captured)),
            invocation_dialect: Some(tcl_native_model::InvocationDialect::of_profile(profile)),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..Default::default()
        };
        for (setup, head) in [
            ("", "array"),
            ("rename array array_old; ", "array_old"),
            ("interp alias {} array_alias {} array; ", "array_alias"),
        ] {
            let source = format!("set a(k) OLD; {setup}set out [{head} get a]");
            let unit = CompilationUnit::build_with_source_entry(
                &source,
                crate::compilation_unit::UnitBuildOptions {
                    registry: &registry,
                    defer_top_level: false,
                    config: tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                    dialect: Some(profile),
                    external_call_sites: None,
                    declared_commands: None,
                },
                &entry,
            );
            let function = &unit.top_level;
            let points = function.ssa.point_contexts.as_ref().unwrap();
            let mut selected_reads = 0;
            let mut stores = Vec::new();
            for (&block, body) in &function.cfg.blocks {
                for (index, statement) in body.statements.iter().enumerate() {
                    let writes = def_places_at(statement, block, index, points, &registry);
                    stores.extend(writes.into_iter().filter_map(|place| {
                        (place.kind == PlaceKind::ArrayElem && place.name == "a").then_some((
                            body.name.clone(),
                            index,
                            place,
                        ))
                    }));
                    let Some(tokens) = points.source_tokens_at(block, index) else {
                        continue;
                    };
                    for child in crate::word_subst::lifted_calls(
                        Some(tokens),
                        unit.ir_module.native_lexer_config(),
                    ) {
                        if child.command != head {
                            continue;
                        }
                        let child_tokens = child.tokens.as_ref().unwrap();
                        assert!(
                            child_tokens.source_binding.as_ref().is_some_and(|binding| {
                                binding.proved_target().is_some_and(|target| {
                                    target.registry_backed
                                        && target.command.trim_start_matches("::") == "array"
                                })
                            }),
                            "{dialect}: actual selected child owner: {source}: {child:?}",
                        );
                        let reads = read_places_at(statement, block, index, points, &registry);
                        assert!(
                            reads.iter().any(|place| {
                                place.kind == PlaceKind::ArrayWhole && place.name == "a"
                            }),
                            "{dialect}: actual array read inventory: {source}: {reads:?}",
                        );
                        selected_reads += 1;
                    }
                }
            }
            assert_eq!(selected_reads, 1, "{dialect}: {source}");
            assert_eq!(stores.len(), 1, "{dialect}: actual element store: {source}");
            let observed = element_writes_observed_with_contexts(&function.cfg, points, &registry);
            let (block, index, _) = &stores[0];
            assert!(
                observed.contains(&(block.clone(), i32::try_from(*index).unwrap())),
                "{dialect}: selected {head} read observes the original element: {observed:?}",
            );
        }
    }
}

fn normal_call(source: &str, registry: &CommandRegistry) -> Statement {
    let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
    let segment = segment_commands_with_offset_and_config(source, 0, config)
        .pop()
        .unwrap();
    let tokens =
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment);
    Statement::Call {
        span: segment.span,
        command: segment.name().to_owned(),
        canonical_command: None,
        args: segment.args().to_vec(),
        defs: vec!["result".to_owned()],
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: Some(tokens),
        foreach_groups: None,
    }
}

#[test]
fn body_output_selects_its_retargeted_normal_binding() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let mut before = ResolveContext::for_function("::p");
    before.invocation_dialect = registry
        .profile()
        .map(tcl_native_model::InvocationDialect::of_profile);
    let mut after = before.clone();
    after.known_namespaces.insert("::static".to_owned());
    after.define_literal("::static::captured", "BOOM", registry);
    let target = crate::var_resolve::resolve_literal_access(
        "::static::captured",
        &after,
        false,
        registry,
        tcl_registry::TraceOperation::Write,
    );
    after
        .alias_bindings
        .insert("result".to_owned(), target.clone());
    let catch = normal_call("catch {error BOOM} result", registry);
    assert_eq!(
        def_places_with_continuation(&catch, &before, &after, registry),
        vec![target.clone()]
    );
    assert!(
        statement_mutation_places_with_continuation(&catch, &before, &after, registry)
            .contains(&target)
    );
    let set = normal_call("set result VALUE", registry);
    let defs = def_places_with_continuation(&set, &before, &after, registry);
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].ns, LOCAL_NS);
    assert_eq!(defs[0].name, "result");
    assert_ne!(defs[0].cell, target.cell);
}

/// Collect every def-place and read-place across a proc's CFG.
fn places_of(src: &str, qname: &str) -> (Vec<Place>, Vec<Place>) {
    let r = registry();
    let cu = CompilationUnit::build_for(src, &r, false);
    let fu = cu.function(qname).expect("proc not found");
    let points = build_point_resolve_contexts(&fu.cfg, &fu.name, &r);
    let mut defs = Vec::new();
    let mut reads = Vec::new();
    for (&id, block) in &fu.cfg.blocks {
        for (index, stmt) in block.statements.iter().enumerate() {
            let ctx = points.before_statement(id, index);
            defs.extend(def_places(stmt, ctx, &r));
            reads.extend(read_places(stmt, ctx, &r));
        }
    }
    for place in defs.iter_mut().chain(&mut reads) {
        place.cell = None;
    }
    (defs, reads)
}

#[test]
fn array_element_def_and_read_are_element_granular() {
    // Element-granular defs/reads: `set a(k) 1` defs a(k); `puts $a(k)`
    // reads a(k); `set a(j) 2` defs a(j), which NO read observes.  Dead-store
    // analysis suppresses the W220 on a(k) (a read overlaps it) but keeps a(j)'s.
    let (defs, reads) = places_of("proc f {} { set a(k) 1; set a(j) 2; puts $a(k) }", "::f");
    let ak = array_elem("a", Index::literal("k"), LOCAL_NS, false);
    let aj = array_elem("a", Index::literal("j"), LOCAL_NS, false);
    assert!(defs.contains(&ak), "expected a(k) def, got {defs:?}");
    assert!(defs.contains(&aj), "expected a(j) def, got {defs:?}");
    // a read observes a(k) …
    assert!(
        reads.iter().any(|r| overlap(r, &ak)),
        "a(k) should be read by `puts $a(k)`, reads={reads:?}",
    );
    // … but nothing observes a(j).
    assert!(
        !reads.iter().any(|r| overlap(r, &aj)),
        "a(j) must not be observed by any read, reads={reads:?}",
    );
}

#[test]
fn scalar_def_and_read() {
    let (defs, reads) = places_of("proc f {} { set x 1; puts $x }", "::f");
    let x = scalar("x", LOCAL_NS, false);
    assert!(defs.contains(&x));
    assert!(reads.iter().any(|r| overlap(r, &x)));
}

#[test]
fn var_read_role_arg_resolves_whole_array() {
    // `array get arr` reads the *whole* array arr → overlaps every element.
    let (_defs, reads) = places_of("proc f {} { set out [array get arr] }", "::f");
    let arr_k = array_elem("arr", Index::literal("k"), LOCAL_NS, false);
    assert!(
        reads.iter().any(|r| overlap(r, &arr_k)),
        "array get arr should read the whole array, reads={reads:?}",
    );
}

#[test]
fn build_context_collects_declarations() {
    let r = registry();
    let cu = CompilationUnit::build_for(
        "proc f {} { global g; variable v; upvar 1 caller y; set x 1 }",
        &r,
        false,
    );
    let fu = cu.function("::f").unwrap();
    let points = build_point_resolve_contexts(&fu.cfg, &fu.name, &r);
    let ctx = points.before_terminator(fu.cfg.entry);
    assert!(ctx.alias_bindings.contains_key("g"));
    assert!(ctx.alias_bindings.contains_key("v"));
    assert!(ctx.alias_bindings.contains_key("y"));
    // `y` resolves to an upvar alias, not a plain local scalar.
    let p = resolve_place("y", ctx, false, &r);
    assert_eq!(p.kind, PlaceKind::UpvarAlias);
}

#[test]
fn build_context_tolerates_qualified_upvar_head() {
    // A fully-qualified `::upvar` head must still register the alias —
    // `collect_upvar_aliases` strips the leading `::` so the bare-form
    // grammar matches (otherwise `upvar_aliases` would be incomplete).
    let r = registry();
    let cu = CompilationUnit::build_for("proc f {} { ::upvar 1 caller y; set x 1 }", &r, false);
    let fu = cu.function("::f").unwrap();
    let points = build_point_resolve_contexts(&fu.cfg, &fu.name, &r);
    let ctx = points.before_terminator(fu.cfg.entry);
    assert!(ctx.alias_bindings.contains_key("y"));
    assert_eq!(
        resolve_place("y", ctx, false, &r).kind,
        PlaceKind::UpvarAlias
    );
}

#[test]
fn dynamic_index_read_records_the_index_var() {
    // `set a($i) 1` defs a($i) (dynamic) and reads `i` (to form the index).
    let (defs, reads) = places_of("proc f {} { set i 0; set a($i) 1 }", "::f");
    assert!(
        defs.iter()
            .any(|d| d.kind == PlaceKind::ArrayElem && d.name == "a"),
        "defs={defs:?}",
    );
    assert!(
        reads
            .iter()
            .any(|r| overlap(r, &scalar("i", LOCAL_NS, false))),
        "the dynamic index read of `i` must be recovered, reads={reads:?}",
    );
}

#[test]
fn standalone_command_reads_keep_original_literal_name_data_and_braced_inert_words() {
    let registry = registry();
    let context = ResolveContext::for_function("::f");
    let mut reads = Vec::new();
    command_reads(
        "info exists {$literal}; info exists {${literal}}; info exists {}; puts {$inert}",
        &context,
        &mut reads,
        &registry,
    );
    for name in ["$literal", "${literal}", ""] {
        assert!(
            reads.iter().any(|place| place.name == name),
            "missing {name:?}"
        );
    }
    assert!(!reads.iter().any(|place| place.name == "inert"));
    assert!(!reads.iter().any(|place| place.name == "literal"));
    assert!(!reads.iter().any(|place| place.kind == PlaceKind::Unknown));
}

#[test]
fn standalone_command_reads_keep_dynamic_names_expansion_and_missing_selection_unknown() {
    let registry = registry();
    let context = ResolveContext::for_function("::f");
    for script in [
        "info exists $computed",
        "$handler {$literal}",
        "info {*}{exists literal}",
        "missing_reader",
    ] {
        let mut reads = Vec::new();
        command_reads(script, &context, &mut reads, &registry);
        assert!(
            reads.iter().any(|place| place.kind == PlaceKind::Unknown),
            "{script}"
        );
    }
    let mut reads = Vec::new();
    command_reads("info exists $computed", &context, &mut reads, &registry);
    assert!(reads.iter().any(|place| place.name == "computed"));
}

#[test]
fn standalone_evaluated_data_keeps_reads_unknown_without_a_component_owner() {
    let registry = registry();
    let context = ResolveContext::for_function("::f");
    for script in ["expr {$value}", "subst {$value}", "eval {puts $value}"] {
        let mut reads = Vec::new();
        command_reads(script, &context, &mut reads, &registry);
        assert!(
            reads.iter().any(|place| place.kind == PlaceKind::Unknown),
            "{script}"
        );
    }
    let mut inert = Vec::new();
    command_reads(
        "expr {{${value}} eq {$literal}}",
        &context,
        &mut inert,
        &registry,
    );
    assert!(inert.iter().any(|place| place.kind == PlaceKind::Unknown));
    assert!(
        !inert
            .iter()
            .any(|place| matches!(place.name.as_str(), "value" | "literal"))
    );
    let mut literal = Vec::new();
    command_reads("puts {$value}", &context, &mut literal, &registry);
    assert_eq!(literal, [] as [tcl_names::place::Place; 0]);
}
