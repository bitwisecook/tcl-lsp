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

//! The procedure every host command a WASM engine registers is registered
//! with: a host function in the runtime's function table, which the
//! interpreter calls as it calls an extension's `Tcl_ObjCmdProc`, its client
//! data the command's index in the store. The words become the interface's
//! values, the command runs with a door onto the interpreter, and its answer
//! becomes the interpreter's result and completion code, as the runtime's
//! native engine makes them (`runtime/rust/src/engine.rs`).

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use crate::command_receipts;
use crate::session::{Exports, HostState, address};
use tcl_engine_api::{
    BudgetKind, CommandRegistrar, EngineError, HostArgumentView, HostCommand, HostOutcome, Value,
};
use wasmtime::{Caller, Func, Store};

/// Authored setup retaining actual publication addresses for fresh replay.
pub(crate) enum Defined {
    Command(Vec<u8>, Rc<dyn HostCommand>),
    Removed(Vec<u8>),
    Package(String, String),
}

pub(crate) fn procedure(store: &mut Store<HostState>) -> Func {
    Func::wrap(
        store,
        |mut caller: Caller<'_, HostState>,
         client: i32,
         interp: i32,
         count: i32,
         words: i32|
         -> wasmtime::Result<i32> { call(&mut caller, client, interp, count, words) },
    )
}

fn call(
    caller: &mut Caller<'_, HostState>,
    client: i32,
    interp: i32,
    count: i32,
    words: i32,
) -> wasmtime::Result<i32> {
    let exports = caller
        .data()
        .exports
        .clone()
        .ok_or_else(|| wasmtime::Error::msg("the runtime is not instantiated"))?;
    let command = usize::try_from(client)
        .ok()
        .and_then(|index| caller.data().commands.get(index).cloned())
        .ok_or_else(|| wasmtime::Error::msg("no host command at this client data"))?;
    match command.argument_view() {
        HostArgumentView::MaterializedStrings => {}
        HostArgumentView::NativeObjectSnapshots => {
            return refuse(
                caller,
                &exports,
                interp,
                "native object snapshot callbacks are unavailable across this WASM boundary",
            );
        }
        HostArgumentView::OriginalObjects => {
            return refuse(
                caller,
                &exports,
                interp,
                "original object callbacks are unavailable across this WASM boundary",
            );
        }
    }
    let count = usize::try_from(count)?;
    let length = count
        .checked_mul(4)
        .ok_or_else(|| wasmtime::Error::msg("argument vector is too large"))?;
    let mut table = vec![0; length];
    exports.memory.read(&*caller, address(words), &mut table)?;
    let mut arguments = Vec::with_capacity(count.saturating_sub(1));
    for word in table.as_chunks::<4>().0.iter().skip(1) {
        let original = i32::from_le_bytes(*word);
        match command_receipts::string_bytes(caller, &exports, interp, original) {
            Ok(bytes) => arguments.push(Value::string_bytes(bytes)),
            Err(error) => return fail(caller, &exports, interp, error.into_engine_error()),
        }
    }
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let mut registrar = WasmRegistrar {
            caller: &mut *caller,
            exports: &exports,
            interp,
        };
        command.invoke_with_registrar(&mut registrar, &arguments)
    }));
    match outcome {
        Ok(Ok(outcome)) => answer(caller, &exports, interp, &outcome),
        Ok(Err(error)) => fail(caller, &exports, interp, error),
        Err(payload) => {
            caller.data_mut().panic = Some(payload);
            refuse(caller, &exports, interp, "a host command panicked")
        }
    }
}

fn refuse(
    caller: &mut Caller<'_, HostState>,
    exports: &Exports,
    interp: i32,
    reason: &str,
) -> wasmtime::Result<i32> {
    let (input, length) = command_receipts::buffer(caller, exports, reason.as_bytes())
        .map_err(|error| wasmtime::Error::msg(error.into_engine_error().to_string()))?;
    let code = exports
        .refuse_host
        .call(&mut *caller, (interp, input, length))?;
    exports.free.call(&mut *caller, input)?;
    Ok(code)
}

fn imported(
    caller: &mut Caller<'_, HostState>,
    exports: &Exports,
    interp: i32,
    value: &Value,
) -> Result<i32, EngineError> {
    command_receipts::value(caller, exports, interp, value)
        .map_err(crate::session::Failure::into_engine_error)
}

fn answer(
    caller: &mut Caller<'_, HostState>,
    exports: &Exports,
    interp: i32,
    outcome: &HostOutcome,
) -> wasmtime::Result<i32> {
    let value = match imported(caller, exports, interp, &outcome.value) {
        Ok(value) => value,
        Err(error) => return fail(caller, exports, interp, error),
    };
    let options = if matches!(outcome.options, Value::Empty) {
        0
    } else {
        match imported(caller, exports, interp, &outcome.options) {
            Ok(options) => options,
            Err(error) => {
                exports.release.call(&mut *caller, value)?;
                return fail(caller, exports, interp, error);
            }
        }
    };
    let code = exports.complete.call(
        &mut *caller,
        (interp, value, options, outcome.code.as_int()),
    )?;
    exports.release.call(&mut *caller, value)?;
    if options != 0 {
        exports.release.call(&mut *caller, options)?;
    }
    Ok(code)
}

