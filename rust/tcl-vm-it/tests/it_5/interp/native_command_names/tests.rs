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

//! `namespace origin` failures per release, against the tclsh captures.

use crate::prelude::*;
use tcl_dialect::TclVersion;

#[test]
fn native_namespace_origin_retains_string_result_birth_and_opaque_diagnostics() {
    // Native proof naming.namespace.origin-generated-opaque-failure-units:
    // docs/design/analysis/name-resolution-proofs/namespace-origin-generated-opaque-failure-units.md
    let source = include_bytes!(
        "../../../../../../runtime/rust/tests/data/native_namespace_origin_failures/source.tcl"
    );
    let rows = include_str!(
        "../../../../../../runtime/rust/tests/data/native_namespace_origin_failures/controls.tsv"
    );
    for (version, row) in TclVersion::ALL.into_iter().zip(rows.lines()) {
        let mut vm = crate::fixture::interpreter(
            tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap(),
        );
        // The retained observer measures binary-scan units after the catch,
        // independently of the result's native modified-UTF-8 string bytes.
        let mut observer = b"set observed [".to_vec();
        observer.extend_from_slice(source.trim_ascii());
        observer.extend_from_slice(b"]\nbinary scan $observed H* hex\nset hex");
        let completion = vm.try_eval_source_bytes(&observer).unwrap();
        assert_eq!(completion.code, Code::Ok, "{version:?}");
        let expected = tcl_test_support::fixture::tsv_split_once(row).unwrap().1;
        assert_eq!(
            crate::fixture::bytes(&vm, &completion.result).as_slice(),
            expected.as_bytes(),
            "{version:?}"
        );
    }
}
