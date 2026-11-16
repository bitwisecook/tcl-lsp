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
fn logical_class_navigation_uses_the_shared_namespace_target() {
    let source = concat!(
        "namespace eval left {oo::class create Widget {}}\n",
        "namespace eval right {oo::class create Widget {}}\n",
        "namespace eval left {Widget new}\n",
        "namespace eval right {Widget new}\n",
        "namespace eval other {Widget new}\n",
        "namespace eval left {::right::Widget new}\n",
    );
    let analysis = Analyser::new().analyse(source, "tcl");
    assert!(analysis.allows_lexical_declaration_advice());
    assert_eq!(analysis.all_classes.len(), 2);
    for (written, word, target) in [
        ("left {Widget new}", "Widget", Some("::left::Widget")),
        ("right {Widget new}", "Widget", Some("::right::Widget")),
        ("other {Widget new}", "Widget", None),
        (
            "left {::right::Widget new}",
            "::right::Widget",
            Some("::right::Widget"),
        ),
    ] {
        let cursor = source.find(written).unwrap() + written.find(word).unwrap();
        let selected = class_declaration_at(
            source,
            &analysis,
            CallResolution::document_only(),
            word,
            u32::try_from(cursor).unwrap(),
        );
        assert_eq!(
            selected,
            target.map(|name| analysis.all_classes[name].name_span),
            "{written}"
        );
    }
    let call = u32::try_from(source.find("left {Widget new}").unwrap() + 6).unwrap();
    assert!(
        class_declaration_at(
            &format!("{source} "),
            &analysis,
            CallResolution::document_only(),
            "Widget",
            call,
        )
        .is_none()
    );
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    assert!(
        class_declaration_at(
            source,
            &missing,
            CallResolution::document_only(),
            "Widget",
            call,
        )
        .is_none()
    );
}

#[test]
fn logical_class_navigation_uses_shared_imports_and_excludes_extension_stubs() {
    let source = concat!(
        "namespace eval exported {oo::class create Imported {}; namespace export Imported}\n",
        "namespace eval client {namespace import ::exported::*; Imported new}\n",
        "oo::define Remote {method extra {} {}}\n",
        "Remote new\n",
    );
    let analysis = Analyser::new().analyse(source, "tcl");
    let call = u32::try_from(source.find("Imported new").unwrap()).unwrap();
    assert_eq!(
        class_declaration_at(
            source,
            &analysis,
            CallResolution::document_only(),
            "Imported",
            call,
        ),
        Some(analysis.all_classes["::exported::Imported"].name_span)
    );
    let call = u32::try_from(source.find("Remote new").unwrap()).unwrap();
    assert!(
        class_declaration_at(
            source,
            &analysis,
            CallResolution::document_only(),
            "Remote",
            call,
        )
        .is_none()
    );
    let stub = analysis
        .all_classes
        .get("::Remote")
        .expect("actual source extension stub");
    assert!(stub.via_define);
    assert_eq!(
        class_declaration_at(
            source,
            &analysis,
            CallResolution::document_only(),
            "Remote",
            stub.name_span.start(),
        ),
        Some(stub.name_span)
    );
}
