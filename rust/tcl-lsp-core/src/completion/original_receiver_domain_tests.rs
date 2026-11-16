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
use tcl_compiler::analyser::Analyser;

fn fuzzy(source: &str, analysis: &AnalysisResult) -> Vec<CompletionItem> {
    fuzzy_command_fallback(
        source,
        0,
        u32::try_from(source.len()).unwrap(),
        analysis,
        analysis.resolved_registry(),
        analysis.resolved_profile().unwrap(),
        &tcl_lexer::LineIndex::new(source),
    )
}

#[test]
fn native_fuzzy_methods_keep_original_class_and_object_records_without_reports() {
    // naming.consumer.original-receiver-completion-domain
    // docs/design/analysis/name-resolution-proofs/original-receiver-completion-domain.md
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for source in [
            "oo::class create C {self method longMethod {} {}}; C longMethd",
            "oo::class create C {method longMethod {} {}}; C create object; object longMethd",
        ] {
            let mut analysis = Analyser::new().analyse(source, dialect);
            analysis.all_classes.clear();
            analysis.global_scope.classes.clear();
            analysis.instance_classes.clear();
            analysis.created_instance_commands.clear();
            let items = fuzzy(source, &analysis);
            assert!(
                items.iter().any(|item| item.label == "longMethod"),
                "{dialect}: {items:?}"
            );
            assert!(
                items
                    .iter()
                    .filter(|item| item.label == "longMethod")
                    .all(|item| item
                        .detail
                        .as_deref()
                        .is_some_and(|detail| detail.contains("source declaration candidate")))
            );
            let mut changed = analysis.clone();
            changed.body_lexer_config.as_mut().unwrap().strict_quoting ^= true;
            assert!(
                !fuzzy(source, &changed)
                    .iter()
                    .any(|item| item.label == "longMethod")
            );
            assert!(
                !fuzzy(&format!("#{source}"), &analysis)
                    .iter()
                    .any(|item| item.label == "longMethod")
            );
            let mut foreign = analysis.clone();
            let profile = crate::profile_for_dialect("jim");
            foreign.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                tcl_registry::model::ingress::context_for_profile(profile),
                analysis.body_lexer_config.unwrap(),
            ));
            assert!(
                !fuzzy(source, &foreign)
                    .iter()
                    .any(|item| item.label == "longMethod")
            );
        }
    }
}

#[test]
fn native_fuzzy_unknown_receiver_cannot_borrow_nominal_widget_reports() {
    // naming.consumer.original-receiver-completion-domain
    // docs/design/analysis/name-resolution-proofs/original-receiver-completion-domain.md
    let source = ".w confgure";
    let mut analysis = Analyser::new().analyse(source, "tk");
    assert!(!analysis.allows_lexical_declaration_advice());
    analysis
        .instance_classes
        .insert(".w".to_owned(), "button".to_owned());
    analysis.created_instance_commands.insert(".w".to_owned());
    let cursor = u32::try_from(source.len()).unwrap();
    assert!(matches!(
        crate::original_oo::class_completion_context(&analysis, source, cursor),
        std::ops::ControlFlow::Break(None)
    ));
    assert!(
        crate::original_oo::instance_method_completion_record(&analysis, source, cursor).is_none()
    );
    assert!(fuzzy(source, &analysis).iter().all(|item| {
        !item
            .detail
            .as_deref()
            .is_some_and(|detail| detail.starts_with("method — button"))
    }));
    assert!(
        registry_method_items(
            &analysis,
            analysis.resolved_registry().unwrap(),
            "button",
            analysis.resolved_profile().unwrap()
        )
        .is_none()
    );
    let foreign = format!("#{source}");
    assert!(fuzzy(&foreign, &analysis).iter().all(|item| {
        !item
            .detail
            .as_deref()
            .is_some_and(|detail| detail.starts_with("method — button"))
    }));
}
