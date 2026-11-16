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
fn namespace_variable_roots_keep_qualification_before_zero_and_counted_simple_keys() {
    // Native proof: naming.tcloo.explicit-variable-link-counted-target
    // docs/design/analysis/name-resolution-proofs/explicit-variable-link-counted-target.md
    for version in tcl_dialect::TclVersion::ALL {
        let mut namespaces = Namespaces::new();
        namespaces.variable_name_protocol =
            Some(tcl_syntax::naming::NativeNameProtocol::C(version));
        let current = namespaces.ensure_namespace(GLOBAL, b"::N");
        let key = if version == tcl_dialect::TclVersion::V8_4 {
            b"k".as_slice()
        } else {
            b"k\0::Q".as_slice()
        };
        assert_eq!(
            namespaces.var_home(current, b"k\0::Q"),
            Some((current, key.to_vec()))
        );
        assert_eq!(
            namespaces.var_home(current, b"::N::k\0::Q"),
            Some((current, b"k".to_vec()))
        );
        assert_eq!(namespaces.var_home(current, b"missing::k"), None);
    }
}
