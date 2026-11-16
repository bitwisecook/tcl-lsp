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
fn original_own_object_identity_keeps_uri_currency_and_wire_reselection() {
    // Implementation contract: naming.editor.original-own-object-member-navigation
    // docs/design/analysis/name-resolution-proofs/original-own-object-member-navigation.md
    let source = r"oo::class create C {}; C create object; oo::objdefine object method p\uD800 {} {}; object p\uD800";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    analysis.all_classes.clear();
    let configuration = analysis.original_object_configurations().next().unwrap();
    let method = configuration.members().declarations().next().unwrap();
    let a = OriginalDeclarationIdentity::for_object_method(
        "file:///a.tcl",
        source,
        &analysis,
        configuration.target(),
        method,
    )
    .unwrap();
    let b = OriginalDeclarationIdentity::for_object_method(
        "file:///b.tcl",
        source,
        &analysis,
        configuration.target(),
        method,
    )
    .unwrap();
    assert_ne!(a, b);
    assert!(a.class_metadata(&analysis).is_none());
    assert_eq!(a.method_metadata(&analysis), Some(method));
    assert!(
        declarations("file:///a.tcl", source, &analysis)
            .unwrap()
            .contains(&a)
    );
    let mut registry = OriginalDeclarationRegistry::default();
    let handle = registry.issue(a.clone()).unwrap();
    assert_eq!(
        registry.resolve(handle, "file:///a.tcl", source, &analysis),
        Some(a.clone())
    );
    assert!(
        registry
            .resolve(handle, "file:///b.tcl", source, &analysis)
            .is_none()
    );
    assert!(
        registry
            .resolve(handle, "file:///a.tcl", &format!("{source} "), &analysis)
            .is_none()
    );
    assert_eq!(reference_spans(&a, source, &analysis, true).len(), 1);
    let lenses = crate::code_lens::code_lenses(
        source,
        crate::profile_for_analysis(&analysis),
        Some(&analysis),
        None,
        "file:///a.tcl",
    );
    assert!(lenses.iter().any(|lens| lens.identity.as_ref() == Some(&a)));
}
