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

//! One instance of the runtime: its store, its interpreter, the extensions
//! linked into it, and the calls an engine makes on it.
//!
//! An extension is a side module (the WebAssembly tool conventions' dynamic
//! linking). It imports the runtime's memory and function table, three globals
//! saying where its data, its functions and its stack are, and the C API it
//! calls, which are the runtime's own exports; it may import no other export of
//! the runtime. Loading one reserves its data in the runtime's heap, grows the
//! table for its functions, gives it a stack of its own, applies its
//! relocations, and runs its entry point with the interpreter, as `load` does.

use std::any::Any;
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::command_receipts::{self, CommandReceipt};
use tcl_engine_api::{BudgetKind, EngineError, HostCommand, Value};
use wasmtime::{
    Extern, Global, GlobalType, Instance, Linker, Memory, Mutability, Ref, ResourceLimiter, Store,
    Table, Trap, TypedFunc, Val, ValType, WasmParams, WasmResults,
};

use crate::dylink::Layout;
use crate::wasi::{self, ProcExit, SharedMemory};
use crate::{Extension, WasmRuntime};

/// The shadow stack each side module runs on, in bytes.
const SIDE_STACK: i32 = 64 * 1024;

/// What a session's store holds.
#[derive(Default)]
pub(crate) struct HostState {
    /// The runtime's exports, once it is instantiated.
    pub(crate) exports: Option<Rc<Exports>>,
    /// The cap on the memory's growth for the running evaluation.
    cap: MemoryCap,
    /// The host commands, by the client data each was registered with.
    pub(crate) commands: Vec<Rc<dyn HostCommand>>,
    /// A panic a host command raised, held until the evaluation unwinds.
    pub(crate) panic: Option<Box<dyn Any + Send>>,
    /// The table slot of the procedure every host command is registered with,
    /// once one is.
    pub(crate) host_procedure: Option<i32>,
    /// What running host commands set up through their doors, for the engine
    /// to keep.
    pub(crate) defined: Vec<crate::host_call::Defined>,
    pub(crate) host_receipts: Vec<CommandReceipt>,
    unit_receipts: BTreeMap<u32, CommandReceipt>,
}

impl SharedMemory for HostState {
    fn memory(&self) -> Option<Memory> {
        self.exports.as_ref().map(|exports| exports.memory)
    }
}

/// The memory an evaluation may grow to.
#[derive(Debug, Default)]
struct MemoryCap {
    limit: Option<usize>,
}

/// The memory cap stopped an evaluation's growth.
#[derive(Debug)]
pub(crate) struct ValueSizeExceeded;

impl std::fmt::Display for ValueSizeExceeded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the evaluation grew the memory past its value-size budget")
    }
}

impl std::error::Error for ValueSizeExceeded {}

impl ResourceLimiter for MemoryCap {
    fn memory_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        match self.limit {
            Some(limit) if desired > limit => Err(wasmtime::Error::new(ValueSizeExceeded)),
            _ => Ok(true),
        }
    }

    fn table_growing(
        &mut self,
        _current: usize,
        _desired: usize,
        _maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        Ok(true)
    }
}

