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

use tcl_engine_api::{
    BudgetKind, CommandRegistrar, CompletionCode, EngineError, HostCommand, HostOutcome, Value,
};
use wasmtime::{Caller, Func, Store};

use crate::session::{Exports, HostState, address, read_string};

/// What a running host command set up through its door, for the engine to
/// replay on a fresh instance.
pub(crate) enum Defined {
    /// A command defined, with its name.
    Command(String, Rc<dyn HostCommand>),
    /// A command removed.
    Removed(String),
    /// A package provided, with its version.
    Package(String, String),
}

/// The host function every host command is registered with.
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

/// Run the host command registered with `client` on the `count` words at
/// `words`, as the interpreter calls a `Tcl_ObjCmdProc`.
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
    let count = usize::try_from(count).unwrap_or(0);
    let mut table = vec![0; count * 4];
    exports.memory.read(&*caller, address(words), &mut table)?;
    let mut arguments = Vec::with_capacity(count.saturating_sub(1));
    for word in table.as_chunks::<4>().0.iter().skip(1) {
        let object = i32::from_le_bytes(*word);
        let bytes = string_of(caller, &exports, object)?;
        arguments.push(Value::string(String::from_utf8_lossy(&bytes)));
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
            fail(
                caller,
                &exports,
                interp,
                EngineError::Crashed("a host command panicked".to_owned()),
            )
        }
    }
}

/// The bytes of `object`'s string, through the runtime's own export.
fn string_of(
    caller: &mut Caller<'_, HostState>,
    exports: &Exports,
    object: i32,
) -> wasmtime::Result<Vec<u8>> {
    let cell = exports.alloc.call(&mut *caller, (4, 4))?;
    let chars = exports.string_of.call(&mut *caller, (object, cell))?;
    let bytes = read_string(&*caller, exports.memory, chars, cell)?;
    exports.free.call(&mut *caller, cell)?;
    Ok(bytes)
}

/// A fresh object holding `bytes`, owned by the caller.
fn object(
    caller: &mut Caller<'_, HostState>,
    exports: &Exports,
    bytes: &[u8],
) -> wasmtime::Result<i32> {
    let length = i32::try_from(bytes.len())?;
    let buffer = exports.alloc.call(&mut *caller, (length.max(1), 1))?;
    exports.memory.write(&mut *caller, address(buffer), bytes)?;
    let made = exports.new_string.call(&mut *caller, (buffer, length))?;
    exports.free.call(&mut *caller, buffer)?;
    Ok(made)
}

/// An interface value's text: a list's and a dict's are the Tcl list of their
/// elements' texts, a double's is the runtime's own spelling of it.
pub(crate) fn text_of(value: &Value) -> String {
    match value {
        Value::Empty => String::new(),
        Value::Str(text) => text.to_string(),
        Value::Int(number) => number.to_string(),
        Value::Double(number) => tcl_syntax::number::format_double(*number),
        Value::List(items) => tcl_syntax::list::join_list(items.iter().map(text_of)),
        Value::Dict(entries) => tcl_syntax::list::join_list(
            entries
                .iter()
                .flat_map(|(key, item)| [text_of(key), text_of(item)]),
        ),
    }
}

/// Leave a host command's outcome as the interpreter's result and answer its
/// code; a `Return` takes effect as `return -options` does.
fn answer(
    caller: &mut Caller<'_, HostState>,
    exports: &Exports,
    interp: i32,
    outcome: &HostOutcome,
) -> wasmtime::Result<i32> {
    let value = object(caller, exports, text_of(&outcome.value).as_bytes())?;
    let code = if outcome.code == CompletionCode::Return {
        let options = object(caller, exports, text_of(&outcome.options).as_bytes())?;
        let code = exports
            .returning
            .call(&mut *caller, (interp, options, value))?;
        exports.release.call(&mut *caller, options)?;
        code
    } else {
        exports.set_result.call(&mut *caller, (interp, value))?;
        outcome.code.as_int()
    };
    exports.release.call(&mut *caller, value)?;
    Ok(code)
}

