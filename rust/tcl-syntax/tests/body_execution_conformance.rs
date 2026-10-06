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

//! Package-versioned captured-body and completion observations against actual
//! C Tcl and Jim interpreters. No compiler result supplies an expectation.

use tcl_dialect::TclVersion;
use tcl_test_support::{
    JimCapability, available_tclshs, locate_jimsh, reference_patchlevel, require_jimsh,
    run_script_fixture,
};

macro_rules! fixture {
    ($name:literal) => {
        include_str!(concat!("data/body_execution/", $name))
    };
}

/// Complete release columns distinguish a missing feature from an untested row.
/// Jim expectations belong to its independent implementation and capability set.
struct CaseSpec {
    name: &'static str,
    source: &'static str,
    c_wants: [Option<&'static str>; 5],
    jim_want: Option<&'static str>,
    jim_capabilities: &'static [(JimCapability, bool)],
}

impl CaseSpec {
    fn c_want(&self, version: TclVersion) -> Option<&'static str> {
        self.c_wants[TclVersion::ALL
            .iter()
            .position(|candidate| *candidate == version)
            .expect("reference belongs to shared release ladder")]
    }
}

const CASES: &[CaseSpec] = &[
    CaseSpec {
        name: "native-dictionary-compilation",
        source: fixture!("native-dictionary-compilation.tcl"),
        c_wants: [
            Some(fixture!("native-dictionary-compilation-8.4.txt")),
            Some(fixture!("native-dictionary-compilation-8.5.txt")),
            Some(fixture!("native-dictionary-compilation-8.6.txt")),
            Some(fixture!("native-dictionary-compilation-9.0.txt")),
            Some(fixture!("native-dictionary-compilation-9.1.txt")),
        ],
        jim_want: Some(fixture!("native-dictionary-compilation-jim.txt")),
        jim_capabilities: &[],
    },
    CaseSpec {
        name: "native-compiler-traversal",
        source: fixture!("native-compiler-traversal.tcl"),
        c_wants: [
            Some(fixture!("native-compiler-traversal-8.4.txt")),
            Some(fixture!("native-compiler-traversal-8.5.txt")),
            Some(fixture!("native-compiler-traversal-8.6.txt")),
            Some(fixture!("native-compiler-traversal-9.0.txt")),
            Some(fixture!("native-compiler-traversal-9.1.txt")),
        ],
        jim_want: Some(fixture!("native-compiler-traversal-jim.txt")),
        jim_capabilities: &[],
    },
    CaseSpec {
        name: "native-static-frame-levels",
        source: fixture!("native-static-frame-levels.tcl"),
        c_wants: [None; 5],
        jim_want: Some(fixture!("native-static-frame-levels-jim.txt")),
        jim_capabilities: &[],
    },
    CaseSpec {
        name: "return-state-observers",
        source: fixture!("return-state-observers.tcl"),
        c_wants: [
            Some(fixture!("return-state-observers-8.4.txt")),
            Some(fixture!("return-state-observers-8.5.txt")),
            Some(fixture!("return-state-observers-8.6.txt")),
            Some(fixture!("return-state-observers-9.0.txt")),
            Some(fixture!("return-state-observers-9.1.txt")),
        ],
        // Jim's lack of C variable traces is independently measured in the
        // portability fixture, not treated as C return-effect evidence.
        jim_want: None,
        jim_capabilities: &[],
    },
    CaseSpec {
        name: "native-formals",
        source: fixture!("native-formals.tcl"),
        c_wants: [
            Some(fixture!("native-formals-8.4.txt")),
            Some(fixture!("native-formals-8.5.txt")),
            Some(fixture!("native-formals-8.6.txt")),
            Some(fixture!("native-formals-9.0.txt")),
            Some(fixture!("native-formals-9.1.txt")),
        ],
        jim_want: Some(fixture!("native-formals-jim.txt")),
        jim_capabilities: &[],
    },
    CaseSpec {
        name: "native-reference-formals",
        source: fixture!("native-reference-formals.tcl"),
        c_wants: [
            Some(fixture!("native-reference-formals-8.4.txt")),
            Some(fixture!("native-reference-formals-8.5.txt")),
            Some(fixture!("native-reference-formals-8.6.txt")),
            Some(fixture!("native-reference-formals-9.0.txt")),
            Some(fixture!("native-reference-formals-9.1.txt")),
        ],
        jim_want: Some(fixture!("native-reference-formals-jim.txt")),
        jim_capabilities: &[],
    },
    CaseSpec {
        name: "native-static-storage",
        source: fixture!("native-static-storage.tcl"),
        c_wants: [None; 5],
        jim_want: Some(fixture!("native-static-storage-jim.txt")),
        jim_capabilities: &[],
    },
    CaseSpec {
        name: "native-c-tail-scheduling",
        source: fixture!("tailcall-c.tcl"),
        c_wants: [
            Some(fixture!("tailcall-c-legacy.txt")),
            Some(fixture!("tailcall-c-legacy.txt")),
            Some(fixture!("tailcall-c-modern.txt")),
            Some(fixture!("tailcall-c-modern.txt")),
            Some(fixture!("tailcall-c-modern.txt")),
        ],
        jim_want: None,
        jim_capabilities: &[],
    },
    CaseSpec {
        name: "native-jim-tail-scheduling",
        source: fixture!("tailcall-jim.tcl"),
        c_wants: [None; 5],
        jim_want: Some(fixture!("tailcall-jim.txt")),
        jim_capabilities: &[],
    },
    CaseSpec {
        name: "native-variable-completion-domain",
        source: fixture!("native-variable-completions.tcl"),
        c_wants: [Some(fixture!("native-variable-completions-c.txt")); 5],
        // Jim has no C variable-trace operation; its absence is independently
        // asserted by the portability fixture rather than inferred here.
        jim_want: None,
        jim_capabilities: &[],
    },
    CaseSpec {
        name: "return-unwind-routes",
        source: fixture!("return-routes.tcl"),
        c_wants: [
            Some(fixture!("return-routes-8.4.txt")),
            Some(fixture!("return-routes-modern8.txt")),
            Some(fixture!("return-routes-modern8.txt")),
            Some(fixture!("return-routes-modern9.txt")),
            Some(fixture!("return-routes-modern9.txt")),
        ],
        jim_want: Some(fixture!("return-routes-jim.txt")),
        jim_capabilities: &[],
    },
    CaseSpec {
        name: "variable-link-identity",
        source: fixture!("variable-links.tcl"),
        c_wants: [Some(fixture!("variable-links-c.txt")); 5],
        jim_want: Some(fixture!("variable-links-jim.txt")),
        jim_capabilities: &[],
    },
    CaseSpec {
        name: "captured-lifecycle",
        source: fixture!("lifecycle.tcl"),
        c_wants: [
            Some(fixture!("lifecycle-2.2.11.txt")),
            Some(fixture!("lifecycle-2.3.8.txt")),
            Some(fixture!("lifecycle-2.5.11.txt")),
            Some(fixture!("lifecycle-2.5.11.txt")),
            Some(fixture!("lifecycle-2.6.0.txt")),
        ],
        jim_want: None,
        jim_capabilities: &[],
    },
    CaseSpec {
        name: "completion-and-frame",
        source: fixture!("completion.tcl"),
        c_wants: [
            Some(fixture!("completion-8.4.txt")),
            Some(fixture!("completion-8.6.txt")),
            Some(fixture!("completion-8.6.txt")),
            Some(fixture!("completion-8.6.txt")),
            Some(fixture!("completion-8.6.txt")),
        ],
        jim_want: Some(fixture!("completion-8.6.txt")),
        jim_capabilities: &[(JimCapability::Apply, true)],
    },
    CaseSpec {
        name: "lifecycle-repetition",
        source: fixture!("repetition.tcl"),
        c_wants: [
            Some(fixture!("repetition-legacy.txt")),
            Some(fixture!("repetition-legacy.txt")),
            Some(fixture!("repetition-legacy.txt")),
            Some(fixture!("repetition-legacy.txt")),
            Some(fixture!("repetition-2.6.0.txt")),
        ],
        jim_want: None,
        jim_capabilities: &[],
    },
    CaseSpec {
        name: "matcher-phase-order",
        source: fixture!("matcher-order.tcl"),
        c_wants: [
            Some(fixture!("matcher-order-2.2.11.txt")),
            Some(fixture!("matcher-order-modern.txt")),
            Some(fixture!("matcher-order-modern.txt")),
            Some(fixture!("matcher-order-modern.txt")),
            Some(fixture!("matcher-order-modern.txt")),
        ],
        jim_want: None,
        jim_capabilities: &[],
    },
    CaseSpec {
        name: "hidden-private-observer",
        source: fixture!("private-trace.tcl"),
        c_wants: [Some(fixture!("private-trace.txt")); 5],
        jim_want: None,
        jim_capabilities: &[],
    },
    CaseSpec {
        name: "iteration-value-versus-observer",
        source: fixture!("iteration-observer.tcl"),
        // Earlier package implementations lack the iteration option; its
        // absence is independently asserted by lifecycle-repetition.
        c_wants: [
            None,
            None,
            None,
            None,
            Some(fixture!("iteration-observer.txt")),
        ],
        jim_want: None,
        jim_capabilities: &[],
    },
    CaseSpec {
        name: "independent-jim-portability",
        source: fixture!("jim-portability.tcl"),
        c_wants: [None; 5],
        jim_want: Some(fixture!("jim-portability.txt")),
        jim_capabilities: &[
            (JimCapability::VariableTrace, false),
            (JimCapability::PackageRequire, true),
        ],
    },
    CaseSpec {
        name: "independent-jim-lifecycle",
        source: fixture!("jim-lifecycle.tcl"),
        c_wants: [None; 5],
        jim_want: Some(fixture!("jim-lifecycle.txt")),
        jim_capabilities: &[(JimCapability::PackageRequire, true)],
    },
];

