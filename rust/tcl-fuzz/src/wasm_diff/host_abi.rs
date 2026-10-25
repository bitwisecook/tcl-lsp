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

//! The real eval-host ABI: owned frames/completions and a separate first refusal.
use std::collections::BTreeMap;

use tcl_runtime_api::Completion;
use tcl_runtime_api::codegen_abi::{
    CodegenAbiImportId, WASM32_COMPLETION_ALIGN, WASM32_COMPLETION_SIZE,
};
use tcl_vm::{Code, TclError, Value};
use wasmtime::{Caller, Linker};

use super::HostState;

// The runtime's documented TCL_INVOKE_ABI_HOST_REFUSED transport status.
// It is never a Tcl completion code and never clears the retained cause.
pub(super) const HOST_REFUSED: i32 = -6;
const WASM_PAGE_BYTES: usize = 65_536;

#[derive(Debug)]
enum Refusal {
    Evaluation(TclError),
    Transport(String),
}

enum OwnedObject {
    Source(Vec<u8>),
    Value(Value),
}

pub(super) struct AbiState {
    frames: BTreeMap<i32, usize>,
    next_frame: Option<usize>,
    heap_base: Option<usize>,
    objects: BTreeMap<i32, OwnedObject>,
    next_object: i32,
    completions: BTreeMap<i32, Completion<i32>>,
    refusal: Option<Refusal>,
}

impl Default for AbiState {
    fn default() -> Self {
        Self {
            frames: BTreeMap::new(),
            next_frame: None,
            heap_base: None,
            objects: BTreeMap::new(),
            next_object: -2,
            completions: BTreeMap::new(),
            refusal: None,
        }
    }
}

impl AbiState {
    pub(super) fn has_refusal(&self) -> bool {
        self.refusal.is_some()
    }

    pub(super) fn retain_host_error(&mut self, error: TclError) {
        self.refusal.get_or_insert(Refusal::Evaluation(error));
    }

    pub(super) fn refusal_message(&self) -> Option<String> {
        self.refusal.as_ref().map(|error| match error {
            Refusal::Evaluation(error) => format!("{error:?}"),
            Refusal::Transport(error) => error.clone(),
        })
    }

    fn transport(&mut self, error: impl Into<String>) {
        self.refusal.get_or_insert(Refusal::Transport(error.into()));
    }

    fn own(&mut self, value: OwnedObject) -> Option<i32> {
        let handle = self.next_object;
        self.next_object = handle.checked_sub(1)?;
        self.objects.insert(handle, value);
        Some(handle)
    }

    fn release(&mut self, handle: i32) -> bool {
        if handle == 0 {
            return true;
        }
        match self.objects.remove(&handle) {
            Some(OwnedObject::Source(bytes)) => drop(bytes),
            Some(OwnedObject::Value(value)) => drop(value),
            None => return false,
        }
        // Individual result/options release is the ABI alternative to completion_release.
        let objects = &self.objects;
        self.completions.retain(|_, record| {
            objects.contains_key(&record.result) || objects.contains_key(&record.options)
        });
        true
    }

    fn frame_range(&self, pointer: i32, bytes: usize) -> Option<std::ops::Range<usize>> {
        let (base, size) = self.frames.range(..=pointer).next_back()?;
        let start = usize::try_from(pointer).ok()?;
        let end = start.checked_add(bytes)?;
        (end <= usize::try_from(*base).ok()?.checked_add(*size)?).then_some(start..end)
    }
}

pub(super) fn register(linker: &mut Linker<HostState>) -> Result<(), String> {
    // The registry owns every module/name/signature; no printed import guesses.
    macro_rules! register {
        ($id:ident, $function:expr) => {{
            let descriptor = CodegenAbiImportId::$id.descriptor();
            linker
                .func_wrap(descriptor.module, descriptor.name, $function)
                .map_err(|error| format!("link {:?}: {error}", CodegenAbiImportId::$id))?;
        }};
    }
    register!(HostRefusalPending, |caller: Caller<'_, HostState>| {
        refusal_pending(&caller)
    });
    register!(ObjectNewString, super::host_obj_new_string);
    register!(EvalCode, super::host_eval_code);
    register!(CallFrameAlloc, frame_alloc);
    register!(CallFrameFree, frame_free);
    register!(NewOwnedString, new_owned_string);
    register!(ExprBoolEval, expr_bool);
    register!(CompletionRelease, completion_release);
    register!(ObjectRelease, object_release);
    Ok(())
}

