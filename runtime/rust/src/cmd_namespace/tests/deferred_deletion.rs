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

//! Deleting a namespace that is still running: the retained token, its recreation, and when traces and teardown finally fire.

use super::*;

// A deleted namespace is retained for its live frames.
//
// C's `Tcl_DeleteNamespace` takes the `activationCount > (nsPtr ==
// globalNsPtr)` branch: `NS_DYING`, unlink the parent edge, and leave the
// contents alone. `Tcl_PopCallFrame` calls the deletion again once the last
// frame holding the token goes away.

/// `namespace delete ::N` from inside `::N::p` unpublishes the name
/// immediately, yet the relative `q` still resolves through the frame's
/// own token.
#[test]
fn deleting_the_running_namespace_retains_it_for_the_frame() {
    pins(
        br#"set answer [namespace eval N {
                    proc q {} {return Q}
                    proc p {} {
                        namespace delete ::N
                        set code [catch {q} message]
                        list [namespace exists ::N] [info commands ::N::p] \
                             [info commands ::N::q] $code $message [namespace current]
                    }
                    p
                }]
                list $answer [namespace exists ::N] [info commands ::N::*]"#,
        b"{0 {} {} 0 Q ::N} 0 {}",
    );
}

/// A `namespace eval` frame is an activation too (C counts every
/// `Tcl_PushCallFrame`), so deleting from the body defers just the same.
#[test]
fn a_namespace_eval_frame_defers_its_own_deletion() {
    pins(
        br#"set r [namespace eval N {
                    proc q {} {return Q}
                    namespace delete ::N
                    list [namespace exists ::N] [catch {q} m] $m [namespace current] \
                         [info commands ::N::*]
                }]
                list $r [namespace exists ::N]"#,
        b"{0 0 Q ::N {}} 0",
    );
}

/// `namespace inscope` and `apply -ns` push the same kind of frame.
#[test]
fn inscope_and_apply_frames_defer_their_own_deletion() {
    pins(
        br#"namespace eval N {proc q {} {return Q}}
                set a [namespace inscope N {
                    namespace delete ::N
                    list [q] [namespace current] [namespace exists ::N]
                }]
                list $a [namespace exists ::N]"#,
        b"{Q ::N 0} 0",
    );
    pins(
        br#"set log {}
                proc rec {old new op} {lappend ::log [list $old $op]}
                namespace eval N {proc q {} {return Q}}
                trace add command ::N::q delete rec
                set a [apply [list {} {namespace delete ::N; list [q] [llength $::log]} ::N]]
                list $a $log"#,
        b"{Q 0} {{::N::q delete}}",
    );
}

/// The name is free the instant the token is retained: `namespace eval ::N`
/// builds a wholly separate token, and neither absorbs the other. C nulls
/// `parentPtr` in the deferred branch, so the child entry is gone before a
/// recreation looks for it (unlike the synchronous window, which errors
/// `already exists`).
#[test]
fn a_recreated_namespace_is_a_distinct_token_from_the_retained_one() {
    pins(
        br#"namespace eval N {
                    proc q {} {return Q}
                    proc p {} {
                        namespace delete ::N
                        namespace eval ::N {proc r {} {return R}}
                        list [q] [catch {r} m] $m [info commands ::N::*] [::N::r] \
                             [namespace current] [namespace exists ::N] [namespace which q] \
                             [namespace which r] [info commands q]
                    }
                }
                set a [::N::p]
                list $a [info commands ::N::*] [namespace exists ::N]"#,
        br#"{Q 1 {invalid command name "r"} ::N::r R ::N 1 ::N::q {} q} ::N::r 1"#,
    );
}

