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

//! Original numeric frame caches retain no selected frame incarnation.

use super::{Interp, OriginalLevel};
use crate::obj::{self, Owned};
use tcl_registry::frame_effect::NativeFrameLevelObject;

fn actual(engine: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(engine),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}

fn push_frame(interp: &mut Interp) {
    let namespace = interp.current_ns();
    interp.enter_namespace_activation(namespace);
    interp.frames.borrow_mut().push(namespace);
}

fn pop_frame(interp: &mut Interp) {
    let namespace = interp.pop_native_call_frame();
    interp.leave_namespace_activation(namespace);
}

#[test]
fn level_reference_reselects_current_frames_in_thirty_native_windows() {
    let mut compared = 0;
    for (engine, rows) in [
        (
            "tcl8.4",
            include_str!(
                "../../../../rust/tcl-syntax/tests/data/native_frame_reference/8.4.20.tsv"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../../rust/tcl-syntax/tests/data/native_frame_reference/8.5.19.tsv"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../../rust/tcl-syntax/tests/data/native_frame_reference/8.6.18.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!("../../../../rust/tcl-syntax/tests/data/native_frame_reference/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../../../rust/tcl-syntax/tests/data/native_frame_reference/9.1.0.tsv"),
        ),
    ] {
        let mut interp = actual(engine);
        let mut other = actual(engine);
        let original = Owned::fresh(obj::new_string_bytes(b"#1"));
        let mut records = rows.lines();
        compare(&mut interp, &original, records.next().unwrap());
        push_frame(&mut interp);
        compare(&mut interp, &original, records.next().unwrap());
        pop_frame(&mut interp);
        compare(&mut interp, &original, records.next().unwrap());
        push_frame(&mut interp);
        compare(&mut interp, &original, records.next().unwrap());
        push_frame(&mut other);
        compare(&mut other, &original, records.next().unwrap());
        let duplicate = Owned::fresh(obj::duplicate(original.as_ptr()));
        drop(original);
        compare(&mut other, &duplicate, records.next().unwrap());
        assert!(records.next().is_none());
        pop_frame(&mut other);
        pop_frame(&mut interp);
        compared += 6;
    }
    assert_eq!(compared, 30);
}

fn compare(interp: &mut Interp, value: &Owned, record: &str) {
    let fields: Vec<_> = record.split('\t').collect();
    let dialect = interp.eval_frame_dialect();
    let mut original = OriginalLevel::selected(interp, value.as_ptr()).unwrap();
    let result = dialect
        .native_frame_level_protocol()
        .unwrap()
        .resolve_object(interp.current_level(), &mut original)
        .unwrap();
    assert_eq!(
        result.as_ref().map_or(-1, |_| 1),
        fields[1].parse::<i32>().unwrap()
    );
    assert_eq!(
        result
            .as_ref()
            .map_or(-1, |selected| i32::try_from(selected.target).unwrap()),
        fields[2].parse::<i32>().unwrap()
    );
    let snapshot = obj::native_object_snapshot(value.as_ptr()).unwrap();
    assert_eq!(snapshot.resident.is_some(), fields[4] == "1");
    if fields[3] == "levelReference" {
        assert!(matches!(
            snapshot.cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::FrameReference {
                relative: false,
                level: 1,
                ..
            }
        ));
        assert_eq!(
            original.native_frame_cache().unwrap(),
            Some(tcl_registry::NativeFrameLevelCache::Absolute(1))
        );
    } else {
        assert_eq!(
            snapshot.cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::None
        );
    }
    // SAFETY: the owning original remains live throughout this header observation.
    unsafe {
        assert_eq!(
            (*value.as_ptr()).ref_count,
            fields[5].parse::<isize>().unwrap()
        );
        let descriptor = (*value.as_ptr()).type_ptr;
        let hooks = if descriptor.is_null() {
            [false; 4]
        } else {
            [
                (*descriptor).free_int_rep_proc.is_some(),
                (*descriptor).dup_int_rep_proc.is_some(),
                (*descriptor).update_string_proc.is_some(),
                (*descriptor).set_from_any_proc.is_some(),
            ]
        };
        for (actual, expected) in hooks.iter().zip(&fields[6..]) {
            assert_eq!(*actual, *expected == "1");
        }
    }
}

#[test]
fn frame_reference_cache_refuses_foreign_issuers_and_retired_headers() {
    let interp = actual("tcl8.5");
    let original = Owned::fresh(obj::new_string_bytes(b"#1"));
    let dialect = interp.eval_frame_dialect();
    obj::install_native_frame_level_cache(
        original.as_ptr(),
        tcl_registry::NativeFrameLevelCache::Absolute(1),
        dialect,
    )
    .unwrap();
    let foreign = actual("tcl8.6");
    assert!(foreign
        .native_object_string_bytes(original.as_ptr())
        .is_err());
    let lifetime = obj::ProcedureObject::retain_lifetime(&original);
    drop(original);
    assert!(obj::native_frame_level_cache_in(lifetime.as_ptr(), dialect).is_err());
    assert!(obj::install_native_frame_level_cache(
        lifetime.as_ptr(),
        tcl_registry::NativeFrameLevelCache::Absolute(1),
        dialect
    )
    .is_err());
    // The normal live interpreter's unrelated objects retain their own recipe.
    let plain = Owned::fresh(obj::new_string_bytes(b"plain"));
    assert_eq!(
        interp
            .native_object_string_bytes(plain.as_ptr())
            .unwrap()
            .as_ref(),
        b"plain"
    );
}

#[test]
fn required_upvar_level_does_not_borrow_optional_leading_frame_default() {

    // Case11's unchanged original C8.6/9.0/9.1 native tables require bad level k.
    for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = actual(engine);
        push_frame(&mut interp);
        let original = Owned::fresh(obj::new_string_bytes(b"k"));
        let dialect = interp.eval_frame_dialect();
        let protocol = dialect.native_frame_level_protocol().unwrap();
        let mut operand = OriginalLevel::selected(&interp, original.as_ptr()).unwrap();
        let optional = protocol
            .resolve_object(interp.current_level(), &mut operand)
            .unwrap()
            .unwrap();
        assert!(!optional.explicit);
        assert_eq!(optional.target, 0);
        for current in [0, interp.current_level()] {
            let result = protocol
                .resolve_required_object(current, &mut operand)
                .unwrap();
            assert!(matches!(result,
                Err(tcl_registry::frame_effect::NativeFrameLevelFailure::BadLevel { name, lookup_code: true, .. })
                    if name == b"k"
            ));
        }
        let x = Owned::fresh(obj::new_string_bytes(b"x"));
        let alias = Owned::fresh(obj::new_string_bytes(b"alias"));
        assert!(super::select_frame(
            &mut interp,
            &[original.as_ptr(), x.as_ptr(), alias.as_ptr()],
            tcl_registry::FrameEffectSpec::UPVAR,
        )
        .is_err());
        assert!(matches!(
            obj::native_object_snapshot(interp.result_obj()).unwrap().cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::String { protocol: producer, .. }
                if Some(producer) == protocol.bad_level_string_result()
        ));
        assert!(obj::has_string_rep(interp.result_obj()));
        assert_eq!(interp.result_bytes(), b"bad level \"k\"");
        assert!(!interp.host_refusal_pending());
        pop_frame(&mut interp);
    }
}

