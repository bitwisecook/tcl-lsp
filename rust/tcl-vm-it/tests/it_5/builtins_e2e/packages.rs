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

//! `package`: require, loaders, `package unknown`, version preference, circular loaders and the bad-result completions.

use super::*;

#[test]
fn package_stubs() {
    out_eq("puts [package require Tcl 8.5-]\n", "9.0.4\n");
    out_eq("puts [package vsatisfies 9.0 9.0-]\n", "1\n");
    out_eq("puts [package vsatisfies 8.4 8.5-]\n", "0\n");
    out_eq(
        "package provide Foo 1.2\nputs [package require Foo]\n",
        "1.2\n",
    );
}

/// An already-provided incompatible version is terminal. Discovery must not
/// run a loader or the unknown callback in an attempt to replace it.
#[test]
fn package_require_rejects_a_provided_conflict_before_discovery() {
    out_eq(
        "set events {}\n\
         package provide P 2.0\n\
         package ifneeded P 1.0 {lappend ::events loader; package provide P 1.0}\n\
         proc discover {name args} {lappend ::events unknown}\n\
         package unknown discover\n\
         catch {package require -exact P 1.0} message options\n\
         puts [list $message [dict get $options -errorcode] $events]\n",
        "{version conflict for package \"P\": have 2.0, need exactly 1.0} {TCL PACKAGE VERSIONCONFLICT} {}\n",
    );
}

/// A known satisfying ifneeded entry is the normal fast path. Inverting these
/// two stages makes the deliberately-failing unknown callback win instead.
#[test]
fn package_require_uses_a_registered_loader_before_unknown() {
    out_eq(
        "set events {}\n\
         package ifneeded P 1.0 {lappend ::events loader; package provide P 1.0}\n\
         proc discover {name args} {lappend ::events unknown; error UNKNOWN-RAN}\n\
         package unknown discover\n\
         puts [list [package require P] $events]\n",
        "1.0 loader\n",
    );
}

/// Both package scripts run at level #0 even when the require originates in a
/// namespaced proc. The unqualified callback also resolves in the global
/// namespace, guarding the frame transition as well as variable visibility.
#[test]
fn package_unknown_and_loader_scripts_run_in_global_scope() {
    out_eq(
        "set events {}\n\
         proc discover {name args} {\n\
             lappend ::events unknown-global\n\
             set ::unknownScope [list [namespace current] [info level] [info exists localOnly]]\n\
             package ifneeded $name 1.0 {\n\
                 set ::loaderScope [list [namespace current] [info level] [info exists localOnly]]\n\
                 package provide P 1.0\n\
             }\n\
         }\n\
         namespace eval N {\n\
             proc discover {name args} {lappend ::events unknown-namespaced}\n\
             proc run {} {set localOnly yes; package require P}\n\
         }\n\
         package unknown discover\n\
         puts [list [N::run] $events $unknownScope $loaderScope]\n",
        "1.0 unknown-global {:: 1 0} {:: 0 0}\n",
    );
}

/// Tcl defaults to preferring a stable release over a newer prerelease. The
/// explicit stable request is a no-op and reports the resulting state.
#[test]
fn package_prefer_defaults_to_stable_selection() {
    out_eq(
        "set initial [package prefer]\n\
         set requested [package prefer stable]\n\
         package ifneeded P 1.2 {package provide P 1.2}\n\
         package ifneeded P 1.3b1 {package provide P 1.3b1}\n\
         puts [list $initial $requested [package require P]]\n",
        "stable stable 1.2\n",
    );
}

/// `latest` is a one-way interpreter latch: a later stable request succeeds
/// but does not lower the policy, and the prerelease is then selected.
#[test]
fn package_prefer_latest_is_stateful_and_sticky() {
    out_eq(
        "set raised [package prefer latest]\n\
         set lowered [package prefer stable]\n\
         package ifneeded P 1.2 {package provide P 1.2}\n\
         package ifneeded P 1.3b1 {package provide P 1.3b1}\n\
         puts [list $raised $lowered [package prefer] [package require P]]\n",
        "latest latest latest 1.3b1\n",
    );
}

/// The selected ifneeded version is the contract the loader must fulfil.
/// Numerically-equivalent spellings are accepted, while a genuinely different
/// provide receives Tcl's dedicated WRONGPROVIDE result.
#[test]
fn package_loader_must_provide_its_selected_version() {
    out_eq(
        "package ifneeded Equivalent 1.0 {package provide Equivalent 1.0.0}\n\
         set equivalent [package require Equivalent]\n\
         package ifneeded Wrong 1.0 {package provide Wrong 2.0}\n\
         catch {package require Wrong} message options\n\
         puts [list $equivalent $message [dict get $options -errorcode]]\n",
        "1.0.0 {attempt to provide package Wrong 1.0 failed: package Wrong 2.0 provided instead} {TCL PACKAGE WRONGPROVIDE}\n",
    );
}

