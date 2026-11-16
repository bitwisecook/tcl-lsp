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

use super::{OriginalRegistrySource, source_registry_words_at};
use crate::analyser::{Analyser, AnalysisResult, ResolvedAnalysisInput};
use crate::command_binding::SourceCommandTransitionObligation;

const PRIVATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../experiments/name-handling/patterns/composed-callback-prefix/fixtures/private-positive.tcl"
));
const WRITER: &str =
    "interp alias {} ::write {} ::set\noo::class create C {method setup {} {::write slot value}}";

fn analyse(source: &str, release: &str) -> AnalysisResult {
    let environment = tcl_registry::model::ingress::resolve_environment(release);
    let profile = environment.analyser_profile();
    let input = ResolvedAnalysisInput::new(
        profile,
        profile,
        environment.default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name)
}

#[test]
fn absolute_member_alias_catalogue_retains_generic_source_metadata() {
    for release in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for (source, needle) in [
            (PRIVATE, "::makePrefix my tick"),
            (WRITER, "::write slot value"),
        ] {
            let analysis = analyse(source, release);
            let offset = u32::try_from(source.find(needle).unwrap()).unwrap();
            let words = source_registry_words_at(source, &analysis, offset).expect(release);
            let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
                panic!("{release}: missing conditional member catalogue")
            };
            assert!(advice.obligations().contains(
                &SourceCommandTransitionObligation::ConditionalMemberBodyCatalogueApplicability
            ));
            assert!(advice.matches_source(
                &tcl_lexer::SourceImage::document(source),
                analysis.body_lexer_config.unwrap()
            ));
            assert!(!advice.matches_source(
                &tcl_lexer::SourceImage::native(source.as_bytes()),
                analysis.body_lexer_config.unwrap()
            ));
            let input = analysis.resolved_input.as_ref().unwrap();
            assert_eq!(
                words.with_source_schema(input.borrowed_context_registry(), |schema| schema
                    .facts()
                    .arity_accepts_frozen_arguments()),
                Some(Some(true))
            );
            let realm = analysis.retained_command_realm().unwrap();
            let config = input.lexer_config();
            let end = advice.original_words().last().unwrap().span().end();
            let command = crate::segmenter::segment_commands_with_offset_and_config(
                &source[offset as usize..end as usize],
                offset,
                config.nested(),
            )
            .remove(0);
            let mut tokens = crate::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceMap::from_image(&tcl_lexer::SourceImage::document(source)),
                config,
                &command,
            );
            realm.stamp_original_tokens(&mut tokens);
            assert!(
                crate::registry_invocation::normal_transfer_invocation(
                    input.borrowed_context_registry().commands(),
                    None,
                    &tokens
                )
                .is_none()
            );
            assert!(
                crate::registry_invocation::admitted_native_compiler_invocation(
                    input.borrowed_context_registry().commands(),
                    None,
                    &tokens
                )
                .is_none()
            );
        }
    }
}

#[test]
fn absolute_member_alias_catalogue_preserves_barriers_and_currency() {
    for release in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for source in [
            PRIVATE.replacen(
                "oo::class create",
                "proc ::makePrefix args {}\noo::class create",
                1,
            ),
            PRIVATE.replacen(
                "oo::class create",
                "rename ::makePrefix {}\noo::class create",
                1,
            ),
            PRIVATE.replacen("set cb", "rename ::makePrefix {}; set cb", 1),
            PRIVATE.replacen("[::makePrefix my tick", "[makePrefix my tick", 1),
        ] {
            let needle = if source.contains("[makePrefix my tick") {
                "makePrefix my tick"
            } else {
                "::makePrefix my tick"
            };
            let analysis = analyse(&source, release);
            let offset = u32::try_from(source.find(needle).unwrap()).unwrap();
            assert!(
                source_registry_words_at(&source, &analysis, offset).is_none(),
                "{release}: {source}"
            );
        }
        let analysis = analyse(PRIVATE, release);
        let offset = u32::try_from(PRIVATE.find("::makePrefix my tick").unwrap()).unwrap();
        assert!(
            source_registry_words_at(&format!("{PRIVATE}# changed image\n"), &analysis, offset)
                .is_none()
        );
        let mut stale = analysis.clone();
        stale.body_lexer_config.as_mut().unwrap().expand_syntax ^= true;
        assert!(source_registry_words_at(PRIVATE, &stale, offset).is_none());
        let input = analysis.resolved_input.as_ref().unwrap();
        let mut store = tcl_registry::CommandRegistry::build_default();
        store.insert(tcl_registry::CommandSpec {
            name: "foreign_member_catalogue_row",
            ..tcl_registry::CommandSpec::DEFAULT
        });
        let mut foreign = analysis.clone();
        foreign.resolved_input = Some(ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            std::sync::Arc::new(
                input
                    .borrowed_context_registry()
                    .with_command_store(std::sync::Arc::new(store)),
            ),
            input.lexer_config(),
        ));
        assert!(source_registry_words_at(PRIVATE, &foreign, offset).is_none());
    }
}
