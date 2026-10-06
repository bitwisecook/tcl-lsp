//! Original-object array-search conversion and cursor lookup shared by backends.

use crate::error::{CmdError, CmdErrorCodeUpdate, CmdErrorDetails};
use tcl_core_types::{NativeArraySearchCache, NativeArraySearchChain};
use tcl_runtime_api::{ArrayTarget, Frames, VarStore};
use tcl_syntax::{
    native_array_search::{
        NativeArraySearchFailure as Failure, NativeArraySearchProtocol as Protocol,
    },
    value::{ValueError, ValueOps},
};

/// The original search-handle object and the observations reached on that same object.
pub struct NativeArraySearchOperand<'a, V> {
    pub original: &'a V,
    pub bytes: &'a [u8],
    pub cache: Option<NativeArraySearchCache>,
}

/// Backend storage on the original array; no persistent Var/root pin is added.
pub trait NativeArraySearchBackend:
    ValueOps + VarStore<Value = <Self as ValueOps>::Value> + Frames
{
    /// Select actual native handlers with independently issued integer ABI facts.
    fn array_search_protocol(&self) -> Option<Protocol>;
    /// Inspect original resident-only primary cache with origin validation.
    fn array_search_cache(
        &self,
        value: &<Self as ValueOps>::Value,
        protocol: Protocol,
    ) -> Result<Option<NativeArraySearchCache>, ValueError>;
    /// Publish a reached legacy conversion on the SAME original handle.
    fn install_array_search_cache(
        &self,
        value: &<Self as ValueOps>::Value,
        cache: NativeArraySearchCache,
        protocol: Protocol,
    ) -> Result<(), ValueError>;
    /// Run the operation against its original selected root and owning chain.
    fn array_search_on_original(
        &mut self,
        target: &ArrayTarget,
        sub: &str,
        name: &[u8],
        operand: Option<&NativeArraySearchOperand<'_, <Self as ValueOps>::Value>>,
        protocol: Protocol,
    ) -> Result<Result<<Self as ValueOps>::Value, Failure>, ValueError>;
}

/// Shared actual handler entry after arity validation and `LocateArray` callbacks.
pub fn dispatch<O: NativeArraySearchBackend>(
    ops: &mut O,
    sub: &str,
    rest: &[<O as ValueOps>::Value],
    target: Option<&ArrayTarget>,
) -> Option<Result<<O as ValueOps>::Value, CmdError>> {
    if !matches!(
        sub,
        "startsearch" | "anymore" | "nextelement" | "donesearch"
    ) {
        return None;
    }
    Some((|| {
        let protocol =
            ops.array_search_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "native array search",
                ))?;
        let expected = if sub == "startsearch" { 1 } else { 2 };
        if rest.len() != expected {
            let tail = if expected == 1 {
                "arrayName"
            } else {
                "arrayName searchId"
            };
            return Err(CmdError::wrong_args(&format!("array {sub} {tail}")));
        }
        let name = ops.native_string_bytes(&rest[0])?;
        let located;
        let target = if let Some(target) = target {
            target
        } else {
            located = ops.array_target_bytes(ops.current(), &name)?;
            &located
        };
        if ops.array_search_key_bytes_at(target)?.is_none() {
            let mut message = b"\"".to_vec();
            message.extend_from_slice(protocol.c_string(&name));
            message.extend_from_slice(b"\" isn't an array");
            return Err(error(protocol, message, b"ARRAY", protocol.c_string(&name)));
        }
        let handle = rest.get(1);
        let mut bytes = None;
        let mut cache = None;
        if let Some(handle) = handle {
            cache = ops.array_search_cache(handle, protocol)?;
            bytes = Some(ops.native_string_bytes(handle)?);
            if protocol.caches_handle() && cache.is_none() {
                match protocol.parse(bytes.as_deref().expect("handle bytes")) {
                    Ok(parsed) => {
                        ops.install_array_search_cache(handle, parsed, protocol)?;
                        cache = Some(parsed);
                    }
                    Err(failure) => {
                        return Err(failure_error(
                            protocol,
                            failure,
                            bytes.as_deref().expect("handle bytes"),
                            &name,
                        ));
                    }
                }
            }
        }
        let operand = handle.map(|original| NativeArraySearchOperand {
            original,
            bytes: bytes.as_deref().expect("original search handle bytes"),
            cache,
        });
        match ops.array_search_on_original(target, sub, &name, operand.as_ref(), protocol)? {
            Ok(value) => Ok(value),
            Err(failure) => Err(failure_error(
                protocol,
                failure,
                bytes.as_deref().unwrap_or_default(),
                &name,
            )),
        }
    })())
}

/// Match against original retained handles first in modern Tcl. Parsed legacy
/// facts never attest that the original array still owns an active search.
pub fn resolve<V, H>(
    chain: &NativeArraySearchChain<V>,
    protocol: Protocol,
    name: &[u8],
    operand: &NativeArraySearchOperand<'_, H>,
    mut same: impl FnMut(&V, &H) -> bool,
    mut string: impl FnMut(&V) -> Result<Vec<u8>, ValueError>,
) -> Result<Result<i32, Failure>, ValueError> {
    let NativeArraySearchOperand {
        original,
        bytes,
        cache,
    } = operand;
    if !protocol.caches_handle() {
        if let Some(id) = chain.find_handle::<ValueError>(|held| Ok(same(held, original)))? {
            return Ok(Ok(id));
        }
        if let Some(id) = chain.find_handle::<ValueError>(|held| {
            Ok(protocol.c_string(&string(held)?) == protocol.c_string(bytes))
        })? {
            return Ok(Ok(id));
        }
    }
    let parsed = if let Some(cache) = cache {
        *cache
    } else {
        match protocol.parse(bytes) {
            Ok(cache) => cache,
            Err(failure) => return Ok(Err(failure)),
        }
    };
    if !protocol.is_for_variable(bytes, parsed, name) {
        return Ok(Err(Failure::WrongVariable));
    }
    if protocol.caches_handle() && chain.contains(parsed.id) {
        Ok(Ok(parsed.id))
    } else {
        Ok(Err(Failure::MissingSearch))
    }
}

fn failure_error(protocol: Protocol, failure: Failure, handle: &[u8], name: &[u8]) -> CmdError {
    error(
        protocol,
        protocol.failure_message(failure, handle, name),
        b"ARRAYSEARCH",
        protocol.c_string(handle),
    )
}

fn error(protocol: Protocol, message: Vec<u8>, category: &[u8], name: &[u8]) -> CmdError {
    let error_code = if protocol.sets_lookup_error_code() {
        let policy = tcl_syntax::list_result::NativeListResultSerialization::for_string_protocol(
            tcl_syntax::native_string::NativeStringProtocol::C(protocol.version()),
        );
        CmdErrorCodeUpdate::Set(policy.render(&[b"TCL".as_slice(), b"LOOKUP", category, name]))
    } else {
        CmdErrorCodeUpdate::Unchanged
    };
    CmdError::from_byte_details(CmdErrorDetails {
        string_result: None,
        message,
        error_code,
        error_info: None,
        error_line: None,
        primitive_getter: None,
    })
}
