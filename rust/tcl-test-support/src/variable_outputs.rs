// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic native variable-output lifecycle observations shared by engines.

/// Observe each output write and an earlier write trace retargeting a later name.
/// These scripts deliberately exercise actual procedure local slots, aliases,
/// and native compiler selection; a generic catch alias is a separate control.
#[must_use]
pub fn variable_output_lookup_scripts(version: tcl_dialect::TclVersion) -> Vec<String> {
    let scan = "namespace eval static {}; set ::static::target OLD; proc onwrite {args} {uplevel 1 {unset second; upvar 0 ::static::target second}}; proc P {} {set first OLD; set second OLD; trace add variable first write onwrite; scan {A B} {%s %s} first second; list $first $second $::static::target}; P";
    let mut cases = vec![scan.to_owned()];
    if version >= tcl_dialect::TclVersion::V8_5 {
        for invocation in ["catch", "cap"] {
            cases.push(format!("set ::order {{}}; proc obs {{kind args}} {{lappend ::order $kind}}; interp alias {{}} cap {{}} catch; proc P {{}} {{set result OLD; set opts OLD; trace add variable result write {{obs result}}; trace add variable opts write {{obs opts}}; {invocation} {{set value OK}} result opts; list $::order $result $opts}}; P"));
            cases.push(format!("namespace eval static {{}}; set ::static::target OLD; proc onwrite {{args}} {{uplevel 1 {{unset opts; upvar 0 ::static::target opts}}}}; interp alias {{}} cap {{}} catch; proc P {{}} {{set result OLD; set opts OLD; trace add variable result write onwrite; {invocation} {{set value OK}} result opts; list $result $opts $::static::target}}; P"));
        }
    }
    cases
}
