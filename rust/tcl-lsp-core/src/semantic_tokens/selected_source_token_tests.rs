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

#[test]
fn original_loop_handle_advice_keeps_actual_availability_captures_and_inert_data() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let mut registry = CommandRegistry::build_default();
    let mut each = registry.get("foreach").unwrap().clone();
    each.name = "selected_each";
    each.surface = Some(tcl_dialect::model::SpecSurface::TCL90_PLUS);
    registry.insert(each);
    let registry = std::sync::Arc::new(registry);
    let source = "interp alias {} each {} selected_each {left right}
set data {selected_each bogus $items {}}
each $items {}";
    let current = crate::refactor::test_logical_analysis(source, "tcl9.0", registry.clone());
    let collections = ObjectClassMap::from([(
        String::from("items"),
        std::collections::HashSet::from([String::from("::Element")]),
    )]);
    let mut handles = ObjectClassMap::default();
    augment_loop_var_handles(source, &current, &collections, &mut handles);
    assert!(handles.contains_key("left"));
    assert!(handles.contains_key("right"));
    assert!(!handles.contains_key("bogus"));
    let old = crate::refactor::test_logical_analysis(source, "tcl8.6", registry.clone());
    handles.clear();
    augment_loop_var_handles(source, &old, &collections, &mut handles);
    assert!(handles.is_empty());
    let replaced = "proc selected_each args {}
selected_each hidden $items {}";
    let analysis = crate::refactor::test_logical_analysis(replaced, "tcl9.0", registry);
    augment_loop_var_handles(replaced, &analysis, &collections, &mut handles);
    assert!(handles.is_empty());
}

#[test]
fn original_semantic_handle_collectors_keep_later_selected_commands_and_inert_data() {
    let source = "puts preface\nforeach first $items {}\nset inert {foreach leaked $items {}}\nforeach second $items {}";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
    let collections = ObjectClassMap::from([(
        "items".to_owned(),
        std::collections::HashSet::from(["::Element".to_owned()]),
    )]);
    let mut handles = ObjectClassMap::default();
    augment_loop_var_handles(source, &analysis, &collections, &mut handles);
    assert!(handles.contains_key("first"), "{handles:?}");
    assert!(handles.contains_key("second"), "{handles:?}");
    assert!(!handles.contains_key("leaked"));
    handles.clear();
    augment_loop_var_handles(
        &source.replace("preface", "changed"),
        &analysis,
        &collections,
        &mut handles,
    );
    assert!(handles.is_empty());
    let mut missing = analysis;
    missing.resolved_input = None;
    augment_loop_var_handles(source, &missing, &collections, &mut handles);
    assert!(handles.is_empty());
}

#[test]
fn original_member_handle_advice_uses_the_genuine_factory_member_body() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let source = "package require snit
snit::type Engine {method run {} {}}
snit::type Wrapper {constructor {} {install item using Engine $win.a; install unknown using $type instance; proc nested {} {install borrowed using Engine instance}}}
set inert {install leaked using Engine instance}";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
    assert!(analysis.allows_lexical_declaration_advice());
    let context = analysis.resolved_input.as_ref().unwrap().context_registry();
    let regions = selected_member_handle_regions(source, &analysis, &context);
    assert!(regions.iter().any(|(span, _)| {
        source[span.as_range()].starts_with("install item using Engine $win.a")
    }));
    let mut handles = ObjectClassMap::default();
    augment_snit_handles(
        source,
        &analysis,
        Some(analysis.class_hierarchy()),
        &mut handles,
    );
    assert!(handles.contains_key("item"));
    assert!(!handles.contains_key("leaked"));
    assert!(!handles.contains_key("unknown"));
    assert!(!handles.contains_key("borrowed"));
    let changed = source.replace("Engine instance", "Engine other");
    handles.clear();
    augment_snit_handles(
        &changed,
        &analysis,
        Some(analysis.class_hierarchy()),
        &mut handles,
    );
    assert!(handles.is_empty());
}

#[test]
fn original_member_handle_advice_preserves_literal_name_and_class_data() {
    let source = "package require snit\n\
            snit::type {Engine $[literal]} {method run {} {}}\n\
            snit::type Wrapper {constructor {} {\n\
            install {slot $[literal]} using {Engine $[literal]} instance\n\
            install dynamic using $type instance\n\
            }}";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
    assert!(analysis.allows_lexical_declaration_advice());
    let mut handles = ObjectClassMap::default();
    augment_snit_handles(
        source,
        &analysis,
        Some(analysis.class_hierarchy()),
        &mut handles,
    );
    assert!(handles.contains_key("slot $[literal]"), "{handles:?}");
    assert!(!handles.contains_key("dynamic"));
    let changed = source.replace("instance", "changed");
    handles.clear();
    augment_snit_handles(
        &changed,
        &analysis,
        Some(analysis.class_hierarchy()),
        &mut handles,
    );
    assert!(handles.is_empty());
}

#[test]
fn original_semantic_tokens_withdraw_stale_grammar_foreign_and_missing_source_owner() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let source = "set x 1
puts $x";
    let mut analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
    let registry = CommandRegistry::build_default();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let tokens = |source: &str, analysis: &AnalysisResult| {
        full_with_cu_and_analysis(source, profile, &registry, None, Some(analysis))
    };
    assert_ne!(tokens(source, &analysis).data, [] as [u32; 0]);
    assert_eq!(
        tokens(
            "set y 1
puts $y",
            &analysis
        )
        .data,
        [] as [u32; 0]
    );
    let config = analysis.body_lexer_config.unwrap();
    analysis.body_lexer_config.as_mut().unwrap().strict_quoting = !config.strict_quoting;
    assert_eq!(tokens(source, &analysis).data, [] as [u32; 0]);
    analysis.body_lexer_config = Some(config);
    analysis.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::resolve_environment("tcl8.4").default_context_registry(),
        config,
    ));
    assert_eq!(tokens(source, &analysis).data, [] as [u32; 0]);
    analysis.resolved_input = None;
    assert_eq!(tokens(source, &analysis).data, [] as [u32; 0]);
}