/// The runtime's exports an engine calls.
pub(crate) struct Exports {
    pub(crate) memory: Memory,
    table: Table,
    instance: Instance,
    pub(crate) alloc: TypedFunc<(i32, i32), i32>,
    pub(crate) free: TypedFunc<i32, i32>,
    pub(crate) new_string: TypedFunc<(i32, i32), i32>,
    pub(crate) release: TypedFunc<i32, ()>,
    pub(crate) string_of: TypedFunc<(i32, i32), i32>,
    invoke_argv: TypedFunc<(i32, i32, i32), i32>,
    pub(crate) completion_release: TypedFunc<i32, ()>,
    pub(crate) capture: TypedFunc<(i32, i32, i32), i32>,
    pub(crate) create_command: TypedFunc<(i32, i32, i32, i32, i32, i32), i32>,
    pub(crate) delete_command: TypedFunc<(i32, i32, i32, i32), i32>,
    set_limits: TypedFunc<(i32, i64, i64), ()>,
    begin: TypedFunc<i32, ()>,
    exceeded: TypedFunc<i32, i32>,
    pub(crate) exceed: TypedFunc<(i32, i32), i32>,
    spent: TypedFunc<i32, i64>,
    confine: TypedFunc<i32, ()>,
    restrict: TypedFunc<(i32, i32, i32, i32), i32>,
    set_release: TypedFunc<(i32, i32, i32), i32>,
    pub(crate) define_unit: TypedFunc<(i32, i32, i32, i32, i32), i32>,
    pub(crate) provide: TypedFunc<(i32, i32, i32), i32>,
    pub(crate) error_code: TypedFunc<i32, i32>,
    pub(crate) fail: TypedFunc<(i32, i32, i32), i32>,
    pub(crate) complete: TypedFunc<(i32, i32, i32, i32), i32>,
    pub(crate) host_refusal_pending: TypedFunc<i32, i32>,
    pub(crate) host_refusal_text: TypedFunc<i32, i32>,
    pub(crate) refuse_host: TypedFunc<(i32, i32, i32), i32>,
    pub(crate) string_snapshot: TypedFunc<(i32, i32), i32>,
    pub(crate) receipt_current: TypedFunc<(i32, i64, i64, i64, i32, i32), i32>,
    pub(crate) new_scalar: TypedFunc<(i32, i32, i32), i32>,
    pub(crate) new_byte_array: TypedFunc<(i32, i32, i32), i32>,
    pub(crate) adopt_resident: TypedFunc<(i32, i32, i32, i32, i32), i32>,
    pub(crate) new_sequence: TypedFunc<(i32, i32, i32, i32), i32>,
    pub(crate) new_int: TypedFunc<i64, i32>,
    pub(crate) new_double: TypedFunc<f64, i32>,
    pub(crate) retain: TypedFunc<i32, i32>,
}

impl Exports {
    fn of(store: &mut Store<HostState>, instance: Instance) -> wasmtime::Result<Self> {
        let memory = instance
            .get_memory(&mut *store, "memory")
            .ok_or_else(|| wasmtime::Error::msg("the runtime exports no memory"))?;
        let table = instance
            .get_table(&mut *store, "__indirect_function_table")
            .ok_or_else(|| wasmtime::Error::msg("the runtime exports no function table"))?;
        Ok(Self {
            memory,
            table,
            instance,
            alloc: instance.get_typed_func(&mut *store, "tcl_codegen_call_frame_alloc")?,
            free: instance.get_typed_func(&mut *store, "tcl_codegen_call_frame_free")?,
            new_string: instance.get_typed_func(&mut *store, "tcl_obj_new_string_owned")?,
            release: instance.get_typed_func(&mut *store, "tcl_obj_release")?,
            string_of: instance.get_typed_func(&mut *store, "Tcl_GetStringFromObj")?,
            invoke_argv: instance.get_typed_func(&mut *store, "tcl_invoke_argv")?,
            completion_release: instance.get_typed_func(&mut *store, "tcl_completion_release")?,
            capture: instance.get_typed_func(&mut *store, "tcl_engine_capture_original")?,
            create_command: instance
                .get_typed_func(&mut *store, "tcl_engine_create_command_counted")?,
            delete_command: instance
                .get_typed_func(&mut *store, "tcl_engine_delete_command_counted")?,
            set_limits: instance.get_typed_func(&mut *store, "tcl_engine_set_limits")?,
            begin: instance.get_typed_func(&mut *store, "tcl_engine_begin")?,
            exceeded: instance.get_typed_func(&mut *store, "tcl_engine_exceeded")?,
            exceed: instance.get_typed_func(&mut *store, "tcl_engine_exceed")?,
            spent: instance.get_typed_func(&mut *store, "tcl_engine_commands_spent")?,
            confine: instance.get_typed_func(&mut *store, "tcl_engine_confine_stores")?,
            restrict: instance
                .get_typed_func(&mut *store, "tcl_engine_restrict_original_receipts")?,
            set_release: instance.get_typed_func(&mut *store, "tcl_engine_set_release")?,
            define_unit: instance.get_typed_func(&mut *store, "tcl_engine_define_unit_receipt")?,
            provide: instance.get_typed_func(&mut *store, "tcl_engine_provide_package")?,
            error_code: instance.get_typed_func(&mut *store, "tcl_engine_error_code")?,
            fail: instance.get_typed_func(&mut *store, "tcl_engine_fail")?,
            complete: instance.get_typed_func(&mut *store, "tcl_engine_complete_original")?,
            host_refusal_pending: instance
                .get_typed_func(&mut *store, "tcl_engine_host_refusal_pending")?,
            host_refusal_text: instance
                .get_typed_func(&mut *store, "tcl_engine_host_refusal_text")?,
            refuse_host: instance.get_typed_func(&mut *store, "tcl_engine_refuse_host_counted")?,
            string_snapshot: instance
                .get_typed_func(&mut *store, "tcl_engine_original_string_snapshot")?,
            receipt_current: instance
                .get_typed_func(&mut *store, "tcl_engine_command_receipt_current")?,
            new_scalar: instance.get_typed_func(&mut *store, "tcl_engine_new_scalar_carrier")?,
            new_byte_array: instance
                .get_typed_func(&mut *store, "tcl_engine_new_byte_array_carrier")?,
            adopt_resident: instance
                .get_typed_func(&mut *store, "tcl_engine_adopt_resident_carrier")?,
            new_sequence: instance
                .get_typed_func(&mut *store, "tcl_engine_new_sequence_carrier")?,
            new_int: instance.get_typed_func(&mut *store, "Tcl_NewWideIntObj")?,
            new_double: instance.get_typed_func(&mut *store, "Tcl_NewDoubleObj")?,
            retain: instance.get_typed_func(&mut *store, "tcl_obj_retain")?,
        })
    }
}

