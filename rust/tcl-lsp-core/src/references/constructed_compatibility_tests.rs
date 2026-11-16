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
use tcl_lexer::Span;

#[test]
fn constructed_reference_projection_keeps_colon_components_and_known_refusal() {
    // naming.analysis.constructed-key-consumer-projections
    // docs/design/analysis/name-resolution-proofs/constructed-key-consumer-projections.md
    // Explicit written metadata carries no positioned declaration receipt.
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::signature_scan::types::SignatureCommandInvocation;
    for (qualified, written, tail) in [
        (":::茶::méthode", ":茶::méthode", "méthode"),
        ("::茶::méthode", "茶::méthode", "méthode"),
        ("::茶:::", "茶:::", ":"),
    ] {
        let analysis = Analyser::new().analyse(written, "tcl");
        let span = Span::new(0, u32::try_from(written.len()).unwrap());
        let invocation = SignatureCommandInvocation::written(written.to_owned(), span, Some(0));
        assert!(matches!(
            retained_definition(&invocation, true, qualified),
            RetainedDefinition::Absent
        ));
        assert!(invocation_references_named(
            &analysis,
            &invocation,
            qualified,
            tail,
            qualified,
            written
        ));
    }
    let source = "茶::méthode";
    let analysis = Analyser::new().analyse(source, "tcl");
    let invocation = SignatureCommandInvocation::written(
        source.to_owned(),
        Span::new(0, u32::try_from(source.len()).unwrap()),
        Some(0),
    );
    assert!(!invocation_references_named(
        &analysis,
        &invocation,
        ":::茶::méthode",
        "méthode",
        ":::茶::méthode",
        source
    ));

    // The actual positioned alias to a builtin terminates before metadata fallback.
    let source =
        "proc :茶 {} {return ORIGINAL}\nrename :茶 kept\ninterp alias {} :茶 {} list\n:茶 VALUE\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let offset = u32::try_from(source.rfind(":茶 VALUE").unwrap()).unwrap();
    let actual = analysis
        .command_invocations
        .iter()
        .find(|invocation| {
            invocation.lookup.is_execution_site() && invocation.range.start() == offset
        })
        .unwrap();
    assert!(matches!(
        retained_definition(actual, true, ":::茶"),
        RetainedDefinition::Known(None)
    ));
    assert!(!invocation_references_named(
        &analysis, actual, ":::茶", ":茶", ":::茶", source
    ));
}
