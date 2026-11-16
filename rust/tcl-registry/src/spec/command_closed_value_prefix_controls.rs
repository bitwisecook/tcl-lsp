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
fn qualified_dictionary_filter_retains_original_value_prefix_policy() {
    let registry = crate::CommandRegistry::build_default();
    let filter = registry.get("::tcl::dict::filter").unwrap();
    assert!(filter.arg_values_accept_prefix);
    assert_eq!(filter.closed_value_args, &[1]);
    assert!(
        filter
            .arg_values_at(1)
            .iter()
            .any(|value| value.value == "script")
    );
    const { assert!(!CommandSpec::DEFAULT.arg_values_accept_prefix) };
}