/// How an evaluation completed: its code, its result's bytes and its options'.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Completion {
    pub(crate) code: i32,
    pub(crate) result: Vec<u8>,
    pub(crate) options: Vec<u8>,
    pub(crate) error_code: Option<Vec<u8>>,
}

/// The limits one evaluation runs under, as the store and the runtime take
/// them.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Limits {
    /// The fuel: every instruction spends some.
    pub(crate) fuel: Option<u64>,
    /// Epoch ticks before the evaluation is interrupted.
    pub(crate) ticks: Option<u64>,
    /// The bytes the memory may grow by.
    pub(crate) growth: Option<u64>,
    /// The runtime's own command count.
    pub(crate) commands: Option<u64>,
    /// The runtime's own value-size limit.
    pub(crate) value_bytes: Option<u64>,
}

impl Limits {
    /// These limits with the fuel a fresh instance's first evaluation has beside
    /// its commands' ([`crate::FIRST_USE_FUEL`]).
    pub(crate) fn with_first_use(self) -> Self {
        Self {
            fuel: self
                .fuel
                .map(|fuel| fuel.saturating_add(crate::FIRST_USE_FUEL)),
            ..self
        }
    }
}

/// One instance of the runtime and its interpreter.
pub(crate) struct Session {
    pub(crate) store: Store<HostState>,
    exports: Rc<Exports>,
    interp: i32,
    /// Sixteen bytes the session's calls write through: a length cell, then a
    /// completion.
    scratch: i32,
}

/// Call `function`, a trap being a failure.
fn call<P: WasmParams, R: WasmResults>(
    store: &mut Store<HostState>,
    function: &TypedFunc<P, R>,
    params: P,
) -> Result<R, Failure> {
    function.call(store, params).map_err(Failure::Trap)
}

impl Session {
    /// A fresh instance of `runtime` with a fresh interpreter.
    pub(crate) fn new(runtime: &WasmRuntime) -> wasmtime::Result<Self> {
        let mut store = Store::new(runtime.engine(), HostState::default());
        store.limiter(|state| &mut state.cap);
        store.set_fuel(u64::MAX)?;
        store.set_epoch_deadline(u64::MAX / 2);
        let mut linker = Linker::new(runtime.engine());
        wasi::define_stubs(&mut linker, runtime.module())?;
        let instance = linker.instantiate(&mut store, runtime.module())?;
        let exports = Rc::new(Exports::of(&mut store, instance)?);
        store.data_mut().exports = Some(Rc::clone(&exports));
        let create = instance.get_typed_func::<(), i32>(&mut store, "tcl_runtime_create_interp")?;
        let current =
            instance.get_typed_func::<i32, ()>(&mut store, "tcl_runtime_set_current_interp")?;
        let interp = create.call(&mut store, ())?;
        current.call(&mut store, interp)?;
        let scratch = exports.alloc.call(&mut store, (16, 8))?;
        Ok(Self {
            store,
            exports,
            interp,
            scratch,
        })
    }

