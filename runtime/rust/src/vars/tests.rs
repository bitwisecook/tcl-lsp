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
use crate::capi::Tcl_NewStringObj;
use crate::counters;

#[test]
fn compiled_alias_observation_does_not_reselect_duplicate_names() {
    let protocol = tcl_syntax::naming::NativeCompiledVariableProtocol::for_native_point(
        tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0)
            .execution_point()
            .unwrap(),
    )
    .unwrap();
    let mut frames = FrameStack::new();
    let namespaces = Namespaces::new();
    let level = frames.push(crate::namespace::GLOBAL);
    frames.install_formal_cells(&[b"x".to_vec(), b"x".to_vec()], protocol);
    let observed =
        OriginalAliasTargetObservation::capture_compiled(&mut frames, &namespaces, 1).unwrap();
    assert!(frames.original_compiled_cell_at_is_current(level, 1, &observed.cell));
    assert!(!frames.original_compiled_cell_at_is_current(level, 0, &observed.cell));
    frames.pop();
    assert!(!frames.original_compiled_cell_at_is_current(level, 1, &observed.cell));
}

#[test]
fn original_compiled_name_keeps_its_selected_frame_across_caller_restoration() {
    let protocol = tcl_syntax::naming::NativeCompiledVariableProtocol::for_native_point(
        tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0)
            .execution_point()
            .unwrap(),
    )
    .unwrap();
    let mut frames = FrameStack::new();
    let namespaces = Namespaces::new();
    let level = frames.push(crate::namespace::GLOBAL);
    let names = [b"same".to_vec(), b"same".to_vec()];
    frames.install_formal_cells(&names, protocol);
    let mut selected = prepare_original_c_indexed_name_cell(&mut frames, &namespaces, 1).unwrap();
    assert!(selected.check_selected(&frames, &namespaces).is_ok());
    selected.compiled = Some(0);
    assert!(selected.check_selected(&frames, &namespaces).is_err());
    selected.compiled = Some(1);
    selected.home = VarHome::Namespace(crate::namespace::GLOBAL);
    assert!(selected.check_selected(&frames, &namespaces).is_err());
    selected.home = VarHome::Frame(level);

    frames.push(crate::namespace::GLOBAL);
    frames.install_formal_cells(&names, protocol);
    assert!(!frames.original_formal_cell_is_current(1, &selected.cell));
    assert!(selected.check_selected(&frames, &namespaces).is_ok());
    frames.pop();
    assert!(selected.check_selected(&frames, &namespaces).is_ok());
    frames.pop();
    assert!(selected.check_selected(&frames, &namespaces).is_err());
    assert_eq!(frames.push(crate::namespace::GLOBAL), level);
    frames.install_formal_cells(&names, protocol);
    assert!(selected.check_selected(&frames, &namespaces).is_err());
    let fresh = prepare_original_c_indexed_name_cell(&mut frames, &namespaces, 1).unwrap();
    assert!(fresh.check_selected(&frames, &namespaces).is_ok());
}

fn sobj(s: &[u8]) -> *mut TclObj {
    // fresh_zero string obj; the table takes the owning +1
    unsafe { Tcl_NewStringObj(s.as_ptr() as *const std::ffi::c_char, s.len() as isize) }
}

fn as_str(p: Option<*mut TclObj>) -> Option<Vec<u8>> {
    p.map(|obj| unsafe {
        let mut len: obj::TclSize = 0;
        let s = obj::get_string(obj, &mut len);
        std::slice::from_raw_parts(s as *const u8, len as usize).to_vec()
    })
}

