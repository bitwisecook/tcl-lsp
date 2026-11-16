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

use super::*;
use crate::counters;
use crate::interp::{drop_fresh, new_string, obj_bytes};
use tcl_runtime_api::{Code, GLOBAL_FRAME, ROOT_NS};

/// Run `body` against a fresh interpreter and assert it leaks nothing (the
/// `*mut TclObj` refcount discipline of the `VarStore` impl is correct).
fn leak_free(body: impl FnOnce(&mut Interp)) {
    counters::reset();
    {
        let mut interp = Interp::new();
        body(&mut interp);
    }
    assert_eq!(
        counters::finalize(),
        0,
        "residual: {} objs, {} bufs",
        counters::live_objs(),
        counters::live_bufs()
    );
    assert_eq!(counters::double_free_count(), 0);
}

#[test]
fn unsupported_variable_inventory_policy_preserves_original_guest_state() {
    // naming.invocation.original-variable-inventory-policy
    // docs/design/analysis/name-resolution-proofs/invocation-original-variable-inventory-policy.md
    // Software purpose/refusal control; no original C/Jim observation.
    leak_free(|interp| {
        interp.set_result_bytes(b"RESULT BEFORE");
        let unknown = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
        )));
        interp.set_dialect_profile(unknown);
        assert!(interp.name_policy_protocol().is_none());
        assert_eq!(Namespaces::variable_lookup_policy(interp), None);
        let pattern = crate::obj::Owned::fresh(crate::obj::new_wide_int_obj(17));
        assert!(!crate::obj::has_string_rep(pattern.as_ptr()));
        for error in [
            tcl_cmd_core::info::vars(interp, Some(&pattern.as_ptr())).unwrap_err(),
            tcl_cmd_core::info::globals(interp, Some(&pattern.as_ptr())).unwrap_err(),
        ] {
            assert_eq!(
                error.native_access_refusal(),
                Some(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "variable inventory lookup policy"
                    )
                )
            );
            assert!(!crate::obj::has_string_rep(pattern.as_ptr()));
            assert_eq!(interp.result_bytes(), b"RESULT BEFORE");
        }
    });
}

#[test]
fn varstore_set_get_unset_exists() {
    leak_free(|i| {
        assert!(!i.exists(GLOBAL_FRAME, "x"));
        // A fresh (rc-0) object handed to `set`; the table takes its +1.
        i.set(GLOBAL_FRAME, "x", new_string(b"hi"));
        assert!(i.exists(GLOBAL_FRAME, "x"));
        // `get` is borrowed — read it without releasing.
        assert_eq!(obj_bytes(i.get(GLOBAL_FRAME, "x").unwrap()), b"hi");
        // Overwrite: the table releases the old value and takes the new.
        i.set(GLOBAL_FRAME, "x", new_string(b"bye"));
        assert_eq!(obj_bytes(i.get(GLOBAL_FRAME, "x").unwrap()), b"bye");
        assert!(i.unset(GLOBAL_FRAME, "x"));
        assert!(!i.exists(GLOBAL_FRAME, "x"));
        assert!(!i.unset(GLOBAL_FRAME, "x")); // already gone
    });
}

#[test]
fn varstore_array_elements() {
    leak_free(|i| {
        assert!(!i.exists_elem(GLOBAL_FRAME, "a", "k"));
        i.set_elem(GLOBAL_FRAME, "a", "k", new_string(b"v"));
        assert!(i.exists_elem(GLOBAL_FRAME, "a", "k"));
        assert_eq!(obj_bytes(i.get_elem(GLOBAL_FRAME, "a", "k").unwrap()), b"v");
        assert!(!i.exists_elem(GLOBAL_FRAME, "a", "nope"));
        assert!(i.unset_elem(GLOBAL_FRAME, "a", "k"));
        assert!(!i.exists_elem(GLOBAL_FRAME, "a", "k"));
        i.unset(GLOBAL_FRAME, "a"); // drop the now-empty array
    });
}

#[test]
fn introspect_level_and_argv() {
    leak_free(|i| {
        // Top level: depth 0, the global frame has no invoking call.
        assert_eq!(Introspect::level(i), 0);
        assert!(Introspect::level_argv(i, 0).unwrap().is_none());
        // Push a proc-call frame and record its invoking words.
        i.frames.borrow_mut().push(crate::namespace::GLOBAL);
        i.frames
            .borrow_mut()
            .set_words(vec![b"p".to_vec(), b"x".to_vec()]);
        assert_eq!(Introspect::level(i), 1);
        // `level_argv` builds a fresh (rc-0) list; the result slot adopts it
        // via `set_result`, so the leak gate stays balanced.
        let argv = Introspect::level_argv(i, 1).unwrap().expect("level 1 argv");
        i.set_result(argv);
        assert_eq!(i.result_bytes(), b"p x");
        i.frames.borrow_mut().pop();
        assert_eq!(Introspect::level(i), 0);
    });
}