    /// Link `extension` into the instance and run its entry point.
    ///
    /// # Errors
    ///
    /// An entry point that answers an error is the script error it left, and
    /// an extension that imports what the runtime does not export is refused.
    pub(crate) fn load(&mut self, extension: &Extension) -> Result<(), Failure> {
        let layout = extension.layout();
        let memory_base = self.reserve(layout.memory_size, layout.memory_align)?;
        let table_base = self.grow_table(layout)?;
        let stack = call(&mut self.store, &self.exports.alloc, (SIDE_STACK, 16))?;
        let mut linker = Linker::new(self.store.engine());
        wasi::define_stubs(&mut linker, extension.module()).map_err(Failure::Trap)?;
        for import in extension.module().imports() {
            let (module, name) = (import.module(), import.name());
            if module == wasi::WASI {
                continue;
            }
            let item: Extern = match (module, name) {
                ("env", "memory") => self.exports.memory.into(),
                ("env", "__indirect_function_table") => self.exports.table.into(),
                ("env", "__memory_base") => self.global(memory_base, Mutability::Const)?.into(),
                ("env", "__table_base") => self.global(table_base, Mutability::Const)?.into(),
                ("env", "__stack_pointer") => {
                    self.global(stack + SIDE_STACK, Mutability::Var)?.into()
                }
                ("env", name) if !in_c_api(name) => {
                    return Err(Failure::Refused(format!(
                        "the extension imports `{name}`, which is not the runtime's C API"
                    )));
                }
                ("env", name) => match self.exports.instance.get_export(&mut self.store, name) {
                    Some(item @ Extern::Func(_)) => item,
                    _ => {
                        return Err(Failure::Refused(format!(
                            "the extension calls `{name}`, which the runtime does not export"
                        )));
                    }
                },
                (module, name) => {
                    return Err(Failure::Refused(format!(
                        "the extension imports `{module}.{name}`, which no host provides"
                    )));
                }
            };
            linker
                .define(&self.store, module, name, item)
                .map_err(Failure::Trap)?;
        }
        let instance = linker
            .instantiate(&mut self.store, extension.module())
            .map_err(Failure::Trap)?;
        for constructor in ["__wasm_apply_data_relocs", "__wasm_call_ctors"] {
            if let Ok(run) = instance.get_typed_func::<(), ()>(&mut self.store, constructor) {
                run.call(&mut self.store, ()).map_err(Failure::Trap)?;
            }
        }
        let entry = format!("{}_Init", extension.prefix());
        let init = instance
            .get_typed_func::<i32, i32>(&mut self.store, &entry)
            .map_err(|_| Failure::Refused(format!("the extension defines no `{entry}`")))?;
        let code = init
            .call(&mut self.store, self.interp)
            .map_err(Failure::Trap)?;
        if code != 0 {
            return Err(self.script_error()?);
        }
        Ok(())
    }

    /// `bytes` of the runtime's heap aligned to `2^align`, kept for the
    /// session's life; zero for none.
    fn reserve(&mut self, bytes: u32, align: u32) -> Result<i32, Failure> {
        if bytes == 0 {
            return Ok(0);
        }
        let size =
            i32::try_from(bytes).map_err(|_| Failure::Refused("too much data".to_owned()))?;
        let align = 1_i32
            .checked_shl(align)
            .filter(|&align| align > 0)
            .ok_or_else(|| Failure::Refused("an impossible alignment".to_owned()))?;
        let base = call(&mut self.store, &self.exports.alloc, (size, align))?;
        if base == 0 {
            return Err(Failure::Refused(
                "the runtime could not reserve the data".to_owned(),
            ));
        }
        Ok(base)
    }

