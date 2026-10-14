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

//! Variable destinations and component masks preserve original word semantics.

mod common;

use tcl_dialect::TclVersion;

const CATCH_RESULT: &str = include_str!(
    "../../tcl-test-support/tests/data/name_lookup_issues/2209-catch-array-result.tcl"
);
const CATCH_OPTIONS: &str = include_str!(
    "../../tcl-test-support/tests/data/name_lookup_issues/2209-catch-array-options.tcl"
);
const INCR_AMOUNT: &str = include_str!(
    "../../tcl-test-support/tests/data/name_lookup_issues/2235-braced-incr-amount.tcl"
);
const INDEX_MASK: &str = include_str!(
    "../../tcl-test-support/tests/data/name_lookup_issues/2255-array-index-outer-mask.tcl"
);

fn matches_original(source: &str, expected: &str, versions: &[TclVersion]) {
    for oracle in
        tcl_test_support::required_tclshs(versions).expect("matching original C interpreters")
    {
        assert_eq!(
            common::oracle_output(&oracle.path, source),
            expected,
            "{}: original",
            oracle.patchlevel
        );
        assert_eq!(
            common::vm_output(source, oracle.version.dialect_name()),
            expected,
            "{}: compiled",
            oracle.patchlevel
        );
    }
}

#[test]
fn catch_result_variables_use_scalar_array_and_namespace_cell_resolution() {
    matches_original(CATCH_RESULT, "1 1 ok\nlocal\nqualified", &TclVersion::ALL);
}

#[test]
fn catch_options_can_name_an_array_element() {
    matches_original(
        CATCH_OPTIONS,
        "ok 1 0",
        &[
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ],
    );
}

#[test]
fn a_braced_increment_amount_is_not_a_variable_reference() {
    matches_original(
        INCR_AMOUNT,
        "1\nexpected integer but got \"$n\"",
        &TclVersion::ALL,
    );
}

#[test]
fn array_index_substitutions_are_independent_of_the_outer_mask() {
    matches_original(INDEX_MASK, "V\nV\nV", &TclVersion::ALL);
}
