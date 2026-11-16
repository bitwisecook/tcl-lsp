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
fn selected_original_method_body_requires_real_return_schema_and_static_operand() {
    for (prefix, value, expected) in [
        ("", "VALUE", true),
        ("", "$dynamic", false),
        ("proc return args {}\n", "VALUE", false),
    ] {
        let source = format!(
            "{prefix}oo::class create C {{method literal {{}} {{return {value}}}; method use {{}} {{[my literal] action}}}}"
        );
        let mut analyser = Analyser::new();
        analyser.analyse(&source, "tcl8.6");
        let config = analyser.file_lexer_config();
        let offset = u32::try_from(source.find("my literal").unwrap()).unwrap();
        let command =
            crate::segmenter::segment_commands_with_offset_and_config("my literal", offset, config)
                .remove(0);
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceImage::document(&source).source_map(),
            config,
            &command,
        );
        analyser
            .head_identities
            .source_bindings_ref()
            .stamp_original_tokens(&mut tokens);
        assert_eq!(
            analyser.oo_self_method_returns_literal(&tokens),
            expected,
            "{source}"
        );
        analyser.source = format!("{}{}", analyser.source, "# changed").into();
        assert!(!analyser.oo_self_method_returns_literal(&tokens));
    }
}