/// Leave a host command's failure as the interpreter's error and answer
/// `TCL_ERROR`; a limit the command's own work outran stays that limit.
fn fail(
    caller: &mut Caller<'_, HostState>,
    exports: &Exports,
    interp: i32,
    error: EngineError,
) -> wasmtime::Result<i32> {
    let (message, code) = match error {
        EngineError::BudgetExceeded(kind) => {
            let kind = match kind {
                BudgetKind::Commands => 1,
                BudgetKind::WallClock => 2,
                BudgetKind::ValueSize => 3,
            };
            return exports.exceed.call(&mut *caller, (interp, kind));
        }
        EngineError::Script { message, code } => (message, code),
        other => (other.to_string(), None),
    };
    let message = object(caller, exports, message.as_bytes())?;
    let code = match code {
        Some(code) => object(caller, exports, code.as_bytes())?,
        None => 0,
    };
    let failed = exports.fail.call(&mut *caller, (interp, message, code))?;
    exports.release.call(&mut *caller, message)?;
    if code != 0 {
        exports.release.call(&mut *caller, code)?;
    }
    Ok(failed)
}

/// The door a running host command holds on a WASM engine: what is set up
/// through it takes effect at once, and is recorded for the engine to replay
/// on a fresh instance.
struct WasmRegistrar<'a, 'b> {
    caller: &'a mut Caller<'b, HostState>,
    exports: &'a Exports,
    interp: i32,
}

impl WasmRegistrar<'_, '_> {
    /// The interface's error for a call that trapped.
    fn trapped(error: &wasmtime::Error) -> EngineError {
        crate::session::trap_error(error)
    }
}

impl CommandRegistrar for WasmRegistrar<'_, '_> {
    fn define_command(
        &mut self,
        name: &str,
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
        let text = c_string(self.caller, self.exports, name).map_err(|e| Self::trapped(&e))?;
        let created = self
            .exports
            .create_command
            .call(&mut *self.caller, (self.interp, text, procedure, client, 0))
            .and_then(|_| self.exports.free.call(&mut *self.caller, text));
        created.map_err(|error| Self::trapped(&error))?;
        self.caller.data_mut().commands.push(Rc::clone(&command));
        self.caller
            .data_mut()
            .defined
            .push(Defined::Command(name.to_owned(), command));
        Ok(())
    }

    fn remove_command(&mut self, name: &str) -> Result<bool, EngineError> {
        let text = c_string(self.caller, self.exports, name).map_err(|e| Self::trapped(&e))?;
        let removed = self
            .exports
            .delete_command
            .call(&mut *self.caller, (self.interp, text))
            .and_then(|code| {
                self.exports.free.call(&mut *self.caller, text)?;
                Ok(code == 0)
            })
            .map_err(|error| Self::trapped(&error))?;
        self.caller
            .data_mut()
            .defined
            .push(Defined::Removed(name.to_owned()));
        Ok(removed)
    }

    fn provide_package(&mut self, name: &str, version: &str) -> Result<(), EngineError> {
        let provided = (|| -> wasmtime::Result<Result<(), EngineError>> {
            let objects = [
                object(self.caller, self.exports, name.as_bytes())?,
                object(self.caller, self.exports, version.as_bytes())?,
            ];
            let code = self
                .exports
                .provide
                .call(&mut *self.caller, (self.interp, objects[0], objects[1]))?;
            for object in objects {
                self.exports.release.call(&mut *self.caller, object)?;
            }
            if code == 0 {
                return Ok(Ok(()));
            }
            let result = self
                .exports
                .get_result
                .call(&mut *self.caller, self.interp)?;
            let message = string_of(self.caller, self.exports, result)?;
            let error_code = self
                .exports
                .error_code
                .call(&mut *self.caller, self.interp)?;
            let code_text = string_of(self.caller, self.exports, error_code)?;
            self.exports.release.call(&mut *self.caller, error_code)?;
            Ok(Err(EngineError::Script {
                message: String::from_utf8_lossy(&message).into_owned(),
                code: Some(String::from_utf8_lossy(&code_text).into_owned()),
            }))
        })()
        .map_err(|error| Self::trapped(&error))?;
        provided?;
        self.caller
            .data_mut()
            .defined
            .push(Defined::Package(name.to_owned(), version.to_owned()));
        Ok(())
    }
}

/// A NUL-terminated copy of `text` in the runtime's heap; free it after.
fn c_string(
    caller: &mut Caller<'_, HostState>,
    exports: &Exports,
    text: &str,
) -> wasmtime::Result<i32> {
    let mut bytes = text.as_bytes().to_vec();
    bytes.push(0);
    let buffer = exports
        .alloc
        .call(&mut *caller, (i32::try_from(bytes.len())?, 1))?;
    exports
        .memory
        .write(&mut *caller, address(buffer), &bytes)?;
    Ok(buffer)
}
