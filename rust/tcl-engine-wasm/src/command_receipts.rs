// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Counted transport of receipts issued by the runtime's installed-command owner.
//! A replay address is data; only its current instance's receipt grants access.

use tcl_engine_api::EngineError;
use wasmtime::{AsContextMut, TypedFunc, WasmParams, WasmResults};

use crate::session::{Exports, Failure, HostState, address, read_string};

#[derive(Clone, Debug)]
pub(crate) struct CommandReceipt {
    owner: u64,
    interpreter: u64,
    pub(crate) generation: u64,
    pub(crate) qualified: Vec<u8>,
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

fn owned_string<C: AsContextMut<Data = HostState>>(
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

/// The runtime retains the full typed first cause. This text is presentation only.
pub(crate) fn host_error<C: AsContextMut<Data = HostState>>(
    context: &mut C,
    exports: &Exports,
    interp: i32,
) -> Result<Option<EngineError>, Failure> {
    if call(context, &exports.host_refusal_pending, interp)? == 0 {
        return Ok(None);
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
    let (input, length) = buffer(context, exports, name)?;
    let out = call(context, &exports.alloc, (32, 8))?;
    let status = call(
        context,
        &exports.create_command,
        (interp, input, length, procedure, client, out),
    )?;
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
    let (input, length) = buffer(context, exports, name)?;
    let out = call(context, &exports.alloc, (32, 8))?;
    let status = call(
        context,
        &exports.delete_command,
        (interp, input, length, out),
    )?;
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