    /// Grow the table for a side module's functions, aligned as it asks, and
    /// answer the first slot.
    fn grow_table(&mut self, layout: Layout) -> Result<i32, Failure> {
        let current = self.exports.table.size(&self.store);
        let align = 1_u64 << layout.table_align.min(31);
        let base = current.div_ceil(align) * align;
        let grow = base - current + u64::from(layout.table_size);
        if grow > 0 {
            self.exports
                .table
                .grow(&mut self.store, grow, Ref::Func(None))
                .map_err(Failure::Trap)?;
        }
        i32::try_from(base).map_err(|_| Failure::Refused("the table is too large".to_owned()))
    }

    /// An `i32` global holding `value`.
    fn global(&mut self, value: i32, mutability: Mutability) -> Result<Global, Failure> {
        Global::new(
            &mut self.store,
            GlobalType::new(ValType::I32, mutability),
            Val::I32(value),
        )
        .map_err(Failure::Trap)
    }

    /// A fresh object holding `bytes`, owned by the caller.
    fn object(&mut self, bytes: &[u8]) -> Result<i32, Failure> {
        let length = i32::try_from(bytes.len())
            .map_err(|_| Failure::Refused("a word is too long".to_owned()))?;
        let buffer = call(&mut self.store, &self.exports.alloc, (length.max(1), 1))?;
        self.exports
            .memory
            .write(&mut self.store, address(buffer), bytes)
            .map_err(|error| Failure::Trap(wasmtime::Error::new(error)))?;
        let object = call(&mut self.store, &self.exports.new_string, (buffer, length))?;
        call(&mut self.store, &self.exports.free, buffer)?;
        Ok(object)
    }

    /// Release `objects`, each held once by the session.
    fn release(&mut self, objects: &[i32]) -> Result<(), Failure> {
        for &object in objects {
            call(&mut self.store, &self.exports.release, object)?;
        }
        Ok(())
    }

    /// A NUL-terminated copy of `text` in the runtime's heap; free it after.
    fn c_string(&mut self, text: &str) -> Result<i32, Failure> {
        let mut bytes = text.as_bytes().to_vec();
        bytes.push(0);
        let length = i32::try_from(bytes.len())
            .map_err(|_| Failure::Refused("a name is too long".to_owned()))?;
        let buffer = call(&mut self.store, &self.exports.alloc, (length, 1))?;
        self.exports
            .memory
            .write(&mut self.store, address(buffer), &bytes)
            .map_err(|error| Failure::Trap(wasmtime::Error::new(error)))?;
        Ok(buffer)
    }

    fn script_error(&mut self) -> Result<Failure, Failure> {
        let error = command_receipts::guest_error(&mut self.store, &self.exports, self.interp, 1)?;
        match error {
            EngineError::ScriptBytes {
                message,
                code,
                options,
            } => Ok(Failure::Guest {
                message,
                code,
                options,
            }),
            _ => unreachable!("guest_error retains the genuine guest channel"),
        }
    }

    /// Lift an evaluation's limits once it has ended, so what is set up on the
    /// instance between evaluations (a command defined, an extension loaded)
    /// runs under none: no fuel left over, no deadline already passed, no cap.
    pub(crate) fn disarm(&mut self) {
        // Fuel is on, so setting it cannot fail.
        let _ = self.store.set_fuel(u64::MAX);
        self.store.set_epoch_deadline(u64::MAX / 2);
        self.store.data_mut().cap.limit = None;
    }

    /// Admit a new independent public engine operation, never a nested callback.
    pub(crate) fn begin_entry(&mut self) -> Result<(), Failure> {
        call(&mut self.store, &self.exports.begin, self.interp)
    }