/// `SelectPackageFinal` names the loader in `errorInfo` for every failure,
/// including the two it raises itself, and withdraws a wrong provide
/// (tclsh 8.5.19 through 9.1.0 agree).
#[test]
fn package_loader_generated_failures_name_the_loader_in_error_info() {
    out_eq(
        r"package ifneeded Unprovided 1.0 {}
catch {package require Unprovided} message options
puts [dict get $options -errorinfo]
package ifneeded Wrong 1.0 {package provide Wrong 2.0}
catch {package require Wrong} message options
puts [list [dict get $options -errorinfo] [package provide Wrong]]
",
        r#"attempt to provide package Unprovided 1.0 failed: no version of package Unprovided provided
    ("package ifneeded Unprovided 1.0" script)
    invoked from within
"package require Unprovided"
{attempt to provide package Wrong 1.0 failed: package Wrong 2.0 provided instead
    ("package ifneeded Wrong 1.0" script)
    invoked from within
"package require Wrong"} {}
"#,
    );
}

/// Package command version arguments use the dialect parser's checked seam,
/// and providing a second, different version preserves Tcl's package state.
#[test]
fn package_version_arguments_match_tcl_validation_and_conflicts() {
    out_eq(
        r"catch {package provide Invalid a.b} message options
puts [list $message [dict get $options -errorcode]]
catch {package ifneeded Invalid xyz {}} message options
puts [list $message [dict get $options -errorcode]]
catch {package require Invalid 2.1-3.2-4.5} message options
puts [list $message [dict get $options -errorcode]]
catch {package vcompare x.y 3.4} message options
puts [list $message [dict get $options -errorcode]]
catch {package vsatisfies 2.1 3.2-x.y} message options
puts [list $message [dict get $options -errorcode]]
package provide Stable 2.3
set same [package provide Stable 2.3.0]
catch {package provide Stable 2.2} message options
puts [list $same $message [dict get $options -errorcode] [package provide Stable]]
",
        r#"{expected version number but got "a.b"} {TCL VALUE VERSION}
{expected version number but got "xyz"} {TCL VALUE VERSION}
{expected versionMin-versionMax but got "2.1-3.2-4.5"} {TCL VALUE VERSIONRANGE}
{expected version number but got "x.y"} {TCL VALUE VERSION}
{expected version number but got "x.y"} {TCL VALUE VERSION}
{} {conflicting versions provided for package "Stable": 2.3, then 2.2} {TCL PACKAGE VERSIONCONFLICT} 2.3
"#,
    );
}

/// Tcl marks the selected ifneeded name/version while its script is running.
/// A nested require of the same package is circular even when it requests a
/// different version, and a mutual dependency reports the original package
/// being provided. The marker must carry Tcl's dedicated error code.
#[test]
fn package_require_detects_circular_ifneeded_loaders() {
    out_eq(
        r"package forget foo
package ifneeded foo 1 {package require foo 1}
catch {package require foo 1} message options
puts [list $message [dict get $options -errorcode]]

package forget foo
package ifneeded foo 1 {package require foo 2}
catch {package require foo 1} message options
puts [list $message [dict get $options -errorcode]]

package forget foo
package ifneeded foo 1 {package require -exact foo 1}
catch {package require foo 1} message options
puts [list $message [dict get $options -errorcode]]

package forget foo
package forget bar
package ifneeded foo 1 {package require bar 1; package provide foo 1}
package ifneeded bar 1 {package require foo 1; package provide bar 1}
catch {package require foo 1} message options
puts [list $message [dict get $options -errorcode]]

package forget foo
package forget bar
package ifneeded foo 1 {package require bar 1; package provide foo 1}
package ifneeded foo 2 {package provide foo 2}
package ifneeded bar 1 {package require foo 2; package provide bar 1}
catch {package require foo 1} message options
puts [list $message [dict get $options -errorcode]]
",
        r"{circular package dependency: attempt to provide foo 1 requires foo 1} {TCL PACKAGE CIRCULARITY}
{circular package dependency: attempt to provide foo 1 requires foo 2} {TCL PACKAGE CIRCULARITY}
{circular package dependency: attempt to provide foo 1 requires foo exactly 1} {TCL PACKAGE CIRCULARITY}
{circular package dependency: attempt to provide foo 1 requires foo 1} {TCL PACKAGE CIRCULARITY}
{circular package dependency: attempt to provide foo 1 requires foo 2} {TCL PACKAGE CIRCULARITY}
",
    );
}

