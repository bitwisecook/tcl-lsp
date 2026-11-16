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

//! Destroying objects and classes: destructors and their errors, rename and namespace deletion, and the retirement of an object's command roles.

use super::*;

#[test]
fn rename_to_empty_frees_object_and_class_names() {
    leak_free(|i| {
        // Deleting an object's command (rename to {}) frees the name so it
        // can be recreated — the OO registry tracks the command's lifetime.
        ok(i, b"oo::object create foo");
        ok(i, b"rename foo {}");
        assert_eq!(ok(i, b"oo::object create foo"), b"::foo");
        // Same for a class (registered in both the object and class maps).
        ok(i, b"oo::class create C");
        ok(i, b"rename C {}");
        assert_eq!(ok(i, b"oo::class create C"), b"::C");
        // A renamed object follows its command to the new name.
        ok(i, b"oo::object create a; rename a b");
        assert_eq!(ok(i, b"info object isa object b"), b"1");
        i.eval_str(b"rename foo {}; rename C {}; rename b {}");
    });
}

#[test]
fn destructor_error_goes_to_bgerror() {
    leak_free(|i| {
        // A destructor erroring during implicit teardown (`rename {}`) is
        // queued as a background error and reported to the `interp bgerror`
        // handler at `update`, not propagated (oo-3.7/3.8).
        ok(
                i,
                b"interp bgerror {} [list apply {{var msg args} {upvar #0 $var v; lappend v $msg}} ::caught]",
            );
        ok(i, b"oo::class create cls { destructor {error boom} }");
        ok(i, b"set ::caught {}; cls create obj");
        ok(i, b"rename obj {}");
        ok(i, b"update idletasks");
        assert_eq!(ok(i, b"set ::caught"), b"boom");
        // An empty cmdPrefix is rejected (C's `ChildBgerror`: length >= 1),
        // so the handler is reset by overwriting it with a one-element list.
        ok(i, b"interp bgerror {} noop");
    });
}

