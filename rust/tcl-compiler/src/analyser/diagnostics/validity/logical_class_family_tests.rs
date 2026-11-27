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
use crate::analyser::types::{MetaclassProvenance, SourceNameAmbiguity};
use std::sync::Arc;

const SOURCE: &str = "oo::class create Dog { method bark {} { return woof } }\n[Dog new]\n";

fn inspected_analysis(dialect: &str) -> Analyser {
    let mut analyser = Analyser::new();
    let result = analyser.analyse(SOURCE, dialect);
    analyser.context = result
        .resolved_input
        .as_ref()
        .map(crate::analyser::ResolvedAnalysisInput::context_registry);
    analyser.resolved_input = result.resolved_input.clone();
    analyser.source = SOURCE.to_owned().into();
    analyser.result = result;
    analyser
}

fn family_is_tcloo(analyser: &Analyser) -> bool {
    analyser
        .result
        .all_classes
        .get("::Dog")
        .is_some_and(|class| is_tcloo_source_class(analyser, class))
}

#[test]
fn logical_class_family_uses_observed_definer_without_borrowing_native_formals() {
    // naming.diagnostics.retained-logical-class-family
    // docs/design/analysis/name-resolution-proofs/diagnostic-retained-logical-class-family.md
    let analyser = inspected_analysis("tcl");
    assert!(analyser.result.allows_retained_logical_declaration_advice());
    let class = analyser.result.all_classes.get("::Dog").unwrap();
    assert_eq!(class.metaclass_provenance, MetaclassProvenance::Observed);
    assert!(class.source_name.is_none());
    assert!(family_is_tcloo(&analyser));
    let dialect = tcl_native_model::InvocationDialect::of_profile(
        analyser
            .result
            .resolved_input
            .as_ref()
            .unwrap()
            .unit_profile(),
    );
    assert!(dialect.native_name_protocol().is_none());
    assert!(dialect.parameter_grammar().is_none());
    assert!(
        analyser
            .result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::E001)
    );
}

#[test]
fn logical_class_family_refuses_defaults_missing_and_stale_source_context() {
    // naming.diagnostics.retained-logical-class-family
    // docs/design/analysis/name-resolution-proofs/diagnostic-retained-logical-class-family.md
    let mut analyser = inspected_analysis("tcl");
    assert!(family_is_tcloo(&analyser));
    let original = analyser.result.all_classes.get("::Dog").unwrap().clone();
    analyser
        .result
        .all_classes
        .get_mut("::Dog")
        .unwrap()
        .metaclass_provenance = MetaclassProvenance::StandIn;
    assert!(!family_is_tcloo(&analyser));
    analyser
        .result
        .all_classes
        .insert("::Dog".into(), original.clone());
    analyser
        .result
        .all_classes
        .get_mut("::Dog")
        .unwrap()
        .source_name_ambiguous = SourceNameAmbiguity::Observed;
    assert!(!family_is_tcloo(&analyser));
    analyser.result.all_classes.insert("::Dog".into(), original);
    analyser.source = format!("{}{}", analyser.source, "# changed whole input").into();
    assert!(!family_is_tcloo(&analyser));
    analyser.source = SOURCE.to_owned().into();
    analyser.profile = tcl_registry::model::ingress::resolve_environment("tcl9.1").unit_profile();
    analyser.result.dialect = "tcl9.1".into();
    assert!(
        family_is_tcloo(&analyser),
        "display profiles cannot replace the retained context"
    );
    let retained_context = analyser.context.clone();
    let unavailable =
        tcl_registry::model::ingress::resolve_environment("tcl8.4").default_context_registry();
    analyser.context = Some(Arc::new(
        unavailable.with_command_store(retained_context.as_ref().unwrap().commands().clone()),
    ));
    assert!(
        !family_is_tcloo(&analyser),
        "same store with another context is stale"
    );
    analyser.context = retained_context;
    let retained_input = analyser.result.resolved_input.take();
    assert!(
        !family_is_tcloo(&analyser),
        "a copied lexical flag supplies no Logical input"
    );
    analyser.result.resolved_input = retained_input;
    assert!(family_is_tcloo(&analyser));
}

#[test]
fn native_and_hosted_family_classification_cannot_fall_back_to_logical_reports() {
    // naming.diagnostics.retained-logical-class-family
    // docs/design/analysis/name-resolution-proofs/diagnostic-retained-logical-class-family.md
    for dialect in [
        "tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl", "irules", "tmsh",
    ] {
        let mut analyser = inspected_analysis(dialect);
        assert!(
            !analyser.result.allows_retained_logical_declaration_advice(),
            "{dialect}"
        );
        assert!(
            analyser
                .retained_logical_class_definer_grammar("::Dog")
                .is_none(),
            "{dialect}"
        );
        if let Some(class) = analyser.result.all_classes.get_mut("::Dog") {
            // Retain every reporting field but remove its canonical source producer.
            class.source_name = None;
            assert!(
                !family_is_tcloo(&analyser),
                "{dialect}: reports cannot donate a Native class receipt"
            );
        }
    }
}
