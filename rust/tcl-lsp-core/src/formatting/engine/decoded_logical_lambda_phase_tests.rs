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
fn formatter_selects_decoded_logical_roles_in_the_field_phase_only() {
    let source = r"apply {{} if\ 1\ \{puts\ hi\}}";
    let config = FormatterConfig::default();
    let registry = CommandRegistry::build_default();
    let layout = FormattingSourceLayout::new(source, &config, &registry);
    let plan = tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::document(source),
        tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
        config.lexer_config(),
    )
    .unwrap();
    let word = &plan.commands[0].words[1];
    let phase = tcl_compiler::command_binding::OriginalConditionalLambdaFormattingBody::capture(
        &layout.realm,
        &layout.input,
        word,
    )
    .unwrap();
    let text = phase.image().try_text().unwrap().to_owned();
    let map = SourceMap::new(&text);
    let tokens = Lexer::with_config(&text, config.lexer_config())
        .tokenise_all()
        .unwrap();
    let mut command = parse_commands(&text, &map, &tokens, config.lexer_config())
        .unwrap()
        .0
        .remove(0);
    let phase_layout = FormattingSourceLayout {
        realm: layout.realm.clone(),
        analysis: None,
        input: layout.input.clone(),
        original: None,
        decoded_phase: None,
        conditional_phase: Some(phase),
    };
    identify_body_args(&mut command, &registry, &phase_layout, 0, &config, &text);
    assert_eq!(command.args[2].kind, ArgKind::Body);
    assert!(command.logical_roles.is_some());
    assert!(command.args.iter().all(|arg| arg.original_input.is_none()));
    assert!(command.expression_bracing.is_none());
    let mut unjoined = parse_commands(&text, &map, &tokens, config.lexer_config())
        .unwrap()
        .0
        .remove(0);
    identify_body_args(&mut unjoined, &registry, &layout, u32::MAX, &config, &text);
    assert_eq!(unjoined.args[2].kind, ArgKind::Word);
}
