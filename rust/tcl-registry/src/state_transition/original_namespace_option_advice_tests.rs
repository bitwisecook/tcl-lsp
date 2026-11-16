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

use super::NamespaceTransition;
use tcl_syntax::naming::NamePolicyProtocol;

#[test]
fn original_namespace_option_advice_selects_only_first_native_control_extent() {
    // Implementation contract: naming.namespace.original-export-source-advice
    // docs/design/analysis/name-resolution-proofs/namespace-original-export-source-advice.md
    for version in tcl_dialect::TclVersion::ALL {
        let policy = NamePolicyProtocol::authored_tcl(version);
        let options = [b"-force\0suffix".as_slice(), b"-force".as_slice()];
        let (forced, patterns) =
            NamespaceTransition::import_pattern_byte_operands(&options, policy).unwrap();
        assert!(forced);
        assert_eq!(patterns, &options[1..]);
        let escaped_zero = [b"-force\xc0\x80suffix".as_slice()];
        assert!(
            !NamespaceTransition::import_pattern_byte_operands(&escaped_zero, policy)
                .unwrap()
                .0
        );
        let clear = [b"-clear".as_slice(), b"-clear".as_slice()];
        let (clears, patterns) =
            NamespaceTransition::export_pattern_byte_operands(&clear, policy).unwrap();
        assert!(clears);
        assert_eq!(patterns, &clear[1..]);
    }
}
