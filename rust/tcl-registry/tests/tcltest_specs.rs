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

//! tcltest command-spec coverage — verified against the bundled tcltest
//! sources (`tmp/tcl{8.4.20,8.5.19,8.6.16,9.0.3}/library/tcltest/tcltest.tcl`)
//! and the test-harness C commands (`generic/tclTest*.c`).  Asserts the public
//! command surface, per-version dialect gating, and the option surface of
//! `test` / `configure`.

use tcl_registry::CommandRegistry;
use tcl_registry::dialects::DialectSet;

fn reg() -> CommandRegistry {
    CommandRegistry::build_default()
}

#[test]
fn public_tcltest_commands_are_registered() {
    let r = reg();
    // The functional + configuration + convenience commands exported by every
    // bundled tcltest (2.2.11 … 2.5.10).
    for name in [
        "tcltest::test",
        "tcltest::cleanupTests",
        "tcltest::configure",
        "tcltest::customMatch",
        "tcltest::testConstraint",
        "tcltest::makeFile",
        "tcltest::makeDirectory",
        "tcltest::removeFile",
        "tcltest::removeDirectory",
        "tcltest::runAllTests",
        "tcltest::loadTestedCommands",
        "tcltest::verbose",
        "tcltest::match",
        "tcltest::skip",
        "tcltest::debug",
        "tcltest::interpreter",
        "tcltest::outputChannel",
        "tcltest::errorChannel",
        "tcltest::workingDirectory",
        "tcltest::temporaryDirectory",
        "tcltest::testsDirectory",
    ] {
        assert!(r.get(name).is_some(), "{name} should be registered");
    }
}

#[test]
fn bytestring_is_tcl8_only() {
    // `tcltest::bytestring` is guarded out under Tcl 9.0+ (tcltest 2.5.10).
    let r = reg();
    let spec = r.get("tcltest::bytestring").expect("bytestring registered");
    assert!(spec.supports_dialect(DialectSet::TCL84));
    assert!(spec.supports_dialect(DialectSet::TCL86));
    assert!(!spec.supports_dialect(DialectSet::TCL90));
    assert!(!spec.supports_dialect(DialectSet::TCL91));
}

#[test]
fn c_harness_commands_are_version_gated() {
    let r = reg();

    // 8.4/8.5 only — removed in 8.6.
    for name in ["testaccessproc", "teststatproc", "testopenfilechannelproc"] {
        let spec = r.get(name).unwrap_or_else(|| panic!("{name} registered"));
        assert!(spec.supports_dialect(DialectSet::TCL84), "{name} in 8.4");
        assert!(spec.supports_dialect(DialectSet::TCL85), "{name} in 8.5");
        assert!(!spec.supports_dialect(DialectSet::TCL86), "{name} not 8.6");
        assert!(!spec.supports_dialect(DialectSet::TCL90), "{name} not 9.0");
    }

    // 8.4 only.
    let convertobj = r.get("testconvertobj").expect("testconvertobj registered");
    assert!(convertobj.supports_dialect(DialectSet::TCL84));
    assert!(!convertobj.supports_dialect(DialectSet::TCL85));

    // `teststaticpkg` (8.x) was renamed to `teststaticlibrary` (9.0+).
    let pkg = r.get("teststaticpkg").expect("teststaticpkg registered");
    assert!(pkg.supports_dialect(DialectSet::TCL86));
    assert!(!pkg.supports_dialect(DialectSet::TCL90));
    let lib = r
        .get("teststaticlibrary")
        .expect("teststaticlibrary registered");
    assert!(lib.supports_dialect(DialectSet::TCL90));
    assert!(!lib.supports_dialect(DialectSet::TCL86));

    // `testsaveresult` was removed in 9.0.
    let saveresult = r.get("testsaveresult").expect("testsaveresult registered");
    assert!(saveresult.supports_dialect(DialectSet::TCL86));
    assert!(!saveresult.supports_dialect(DialectSet::TCL90));

    // `testcmdobj2` is 9.0+ only.
    let cmdobj2 = r.get("testcmdobj2").expect("testcmdobj2 registered");
    assert!(cmdobj2.supports_dialect(DialectSet::TCL90));
    assert!(cmdobj2.supports_dialect(DialectSet::TCL91));
    assert!(!cmdobj2.supports_dialect(DialectSet::TCL86));
}

#[test]
fn configure_models_its_full_option_surface() {
    let r = reg();
    let spec = r.get("tcltest::configure").expect("configure registered");
    for opt in [
        "-verbose",
        "-match",
        "-skip",
        "-file",
        "-notfile",
        "-relateddir",
        "-asidefromdir",
        "-constraints",
        "-limitconstraints",
        "-singleproc",
        "-debug",
        "-preservecore",
        "-load",
        "-loadfile",
        "-tmpdir",
        "-testdir",
        "-outfile",
        "-errfile",
    ] {
        assert!(
            spec.find_option(opt, None, None).is_some(),
            "configure should model {opt}"
        );
    }
}

#[test]
fn test_error_code_option_needs_tcltest_2_5() {
    // `test -errorCode` was added in tcltest 2.5 (bundled with Tcl 8.6).  It is
    // absent when the resolved package floor is 2.3 (Tcl 8.5's tcltest).
    let r = reg();
    let spec = r.get("tcltest::test").expect("test registered");
    assert!(
        spec.find_option("-errorCode", None, Some("2.5")).is_some(),
        "-errorCode present at tcltest 2.5"
    );
    assert!(
        spec.find_option("-errorCode", None, Some("2.3")).is_none(),
        "-errorCode absent at tcltest 2.3"
    );
    // Options present since 2.2 remain available at every floor.
    assert!(spec.find_option("-body", None, Some("2.2")).is_some());
    assert!(spec.find_option("-match", None, Some("2.3")).is_some());
}