/// #1764: the object command retained by an active namespace frame keeps
/// its original OO identity when a new namespace generation publishes a
/// same-named object. Absolute lookup reaches the new command while the
/// relative retained command continues to dispatch to the old object.
#[test]
fn recreated_namespace_keeps_old_and_new_object_identities_distinct() {
    pins(
        br#"set r [namespace eval N {
                    oo::class create C {method m {} {return OLD}}
                    C create o
                    proc p {} {
                        namespace delete ::N
                        namespace eval ::N {
                            oo::class create C {method m {} {return NEW}}
                            C create o
                        }
                        set a [catch {o m} am]
                        set b [catch {::N::o m} bm]
                        list $a $am $b $bm [info commands o] [info commands ::N::o]
                    }
                    p
                }]
                set c [catch {::N::o m} cm]
                list $r $c $cm"#,
        b"{0 OLD 0 NEW o ::N::o} 0 NEW",
    );
}

#[test]
fn retained_object_teardown_preserves_recreated_method_dispatch() {
    // Native proof: naming.namespace.original-command-holder-routing
    // docs/design/analysis/name-resolution-proofs/namespace-original-command-holder-routing.md
    // The original public control reports no private dispatcher command,
    // while the recreated object's method still invokes my successfully.
    pins(
        br#"set r [namespace eval N {
                    oo::class create C {method m {} {return OLD}}
                    C create o
                    proc p {} {
                        namespace delete ::N
                        namespace eval ::N {
                            oo::class create C {
                                method m {} {return NEW}
                                method n {} {my m}
                            }
                            C create o
                        }
                        o m
                    }
                    p
                }]
                list $r [info commands ::N::o::my] [::N::o n]"#,
        b"OLD {} NEW",
    );
}

/// A command trace belongs to a token, not to a spelling: the retained
/// `::N::q` and the recreated `::N::q` each fire their own, in their own
/// teardown (C walks `cmdPtr->tracePtr`).
#[test]
fn a_retained_token_and_its_recreation_fire_separate_command_traces() {
    pins(
        br#"set log {}
                proc rec {old new op} {lappend ::log [list old $old $op]}
                proc rec2 {old new op} {lappend ::log [list new $old $op]}
                namespace eval N {
                    proc q {} {return Q}
                    proc p {} {
                        namespace delete ::N
                        namespace eval ::N {proc q {} {return R}}
                        trace add command ::N::q delete rec2
                        set ::mid $::log
                        return [q]
                    }
                }
                trace add command ::N::q delete rec
                set r [::N::p]
                set after $::log
                namespace delete ::N
                list $r $mid $after $::log"#,
        b"Q {} {{old ::N::q delete}} {{old ::N::q delete} {new ::N::q delete}}",
    );
}

/// A qualified definition cannot reach the retained token — the public name
/// is gone — but an unqualified one lands in it through the frame.
#[test]
fn a_retained_token_takes_relative_definitions_only() {
    pins(
        br#"namespace eval N {
                    proc p {} {
                        namespace delete ::N
                        proc r {} {return R}
                        list [r] [info commands ::N::r] [namespace which r] \
                             [catch {proc ::N::s {} {}} m] $m [info commands ::N::*] \
                             [namespace exists ::N]
                    }
                }
                set a [::N::p]
                list $a [info commands ::N::*] [namespace exists ::N]"#,
        br#"{R {} ::N::r 1 {can't create procedure "::N::s": unknown namespace} {} 0} {} 0"#,
    );
}

/// Variables ride with the token: `variable v` still reads it, while every
/// name-addressed probe fails.
#[test]
fn a_retained_token_keeps_its_variables() {
    pins(
        br#"namespace eval N {
                    variable v V
                    proc p {} {
                        namespace delete ::N
                        variable v
                        list [catch {set ::N::v} m] $m [set v] [info vars ::N::*] \
                             [info vars [namespace current]::*]
                    }
                }
                ::N::p"#,
        br#"1 {can't read "::N::v": no such variable} V {} {}"#,
    );
}

