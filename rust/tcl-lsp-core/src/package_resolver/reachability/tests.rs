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

//! Guard-reachability tests.  Every expectation is the answer a real
//! `tclsh8.6` / `tclsh9.0` gives for the same index file.

use super::{Availability, scan};
use tcl_dialect::TclVersion;

/// `(package name, availability)` for each declaration `content` yields,
/// evaluated for `target`.
fn availabilities(content: &str, target: Option<TclVersion>) -> Vec<(String, Availability)> {
    let registry = tcl_registry::registry_for_dialect("tcl");
    let mut out = Vec::new();
    scan(content, registry, &mut |reached| {
        let name = super::word_raw(reached.text, &reached.words[2]).to_owned();
        out.push((name, reached.conditions.availability(target)));
    });
    out
}

/// The availability of the single declaration `content` yields.
fn only(content: &str, target: Option<TclVersion>) -> Availability {
    let found = availabilities(content, target);
    assert_eq!(found.len(), 1, "expected one declaration, got {found:?}");
    found[0].1
}

const V86: Option<TclVersion> = Some(TclVersion::V8_6);
const V90: Option<TclVersion> = Some(TclVersion::V9_0);

/// The unguarded index file — by far the commonest shape — is unconditional
/// on every release.
#[test]
fn an_unguarded_declaration_is_available_everywhere() {
    let src = "package ifneeded mypkg 1.0 [list source [file join $dir mypkg.tcl]]\n";
    assert_eq!(only(src, V86), Availability::Available);
    assert_eq!(only(src, V90), Availability::Available);
    assert_eq!(only(src, None), Availability::Available);
}

/// Issue #1017's exact repro, shrunk from tcllib's `modules/try/pkgIndex.tcl`
/// `file::home` gate.
///
/// Oracle: `tclsh9.0` → `package require mypkg` fails ("can't find package
/// mypkg"); `tclsh8.6` → it loads and returns 1.0.
#[test]
fn a_vsatisfies_early_return_gates_the_declaration_1017() {
    let src = "if {[package vsatisfies [package provide Tcl] 9-]} { return }\n\
               package ifneeded mypkg 1.0 [list source [file join $dir mypkg.tcl]]\n";
    assert_eq!(only(src, V90), Availability::Unavailable);
    assert_eq!(only(src, V86), Availability::Available);
    // With no known target release the guard cannot be decided either way.
    assert_eq!(only(src, None), Availability::Conditional);
}

/// The real tcllib head guard — a *negated* test with two requirements.
///
/// Oracle: `package vsatisfies [package provide Tcl] 8.5 9` is 1 on both
/// 8.6.14 and 9.0.4, so the `return` never fires on either and the package
/// loads; it does fire on 8.4.
#[test]
fn the_tcllib_head_guard_admits_both_supported_releases() {
    let src = "if {![package vsatisfies [package provide Tcl] 8.5 9]} {return}\n\
               package ifneeded log 1.5 [list source [file join $dir log.tcl]]\n";
    assert_eq!(only(src, V86), Availability::Available);
    assert_eq!(only(src, V90), Availability::Available);
    assert_eq!(only(src, Some(TclVersion::V8_4)), Availability::Unavailable);
}

/// The trivial control from issue #1017: unreachable under *every* release,
/// with no version reasoning involved at all.
#[test]
fn a_constant_true_early_return_gates_the_declaration_1017() {
    for src in [
        "if {1} { return }\npackage ifneeded mypkg 1.0 [list source [file join $dir mypkg.tcl]]\n",
        "if {true} {return}\npackage ifneeded mypkg 1.0 [list source [file join $dir mypkg.tcl]]\n",
        "return\npackage ifneeded mypkg 1.0 [list source [file join $dir mypkg.tcl]]\n",
    ] {
        assert!(
            availabilities(src, V90).is_empty(),
            "nothing after an unconditional return is reachable: {src}",
        );
    }
}

