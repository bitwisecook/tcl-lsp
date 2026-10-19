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

//! `package require` inside a coroutine: its `package unknown` callback and
//! selected `package ifneeded` loader run as explicit-stack activations at
//! level #0, so a `yield` inside either suspends the coroutine and a resume
//! finishes the require (#2200).
//!
//! Every expectation is the same script's result under tclsh 8.6.18, 9.0.4
//! and 9.1.0, which agree on all of them; each script runs under each of
//! those pins.

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::TclVersion;
use tcl_vm::{CompileService, Vm};

const COROUTINE_RELEASES: [TclVersion; 3] = [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1];

/// The result of `src` under the `version` pin; panics if it errors.
fn result_for(version: TclVersion, src: &str) -> String {
    let profile = tcl_registry::model::ingress::resolve_environment(version.dialect_name())
        .analyser_profile();
    let service = BytecodeCompileService::for_profile(profile);
    let asm = service
        .compile_for_profile(src, profile)
        .expect("test script compiles for its selected profile");
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(service));
    vm.set_runtime_version(version);
    let completion = vm.run_module(&asm);
    assert!(
        completion.code.is_ok(),
        "{version:?}: script errored: {}",
        completion.result.to_str()
    );
    completion.result.to_str().to_string()
}

fn assert_every_release(src: &str, expected: &str) {
    for version in COROUTINE_RELEASES {
        assert_eq!(result_for(version, src), expected, "{version:?}");
    }
}

/// package-3.54: the loader's `yield` suspends the coroutine (creation
/// returns the empty yield value) and the resume completes the require.
#[test]
fn yield_inside_ifneeded_loader_suspends_and_resumes_the_require() {
    assert_every_release(
        r"package forget t
set created [coroutine coro1 apply {{} {
    package ifneeded t 2.1 {
        yield
        package provide t 2.1
    }
    package require t 2.1
}}]
list $created [catch {coro1} msg] $msg [package provide t] [info commands coro1]",
        "{} 0 2.1 2.1 {}",
    );
}

/// The `package unknown` callback is the require's other discovery script;
/// it suspends the same way, and the resume value reaches its `yield`.
#[test]
fn yield_inside_package_unknown_callback_suspends_the_require() {
    assert_every_release(
        r"package forget u
set previous [package unknown]
package unknown {apply {{name args} {
    package ifneeded $name 3.0 [list package provide $name 3.0]
    set ::got [yield unk-yield]
}}}
set created [coroutine coro2 apply {{} {package require u}}]
set resumed [coro2 resumed]
package unknown $previous
list $created $resumed $::got",
        "unk-yield 3.0 resumed",
    );
}

/// The loader runs at level #0 in the global namespace even when the require
/// came from a namespace proc inside a coroutine, before and after the
/// suspension; its variables are globals, and the caller's frame is intact.
#[test]
fn suspended_loader_runs_at_level_zero_and_restores_the_caller() {
    assert_every_release(
        r"package forget pc
package ifneeded pc 1.0 {
    set ::lv [list [info level] [namespace current] [info coroutine]]
    set gvar [yield]
    lappend ::lv [info level] [namespace current] [info coroutine]
    package provide pc 1.0
}
namespace eval ns {
    proc go {} {
        set gvar local
        list [package require pc] $gvar [info level] [namespace current]
    }
}
set created [coroutine cC ns::go]
list $created [cC global] $::lv $::gvar",
        "{} {1.0 local 1 ::ns} {0 :: ::cC 0 :: ::cC} global",
    );
}

