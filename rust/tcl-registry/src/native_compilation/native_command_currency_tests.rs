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
fn original_command_currency_uses_actual_engine_and_independent_epochs() {
    // naming.command.native-command-entry-currency
    // docs/design/analysis/name-resolution-proofs/command-native-command-entry-currency.md
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let dialect = tcl_native_model::InvocationDialect::of_profile(
            tcl_dialect::DialectProfile::find(profile).unwrap(),
        );
        let guard = NativeCompilationGuard::for_native_dialect(dialect).unwrap();
        assert!(!guard.revalidate_source((4, 7), (4, 7)));
        let command_boundary = profile != "tcl8.4";
        assert_eq!(guard.requires_current_epochs(), command_boundary);
        assert_eq!(guard.revalidate_source((4, 7), (5, 7)), command_boundary);
        assert_eq!(guard.revalidate_source((4, 7), (4, 8)), command_boundary);
    }
    for environment in ["tcl", "jim", "f5-irules", "f5-bigip"] {
        let profile = crate::model::resolve_environment(environment).unit_profile();
        let dialect = tcl_native_model::InvocationDialect::of_profile(profile);
        assert!(
            NativeCompilationGuard::for_native_dialect(dialect).is_none(),
            "{environment}"
        );
    }
}
