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

#[path = "../common/mod.rs"]
mod common;

use tcl_dialect::TclVersion;

const CATCH_RESULT: &str = include_str!(
    "../../../tcl-test-support/tests/data/name_lookup_issues/2209-catch-array-result.tcl"
);
const CATCH_OPTIONS: &str = include_str!(
    "../../../tcl-test-support/tests/data/name_lookup_issues/2209-catch-array-options.tcl"
);
const INCR_AMOUNT: &str = include_str!(
    "../../../tcl-test-support/tests/data/name_lookup_issues/2235-braced-incr-amount.tcl"
);
const INDEX_MASK: &str = include_str!(
    "../../../tcl-test-support/tests/data/name_lookup_issues/2255-array-index-outer-mask.tcl"
);

const BRACED_INFO_EXISTS: &str = include_str!(
    "../../../tcl-test-support/tests/data/name_lookup_issues/2210-braced-info-exists-name.tcl"
);
const NESTED_ARM_VALUE: &str = include_str!(
    "../../../tcl-test-support/tests/data/name_lookup_issues/2337-nested-structured-arm-value.tcl"
);
const VARIABLE_ERROR_CODES: &str = include_str!(
    "../../../tcl-test-support/tests/data/name_lookup_issues/2342-variable-error-codes.tcl"
);
const QUALIFIED_ALIAS_NAMESPACE: &str = include_str!(
    "../../../tcl-test-support/tests/data/name_lookup_issues/2269-qualified-alias-namespace.tcl"
);
const QUALIFIED_ALIAS_DISPATCH: &str = include_str!(
    "../../../tcl-test-support/tests/data/name_lookup_issues/2193-qualified-alias-dispatch.tcl"
);
const NESTED_INTERP_CREATE: &str = include_str!(
    "../../../tcl-test-support/tests/data/name_lookup_issues/2276-nested-interp-create.tcl"
);
const CHILD_ALIAS_SUBCOMMANDS: &str = include_str!(
    "../../../tcl-test-support/tests/data/name_lookup_issues/2275-child-alias-subcommands.tcl"
);
const NESTED_INTERP_QUERIES: &str = include_str!(
    "../../../tcl-test-support/tests/data/name_lookup_issues/2273-nested-interp-queries.tcl"
);

const CATCH_FOREACH_CONTEXT: &str = include_str!(
    "../../../tcl-test-support/tests/data/name_lookup_issues/2274-catch-foreach-error-context.tcl"
);
const IFNEEDED_FOREACH_CONTEXT: &str = include_str!(
    "../../../tcl-test-support/tests/data/name_lookup_issues/2239-ifneeded-foreach-error-context.tcl"
);

const RELEASE_NUMERIC_ROWS: &str = include_str!(
    "../../../tcl-test-support/tests/data/name_lookup_issues/2333-release-numeric-rows.tcl"
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

#[test]
fn info_exists_reads_a_braced_name_as_written() {
    matches_original(BRACED_INFO_EXISTS, "1", &TclVersion::ALL);
}

#[test]
fn a_structured_command_ending_an_arm_is_the_arm_value() {
    matches_original(NESTED_ARM_VALUE, "<5>\n<5>\n<5>", &TclVersion::ALL);
}

#[test]
fn variable_errors_carry_the_original_error_codes() {
    matches_original(
        VARIABLE_ERROR_CODES,
        "TCL LOOKUP VARNAME nosuch\nTCL LOOKUP VARNAME nosuch2\nTCL READ VARNAME",
        &[TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1],
    );
}

#[test]
fn a_qualified_alias_creates_its_namespace_and_describes_its_target() {
    matches_original(QUALIFIED_ALIAS_NAMESPACE, "1\nb\nlindex", &TclVersion::ALL);
}

#[test]
fn a_qualified_alias_under_a_new_namespace_dispatches() {
    matches_original(
        QUALIFIED_ALIAS_DISPATCH,
        "1 ::fresh::alias 0 value",
        &TclVersion::ALL,
    );
}

#[test]
fn a_nested_interp_path_creates_the_child_under_its_parent() {
    matches_original(
        NESTED_INTERP_CREATE,
        "a x2\nx2\n1",
        &[TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1],
    );
}

#[test]
fn a_child_interp_command_has_alias_and_aliases() {
    matches_original(CHILD_ALIAS_SUBCOMMANDS, "foo\nfoo\nx y", &TclVersion::ALL);
}

#[test]
fn interp_exists_and_issafe_resolve_nested_paths() {
    matches_original(NESTED_INTERP_QUERIES, "1\n0\n1", &TclVersion::ALL);
}

#[test]
fn a_caught_foreach_error_keeps_its_body_context_by_frame() {
    matches_original(
        CATCH_FOREACH_CONTEXT,
        "{EI\n    (\"foreach\" body line 1)\n    invoked from within\n\"foreach x 1 {error direct EI}\"}\nEI",
        &TclVersion::ALL,
    );
}

#[test]
fn an_ifneeded_foreach_error_keeps_its_body_and_package_frames() {
    matches_original(
        IFNEEDED_FOREACH_CONTEXT,
        "1 {ifneeded test} {EI\n    (\"foreach\" body line 1)\n    invoked from within\n\"foreach x 1 {error \"ifneeded test\" EI}\"\n    (\"package ifneeded t 2.1\" script)\n    invoked from within\n\"package require t 2.1\"}",
        &[
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ],
    );
}

#[test]
fn a_tcl84_ifneeded_error_names_only_the_subcommand() {
    matches_original(
        IFNEEDED_FOREACH_CONTEXT,
        "1 {ifneeded test} {EI\n    (\"foreach\" body line 1)\n    invoked from within\n\"foreach x 1 {error \"ifneeded test\" EI}\"\n    (\"package ifneeded\" script)\n    invoked from within\n\"package require t 2.1\"}",
        &[TclVersion::V8_4],
    );
}

#[test]
fn integer_class_and_math_results_follow_the_pinned_release() {
    let wrapped = "int(1e20) 0 7766279631452241920\nwide(-9.3e18) 0 9146744073709551616\nround(1e20) 0 100000000000000000000\nint(9.2e18) 0 9200000000000000000\nexp(710) 0 Inf\nlog(0) 0 -Inf";
    matches_original(
        RELEASE_NUMERIC_ROWS,
        "0 1 1 0 1\nint(1e20) 1 integer value too large to represent\nwide(-9.3e18) 1 integer value too large to represent\nround(1e20) 1 integer value too large to represent\nint(9.2e18) 0 9200000000000000000\nexp(710) 1 floating-point value too large to represent\nlog(0) 1 floating-point value too large to represent",
        &[TclVersion::V8_4],
    );
    matches_original(
        RELEASE_NUMERIC_ROWS,
        &format!("0 1 1 0 1\n{wrapped}"),
        &[TclVersion::V8_5],
    );
    matches_original(
        RELEASE_NUMERIC_ROWS,
        &format!("0 1 1 0 0\n{wrapped}"),
        &[TclVersion::V8_6],
    );
    matches_original(
        RELEASE_NUMERIC_ROWS,
        "1 1 1 1 1\nint(1e20) 0 100000000000000000000\nwide(-9.3e18) 0 9146744073709551616\nround(1e20) 0 100000000000000000000\nint(9.2e18) 0 9200000000000000000\nexp(710) 0 Inf\nlog(0) 0 -Inf",
        &[TclVersion::V9_0, TclVersion::V9_1],
    );
}
