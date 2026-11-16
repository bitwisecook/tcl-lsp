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
fn semantic_subject_rows_do_not_infer_interpreter_identity_from_other_packages() {
    for target in [
        TclVersion::V8_4,
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        let rows = target.core_provided_packages();
        assert!(
            rows.iter()
                .any(|row| row.role == CorePackageRole::InterpreterRelease)
        );
        for row in rows {
            assert_eq!(row.role == CorePackageRole::ObjectSystem, row.ifneeded_stub);
        }
    }
    assert!(
        TclVersion::V9_0
            .core_provided_packages()
            .iter()
            .filter(|row| row.role == CorePackageRole::InterpreterRelease)
            .count()
            > 1
    );
}
