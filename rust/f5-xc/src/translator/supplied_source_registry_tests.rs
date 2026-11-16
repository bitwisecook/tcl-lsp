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

fn assert_selected_override(
    source: &str,
    registry: &CommandRegistry,
    offset: u32,
    name: &str,
    flag: Option<bool>,
) {
    let (_, analysis) = translation_source(source, registry);
    let words = tcl_compiler::registry_invocation::source_structure::source_registry_words_at(
        source, &analysis, offset,
    )
    .expect("original selected source occurrence");
    let input = analysis.resolved_input.as_ref().unwrap();
    assert_eq!(
        words.with_source_schema(&input.context_registry(), |schema| {
            let actual = schema.authored_source_descriptors().command;
            assert!(
                std::ptr::eq(actual, registry.get(name).unwrap()),
                "actual supplied descriptor"
            );
            assert_eq!(tcl_registry::registry::spec_pack_of(actual), None);
            actual.xc_translatable
        }),
        Some(flag)
    );
}

#[test]
fn supplied_registry_source_availability_cannot_borrow_shipped_translation() {
    let mut registry = irules_registry().project_for_profile(tcl_dialect::DialectProfile::irules());
    registry.insert(tcl_registry::CommandSpec {
        name: "pool",
        xc_translatable: Some(false),
        ..irules_registry().get("pool").unwrap().clone()
    });
    let source = "when HTTP_REQUEST {pool retained_custom_pool}";
    let (unit, analysis) = translation_source(source, &registry);
    assert_eq!(
        unit.ir_module
            .registry_snapshot
            .as_ref()
            .unwrap()
            .semantic_key(),
        registry.snapshot().semantic_key()
    );
    assert_eq!(
        analysis
            .resolved_registry()
            .unwrap()
            .snapshot()
            .semantic_key(),
        registry.snapshot().semantic_key()
    );
    assert_selected_override(source, &registry, 19, "pool", Some(false));
    let result = translate_irule_with_registry(source, &registry);
    assert_eq!(result.origin_pools, [] as [crate::model::XCOriginPool; 0]);
    assert_eq!(result.routes, [] as [crate::model::XCRoute; 0]);
    assert_ne!(result.items, [] as [crate::model::TranslationItem; 0]);
    for override_flag in [None, Some(true)] {
        let mut custom =
            irules_registry().project_for_profile(tcl_dialect::DialectProfile::irules());
        custom.insert(tcl_registry::CommandSpec {
            name: "pool",
            xc_translatable: override_flag,
            ..irules_registry().get("pool").unwrap().clone()
        });
        assert_selected_override(source, &custom, 19, "pool", override_flag);
        let result = translate_irule_with_registry(source, &custom);
        assert_eq!(result.origin_pools, [] as [crate::model::XCOriginPool; 0]);
        assert_eq!(result.routes, [] as [crate::model::XCRoute; 0]);
    }
    let getter_source = r#"when HTTP_REQUEST {if {[HTTP::path] eq "/a"} {pool p}}"#;
    let shipped = irules_registry().project_for_profile(tcl_dialect::DialectProfile::irules());
    let original = translate_irule_with_registry(getter_source, &shipped);
    assert_eq!(original.routes.len(), 1);
    assert_eq!(original.origin_pools.len(), 1);
    for override_flag in [None, Some(true), Some(false)] {
        let mut custom =
            irules_registry().project_for_profile(tcl_dialect::DialectProfile::irules());
        custom.insert(tcl_registry::CommandSpec {
            name: "HTTP::path",
            xc_translatable: override_flag,
            ..irules_registry().get("HTTP::path").unwrap().clone()
        });
        assert_selected_override(getter_source, &custom, 24, "HTTP::path", override_flag);
        let result = translate_irule_with_registry(getter_source, &custom);
        assert!(
            result.routes.is_empty(),
            "custom getter flag {override_flag:?}"
        );
        assert_ne!(result.items, [] as [crate::model::TranslationItem; 0]);
    }
}

#[test]
fn document_translation_walks_the_documents_retained_registry() {
    let source = "when HTTP_REQUEST {pool retained_custom_pool}";
    let standalone = translate_irule(source);
    assert_eq!(standalone.origin_pools.len(), 1, "shipped pool translates");

    let mut registry = irules_registry().project_for_profile(tcl_dialect::DialectProfile::irules());
    registry.insert(tcl_registry::CommandSpec {
        name: "pool",
        xc_translatable: Some(false),
        ..irules_registry().get("pool").unwrap().clone()
    });
    let (unit, analysis) = translation_source(source, &registry);
    let document = translate_irule_for_document(source, &unit, &analysis)
        .expect("an iRules document analysis of this exact source");
    let supplied = translate_irule_with_registry(source, &registry);
    assert!(document.origin_pools.is_empty(), "the document's own spec");
    assert_eq!(document.items, supplied.items);
    assert_eq!(document.coverage_pct, supplied.coverage_pct);

    let (shipped_unit, shipped_analysis) = translation_source(source, irules_registry());
    let shipped = translate_irule_for_document(source, &shipped_unit, &shipped_analysis)
        .expect("shipped iRules document");
    assert_eq!(shipped.items, standalone.items);
    assert_eq!(shipped.origin_pools.len(), 1);
}

#[test]
fn document_translation_refuses_other_source_or_a_non_irules_input() {
    let source = "when HTTP_REQUEST {pool p}";
    let (unit, analysis) = translation_source(source, irules_registry());
    let edited = "when HTTP_REQUEST {pool q}";
    assert!(translate_irule_for_document(edited, &unit, &analysis).is_none());
    let (edited_unit, _) = translation_source(edited, irules_registry());
    assert!(translate_irule_for_document(source, &edited_unit, &analysis).is_none());
    assert!(translate_irule_for_document(edited, &edited_unit, &analysis).is_none());

    let tcl = Analyser::new().analyse(source, "tcl8.6");
    assert!(translate_irule_for_document(source, &unit, &tcl).is_none());
}
