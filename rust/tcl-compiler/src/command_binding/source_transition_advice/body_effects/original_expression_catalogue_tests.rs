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

use crate::registry_invocation::source_structure::{OriginalRegistrySource, source_registry_words};

fn child(
    source: &str,
    spelling: &str,
    config: tcl_lexer::LexerConfig,
) -> tcl_source::segment::SegmentedCommand {
    let start = source.rfind(spelling).unwrap();
    tcl_source::segment::segment_commands_with_offset_and_config(
        &source[start..start + spelling.len()],
        u32::try_from(start).unwrap(),
        config.nested(),
    )
    .remove(0)
}

#[test]
fn original_expression_children_keep_the_parent_word_without_reached_lookup() {
    let source = "expr {{${inert} [missing $also_inert]} eq \"${active}\" && $array($index) ne [set value $nested]}";
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let analysis = crate::analyser::Analyser::new().analyse(source, dialect);
        let input = analysis.resolved_input.as_ref().unwrap();
        let config = input.lexer_config();
        let command = child(source, "set value $nested", config);
        let offset = command.argv[0].span.start();
        let realm = analysis.retained_command_realm().unwrap();
        let original = realm.source_bindings_ref().invocation_at_source("", offset);
        assert!(original.lookup_state.is_none());
        assert!(original.unknown && original.targets.is_empty());
        let words = source_registry_words(source, &analysis, &command).expect(dialect);
        let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
            panic!("original expression child catalogue")
        };
        let Some(super::super::OriginalSourceCatalogueBody::Expression(owner)) =
            advice.original_catalogue_source_body()
        else {
            panic!("checked original expression parent required")
        };
        assert_eq!(owner.content_span().start(), offset);
        assert_eq!(
            owner.original.written_bytes(),
            &source.as_bytes()[5..source.len()]
        );
        assert!(owner.matches_source(&tcl_lexer::SourceImage::document(source), config));
        assert!(owner.matches_context(input.borrowed_context_registry()));
        assert!(advice.logical_source_body().is_none());
        assert!(advice.original_procedure_source_body().is_none());
        assert!(!words.operands_preserve_source_lookup());
        assert!(advice.obligations().contains(&super::SourceCommandTransitionObligation::ConditionalOriginalBodyCatalogueApplicability));
        assert!(matches!(
            realm
                .source_bindings_ref()
                .original_source_transition_advice_tape(input.borrowed_context_registry())
                .unwrap()
                .lookup(offset),
            super::super::OriginalSourceTransitionAdviceLookup::Unrepresented
        ));
        assert!(source_registry_words(&format!("{source} "), &analysis, &command).is_none());
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(source_registry_words(source, &missing, &command).is_none());
        let mut changed = analysis.clone();
        changed.body_lexer_config.as_mut().unwrap().strict_quoting = !config.strict_quoting;
        assert!(source_registry_words(source, &changed, &command).is_none());
    }
}

#[test]
fn original_expression_catalogue_keeps_aliases_and_rejects_inert_or_replaced_commands() {
    for source in [
        "expr {{[set inert $unread]} eq \"${active}\" && [set value $nested]}",
        "interp alias {} expression {} expr; expression {[set value $nested]}",
    ] {
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let config = analysis.body_lexer_config.unwrap();
        let selected = child(source, "set value $nested", config);
        assert!(source_registry_words(source, &analysis, &selected).is_some());
        if source.contains("set inert") {
            let inert = child(source, "set inert $unread", config);
            assert!(source_registry_words(source, &analysis, &inert).is_none());
        }
        let foreign = crate::analyser::Analyser::new().analyse(source, "tcl9.0");
        let mut changed = analysis.clone();
        changed.resolved_input = foreign.resolved_input;
        assert!(source_registry_words(source, &changed, &selected).is_none());
    }
    for source in [
        "proc expr {args} {}; expr {[set value $nested]}",
        "proc set {args} {}; expr {[set value $nested]}",
    ] {
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let selected = child(
            source,
            "set value $nested",
            analysis.body_lexer_config.unwrap(),
        );
        assert!(source_registry_words(source, &analysis, &selected).is_none());
    }
}
