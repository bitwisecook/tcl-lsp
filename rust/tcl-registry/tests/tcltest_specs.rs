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
fn tcltest_and_harness_commands_are_never_in_irules_or_iapps() {
    // tcltest and the C test-harness commands are Tcl-only; they must not leak
    // into the restricted F5 iRules / iApps dialects.
    let r = reg();
    for name in [
        "tcltest::test",
        "tcltest::configure",
        "tcltest::cleanupTests",
        "tcltest::testConstraint",
        "tcltest::makeFile",
        "tcltest::bytestring",
        "testchannel",
        "testobj",
        "teststringobj",
        "testcmdobj2",
        "testsaveresult",
    ] {
        let spec = r.get(name).unwrap_or_else(|| panic!("{name} registered"));
        assert!(
            !spec.supports_dialect(DialectSet::IRULES),
            "{name} must not be available in f5-irules"
        );
        assert!(
            !spec.supports_dialect(DialectSet::IAPPS),
            "{name} must not be available in f5-iapps"
        );
        // …but must remain available under a real Tcl core version.
        assert!(
            spec.supports_dialect(DialectSet::TCL84)
                || spec.supports_dialect(DialectSet::TCL86)
                || spec.supports_dialect(DialectSet::TCL90),
            "{name} must stay available under some Tcl version"
        );
    }
}

#[test]
fn test_command_declares_a_symbol_definer() {
    // Issue #790: `tcltest::test` binds a navigable definition name (arg 0)
    // with its description at arg 1, filed under the `Test` outline category —
    // and this must be *registry data* the analyser / symbol providers read,
    // not a command-name check baked into the compiler.
    use tcl_registry::symbol_def::DefinedSymbolKind;
    let r = reg();
    let sym = r
        .defines_symbol("tcltest::test", DialectSet::ALL_TCL)
        .expect("tcltest::test should declare a symbol definer");
    assert_eq!(sym.name_arg, 0, "test name is the first argument");
    assert_eq!(sym.detail_arg, Some(1), "test description is the second argument");
    assert_eq!(sym.kind, DefinedSymbolKind::Test);

    // The `commands_defining_symbols` aggregate lists it.
    assert!(
        r.commands_defining_symbols().contains(&"tcltest::test"),
        "tcltest::test should appear in the symbol-definer set"
    );

    // A plain command that defines no outline symbol returns `None` — the
    // mechanism is opt-in, not blanket.
    assert!(
        r.defines_symbol("puts", DialectSet::ALL_TCL).is_none(),
        "puts defines no outline symbol"
    );
    assert!(
        r.defines_symbol("tcltest::cleanupTests", DialectSet::ALL_TCL)
            .is_none(),
        "cleanupTests defines no outline symbol"
    );
}

#[test]
fn testconstraint_and_custommatch_declare_their_own_symbol_kinds() {
    // Every tcltest command that *binds a name* is a symbol definer with its
    // own outline category — a constraint and a match mode are distinct from a
    // test case.
    use tcl_registry::symbol_def::DefinedSymbolKind;
    let r = reg();

    let constraint = r
        .defines_symbol("tcltest::testConstraint", DialectSet::ALL_TCL)
        .expect("testConstraint should declare a symbol definer");
    assert_eq!(constraint.name_arg, 0);
    assert_eq!(constraint.kind, DefinedSymbolKind::Constraint);
    // Only the two-arg setter defines; the one-arg getter merely reads.
    assert_eq!(constraint.requires_arg, Some(1));
    assert_eq!(constraint.detail_arg, Some(1));

    let matcher = r
        .defines_symbol("tcltest::customMatch", DialectSet::ALL_TCL)
        .expect("customMatch should declare a symbol definer");
    assert_eq!(matcher.name_arg, 0);
    assert_eq!(matcher.kind, DefinedSymbolKind::Matcher);
    assert_eq!(matcher.requires_arg, None);

    // All three name-binding tcltest commands are in the aggregate set.
    let definers = r.commands_defining_symbols();
    for name in [
        "tcltest::test",
        "tcltest::testConstraint",
        "tcltest::customMatch",
    ] {
        assert!(definers.contains(&name), "{name} should define a symbol");
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

    // 8.4 only — the obsolete FS hooks are compiled out (`#undef
    // USE_OBSOLETE_FS_HOOKS`) from Tcl 8.5 onwards.
    for name in ["testaccessproc", "teststatproc", "testopenfilechannelproc"] {
        let spec = r.get(name).unwrap_or_else(|| panic!("{name} registered"));
        assert!(spec.supports_dialect(DialectSet::TCL84), "{name} in 8.4");
        assert!(!spec.supports_dialect(DialectSet::TCL85), "{name} not 8.5");
        assert!(!spec.supports_dialect(DialectSet::TCL86), "{name} not 8.6");
    }

    // `testevent` is registered as far back as 8.4 (via `Tcl_CreateObjCommand(
    // interp, "testevent", …)` — a whitespace variant), so it is all-Tcl.
    let event = r.get("testevent").expect("testevent registered");
    assert!(event.supports_dialect(DialectSet::TCL84));
    assert!(event.supports_dialect(DialectSet::TCL90));

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
    // With no package floor (the common unversioned `package require tcltest`),
    // the option is gated by Tcl core dialect instead: hidden under 8.5,
    // offered under 8.6+ (which bundles tcltest 2.5).
    assert!(
        spec.find_option("-errorCode", Some(DialectSet::TCL85), None)
            .is_none(),
        "-errorCode hidden under Tcl 8.5 with no package floor"
    );
    assert!(
        spec.find_option("-errorCode", Some(DialectSet::TCL86), None)
            .is_some(),
        "-errorCode offered under Tcl 8.6"
    );
    // Options present since 2.2 remain available at every floor.
    assert!(spec.find_option("-body", None, Some("2.2")).is_some());
    assert!(spec.find_option("-match", None, Some("2.3")).is_some());
}
