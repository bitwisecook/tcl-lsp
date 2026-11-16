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

//! Original name caches: the existence purpose, the alias and global caches, and the order the local cache is consulted and kept.

use super::*;

fn exists_cache_purpose_probe(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    let version = interp.native_c_variable_name_protocol().unwrap().version();
    for name in [b"x".as_slice(), b"a(k)", b"x\0tail"] {
        let original = Owned::fresh(obj::new_string_bytes(name));
        let before = obj::obj_type_ptr(original.as_ptr());
        assert!(interp
            .exists_original_c_parts(original.as_ptr(), None)
            .unwrap());
        if version == tcl_dialect::TclVersion::V8_4 {
            assert_eq!(obj::obj_type_ptr(original.as_ptr()), before);
        } else {
            assert_ne!(obj::obj_type_ptr(original.as_ptr()), before);
        }
    }
    let member = Owned::fresh(obj::new_string_bytes(b"x"));
    let original = Owned::fresh(crate::list::new_list_obj(&[member.as_ptr()]));
    let before = obj::obj_type_ptr(original.as_ptr());
    assert!(interp
        .exists_original_c_parts(original.as_ptr(), None)
        .unwrap());
    if version == tcl_dialect::TclVersion::V8_4 {
        assert_eq!(obj::obj_type_ptr(original.as_ptr()), before);
        assert!(interp.read_original_c_variable(original.as_ptr()).is_ok());
        assert_ne!(
            obj::obj_type_ptr(original.as_ptr()),
            before,
            "actual object read still selects its original name cache"
        );
    } else {
        assert_ne!(obj::obj_type_ptr(original.as_ptr()), before);
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

#[test]
fn legacy_existence_preserves_original_name_caches_and_object_reads_still_adopt_them() {
    // naming.variable.original-upvar-and-exists-completion-and-name-windows
    // docs/design/analysis/name-resolution-proofs/variable.original-upvar-and-exists-completion-and-name-windows.md
    // Native case16 binds the fresh scalar primary. The list-cache,
    // combined name and NUL controls separately test this source/API owner.
    for engine in ["tcl8.4", "tcl8.5"] {
        let mut interp = Interp::with_native_core(
            super::super::super::default_host(),
            crate::environment::profile_for_dialect(engine),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        interp.register_builtin(b"existsCachePurpose", exists_cache_purpose_probe);
        let code =
            interp.eval_str(b"proc run {} {set x LOCAL; set a(k) VALUE; existsCachePurpose}; run");
        assert_eq!(code, Code::Ok, "{engine}: {:?}", interp.result_bytes());
        assert!(!interp.host_refusal_pending());
    }
}

fn alias_simple_probe(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    for (case, name) in [b"v".as_slice(), b"d", b"n\0(k)"].into_iter().enumerate() {
        let local = if case == 2 {
            Owned::fresh(obj::new_string_bytes(name))
        } else {
            let member = Owned::fresh(obj::new_string_bytes(name));
            Owned::fresh(crate::list::new_list_obj(&[member.as_ptr()]))
        };
        let kind = if case == 2 { "NULL" } else { "list" };
        // SAFETY: local retains the original object throughout this callback.
        let references = unsafe { (*local.as_ptr()).ref_count };
        ALIAS_SIMPLE_ROWS.with(|rows| {
            rows.borrow_mut()
                .push(format!("before{case}|{kind}|{references}"))
        });
        let original_type = obj::obj_type_ptr(local.as_ptr());
        let target = Owned::fresh(obj::new_string_bytes(b"x"));
        let mut link = match interp.prepare_original_c_link_target(target.as_ptr(), 0) {
            Ok(Some(link)) => link,
            Ok(None) => panic!("actual C target unavailable"),
            Err(code) => return code,
        };
        interp.prepare_upvar_target(&mut link).unwrap();
        let code = interp.bind_original_c_alias_local(local.as_ptr(), link);
        if code != Code::Ok {
            return code;
        }
        assert_eq!(obj::obj_type_ptr(local.as_ptr()), original_type);
        // SAFETY: local retains the original object after alias installation.
        let references = unsafe { (*local.as_ptr()).ref_count };
        ALIAS_SIMPLE_ROWS.with(|rows| {
            rows.borrow_mut()
                .push(format!("after{case}|0|{kind}|{references}"))
        });
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

fn global_cache_probe(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let original = argv[1];
    let value = match interp.read_original_named_variable(original) {
        Ok(value) => value,
        Err(code) => return code,
    };
    let kind = obj::obj_type_ptr(original);
    assert!(
        !kind.is_null(),
        "successful original-name lookup owns a cache"
    );
    // SAFETY: original owns its selected descriptor throughout this callback.
    let name = unsafe { std::ffi::CStr::from_ptr((*kind).name).to_str().unwrap() };
    let bytes = interp.native_string_bytes(&value).unwrap();
    GLOBAL_CACHE_ROWS.with(|rows| {
        rows.borrow_mut().push(format!(
            "{name}|{}",
            String::from_utf8(bytes.to_vec()).unwrap(),
        ))
    });
    interp.set_result_bytes(b"");
    Code::Ok
}

#[test]
fn dynamic_global_uses_original_compiler_token_tail_and_name_cache() {
    // Native proof: naming.variable.original-global-cache-token
    // docs/design/analysis/name-resolution-proofs/variable-original-global-cache-token.md
    for (engine, expected) in [
        (
            "tcl8.4",
            include_str!("../../../../tests/data/native_global_cache_token/8.4.20.txt"),
        ),
        (
            "tcl8.5",
            include_str!("../../../../tests/data/native_global_cache_token/8.5.19.txt"),
        ),
        (
            "tcl8.6",
            include_str!("../../../../tests/data/native_global_cache_token/8.6.18.txt"),
        ),
        (
            "tcl9.0",
            include_str!("../../../../tests/data/native_global_cache_token/9.0.4.txt"),
        ),
        (
            "tcl9.1",
            include_str!("../../../../tests/data/native_global_cache_token/9.1.0.txt"),
        ),
    ] {
        crate::counters::reset();
        GLOBAL_CACHE_ROWS.with(|rows| rows.borrow_mut().clear());
        {
            let mut interp = Interp::with_native_core(
                super::super::super::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            interp.register_builtin(b"probe", global_cache_probe);
            let original = Owned::fresh(obj::new_string_bytes(b"v"));
            interp.var_set(b"name", original.as_ptr()).unwrap();
            drop(original);
            for (label, source) in [
                    ("simple", b"set v GLOBAL;proc p {} {set v LOCAL;probe $::name;unset v;global $::name;probe $::name;return $v};p".as_slice()),
                    ("qualified", b"namespace eval N {variable v QUALIFIED};set name ::N::v;p".as_slice()),
                ] {
                    let code = interp.eval_str(source);
                    GLOBAL_CACHE_ROWS.with(|rows| rows.borrow_mut().push(format!(
                        "{label}|{}|{}", code.as_int(), String::from_utf8(interp.result_bytes()).unwrap(),
                    )));
                    assert!(!interp.host_refusal_pending(), "{engine}");
                }
            let rows = GLOBAL_CACHE_ROWS.with(|rows| rows.borrow().join("\n") + "\n");
            assert_eq!(rows, expected, "{engine}");
        }
        assert_eq!(crate::counters::finalize(), 0, "{engine}");
        assert_eq!(crate::counters::double_free_count(), 0, "{engine}");
    }
}

#[test]
fn alias_local_simple_lookup_matches_all_15_native_primary_and_key_owners() {
    // Native proof: naming.alias.original-local-name-object
    // docs/design/analysis/name-resolution-proofs/alias-original-local-name-object.md
    for (engine, expected) in [
        (
            "tcl8.4",
            include_str!("../../../../tests/data/native_alias_simple/8.4.20.txt"),
        ),
        (
            "tcl8.5",
            include_str!("../../../../tests/data/native_alias_simple/8.5.19.txt"),
        ),
        (
            "tcl8.6",
            include_str!("../../../../tests/data/native_alias_simple/8.6.18.txt"),
        ),
        (
            "tcl9.0",
            include_str!("../../../../tests/data/native_alias_simple/9.0.4.txt"),
        ),
        (
            "tcl9.1",
            include_str!("../../../../tests/data/native_alias_simple/9.1.0.txt"),
        ),
    ] {
        crate::counters::reset();
        ALIAS_SIMPLE_ROWS.with(|rows| rows.borrow_mut().clear());
        {
            let mut interp = Interp::with_native_core(
                super::super::super::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            interp.register_builtin(b"alias_probe", alias_simple_probe);
            let code = interp.eval_str(b"set x X; proc p {} {alias_probe; return $v}; p");
            assert_eq!(code, Code::Ok, "{engine}: {:?}", interp.result_bytes());
            assert_eq!(interp.result_bytes(), b"X");
            ALIAS_SIMPLE_ROWS.with(|rows| rows.borrow_mut().push("completion|0|X".into()));
            let rows = ALIAS_SIMPLE_ROWS.with(|rows| rows.borrow().join("\n") + "\n");
            assert_eq!(rows, expected, "{engine}");
            assert!(!interp.host_refusal_pending());
        }
        assert_eq!(crate::counters::finalize(), 0, "{engine}");
        assert_eq!(crate::counters::double_free_count(), 0, "{engine}");
    }
}

struct CacheLookupObserver;

impl tcl_runtime_api::native_variable_trace::NativeVariableObserver<Interp>
    for CacheLookupObserver
{
    type Error = tcl_cmd_core::CmdError;
    fn observe(
        &self,
        _interp: &mut Interp,
        _access: tcl_runtime_api::native_variable_trace::NativeVariableTraceAccess<'_>,
    ) -> Result<(), Self::Error> {
        CACHE_LOOKUP_CALLBACKS.with(|count| count.set(count.get() + 1));
        Ok(())
    }
}

fn cache_lookup_probe(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    use tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation as Op;
    let original = argv[1];
    let case = CACHE_LOOKUP_CASE.with(std::cell::Cell::get);
    let version = interp.native_c_variable_name_protocol().unwrap().version();
    interp.read_original_c_variable(original).unwrap();
    let before = interp
        .original_c_local_index(original)
        .expect("genuine local cache");
    let before_slot_cell = interp
        .frames
        .borrow()
        .native_compiled_cell_identity(before)
        .expect("actual installed compiled cell");
    let selection = interp.prepare_original_c_name(original, false).unwrap();
    let before_cell = interp
        .capture_original_c_selection(original, &selection, NativeVariableNameLookupPurpose::Read)
        .unwrap()
        .unwrap()
        .1
        .binding_id;
    if matches!(case, 2 | 3) {
        interp
            .add_native_variable_observer(
                original,
                &[Op::Read, Op::Write],
                Rc::new(CacheLookupObserver),
            )
            .unwrap();
    }
    if case == 4 {
        assert!(interp.var_unset(b"x"));
    }
    obj::invalidate_string(original);
    let result = if matches!(case, 1 | 3) {
        let next = Owned::fresh(obj::new_string_bytes(b"NEXT"));
        interp.store_original_c_variable(original, next.as_ptr())
    } else {
        interp.read_original_c_variable(original).map(|_| ())
    };
    assert_eq!(interp.original_c_local_index(original), Some(before));
    assert_eq!(
        interp.frames.borrow().native_compiled_cell_identity(before),
        Some(before_slot_cell),
        "same original cell after {version:?}/{case}"
    );
    // Unset withdraws the value, while the actual compiled Var allocation
    // and the original cache keep their identity through undefinedness.
    assert_eq!(interp.trace_identity(b"x").binding_id, before_cell);
    if case == 4 {
        let frames = interp.frames.borrow();
        assert!(frames
            .table(frames.current_level())
            .unwrap()
            .binding_id(b"x")
            .is_none());
    }
    if version >= tcl_dialect::TclVersion::V8_5 && case >= 2 {
        assert!(result.is_err());
        assert!(
            interp.host_refusal_pending(),
            "native updater abort must remain a host refusal"
        );
        assert!(!obj::has_string_rep(original));
        assert_eq!(CACHE_LOOKUP_CALLBACKS.with(std::cell::Cell::get), 0);
    } else {
        assert_eq!(result.is_err(), case == 4);
        assert_eq!(
            obj::has_string_rep(original),
            version == tcl_dialect::TclVersion::V8_4
        );
        assert_eq!(
            CACHE_LOOKUP_CALLBACKS.with(std::cell::Cell::get),
            usize::from(matches!(case, 2 | 3))
        );
    }
    let version_label = match version {
        tcl_dialect::TclVersion::V8_4 => "8.4.20",
        tcl_dialect::TclVersion::V8_5 => "8.5.19",
        tcl_dialect::TclVersion::V8_6 => "8.6.18",
        tcl_dialect::TclVersion::V9_0 => "9.0.4",
        tcl_dialect::TclVersion::V9_1 => "9.1.0",
    };
    let row = if interp.host_refusal_pending() {
        format!("{version_label}|{case}|updater-unavailable")
    } else {
        format!(
            "{version_label}|{case}|{}|{}|localVarName|1",
            usize::from(result.is_err()),
            usize::from(obj::has_string_rep(original))
        )
    };
    CACHE_LOOKUP_ROWS.with(|rows| rows.borrow_mut().push(row));
    if case != 4 {
        // Inspect reached storage without entering another read after refusal.
        let frames = interp.frames.borrow();
        let stored = frames.compiled_slot_var(before).unwrap();
        let crate::frame::Var::Scalar(value) = &*stored else {
            panic!("actual compiled scalar contents");
        };
        assert!(obj::has_string_rep(*value));
        assert_eq!(
            obj::bytes_of(*value),
            if matches!(case, 1 | 3) {
                b"NEXT".as_slice()
            } else {
                b"VALUE".as_slice()
            }
        );
    }
    result.map_or_else(|code| code, |()| Code::Ok)
}

#[test]
fn original_local_cache_getter_order_matches_all_25_native_paths() {
    // Native proof: naming.variable.original-local-cache-before-getter
    // docs/design/analysis/name-resolution-proofs/variable-original-local-cache-before-getter.md
    // Native proof: naming.variable.original-local-cache-unavailable-updater
    // docs/design/analysis/name-resolution-proofs/variable-original-local-cache-unavailable-updater.md
    use super::super::*;
    let expected = include_str!(
        "../../../../../../rust/tcl-syntax/tests/data/native_variable_name/cache_lookup/paths.txt"
    );
    assert_eq!(expected.lines().count(), 25);
    CACHE_LOOKUP_ROWS.with(|rows| rows.borrow_mut().clear());
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for case in 0..5 {
            CACHE_LOOKUP_CASE.with(|mode| mode.set(case));
            CACHE_LOOKUP_CALLBACKS.with(|count| count.set(0));
            let mut interp = Interp::with_native_core(
                super::super::super::default_host(),
                crate::environment::profile_for_dialect(environment),
                tcl_registry::special_vars::NativeBootstrapInputs {
                    package_path: Vec::new(),
                    default_library: None,
                },
            )
            .unwrap();
            interp.register_builtin(b"probe", cache_lookup_probe);
            let code = interp.eval_str(b"proc p {} {set x VALUE;probe x};p");
            assert_eq!(
                code == Code::Ok,
                case < 2 || (environment == "tcl8.4" && case < 4),
                "{environment}/{case}"
            );
        }
    }
    CACHE_LOOKUP_ROWS.with(|rows| {
        assert_eq!(
            rows.borrow().as_slice(),
            expected.lines().collect::<Vec<_>>()
        )
    });
}

#[test]
fn original_link_lookup_and_unset_preserve_target_name_cache() {
    use super::super::*;
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = Interp::new();
        interp.set_dialect_profile(crate::environment::profile_for_dialect(environment));
        let target = Owned::fresh(obj::new_string_bytes(b"target"));
        let prepared = interp
            .prepare_original_c_link_target(target.as_ptr(), 0)
            .unwrap();
        let prepared = prepared.expect("actual selected target");
        assert_eq!(prepared.name, b"target");
        assert_eq!(prepared.elem, None);
        assert!(
            obj::native_variable_name::with_parsed(target.as_ptr(), |cache| cache.array.is_none())
                .unwrap()
        );
        let value = Owned::fresh(obj::new_string_bytes(b"7"));
        interp
            .assign_original_named_variable(target.as_ptr(), value.as_ptr())
            .unwrap();
        interp
            .unset_original_c_variable(target.as_ptr(), true)
            .unwrap();
        assert!(
            obj::native_variable_name::with_parsed(target.as_ptr(), |cache| cache.array.is_none())
                .unwrap()
        );
        assert!(interp
            .read_original_named_variable(target.as_ptr())
            .is_err());
    }
}
