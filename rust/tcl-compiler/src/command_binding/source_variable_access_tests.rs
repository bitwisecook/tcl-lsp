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

fn analyse(source: &str) -> (SourceCommandBindings, Arc<CommandRegistry>) {
    let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = Arc::clone(context.commands());
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        tcl_lexer::LexerConfig::for_profile(registry.profile()),
        &registry,
        SourceAnalysisOptions {
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            invocation_dialect: Some(tcl_native_model::InvocationDialect::for_version(
                tcl_dialect::TclVersion::V8_6,
            )),
            ..SourceAnalysisOptions::default()
        },
    );
    (bindings, registry)
}

fn reads(bindings: &SourceCommandBindings, source: &str) -> Vec<SourceVariableAccess> {
    bindings.variable_accesses_in_span(tcl_lexer::Span::new(
        0,
        u32::try_from(source.len().saturating_sub(1)).unwrap(),
    ))
}

#[test]
fn full_depth_array_indices_resolve_every_source_read_after_its_index() {
    let source = include_str!("../../tests/data/native_deep_array_source.tcl");
    let (bindings, registry) = analyse(source);
    let accesses = reads(&bindings, source);
    let elements = accesses
        .iter()
        .filter(|access| access.original_spelling.starts_with("$a("))
        .collect::<Vec<_>>();
    assert_eq!(elements.len(), 2000);
    for access in elements {
        for context in access.context_alternatives() {
            let place = access.place_in_context(context, &registry);
            let index = place.index.as_ref().expect("resolved native element");
            assert_eq!(index.kind, crate::place::IndexKind::Literal);
            assert_eq!(index.value, "x");
        }
    }
}

#[test]
fn evaluated_indices_freeze_values_before_later_parts_and_read_observers() {
    for (source, spelling, key, observed) in [
        (
            "set k old; set a(oldnew) VALUE; set view $a($k[set k new])",
            "$a($k[set k new])",
            "oldnew",
            false,
        ),
        (
            "proc observe {name key op} {upvar 1 $name value; set value old}; set k new; set a(old) VALUE; trace add variable k read observe; set view $a($k)",
            "$a($k)",
            "old",
            false,
        ),
        (
            "proc observe args {}; set k k; set a(k) VALUE; trace add variable a(k) read observe; set view $a($k)",
            "$a($k)",
            "k",
            true,
        ),
        (
            "set k k; set {a($k)} VALUE; set view ${a($k)}",
            "${a($k)}",
            "$k",
            false,
        ),
    ] {
        let (bindings, registry) = analyse(source);
        let all = reads(&bindings, source);
        let access = all
            .iter()
            .find(|access| access.original_spelling == spelling)
            .unwrap();
        for context in access.context_alternatives() {
            let place = access.place_in_context(context, &registry);
            let index = place.index.as_ref().expect("actual element address");
            assert_eq!(index.kind, crate::place::IndexKind::Literal, "{source}");
            assert_eq!(index.value, key, "{source}");
            assert_eq!(place.observed, observed, "{source}");
            if observed {
                assert!(!context.read_produces_value(&place, &registry));
            }
        }
    }
}

#[test]
fn evaluated_element_addresses_relocate_and_disagreements_remain_unknown() {
    let source = "proc f {} {set k k; set a(k) VALUE; set view $a($k)}";
    let (bindings, registry) = analyse(source);
    let all = reads(&bindings, source);
    let access = all
        .iter()
        .find(|access| access.original_spelling == "$a($k)")
        .unwrap();
    let context = &access.context_alternatives()[0];
    let place = access.place_in_context(context, &registry);
    let crate::place::CellOwner::Activation(owner) = &place.cell.as_ref().unwrap().owner else {
        panic!("actual local element owner");
    };
    let mut relocation = crate::var_resolve::VariableProofRelocation::default();
    relocation
        .activations
        .insert(owner.clone(), "relocated-read".to_owned());
    let relocated = access.relocated_variables(&relocation);
    let target = relocated.place_in_context(&relocated.context_alternatives()[0], &registry);
    assert_eq!(
        target.cell.as_ref().unwrap().owner,
        crate::place::CellOwner::Activation("relocated-read".to_owned())
    );
    assert_eq!(target.index.as_ref().unwrap().value, "k");
    assert_eq!(relocated.source, access.source);
    assert_eq!(relocated.original_spelling, access.original_spelling);

    let mut conflicting = access.clone();
    let mut other = place;
    other.index = Some(crate::place::Index::literal("different"));
    conflicting.retain_selected_place(context, &other);
    let unresolved = conflicting.place_in_context(context, &registry);
    assert_eq!(
        unresolved.index.as_ref().unwrap().kind,
        crate::place::IndexKind::Dynamic
    );
    assert!(!context.read_produces_value(&unresolved, &registry));
}

