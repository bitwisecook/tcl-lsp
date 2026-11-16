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
use tcl_compiler::analyser::Analyser;

#[test]
fn original_package_completion_uses_retained_context_and_ignores_reported_names() {
    // Implementation contract: naming.core.original-package-completion-advice
    // docs/design/analysis/name-resolution-proofs/original-package-completion-advice.md
    let source = "package require Tk\nbutt\n";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    assert!(analysis.package_requires[0].original_name.is_some());
    analysis.package_requires[0].name = "Other".to_owned();
    let wrong_profile = tcl_dialect::DialectProfile::find("f5-irules").unwrap();
    let items = completions(
        source,
        1,
        4,
        &analysis,
        analysis.resolved_registry(),
        None,
        wrong_profile,
    );
    assert!(items.iter().any(|item| item.label == "button"));
    assert_eq!(
        completions(
            &format!("{source} "),
            1,
            4,
            &analysis,
            analysis.resolved_registry(),
            None,
            wrong_profile
        ),
        [] as [crate::completion::CompletionItem; 0]
    );
    let plain_source = "butt\n";
    let mut plain = Analyser::new().analyse(plain_source, "tcl8.6");
    let mut reported = analysis.package_requires[0].clone();
    reported.original_name = None;
    reported.original_requirements.clear();
    reported.name = "Tk".to_owned();
    plain.package_requires.push(reported);
    let items = completions(
        plain_source,
        0,
        4,
        &plain,
        plain.resolved_registry(),
        None,
        tcl_dialect::DialectProfile::find("tcl9.1").unwrap(),
    );
    assert!(!items.iter().any(|item| item.label == "button"));
}