/// Run `body` against a fresh `(frames, namespaces)` pair, then assert zero
/// residual once both are dropped — the leak gate.
fn leak_free(body: impl FnOnce(&mut FrameStack, &mut Namespaces)) {
    counters::reset();
    {
        let mut frames = FrameStack::new();
        let mut ns = Namespaces::new();
        body(&mut frames, &mut ns);
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
fn selected_scalar_trace_home_retains_the_actual_undefined_root_binding() {
    // naming.variable.original-array-read-destruction
    // docs/design/analysis/name-resolution-proofs/variable-original-array-read-destruction.md
    // Rust cell correspondence only; native public rows do not observe IDs.
    leak_free(|frames, ns| {
        let selected = capture_variable_receiver(frames, ns, GLOBAL, b"fresh", None).unwrap();
        let home = trace_home(frames, ns, GLOBAL, b"fresh");
        let physical = table(frames, ns, VarHome::Namespace(GLOBAL)).unwrap();
        assert!(physical.binding_id(b"fresh").is_none());
        assert!(!physical.is_set(b"fresh"));
        assert!(selected.read().unwrap().is_none());
        assert_eq!(home.binding_id, selected.binding_id());
        assert!(home.binding_id.is_some());
        assert!(home
            .for_selected_receiver(&selected)
            .unwrap()
            .selected_member
            .is_none());
        let other = capture_variable_receiver(frames, ns, GLOBAL, b"other", None).unwrap();
        assert!(home.for_selected_receiver(&other).is_none());
        let value = crate::obj::Owned::fresh(sobj(b"value"));
        selected.store(value.as_ptr()).unwrap();
        drop(selected);
        unset(frames, ns, GLOBAL, b"fresh");
        let replacement = capture_variable_receiver(frames, ns, GLOBAL, b"fresh", None).unwrap();
        assert!(home.for_selected_receiver(&replacement).is_none());
    });
}

#[test]
fn selected_member_trace_home_requires_original_root_correspondence() {
    leak_free(|frames, ns| {
        let value = crate::obj::Owned::fresh(sobj(b"value"));
        set_elem(frames, ns, GLOBAL, b"a", b"k", value.as_ptr()).unwrap();
        set_elem(frames, ns, GLOBAL, b"b", b"k", value.as_ptr()).unwrap();
        let home = trace_home(frames, ns, GLOBAL, b"a");
        let first = capture_variable_receiver(frames, ns, GLOBAL, b"a", Some(b"k")).unwrap();
        let other = capture_variable_receiver(frames, ns, GLOBAL, b"b", Some(b"k")).unwrap();
        let selected = home.for_selected_receiver(&first).unwrap();
        assert_eq!(selected.ns, home.ns);
        assert_eq!(selected.base, home.base);
        assert_eq!(selected.selected_member, first.trace_member());
        assert!(home.for_selected_receiver(&other).is_none());
        drop(first);
        unset(frames, ns, GLOBAL, b"a");
        set_elem(frames, ns, GLOBAL, b"a", b"k", value.as_ptr()).unwrap();
        let replacement = capture_variable_receiver(frames, ns, GLOBAL, b"a", Some(b"k")).unwrap();
        assert!(home.for_selected_receiver(&replacement).is_none());
    });
}

#[test]
fn absent_name_recipe_refuses_publication_before_any_cell_is_created() {
    leak_free(|frames, ns| {
        ns.variable_name_protocol = None;
        let value = crate::obj::Owned::fresh(sobj(b"value"));
        assert_eq!(
            set(frames, ns, GLOBAL, b"a", value.as_ptr()),
            Err(VarError::NameProtocolUnavailable)
        );
        assert!(ns.var_table(GLOBAL).cell(b"a").is_none());
    });
}

#[test]
fn followed_alias_keeps_its_selected_byte_key_when_fresh_name_recipe_changes() {
    leak_free(|frames, ns| {
        ns.variable_name_protocol = Some(NativeNameProtocol::for_tcl_version(
            tcl_dialect::TclVersion::V8_5,
        ));
        set(frames, ns, GLOBAL, b"target\0tail", sobj(b"retained")).unwrap();
        frames.push(GLOBAL);
        make_upvar(
            frames,
            ns,
            GLOBAL,
            Link {
                original_jim_target: None,
                native_scalar_entry: None,
                native_element_entry: None,
                home: VarHome::Namespace(GLOBAL),
                name: b"target\0tail".to_vec(),
                elem: None,
                array_identity: None,
                array_cell: None,
            },
            b"alias",
        );
        ns.variable_name_protocol = Some(NativeNameProtocol::for_tcl_version(
            tcl_dialect::TclVersion::V8_4,
        ));
        assert_eq!(
            as_str(get(frames, ns, GLOBAL, b"alias")),
            Some(b"retained".to_vec())
        );
        assert_eq!(get(frames, ns, GLOBAL, b"::target\0tail"), None);
    });
}

#[test]
fn stable_alias_preparation_follows_existing_namespace_alias_before_capture() {
    // naming.tcloo.original-constant-link-introspection
    // docs/design/analysis/name-resolution-proofs/tcloo-original-constant-link-introspection.md
    // Cell correspondence control for the shared preparation owner. Actual
    // TclOO public results are tested independently in cmd_info; this test
    // does not attribute Rust cell identities to the native observation.
    leak_free(|frames, ns| {
        set(frames, ns, GLOBAL, b"G", sobj(b"9")).unwrap();
        make_upvar_in(
            frames,
            ns,
            GLOBAL,
            b"X",
            Link {
                original_jim_target: None,
                native_scalar_entry: None,
                native_element_entry: None,
                home: VarHome::Namespace(GLOBAL),
                name: b"G".to_vec(),
                elem: None,
                array_identity: None,
                array_cell: None,
            },
        );
        frames.push(GLOBAL);
        make_variable(frames, ns, GLOBAL, GLOBAL, b"X");
        let local = frames
            .table(frames.current_level())
            .unwrap()
            .cell(b"X")
            .unwrap();
        let Var::Link(link) = &*local else {
            panic!("actual local alias")
        };
        assert_eq!(link.home, VarHome::Namespace(GLOBAL));
        assert_eq!(link.name, b"G");
        assert!(link.native_scalar_entry.is_some());
        drop(local);
        assert_eq!(as_str(get(frames, ns, GLOBAL, b"X")), Some(b"9".to_vec()));
        set(frames, ns, GLOBAL, b"X", sobj(b"10")).unwrap();
        assert_eq!(
            as_str(get(frames, ns, GLOBAL, b"::G")),
            Some(b"10".to_vec())
        );
    });
}

#[test]
fn global_scalar_set_get_unset() {
    leak_free(|f, ns| {
        set(f, ns, GLOBAL, b"x", sobj(b"hello")).unwrap();
        assert_eq!(as_str(get(f, ns, GLOBAL, b"x")), Some(b"hello".to_vec()));
        set(f, ns, GLOBAL, b"x", sobj(b"world")).unwrap(); // overwrite releases
        assert_eq!(as_str(get(f, ns, GLOBAL, b"x")), Some(b"world".to_vec()));
        assert!(get(f, ns, GLOBAL, b"x").is_some());
        assert!(unset(f, ns, GLOBAL, b"x"));
        assert_eq!(get(f, ns, GLOBAL, b"x"), None);
    });
}

#[test]
fn plain_and_global_qualified_alias_the_same_var() {
    // The headline fix: `::x` and `x` at global scope are one variable.
    leak_free(|f, ns| {
        set(f, ns, GLOBAL, b"pinged", sobj(b"1")).unwrap();
        assert_eq!(as_str(get(f, ns, GLOBAL, b"::pinged")), Some(b"1".to_vec()));
        set(f, ns, GLOBAL, b"::pinged", sobj(b"2")).unwrap();
        assert_eq!(as_str(get(f, ns, GLOBAL, b"pinged")), Some(b"2".to_vec()));
        unset(f, ns, GLOBAL, b"pinged");
    });
}

#[test]
fn qualified_set_into_existing_namespace() {
    leak_free(|f, ns| {
        let a = ns.ensure_namespace(GLOBAL, b"::a");
        set(f, ns, GLOBAL, b"::a::x", sobj(b"5")).unwrap();
        assert_eq!(as_str(get(f, ns, GLOBAL, b"::a::x")), Some(b"5".to_vec()));
        // and it reads as the unqualified `x` *inside* ::a.
        assert_eq!(as_str(get(f, ns, a, b"x")), Some(b"5".to_vec()));
        unset(f, ns, GLOBAL, b"::a::x");
    });
}

#[test]
fn qualified_set_into_missing_namespace_errors() {
    leak_free(|f, ns| {
        // On the error path `set` does not take ownership, so the caller still
        // owns the fresh obj and must free it (the borrowed-on-error contract).
        let rejected = sobj(b"1");
        unsafe { obj::incr_ref_count(rejected) }; // caller owns +1
        assert_eq!(
            set(f, ns, GLOBAL, b"::nosuch::x", rejected),
            Err(VarError::NoSuchNamespace)
        );
        // The caller frees the rejected object because the failed set did
        // not retain it. A read of the same name simply misses.
        unsafe { obj::decr_ref_count(rejected) };
        assert_eq!(get(f, ns, GLOBAL, b"::nosuch::x"), None);
    });
}

#[test]
fn qualified_array_element() {
    leak_free(|f, ns| {
        ns.ensure_namespace(GLOBAL, b"::a");
        set_elem(f, ns, GLOBAL, b"::a::arr", b"k", sobj(b"v")).unwrap();
        assert_eq!(
            as_str(get_elem(f, ns, GLOBAL, b"::a::arr", b"k")),
            Some(b"v".to_vec())
        );
        assert!(is_array(f, ns, GLOBAL, b"::a::arr"));
        unset(f, ns, GLOBAL, b"::a::arr");
    });
}

#[test]
fn upvar_links_local_to_namespace_var() {
    leak_free(|f, ns| {
        let a = ns.ensure_namespace(GLOBAL, b"::a");
        set(f, ns, GLOBAL, b"::a::x", sobj(b"5")).unwrap();
        make_upvar(
            f,
            ns,
            GLOBAL,
            Link {
                original_jim_target: None,
                native_scalar_entry: None,
                native_element_entry: None,
                array_identity: None,
                array_cell: None,
                home: VarHome::Namespace(a),
                name: b"x".to_vec(),
                elem: None,
            },
            b"y",
        );
        assert_eq!(as_str(get(f, ns, GLOBAL, b"y")), Some(b"5".to_vec()));
        // write through the link updates the namespace var
        set(f, ns, GLOBAL, b"y", sobj(b"99")).unwrap();
        assert_eq!(as_str(get(f, ns, GLOBAL, b"::a::x")), Some(b"99".to_vec()));
        // `unset y` follows the link and unsets the *target* (C Tcl); the link
        // cell remains but now points at an unset var.
        unset(f, ns, GLOBAL, b"y");
        assert_eq!(get(f, ns, GLOBAL, b"::a::x"), None);
        assert_eq!(get(f, ns, GLOBAL, b"y"), None);
        // the residual link cell owns nothing and is released on table drop.
    });
}

#[test]
fn global_in_proc_links_to_global() {
    leak_free(|f, ns| {
        set(f, ns, GLOBAL, b"g", sobj(b"global-val")).unwrap();
        f.push(GLOBAL); // enter a proc frame
        assert_eq!(get(f, ns, GLOBAL, b"g"), None); // not visible without `global`
        make_variable(f, ns, GLOBAL, GLOBAL, b"g"); // `global g` == variable in :: context
        assert_eq!(
            as_str(get(f, ns, GLOBAL, b"g")),
            Some(b"global-val".to_vec())
        );
        set(f, ns, GLOBAL, b"g", sobj(b"updated")).unwrap(); // through the link
        f.pop();
        assert_eq!(as_str(get(f, ns, GLOBAL, b"g")), Some(b"updated".to_vec()));
        unset(f, ns, GLOBAL, b"g");
    });
}

#[test]
fn unqualified_in_proc_is_frame_local() {
    // Inside a proc, `set v` is a local — it does NOT touch the namespace var.
    leak_free(|f, ns| {
        let a = ns.ensure_namespace(GLOBAL, b"::a");
        set(f, ns, GLOBAL, b"::a::v", sobj(b"10")).unwrap();
        f.push(a);
        set(f, ns, a, b"v", sobj(b"99")).unwrap(); // current_ns = ::a, but in a proc
        assert_eq!(as_str(get(f, ns, a, b"v")), Some(b"99".to_vec())); // the local
        f.pop();
        assert_eq!(as_str(get(f, ns, GLOBAL, b"::a::v")), Some(b"10".to_vec())); // untouched
        unset(f, ns, GLOBAL, b"::a::v");
    });
}

#[test]
fn variable_in_proc_links_to_namespace_var() {
    leak_free(|f, ns| {
        let a = ns.ensure_namespace(GLOBAL, b"::a");
        f.push(a);
        make_variable(f, ns, a, a, b"v"); // `variable v` inside a proc of ::a
        set(f, ns, a, b"v", sobj(b"7")).unwrap(); // writes through to ::a::v
        f.pop();
        assert_eq!(as_str(get(f, ns, GLOBAL, b"::a::v")), Some(b"7".to_vec()));
        unset(f, ns, GLOBAL, b"::a::v");
    });
}

#[test]
fn global_at_top_level_is_a_noop() {
    // `global g` at global scope must not create a self-link (which would loop).
    leak_free(|f, ns| {
        make_variable(f, ns, GLOBAL, GLOBAL, b"g");
        assert_eq!(get(f, ns, GLOBAL, b"g"), None); // still unset, no link installed
        set(f, ns, GLOBAL, b"g", sobj(b"1")).unwrap();
        assert_eq!(as_str(get(f, ns, GLOBAL, b"g")), Some(b"1".to_vec()));
        unset(f, ns, GLOBAL, b"g");
    });
}
