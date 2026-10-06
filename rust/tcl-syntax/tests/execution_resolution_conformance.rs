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

//! Executable binding/package/autoload vectors against independent references.

use std::collections::HashSet;

use tcl_syntax::execution_conformance::{ExecutionDomain, UNSUPPORTED, filesystem_cases, vectors};
use tcl_test_support::{
    JimCapability, available_tclshs, locate_jimsh, require_jimsh, run_script, run_script_fixture,
};

#[test]
fn shared_execution_vector_definitions_are_complete() {
    let mut identifiers = HashSet::new();
    for domain in ExecutionDomain::ALL {
        let cases = vectors(domain);
        assert!(!cases.is_empty(), "empty {} domain", domain.name());
        for case in cases {
            assert!(identifiers.insert(case.id.clone()), "duplicate {}", case.id);
            for capability in &case.capabilities {
                assert!(
                    JimCapability::named(capability).is_some(),
                    "{}: unknown capability {}",
                    case.id,
                    capability
                );
            }
            assert!(
                case.jim_want != UNSUPPORTED || !case.capabilities.is_empty(),
                "{}: unsupported scenario needs a capability declaration",
                case.id
            );
        }
    }
}

#[test]
fn c_tcl_matches_shared_execution_vectors() {
    let references = available_tclshs();
    assert!(
        !references.is_empty(),
        "no C Tcl oracle: configure TCL_LSP_TCLSH84 .. TCL_LSP_TCLSH91"
    );
    for reference in references {
        for case in filesystem_cases() {
            let outcome = run_script_fixture(&reference.path, &case.script(), case.files)
                .expect("C package fixture")
                .strict_text()
                .expect("clean C package fixture");
            assert_eq!(
                outcome, case.c_want,
                "{} on {}",
                case.name, reference.patchlevel
            );
        }
        for domain in ExecutionDomain::ALL {
            for case in vectors(domain) {
                let outcome = run_script(&reference.path, case.script().as_bytes())
                    .expect("execute C Tcl vector")
                    .strict_text()
                    .expect("clean C Tcl outcome");
                let want = case.wants.get(reference.version);
                if want == UNSUPPORTED {
                    assert!(
                        outcome.starts_with("1 "),
                        "{} on {}: expected unsupported operation, got {outcome:?}",
                        case.id,
                        reference.patchlevel
                    );
                } else {
                    assert_eq!(outcome, want, "{} on {}", case.id, reference.patchlevel);
                }
            }
        }
    }
}

#[test]
fn jim_matches_shared_execution_vectors_with_measured_capabilities() {
    let reference = if std::env::var_os("TCL_LSP_REQUIRE_JIM_ORACLE").is_some() {
        Some(require_jimsh().expect("required Jim oracle"))
    } else {
        locate_jimsh().expect("valid Jim oracle override")
    };
    let Some(reference) = reference else {
        eprintln!("Jim oracle not requested and unavailable; set TCL_LSP_JIMSH");
        return;
    };
    for case in filesystem_cases() {
        let outcome = run_script_fixture(&reference.path, &case.script(), case.files)
            .expect("Jim package fixture")
            .strict_text()
            .expect("clean Jim package fixture");
        assert_eq!(
            outcome, case.jim_want,
            "{} on Jim {}",
            case.name, reference.patchlevel
        );
    }
    for domain in ExecutionDomain::ALL {
        for case in vectors(domain) {
            let supported = case.capabilities.iter().all(|name| {
                reference.supports(JimCapability::named(name).expect("known capability"))
            });
            assert_eq!(
                supported,
                case.jim_want != UNSUPPORTED,
                "{}: Jim {} capability surface changed; remeasure the row",
                case.id,
                reference.patchlevel
            );
            let outcome = run_script(&reference.path, case.script().as_bytes())
                .expect("execute Jim vector")
                .strict_text()
                .expect("clean Jim outcome");
            if supported {
                assert_eq!(
                    outcome, case.jim_want,
                    "{} on Jim {}",
                    case.id, reference.patchlevel
                );
            } else {
                assert!(
                    outcome.starts_with("1 "),
                    "{}: unsupported Jim operation unexpectedly succeeded: {outcome:?}",
                    case.id
                );
            }
        }
    }
}
