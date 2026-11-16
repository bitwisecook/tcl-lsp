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
fn proposed_metadata_keeps_separate_image_and_refuses_extra_commands_or_expansion() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let context = tcl_registry::model::ingress::context_for_profile(profile);
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, context.clone(), config);
    let source = "puts original";
    let syntax = OriginalSourceRegistryContext::capture(source, input);
    let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    let original = syntax.words(&commands[0]).unwrap();
    for (proposal, accepted) in [
        ("list proposed", true),
        ("list proposed; puts extra", false),
        ("list {*}$args", false),
    ] {
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            tcl_lexer::SourceImage::document(proposal),
            tcl_lexer::Span::new(0, u32::try_from(proposal.len()).unwrap()),
            config,
            tcl_lexer::SourceSegmentationPurpose::WholeInput,
        )
        .unwrap();
        assert_eq!(
            original
                .with_stock_source_command_proposal(&context, &plan.commands[0].words, |_| ())
                .is_some(),
            accepted
        );
    }
    assert_eq!(original.command(), "puts");
    assert_eq!(
        original.head_source().unwrap().word().unwrap().image(),
        syntax.source_image()
    );
}
