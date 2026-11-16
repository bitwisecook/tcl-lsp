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

//! `tcl spec upgrade`: the 1.x `dialects` vocabulary rewritten in place, restyled and verified.

use super::*;

/// `tcl spec upgrade` rewrites the 1.x `dialects` vocabulary in place,
/// moves the `speclib` word, and leaves every other byte alone.
#[test]
fn spec_upgrade_translates_dialects_rows_and_moves_the_version_word() {
    let tree = Tree::new("upgrade");
    let pack = tree.path().join("demo.tclspec");
    let source = "# a comment the rewriter must not touch\n\
                  speclib demo 1.2 {\n\
                  \x20   command demo::greet {\n\
                  \x20       arity 1\n\
                  \x20       dialects {tcl8.6+ tk}\n\
                  \x20   }\n\
                  }\n";
    std::fs::write(&pack, source).expect("write pack");

    let path = pack.to_string_lossy().into_owned();
    let (stdout, _stderr, code) = run(&["spec", "upgrade", "--check", &path]);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("would translate 1 row(s)"), "{stdout}");
    assert_eq!(
        std::fs::read_to_string(&pack).expect("read back"),
        source,
        "--check writes nothing"
    );

    let (stdout, _stderr, code) = run(&["spec", "upgrade", &path]);
    assert_eq!(code, 0, "{stdout}");
    let upgraded = std::fs::read_to_string(&pack).expect("read back");
    assert_eq!(
        upgraded,
        "# a comment the rewriter must not touch\n\
         speclib demo 2.0 {\n\
         \x20   command demo::greet {\n\
         \x20       arity 1\n\
         \x20       available {tcl 8.6-} {package Tk}\n\
         \x20   }\n\
         }\n"
    );
}

/// `--restyle` re-emits the upgraded pack in canonical form; with
/// `--check` it only says so.
#[test]
fn spec_upgrade_restyle_emits_canonical_form() {
    let tree = Tree::new("upgrade-restyle");
    let pack = tree.path().join("demo.tclspec");
    let source = "# a comment the restyle drops\n\
                  speclib demo 1.2 {\n\
                  \x20   command demo::greet {\n\
                  \x20           arity 1 ;# odd indent\n\
                  \x20       dialects {tcl8.6+ tk}\n\
                  \x20   }\n\
                  }\n";
    std::fs::write(&pack, source).expect("write pack");

    let path = pack.to_string_lossy().into_owned();
    let (stdout, _stderr, code) = run(&["spec", "upgrade", "--check", "--restyle", &path]);
    assert_eq!(code, 0, "{stdout}");
    assert!(
        stdout.contains("would re-emit in canonical form"),
        "{stdout}"
    );
    assert_eq!(
        std::fs::read_to_string(&pack).expect("read back"),
        source,
        "--check writes nothing"
    );

    let (stdout, _stderr, code) = run(&["spec", "upgrade", "--restyle", &path]);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("re-emitted in canonical form"), "{stdout}");
    assert_eq!(
        std::fs::read_to_string(&pack).expect("read back"),
        "speclib demo 2.0 {\n\
         \n\
         command demo::greet {\n\
         \x20   arity 1\n\
         \x20   available {tcl 8.6-} {package Tk}\n\
         }\n\
         \n\
         }\n"
    );

    // A programmed pack is refused whole (E-R12), and the file is untouched.
    let programmed = "speclib demo 2.0 {\n\
                      foreach name {a b} {\n\
                      \x20   command $name { arity 0 }\n\
                      }\n\
                      }\n";
    std::fs::write(&pack, programmed).expect("write pack");
    let (_stdout, stderr, code) = run(&["spec", "upgrade", "--restyle", &path]);
    assert_ne!(code, 0);
    assert!(stderr.contains("E-R12"), "{stderr}");
    assert_eq!(
        std::fs::read_to_string(&pack).expect("read back"),
        programmed
    );
}

/// `--verify` proves the rewrite is behaviour-preserving (U9) and never
/// writes; `--to` older than `--from` is refused (U10).
#[test]
fn spec_upgrade_verifies_and_refuses_downgrades() {
    let tree = Tree::new("upgrade-verify");
    let pack = tree.path().join("demo.tclspec");
    let source = "speclib demo 1.2 {\n\
                  \x20 command demo::greet {\n\
                  \x20   arity 1\n\
                  \x20   dialects tcl8.x\n\
                  \x20 }\n\
                  }\n";
    std::fs::write(&pack, source).expect("write pack");
    let path = pack.to_string_lossy().into_owned();

    let (stdout, stderr, code) = run(&["spec", "upgrade", "--verify", &path]);
    assert_eq!(code, 0, "{stdout}{stderr}");
    assert!(stdout.contains("byte-identical"), "{stdout}");
    assert_eq!(
        std::fs::read_to_string(&pack).expect("read back"),
        source,
        "--verify writes nothing"
    );

    let (_stdout, stderr, code) = run(&["spec", "upgrade", "--from", "2.0", "--to", "1.2", &path]);
    assert_ne!(code, 0, "a downgrade must not succeed");
    assert!(stderr.contains("refusing to downgrade"), "{stderr}");
}

/// An environment-membership token whose environment declares **no**
/// ambient package provider (`spectcl` — its surface is compiled) is
/// left byte-identical, marked, and the file reports partial; a token
/// whose environment does declare one (`f5-iapps`) translates for real
/// through the live registry.
#[test]
fn spec_upgrade_defers_environment_membership_tokens() {
    let tree = Tree::new("upgrade-partial");
    let pack = tree.path().join("demo.tclspec");
    std::fs::write(
        &pack,
        "speclib demo 1.2 {\n command demo::greet {\n arity 1\n \
         dialects {tcl8.6 spectcl}\n }\n}\n",
    )
    .expect("write pack");
    let path = pack.to_string_lossy().into_owned();

    let (stdout, _stderr, code) = run(&["spec", "upgrade", &path]);
    assert_ne!(code, 0, "a partial upgrade exits non-zero: {stdout}");
    assert!(stdout.contains("partially upgraded"), "{stdout}");
    let written = std::fs::read_to_string(&pack).expect("read back");
    assert!(written.contains("# TODO(spectcl 2.0):"), "{written}");
    assert!(written.contains("speclib demo 1.2"), "{written}");
    assert!(written.contains("dialects {tcl8.6 spectcl}"), "{written}");

    // The ambient-provider half really translates (U3): the row becomes
    // the environment's own package claim and the header moves to 2.0.
    let full = tree.path().join("full.tclspec");
    std::fs::write(
        &full,
        "speclib demo 1.2 {\n command demo::greet {\n arity 1\n \
         dialects {tcl8.6 f5-iapps}\n }\n}\n",
    )
    .expect("write pack");
    let path = full.to_string_lossy().into_owned();
    let (stdout, stderr, code) = run(&["spec", "upgrade", &path]);
    assert_eq!(code, 0, "{stdout}{stderr}");
    let written = std::fs::read_to_string(&full).expect("read back");
    assert!(
        written.contains("available {tcl 8.6} {package f5-iapps-cmds}"),
        "{written}"
    );
    assert!(written.contains("speclib demo 2.0"), "{written}");
}