    /// Arm the limits of one evaluation.
    pub(crate) fn arm(&mut self, limits: Limits) -> Result<(), Failure> {
        self.store
            .set_fuel(limits.fuel.unwrap_or(u64::MAX))
            .map_err(Failure::Trap)?;
        self.store
            .set_epoch_deadline(limits.ticks.unwrap_or(u64::MAX / 2));
        let current = self.exports.memory.data_size(&self.store) as u64;
        self.store.data_mut().cap.limit = limits
            .growth
            .map(|growth| usize::try_from(current.saturating_add(growth)).unwrap_or(usize::MAX));
        let commands = limits
            .commands
            .map_or(-1, |commands| i64::try_from(commands).unwrap_or(i64::MAX));
        let value_bytes = limits
            .value_bytes
            .map_or(-1, |bytes| i64::try_from(bytes).unwrap_or(i64::MAX));
        call(
            &mut self.store,
            &self.exports.set_limits,
            (self.interp, commands, value_bytes),
        )?;
        call(&mut self.store, &self.exports.begin, self.interp)
    }

    /// The limit the interpreter says the evaluation outran.
    pub(crate) fn exceeded(&mut self) -> Result<Option<BudgetKind>, Failure> {
        Ok(
            match call(&mut self.store, &self.exports.exceeded, self.interp)? {
                1 => Some(BudgetKind::Commands),
                2 => Some(BudgetKind::WallClock),
                3 => Some(BudgetKind::ValueSize),
                _ => None,
            },
        )
    }

    /// The commands the evaluation dispatched.
    pub(crate) fn commands_spent(&mut self) -> Option<u64> {
        call(&mut self.store, &self.exports.spent, self.interp)
            .ok()
            .and_then(|spent| u64::try_from(spent).ok())
    }

    /// Evaluate the command `words` at the interpreter's top level.
    pub(crate) fn evaluate(&mut self, words: &[&[u8]]) -> Result<Completion, Failure> {
        let mut originals = Vec::new();
        for word in words {
            originals.push(self.object(word)?);
        }
        self.evaluate_originals(&originals)
    }

    /// Owns and releases every argument reference even after a host refusal.
    fn evaluate_originals(&mut self, objects: &[i32]) -> Result<Completion, Failure> {
        let count =
            i32::try_from(objects.len()).map_err(|_| Failure::Refused("too many words".into()))?;
        let size = count
            .max(1)
            .checked_mul(4)
            .ok_or_else(|| Failure::Refused("argument vector is too large".into()))?;
        let argv = call(&mut self.store, &self.exports.alloc, (size, 4))?;
        for (index, object) in objects.iter().enumerate() {
            self.exports
                .memory
                .write(
                    &mut self.store,
                    address(argv) + index * 4,
                    &object.to_le_bytes(),
                )
                .map_err(|error| Failure::Trap(wasmtime::Error::new(error)))?;
        }
        let out = self.scratch + 4;
        call(
            &mut self.store,
            &self.exports.invoke_argv,
            (argv, count, out),
        )?;
        if let Err(error) = command_receipts::settle(&mut self.store, &self.exports, self.interp) {
            self.release(objects)?;
            call(&mut self.store, &self.exports.free, argv)?;
            return Err(error);
        }
        let completion =
            command_receipts::completion(&mut self.store, &self.exports, self.interp, out);
        self.release(objects)?;
        call(&mut self.store, &self.exports.free, argv)?;
        completion
    }

    /// Register `command` as the host command `name`.
    pub(crate) fn define_command(
        &mut self,
        name: &[u8],
        command: Rc<dyn HostCommand>,
    ) -> Result<CommandReceipt, Failure> {
        let procedure = if let Some(procedure) = self.store.data().host_procedure {
            procedure
        } else {
            let function = crate::host_call::procedure(&mut self.store);
            let slot = self
                .exports
                .table
                .grow(&mut self.store, 1, Ref::Func(Some(function)))
                .map_err(Failure::Trap)?;
            let slot = i32::try_from(slot)
                .map_err(|_| Failure::Refused("the table is too large".to_owned()))?;
            self.store.data_mut().host_procedure = Some(slot);
            slot
        };
        let client = i32::try_from(self.store.data().commands.len())
            .map_err(|_| Failure::Refused("too many host commands".to_owned()))?;
        let receipt = command_receipts::create(
            &mut self.store,
            &self.exports,
            self.interp,
            name,
            procedure,
            client,
        )?;
        self.store.data_mut().commands.push(command);
        self.store.data_mut().host_receipts.push(receipt.clone());
        Ok(receipt)
    }