#[test]
fn varstore_honours_frame_id() {
    leak_free(|i| {
        // A global, written while the global frame is active.
        i.set(GLOBAL_FRAME, "g", new_string(b"global"));
        // Enter a proc-call frame (its own local table).
        let lvl = i.frames.borrow_mut().push(crate::namespace::GLOBAL);
        let here = FrameId(lvl);
        assert_ne!(here, GLOBAL_FRAME);
        i.set(here, "loc", new_string(b"local"));
        // FrameId is honoured: the global is reachable via GLOBAL_FRAME but
        // is not a proc local; the local lives in the proc frame only.
        assert_eq!(obj_bytes(i.get(GLOBAL_FRAME, "g").unwrap()), b"global");
        assert!(i.get(here, "g").is_none());
        assert!(i.exists(here, "loc"));
        assert!(!i.exists(GLOBAL_FRAME, "loc"));
        // Frame-addressed pair access must not rebuild `z(b(key)` and
        // reparse it as a different array reference.
        i.set_elem(GLOBAL_FRAME, "z(b", "key", new_string(b"pair"));
        assert_eq!(
            obj_bytes(i.get_elem(GLOBAL_FRAME, "z(b", "key").unwrap()),
            b"pair"
        );
        assert!(i.exists_elem(GLOBAL_FRAME, "z(b", "key"));
        assert!(i.unset_elem(GLOBAL_FRAME, "z(b", "key"));
        // Reach back into the global frame from the proc frame.
        i.set(GLOBAL_FRAME, "g2", new_string(b"two"));
        // Pop the proc frame (frees `loc`); the reached-back write is visible.
        i.frames.borrow_mut().pop();
        assert_eq!(obj_bytes(i.get(GLOBAL_FRAME, "g2").unwrap()), b"two");
        assert!(i.unset(GLOBAL_FRAME, "g2"));
        assert!(i.unset(GLOBAL_FRAME, "g"));
        assert!(i.unset(GLOBAL_FRAME, "z(b"));
        assert!(!i.exists(GLOBAL_FRAME, "g"));
    });
}

#[test]
fn commands_dispatch_builtin_and_unknown() {
    leak_free(|i| {
        // A builtin runs and yields its result. The caller owns its argv
        // (rc >= 1, the dispatch contract); the completion owns +1 on the
        // result + options, which the caller releases. UFCS reaches the trait
        // method — the inherent `Interp::dispatch` shadows it for method
        // syntax (a generic `T: Commands` consumer is unaffected).
        let lst = new_string(b"a b c");
        unsafe { obj::incr_ref_count(lst) };
        let c = Commands::dispatch(i, "llength", &[lst]).unwrap();
        assert_eq!(c.code, Code::Ok);
        assert_eq!(obj_bytes(c.result), b"3");
        unsafe {
            obj::decr_ref_count(c.result);
            obj::decr_ref_count(c.options);
            obj::decr_ref_count(lst);
        }

        // An unknown command name is an error completion.
        let c = Commands::dispatch(i, "no_such_command", &[]).unwrap();
        assert_eq!(c.code, Code::Error);
        assert!(obj_bytes(c.result).starts_with(b"invalid command name"));
        unsafe {
            obj::decr_ref_count(c.result);
            obj::decr_ref_count(c.options);
        }
    });
}

#[test]
fn traces_fire_ok_and_error() {
    leak_free(|i| {
        // A read trace whose callback succeeds → the access proceeds (Ok).
        // (`;#` comments out the appended `name elem op` words.)
        let _ = i.eval_str(b"trace add variable x read {list ok;#}");
        assert!(Traces::fire(i, "x", "read")
            .expect("actual Guest trace has no Host refusal")
            .is_ok());

        // A read trace whose callback errors → the access aborts, with the
        // error wrapped to the user-facing `can't read "var"` form. The error
        // value is fresh (rc-0); adopt and release it.
        let _ = i.eval_str(b"trace add variable y read {error boom;#}");
        let e = Traces::fire(i, "y", "read")
            .expect("actual Guest trace has no Host refusal")
            .unwrap_err();
        assert_eq!(obj_bytes(e), b"can't read \"y\": boom");
        drop_fresh(e);

        // An `unset` trace error does *not* abort (matches C / the VM): Ok.
        let _ = i.eval_str(b"trace add variable z unset {error nope;#}");
        assert!(Traces::fire(i, "z", "unset")
            .expect("actual Guest trace has no Host refusal")
            .is_ok());
    });
}