fn package_version(version: TclVersion) -> &'static str {
    match version {
        TclVersion::V8_4 => "2.2.11",
        TclVersion::V8_5 => "2.3.8",
        TclVersion::V8_6 | TclVersion::V9_0 => "2.5.11",
        TclVersion::V9_1 => "2.6.0",
    }
}

fn assert_observations(actual: &str, wanted: &str, case: &str, reference: &str) -> usize {
    let actual: Vec<_> = actual.lines().collect();
    let wanted: Vec<_> = wanted.trim().lines().collect();
    assert_eq!(
        actual.len(),
        wanted.len(),
        "{case} on {reference}: unexpected output {actual:?}"
    );
    for (index, (actual, wanted)) in actual.iter().zip(&wanted).enumerate() {
        assert_eq!(actual, wanted, "{case} observation {index} on {reference}");
    }
    wanted.len()
}

#[test]
fn body_case_definitions_cover_768_c_and_140_jim_observations() {
    let counts: Vec<_> = TclVersion::ALL
        .iter()
        .map(|version| {
            CASES
                .iter()
                .filter_map(|case| case.c_want(*version))
                .map(|wanted| wanted.trim().lines().count())
                .sum::<usize>()
        })
        .collect();
    assert_eq!(counts, [138, 151, 158, 158, 163]);
    assert_eq!(counts.iter().sum::<usize>(), 768);
    assert_eq!(
        CASES
            .iter()
            .filter_map(|case| case.jim_want)
            .map(|wanted| wanted.trim().lines().count())
            .sum::<usize>(),
        140
    );
}