use std::{cell::Cell, rc::Rc};
use tcl_platform::{
    Capabilities, Clock, DoubleNumericConversion, Env, Host, NativeCIntegerAbi, NumericEnvironment,
    NumericEnvironmentUnavailable, NumericErrorState, SignedNumericConversion, StdIo,
    UnsignedNumericConversion,
};

struct FrameHost {
    actual: Rc<dyn Host>,
    numeric: bool,
    change_at: Cell<u8>,
    abi_calls: Cell<usize>,
    reset_calls: Cell<usize>,
    unsigned_calls: Cell<usize>,
}
impl FrameHost {
    fn selected(interp: &Interp, numeric: bool) -> Rc<Self> {
        Rc::new(Self {
            actual: interp.host(),
            numeric,
            change_at: Cell::new(0),
            abi_calls: Cell::new(0),
            reset_calls: Cell::new(0),
            unsigned_calls: Cell::new(0),
        })
    }
    fn actual_numeric(&self) -> &dyn NumericEnvironment {
        self.actual
            .numeric_environment()
            .expect("actual native test Host")
    }
    fn change(&self, stage: u8) {
        if self.change_at.get() == stage {
            self.change_at.set(0);
            let mut interp = unsafe { crate::codegen_abi::current_interp().as_ref() }
                .expect("actual test interpreter")
                .clone();
            change_frame_context(&mut interp);
        }
    }
}
impl Host for FrameHost {
    fn capabilities(&self) -> Capabilities {
        self.actual.capabilities()
    }
    fn clock(&self) -> &dyn Clock {
        self.actual.clock()
    }
    fn stdio(&self) -> &dyn StdIo {
        self.actual.stdio()
    }
    fn env(&self) -> &dyn Env {
        self.actual.env()
    }
    fn numeric_environment(&self) -> Option<&dyn NumericEnvironment> {
        self.numeric.then_some(self)
    }
    fn native_integer_formatter(&self) -> Option<&dyn tcl_platform::NativeIntegerFormatter> {
        self.actual.native_integer_formatter()
    }
}
impl NumericEnvironment for FrameHost {
    fn c_integer_abi(&self) -> Result<NativeCIntegerAbi, NumericEnvironmentUnavailable> {
        self.abi_calls.set(self.abi_calls.get() + 1);
        let result = self.actual_numeric().c_integer_abi();
        self.change(1);
        result
    }
    fn state(&self) -> Result<NumericErrorState, NumericEnvironmentUnavailable> {
        self.actual_numeric().state()
    }
    fn reset(&self) -> Result<(), NumericEnvironmentUnavailable> {
        self.reset_calls.set(self.reset_calls.get() + 1);
        self.actual_numeric().reset()
    }
    fn unsigned_c84(
        &self,
        input: &[u8],
        offset: usize,
        long: bool,
    ) -> Result<UnsignedNumericConversion, NumericEnvironmentUnavailable> {
        self.unsigned_calls.set(self.unsigned_calls.get() + 1);
        let result = self.actual_numeric().unsigned_c84(input, offset, long);
        self.change(2);
        result
    }
    fn unsigned(
        &self,
        input: &[u8],
        offset: usize,
        base: u32,
    ) -> Result<UnsignedNumericConversion, NumericEnvironmentUnavailable> {
        self.actual_numeric().unsigned(input, offset, base)
    }
    fn signed_long(
        &self,
        input: &[u8],
        base: u32,
    ) -> Result<SignedNumericConversion, NumericEnvironmentUnavailable> {
        self.actual_numeric().signed_long(input, base)
    }
    fn double(
        &self,
        input: &[u8],
        reset: bool,
    ) -> Result<DoubleNumericConversion, NumericEnvironmentUnavailable> {
        self.actual_numeric().double(input, reset)
    }
}

