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

use super::function_namespace;

#[test]
fn constructor_advice_reporting_namespace_preserves_literal_prefixes() {
    // Implementation contract: naming.analysis.reported-qname-namespace-prefix
    // docs/design/analysis/name-resolution-proofs/reported-qname-namespace-prefix.md
    // Shared analytical name contract:
    // docs/design/contracts/shared-utility-contracts-rust.md (names / namespaces).
    // Actual consumer purpose selection; reporting text grants no lookup identity.
    for (qname, expected) in [
        ("n::p", "n"),
        (":::p", ":"),
        ("::n::", "::n"),
        ("é::λ", "é"),
    ] {
        assert_eq!(function_namespace(qname), expected);
    }
}