#[test]
fn word_variable_sites_equal_the_native_read_inventory_on_every_c_release() {
    let source = r#"set a k; set arr(k) VALUE; set {} EMPTY; puts $a; puts ${a}; puts "$a"; puts "${a}"; puts $arr(k); puts "$arr($a)"; puts ${}"#;
    for release in tcl_dialect::TclVersion::ALL {
        let registry =
            tcl_registry::model::ingress::static_context_for(release.dialect_profile_name())
                .commands();
        let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            config,
            registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_native_model::InvocationDialect::for_version(release)),
                ..SourceAnalysisOptions::default()
            },
        );
        let accesses = reads(&bindings, source);
        let map = tcl_lexer::SourceMap::new(source);
        let mut represented = 0;
        for segment in
            tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config)
        {
            let tokens = crate::ir::CommandTokens::from_segmented(&map, config, &segment);
            for word in tokens.words() {
                let Some((_, site)) = word.sole_variable_substitution() else {
                    continue;
                };
                let access =
                    SourceVariableAccess::find_at_site(&accesses, site).unwrap_or_else(|| {
                        panic!("missing exact site on {release:?}: {site:?}; {accesses:#?}")
                    });
                let at = usize::try_from(site.span.start()).unwrap();
                let native = tcl_lexer::word_parts::scan_var_ref(source.as_bytes(), at, config)
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    site.span,
                    native.source_span(source.as_bytes(), at, 0).unwrap()
                );
                assert_eq!(access.original_spelling, &source[at..native.next]);
                represented += 1;
            }
        }
        assert_eq!(represented, 7);
        assert_eq!(
            accesses.len(),
            8,
            "nested index read is independently represented"
        );
    }
}

#[test]
fn argument_read_ownership_follows_evaluation_instead_of_lexical_nesting() {
    let source = "set x 1; proc p {} {puts $x}; puts [expr {$x + 1}]";
    let (bindings, _) = analyse(source);
    let definition = u32::try_from(source.find("proc p").unwrap()).unwrap();
    assert_eq!(
        bindings.variable_accesses_during_argument_evaluation(definition),
        [] as [SourceVariableAccess; 0]
    );
    let outer = u32::try_from(source.rfind("puts [").unwrap()).unwrap();
    let nested = u32::try_from(source.find("expr {").unwrap()).unwrap();
    assert_eq!(
        bindings.variable_accesses_for_invocation_args(outer),
        [] as [SourceVariableAccess; 0]
    );
    assert_eq!(
        bindings.variable_accesses_for_invocation_args(nested),
        [] as [SourceVariableAccess; 0]
    );
    let reached = bindings.variable_accesses_during_argument_evaluation(outer);
    assert_eq!(reached.len(), 1);
    assert!(matches!(
        reached[0].owner,
        SourceVariableEvaluationOwner::NativeExpression { .. }
    ));
    assert_eq!(
        bindings
            .variable_accesses_during_expression_invocation(nested)
            .len(),
        1
    );
    assert_eq!(
        bindings.variable_accesses_during_argument_evaluation(nested),
        [] as [SourceVariableAccess; 0]
    );
}

