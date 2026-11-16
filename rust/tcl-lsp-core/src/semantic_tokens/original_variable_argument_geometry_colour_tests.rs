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
use tcl_compiler::analyser::{Analyser, AnalysisResult};

fn declarations_at(source: &str, analysis: &AnalysisResult, spelling: &str) -> bool {
    let start = u32::try_from(source.rfind(spelling).unwrap()).unwrap();
    let expected = LineIndex::new(source).position_at_utf16(start, source);
    tests::decode_semantic(&full_with_cu_and_analysis(
        source,
        analysis.resolved_profile().unwrap(),
        analysis.resolved_registry().unwrap(),
        None,
        Some(analysis),
    ))
    .iter()
    .any(|&(line, column, _, kind, modifiers)| {
        line == expected.line
            && column == expected.character.get()
            && kind == TokenKind::Variable as u32
            && modifiers == MOD_DECLARATION
    })
}

#[test]
fn member_variable_colours_require_genuine_parent_roles_and_original_words() {
    for profile in ["tcl", "tcl8.6"] {
        for (source, spelling) in [
            (r"oo::class create C {variable a\ b}", r"a\ b"),
            ("oo::class create C {variable {a b}}", "{a b}"),
            ("oo::class create C {variable arr($index)}", "arr("),
            (
                r"oo::class create C {self variable wrapped\ name}",
                r"wrapped\ name",
            ),
            (
                r"oo::class create C {}; oo::define C variable inline\ name",
                r"inline\ name",
            ),
        ] {
            let analysis = Analyser::new().analyse(source, profile);
            assert!(
                declarations_at(source, &analysis, spelling),
                "{profile}: {source}"
            );
            let mut missing = analysis.clone();
            missing.body_lexer_config = None;
            assert!(!declarations_at(source, &missing, spelling));
        }
        for (source, spelling) in [
            (r"puts {variable a\ b}", r"a\ b"),
            (
                r"proc oo::class args {}; oo::class create C {variable a\ b}",
                r"a\ b",
            ),
            ("oo::class create C {variable a${root}(index)}", "a${root}"),
            ("oo::class create C {variable arr($index)tail}", "arr("),
        ] {
            let analysis = Analyser::new().analyse(source, profile);
            assert!(
                !declarations_at(source, &analysis, spelling),
                "{profile}: {source}"
            );
        }
    }
}

#[test]
fn variable_argument_colours_require_actual_role_and_original_literal_root_geometry() {
    for profile in ["tcl", "tcl8.6"] {
        for (source, spelling) in [
            (r"set a\x28$index\x29 1", r"a\x28"),
            ("set arr($index) 1", "arr("),
            (r"set a\ b 1", r"a\ b"),
            ("set {a b} 1", "{a b}"),
            ("set \"a b\" 1", "\"a b\""),
        ] {
            let analysis = Analyser::new().analyse(source, profile);
            assert!(
                declarations_at(source, &analysis, spelling),
                "{profile}: {source}"
            );
            let mut missing = analysis.clone();
            missing.body_lexer_config = None;
            assert!(
                !declarations_at(source, &missing, spelling),
                "{profile}: {source}"
            );
        }
        for (source, spelling) in [
            ("puts arr($index)", "arr("),
            ("set a${root}($index) 1", "a${root}"),
            ("set arr($index)tail 1", "arr("),
            ("proc set args {}; set arr($index) 1", "arr("),
            (
                "interp alias {} assign {} set fixed; assign destination",
                "destination",
            ),
        ] {
            let analysis = Analyser::new().analyse(source, profile);
            assert!(
                !declarations_at(source, &analysis, spelling),
                "{profile}: {source}"
            );
        }
    }
}
