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
fn alias_inventory_keeps_inert_data_out_of_selected_nested_phases() {
    let source = "set data {foo --verylongflag --verylongflag}\nproc p {} {foo --verylongflag;foo --verylongflag}\nputs [foo --verylongflag]\n";
    let registry = CommandRegistry::build_default();
    let analysis =
        original::analysis_for_profile(source, tcl_dialect::DialectProfile::plain_tcl(), &registry);
    let input = analysis.resolved_input.as_ref().unwrap();
    let (_, uses) = source_alias_uses(source, input, AliasWordPurpose::Argument);
    assert_eq!(uses["--verylongflag"].len(), 3);
    assert!(
        uses["--verylongflag"]
            .iter()
            .all(|off| *off > source.find("proc p").unwrap())
    );
    let (rewritten, aliases) = alias_repeated_arguments(source, input, &mut HashSet::new());
    assert!(aliases.contains_key("--verylongflag"));
    assert!(rewritten.contains("{foo --verylongflag --verylongflag}"));
}

#[test]
fn alias_heads_retain_whole_bare_geometry_and_native_refusal() {
    let source =
        "mylongcommand x\n{mylongcommand} x\n\"mylongcommand\" x\nset x 1\nset y 2\nset z 3\n";
    let registry = CommandRegistry::build_default();
    let analysis =
        original::analysis_for_profile(source, tcl_dialect::DialectProfile::plain_tcl(), &registry);
    let (_, uses) = source_alias_uses(
        source,
        analysis.resolved_input.as_ref().unwrap(),
        AliasWordPurpose::CommandHead,
    );
    assert_eq!(uses["mylongcommand"], vec![0]);
    assert!(!uses.contains_key("set"));
    let native = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
    assert!(
        source_alias_uses(
            source,
            native.resolved_input.as_ref().unwrap(),
            AliasWordPurpose::CommandHead
        )
        .1
        .is_empty()
    );
}

#[test]
fn alias_source_constructor_preserves_literal_data_and_scalar_read() {
    let config = LexerConfig::for_profile(Some(tcl_dialect::DialectProfile::plain_tcl()));
    let value = "literal;set victim [danger] $read\\tail";
    let spelling = authored_alias_spelling("a", value, config).unwrap();
    let image = tcl_lexer::SourceImage::document(&spelling.assignment);
    let plan = tcl_lexer::native_script_words_in(
        image,
        Span::new(0, u32::try_from(spelling.assignment.len()).unwrap()),
        config,
    )
    .unwrap();
    assert!(plan.fatal_tail.is_none());
    assert_eq!(plan.commands.len(), 1);
    assert_eq!(plan.commands[0].words.len(), 3);
    assert_eq!(
        tcl_syntax::word_rules::original_static_word_unicode_value(&plan.commands[0].words[2])
            .as_deref(),
        Some(value.as_bytes())
    );
    let reference = tcl_lexer::scan_var_ref(spelling.reference.as_bytes(), 0, config)
        .unwrap()
        .unwrap();
    assert_eq!(reference.next, spelling.reference.len());
    assert_eq!(reference.name, b"a");
    assert!(reference.index.is_none());
    assert!(authored_alias_spelling("bad(name)", value, config).is_none());
}