fn refusal_pending(caller: &Caller<'_, HostState>) -> i32 {
    i32::from(caller.data().abi.refusal.is_some())
}

fn refuse(caller: &mut Caller<'_, HostState>, message: &str) -> i32 {
    caller.data_mut().abi.transport(message);
    HOST_REFUSED
}

fn frame_alloc(mut caller: Caller<'_, HostState>, bytes: i32, alignment: i32) -> i32 {
    if caller.data().abi.refusal.is_some() {
        return 0;
    }
    let Some(memory) = caller.data().memory else {
        refuse(&mut caller, "frame allocation has no memory");
        return 0;
    };
    let layout = usize::try_from(bytes)
        .ok()
        .zip(usize::try_from(alignment).ok());
    let Some((bytes, alignment)) = layout.filter(|(n, a)| *n != 0 && a.is_power_of_two()) else {
        refuse(&mut caller, "invalid call-frame layout");
        return 0;
    };
    let bump = caller
        .data()
        .abi
        .next_frame
        .unwrap_or_else(|| memory.data_size(&caller));
    let allocation = bump
        .checked_add(alignment - 1)
        .map(|n| n & !(alignment - 1))
        .and_then(|start| Some((i32::try_from(start).ok()?, start.checked_add(bytes)?)));
    let Some((pointer, end)) = allocation.filter(|(_, end)| i32::try_from(*end).is_ok()) else {
        refuse(&mut caller, "call-frame address overflow");
        return 0;
    };
    let extra = end
        .div_ceil(WASM_PAGE_BYTES)
        .saturating_sub(usize::try_from(memory.size(&caller)).unwrap_or(usize::MAX));
    if extra != 0
        && memory
            .grow(
                &mut caller,
                u64::try_from(extra).expect("page count fits u64"),
            )
            .is_err()
    {
        refuse(&mut caller, "call-frame memory growth failed");
        return 0;
    }
    let state = &mut caller.data_mut().abi;
    state.heap_base.get_or_insert(bump);
    state.frames.insert(pointer, bytes);
    state.next_frame = Some(end);
    pointer
}

fn frame_free(mut caller: Caller<'_, HostState>, pointer: i32) -> i32 {
    let Some(range) = caller
        .data()
        .abi
        .frames
        .get(&pointer)
        .and_then(|size| caller.data().abi.frame_range(pointer, *size))
    else {
        return refuse(&mut caller, "free of unknown call frame");
    };
    if caller
        .data()
        .abi
        .completions
        .keys()
        .any(|p| usize::try_from(*p).is_ok_and(|p| range.contains(&p)))
    {
        return refuse(&mut caller, "call frame still owns a completion");
    }
    let state = &mut caller.data_mut().abi;
    state.frames.remove(&pointer);
    if state.frames.is_empty() {
        state.next_frame = state.heap_base;
    }
    0
}

fn new_owned_string(mut caller: Caller<'_, HostState>, pointer: i32, length: i32) -> i32 {
    if caller.data().abi.refusal.is_some() {
        return 0;
    }
    let bytes = caller.data().memory.and_then(|memory| {
        let start = usize::try_from(pointer).ok()?;
        let length = usize::try_from(length).ok()?;
        Some(
            memory
                .data(&caller)
                .get(start..start.checked_add(length)?)?
                .to_vec(),
        )
    });
    let Some(bytes) = bytes else {
        refuse(&mut caller, "owned string range is outside memory");
        return 0;
    };
    if let Some(handle) = caller.data_mut().abi.own(OwnedObject::Source(bytes)) {
        handle
    } else {
        refuse(&mut caller, "owned object handle overflow");
        0
    }
}

fn object_release(mut caller: Caller<'_, HostState>, handle: i32) {
    // Cleanup is allowed under refusal and never consumes/replaces its cause.
    if !caller.data_mut().abi.release(handle) {
        refuse(&mut caller, "release of unknown owned object");
    }
}

