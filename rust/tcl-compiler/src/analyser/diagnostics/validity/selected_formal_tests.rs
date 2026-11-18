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
    // jimsh 0.84 accepts `proc h {p p(key)} {...}`, but `h 1 2` raises
    // `can't set "p(key)": variable isn't array` (p is bound as a scalar
    // first), so a read after the call is unreached. With a plain formal list
    // the call returns and the later read of an unset name warns.
    let raising = "proc h {p p(key)} {return $p}\nh 1 2\nputs $never\n";
    let returning = "proc h {p} {return $p}\nh 1\nputs $never\n";
    let w210 = |source: &str| {
        Analyser::new()
            .analyse(source, "jimtcl")
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W210 && d.message.contains("'never'"))
    };
    assert!(!w210(raising), "the read after a raising call is unreached");
    assert!(w210(returning), "the read after a returning call warns");
}