    /// Retire the selected actual command, retaining its original replay address.
    pub(crate) fn delete_command(
        &mut self,
        name: &[u8],
    ) -> Result<Option<CommandReceipt>, Failure> {
        let receipt = command_receipts::remove(&mut self.store, &self.exports, self.interp, name)?;
        if let Some(receipt) = &receipt {
            self.store
                .data_mut()
                .host_receipts
                .retain(|known| known.generation != receipt.generation);
        }
        Ok(receipt)
    }

    /// Record that the package `name` is provided at `version`.
    pub(crate) fn provide_package(&mut self, name: &str, version: &str) -> Result<(), Failure> {
        let name = self.object(name.as_bytes())?;
        let version = self.object(version.as_bytes())?;
        let code = call(
            &mut self.store,
            &self.exports.provide,
            (self.interp, name, version),
        )?;
        self.release(&[name, version])?;
        command_receipts::settle(&mut self.store, &self.exports, self.interp)?;
        if code == 0 {
            Ok(())
        } else {
            Err(self.script_error()?)
        }
    }

    /// Define the procedure `name` over `parameters` and `body`.
    pub(crate) fn define_unit(
        &mut self,
        key: u32,
        name: &[u8],
        parameters: &[String],
        body: &str,
    ) -> Result<(), Failure> {
        let list = tcl_syntax::list::join_list(parameters.iter().map(String::as_str));
        let objects = [
            self.object(name)?,
            self.object(list.as_bytes())?,
            self.object(body.as_bytes())?,
        ];
        let receipt =
            command_receipts::defined(&mut self.store, &self.exports, self.interp, objects);
        self.release(&objects)?;
        let Some(receipt) = receipt? else {
            return Err(self.script_error()?);
        };
        self.store.data_mut().unit_receipts.insert(key, receipt);
        Ok(())
    }

    /// Keep actual installed host/unit generations and explicitly allowed names.
    pub(crate) fn restrict(&mut self, allowed: &[String]) -> Result<(), Failure> {
        let allowed = self
            .object(tcl_syntax::list::join_list(allowed.iter().map(String::as_str)).as_bytes())?;
        let identities = self
            .store
            .data()
            .host_receipts
            .iter()
            .chain(self.store.data().unit_receipts.values())
            .map(CommandReceipt::identity_bytes)
            .collect::<Vec<_>>();
        let bytes = identities.iter().flatten().copied().collect::<Vec<_>>();
        let count = i32::try_from(identities.len())
            .map_err(|_| Failure::Refused("too many command receipts".into()))?;
        let length = i32::try_from(bytes.len())
            .map_err(|_| Failure::Refused("too many command identity bytes".into()))?;
        let input = call(&mut self.store, &self.exports.alloc, (length.max(1), 8))?;
        self.exports
            .memory
            .write(&mut self.store, address(input), &bytes)
            .map_err(|error| Failure::Trap(wasmtime::Error::new(error)))?;
        let status = call(
            &mut self.store,
            &self.exports.restrict,
            (self.interp, allowed, input, count),
        )?;
        call(&mut self.store, &self.exports.free, input)?;
        self.release(&[allowed])?;
        command_receipts::settle(&mut self.store, &self.exports, self.interp)?;
        if status == 0 {
            Ok(())
        } else {
            Err(Failure::ExecutionRefusal(
                "command restriction failed".into(),
            ))
        }
    }

    pub(crate) fn unit_receipt(&self, key: u32) -> Result<CommandReceipt, Failure> {
        self.store
            .data()
            .unit_receipts
            .get(&key)
            .cloned()
            .ok_or_else(|| Failure::ExecutionRefusal("compiled unit receipt is unavailable".into()))
    }

    pub(crate) fn guard_unit(&mut self, receipt: &CommandReceipt) -> Result<(), Failure> {
        command_receipts::current(&mut self.store, &self.exports, self.interp, receipt)
    }

