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
fn source_values_and_resident_byte_controls_retain_their_actual_extents() {
    let policy = NamePolicyProtocol::authored_tcl(TclVersion::V8_6);
    let source = policy.namespace_import_source_control("-force").unwrap();
    assert!(source.forces());
    assert_eq!(source.policy(), policy);
    assert_eq!(source.source_value(), "-force");
    for value in ["-force\0tail", "-forceé", "-for", "", "::name"] {
        let source = policy.namespace_import_source_control(value).unwrap();
        assert_eq!(source.source_value(), value);
        assert!(!source.forces(), "{value:?}");
    }
    assert!(
        NativeNameProtocol::C(TclVersion::V8_6)
            .namespace_import_forces(b"-force\0tail")
            .unwrap()
    );
    assert!(
        NamePolicyProtocol::authored_jim084()
            .namespace_import_source_control("-force")
            .is_err()
    );
    assert!(
        NativeNameProtocol::Jim084
            .namespace_import_forces(b"-force")
            .is_err()
    );
}