struct CurrentFrameInterp(*mut Interp);
impl CurrentFrameInterp {
    fn enter(interp: &mut Interp) -> Self {
        let previous = crate::codegen_abi::current_interp();
        crate::codegen_abi::tcl_runtime_set_current_interp(interp);
        Self(previous)
    }
}
impl Drop for CurrentFrameInterp {
    fn drop(&mut self) {
        crate::codegen_abi::tcl_runtime_set_current_interp(self.0);
    }
}
fn change_frame_context(interp: &mut Interp) {
    let original = interp.runtime_context();
    let mut changed = original.clone();
    changed.packages = vec![("original-frame-test".to_owned(), "1.0".to_owned())];
    interp.pin_context(&changed).unwrap();
    interp.pin_context(&original).unwrap();
}
thread_local! {
    static FRAME_UPDATER_CALLS: Cell<usize> = const { Cell::new(0) };
    static FRAME_UPDATER_CHANGE: Cell<bool> = const { Cell::new(false) };
}
extern "C" fn frame_updater(original: *mut obj::TclObj) {
    FRAME_UPDATER_CALLS.with(|calls| calls.set(calls.get() + 1));
    let mut interp = unsafe { crate::codegen_abi::current_interp().as_ref() }
        .expect("original frame test interpreter")
        .clone();
    if FRAME_UPDATER_CHANGE.with(Cell::get) {
        change_frame_context(&mut interp);
    } else {
        interp.refuse_host_command("first original frame updater refusal");
    }
    // Preserve the reached original String effect before the checked refusal.
    unsafe { obj::set_native_updater_string_rep(original, b"1", false) };
}
static FRAME_UPDATER: obj::TclObjType = obj::TclObjType {
    name: c"originalFrameUpdater".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(frame_updater),
    set_from_any_proc: None,
};

