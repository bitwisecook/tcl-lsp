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

//! Live package comparator, provider selection and Jim absence controls.
//! Reference discovery and exact engine validation belong to tcl-test-support.

use std::cmp::Ordering;
use std::path::Path;
use tcl_dialect::{
    PackagePrefer, compare_versions_bytes_for, select_package_version_for,
    validate_version_bytes_for, version_satisfies_bytes_for,
};
use tcl_registry::native_package::{NativePackageProtocol, PackageDispatch};
use tcl_test_support::{JimCapability, available_tclshs, locate_jimsh, require_jimsh, run_script};

fn output(binary: &Path, script: &str) -> String {
    run_script(binary, script.as_bytes())
        .expect("execute validated package reference")
        .strict_text()
        .expect("package reference exits successfully with UTF-8 output")
}

#[test]
fn package_comparator_matches_all_live_c_releases() {
    const PAIRS: &[(&str, &str)] = &[
        ("01.02", "1.2"),
        ("1.0", "1.0.0"),
        ("1.2a1", "1.2b1"),
        ("1.2b1", "1.2"),
        ("4294967295", "4294967296"),
        ("18446744073709551615", "18446744073709551616"),
        ("1.0000000000000000000000000000000000000005", "1.5"),
    ];
    const REQUIREMENTS: &[(&str, &str)] = &[
        ("1.2", "1.0"),
        ("2.0", "1.0"),
        ("1.2a1", "1.2"),
        ("01.02", "1.2"),
    ];
    for oracle in available_tclshs() {
        let mut script = String::new();
        let mut expected = String::new();
        for &(a, b) in PAIRS {
            script.push_str(&format!("set code [catch {{package vcompare {{{a}}} {{{b}}}}} answer]\nputs $code\nif {{$code == 0}} {{puts $answer}}\n"));
            if !validate_version_bytes_for(a.as_bytes(), oracle.version)
                || !validate_version_bytes_for(b.as_bytes(), oracle.version)
            {
                expected.push_str("1\n");
                continue;
            }
            expected.push_str("0\n");
            let answer =
                match compare_versions_bytes_for(a.as_bytes(), b.as_bytes(), oracle.version) {
                    Ordering::Less => -1,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
            expected.push_str(&format!("{answer}\n"));
        }
        for &(version, requirement) in REQUIREMENTS {
            script.push_str(&format!(
                "set code [catch {{package vsatisfies {{{version}}} {{{requirement}}}}} answer]\nputs $code\nif {{$code == 0}} {{puts $answer}}\n"
            ));
            if !validate_version_bytes_for(version.as_bytes(), oracle.version) {
                expected.push_str("1\n");
                continue;
            }
            expected.push_str("0\n");
            expected.push_str(
                if version_satisfies_bytes_for(
                    version.as_bytes(),
                    requirement.as_bytes(),
                    oracle.version,
                ) {
                    "1\n"
                } else {
                    "0\n"
                },
            );
        }
        assert_eq!(
            output(&oracle.path, &script),
            expected.trim(),
            "{}",
            oracle.patchlevel
        );
    }
}

#[test]
fn package_lookup_matches_all_live_c_releases() {
    for oracle in available_tclshs() {
        let protocol = NativePackageProtocol::C(oracle.version);
        let providers = ["1.1", "1.4", "1.5b1", "2.0"];
        for (requirement, exact) in [("1.0", false), ("1.4", true), ("3.0", false)] {
            let mut script = String::new();
            let mut expected = String::new();
            for version in providers {
                script.push_str(&format!("puts [catch {{package ifneeded OracleSelection {version} {{package provide OracleSelection {version}}}}}]\n"));
                expected.push_str(
                    if validate_version_bytes_for(version.as_bytes(), oracle.version) {
                        "0\n"
                    } else {
                        "1\n"
                    },
                );
            }
            script.push_str(&format!("set code [catch {{package require {}OracleSelection {requirement}}} answer]\nputs $code\nif {{$code == 0}} {{puts $answer}}\n", if exact { "-exact " } else { "" }));
            let selection = if exact {
                tcl_dialect::select_package_version_exact_for(
                    &providers,
                    requirement,
                    oracle.version,
                )
            } else {
                select_package_version_for(
                    &providers,
                    &[requirement],
                    PackagePrefer::Stable,
                    oracle.version,
                )
            };
            expected.push_str(&selection.map_or_else(
                || "1\n".to_owned(),
                |index| format!("0\n{}\n", providers[index]),
            ));
            assert_eq!(
                output(&oracle.path, &script),
                expected.trim(),
                "{} require {requirement} exact={exact}",
                oracle.patchlevel
            );
        }
        let prefer = output(
            &oracle.path,
            "puts [catch {package prefer} answer]\nputs $answer\n",
        );
        if protocol.preference_members().is_some() {
            assert_eq!(prefer, "0\nstable", "{}", oracle.patchlevel);
        } else {
            assert!(prefer.starts_with("1\n"), "{}: {prefer}", oracle.patchlevel);
        }
        assert_eq!(
            output(
                &oracle.path,
                "package provide OracleProvided 01.02\nputs [package require OracleProvided]\nputs [catch {package require OracleProvided 3.0}]\n"
            ),
            "01.02\n1",
            "{}",
            oracle.patchlevel
        );
    }
}

#[test]
fn jim_package_lookup_and_unsupported_version_surfaces_match_live_reference() {
    let oracle = if std::env::var_os("TCL_LSP_REQUIRE_JIM_ORACLE").is_some() {
        require_jimsh().expect("required pinned Jim package reference")
    } else {
        let Some(oracle) = locate_jimsh().expect("valid optional Jim package reference") else {
            return;
        };
        oracle
    };
    assert!(oracle.supports(JimCapability::PackageProvide));
    assert!(oracle.supports(JimCapability::PackageRequire));
    assert!(!oracle.supports(JimCapability::PackageIfneeded));
    assert!(!oracle.supports(JimCapability::PackagePrefer));
    let protocol = NativePackageProtocol::Jim084;
    for member in ["vcompare", "vsatisfies", "ifneeded", "prefer"] {
        let Some(PackageDispatch::Error(expected)) = protocol.jim_dispatch(member.as_bytes(), None)
        else {
            panic!("unsupported Jim package member must retain its own diagnostic");
        };
        let actual = output(
            &oracle.path,
            &format!("puts [catch {{package {member}}} answer]\nputs $answer\n"),
        );
        assert_eq!(
            actual,
            format!("1\n{}", String::from_utf8(expected).unwrap()),
            "{member}"
        );
    }
    // Jim stores its direct-loader version marker and ignores optional version operands.
    assert_eq!(
        output(
            &oracle.path,
            "package provide OracleProvided 01.02\nputs [package require OracleProvided]\nputs [package require OracleProvided 3.0]\npackage forget OracleProvided\nputs [catch {package require OracleProvided}]\n"
        ),
        "1.0\n1.0\n1"
    );
}
