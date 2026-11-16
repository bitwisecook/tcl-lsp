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
fn aggressive_alias_candidates_reject_only_observed_spellings_under_whole_config() {
    let word = "retained_long_argument";
    let source = [word; 3].join(" ");
    let order = vec![word.to_owned()];
    let uses = HashMap::from([(
        word.to_owned(),
        vec![0, word.len() + 1, 2 * (word.len() + 1)],
    )]);
    let mut claimed = HashSet::from([
        "a".to_owned(),
        "b".to_owned(),
        "$c".to_owned(),
        "c(index)".to_owned(),
        "::c".to_owned(),
        "é".to_owned(),
    ]);
    let config = LexerConfig {
        strict_quoting: true,
        leading_bom: tcl_lexer::LeadingBom::Skip,
        ..LexerConfig::default()
    };
    let (rewritten, aliases) = alias_by_uses(&source, &order, &uses, &mut claimed, config);
    assert_eq!(aliases.get(word).map(String::as_str), Some("c"));
    assert_eq!(rewritten, format!("set c {word}\n$c $c $c"));
    assert!(claimed.contains("c"));
    assert_eq!(
        tcl_syntax::naming::scalar_source_reference("c", config, false).as_deref(),
        Some("$c")
    );
}
