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
fn return_dependencies_keep_original_template_reads_and_inert_text() {
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for(
        "set value TEXT; return \"#prefix:\\$inert/$value\"",
        &registry,
        false,
    );
    let function = &unit.top_level;
    let symbol = function
        .ssa
        .var_symbol("value")
        .expect("actual scalar source symbol");
    let (value, word, versions) = function
        .cfg
        .blocks
        .iter()
        .find_map(|(id, block)| {
            let Terminator::Return {
                value: Some(value),
                value_word: Some(word),
                ..
            } = block.terminator.as_ref()?
            else {
                return None;
            };
            Some((value, word, &function.ssa.blocks.get(id)?.exit_versions))
        })
        .expect("actual original return word");
    let uses = original_return_word_uses(
        value,
        Some(word),
        versions,
        &function.ssa,
        function.source_lexer_config(),
    );
    assert!(!uses.unknown);
    assert_eq!(uses.uses, HashMap::from([(symbol, versions[&symbol])]));
}

#[test]
fn return_dependencies_keep_literal_names_index_children_and_unknown_versions() {
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let unit = CompilationUnit::build_for(
        "set array(key) VALUE; set index key; set {$literal} VALUE; set {} EMPTY; return [list $array($index) ${$literal} ${}]",
        &registry,
        false,
    );
    let function = &unit.top_level;
    let names = ["array", "index", "$literal", ""];
    let symbols = names.map(|name| {
        function
            .ssa
            .var_symbol(name)
            .expect("actual function symbol")
    });
    let versions = function
        .ssa
        .blocks
        .values()
        .find_map(|block| {
            symbols
                .iter()
                .all(|symbol| block.exit_versions.contains_key(symbol))
                .then_some(&block.exit_versions)
        })
        .expect("actual function exit versions");
    let config = function.source_lexer_config();
    let inventory = return_word_uses_from_versions(
        "[list $array($index) ${$literal} ${}]",
        versions,
        &function.ssa,
        config,
    );
    assert!(!inventory.unknown);
    for symbol in symbols {
        assert_eq!(inventory.uses.get(&symbol), versions.get(&symbol));
    }
    let inert = return_word_uses_from_versions(
        "{$array($index) ${$literal} ${}}",
        versions,
        &function.ssa,
        config,
    );
    assert!(!inert.unknown && inert.uses.is_empty());
    let escaped =
        return_word_uses_from_versions("\\$inert/$index", versions, &function.ssa, config);
    assert!(!escaped.unknown);
    assert_eq!(
        escaped.uses,
        HashMap::from([(symbols[1], versions[&symbols[1]])])
    );
    for original in ["[missing $index", "${name\0tail}", "$unrepresented"] {
        let unknown = return_word_uses_from_versions(original, versions, &function.ssa, config);
        assert!(unknown.unknown, "{original:?}");
        for (symbol, version) in versions {
            assert_eq!(unknown.uses.get(symbol), Some(version));
        }
    }
    let mut missing = versions.clone();
    missing.remove(&symbols[1]);
    let unknown = return_word_uses_from_versions("$index", &missing, &function.ssa, config);
    assert!(unknown.unknown);
    assert!(!unknown.uses.contains_key(&symbols[1]));
}

#[test]
fn original_return_word_keeps_bom_and_actual_ssa_exit_versions() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Lexical SSA-use projection only; no executable return-taint closure.
    let config = tcl_lexer::LexerConfig {
        leading_bom: tcl_lexer::LeadingBom::Skip,
        braced_var: tcl_lexer::BracedVarStyle::FirstClose,
        strict_quoting: true,
        ..tcl_lexer::LexerConfig::default()
    };
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for_with_config(
        "\u{feff}set x VALUE\nreturn $x",
        &registry,
        false,
        config,
    );
    let function = &unit.top_level;
    assert_eq!(function.source_lexer_config(), config);
    let symbol = function.ssa.var_symbol("x").expect("actual SSA variable");
    let versions = function
        .ssa
        .blocks
        .values()
        .find_map(|block| {
            block
                .exit_versions
                .contains_key(&symbol)
                .then_some(&block.exit_versions)
        })
        .expect("actual exit version");
    let expected = HashMap::from([(symbol, versions[&symbol])]);
    let text = "\u{feff}#prefix $x $not_in_this_ssa";
    assert_eq!(
        word_uses_from_versions(
            text,
            versions,
            &function.ssa,
            function.source_lexer_config()
        ),
        expected
    );
    assert_eq!(
        word_uses_from_versions(text, versions, &function.ssa, config.nested()),
        expected
    );
    assert!(
        word_uses_from_versions("{\u{feff}#prefix $x}", versions, &function.ssa, config).is_empty()
    );
    assert_eq!(config.leading_bom, tcl_lexer::LeadingBom::Skip);
}