fn fail(
    caller: &mut Caller<'_, HostState>,
    exports: &Exports,
    interp: i32,
    error: EngineError,
) -> wasmtime::Result<i32> {
    let (message, error_code, options) = match error {
        EngineError::BudgetExceeded(kind) => {
            return exports.exceed.call(
                &mut *caller,
                (
                    interp,
                    match kind {
                        BudgetKind::Commands => 1,
                        BudgetKind::WallClock => 2,
                        BudgetKind::ValueSize => 3,
                    },
                ),
            );
        }
        EngineError::Script { message, code } => {
            (message.into_bytes(), code.map(String::into_bytes), None)
        }
        EngineError::ScriptBytes {
            message,
            code,
            options,
        } => (message, code, options),
        EngineError::ExecutionRefusal(reason) => return refuse(caller, exports, interp, &reason),
        EngineError::Compile(reason) | EngineError::Crashed(reason) => {
            return refuse(caller, exports, interp, &reason);
        }
        EngineError::Unsupported(reason) => return refuse(caller, exports, interp, reason),
    };
    if let Some(options) = options {
        return answer(
            caller,
            exports,
            interp,
            &HostOutcome {
                value: Value::string_bytes(message),
                code: tcl_engine_api::CompletionCode::Error,
                options: Value::string_bytes(options),
            },
        );
    }
    let message = command_receipts::object(caller, exports, &message)
        .map_err(|error| wasmtime::Error::msg(error.into_engine_error().to_string()))?;
    let code = match error_code {
        Some(bytes) => command_receipts::object(caller, exports, &bytes)
            .map_err(|error| wasmtime::Error::msg(error.into_engine_error().to_string()))?,
        None => 0,
    };
    let failed = exports.fail.call(&mut *caller, (interp, message, code))?;
    exports.release.call(&mut *caller, message)?;
    if code != 0 {
        exports.release.call(&mut *caller, code)?;
    }
    Ok(failed)
}

struct WasmRegistrar<'a, 'b> {
    caller: &'a mut Caller<'b, HostState>,
    exports: &'a Exports,
    interp: i32,
}

impl CommandRegistrar for WasmRegistrar<'_, '_> {
    fn define_command(
        &mut self,
        name: &str,
        command: Rc<dyn HostCommand>,
    ) -> Result<(), EngineError> {
        self.define_command_bytes(name.as_bytes(), command)
    }

    fn define_command_bytes(
        &mut self,
        name: &[u8],
        command: Rc<dyn HostCommand>,
    ) -> Result<(), EngineError> {
        let procedure = self
            .caller
            .data()
            .host_procedure
            .ok_or(EngineError::Unsupported(
                "defining a command before any host command",
            ))?;
        let client = i32::try_from(self.caller.data().commands.len())
            .map_err(|_| EngineError::Unsupported("so many host commands"))?;
        let receipt = command_receipts::create(
            self.caller,
            self.exports,
            self.interp,
            name,
            procedure,
            client,
        )
        .map_err(crate::session::Failure::into_engine_error)?;
        self.caller.data_mut().commands.push(Rc::clone(&command));
        self.caller.data_mut().host_receipts.push(receipt.clone());
        self.caller
            .data_mut()
            .defined
            .push(Defined::Command(receipt.qualified, command));
        Ok(())
    }

    fn remove_command(&mut self, name: &str) -> Result<bool, EngineError> {
        self.remove_command_bytes(name.as_bytes())
    }

    fn remove_command_bytes(&mut self, name: &[u8]) -> Result<bool, EngineError> {
        let receipt = command_receipts::remove(self.caller, self.exports, self.interp, name)
            .map_err(crate::session::Failure::into_engine_error)?;
        if let Some(receipt) = receipt {
            self.caller
                .data_mut()
                .host_receipts
                .retain(|known| known.generation != receipt.generation);
            self.caller
                .data_mut()
                .defined
                .push(Defined::Removed(receipt.qualified));
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn provide_package(&mut self, name: &str, version: &str) -> Result<(), EngineError> {
        let provided = (|| -> Result<(), crate::session::Failure> {
            let name = command_receipts::object(self.caller, self.exports, name.as_bytes())?;
            let version = command_receipts::object(self.caller, self.exports, version.as_bytes())?;
            let code = command_receipts::call(
                self.caller,
                &self.exports.provide,
                (self.interp, name, version),
            )?;
            command_receipts::call(self.caller, &self.exports.release, name)?;
            command_receipts::call(self.caller, &self.exports.release, version)?;
            command_receipts::settle(self.caller, self.exports, self.interp)?;
            if code == 0 {
                Ok(())
            } else {
                let error =
                    command_receipts::guest_error(self.caller, self.exports, self.interp, code)?;
                match error {
                    EngineError::ScriptBytes {
                        message,
                        code,
                        options,
                    } => Err(crate::session::Failure::Guest {
                        message,
                        code,
                        options,
                    }),
                    _ => unreachable!("guest_error produces only actual guest failures"),
                }
            }
        })();
        provided.map_err(crate::session::Failure::into_engine_error)?;
        self.caller
            .data_mut()
            .defined
            .push(Defined::Package(name.to_owned(), version.to_owned()));
        Ok(())
    }
}
