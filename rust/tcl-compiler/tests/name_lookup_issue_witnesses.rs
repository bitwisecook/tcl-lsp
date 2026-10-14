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

//! Naming consumers agree on original words, cells and executable bodies.

use tcl_compiler::analyser::Analyser;
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::compiler_checks::run_all_checks;
use tcl_compiler::optimiser::manager::optimise_source_multipass;
use tcl_core_types::DiagCode;
use tcl_dialect::TclVersion;
use tcl_registry::model::ingress::{resolve_environment, static_context_for};

const ROOT_BODY: &str = include_str!(
    "../../tcl-test-support/tests/data/name_lookup_issues/2417-root-body-collision.tcl"
);
const ARRAY_KEY: &str = include_str!(
    "../../tcl-test-support/tests/data/name_lookup_issues/2296-computed-array-key-read.tcl"
);
const ARRAY_EXISTS: &str = include_str!(
    "../../tcl-test-support/tests/data/name_lookup_issues/2268-unknown-and-distinct-array-key.tcl"
);
const CREATED_CHILD: &str = include_str!(
    "../../tcl-test-support/tests/data/name_lookup_issues/2289-child-created-in-procedure.tcl"
);
const DYNAMIC_BINDING: &str = include_str!(
    "../../tcl-test-support/tests/data/name_lookup_issues/2218-dynamic-proc-head-rebinding.tcl"
);
const NAME_EFFECTS: &str = include_str!(
    "../../tcl-test-support/tests/data/name_lookup_issues/2339-assignment-name-evaluation.tcl"
);
const GLOBAL_STORE: &str = include_str!(
    "../../tcl-test-support/tests/data/name_lookup_issues/2399-global-read-store-before-unset.tcl"
);
const FACTORY: &str = include_str!(
    "../../tcl-test-support/tests/data/name_lookup_issues/2143-materialised-factory.tcl"
);

fn diagnostics(source: &str, dialect: &str) -> Vec<(DiagCode, String)> {
    let mut result: Vec<_> = Analyser::new()
        .analyse(source, dialect)
        .diagnostics
        .into_iter()
        .map(|diagnostic| (diagnostic.code, diagnostic.message))
        .collect();
    let registry = static_context_for(dialect).commands();
    let unit = CompilationUnit::build_for_dialect(source, registry, false, dialect);
    result.extend(
        run_all_checks(
            &unit,
            registry,
            Some(resolve_environment(dialect).analyser_profile()),
        )
        .into_iter()
        .map(|diagnostic| (diagnostic.code, diagnostic.message)),
    );
    result
}

fn original_and_rewritten_print(source: &str, expected: &str) {
    for oracle in
        tcl_test_support::required_tclshs(&TclVersion::ALL).expect("original C Tcl matrix")
    {
        let dialect = oracle.version.dialect_name();
        let registry = static_context_for(dialect).commands();
        let (rewritten, _) = optimise_source_multipass(
            source,
            registry,
            Some(resolve_environment(dialect).analyser_profile()),
            8,
        );
        for (label, candidate) in [("original", source), ("rewritten", rewritten.as_str())] {
            let output = tcl_test_support::run_script(&oracle.path, candidate.as_bytes())
                .expect("original interpreter process")
                .strict_text()
                .expect("fixed program completes");
            assert_eq!(output, expected, "{dialect}: {label}: {candidate}");
        }
    }
}

#[test]
fn root_and_a_procedure_named_top_have_independent_trace_versions() {
    for version in TclVersion::ALL {
        let dialect = version.dialect_name();
        let registry = static_context_for(dialect).commands();
        let unit = CompilationUnit::build_for_dialect(ROOT_BODY, registry, false, dialect);
        assert_eq!(unit.procedures.len(), 1);
        let _ = diagnostics(ROOT_BODY, dialect);
    }
    original_and_rewritten_print(ROOT_BODY, "1 top");
}

#[test]
fn a_computed_array_key_reads_its_variable() {
    for version in TclVersion::ALL {
        let reported = diagnostics(ARRAY_KEY, version.dialect_name());
        assert!(
            !reported
                .iter()
                .any(|(code, message)| *code == DiagCode::W220 && message.contains("'k'")),
            "{}: {reported:?}",
            version.dialect_name()
        );
    }
    original_and_rewritten_print(ARRAY_KEY, "1");
}

#[test]
fn an_unknown_array_key_observes_possible_elements() {
    for version in TclVersion::ALL {
        let dialect = version.dialect_name();
        let reported = diagnostics(ARRAY_EXISTS, dialect);
        assert!(
            !reported
                .iter()
                .any(|(code, message)| *code == DiagCode::W211 && message.contains("'a(x)'")),
            "{dialect}: {reported:?}"
        );
        let distinct = diagnostics(
            "proc p {} {set a(x) 1; if {[info exists a(y)]} {puts yes}}\np\n",
            dialect,
        );
        assert!(
            distinct
                .iter()
                .any(|(code, message)| *code == DiagCode::W211 && message.contains("'a(x)'")),
            "{dialect}: distinct literal key: {distinct:?}"
        );
    }
    original_and_rewritten_print(ARRAY_EXISTS, "yes\nno");
}

#[test]
fn a_called_creator_publishes_its_original_child_interpreter() {
    let created = CREATED_CHILD
        .split("interp delete vectorCoreChild\nputs")
        .next()
        .expect("creation prefix");
    for version in TclVersion::ALL {
        let dialect = version.dialect_name();
        let reported = diagnostics(created, dialect);
        assert!(
            !reported.iter().any(|(code, _)| *code == DiagCode::W140),
            "{dialect}: {reported:?}"
        );
        let missing = diagnostics("interp eval vectorCoreChild {set child}\n", dialect);
        assert!(
            missing.iter().any(|(code, _)| *code == DiagCode::W140),
            "{dialect}: {missing:?}"
        );
    }
    original_and_rewritten_print(CREATED_CHILD, "VALUE\n1");
}

#[test]
fn dynamic_procedure_heads_revoke_expression_binding_trust() {
    original_and_rewritten_print(DYNAMIC_BINDING, "[expr {$x * 2}]");
}

#[test]
fn assignment_name_components_keep_their_original_writes() {
    original_and_rewritten_print(NAME_EFFECTS, "nonzero");
}

#[test]
fn a_later_unset_does_not_kill_stores_read_by_a_callee() {
    original_and_rewritten_print(GLOBAL_STORE, "<6>\n<123>");
}

#[test]
fn materialised_factory_commands_are_visible_to_unknown_command_checks() {
    for version in TclVersion::ALL {
        let reported = diagnostics(FACTORY, version.dialect_name());
        assert!(
            !reported
                .iter()
                .any(|(code, message)| *code == DiagCode::W123 && message.contains("port")),
            "{}: {reported:?}",
            version.dialect_name()
        );
    }
    original_and_rewritten_print(FACTORY, "8080");
}