#[test]
fn c_tcl_matches_versioned_body_and_completion_contracts() {
    let references = available_tclshs();
    assert!(
        !references.is_empty(),
        "no C Tcl oracle: configure TCL_LSP_TCLSH84 .. TCL_LSP_TCLSH91"
    );
    for reference in references {
        assert_eq!(
            reference.patchlevel,
            reference_patchlevel(reference.version),
            "body corpus requires its audited C source release"
        );
        let package = package_version(reference.version);
        let mut count = 0;
        for case in CASES {
            let Some(wanted) = case.c_want(reference.version) else {
                continue;
            };
            let source = format!("package require -exact tcltest {package}\n{}", case.source);
            let actual = run_script_fixture(&reference.path, &source, &[])
                .expect("run isolated C body fixture")
                .strict_text()
                .expect("clean C body fixture");
            count += assert_observations(&actual, wanted, case.name, &reference.patchlevel);
        }
        eprintln!(
            "C Tcl {} / tcltest {package}: {count} body observations passed",
            reference.patchlevel
        );
    }
}

#[test]
fn jim_matches_its_independent_body_and_completion_contracts() {
    let reference = if std::env::var_os("TCL_LSP_REQUIRE_JIM_ORACLE").is_some() {
        Some(require_jimsh().expect("required Jim oracle"))
    } else {
        locate_jimsh().expect("valid Jim oracle override")
    };
    let Some(reference) = reference else {
        eprintln!("Jim oracle not requested and unavailable; set TCL_LSP_JIMSH");
        return;
    };
    let mut count = 0;
    for case in CASES {
        let Some(wanted) = case.jim_want else {
            continue;
        };
        for &(capability, expected) in case.jim_capabilities {
            assert_eq!(
                reference.supports(capability),
                expected,
                "{}: Jim capability {} changed; remeasure the fixture",
                case.name,
                capability.name()
            );
        }
        let actual = run_script_fixture(&reference.path, case.source, &[])
            .expect("run isolated Jim body fixture")
            .strict_text()
            .expect("clean Jim body fixture");
        count += assert_observations(&actual, wanted, case.name, &reference.patchlevel);
    }
    assert_eq!(count, 140);
    eprintln!(
        "Jim {}: {count} independent body observations passed",
        reference.patchlevel
    );
}
