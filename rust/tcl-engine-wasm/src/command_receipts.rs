// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Counted transport of receipts issued by the runtime's installed-command owner.
//! A replay address is data; only its current instance's receipt grants access.

use tcl_engine_api::engine_command_replay::{
    EngineCommandReplayContext, EngineCommandReplayPurpose,
};
use tcl_engine_api::{EngineError, engine_interpreter_identity::EngineInterpreterIdentity};
use wasmtime::{AsContextMut, TypedFunc, WasmParams, WasmResults};

use crate::session::{Exports, Failure, HostState, address, read_string};

#[derive(Clone, Debug)]
pub(crate) struct CommandReceipt {
    owner: u64,
    interpreter: u64,
    pub(crate) generation: u64,
    pub(crate) qualified: Vec<u8>,
}

/// Fresh setup retains original bytes and separately captured holder geometry.
#[derive(Clone, Debug)]
pub(crate) struct CommandReplay {
    pub(crate) original: Vec<u8>,
    pub(crate) context: Option<EngineCommandReplayContext>,
    pub(crate) interpreter: ReplayInterpreter,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum ReplayInterpreter {
    Root,
    Unsupported,
}

impl CommandReplay {
    pub(crate) fn root(original: &[u8]) -> Self {
        Self {
            original: original.to_vec(),
            context: None,
            interpreter: ReplayInterpreter::Root,
        }
    }
}

pub(crate) fn replay_input<C: AsContextMut<Data = HostState>>(
    context: &mut C,
    exports: &Exports,
    interp: i32,
    purpose: EngineCommandReplayPurpose,
    original: &[u8],
) -> Result<CommandReplay, Failure> {
    let purpose_number = match purpose {
        EngineCommandReplayPurpose::Create => 1,
        EngineCommandReplayPurpose::Remove => 2,
    };
    let object = call(context, &exports.replay_context, (interp, purpose_number))?;
    settle(context, exports, interp)?;
    if object == 0 {
        return Err(Failure::ExecutionRefusal(
            "actual command replay context unavailable".into(),
        ));
    }
    let bytes = string_bytes(context, exports, interp, object);
    call(context, &exports.release, object)?;
    let replay_context = EngineCommandReplayContext::decode(&bytes?).ok_or_else(|| {
        Failure::ExecutionRefusal("actual command replay context has no counted recipe".into())
    })?;
    if replay_context.purpose != purpose {
        return Err(Failure::ExecutionRefusal(
            "actual command replay purpose changed".into(),
        ));
    }
    let identity = interpreter_identity(context, exports, interp)?;
    // Callback Interp wrappers can move while retaining the installed owner.
    let is_root = replay_context.root_interpreter
        && context
            .as_context()
            .data()
            .root_interpreter
            .is_some_and(|(_, installed)| installed == identity);
    Ok(CommandReplay {
        original: original.to_vec(),
        context: Some(replay_context),
        interpreter: if is_root {
            ReplayInterpreter::Root
        } else {
            ReplayInterpreter::Unsupported
        },
    })
}

impl CommandReceipt {
    /// Counted repr(C) identity header, independently of its replay address.
    pub(crate) fn identity_bytes(&self) -> [u8; 24] {
        let mut bytes = [0; 24];
        bytes[..8].copy_from_slice(&self.owner.to_le_bytes());
        bytes[8..16].copy_from_slice(&self.interpreter.to_le_bytes());
        bytes[16..].copy_from_slice(&self.generation.to_le_bytes());
        bytes
    }
}

pub(crate) fn call<C: AsContextMut<Data = HostState>, P: WasmParams, R: WasmResults>(
    context: &mut C,
    function: &TypedFunc<P, R>,
    params: P,
) -> Result<R, Failure> {
    function
        .call(context.as_context_mut(), params)
        .map_err(Failure::Trap)
}

pub(crate) fn buffer<C: AsContextMut<Data = HostState>>(
    context: &mut C,
    exports: &Exports,
    bytes: &[u8],
) -> Result<(i32, i32), Failure> {
    let length = i32::try_from(bytes.len())
        .map_err(|_| Failure::Refused("a counted input is too long".into()))?;
    let pointer = call(context, &exports.alloc, (length.max(1), 1))?;
    exports
        .memory
        .write(context.as_context_mut(), address(pointer), bytes)
        .map_err(|error| Failure::Trap(wasmtime::Error::new(error)))?;
    Ok((pointer, length))
}

pub(crate) fn owned_string<C: AsContextMut<Data = HostState>>(
    context: &mut C,
    exports: &Exports,
    object: i32,
) -> Result<Vec<u8>, Failure> {
    let cell = call(context, &exports.alloc, (4, 4))?;
    let chars = call(context, &exports.string_of, (object, cell))?;
    let bytes =
        read_string(context.as_context(), exports.memory, chars, cell).map_err(Failure::Trap);
    call(context, &exports.free, cell)?;
    call(context, &exports.release, object)?;
    bytes
}

/// Decode only the actual callback-free diagnostic projection. Missing identity
/// is terminal for typed transport; no raw pointer or command lookup fallback.
pub(crate) fn interpreter_identity<C: AsContextMut<Data = HostState>>(
    context: &mut C,
    exports: &Exports,
    interp: i32,
) -> Result<EngineInterpreterIdentity, Failure> {
    let out = call(context, &exports.alloc, (16, 1))?;
    let reached = (|| {
        if call(context, &exports.interpreter_identity, (interp, out))? != 0 {
            return Err(Failure::ExecutionRefusal(
                "actual interpreter diagnostic identity unavailable".into(),
            ));
        }
        let mut bytes = [0; EngineInterpreterIdentity::BYTE_LENGTH];
        exports
            .memory
            .read(context.as_context(), address(out), &mut bytes)
            .map_err(|error| Failure::Trap(wasmtime::Error::new(error)))?;
        Ok(EngineInterpreterIdentity::from_bytes(bytes))
    })();
    call(context, &exports.free, out)?;
    reached
}

/// The runtime retains the full typed first cause. This text is presentation only.
pub(crate) fn host_error<C: AsContextMut<Data = HostState>>(
    context: &mut C,
    exports: &Exports,
    interp: i32,
) -> Result<Option<EngineError>, Failure> {
    if call(context, &exports.host_refusal_pending, interp)? == 0 {
        return Ok(None);
    }
    let identity = interpreter_identity(context, exports, interp)?;
    let state = context.as_context();
    if let Some(record) = state.data().execution_causes.get(&identity)
        && let Some(original) = record.episode.upgrade()
        && state
            .data()
            .entry_episode
            .as_ref()
            .is_some_and(|current| std::rc::Rc::ptr_eq(current, &original))
    {
        return Ok(Some(EngineError::execution_failure(record.cause.clone())));
    }
    let object = call(context, &exports.host_refusal_text, interp)?;
    if object == 0 {
        return Err(Failure::ExecutionRefusal(
            "host refusal presentation is unavailable".into(),
        ));
    }
    let bytes = owned_string(context, exports, object)?;
    let reason = String::from_utf8(bytes).map_err(|_| {
        Failure::ExecutionRefusal("host refusal presentation is not Unicode".into())
    })?;
    Ok(Some(EngineError::ExecutionRefusal(reason)))
}

pub(crate) fn settle<C: AsContextMut<Data = HostState>>(
    context: &mut C,
    exports: &Exports,
    interp: i32,
) -> Result<(), Failure> {
    match host_error(context, exports, interp)? {
        None => Ok(()),
        Some(EngineError::ExecutionRefusal(reason)) => Err(Failure::ExecutionRefusal(reason)),
        Some(EngineError::ExecutionFailure(cause)) => Err(Failure::ExecutionFailure(*cause)),
        Some(_) => unreachable!("host_error supplies only the host channel"),
    }
}

/// Materialise through the authentic selected getter before reading owned bytes.
pub(crate) fn string_bytes<C: AsContextMut<Data = HostState>>(
    context: &mut C,
    exports: &Exports,
    interp: i32,
    original: i32,
) -> Result<Vec<u8>, Failure> {
    settle(context, exports, interp)?;
    let object = call(context, &exports.string_snapshot, (interp, original))?;
    settle(context, exports, interp)?;
    if object == 0 {
        return Err(Failure::ExecutionRefusal(
            "original string snapshot is unavailable".into(),
        ));
    }
    owned_string(context, exports, object)
}

fn read_receipt<C: AsContextMut<Data = HostState>>(
    context: &mut C,
    exports: &Exports,
    interp: i32,
    out: i32,
) -> Result<CommandReceipt, Failure> {
    let mut raw = [0; 32];
    exports
        .memory
        .read(context.as_context(), address(out), &mut raw)
        .map_err(|error| Failure::Trap(wasmtime::Error::new(error)))?;
    let number = |at| u64::from_le_bytes(raw[at..at + 8].try_into().expect("eight receipt bytes"));
    let object = i32::from_le_bytes(raw[24..28].try_into().expect("four receipt bytes"));
    let qualified = string_bytes(context, exports, interp, object);
    call(context, &exports.release, object)?;
    Ok(CommandReceipt {
        owner: number(0),
        interpreter: number(8),
        generation: number(16),
        qualified: qualified?,
    })
}

pub(crate) fn create<C: AsContextMut<Data = HostState>>(
    context: &mut C,
    exports: &Exports,
    interp: i32,
    name: &[u8],
    procedure: i32,
    client: i32,
) -> Result<CommandReceipt, Failure> {
    create_with_context(context, exports, interp, name, procedure, client, None)
}

pub(crate) fn create_with_context<C: AsContextMut<Data = HostState>>(
    context: &mut C,
    exports: &Exports,
    interp: i32,
    name: &[u8],
    procedure: i32,
    client: i32,
    replay: Option<&EngineCommandReplayContext>,
) -> Result<CommandReceipt, Failure> {
    let (input, length) = buffer(context, exports, name)?;
    let out = call(context, &exports.alloc, (32, 8))?;
    let status = if let Some(replay) = replay {
        let bytes = replay.encode().ok_or_else(|| {
            Failure::ExecutionRefusal("command replay context is unavailable".into())
        })?;
        let (source, count) = buffer(context, exports, &bytes)?;
        let status = call(
            context,
            &exports.replay_create_command,
            (interp, source, count, input, length, procedure, client, out),
        )?;
        call(context, &exports.free, source)?;
        status
    } else {
        call(
            context,
            &exports.create_command,
            (interp, input, length, procedure, client, out),
        )?
    };
    call(context, &exports.free, input)?;
    let answer = settle(context, exports, interp).and_then(|()| {
        if status != 0 {
            return Err(Failure::ExecutionRefusal(
                "command publication returned no receipt".into(),
            ));
        }
        read_receipt(context, exports, interp, out)
    });
    call(context, &exports.free, out)?;
    answer
}

pub(crate) fn remove<C: AsContextMut<Data = HostState>>(
    context: &mut C,
    exports: &Exports,
    interp: i32,
    name: &[u8],
) -> Result<Option<CommandReceipt>, Failure> {
    remove_with_context(context, exports, interp, name, None)
}

pub(crate) fn remove_with_context<C: AsContextMut<Data = HostState>>(
    context: &mut C,
    exports: &Exports,
    interp: i32,
    name: &[u8],
    replay: Option<&EngineCommandReplayContext>,
) -> Result<Option<CommandReceipt>, Failure> {
    let (input, length) = buffer(context, exports, name)?;
    let out = call(context, &exports.alloc, (32, 8))?;
    let status = if let Some(replay) = replay {
        let bytes = replay.encode().ok_or_else(|| {
            Failure::ExecutionRefusal("command replay context is unavailable".into())
        })?;
        let (source, count) = buffer(context, exports, &bytes)?;
        let status = call(
            context,
            &exports.replay_delete_command,
            (interp, source, count, input, length, out),
        )?;
        call(context, &exports.free, source)?;
        status
    } else {
        call(
            context,
            &exports.delete_command,
            (interp, input, length, out),
        )?
    };
    call(context, &exports.free, input)?;
    let answer = settle(context, exports, interp).and_then(|()| match status {
        0 => read_receipt(context, exports, interp, out).map(Some),
        -1 => Ok(None),
        _ => Err(Failure::ExecutionRefusal(
            "command retirement returned no receipt".into(),
        )),
    });
    call(context, &exports.free, out)?;
    answer
}

pub(crate) fn defined<C: AsContextMut<Data = HostState>>(
    context: &mut C,
    exports: &Exports,
    interp: i32,
    objects: [i32; 3],
) -> Result<Option<CommandReceipt>, Failure> {
    let out = call(context, &exports.alloc, (32, 8))?;
    let status = call(
        context,
        &exports.define_unit,
        (interp, objects[0], objects[1], objects[2], out),
    )?;
    let answer = settle(context, exports, interp).and_then(|()| {
        if status != 0 {
            return Ok(None);
        }
        read_receipt(context, exports, interp, out).map(Some)
    });
    call(context, &exports.free, out)?;
    answer
}

pub(crate) fn current<C: AsContextMut<Data = HostState>>(
    context: &mut C,
    exports: &Exports,
    interp: i32,
    receipt: &CommandReceipt,
) -> Result<(), Failure> {
    let (input, length) = buffer(context, exports, &receipt.qualified)?;
    let status = call(
        context,
        &exports.receipt_current,
        (
            interp,
            receipt.owner.cast_signed(),
            receipt.interpreter.cast_signed(),
            receipt.generation.cast_signed(),
            input,
            length,
        ),
    )?;
    call(context, &exports.free, input)?;
    settle(context, exports, interp)?;
    if status == 0 {
        Ok(())
    } else {
        Err(Failure::ExecutionRefusal(
            "installed command receipt is not current".into(),
        ))
    }
}

pub(crate) fn object<C: AsContextMut<Data = HostState>>(
    context: &mut C,
    exports: &Exports,
    bytes: &[u8],
) -> Result<i32, Failure> {
    let (input, length) = buffer(context, exports, bytes)?;
    let original = call(context, &exports.new_string, (input, length))?;
    call(context, &exports.free, input)?;
    Ok(original)
}

/// Import payload storage through original producers; no host text conversion.
pub(crate) fn value<C: AsContextMut<Data = HostState>>(
    context: &mut C,
    exports: &Exports,
    interp: i32,
    payload: &tcl_engine_api::Value,
) -> Result<i32, Failure> {
    use tcl_engine_api::{NativeStringStorageIdentity, Value};
    settle(context, exports, interp)?;
    let original = match payload {
        Value::Empty => object(context, exports, b"")?,
        Value::Str(text) => object(context, exports, text.as_bytes())?,
        Value::StringBytes(bytes) => object(context, exports, bytes)?,
        Value::ByteArray(bytes) => {
            let (input, length) = buffer(context, exports, bytes)?;
            let original = call(context, &exports.new_byte_array, (interp, input, length))?;
            call(context, &exports.free, input)?;
            original
        }
        Value::NativeScalar(cache) => {
            let bytes = tcl_syntax::scalar_getter::carrier::encode_scalar(cache)
                .map_err(|error| Failure::ExecutionRefusal(error.to_string()))?;
            let (input, length) = buffer(context, exports, &bytes)?;
            let original = call(context, &exports.new_scalar, (interp, input, length))?;
            call(context, &exports.free, input)?;
            original
        }
        Value::Resident {
            value: payload,
            string,
            storage,
        } => {
            tcl_syntax::scalar_getter::carrier::checked_storage(*storage, string.len())
                .map_err(|error| Failure::ExecutionRefusal(error.to_string()))?;
            let original = value(context, exports, interp, payload)?;
            let (input, length) = buffer(context, exports, string)?;
            let kind = match storage {
                NativeStringStorageIdentity::CanonicalEmpty => 0,
                NativeStringStorageIdentity::Allocated => 1,
                NativeStringStorageIdentity::Unknown => 2,
            };
            let status = call(
                context,
                &exports.adopt_resident,
                (interp, original, input, length, kind),
            )?;
            call(context, &exports.free, input)?;
            if status != 0 {
                call(context, &exports.release, original)?;
                settle(context, exports, interp)?;
                return Err(Failure::ExecutionRefusal(
                    "resident carrier adoption failed".into(),
                ));
            }
            original
        }
        Value::Int(integer) => {
            let original = call(context, &exports.new_int, *integer)?;
            call(context, &exports.retain, original)?;
            original
        }
        Value::Double(double) => {
            let original = call(context, &exports.new_double, *double)?;
            call(context, &exports.retain, original)?;
            original
        }
        Value::List(items) => sequence(context, exports, interp, items.iter(), false)?,
        Value::Dict(items) => sequence(
            context,
            exports,
            interp,
            items.iter().flat_map(|(key, value)| [key, value]),
            true,
        )?,
    };
    if let Err(error) = settle(context, exports, interp) {
        if original != 0 {
            call(context, &exports.release, original)?;
        }
        return Err(error);
    }
    if original == 0 {
        return Err(Failure::ExecutionRefusal(
            "original value issuer is unavailable".into(),
        ));
    }
    Ok(original)
}

fn sequence<'a, C: AsContextMut<Data = HostState>>(
    context: &mut C,
    exports: &Exports,
    interp: i32,
    values: impl Iterator<Item = &'a tcl_engine_api::Value>,
    dictionary: bool,
) -> Result<i32, Failure> {
    let mut children = Vec::new();
    for child in values {
        match value(context, exports, interp, child) {
            Ok(child) => children.push(child),
            Err(error) => {
                for child in children {
                    call(context, &exports.release, child)?;
                }
                return Err(error);
            }
        }
    }
    let count = i32::try_from(children.len())
        .map_err(|_| Failure::Refused("too many original children".into()))?;
    let bytes = children
        .iter()
        .flat_map(|child| child.to_le_bytes())
        .collect::<Vec<_>>();
    let (input, _) = buffer(context, exports, &bytes)?;
    let original = call(
        context,
        &exports.new_sequence,
        (interp, input, count, i32::from(dictionary)),
    )?;
    call(context, &exports.free, input)?;
    for child in children {
        call(context, &exports.release, child)?;
    }
    Ok(original)
}