/// The resumed loader's completion goes through the same post-processing as
/// a synchronous one: an error keeps its code and leaves the ifneeded script
/// registered, and non-ok codes, a wrong provide, and no provide all fail
/// with the loader's own diagnostic.
#[test]
fn resumed_loader_completion_is_checked_like_a_synchronous_one() {
    assert_every_release(
        r"set out {}
foreach {name script} {
    e {yield y; error boom}
    w {yield y; package provide w 2.0}
    r {yield y; package provide r 1.0; return ignored}
    b {yield y; break}
    n {yield y}
} {
    package forget $name
    package ifneeded $name 1.0 $script
    set created [coroutine coro apply {{name} {package require $name 1.0}} $name]
    set code [catch {coro} msg opts]
    lappend out [list $created $code $msg [dict get $opts -errorcode] \
        [package provide $name] [expr {[package ifneeded $name 1.0] eq $script}]]
}
join $out \n",
        "y 1 boom NONE {} 1\n\
y 1 {attempt to provide package w 1.0 failed: package w 2.0 provided instead} {TCL PACKAGE WRONGPROVIDE} {} 1\n\
y 1 {attempt to provide package r 1.0 failed: bad return code: 2} {TCL PACKAGE BADRESULT} {} 1\n\
y 1 {attempt to provide package b 1.0 failed: bad return code: 3} {TCL PACKAGE BADRESULT} {} 1\n\
y 1 {attempt to provide package n 1.0 failed: no version of package n provided} {TCL PACKAGE UNPROVIDED} {} 1",
    );
}

/// The selected loader stays marked across the suspension: requiring the
/// package from the main flow, or again from inside its own resumed loader,
/// is the circular dependency C reports, and the parked loader still
/// completes afterwards.
#[test]
fn circular_dependency_marker_survives_suspension() {
    assert_every_release(
        r"package forget c s
package ifneeded c 1.0 {yield y8; package provide c 1.0}
set created [coroutine coro8 apply {{} {package require c}}]
catch {package require c 1.0} cycle opts
set resumed [coro8]
package ifneeded s 1.0 {yield y12; package require s; package provide s 1.0}
coroutine coro12 apply {{} {package require s}}
catch {coro12} self
list $created [list $cycle [dict get $opts -errorcode]] $resumed $self",
        "y8 {{circular package dependency: attempt to provide c 1.0 requires c 1.0} {TCL PACKAGE CIRCULARITY}} 1.0 \
{circular package dependency: attempt to provide s 1.0 requires s}",
    );
}

/// A nested require inside a loader suspends the whole chain; each resume
/// moves one loader on and both packages end up provided.
#[test]
fn nested_require_inside_a_suspending_loader() {
    assert_every_release(
        r"package forget outer inner
package ifneeded inner 1.0 {yield y5inner; package provide inner 1.0}
package ifneeded outer 1.0 {
    package require inner
    yield y5outer
    package provide outer 1.0
}
set created [coroutine coro5 apply {{} {package require outer}}]
list $created [coro5] [coro5] [package provide inner] [package provide outer]",
        "y5inner y5outer 1.0 1.0 1.0",
    );
}

/// Loaders can finish out of order: a main-flow loader resumes a coroutine
/// parked in another package's loader. Each completion retires only its own
/// marker.
#[test]
fn loaders_completing_out_of_order_retire_their_own_markers() {
    assert_every_release(
        r"package forget pj1 pj2
package ifneeded pj1 1.0 {yield j1; package provide pj1 1.0}
package ifneeded pj2 1.0 {set ::r [cJ1]; package provide pj2 1.0}
set created [coroutine cJ1 apply {{} {package require pj1}}]
list $created [package require pj2] $::r [package provide pj1] [package provide pj2]",
        "j1 1.0 1.0 1.0 1.0",
    );
}

/// `yieldto` and a `catch` around the `yield` work inside a loader too.
#[test]
fn yieldto_and_caught_yield_inside_a_loader() {
    assert_every_release(
        r"package forget pk pl
package ifneeded pk 1.0 {yieldto string cat kk; package provide pk 1.0}
package ifneeded pl 1.0 {catch {yield ly}; package provide pl 1.0}
list [coroutine cK apply {{} {package require pk}}] [cK] \
    [coroutine cL apply {{} {package require pl}}] [cL]",
        "kk 1.0 ly 1.0",
    );
}