#[test]
fn destructors_fire_on_rename_destroy_and_namespace_delete() {
    leak_free(|i| {
        ok(
            i,
            b"oo::class create cls { destructor { lappend ::result died } }",
        );
        // `obj destroy`
        ok(i, b"set ::result {}; cls create a; a destroy");
        assert_eq!(ok(i, b"set ::result"), b"died");
        // `rename obj {}`
        ok(i, b"set ::result {}; cls create b; rename b {}");
        assert_eq!(ok(i, b"set ::result"), b"died");
        // `namespace delete <object-namespace>`
        ok(
            i,
            b"set ::result {}; cls create c; namespace delete [info object namespace c]",
        );
        assert_eq!(ok(i, b"set ::result"), b"died");
        // Re-entrancy guard: a destructor that re-destroys the object runs once.
        ok(
            i,
            b"oo::class create g { destructor { lappend ::result x; catch {[self] destroy} } }",
        );
        ok(i, b"set ::result {}; g create o; o destroy");
        assert_eq!(ok(i, b"set ::result"), b"x");
        // A2: the dup-name error uses the as-written name and `object`.
        ok(i, b"oo::object create foo");
        assert_eq!(i.eval_str(b"oo::object create foo"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"can't create object \"foo\": command already exists with that name"
        );
        ok(i, b"oo::class create K");
        assert_eq!(i.eval_str(b"oo::class create K"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"can't create object \"K\": command already exists with that name"
        );
    });
}

/// TclOO command lifetime is the stable owner identity, not the current
/// table/name. Explicit destruction must retire a hidden object token and
/// preserve the destructor's successful result.
#[test]
fn hidden_object_destruction_retires_identity_and_preserves_result() {
    leak_free(|i| {
        assert_eq!(
            ok(
                i,
                br#"set log {}
                        oo::class create C {
                            destructor {lappend ::log [self object]}
                        }
                        C create x
                        interp hide {} x held
                        set a [interp invokehidden {} held destroy]
                        list $a $log [interp hidden {}] \
                             [catch {interp invokehidden {} held destroy} m] $m"#,
            ),
            br#"::x ::x {} 1 {invalid hidden command name "held"}"#,
        );
    });
    leak_free(|i| {
        assert_eq!(
            ok(
                i,
                br#"namespace eval N {
                            oo::class create C
                            C create o
                            namespace export o
                        }
                        namespace import N::o
                        set a [o destroy]
                        list $a [info commands o] [info commands ::N::o]"#,
            ),
            b"{} {} {}",
        );
    });
}

/// Class commands and the private dispatchers share the same retirement
/// seam, including after independent rename and hide operations.
#[test]
fn hidden_class_and_private_dispatchers_retire_by_owner_identity() {
    leak_free(|i| {
        assert_eq!(
            ok(
                i,
                br#"oo::class create C
                        C create x
                        interp hide {} C heldC
                        interp invokehidden {} heldC destroy
                        list [interp hidden {}] [info commands x] \
                             [catch {interp invokehidden {} heldC new} m] $m"#,
            ),
            br#"{} {} 1 {invalid hidden command name "heldC"}"#,
        );
    });
    leak_free(|i| {
        assert_eq!(
            ok(
                i,
                br#"oo::class create C {method who {} {self object}}
                        C create x
                        set ns [info object namespace x]
                        rename ${ns}::my myx
                        interp hide {} myx heldmy
                        x destroy
                        list [interp hidden {}] \
                             [catch {interp invokehidden {} heldmy who} m] $m"#,
            ),
            br#"{} 1 {invalid hidden command name "heldmy"}"#,
        );
    });
}

/// TclOO teardown removes the old command token's trace list, not traces a
/// delete callback attaches to a same-named replacement (Tcl 9.0.4).
#[test]
fn object_retirement_preserves_visible_replacement_traces() {
    leak_free(|i| {
        assert_eq!(
                ok(
                    i,
                    br#"set log {}
                        oo::class create C {
                            destructor {lappend ::log [list d [self object]]}
                        }
                        C create x
                        proc cb {old new op} {
                            lappend ::log [list t $old $new $op]
                            proc x {} {return P}
                            trace add command x delete cbnew
                        }
                        proc cbnew {old new op} {
                            lappend ::log [list r $old $new $op]
                        }
                        trace add command x delete cb
                        x destroy
                        set before [list [x] [trace info command x] $log]
                        rename x {}
                        list $before $log"#,
                ),
                br#"{P {{delete cbnew}} {{d ::x} {t ::x {} delete}}} {{d ::x} {t ::x {} delete} {r ::x {} delete}}"#,
            );
    });
}

/// A hidden token keeps its generation, so its delete trace does not own a
/// visible replacement and trace at the same display spelling.
#[test]
fn object_retirement_preserves_hidden_name_replacement_traces() {
    leak_free(|i| {
        assert_eq!(
                ok(
                    i,
                    br#"set log {}
                        oo::class create C {
                            destructor {lappend ::log [list d [self object]]}
                        }
                        C create x
                        proc cb {old new op} {
                            lappend ::log [list t $old $new $op]
                            proc held {} {return H}
                            trace add command held delete cbnew
                        }
                        proc cbnew {old new op} {
                            lappend ::log [list r $old $new $op]
                        }
                        trace add command x delete cb
                        interp hide {} x held
                        interp invokehidden {} held destroy
                        set before [list [held] [trace info command held] $log \
                                         [interp hidden {}]]
                        rename held {}
                        list $before $log"#,
                ),
                br#"{H {{delete cbnew}} {{d ::x} {t ::held {} delete}} {}} {{d ::x} {t ::held {} delete} {r ::held {} delete}}"#,
            );
    });
}

/// Renaming an object's private dispatcher does not turn its mutable name
/// into ownership: a callback replacement at that name remains independent.
#[test]
fn object_retirement_preserves_private_dispatcher_replacement_traces() {
    leak_free(|i| {
        assert_eq!(
                ok(
                    i,
                    br#"set log {}
                        oo::class create C {
                            destructor {lappend ::log [list d [self object]]}
                        }
                        C create x
                        set ns [info object namespace x]
                        rename ${ns}::my myx
                        proc cb {old new op} {
                            lappend ::log [list t $old $new $op]
                            proc myx {} {return M}
                            trace add command myx delete cbnew
                        }
                        proc cbnew {old new op} {
                            lappend ::log [list r $old $new $op]
                        }
                        trace add command myx delete cb
                        x destroy
                        set before [list [myx] [trace info command myx] $log]
                        rename myx {}
                        list $before $log"#,
                ),
                br#"{M {{delete cbnew}} {{d ::x} {t ::myx {} delete}}} {{d ::x} {t ::myx {} delete} {r ::myx {} delete}}"#,
            );
    });
}

/// The public object's delete callback runs before private dispatchers. A
/// dispatcher it moves is found again by identity and retired at its new
/// location, including a trace attached there by the callback.
#[test]
fn public_delete_callback_can_relocate_a_private_dispatcher() {
    leak_free(|i| {
        assert_eq!(
            ok(
                i,
                br#"set log {}
                        oo::class create C
                        C create x
                        set ns [info object namespace x]
                        proc cb {ns old new op} {
                            lappend ::log [list move \
                                [catch {rename ${ns}::my ::movedmy} msg] \
                                $msg [info commands ::movedmy]]
                            trace add command ::movedmy delete mygone
                        }
                        proc mygone {old new op} {
                            lappend ::log [list mygone $old $op]
                        }
                        trace add command x delete [list cb $ns]
                        x destroy
                        list $log [info commands ::movedmy]"#,
            ),
            br#"{{move 0 {} ::movedmy} {mygone ::movedmy delete}} {}"#,
        );
    });
}

/// Tcl retires an object's three command roles in semantic order rather
/// than their table-generation order, even after both private dispatchers
/// have moved out of the instance namespace.
#[test]
fn object_command_roles_retire_public_myclass_my() {
    leak_free(|i| {
        assert_eq!(
            ok(
                i,
                br#"set log {}
                        oo::class create C
                        C create x
                        set ns [info object namespace x]
                        rename ${ns}::my ::mm
                        rename ${ns}::myclass ::mc
                        proc t {tag old new op} {lappend ::log $tag}
                        trace add command x delete [list t public]
                        trace add command ::mm delete [list t my]
                        trace add command ::mc delete [list t myclass]
                        x destroy
                        set log"#,
            ),
            b"public myclass my",
        );
    });
}

/// TclOO's semantic role order also precedes instance-variable teardown.
/// Moving only `myclass` from the public callback is the asymmetric case:
/// table order would otherwise delete the remaining `my` first.
#[test]
fn object_command_roles_precede_variables_after_asymmetric_move() {
    leak_free(|i| {
        assert_eq!(
            ok(
                i,
                br#"set log {}
                        oo::class create C
                        C create x
                        set ns [info object namespace x]
                        set ${ns}::v 1
                        proc public {ns old new op} {
                            lappend ::log public
                            rename ${ns}::myclass ::mc
                        }
                        proc t {tag args} {lappend ::log $tag}
                        proc var {name1 name2 op} {
                            set c [catch {trace add command ::mc delete late} m]
                            lappend ::log [list var [info commands ::mc] $c $m \
                                                [info commands ::x]]
                        }
                        proc late {args} {lappend ::log late}
                        trace add command x delete [list public $ns]
                        trace add command ${ns}::my delete [list t my]
                        trace add command ${ns}::myclass delete [list t myclass]
                        trace add variable ${ns}::v unset var
                        x destroy
                        set log"#,
            ),
            b"public myclass my {var {} 1 {unknown command \"::mc\"} ::x}",
        );
    });
}

/// A later role may attach a trace to an earlier role while every command
/// token is still visible. Tcl discards that late trace at final removal;
/// it does not restart the semantic role walk (Tcl 9.0.4).
#[test]
fn object_command_role_walk_does_not_revisit_earlier_roles() {
    leak_free(|i| {
        assert_eq!(
            ok(
                i,
                br#"set log {}
                        oo::class create C
                        C create x
                        set ns [info object namespace x]
                        proc t {tag args} {lappend ::log $tag}
                        proc mc {old new op} {
                            set c [catch {trace add command ::x delete late} m]
                            lappend ::log [list mc ::x $c $m]
                        }
                        proc late {old new op} {lappend ::log late}
                        trace add command x delete [list t public]
                        trace add command ${ns}::myclass delete mc
                        trace add command ${ns}::my delete [list t my]
                        x destroy
                        set log"#,
            ),
            b"public {mc ::x 0 {}} my",
        );
    });
}

/// Namespace imports retain the source command token across `interp hide`.
/// They keep dispatching the hidden OO object, report its hidden origin,
/// and retire with it.
#[test]
fn hidden_object_imports_follow_and_retire_with_the_source_token() {
    leak_free(|i| {
        assert_eq!(
            ok(
                i,
                br#"oo::class create C {method ping {} {return ok}}
                        C create x
                        namespace export x
                        namespace eval dst {namespace import ::x}
                        set before [dst::x ping]
                        interp hide {} x hx
                        set hiddenCall [dst::x ping]
                        set hiddenOrigin [namespace eval dst {namespace origin x}]
                        interp invokehidden {} hx destroy
                        list $before $hiddenCall $hiddenOrigin \
                             [namespace eval dst {info commands x}]"#,
            ),
            b"ok ok ::hx {}",
        );
    });
}

/// Installing an ordinary command over an object command retires that
/// exact TclOO owner—including destructors and class descendant cascades—
/// while leaving the newly-installed command intact.
#[test]
fn ordinary_command_replacement_runs_displaced_oo_lifecycle() {
    leak_free(|i| {
        assert_eq!(
            ok(
                i,
                br#"set log {}
                        oo::class create C {
                            destructor {lappend ::log [self object]}
                        }
                        C create x
                        proc x {} {return P}
                        list $log [info object isa object x] [x]"#,
            ),
            b"::x 0 P",
        );
    });
    leak_free(|i| {
        assert_eq!(
            ok(
                i,
                br#"oo::class create C
                        C create x
                        proc C {} {return P}
                        list [info commands x] [info object isa object x] [C]"#,
            ),
            b"{} 0 P",
        );
    });
}

#[test]
fn destructor_error_propagation_and_renamed_my_teardown() {
    leak_free(|i| {
        // Explicit `obj destroy` propagates a destructor error to its caller
        // (oo-3.6); the object command is still removed.
        ok(i, b"oo::class create cls { destructor { error foo } }");
        assert_eq!(
            ok(
                i,
                b"list [catch {[cls create obj] destroy} m] $m [info commands obj]"
            ),
            b"1 foo {}"
        );
        // Implicit teardown (rename {}) swallows the destructor error.
        ok(i, b"cls create obj2");
        ok(i, b"rename obj2 {}");
        // A `my` renamed out of the instance namespace is deleted when the
        // object is destroyed (oo-3.4): tie its lifetime to the object.
        ok(i, b"oo::class create plain { constructor {} {} }");
        ok(i, b"plain create p");
        ok(i, b"rename [info object namespace p]::my ::pmy");
        assert_eq!(ok(i, b"info commands ::pmy"), b"::pmy");
        ok(i, b"p destroy");
        assert_eq!(ok(i, b"info commands ::pmy"), b"");
    });
}

#[test]
fn class_destroy_cascades_to_mixin_users() {
    leak_free(|i| {
        // Destroying a class destroys objects that mix it in; an object
        // whose mixin was cleared survives.
        ok(i, b"oo::class create A");
        ok(i, b"oo::class create B");
        ok(i, b"oo::objdefine [A create keep] mixin B");
        ok(i, b"oo::objdefine [A create gone] mixin B");
        ok(i, b"oo::objdefine keep mixin");
        ok(i, b"B destroy");
        assert_eq!(ok(i, b"info object isa object gone"), b"0");
        assert_eq!(ok(i, b"info object isa object keep"), b"1");
    });
}