#[test]
fn frames_push_pop_current_link() {
    leak_free(|i| {
        // A global, set while the global frame is current.
        i.set(GLOBAL_FRAME, "g", new_string(b"orig"));
        let outer = Frames::current(i);
        assert_eq!(outer, GLOBAL_FRAME);
        // Push a proc-call frame in the global namespace; it becomes current.
        let inner = Frames::push(i, ROOT_NS);
        assert_ne!(inner, outer);
        assert_eq!(Frames::current(i), inner);
        // `upvar`: link `gg` (inner) to the outer frame's global `g`. The
        // link target resolves to the global namespace table (level 0), so
        // reads and writes through `gg` reach `g`.
        Frames::link(i, inner, outer, "gg", "g");
        assert_eq!(obj_bytes(i.get(inner, "gg").unwrap()), b"orig");
        i.set(inner, "gg", new_string(b"changed"));
        // Pop back to the global frame: the link is gone, the global updated.
        Frames::pop(i);
        assert_eq!(Frames::current(i), outer);
        assert_eq!(obj_bytes(i.get(GLOBAL_FRAME, "g").unwrap()), b"changed");
        assert!(i.unset(GLOBAL_FRAME, "g"));
    });
}

#[test]
fn namespaces_current_and_find_command() {
    leak_free(|i| {
        // At the top level the current namespace is the global root.
        assert_eq!(Namespaces::current(i), ROOT_NS);
        // A builtin resolves from the global namespace to a stable CommandId.
        let a = Namespaces::find_command(i, ROOT_NS, "list").expect("list resolves");
        let b = Namespaces::find_command(i, ROOT_NS, "list").expect("list resolves");
        assert_eq!(a, b);
        // A different command resolves to a distinct id.
        let c = Namespaces::find_command(i, ROOT_NS, "llength").expect("llength resolves");
        assert_ne!(a, c);
        // An unknown command resolves to nothing.
        assert!(Namespaces::find_command(i, ROOT_NS, "no_such_command").is_none());
    });
}

#[test]
fn commands_dispatch_id_composes() {
    leak_free(|i| {
        // Resolve a command to a handle, then invoke it *by that handle* —
        // the find_command -> dispatch_id composition.
        let id = Namespaces::find_command(i, ROOT_NS, "list").expect("list resolves");
        let arg = new_string(b"x");
        unsafe { obj::incr_ref_count(arg) };
        let c = Commands::dispatch_id(i, id, &[arg]).unwrap();
        assert_eq!(c.code, Code::Ok);
        assert_eq!(obj_bytes(c.result), b"x");
        unsafe {
            obj::decr_ref_count(c.result);
            obj::decr_ref_count(c.options);
            obj::decr_ref_count(arg);
        }
        // A fabricated id yields an error completion (no such command).
        let c = Commands::dispatch_id(i, CommandId(9999), &[]).unwrap();
        assert_eq!(c.code, Code::Error);
        assert_eq!(obj_bytes(c.result), b"invalid command id");
        unsafe {
            obj::decr_ref_count(c.result);
            obj::decr_ref_count(c.options);
        }
    });
}

#[test]
fn command_ids_do_not_rebind_to_same_named_replacements() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(b"proc p {} {return OLD}"),
            crate::interp::Code::Ok
        );
        let old = Namespaces::find_command(i, ROOT_NS, "p").expect("old p resolves");
        assert_eq!(
            i.eval_str(b"proc p {} {return NEW}"),
            crate::interp::Code::Ok
        );
        let new = Namespaces::find_command(i, ROOT_NS, "p").expect("new p resolves");
        assert_ne!(old, new);

        let stale = Commands::dispatch_id(i, old, &[]).unwrap();
        assert_eq!(stale.code, Code::Error);
        assert_eq!(obj_bytes(stale.result), b"invalid command id");
        unsafe {
            obj::decr_ref_count(stale.result);
            obj::decr_ref_count(stale.options);
        }

        let live = Commands::dispatch_id(i, new, &[]).unwrap();
        assert_eq!(live.code, Code::Ok);
        assert_eq!(obj_bytes(live.result), b"NEW");
        unsafe {
            obj::decr_ref_count(live.result);
            obj::decr_ref_count(live.options);
        }
    });
}