#[test]
fn script_body_reads_belong_to_enclosing_arguments_but_not_the_script_invocation() {
    let source = "set x 1; puts [catch {puts $x}]";
    let (bindings, _) = analyse(source);
    let outer = u32::try_from(source.find("puts [").unwrap()).unwrap();
    let captured = u32::try_from(source.find("catch {").unwrap()).unwrap();
    assert_eq!(
        bindings
            .variable_accesses_during_argument_evaluation(outer)
            .len(),
        1
    );
    assert_eq!(
        bindings.variable_accesses_during_argument_evaluation(captured),
        [] as [SourceVariableAccess; 0]
    );
    assert_eq!(
        bindings.variable_accesses_during_expression_invocation(captured),
        [] as [SourceVariableAccess; 0]
    );
}

#[test]
fn reads_in_one_word_keep_their_actual_alias_selection() {
    let source = r#"set x X; set y Y; upvar 0 x a; puts "$a[upvar 0 y a]$a""#;
    let (bindings, registry) = analyse(source);
    let accesses = reads(&bindings, source);
    assert_eq!(accesses.len(), 2);
    let places: Vec<_> = accesses
        .iter()
        .map(|access| {
            crate::var_resolve::resolve_substitution_access(
                &access.original_spelling,
                &access.variable_context,
                &registry,
                tcl_registry::TraceOperation::Read,
            )
        })
        .collect();
    assert_eq!(places[0].name, "x");
    assert_eq!(places[1].name, "y");
    assert_ne!(accesses[0].source, accesses[1].source);
    assert!(
        SourceVariableAccess::find_at_source(&accesses, &accesses[0].source, "$unrecorded",)
            .is_none()
    );
}

#[test]
fn exact_read_query_requires_one_retained_record_at_the_site() {
    let source = "set x X; puts $x";
    let (bindings, _) = analyse(source);
    let accesses = reads(&bindings, source);
    let original = &accesses[0];
    assert!(SourceVariableAccess::find_at_source(&accesses, &original.source, "$x").is_some());
    for spelling in ["$x", "$z"] {
        let mut conflicting = original.clone();
        conflicting.original_spelling = spelling.to_owned();
        let duplicated = [original.clone(), conflicting];
        assert!(
            SourceVariableAccess::find_at_source(&duplicated, &original.source, "$x").is_none()
        );
        assert!(SourceVariableAccess::find_at_site(&duplicated, &original.source).is_none());
    }
}

#[test]
fn array_read_context_follows_index_effects_and_records_nested_reads() {
    let source = r#"set k 0; set a(0) FIRST; set a(1) SECOND; puts "$a([set k 1])[set k 0]$a($k)""#;
    let (bindings, registry) = analyse(source);
    let accesses = reads(&bindings, source);
    assert_eq!(accesses.len(), 3, "{accesses:#?}");
    for (spelling, wanted) in [("$a([set k 1])", "1"), ("$a($k)", "0"), ("$k", "0")] {
        let access = accesses
            .iter()
            .find(|access| access.original_spelling == spelling)
            .expect("reached original reference");
        assert_eq!(
            access
                .variable_context
                .substitution_literal_value("$k", &registry),
            Some(wanted)
        );
        let start = usize::try_from(access.source.span.start()).unwrap();
        assert_eq!(&source[start..start + spelling.len()], spelling);
    }
}

#[test]
fn skipped_lazy_and_abrupt_reads_have_no_dispatch_fallback() {
    let source = "set x 1; expr {0 && $lazy}; catch {return; puts $abrupt}; expr {1 ? $x : $other}";
    let (bindings, _) = analyse(source);
    let accesses = reads(&bindings, source);
    assert_eq!(accesses.len(), 1, "{accesses:#?}");
    assert_eq!(accesses[0].original_spelling, "$x");
}

#[test]
fn read_snapshot_precedes_its_own_observer_effect() {
    let source = "set x 1; proc observer args {}; trace add variable x read observer; puts $x";
    let (bindings, registry) = analyse(source);
    let accesses = reads(&bindings, source);
    assert_eq!(accesses.len(), 1);
    let access = &accesses[0];
    assert!(!access.variable_context.dynamic_bindings);
    let place = crate::var_resolve::resolve_substitution_access(
        &access.original_spelling,
        &access.variable_context,
        &registry,
        tcl_registry::TraceOperation::Read,
    );
    assert!(place.observed);
}