/// The negative control from issue #1017, which already behaved: a
/// declaration nested in a constant-false branch never registers.  It is now
/// *seen* (the scan descends into branches) and reported unavailable, rather
/// than being invisible by accident.
#[test]
fn a_constant_false_branch_never_registers_1017() {
    let src =
        "if {0} {\n  package ifneeded mypkg 1.0 [list source [file join $dir mypkg.tcl]]\n}\n";
    assert_eq!(only(src, V90), Availability::Unavailable);
    assert_eq!(only(src, V86), Availability::Unavailable);
}

/// Issue #923 idx 42: the TEA idiom that picks a Tcl-8 or Tcl-9 build.  Both
/// arms declare the package, so it is available on both releases — the
/// over-flagging direction of the same mechanism.
///
/// Oracle: `tclsh8.6` and `tclsh9.0` both load the package and run its
/// command.
#[test]
fn both_arms_of_a_tea_version_branch_declare_the_package_923_idx42() {
    let src = "if {[package vsatisfies [package provide Tcl] 9.0-]} {\n\
               \x20   package ifneeded tclopt 0.4 [list source [file join $dir tclopt.tcl]]\n\
               } else {\n\
               \x20   package ifneeded tclopt 0.4 [list source [file join $dir tclopt.tcl]]\n\
               }\n";
    for target in [V86, V90] {
        let found = availabilities(src, target);
        assert_eq!(found.len(), 2, "both arms are scanned: {found:?}");
        assert!(
            found
                .iter()
                .any(|(_, availability)| *availability == Availability::Available),
            "one arm must be the taken one for {target:?}: {found:?}",
        );
        assert!(
            found
                .iter()
                .any(|(_, availability)| *availability == Availability::Unavailable),
            "…and the other the untaken one: {found:?}",
        );
    }
}

/// The whole `modules/try/pkgIndex.tcl` from tcllib 2.0, verbatim: two guards
/// in sequence, the second wrapping a `package provide` beside its `return`.
///
/// Oracle (`tclsh8.6`): `try`, `throw`, and `file::home` all load.
/// Oracle (`tclsh9.0`): `try` and `throw` load; `file::home` is provided by
/// the guard's own `package provide`, so `fhome.tcl` is never sourced.
#[test]
fn the_real_tcllib_try_index_resolves_per_release() {
    let src = "if {![package vsatisfies [package provide Tcl] 8.5 9]} {\n\
               \x20   return\n\
               }\n\
               package ifneeded try   1.1 [list source [file join $dir try.tcl]]\n\
               package ifneeded throw 1.1 [list source [file join $dir throw.tcl]]\n\
               if {[package vsatisfies [package provide Tcl] 9-]} {\n\
               \x20   package provide file::home 1\n\
               \x20   return\n\
               }\n\
               package ifneeded file::home 1 [list source [file join $dir fhome.tcl]]\n";

    let on_86 = availabilities(src, V86);
    assert_eq!(
        on_86,
        vec![
            ("try".to_owned(), Availability::Available),
            ("throw".to_owned(), Availability::Available),
            ("file::home".to_owned(), Availability::Available),
        ],
    );

    let on_90 = availabilities(src, V90);
    assert_eq!(
        on_90,
        vec![
            ("try".to_owned(), Availability::Available),
            ("throw".to_owned(), Availability::Available),
            ("file::home".to_owned(), Availability::Unavailable),
        ],
    );
}

/// A guard this scan does not model must leave the declaration *conditional*,
/// never decided.  These are the documented limits, pinned so a future change
/// that starts guessing is caught.
#[test]
fn an_unmodelled_guard_leaves_the_declaration_conditional() {
    for src in [
        // A platform test, not a version test.
        "if {$::tcl_platform(platform) eq \"windows\"} { return }\n\
         package ifneeded mypkg 1.0 [list source [file join $dir mypkg.tcl]]\n",
        // A filesystem probe.
        "if {![file exists [file join $dir mypkg.tcl]]} { return }\n\
         package ifneeded mypkg 1.0 [list source [file join $dir mypkg.tcl]]\n",
        // A compound expression over two version tests.
        "if {[package vsatisfies [package provide Tcl] 9-] && $::force} { return }\n\
         package ifneeded mypkg 1.0 [list source [file join $dir mypkg.tcl]]\n",
        // `vsatisfies` over a package that is not Tcl says nothing about the
        // Tcl release.
        "if {[package vsatisfies [package provide Tk] 9-]} { return }\n\
         package ifneeded mypkg 1.0 [list source [file join $dir mypkg.tcl]]\n",
        // A loop body: a `return` inside one is never treated as certain.
        "foreach v {8 9} { if {$v} { return } }\n\
         package ifneeded mypkg 1.0 [list source [file join $dir mypkg.tcl]]\n",
    ] {
        assert_eq!(
            only(src, V90),
            Availability::Conditional,
            "must abstain, not decide: {src}",
        );
    }
}