/// A loader may provide its package before requiring it recursively. That
/// nested require sees the provided version and succeeds; only an absent
/// package is circular.
#[test]
fn package_require_allows_nested_require_after_provide() {
    out_eq(
        r"package forget P
package ifneeded P 1 {package provide P 1; package require P 1}
puts [list [package require P 1] [package provide P]]
",
        "1 1\n",
    );
}

/// A failed loader clears the transient provided version and its active
/// circular marker, while retaining the ifneeded registration for a retry.
#[test]
fn package_require_loader_failure_can_be_retried() {
    out_eq(
        r"package forget P
package ifneeded P 1 {package provide P 1; error boom}
catch {package require P 1} message options
set afterFailure [package provide P]
package ifneeded P 1 {package provide P 1}
puts [list $afterFailure [package require P 1] [package provide P]]
",
        "{} 1 1\n",
    );
}

#[test]
fn package_ifneeded_non_error_completions_become_badresult_and_can_be_retried() {
    out_eq(
        r#"foreach {kind script} {break {break} continue {continue} return {return} custom {return -level 0 -code 10}} {
    package forget foo
    package ifneeded foo 1 "package provide foo 1; $script"
    set status [catch {package require foo 1} message options]
    puts [list $kind $status $message [dict get $options -errorcode] [string match {* ("package ifneeded foo 1" script)*} [dict get $options -errorinfo]] [package provide foo] [expr {[package ifneeded foo 1] eq "package provide foo 1; $script"}]]
    package ifneeded foo 1 {package provide foo 1}
    puts [list retry [package require foo 1]]
}
"#,
        "break 1 {attempt to provide package foo 1 failed: bad return code: 3} {TCL PACKAGE BADRESULT} 1 {} 1\n\
retry 1\n\
continue 1 {attempt to provide package foo 1 failed: bad return code: 4} {TCL PACKAGE BADRESULT} 1 {} 1\n\
retry 1\n\
return 1 {attempt to provide package foo 1 failed: bad return code: 2} {TCL PACKAGE BADRESULT} 1 {} 1\n\
retry 1\n\
custom 1 {attempt to provide package foo 1 failed: bad return code: 10} {TCL PACKAGE BADRESULT} 1 {} 1\n\
retry 1\n",
    );
}