fn output_ranges(
    caller: &Caller<'_, HostState>,
    completion: i32,
    truth: i32,
) -> Option<(std::ops::Range<usize>, std::ops::Range<usize>)> {
    if completion.rem_euclid(WASM32_COMPLETION_ALIGN) != 0
        || truth.rem_euclid(4) != 0
        || caller.data().abi.completions.contains_key(&completion)
    {
        return None;
    }
    let a = caller
        .data()
        .abi
        .frame_range(completion, usize::try_from(WASM32_COMPLETION_SIZE).ok()?)?;
    let b = caller.data().abi.frame_range(truth, 4)?;
    ((a.end <= b.start || b.end <= a.start)
        && caller.data().memory?.data(caller).get(a.clone()).is_some()
        && caller.data().memory?.data(caller).get(b.clone()).is_some())
    .then_some((a, b))
}

fn expr_bool(
    mut caller: Caller<'_, HostState>,
    expression: i32,
    completion: i32,
    truth: i32,
) -> i32 {
    if caller.data().abi.refusal.is_some() {
        return HOST_REFUSED;
    }
    let Some((completion_range, truth_range)) = output_ranges(&caller, completion, truth) else {
        return refuse(&mut caller, "invalid Boolean-expression output storage");
    };
    let Some(OwnedObject::Source(source)) = caller.data().abi.objects.get(&expression) else {
        return refuse(&mut caller, "Boolean expression has no owned source");
    };
    let source = source.clone();
    // The VM owner retains one actual scope through producer and Boolean getter.
    let evaluated = caller.data_mut().vm.try_eval_expr_bool_bytes(&source);
    let (guest, boolean) = match evaluated {
        Ok(result) => result,
        Err(error) => {
            caller
                .data_mut()
                .abi
                .retain_host_error(TclError::from_execution_failure(error));
            return HOST_REFUSED;
        }
    };
    if (guest.code == Code::Ok) != boolean.is_some() {
        return refuse(&mut caller, "VM Boolean completion and truth disagree");
    }
    let Ok(code) = i32::try_from(guest.code.as_int()) else {
        return refuse(&mut caller, "guest completion code exceeds wasm32 ABI");
    };
    let Some(result) = caller.data_mut().abi.own(OwnedObject::Value(guest.result)) else {
        return refuse(&mut caller, "completion result handle overflow");
    };
    let Some(options) = caller.data_mut().abi.own(OwnedObject::Value(guest.options)) else {
        caller.data_mut().abi.release(result);
        return refuse(&mut caller, "completion options handle overflow");
    };
    let words = [code, result, options];
    // Keep ALL four original completion fields; the wasm32 record carries three
    // words while option_origin remains on its exact private transport owner.
    let record = Completion {
        code: guest.code,
        result,
        options,
        option_origin: guest.option_origin,
    };
    caller.data_mut().abi.completions.insert(completion, record);
    caller.data_mut().errored |=
        code == i32::try_from(Code::Error.as_int()).expect("Tcl error code");
    let memory = caller.data().memory.expect("validated output memory");
    let data = memory.data_mut(&mut caller);
    for (slot, value) in data[completion_range]
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(words)
    {
        slot.copy_from_slice(&value.to_le_bytes());
    }
    if let Some(boolean) = boolean {
        data[truth_range].copy_from_slice(&i32::from(boolean).to_le_bytes());
    }
    0
}

