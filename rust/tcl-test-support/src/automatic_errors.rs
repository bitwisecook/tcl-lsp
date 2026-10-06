// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Fixed automatic error-stack scripts shared by native runtime adapters.

/// Jim's automatic stack records actual argv and evaluation nesting. Each
/// specimen ends with a value suitable for direct comparison with Jim.
pub const JIM_AUTOMATIC_ERROR_CASES: &[(&str, &str)] = &[
    (
        "root",
        "catch {error BOOM} r o; list $r [dict get $o -errorinfo] [dict exists $o -errorline] [dict exists $o -errorstack]",
    ),
    (
        "procedure",
        "proc fail {} {error BOOM}\ncatch {fail} r o; list $r [dict get $o -errorinfo]",
    ),
    (
        "nested",
        "proc inner {x} {error $x}\nproc outer {} {inner BOOM}\ncatch {outer} r o; list $r [dict get $o -errorinfo]",
    ),
    (
        "eval",
        "proc fail {} {eval {error BOOM}}\ncatch {fail} r o; list $r [dict get $o -errorinfo]",
    ),
    (
        "uplevel",
        "proc inner {} {uplevel 1 {error BOOM}}\nproc outer {} {inner}\ncatch {outer} r o; list $r [dict get $o -errorinfo]",
    ),
    (
        "rename",
        "proc fail {} {error BOOM}\nrename fail saved\ncatch {saved} r o; list $r [dict get $o -errorinfo]",
    ),
    (
        "tailcall",
        "proc target {} {error BOOM}\nproc issuer {} {tailcall target}\ncatch {issuer} r o; list $r [dict get $o -errorinfo]",
    ),
    (
        "explicit",
        "catch {error BOOM {P file 3}} r o; list $r [dict get $o -errorinfo]",
    ),
    (
        "return",
        "proc fail {} {return -code error BOOM}\ncatch {fail} r o; list $r [dict get $o -errorinfo]",
    ),
    (
        "new-catch",
        "catch {error FIRST}; catch {error SECOND} r o; list $r [dict get $o -errorinfo]",
    ),
    (
        "substitution",
        "proc fail {} {set x [error BOOM]}\ncatch {fail} r o; list $r [dict get $o -errorinfo]",
    ),
    (
        "missing",
        "proc fail {} {missingCommand BOOM}\ncatch {fail} r o; list $r [dict get $o -errorinfo]",
    ),
    (
        "wrong-args",
        "proc fail {x} {error NEVER}\ncatch {fail} r o; dict get $o -errorinfo",
    ),
    (
        "namespace",
        "namespace eval N {proc fail {} {error BOOM}}\ncatch {N::fail} r o; list $r [dict get $o -errorinfo]",
    ),
    (
        "cached",
        "catch {error BOOM} r o; set saved [dict get $o -errorinfo]; list $saved [info stacktrace]",
    ),
    (
        "body-location",
        "\n\nproc fail {} {\nerror BOOM\n}\ncatch {fail} r o; list $r [dict get $o -errorinfo]",
    ),
    (
        "parse-root",
        "set bad {set x \"}; catch {eval $bad} r o; list [dict get $o -errorinfo]",
    ),
    (
        "parse-procedure",
        "proc fail {} {set x \"}; catch {fail} r o; list [dict get $o -errorinfo]",
    ),
    (
        "parse-before-prefix-effects",
        "proc fail {} {set ::before 1;set x \"}; catch {fail} r o; list [info exists ::before] [dict get $o -errorinfo]",
    ),
    (
        "parse-after-caught-error",
        "catch {error OLD}; set bad {set x \"}; catch {eval $bad} r o; list [dict get $o -errorinfo]",
    ),
];