#[test]
fn package_unknown_non_error_completions_become_badresult() {
    out_eq(
        r#"set previous [package unknown]
foreach {kind script} {break {break} continue {continue} return {return} custom {return -level 0 -code 10}} {
    package forget foo
    package unknown "$script ;#"
    set status [catch {package require foo 1} message options]
    puts [list $kind $status $message [dict get $options -errorcode] [string match {* ("package unknown" script)*} [dict get $options -errorinfo]]]
}
package unknown {package ifneeded foo 1 {package provide foo 1};#}
puts [list retry [package require foo 1]]
package unknown $previous
"#,
        "break 1 {bad return code: 3} {TCL PACKAGE BADRESULT} 1\n\
continue 1 {bad return code: 4} {TCL PACKAGE BADRESULT} 1\n\
return 1 {bad return code: 2} {TCL PACKAGE BADRESULT} 1\n\
custom 1 {bad return code: 10} {TCL PACKAGE BADRESULT} 1\n\
retry 1\n",
    );
}

#[test]
fn package_discovery_preserves_ordinary_script_errors() {
    out_eq(
        r#"package forget foo bar
package ifneeded foo 1 {error boom LOAD {LOADER ORIGINAL}}
set status [catch {package require foo 1} message options]
puts [list ifneeded $status $message [dict get $options -errorcode] [string match {LOAD*} [dict get $options -errorinfo]] [string match {* ("package ifneeded foo 1" script)*} [dict get $options -errorinfo]] [package provide foo]]
set previous [package unknown]
package unknown {error boom UNKNOWN {UNKNOWN ORIGINAL};#}
set status [catch {package require bar 1} message options]
puts [list unknown $status $message [dict get $options -errorcode] [string match {UNKNOWN*} [dict get $options -errorinfo]] [string match {* ("package unknown" script)*} [dict get $options -errorinfo]]]
package unknown $previous
"#,
        "ifneeded 1 boom {LOADER ORIGINAL} 1 1 {}\nunknown 1 boom {UNKNOWN ORIGINAL} 1 1\n",
    );
}

#[test]
fn package_badresult_seeds_explicitly_empty_error_info() {
    // Unmodified Tcl 9.0.4 seeds the generated BADRESULT message before the
    // loader frame even when return carries an explicitly empty -errorinfo.
    out_eq(
        r"package ifneeded foo 1 {return -code error -errorinfo {} boom}
catch {package require foo 1} message options
puts [list $message [dict get $options -errorinfo] [dict get $options -errorcode]]
package unknown {return -code error -errorinfo {} boom;#}
catch {package require bar 1} message options
puts [list $message [dict get $options -errorinfo] [dict get $options -errorcode]]
",
        concat!(
            "{attempt to provide package foo 1 failed: bad return code: 2} ",
            "{attempt to provide package foo 1 failed: bad return code: 2\n",
            "    (\"package ifneeded foo 1\" script)\n",
            "    invoked from within\n\"package require foo 1\"} {TCL PACKAGE BADRESULT}\n",
            "{bad return code: 2} {bad return code: 2\n",
            "    (\"package unknown\" script)\n",
            "    invoked from within\n\"package require bar 1\"} {TCL PACKAGE BADRESULT}\n",
        ),
    );
}

#[test]
fn package_badresult_resets_non_error_traces_and_preserves_carried_error_info() {
    out_eq(
        r#"set previous [package unknown]
foreach {kind script carried} {
    break {break} {}
    continue {continue} {}
    return {return} {}
    custom {return -level 0 -code 10} {}
    pendingError {return -code error boom} {}
    nonErrorInfo {return -errorinfo EXPLICIT boom} {}
    errorInfo {return -code error -errorinfo EXPLICIT boom} EXPLICIT
} {
    package forget foo bar
    package ifneeded foo 1 $script
    catch {package require foo 1} message options
    set prefix [expr {$carried eq "" ? $message : $carried}]
    set expected [format "%s\n    (\"package ifneeded foo 1\" script)\n    invoked from within\n\"package require foo 1\"" $prefix]
    puts [list ifneeded $kind [expr {[dict get $options -errorinfo] eq $expected}]]
    package unknown "$script ;#"
    catch {package require bar 1} message options
    set prefix [expr {$carried eq "" ? $message : $carried}]
    set expected [format "%s\n    (\"package unknown\" script)\n    invoked from within\n\"package require bar 1\"" $prefix]
    puts [list unknown $kind [expr {[dict get $options -errorinfo] eq $expected}]]
}

package unknown $previous
"#,
        concat!(
            "ifneeded break 1\nunknown break 1\n",
            "ifneeded continue 1\nunknown continue 1\n",
            "ifneeded return 1\nunknown return 1\n",
            "ifneeded custom 1\nunknown custom 1\n",
            "ifneeded pendingError 1\nunknown pendingError 1\n",
            "ifneeded nonErrorInfo 1\nunknown nonErrorInfo 1\n",
            "ifneeded errorInfo 1\nunknown errorInfo 1\n",
        ),
    );
}

#[test]
fn package_badresult_preserves_ancillary_options_and_pending_error_stack() {
    use std::fmt::Write as _;
    // Tcl 9.0.4: custom options survive both discovery paths, while only a
    // pending error return retains its explicit error stack.
    for (script, pending) in [
        (
            "return -level 0 -code break -foo BAR -errorinfo OLD -errorcode OLD",
            false,
        ),
        (
            "return -level 0 -code 10 -foo BAR -errorinfo OLD -errorcode OLD",
            false,
        ),
        (
            "return -code error -errorinfo EXPLICIT -errorstack {INNER {invokeStk1 boom}} -foo BAR boom",
            true,
        ),
        (
            "return -foo BAR -errorinfo OLD -errorstack {INNER {invokeStk1 boom}}",
            false,
        ),
    ] {
        for package in ["foo", "bar"] {
            let mut source = format!(
                "package forget foo bar\npackage ifneeded foo 1 {{{script}}}\npackage unknown {{{script} ;#}}\ncatch {{package require {package} 1}} message options\n"
            );
            if pending {
                source.push_str("set expected {INNER {invokeStk1 boom}}\nset prefix EXPLICIT\nset stackOK [expr {[dict get $options -errorstack] eq $expected}]\n");
            } else {
                writeln!(source, "set expected {{INNER {{invokeStk1 package require {package} 1}}}}\nset prefix $message\nset stackOK [string match {{*package require {package} 1}} [lindex [dict get $options -errorstack] 1]]").unwrap();
            }
            source.push_str(r#"puts [list [dict get $options -foo] [dict get $options -code] [dict get $options -level] [dict get $options -errorcode] $stackOK [string match "$prefix\n*" [dict get $options -errorinfo]]]
"#);
            out_eq(&source, "BAR 1 0 {TCL PACKAGE BADRESULT} 1 1\n");
        }
    }
}

/// `package forget` removes the active package record, including a loader's
/// circular marker. A loader can therefore replace its record and require the
/// replacement before the outer loader returns.
#[test]
fn package_forget_clears_active_loader_marker() {
    out_eq(
        r"package forget P
package ifneeded P 1 {package forget P; package ifneeded P 1 {package provide P 1}; package require P 1}
puts [list [package require P 1] [package provide P]]
",
        "1 1\n",
    );
}