/// The current-namespace probes the issue names, from inside the frame.
#[test]
fn a_retained_token_reports_its_own_name_and_commands() {
    pins(
        br#"namespace eval N {
                    variable v 1
                    proc p {} {
                        namespace delete ::N
                        list [namespace exists [namespace current]] \
                             [info commands [namespace current]::*] [info procs] \
                             [namespace which -variable v]
                    }
                }
                ::N::p"#,
        b"0 {} p ::N::v",
    );
}

/// A child of the retained token stays reachable relatively, and reachable
/// by no absolute name at all. The absolute name is built at run time on
/// purpose: a literal would measure tclsh's `ResolvedCmdName` bytecode
/// cache, which only invalidates on `NS_DYING` of the command's *own*
/// namespace.
#[test]
fn a_retained_token_keeps_its_children() {
    pins(
        br#"namespace eval N {
                    namespace eval C {proc x {} {return X}}
                    proc p {} {
                        namespace delete ::N
                        set n [join [list {} N C x] ::]
                        list [C::x] [namespace exists ::N::C] [catch {$n} m] $m \
                             [namespace children] [catch {namespace children ::N} m2] $m2
                    }
                }
                set a [::N::p]
                list $a [namespace exists ::N::C]"#,
        br#"{X 0 1 {invalid command name "::N::C::x"} ::N::C 1 {namespace "::N" not found}} 0"#,
    );
}

/// Deletion recurses through the same count-checking entry C's
/// `TclDeleteNamespaceChildren` uses: the parent has no frame and tears down
/// at once, the child holding the frame is retained until it pops.
#[test]
fn an_active_child_defers_while_its_parent_tears_down() {
    pins(
            br#"set log {}
                proc rec {old new op} {lappend ::log [list $old $op]}
                namespace eval N {
                    proc np {} {return NP}
                    namespace eval C {
                        proc cq {} {return CQ}
                        proc p {} {
                            namespace delete ::N
                            set abs [join [list {} N C cq] ::]
                            list [namespace exists ::N] [namespace exists ::N::C] \
                                 [namespace current] [cq] [catch {$abs} m] $m [llength $::log]
                        }
                    }
                }
                trace add command ::N::np delete rec
                trace add command ::N::C::cq delete rec
                set a [::N::C::p]
                list $a $log"#,
            br#"{0 0 ::N::C CQ 1 {invalid command name "::N::C::cq"} 1} {{::N::np delete} {::N::C::cq delete}}"#,
        );
}

/// TclOO destruction follows the same per-token activation check as the
/// namespace command table. Deleting an inactive parent must not tear down
/// objects owned by its active child generation.
#[test]
fn active_child_retains_its_oo_state_during_parent_deletion() {
    pins(
        br#"namespace eval P::N {
                    oo::class create C {method m {} {return OLD}}
                    C create o
                    proc p {} {
                        namespace delete ::P
                        set a [list [namespace current] [namespace exists ::P] \
                                    [catch {o m} om] $om]
                        namespace eval ::P::N {
                            oo::class create C {method m {} {return NEW}}
                            C create o
                        }
                        set b [list [catch {o m} om] $om \
                                    [catch {::P::N::o m} nm] $nm]
                        list $a $b
                    }
                }
                set r [::P::N::p]
                list $r [::P::N::o m]"#,
        b"{{::P::N 0 0 OLD} {0 OLD 0 NEW}} NEW",
    );
}

/// An OO command and its default instance namespace created by a delete
/// trace join the exact dying namespace token. The fixed-point sweep then
/// retires them instead of publishing a fresh visible namespace tree.
#[test]
fn oo_created_by_delete_trace_joins_dying_generation() {
    pins(
        br#"set log {}
                proc mk {old new op} {
                    lappend ::log make
                    oo::object create ::N::late
                    lappend ::log made
                }
                namespace eval N {proc p {} {}}
                trace add command ::N::p delete mk
                namespace delete ::N
                list $log [namespace exists ::N] [info commands ::N::*] \
                     [info object isa object ::N::late] \
                     [catch {::N::late destroy} m] $m"#,
        br#"{make made} 0 {} 0 1 {invalid command name "::N::late"}"#,
    );
}