fn completion_release(mut caller: Caller<'_, HostState>, pointer: i32) {
    let Some(range) = caller.data().abi.frame_range(
        pointer,
        usize::try_from(WASM32_COMPLETION_SIZE).expect("completion size"),
    ) else {
        refuse(&mut caller, "completion release outside a live frame");
        return;
    };
    let Some(memory) = caller.data().memory else {
        refuse(&mut caller, "completion release has no memory");
        return;
    };
    let data = memory.data(&caller);
    let Some(bytes) = data.get(range.clone()) else {
        refuse(&mut caller, "completion release outside memory");
        return;
    };
    let actual: Vec<i32> = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| i32::from_le_bytes(*b))
        .collect();
    let expected = caller.data().abi.completions.get(&pointer).map(|record| {
        [
            i32::try_from(record.code.as_int()).expect("validated completion code"),
            record.result,
            record.options,
        ]
    });
    if expected.as_ref().map_or_else(
        || actual.iter().any(|v| *v != 0),
        |record| actual.as_slice() != record,
    ) {
        refuse(
            &mut caller,
            "completion release does not match original record",
        );
        return;
    }
    if let Some(record) = caller.data_mut().abi.completions.remove(&pointer) {
        for handle in [record.result, record.options] {
            if !caller.data_mut().abi.release(handle) {
                refuse(&mut caller, "completion owned object is unavailable");
            }
        }
    }
    memory.data_mut(&mut caller)[range].fill(0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasmtime::{Memory, MemoryType, Store, TypedFunc, WasmParams, WasmResults};

    fn host() -> (Store<HostState>, Linker<HostState>) {
        let (vm, _) = super::super::fresh_vm();
        host_for_vm(vm)
    }

    fn host_for_vm(vm: tcl_vm::Vm) -> (Store<HostState>, Linker<HostState>) {
        let engine = super::super::engine();
        let mut store = Store::new(
            &engine,
            HostState {
                abi: AbiState::default(),
                vm,
                handles: Vec::new(),
                memory: None,
                errored: false,
            },
        );
        let memory = Memory::new(&mut store, MemoryType::new(1, None)).unwrap();
        store.data_mut().memory = Some(memory);
        let mut linker = Linker::new(&engine);
        register(&mut linker).unwrap();
        (store, linker)
    }

    fn function<P: WasmParams, R: WasmResults>(
        store: &mut Store<HostState>,
        linker: &Linker<HostState>,
        id: CodegenAbiImportId,
    ) -> TypedFunc<P, R> {
        let descriptor = id.descriptor();
        linker
            .get(&mut *store, descriptor.module, descriptor.name)
            .unwrap()
            .into_func()
            .unwrap()
            .typed::<P, R>(&*store)
            .unwrap()
    }

    #[test]
    fn every_selected_import_has_the_actual_registry_signature() {
        use tcl_runtime_api::codegen_abi::CodegenAbiValueType;
        let (mut store, linker) = host();
        for id in [
            CodegenAbiImportId::HostRefusalPending,
            CodegenAbiImportId::ObjectNewString,
            CodegenAbiImportId::EvalCode,
            CodegenAbiImportId::CallFrameAlloc,
            CodegenAbiImportId::CallFrameFree,
            CodegenAbiImportId::NewOwnedString,
            CodegenAbiImportId::ExprBoolEval,
            CodegenAbiImportId::CompletionRelease,
            CodegenAbiImportId::ObjectRelease,
        ] {
            let descriptor = id.descriptor();
            let func = linker
                .get(&mut store, descriptor.module, descriptor.name)
                .unwrap()
                .into_func()
                .unwrap();
            let ty = func.ty(&store);
            assert_eq!(ty.params().len(), descriptor.parameters.len(), "{id:?}");
            assert_eq!(ty.results().len(), descriptor.results.len(), "{id:?}");
            for (actual, expected) in ty
                .params()
                .zip(descriptor.parameters)
                .chain(ty.results().zip(descriptor.results))
            {
                assert!(
                    matches!(
                        (actual, expected),
                        (wasmtime::ValType::I32, CodegenAbiValueType::I32)
                    ),
                    "{id:?}"
                );
            }
        }
    }

    #[test]
    fn live_frames_are_disjoint_and_freed_storage_is_reused() {
        let (mut store, linker) = host();
        let alloc =
            function::<(i32, i32), i32>(&mut store, &linker, CodegenAbiImportId::CallFrameAlloc);
        let free = function::<i32, i32>(&mut store, &linker, CodegenAbiImportId::CallFrameFree);
        let a = alloc.call(&mut store, (32, 8)).unwrap();
        let b = alloc.call(&mut store, (64, 8)).unwrap();
        assert_ne!(a, 0);
        assert!(b >= a + 32);
        assert_eq!(a.rem_euclid(8), 0);
        assert_eq!(free.call(&mut store, a).unwrap(), 0);
        assert_eq!(free.call(&mut store, b).unwrap(), 0);
        let reused = alloc.call(&mut store, (32, 8)).unwrap();
        assert_eq!(reused, a);
        assert_eq!(free.call(&mut store, reused).unwrap(), 0);
        assert_eq!(store.data().abi.frames.len(), 0);
        assert!(store.data().abi.refusal_message().is_none());
    }

    #[test]
    fn cleanup_keeps_the_first_actual_transport_refusal_pending() {
        let (mut store, linker) = host();
        assert_eq!(
            store
                .data_mut()
                .vm
                .try_eval_source("set pending_after 0")
                .unwrap()
                .code,
            Code::Ok
        );
        let bytes = b"set pending_after 1";
        let memory = store.data().memory.unwrap();
        memory.write(&mut store, 0, bytes).unwrap();
        let constructor =
            function::<(i32, i32), i32>(&mut store, &linker, CodegenAbiImportId::ObjectNewString);
        let eval = function::<i32, i32>(&mut store, &linker, CodegenAbiImportId::EvalCode);
        let script = constructor
            .call(&mut store, (0, i32::try_from(bytes.len()).unwrap()))
            .unwrap();
        let alloc =
            function::<(i32, i32), i32>(&mut store, &linker, CodegenAbiImportId::CallFrameAlloc);
        let free = function::<i32, i32>(&mut store, &linker, CodegenAbiImportId::CallFrameFree);
        let pending =
            function::<(), i32>(&mut store, &linker, CodegenAbiImportId::HostRefusalPending);
        let a = alloc.call(&mut store, (32, 4)).unwrap();
        let b = alloc.call(&mut store, (32, 4)).unwrap();
        assert_eq!(free.call(&mut store, a).unwrap(), 0);
        assert_eq!(free.call(&mut store, a).unwrap(), HOST_REFUSED);
        let first = store.data().abi.refusal_message();
        assert!(first.is_some());
        assert_eq!(pending.call(&mut store, ()).unwrap(), 1);
        assert_eq!(eval.call(&mut store, script).unwrap(), HOST_REFUSED);
        assert_eq!(
            store
                .data_mut()
                .vm
                .try_eval_expr_bool_bytes(b"$pending_after == 0")
                .unwrap()
                .1,
            Some(true)
        );
        assert_eq!(store.data().abi.refusal_message(), first);
        let handles = store.data().handles.len();
        assert_eq!(
            constructor
                .call(&mut store, (0, i32::try_from(bytes.len()).unwrap()))
                .unwrap(),
            0
        );
        assert_eq!(store.data().handles.len(), handles);
        assert_eq!(free.call(&mut store, b).unwrap(), 0);
        assert_eq!(free.call(&mut store, b).unwrap(), HOST_REFUSED);
        assert_eq!(store.data().abi.refusal_message(), first);
        assert_eq!(pending.call(&mut store, ()).unwrap(), 1);
        assert_eq!(store.data().abi.frames.len(), 0);
        assert!(
            !store.data().errored,
            "host refusal is not a guest Tcl error"
        );
    }

    #[test]
    fn condition_keeps_full_owned_guest_completion_and_truth_publication_order() {
        let (mut store, linker) = host();
        let alloc =
            function::<(i32, i32), i32>(&mut store, &linker, CodegenAbiImportId::CallFrameAlloc);
        let free = function::<i32, i32>(&mut store, &linker, CodegenAbiImportId::CallFrameFree);
        let source =
            function::<(i32, i32), i32>(&mut store, &linker, CodegenAbiImportId::NewOwnedString);
        let evaluate =
            function::<(i32, i32, i32), i32>(&mut store, &linker, CodegenAbiImportId::ExprBoolEval);
        let release =
            function::<i32, ()>(&mut store, &linker, CodegenAbiImportId::CompletionRelease);
        let object = function::<i32, ()>(&mut store, &linker, CodegenAbiImportId::ObjectRelease);
        for (bytes, expected_code, expected_truth) in
            [(b"1".as_slice(), 0, 1), (b"1 +".as_slice(), 1, 77)]
        {
            let memory = store.data().memory.unwrap();
            memory.write(&mut store, 0, bytes).unwrap();
            let owned = source
                .call(&mut store, (0, i32::try_from(bytes.len()).unwrap()))
                .unwrap();
            let frame = alloc.call(&mut store, (20, 4)).unwrap();
            let completion = frame + 4;
            let truth = frame + 16;
            memory
                .write(&mut store, usize::try_from(completion).unwrap(), &[0; 12])
                .unwrap();
            memory
                .write(
                    &mut store,
                    usize::try_from(truth).unwrap(),
                    &77_i32.to_le_bytes(),
                )
                .unwrap();
            assert_eq!(
                evaluate
                    .call(&mut store, (owned, completion, truth))
                    .unwrap(),
                0
            );
            let data = memory.data(&store);
            let read = |pointer: i32| {
                i32::from_le_bytes(
                    data[usize::try_from(pointer).unwrap()..usize::try_from(pointer).unwrap() + 4]
                        .try_into()
                        .unwrap(),
                )
            };
            assert_eq!(read(completion), expected_code);
            assert_eq!(read(truth), expected_truth);
            for handle in [read(completion + 4), read(completion + 8)] {
                assert!(matches!(
                    store.data().abi.objects.get(&handle),
                    Some(OwnedObject::Value(_))
                ));
            }
            assert_eq!(store.data().abi.objects.len(), 3);
            release.call(&mut store, completion).unwrap();
            object.call(&mut store, owned).unwrap();
            assert_eq!(free.call(&mut store, frame).unwrap(), 0);
            assert_eq!(store.data().abi.objects.len(), 0);
            assert_eq!(store.data().abi.completions.len(), 0);
        }
        assert!(store.data().abi.refusal_message().is_none());
    }

    #[test]
    fn actual_missing_compiler_refusal_leaves_outputs_untouched_through_cleanup() {
        let profile = tcl_registry::model::resolve_environment("tcl9.0").unit_profile();
        let vm = tcl_vm::Vm::with_native_core(
            Box::new(std::io::sink()),
            tcl_vm::standard_host(),
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        // This actual native interpreter deliberately has no CompileService.
        let (mut store, linker) = host_for_vm(vm);
        let alloc =
            function::<(i32, i32), i32>(&mut store, &linker, CodegenAbiImportId::CallFrameAlloc);
        let free = function::<i32, i32>(&mut store, &linker, CodegenAbiImportId::CallFrameFree);
        let source =
            function::<(i32, i32), i32>(&mut store, &linker, CodegenAbiImportId::NewOwnedString);
        let evaluate =
            function::<(i32, i32, i32), i32>(&mut store, &linker, CodegenAbiImportId::ExprBoolEval);
        let release =
            function::<i32, ()>(&mut store, &linker, CodegenAbiImportId::CompletionRelease);
        let object = function::<i32, ()>(&mut store, &linker, CodegenAbiImportId::ObjectRelease);
        let pending =
            function::<(), i32>(&mut store, &linker, CodegenAbiImportId::HostRefusalPending);
        let bytes = b"[set condition_probe 1]";
        let memory = store.data().memory.unwrap();
        memory.write(&mut store, 0, bytes).unwrap();
        let owned = source
            .call(&mut store, (0, i32::try_from(bytes.len()).unwrap()))
            .unwrap();
        let frame = alloc.call(&mut store, (20, 4)).unwrap();
        let completion = frame + 4;
        let truth = frame + 16;
        memory
            .write(
                &mut store,
                usize::try_from(truth).unwrap(),
                &77_i32.to_le_bytes(),
            )
            .unwrap();
        assert_eq!(
            evaluate
                .call(&mut store, (owned, completion, truth))
                .unwrap(),
            HOST_REFUSED
        );
        assert!(matches!(
            store.data().abi.refusal,
            Some(Refusal::Evaluation(TclError::Host(_)))
        ));
        let original = store.data().abi.refusal_message();
        assert_eq!(pending.call(&mut store, ()).unwrap(), 1);
        assert_eq!(
            &memory.data(&store)
                [usize::try_from(completion).unwrap()..usize::try_from(completion).unwrap() + 12],
            &[0; 12]
        );
        assert_eq!(
            &memory.data(&store)
                [usize::try_from(truth).unwrap()..usize::try_from(truth).unwrap() + 4],
            &77_i32.to_le_bytes()
        );
        release.call(&mut store, completion).unwrap();
        object.call(&mut store, owned).unwrap();
        assert_eq!(free.call(&mut store, frame).unwrap(), 0);
        assert_eq!(store.data().abi.refusal_message(), original);
        assert_eq!(pending.call(&mut store, ()).unwrap(), 1);
        assert_eq!(store.data().abi.objects.len(), 0);
        assert_eq!(store.data().abi.frames.len(), 0);
        assert!(
            !store.data().errored,
            "actual host refusal never becomes catch-visible Tcl error"
        );
    }
}
