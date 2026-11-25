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
use std::sync::Arc;
use tcl_compiler::analyser::{Analyser, ResolvedAnalysisInput};
use tcl_registry::definer::{DefinerFamily, DefinitionBodyGrammar, ITCL_GRAMMAR};

const OTHER_FAMILY: DefinitionBodyGrammar = DefinitionBodyGrammar {
    family: DefinerFamily::Snit,
    profile: tcl_registry::definer::SNIT_PROFILE,
    ..ITCL_GRAMMAR
};

fn analyse(source: &str, grammar: Option<&'static DefinitionBodyGrammar>) -> AnalysisResult {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let mut registry = CommandRegistry::build_default();
    let mut spec = registry.get("itcl::class").unwrap().clone();
    spec.name = "source_class";
    spec.required_package = None;
    spec.definition_body = grammar;
    registry.insert(spec);
    let context = tcl_registry::model::ingress::context_for_profile(profile)
        .with_command_store(Arc::new(registry));
    let input = ResolvedAnalysisInput::new(
        profile,
        profile,
        Arc::new(context),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name)
}

#[test]
fn contextual_itcl_targets_use_the_actual_store_instead_of_a_display_profile() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    // Authored Logical definition grammar; no Native Itcl loader or class issuer.
    let source = "source_class C {proc make {} {return OK}}\nC::make\n";
    let analysis = analyse(source, Some(&ITCL_GRAMMAR));
    assert!(analysis.allows_lexical_declaration_advice());
    let class = &analysis.all_classes["::C"];
    assert_eq!(class.metaclass, "source_class");
    assert!(
        crate::registry_for_dialect_profile(tcl_dialect::DialectProfile::plain_tcl())
            .get("source_class")
            .is_none()
    );
    assert_eq!(
        class_definer_family(&analysis, class),
        Some(DefinerFamily::Itcl)
    );
    let expected = Some(("::C".to_owned(), "make".to_owned()));
    for nominal in ["tcl8.4", "tcl9.1"] {
        assert_eq!(
            itcl_class_proc_target_at(
                source,
                tcl_dialect::DialectProfile::find(nominal).unwrap(),
                1,
                4,
                &analysis
            ),
            expected
        );
    }
    assert_eq!(
        itcl_class_proc_declaration(&analysis, "::", "C::make"),
        Some(class.class_methods["make"].name_span)
    );
    assert_eq!(
        itcl_self_qualified_candidate(&analysis, "::C", "C::make"),
        Some("::C::make".to_owned())
    );
    assert!(itcl_self_qualified_candidate(&analysis, "::C", "C::make::extra").is_none());
    assert!(itcl_self_qualified_candidate(&analysis, "::C", "Other::make").is_none());
    assert!(
        itcl_class_proc_target_at(
            &format!("{source}# changed"),
            tcl_dialect::DialectProfile::plain_tcl(),
            1,
            4,
            &analysis
        )
        .is_none()
    );
}

#[test]
fn same_profile_family_metadata_keeps_known_non_itcl_and_missing_grammar_distinct() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    let source = "source_class C {proc make {} {return OK}}\nC::make\nC make\n";
    let itcl = analyse(source, Some(&ITCL_GRAMMAR));
    let other = analyse(source, Some(&OTHER_FAMILY));
    assert_eq!(
        itcl.resolved_profile().unwrap().name,
        other.resolved_profile().unwrap().name
    );
    assert_ne!(
        itcl.resolved_registry().unwrap().snapshot().semantic_key(),
        other.resolved_registry().unwrap().snapshot().semantic_key()
    );
    assert_eq!(
        class_definer_family(&other, &other.all_classes["::C"]),
        Some(DefinerFamily::Snit)
    );
    assert!(itcl_class_proc_at(&other, "::C::make").is_none());
    let missing = analyse(source, None);
    // An explicitly supplied descriptive class row is insufficient without
    // its actual store's grammar; it does not acquire the old class receipt.
    assert!(class_definer_family(&missing, &itcl.all_classes["::C"]).is_none());
    assert!(itcl_class_proc_at(&missing, "::C::make").is_none());
}

#[test]
fn contextual_itcl_family_refuses_missing_or_foreign_retained_input() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    let source = "source_class C {proc make {} {return OK}}\nC::make\n";
    let original = analyse(source, Some(&ITCL_GRAMMAR));
    let mut changed = original.clone();
    changed.resolved_input = None;
    assert!(class_definer_family(&changed, &changed.all_classes["::C"]).is_none());
    assert!(itcl_class_proc_at(&changed, "::C::make").is_none());
    let other = analyse(source, Some(&OTHER_FAMILY));
    changed.resolved_input = other.resolved_input.clone();
    assert!(class_definer_family(&changed, &changed.all_classes["::C"]).is_none());
    assert!(
        itcl_class_proc_target_at(
            source,
            tcl_dialect::DialectProfile::plain_tcl(),
            1,
            4,
            &changed
        )
        .is_none()
    );
    changed = original;
    changed.dialect = "a reporting label".to_owned();
    assert_eq!(
        class_definer_family(&changed, &changed.all_classes["::C"]),
        Some(DefinerFamily::Itcl)
    );
}

#[test]
fn actual_itcl_reference_calls_keep_source_currency_and_the_selected_family() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    let source = "source_class C {proc make {} {return OK}}\nC::make\nC make\n";
    let itcl = analyse(source, Some(&ITCL_GRAMMAR));
    let sites = crate::references::obj_method_call_sites(
        source,
        tcl_dialect::DialectProfile::find("tcl9.1").unwrap(),
        &itcl,
        "::C",
        "make",
        true,
        &[],
    );
    assert_eq!(sites.len(), 1);
    assert_eq!(
        &source[sites[0].start() as usize..sites[0].end() as usize],
        "make"
    );
    assert_eq!(
        sites[0].start() as usize,
        source.find("C::make").unwrap() + 3
    );
    let changed = format!("{source}# changed");
    assert_eq!(
        crate::references::obj_method_call_sites(
            &changed,
            tcl_dialect::DialectProfile::plain_tcl(),
            &itcl,
            "::C",
            "make",
            true,
            &[]
        ),
        [] as [tcl_lexer::Span; 0]
    );
    let mut missing = itcl.clone();
    missing.resolved_input = None;
    assert_eq!(
        crate::references::obj_method_call_sites(
            source,
            tcl_dialect::DialectProfile::plain_tcl(),
            &missing,
            "::C",
            "make",
            true,
            &[]
        ),
        [] as [tcl_lexer::Span; 0]
    );
    let other = analyse(source, Some(&OTHER_FAMILY));
    assert!(
        crate::references::obj_method_call_sites(
            source,
            tcl_dialect::DialectProfile::plain_tcl(),
            &other,
            "::C",
            "make",
            true,
            &[]
        )
        .iter()
        .all(|span| span.start() as usize != source.find("C::make").unwrap() + 3)
    );
}

#[test]
fn family_identity_does_not_change_qualified_member_dispatch() {
    const RENAMED_FAMILY: DefinitionBodyGrammar = DefinitionBodyGrammar {
        family: DefinerFamily::Snit,
        ..ITCL_GRAMMAR
    };
    let source = "source_class C {proc make {} {return OK}}\nC::make\n";
    let analysis = analyse(source, Some(&RENAMED_FAMILY));
    assert_eq!(
        class_definer_family(&analysis, &analysis.all_classes["::C"]),
        Some(DefinerFamily::Snit)
    );
    assert_eq!(
        itcl_class_proc_at(&analysis, "::C::make"),
        Some((&"::C".to_owned(), "make".to_owned()))
    );
}
