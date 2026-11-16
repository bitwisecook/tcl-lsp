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

use super::OriginalDeclarationIdentity;
use tcl_compiler::analyser::Analyser;

#[test]
fn property_identity_refuses_same_source_foreign_producer() {
    let source = "oo::configurable create C {property size}";
    let current = Analyser::new().analyse(source, "tcl9.0");
    let class = current.original_class_declarations().next().unwrap();
    let property = class
        .metadata()
        .original_properties
        .declarations()
        .next()
        .unwrap();
    assert!(
        OriginalDeclarationIdentity::for_property("", source, &current, class, property).is_some()
    );
    let mut foreign = current.clone();
    foreign.original_class_metadata = Analyser::new()
        .analyse(source, "tcl9.1")
        .original_class_metadata;
    let class = foreign.original_class_declarations().next().unwrap();
    let property = class
        .metadata()
        .original_properties
        .declarations()
        .next()
        .unwrap();
    assert!(
        OriginalDeclarationIdentity::for_property("", source, &foreign, class, property).is_none()
    );
}
