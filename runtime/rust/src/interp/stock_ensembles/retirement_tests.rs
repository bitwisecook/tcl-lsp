// SPDX-License-Identifier: AGPL-3.0-or-later
//! Genuine configured-role retirement callbacks; software continuation controls.

use super::*;
use crate::{
    ensemble::NativeEnsembleRoot,
    obj::{self, Owned},
};
use std::cell::{Cell, RefCell};
use tcl_cmd_core::ensemble::EnsembleObjectRole;

fn native(profile: &str) -> Interp {
    Interp::with_native_core(
        super::super::default_host(),
        crate::environment::profile_for_dialect(profile),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
fn noop(_: &mut Interp, _: &[*mut TclObj]) -> Code {
    Code::Ok
}
thread_local! {
    static OWNER: RefCell<Option<super::super::WeakInterp>> = const { RefCell::new(None) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static CALLS: Cell<usize> = const { Cell::new(0) };
    static COUNTS: RefCell<Vec<obj::TclSize>> = const { RefCell::new(Vec::new()) };
    static OBSERVED_GENERATION: Cell<u64> = const { Cell::new(0) };
    static REACHED: Cell<*mut TclObj> = const { Cell::new(std::ptr::null_mut()) };
    static ROOT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}
struct Reset;
impl Drop for Reset {
    fn drop(&mut self) {
        OWNER.with(|v| v.borrow_mut().take());
    }
}
fn arm(interpreter: &Interp, mode: u8, root: &[u8]) -> Reset {
    OWNER.with(|v| *v.borrow_mut() = Some(interpreter.downgrade()));
    MODE.with(|v| v.set(mode));
    CALLS.with(|v| v.set(0));
    COUNTS.with(|v| v.borrow_mut().clear());
    ROOT.with(|v| *v.borrow_mut() = root.to_vec());
    OBSERVED_GENERATION.with(|v| v.set(0));
    REACHED.with(|v| v.set(std::ptr::null_mut()));
    Reset
}
extern "C" fn retired(original: *mut TclObj) {
    CALLS.with(|v| v.set(v.get() + 1));
    COUNTS.with(|v| v.borrow_mut().push(unsafe { (*original).ref_count }));
    let Some(mut interpreter) = OWNER.with(|v| {
        v.borrow()
            .as_ref()
            .and_then(super::super::WeakInterp::upgrade)
    }) else {
        return;
    };
    assert!(interpreter.stock_ensembles.try_borrow_mut().is_ok());
    assert!(interpreter.namespaces.try_borrow_mut().is_ok());
    assert!(interpreter.frames.try_borrow_mut().is_ok());
    let root = ROOT.with(|v| v.borrow().clone());
    let current = interpreter
        .namespaces()
        .resolve_generation(GLOBAL, &root)
        .unwrap();
    OBSERVED_GENERATION.with(|v| v.set(current));
    interpreter.set_result_bytes(b"STOCK_CONFIG_REACHED");
    REACHED.with(|v| v.set(interpreter.result_obj()));
    match MODE.with(Cell::get) {
        1 => {
            interpreter.refuse_host_command("original stock configuration retirement");
        }
        2 => {
            let original_context = interpreter.runtime_context();
            let mut changed = original_context.clone();
            changed
                .packages
                .push(("stock-config-retirement".into(), "1.0".into()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original_context).unwrap();
        }
        3 => {
            interpreter.register_builtin(&root, noop);
        }
        _ => {}
    }
}
static CONFIG: obj::TclObjType = obj::TclObjType {
    name: c"originalStockConfigurationRetirement".as_ptr(),
    free_int_rep_proc: Some(retired),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
fn installed(public: &'static [u8], generation: u64, owns_config: bool) -> InstalledFamily {
    let config = owns_config.then(|| {
        // This private software fixture supplies a real owning configuration
        // role. No public configuration issuer or Native chronology is claimed.
        let original = Owned::fresh(obj::alloc_typed(&CONFIG, 0));
        assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
        let default_originals = crate::ensemble::NativeEnsembleObjects::default();
        let unknown = Some(EnsembleObjectRole::new(NativeEnsembleRoot::owned(original)));
        let originals = crate::ensemble::NativeEnsembleObjects {
            unknown,
            ..default_originals
        };
        EnsembleConfig {
            originals,
            ns: GLOBAL,
            map: None,
            subcommands: None,
            prefixes: true,
            parameters: Vec::new(),
            unknown: Vec::new(),
        }
    });
    InstalledFamily {
        public,
        generation,
        config,
        compiler: noop,
        scripted: false,
        scripted_helpers: Vec::new(),
        members: Vec::new(),
        namespaces: Vec::new(),
        nested: Vec::new(),
    }
}
fn first_is_original(interpreter: &Interp, mode: u8) {
    let first = interpreter.native_execution_refusal().unwrap();
    if mode == 1 {
        assert!(
            matches!(first, tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first)
            if first.reason == "original stock configuration retirement")
        );
    } else if mode == 3 {
        assert!(matches!(
            first,
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original command publication binding changed"
                )
            )
        ));
    } else {
        assert!(mode == 2);
    }
}

#[test]
fn original_stock_family_replacement_retires_configuration_outside_borrows_before_continuation() {
    // naming.embedding.original-host-publication-and-fact-transport

    // A private configured role supplies its actual +1; slot observations add none.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        for mode in 0..4 {
            crate::counters::reset();
            {
                let mut interpreter = native(profile);
                let family = EnsembleImplementationFamily::String;
                interpreter.register_builtin(b"stockRetirementRoot", noop);
                let generation = interpreter
                    .namespaces()
                    .resolve_generation(GLOBAL, b"stockRetirementRoot")
                    .unwrap();
                let old = interpreter
                    .stock_ensembles
                    .borrow_mut()
                    .families
                    .insert(family, installed(b"stockRetirementRoot", generation, true));
                drop(old);
                let _reset = arm(&interpreter, mode, b"stockRetirementRoot");
                let scope = NativeOperationScope::enter(&interpreter).unwrap();
                let publication = OriginalStockSlots::capture(&interpreter, [generation]).unwrap();
                let continued = interpreter.publish_stock_family(
                    family,
                    installed(b"stockRetirementRoot", generation, false),
                    &scope,
                    Some(&publication),
                );
                assert_eq!(CALLS.with(Cell::get), 1, "{profile}/{mode}");
                assert_eq!(COUNTS.with(|v| v.borrow().clone()), [0]);
                assert_eq!(OBSERVED_GENERATION.with(Cell::get), generation);
                assert_eq!(continued, mode == 0, "{profile}/{mode}");
                assert_eq!(interpreter.result_obj(), REACHED.with(Cell::get));
                assert_eq!(
                    interpreter.stock_ensembles.borrow().families[&family].generation,
                    generation
                );
                if mode == 0 {
                    assert!(interpreter.native_execution_refusal().is_none());
                } else {
                    first_is_original(&interpreter, mode);
                }
            }
            assert_eq!(crate::counters::finalize(), 0, "{profile}/{mode}");
            assert_eq!(crate::counters::double_free_count(), 0);
        }
    }
}