/// `error` / `throw` / `exit` end the script exactly as `return` does —
/// `tclPkgUnknown` sources each index inside a `catch`, so a raise stops
/// registration.  Recognised from the registry's `TERMINATES_BLOCK` trait,
/// so this needs no per-command list here.
#[test]
fn any_registry_terminator_ends_the_index() {
    for terminator in ["return", "error boom", "exit 1"] {
        let src = format!(
            "if {{[package vsatisfies [package provide Tcl] 9-]}} {{ {terminator} }}\n\
             package ifneeded mypkg 1.0 [list source [file join $dir mypkg.tcl]]\n"
        );
        assert_eq!(only(&src, V90), Availability::Unavailable, "{terminator}");
        assert_eq!(only(&src, V86), Availability::Available, "{terminator}");
    }
}

/// A guard body that does something *other* than terminate leaves everything
/// after it unconditional — the guard selects behaviour, not reachability.
#[test]
fn a_non_terminating_guard_body_does_not_gate_what_follows() {
    let src = "if {[package vsatisfies [package provide Tcl] 9-]} { set extra 1 }\n\
               package ifneeded mypkg 1.0 [list source [file join $dir mypkg.tcl]]\n";
    assert_eq!(only(src, V90), Availability::Available);
    assert_eq!(only(src, V86), Availability::Available);
}

/// An `elseif` chain: each arm's condition is read, and an arm is reached only
/// when every earlier one tested false.
#[test]
fn an_elseif_chain_selects_one_arm_per_release() {
    let src = "if {[package vsatisfies [package provide Tcl] 9-]} {\n\
               \x20   package ifneeded mypkg 2.0 [list source [file join $dir new.tcl]]\n\
               } elseif {[package vsatisfies [package provide Tcl] 8.5-]} {\n\
               \x20   package ifneeded mypkg 1.0 [list source [file join $dir old.tcl]]\n\
               }\n";
    assert_eq!(
        availabilities(src, V90),
        vec![
            ("mypkg".to_owned(), Availability::Available),
            ("mypkg".to_owned(), Availability::Unavailable),
        ],
    );
    assert_eq!(
        availabilities(src, V86),
        vec![
            ("mypkg".to_owned(), Availability::Unavailable),
            ("mypkg".to_owned(), Availability::Available),
        ],
    );
}

/// The `then` noise word and a brace-less body are part of `if`'s real
/// grammar; the clause chain comes from the registry, so both work without
/// this module knowing about them.
#[test]
fn the_then_noise_word_and_a_braceless_return_are_understood() {
    let src = "if {1} then {return}\n\
               package ifneeded mypkg 1.0 [list source [file join $dir mypkg.tcl]]\n";
    assert!(availabilities(src, V90).is_empty());

    let bare = "if {![package vsatisfies [package provide Tcl] 8.5 9]} return\n\
                package ifneeded mypkg 1.0 [list source [file join $dir mypkg.tcl]]\n";
    assert_eq!(only(bare, V86), Availability::Available);
    assert_eq!(
        only(bare, Some(TclVersion::V8_4)),
        Availability::Unavailable
    );
}

/// Deeply nested guards must not recurse without bound; past the cap the scan
/// abstains rather than descending further.
#[test]
fn pathological_nesting_terminates() {
    let depth = 200;
    let mut src = String::new();
    for _ in 0..depth {
        src.push_str("if {1} {\n");
    }
    src.push_str("package ifneeded mypkg 1.0 [list source [file join $dir mypkg.tcl]]\n");
    for _ in 0..depth {
        src.push_str("}\n");
    }
    // The only requirement is that this returns at all.
    let _ = availabilities(&src, V90);
}