/// Read a successfully published guest completion and release its real references.
pub(crate) fn completion<C: AsContextMut<Data = HostState>>(
    context: &mut C,
    exports: &Exports,
    interp: i32,
    out: i32,
) -> Result<crate::session::Completion, Failure> {
    settle(context, exports, interp)?;
    let mut raw = [0; 12];
    exports
        .memory
        .read(context.as_context(), address(out), &mut raw)
        .map_err(|error| Failure::Trap(wasmtime::Error::new(error)))?;
    let field = |at| i32::from_le_bytes(raw[at..at + 4].try_into().expect("four completion bytes"));
    let (code, result, options) = (field(0), field(4), field(8));
    let answer = (|| {
        let result = string_bytes(context, exports, interp, result)?;
        let options = string_bytes(context, exports, interp, options)?;
        let error_code = if code == 1 {
            let original = call(context, &exports.error_code, interp)?;
            settle(context, exports, interp)?;
            let bytes = string_bytes(context, exports, interp, original);
            call(context, &exports.release, original)?;
            Some(bytes?)
        } else {
            None
        };
        Ok(crate::session::Completion {
            code,
            result,
            options,
            error_code,
        })
    })();
    call(context, &exports.completion_release, out)?;
    answer
}

pub(crate) fn guest_error<C: AsContextMut<Data = HostState>>(
    context: &mut C,
    exports: &Exports,
    interp: i32,
    code: i32,
) -> Result<EngineError, Failure> {
    settle(context, exports, interp)?;
    let out = call(context, &exports.alloc, (12, 4))?;
    let status = call(context, &exports.capture, (interp, code, out))?;
    let answer = settle(context, exports, interp).and_then(|()| {
        if status != 0 {
            return Err(Failure::ExecutionRefusal(
                "guest completion capture is unavailable".into(),
            ));
        }
        completion(context, exports, interp, out)
    });
    call(context, &exports.free, out)?;
    let original = answer?;
    Ok(EngineError::ScriptBytes {
        message: original.result,
        code: original.error_code,
        options: Some(original.options),
    })
}