/// Nothing in the retained token fires until the last frame pops — and a
/// second `namespace eval N` around the call is a second activation, so
/// even returning from the proc is not enough.
#[test]
fn command_delete_traces_wait_for_the_last_activation() {
    pins(
        br#"set log {}
                proc rec {old new op} {lappend ::log [list $old $op]}
                namespace eval N {
                    proc q {} {return Q}
                    proc p {} {namespace delete ::N; set ::mid $::log; return [q]}
                }
                trace add command ::N::p delete rec
                trace add command ::N::q delete rec
                set r [namespace eval N {set inner [p]; set ::after_p $::log; set inner}]
                list $r $mid $after_p $log"#,
        b"Q {} {} {{::N::p delete} {::N::q delete}}",
    );
    pins(
        br#"set log {}
                namespace eval N {
                    variable v 1
                    proc p {} {namespace delete ::N; set ::mid $::log; return ok}
                }
                trace add variable ::N::v unset {apply {{a b c} {lappend ::log unset-v}}}
                set r [::N::p]
                list $r $mid $log"#,
        b"ok {} unset-v",
    );
}

/// The deferring frame need not be the one running `namespace delete`, and
/// the retained token refuses a second deletion (its name is already gone).
#[test]
fn a_deferred_namespace_is_unpublished_for_every_caller() {
    pins(
        br#"set log {}
                proc rec {old new op} {lappend ::log [list $old $op]}
                namespace eval N {
                    proc q {} {return Q}
                    proc p2 {} {namespace delete ::N}
                    proc p {} {p2; list [q] [llength $::log]}
                }
                trace add command ::N::q delete rec
                set a [::N::p]
                list $a [llength $log]"#,
        b"{Q 0} 1",
    );
    pins(
        br#"namespace eval N {
                    proc p {} {namespace delete ::N; list [catch {namespace delete ::N} m] $m}
                }
                ::N::p"#,
        br#"1 {unknown namespace "::N" in namespace delete command}"#,
    );
}

/// An ensemble the namespace owns dies at once even when the namespace is
/// retained: C pops `nsPtr->ensembles` before it looks at the count.
#[test]
fn an_owned_ensemble_dies_while_its_namespace_is_retained() {
    pins(
        br#"set log {}
                proc rec {old new op} {lappend ::log [list $old $op]}
                namespace eval N {
                    namespace ensemble create -command ::E
                    proc p {} {namespace delete ::N; list [info commands ::E] $::log}
                }
                trace add command ::E delete rec
                ::N::p"#,
        b"{} {{::E delete}}",
    );
}

/// The deferred teardown is the ordinary one, run later: the command table
/// still empties in `Tcl_FirstHashEntry` order, imports included.
#[test]
fn the_deferred_teardown_keeps_tcl_hash_order() {
    pins(
        br#"set log {}
                proc rec2 {old new op} {lappend ::log [namespace tail $old]}
                namespace eval N {}
                foreach n {one two three four five six seven eight nine ten} {
                    proc ::N::$n {} {}
                }
                proc ::N::p {} {namespace delete ::N; return [llength $::log]}
                foreach n [info commands ::N::*] {trace add command $n delete rec2}
                set r [::N::p]
                list $r $log"#,
        b"0 {p six four three eight seven nine five two one ten}",
    );
    pins(
        br#"set log {}
                proc rec2 {old new op} {lappend ::log [namespace tail $old]}
                namespace eval S {proc s1 {} {}; namespace export s*}
                namespace eval N {
                    namespace import ::S::s1
                    proc p {} {namespace delete ::N; return}
                }
                foreach n {one two three four five six seven eight nine ten} {
                    proc ::N::$n {} {}
                }
                foreach n [info commands ::N::*] {trace add command $n delete rec2}
                ::N::p
                set log"#,
        b"p three seven one two four eight five nine s1 six ten",
    );
}