#[test]
fn original_frame_level_uses_entered_environment_and_preserves_native_counts() {

    // Software operation-owner control; existing provider recordings remain independent.
    for engine in ["tcl8.4", "tcl8.6", "tcl9.1", "jimtcl"] {
        let mut interp = actual(engine);
        let host = FrameHost::selected(&interp, true);
        interp.set_host(host.clone());
        push_frame(&mut interp);
        let original = Owned::fresh(obj::new_string_bytes(b"1"));
        let before = unsafe { (*original.as_ptr()).ref_count };
        assert!(obj::scalar_object_context(original.as_ptr())
            .unwrap()
            .is_none());
        let protocol = interp
            .eval_frame_dialect()
            .native_frame_level_protocol()
            .unwrap();
        let mut operand = OriginalLevel::selected(&interp, original.as_ptr()).unwrap();
        let result = protocol.resolve_object(1, &mut operand).unwrap().unwrap();
        assert_eq!(result.target, 0, "{engine}");
        assert!(result.explicit);
        operand.check().unwrap();
        assert!(host.abi_calls.get() > 0, "{engine}");
        if engine == "tcl8.4" {
            assert!(host.reset_calls.get() > 0);
            assert!(host.unsigned_calls.get() > 0);
            assert!(obj::native_scalar_cache(original.as_ptr())
                .unwrap()
                .is_none());
        }
        assert_eq!(unsafe { (*original.as_ptr()).ref_count }, before);
        assert!(obj::scalar_object_context(original.as_ptr())
            .unwrap()
            .is_none());
        drop(operand);
        assert_eq!(interp.entered_native_operation_count(), 0);
        pop_frame(&mut interp);
    }
    // An independently bound C-API scalar issuer does not bind a portable
    // levelReference to that issuer's interpreter or lifetime.
    let mut first = actual("tcl8.5");
    push_frame(&mut first);
    let original = Owned::fresh(obj::new_string_bytes(b"#1"));
    unsafe { crate::capi::bind_scalar_getter_context(&first, original.as_ptr()) }.unwrap();
    let protocol = first
        .eval_frame_dialect()
        .native_frame_level_protocol()
        .unwrap();
    let mut operand = OriginalLevel::selected(&first, original.as_ptr()).unwrap();
    assert_eq!(
        protocol
            .resolve_object(1, &mut operand)
            .unwrap()
            .unwrap()
            .target,
        1
    );
    drop(operand);
    pop_frame(&mut first);
    drop(first);
    assert!(unsafe {
        crate::capi::probe_scalar_getter(
            None,
            original.as_ptr(),
            tcl_syntax::scalar_getter::NativeScalarGetterKind::Int,
        )
    }
    .is_err());
    let mut second = actual("tcl8.5");
    push_frame(&mut second);
    let mut operand = OriginalLevel::selected(&second, original.as_ptr()).unwrap();
    assert_eq!(
        protocol
            .resolve_object(1, &mut operand)
            .unwrap()
            .unwrap()
            .target,
        1
    );
    assert!(second.native_execution_refusal().is_none());
    drop(operand);
    pop_frame(&mut second);
}

