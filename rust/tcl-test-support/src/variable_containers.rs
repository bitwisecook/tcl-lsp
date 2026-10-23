// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Fixed native storage specimens shared by C/Jim oracles and both Rust engines.

/// Each script returns one Tcl list; cases are isolated interpreter executions.
/// These exercise storage through ordinary variables, aliases and dictionary
/// body mappings, rather than a command-specific array replacement shortcut.
#[must_use]
pub fn variable_container_scripts() -> &'static [&'static str] {
    &[
        "array set a {k OLD};set c [catch {set a} r];list $c $r",
        "array set a {k OLD};set c [catch {set a NEW} r];list $c $r [array exists a]",
        "set a {k OLD};set c [catch {set a(k) NEW} r];list $c $r [array exists a]",
        "array set a {k OLD};set c [catch {set copy $a} r];set a(k) NEW;list $c $r [info exists copy]",
        "array set a {k OLD};upvar 0 a root;set c [catch {set root NEW} r];list $c $r [array exists a]",
        "set a OTHER;list [catch {array names a}] [catch {array get a}] [array exists a]",
        "set a OTHER;array unset a;info exists a",
        "set a {k OLD};set c [catch {array set a {other NEW}} r];list $c $r [catch {set a(k)} v] $v",
        "if {[llength [info commands dict]] == 0} {list UNAVAILABLE} else {array set blocked {k OLD};set d {first NEW second OTHER};set ok BEFORE;set entered 0;set c [catch {dict update d first ok second blocked {set entered 1}} r];list $c $ok $entered [catch {set blocked} v] $v}",
        "proc p {} {array set a {k OLD};upvar 0 a root;set c [catch {set root NEW} r];list $c $r [array exists a]};p",
    ]
}

/// Output recorded independently from the pinned C8.4–9.1/Jim0.84 engines.
/// Unknown C release deliberately has no expectation column.
#[must_use]
pub fn variable_container_expectations(
    model: tcl_dialect::VariableContainerModel,
    version: Option<tcl_dialect::TclVersion>,
) -> Option<&'static str> {
    use tcl_dialect::{TclVersion, VariableContainerModel};
    Some(match model {
        VariableContainerModel::DictionaryValue => {
            include_str!("../tests/data/variable_containers/jim.txt")
        }
        VariableContainerModel::DistinctArray => match version? {
            TclVersion::V8_4 => include_str!("../tests/data/variable_containers/8.4.txt"),
            TclVersion::V8_5 => include_str!("../tests/data/variable_containers/8.5.txt"),
            TclVersion::V8_6 => include_str!("../tests/data/variable_containers/8.6.txt"),
            TclVersion::V9_0 => include_str!("../tests/data/variable_containers/9.0.txt"),
            TclVersion::V9_1 => include_str!("../tests/data/variable_containers/9.1.txt"),
        },
    })
}