/// Deleting a coroutine parked in a loader unwinds the require as C does:
/// the version the loader already provided is withdrawn and its marker is
/// retired, so a later require loads normally instead of reporting a
/// circular dependency. The same holds for a coroutine parked in the
/// `package unknown` callback, and for one replaced by a new coroutine of
/// the same name.
#[test]
fn deleting_a_suspended_coroutine_retires_its_require() {
    assert_every_release(
        r"package forget d
package ifneeded d 1.0 {package provide d 1.0; yield y4}
set created [coroutine coro4 apply {{} {package require d}}]
set during [package provide d]
rename coro4 {}
set after [package provide d]
package ifneeded d 1.0 {package provide d 1.0}
set again [package require d]

package forget ph
set previous [package unknown]
package unknown {apply {{name args} {
    yield hy
    package ifneeded $name 1.0 [list package provide $name 1.0]
}}}
set unknownCreated [coroutine cH apply {{} {package require ph}}]
rename cH {}
package unknown {apply {{name args} {
    package ifneeded $name 1.0 [list package provide $name 1.0]
}}}
set unknownAgain [package require ph]
package unknown $previous

package forget rp
package ifneeded rp 1.0 {yield r1; package provide rp 1.0}
set replaced [list [coroutine crp apply {{} {package require rp}}] \
    [coroutine crp apply {{} {package require rp}}] [crp]]
list $created $during $after $again $unknownCreated $unknownAgain $replaced",
        "y4 1.0 {} 1.0 hy 1.0 {r1 r1 1.0}",
    );
}

/// A loader that deletes its own coroutine and then yields cannot park: the
/// require is retired (no false circular dependency) and a version it had
/// already provided is withdrawn.
#[test]
fn self_deleting_coroutine_yield_retires_its_require() {
    assert_every_release(
        r"package forget sd
package ifneeded sd 1.0 {
    if {[info coroutine] ne {}} {
        rename [info coroutine] {}
        yield sy
    }
    package provide sd 1.0
}
set created [coroutine csd apply {{} {package require sd}}]
set gone [info commands csd]
set again [package require sd]

package forget sp
package ifneeded sp 1.0 {
    package provide sp 1.0
    if {[info coroutine] ne {}} {
        rename [info coroutine] {}
        yield py
    }
}
set pcreated [coroutine csp apply {{} {package require sp}}]
list $created $gone $again $pcreated [package provide sp]",
        "sy {} 1.0 py {}",
    );
}

/// A failing `coroinject` command unwinds the coroutine through its parked
/// loader (9.0+; tclsh 9.0.4 and 9.1.0 agree): the error escapes, the version
/// the loader provided before suspending is withdrawn, and a later require
/// is not circular.
#[test]
fn failed_injection_unwinds_a_coroutine_parked_in_a_loader() {
    let src = r"package forget pi
package ifneeded pi 1.0 {package provide pi 1.0; yield y; package provide pi 1.0}
set created [coroutine ci apply {{} {package require pi}}]
coroinject ci apply {{args} {error injfail}}
set code [catch {ci} msg]
set after [list $code $msg [info commands ci] [package provide pi]]
package ifneeded pi 1.0 {package provide pi 1.0}
list $created $after [package require pi]";
    for version in [TclVersion::V9_0, TclVersion::V9_1] {
        assert_eq!(
            result_for(version, src),
            "y {1 injfail {} {}} 1.0",
            "{version:?}"
        );
    }
}

/// A require reached through a host re-entry (`lsort -command`) still
/// cannot suspend, as in C; the failed loader leaves no marker behind.
#[test]
fn require_across_a_host_reentry_still_cannot_yield() {
    assert_every_release(
        r"package forget ls
package ifneeded ls 1.0 {yield inner; package provide ls 1.0}
set code [catch {coroutine cls apply {{} {
    lsort -command {apply {{a b} {package require ls; return 0}}} {b a}
}}} msg]
list $code $msg [package provide ls] [catch {package require ls} again] $again",
        "1 {cannot yield: C stack busy} {} 1 {yield can only be called in a coroutine}",
    );
}