    pub(crate) fn evaluate_values(&mut self, words: &[Value]) -> Result<Completion, Failure> {
        let mut originals = Vec::new();
        for word in words {
            match command_receipts::value(&mut self.store, &self.exports, self.interp, word) {
                Ok(original) => originals.push(original),
                Err(error) => {
                    self.release(&originals)?;
                    return Err(error);
                }
            }
        }
        self.evaluate_originals(&originals)
    }

    /// Confine stores to the activation.
    pub(crate) fn confine(&mut self) -> Result<(), Failure> {
        call(&mut self.store, &self.exports.confine, self.interp)
    }

    /// Pin the release `profile` names.
    pub(crate) fn set_release(&mut self, profile: &str) -> Result<(), Failure> {
        let length =
            i32::try_from(profile.len()).map_err(|_| Failure::Refused("a long name".to_owned()))?;
        let text = self.c_string(profile)?;
        let code = call(
            &mut self.store,
            &self.exports.set_release,
            (self.interp, text, length),
        )?;
        call(&mut self.store, &self.exports.free, text)?;
        if code == 0 {
            Ok(())
        } else {
            Err(Failure::Refused(format!("no release `{profile}` to pin")))
        }
    }
}

/// Whether `name` is one of the runtime's C API functions, the only exports an
/// extension may import: the header's WASM leg declares them (`Tcl_*`), and its
/// macros and inline functions call `TclHost_*` and `TclFreeObj`. Its other
/// exports — the compiled code's ABI, `tcl_eval`, the engine's own
/// `tcl_engine_*` — would let an extension evaluate a script, store a variable
/// or lift its own budget.
fn in_c_api(name: &str) -> bool {
    name.starts_with("Tcl_") || name.starts_with("TclHost_") || name == "TclFreeObj"
}

/// Why a session call did not answer.
#[derive(Debug)]
pub(crate) enum Failure {
    /// The instance trapped: out of fuel, interrupted, over its memory cap, a
    /// call to `proc_exit`, or a fault. The instance is not used again.
    Trap(wasmtime::Error),
    /// What was asked is not something this host does.
    Refused(String),
    /// A script error the interpreter reported, with its error code.
    Guest {
        message: Vec<u8>,
        code: Option<Vec<u8>>,
        options: Option<Vec<u8>>,
    },
    ExecutionRefusal(String),
}

impl Failure {
    /// The interface's error for this failure.
    pub(crate) fn into_engine_error(self) -> EngineError {
        match self {
            Self::Trap(error) => trap_error(&error),
            Self::Refused(message) => EngineError::Crashed(message),
            Self::Guest {
                message,
                code,
                options,
            } => EngineError::ScriptBytes {
                message,
                code,
                options,
            },
            Self::ExecutionRefusal(reason) => EngineError::ExecutionRefusal(reason),
        }
    }
}

/// The interface's error for a trap: a budget the trap was, or a crash.
pub(crate) fn trap_error(error: &wasmtime::Error) -> EngineError {
    match error.downcast_ref::<Trap>() {
        Some(Trap::OutOfFuel) => return EngineError::BudgetExceeded(BudgetKind::Commands),
        Some(Trap::Interrupt) => return EngineError::BudgetExceeded(BudgetKind::WallClock),
        _ => {}
    }
    if error.downcast_ref::<ValueSizeExceeded>().is_some() {
        return EngineError::BudgetExceeded(BudgetKind::ValueSize);
    }
    if let Some(exit) = error.downcast_ref::<ProcExit>() {
        return EngineError::Crashed(exit.to_string());
    }
    EngineError::Crashed(format!("{error:#}"))
}

/// A linear-memory address as an offset.
pub(crate) fn address(pointer: i32) -> usize {
    pointer.cast_unsigned() as usize
}

/// The string at `chars`, whose length `Tcl_GetStringFromObj` left at `cell`.
pub(crate) fn read_string(
    store: impl wasmtime::AsContext,
    memory: Memory,
    chars: i32,
    cell: i32,
) -> wasmtime::Result<Vec<u8>> {
    let store = store.as_context();
    let mut length = [0; 4];
    memory.read(&store, address(cell), &mut length)?;
    let length = u32::from_le_bytes(length) as usize;
    let mut bytes = vec![0; length];
    memory.read(&store, address(chars), &mut bytes)?;
    Ok(bytes)
}