#[test]
fn original_stock_family_cleanup_restores_reached_records_after_first_refusal_without_reentry_grant(
) {
    // naming.embedding.original-host-publication-and-fact-transport

    let mut interpreter = native("tcl8.6");
    interpreter.register_builtin(b"stockRetirementRoot", noop);
    let generation = interpreter
        .namespaces()
        .resolve_generation(GLOBAL, b"stockRetirementRoot")
        .unwrap();
    let family = EnsembleImplementationFamily::String;
    let old = interpreter
        .stock_ensembles
        .borrow_mut()
        .families
        .insert(family, installed(b"stockRetirementRoot", generation, true));
    drop(old);
    let _reset = arm(&interpreter, 0, b"stockRetirementRoot");
    let scope = NativeOperationScope::enter(&interpreter).unwrap();
    interpreter.refuse_host_command("prior stock configuration refusal");
    interpreter.restore_stock_families(
        std::iter::once((family, installed(b"stockRetirementRoot", generation, false))),
        &scope,
    );
    assert_eq!(CALLS.with(Cell::get), 1);
    assert_eq!(COUNTS.with(|v| v.borrow().clone()), [0]);
    assert_eq!(interpreter.result_obj(), REACHED.with(Cell::get));
    assert_eq!(
        interpreter.stock_ensembles.borrow().families[&family].generation,
        generation
    );
    assert!(matches!(interpreter.native_execution_refusal().unwrap(),
        tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first)
            if first.reason == "prior stock configuration refusal"));
    assert!(scope.currency().ensure_current_or_refuse().is_err());
}

#[test]
fn original_stock_refresh_checks_new_root_after_the_extracted_old_configuration_retires() {
    // naming.embedding.original-host-publication-and-fact-transport

    // One real refresh family, plus its private owning-role fixture. No Native chronology.
    for mode in 0..4 {
        let mut interpreter = native("tcl8.6");
        let family = EnsembleImplementationFamily::String;
        let extracted = std::mem::take(&mut interpreter.stock_ensembles.borrow_mut().families);
        let mut selected = None;
        for (key, value) in extracted {
            if key == family {
                selected = Some(value);
            } else {
                drop(value);
            }
        }
        let mut original = selected.unwrap();
        let generation = original.generation;
        let owned = installed(b"string", generation, true)
            .config
            .unwrap()
            .originals;
        // Configuration equality compares semantic fields. This software
        // owning role is held only by the extracted bookkeeping record.
        original.config.as_mut().unwrap().originals = owned;
        interpreter
            .stock_ensembles
            .borrow_mut()
            .families
            .insert(family, original);
        let _reset = arm(&interpreter, mode, b"string");
        let scope = NativeOperationScope::enter(&interpreter).unwrap();
        interpreter.refresh_stock_ensembles();
        assert_eq!(CALLS.with(Cell::get), 1);
        assert_eq!(COUNTS.with(|v| v.borrow().clone()), [0]);
        let new_generation = OBSERVED_GENERATION.with(Cell::get);
        assert_ne!(new_generation, generation);
        assert_eq!(
            interpreter.stock_ensembles.borrow().families[&family].generation,
            new_generation
        );
        assert_eq!(interpreter.result_obj(), REACHED.with(Cell::get));
        assert_eq!(
            scope.currency().ensure_current_or_refuse().is_ok(),
            mode == 0
        );
        if mode != 0 {
            first_is_original(&interpreter, mode);
        }
    }
}