#[test]
fn original_frame_level_missing_environment_is_terminal_only_at_reached_numeric_stage() {

    // Host refusal is independent of a native bad-level outcome or portable level cache.
    let mut interp = actual("tcl8.4");
    let host = FrameHost::selected(&interp, false);
    interp.set_host(host);
    interp.set_result_bytes(b"PRIOR");
    let prior = interp.get_obj_result();
    let original = Owned::fresh(obj::new_string_bytes(b"#0"));
    let protocol = interp
        .eval_frame_dialect()
        .native_frame_level_protocol()
        .unwrap();
    let mut operand = OriginalLevel::selected(&interp, original.as_ptr()).unwrap();
    let failure = protocol
        .resolve_object(0, &mut operand)
        .err()
        .unwrap_or_else(|| panic!("temporary Int requires actual ENV"));
    assert!(failure.native_execution_refusal().is_some());
    assert!(obj::native_scalar_cache(original.as_ptr())
        .unwrap()
        .is_none());
    assert_eq!(interp.get_obj_result(), prior);
    drop(operand);

    // C8.5's already reached original frame cache bypasses the temporary stage.
    let interp = actual("tcl8.5");
    interp.set_host(FrameHost::selected(&interp, false));
    let original = Owned::fresh(obj::new_string_bytes(b"#0"));
    let dialect = interp.eval_frame_dialect();
    obj::install_native_frame_level_cache(
        original.as_ptr(),
        tcl_registry::NativeFrameLevelCache::Absolute(0),
        dialect,
    )
    .unwrap();
    let mut operand = OriginalLevel::selected(&interp, original.as_ptr()).unwrap();
    let result = dialect
        .native_frame_level_protocol()
        .unwrap()
        .resolve_object(0, &mut operand)
        .unwrap()
        .unwrap();
    assert_eq!(result.target, 0);
    assert!(interp.native_execution_refusal().is_none());
    drop(operand);

    // Jim's #absolute suffix stage is independently selected and errno-free.
    let interp = actual("jimtcl");
    interp.set_host(FrameHost::selected(&interp, false));
    let original = Owned::fresh(obj::new_string_bytes(b"#0"));
    let protocol = interp
        .eval_frame_dialect()
        .native_frame_level_protocol()
        .unwrap();
    let mut operand = OriginalLevel::selected(&interp, original.as_ptr()).unwrap();
    assert_eq!(
        protocol
            .resolve_object(0, &mut operand)
            .unwrap()
            .unwrap()
            .target,
        0
    );
    assert!(interp.native_execution_refusal().is_none());
}

#[test]
fn original_frame_level_callbacks_cannot_restore_currency_before_script_or_cache() {

    // Reached extension effects remain; these are software currency controls, not native rows.
    for change in [false, true] {
        let mut interp = actual("tcl8.4");
        push_frame(&mut interp);
        interp.set_result_bytes(b"PRIOR");
        let prior = interp.get_obj_result();
        let original = Owned::fresh(obj::alloc_typed(&FRAME_UPDATER, 0));
        let command = Owned::fresh(obj::new_string_bytes(b"uplevel"));
        let script = Owned::fresh(obj::new_string_bytes(b"set frame_touched YES"));
        FRAME_UPDATER_CALLS.with(|calls| calls.set(0));
        FRAME_UPDATER_CHANGE.with(|selected| selected.set(change));
        let _current = CurrentFrameInterp::enter(&mut interp);
        assert_eq!(
            super::uplevel_cmd(
                &mut interp,
                &[command.as_ptr(), original.as_ptr(), script.as_ptr()]
            ),
            crate::interp::Code::Error
        );
        let first = interp
            .native_execution_refusal()
            .expect("original updater first cause");
        assert_eq!(FRAME_UPDATER_CALLS.with(Cell::get), 1);
        assert_eq!(obj::bytes_of(original.as_ptr()), b"1");
        assert!(obj::native_scalar_cache(original.as_ptr())
            .unwrap()
            .is_none());
        assert!(interp.var_get(b"frame_touched").is_none());
        assert_eq!(interp.get_obj_result(), prior);
        assert_eq!(interp.entered_native_operation_count(), 0);
        assert_eq!(interp.native_execution_refusal(), Some(first));
        pop_frame(&mut interp);
    }
}

