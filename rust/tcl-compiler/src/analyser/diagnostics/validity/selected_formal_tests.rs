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

use crate::analyser::Analyser;
use tcl_core_types::DiagCode;

#[test]
fn original_formal_diagnostics_use_selected_name_grammar_and_whole_words() {
    // naming.variable.original-readonly-formal-topology
    // docs/design/analysis/name-resolution-proofs/original-readonly-formal-topology.md
    // Source consumer contract; these assertions do not enter a native frame.
    for name in ["a::b", "a(1)"] {
        let source = format!("proc target {{{name}}} {{}}");
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let analysis = Analyser::new().analyse(&source, dialect);
            let diagnostic = analysis
                .diagnostics
                .iter()
                .find(|d| d.code == DiagCode::E006)
                .expect(dialect);
            assert_eq!(
                &source[diagnostic.span.start() as usize..diagnostic.span.end() as usize],
                format!("{{{name}}}")
            );
        }
        let jim = Analyser::new().analyse(&source, "jimtcl");
        assert!(
            !jim.diagnostics.iter().any(|d| d.code == DiagCode::E006),
            "Jim source name grammar: {name}: {:?}",
            jim.diagnostics
        );
    }
}

#[test]
fn original_formal_diagnostics_reject_three_fields_and_withdraw_known_shadows() {
    // naming.variable.original-readonly-formal-topology
    // docs/design/analysis/name-resolution-proofs/original-readonly-formal-topology.md
    let malformed = "proc target {{a b c}} {}";
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let analysis = Analyser::new().analyse(malformed, dialect);
        assert!(
            analysis
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::E006),
            "selected malformed formal: {dialect}"
        );
    }
    for source in [
        "rename proc define; define target {{a b c}} {}",
        "interp alias {} define {} proc target; define {{a b c}} {}",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl9.0");
        assert!(
            analysis
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::E006),
            "effective original formal role: {source}"
        );
    }
    let source = "proc proc args {}
proc target {{a b c}} {}";
    let analysis = Analyser::new().analyse(source, "tcl9.0");
    assert!(
        !analysis
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::E006),
        "a known replacement has no original formal-list role: {:?}",
        analysis.diagnostics
    );
}

#[test]
fn a_jim_scalar_formal_beside_its_own_element_formal_raises_at_the_call() {
    // Jim binds the scalar first. Its exact value determines whether the
    // indexed setter can treat it as a dictionary. An empty body skips formal
    // installation after checking the argument count.
    for (body, actual, reaches_read) in [
        ("return $p", "1 2", false),
        ("return $p", "{} 2", true),
        ("return $p", "{a b} 2", true),
        ("", "1 2", true),
        (" ", "1 2", false),
        ("", "", false),
    ] {
        let source = format!("proc h {{p p(key)}} {{{body}}}\nh {actual}\nputs $never\n");
        let result = Analyser::new().analyse(&source, "jimtcl");
        let warns = result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W210 && d.message.contains("'never'"));
        assert_eq!(warns, reaches_read, "{source}");
    }
}