#[test]
fn original_frame_level_numeric_callbacks_stop_after_reached_environment_effect() {

    // Actual Host stages may change and restore policy; no later stage inherits new currency.
    for stage in [1, 2] {
        let mut interp = actual("tcl8.4");
        let host = FrameHost::selected(&interp, true);
        interp.set_host(host.clone());
        host.change_at.set(stage);
        let original = Owned::fresh(obj::new_string_bytes(b"#0"));
        let _current = CurrentFrameInterp::enter(&mut interp);
        let protocol = interp
            .eval_frame_dialect()
            .native_frame_level_protocol()
            .unwrap();
        let mut operand = OriginalLevel::selected(&interp, original.as_ptr()).unwrap();
        assert!(protocol.resolve_object(0, &mut operand).is_err());
        assert!(interp.native_execution_refusal().is_some());
        assert!(host.abi_calls.get() > 0);
        if stage == 1 {
            assert_eq!(host.reset_calls.get(), 0);
            assert_eq!(host.unsigned_calls.get(), 0);
        } else {
            assert_eq!(host.reset_calls.get(), 1);
            assert_eq!(host.unsigned_calls.get(), 1);
        }
        assert!(obj::native_scalar_cache(original.as_ptr())
            .unwrap()
            .is_none());
        assert!(
            obj::native_frame_level_cache_in(original.as_ptr(), interp.eval_frame_dialect())
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn original_frame_level_lease_refuses_retired_header_and_replaced_current_activation() {

    let interp = actual("tcl8.6");
    let original = Owned::fresh(obj::new_string_bytes(b"1"));
    let pointer = original.as_ptr();
    let mut operand = OriginalLevel::selected(&interp, pointer).unwrap();
    assert_eq!(unsafe { (*pointer).ref_count }, 1);
    drop(original);
    assert!(!obj::allocation_is_live(pointer));
    assert!(operand.native_frame_is_integer().is_err());
    assert!(operand
        .native_frame_probe(tcl_syntax::scalar_getter::NativeScalarGetterKind::Int)
        .is_err());
    drop(operand);
    assert_eq!(interp.entered_native_operation_count(), 0);

    let mut interp = actual("tcl8.5");
    push_frame(&mut interp);
    let original = Owned::fresh(obj::new_string_bytes(b"#1"));
    let mut operand = OriginalLevel::selected(&interp, original.as_ptr()).unwrap();
    let before = interp.frames.borrow().current_activation();
    pop_frame(&mut interp);
    push_frame(&mut interp);
    assert_eq!(interp.current_level(), 1);
    assert_ne!(interp.frames.borrow().current_activation(), before);
    assert!(operand.native_frame_string().is_err());
    assert!(operand
        .native_frame_set_cache(tcl_registry::NativeFrameLevelCache::Absolute(1))
        .is_err());
    assert!(
        obj::native_frame_level_cache_in(original.as_ptr(), interp.eval_frame_dialect())
            .unwrap()
            .is_none()
    );
    drop(operand);
    pop_frame(&mut interp);
}

#[test]
fn compiled_frame_and_single_script_list_use_same_original_operation() {

    // Single-List and compiled C9.1 routes retain the same original allocation and purpose.
    let mut interp = actual("tcl9.1");
    interp.set_host(FrameHost::selected(&interp, false));
    push_frame(&mut interp);
    let first = Owned::fresh(obj::new_string_bytes(b"one"));
    let second = Owned::fresh(obj::new_string_bytes(b"two"));
    let script = Owned::fresh(crate::list::new_list_obj(&[
        first.as_ptr(),
        second.as_ptr(),
    ]));
    let before = unsafe { (*script.as_ptr()).ref_count };
    assert!(!obj::has_string_rep(script.as_ptr()));
    assert_eq!(
        super::select_frame(
            &mut interp,
            &[script.as_ptr()],
            tcl_registry::FrameEffectSpec::UPLEVEL
        ),
        Ok((0, 0))
    );
    assert!(!obj::has_string_rep(script.as_ptr()));
    assert_eq!(unsafe { (*script.as_ptr()).ref_count }, before);
    assert!(interp.native_execution_refusal().is_none());
    pop_frame(&mut interp);

    let mut interp = actual("tcl9.1");
    push_frame(&mut interp);
    let original = Owned::fresh(obj::alloc_typed(&FRAME_UPDATER, 0));
    FRAME_UPDATER_CALLS.with(|calls| calls.set(0));
    FRAME_UPDATER_CHANGE.with(|selected| selected.set(true));
    let _current = CurrentFrameInterp::enter(&mut interp);
    assert!(super::select_compiled_uplevel_frame(&mut interp, original.as_ptr()).is_err());
    assert_eq!(FRAME_UPDATER_CALLS.with(Cell::get), 1);
    assert_eq!(obj::bytes_of(original.as_ptr()), b"1");
    assert!(obj::native_scalar_cache(original.as_ptr())
        .unwrap()
        .is_none());
    assert!(interp.native_execution_refusal().is_some());
    assert_eq!(interp.entered_native_operation_count(), 0);
    pop_frame(&mut interp);
}
