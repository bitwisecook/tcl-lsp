// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original-object append operations, shared by the concrete object backends.

use std::rc::Rc;

use tcl_syntax::native_object::{NativeObjectCacheSnapshot as Cache, NativeObjectSnapshot};
use tcl_syntax::native_object_append::{
    NativeObjectAppendAction as Action, NativeObjectAppendProtocol,
};
use tcl_syntax::native_string::{NativeStringProtocol, NativeStringStorageIdentity as Storage};
use tcl_syntax::native_tcl_utf::NativeTclUtf;
use tcl_syntax::raw_string::RawString;
use tcl_syntax::value::ValueError;

/// Concrete physical operations. Recipes supply behavior, never engine authority.
pub trait NativeAppendObjects {
    /// A retained physical object handle.
    type Value: Clone;
    /// Inspect without generating a string or replacing the primary cache.
    fn snapshot(&self, value: &Self::Value) -> Result<NativeObjectSnapshot, ValueError>;
    /// Apply COW using the original receiver's sharing, before retaining a working handle.
    fn prepare_receiver(&self, value: &Self::Value, protocol: NativeStringProtocol) -> Self::Value;
    /// Duplicate source representations into the prepared receiver, preserving its identity.
    fn duplicate_into(&self, receiver: &Self::Value, source: &Self::Value);
    /// Reach the original object's selected string updater.
    fn string(
        &self,
        value: &Self::Value,
        protocol: NativeStringProtocol,
    ) -> Result<Rc<[u8]>, ValueError>;
    /// Reach C Unicode storage on the original object.
    fn unicode(
        &self,
        value: &Self::Value,
        protocol: NativeStringProtocol,
    ) -> Result<Rc<[u32]>, ValueError>;
    /// Replace the primary String cache, retaining supplied original resident bytes and storage.
    fn set_string(
        &self,
        value: &Self::Value,
        protocol: NativeStringProtocol,
        resident: Option<(Rc<[u8]>, Storage)>,
        count: Option<usize>,
        unicode: Option<Rc<[u32]>>,
    ) -> Result<(), ValueError>;
    /// Replace the receiver with proper binary backing and no string representation.
    fn set_binary(
        &self,
        value: &Self::Value,
        protocol: NativeStringProtocol,
        bytes: Rc<[u8]>,
    ) -> Result<(), ValueError>;
    /// Create a native string constructor result.
    fn new_string(&self, bytes: Rc<[u8]>) -> Self::Value;
}

fn resident(snapshot: &NativeObjectSnapshot) -> Option<(Rc<[u8]>, Storage)> {
    snapshot
        .resident
        .clone()
        .map(|bytes| (bytes, snapshot.storage.unwrap_or(Storage::Unknown)))
}

/// Working receiver returned by the physical append owner. Its recipe and
/// continuation permission cannot be supplied independently by a consumer.
pub struct PreparedAppendValue<V> {
    value: V,
    protocol: NativeObjectAppendProtocol,
}

impl<V> PreparedAppendValue<V> {
    /// Inspect the working handle without acquiring another ownership reference.
    #[must_use]
    pub const fn value(&self) -> &V {
        &self.value
    }
    /// Transfer the working receiver to the selected publication boundary.
    #[must_use]
    pub fn into_value(self) -> V {
        self.value
    }
}

/// Start an original-object append and retain its prepared ownership receipt.
///
/// # Errors
/// Preserves the reached physical operation's typed storage refusal.
pub fn append_object<O: NativeAppendObjects>(
    ops: &O,
    protocol: NativeObjectAppendProtocol,
    receiver: Option<&O::Value>,
    source: &O::Value,
) -> Result<PreparedAppendValue<O::Value>, ValueError> {
    append_object_inner(ops, protocol, receiver, source, true, false)
        .map(|value| PreparedAppendValue { value, protocol })
}

/// Append C Unicode units to a privately owned result receiver. This is the
/// native Unicode append purpose: existing Unicode receives units directly;
/// a byte-backed String receives the selected native encoding of those units.
pub fn append_unicode_units<O: NativeAppendObjects>(
    ops: &O,
    protocol: NativeObjectAppendProtocol,
    receiver: &O::Value,
    units: &[u32],
) -> Result<(), ValueError> {
    let strings = protocol.string_protocol();
    if strings.tcl_version().is_none() {
        return Err(ValueError::CommandProtocolUnavailable("C Unicode append"));
    }
    if units.is_empty() {
        return Ok(());
    }
    let shape = ops.snapshot(receiver)?;
    if let Cache::String {
        protocol: origin,
        unicode: None,
        ..
    } = shape.cache
    {
        if origin != strings {
            return Err(ValueError::CommandProtocolUnavailable(
                "native Unicode append cache origin",
            ));
        }
        let mut bytes = ops.string(receiver, strings)?.to_vec();
        bytes.extend_from_slice(
            &NativeTclUtf::for_version(strings.tcl_version().expect("C append"))
                .encode_units(units)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "native Unicode append unit width",
                ))?,
        );
        return ops.set_string(
            receiver,
            strings,
            Some((Rc::from(bytes), Storage::Allocated)),
            None,
            None,
        );
    }
    let mut unicode = ops.unicode(receiver, strings)?.to_vec();
    unicode.extend_from_slice(units);
    let length = unicode.len();
    ops.set_string(
        receiver,
        strings,
        None,
        Some(length),
        Some(Rc::from(unicode)),
    )
}

/// Append counted bytes through C's `Tcl_AppendToObj` String/Unicode path.
/// This purpose does not apply `Tcl_AppendObjToObj` binary or empty-object shortcuts.
/// The receiver is borrowed until its actual copy-on-write decision is made.
///
/// # Errors
/// Refuses a non-C issuer or an unavailable original receiver string updater.
pub fn append_counted_bytes<O: NativeAppendObjects>(
    ops: &O,
    protocol: NativeObjectAppendProtocol,
    receiver: &O::Value,
    bytes: &[u8],
) -> Result<O::Value, ValueError> {
    if protocol.string_protocol().tcl_version().is_none() {
        return Err(ValueError::CommandProtocolUnavailable(
            "C counted String append",
        ));
    }
    if bytes.is_empty() {
        return Ok(receiver.clone());
    }
    let source = ops.new_string(Rc::from(bytes));
    append_object_inner(ops, protocol, Some(receiver), &source, true, true)
}

/// Continue the same Jim append batch without repeating its original COW decision.
/// No variable publication, callback, or additional value owner may intervene.
///
/// # Errors
/// Refuses a C publication receipt or an unavailable reached native storage operation.
pub fn append_continuation<O: NativeAppendObjects>(
    ops: &O,
    receiver: &mut PreparedAppendValue<O::Value>,
    source: &O::Value,
) -> Result<(), ValueError> {
    if !receiver.protocol.string_protocol().is_jim084() {
        return Err(ValueError::CommandProtocolUnavailable(
            "native append continuation ownership",
        ));
    }
    receiver.value = append_object_inner(
        ops,
        receiver.protocol,
        Some(&receiver.value),
        source,
        false,
        false,
    )?;
    Ok(())
}

/// Closed member-append batch. Its continuation permission is distinct from
/// a variable append, which publishes and reselects its receiver per operand.
pub struct PreparedDictionaryMemberAppend<V>(PreparedAppendValue<V>);
impl<V> PreparedDictionaryMemberAppend<V> {
    /// Transfer the prepared member to its dictionary publication owner.
    #[must_use]
    pub fn into_value(self) -> V {
        self.0.into_value()
    }
}

/// Prepare one member append with the selected original receiver and source.
///
/// # Errors
/// Preserves reached native storage and materialisation failures.
pub fn append_dictionary_member<O: NativeAppendObjects>(
    ops: &O,
    protocol: NativeObjectAppendProtocol,
    receiver: Option<&O::Value>,
    source: &O::Value,
) -> Result<PreparedDictionaryMemberAppend<O::Value>, ValueError> {
    append_object(ops, protocol, receiver, source).map(PreparedDictionaryMemberAppend)
}

/// Append an already prepared dictionary argument batch into one physical member.
/// The command policy supplies its missing receiver and performs any C9 Cat
/// preprocessing before this door. No member publication intervenes.
///
/// # Errors
/// Preserves reached storage failures and rejects unprepared C9 multi-input use.
pub fn append_dictionary_operands<O: NativeAppendObjects>(
    ops: &O,
    protocol: NativeObjectAppendProtocol,
    receiver: Option<&O::Value>,
    sources: &[O::Value],
) -> Result<O::Value, ValueError> {
    let Some((first, rest)) = sources.split_first() else {
        return Ok(receiver.map_or_else(
            || ops.new_string(std::rc::Rc::from(&b""[..])),
            |value| ops.prepare_receiver(value, protocol.string_protocol()),
        ));
    };
    let mut prepared = append_dictionary_member(ops, protocol, receiver, first)?;
    for source in rest {
        append_dictionary_member_continuation(ops, &mut prepared, source)?;
    }
    Ok(prepared.into_value())
}

/// Continue a pinned Jim or C Tcl 8 member batch without repeating COW.
/// No dictionary publication or callback may intervene. C9 multiple source
/// operands require its independent concatenation operation before this door.
///
/// # Errors
/// Refuses a C9 concatenation purpose or unavailable native storage.
pub fn append_dictionary_member_continuation<O: NativeAppendObjects>(
    ops: &O,
    receiver: &mut PreparedDictionaryMemberAppend<O::Value>,
    source: &O::Value,
) -> Result<(), ValueError> {
    let protocol = receiver.0.protocol.string_protocol();
    if protocol
        .tcl_version()
        .is_some_and(|version| version >= tcl_dialect::TclVersion::V9_0)
    {
        return Err(ValueError::CommandProtocolUnavailable(
            "native dictionary concatenation purpose",
        ));
    }
    receiver.0.value = append_object_inner(
        ops,
        receiver.0.protocol,
        Some(&receiver.0.value),
        source,
        false,
        false,
    )?;
    Ok(())
}

/// Append one operand while preserving native source cache effects and receiver identity.
/// Variable lookup, publication, and write observers belong to the retained-cell owner.
///
/// # Errors
/// Refuses unauthenticated storage or unavailable native updater capabilities.
fn append_object_inner<O: NativeAppendObjects>(
    ops: &O,
    protocol: NativeObjectAppendProtocol,
    receiver: Option<&O::Value>,
    source: &O::Value,
    prepare_receiver: bool,
    counted_bytes: bool,
) -> Result<O::Value, ValueError> {
    let source_shape = ops.snapshot(source)?;
    let receiver_shape = receiver.map(|value| ops.snapshot(value)).transpose()?;
    let action = if counted_bytes {
        Action::AppendString
    } else {
        protocol
            .action(receiver_shape.as_ref(), &source_shape)
            .map_err(|_| ValueError::CommandProtocolUnavailable("native append object shape"))?
    };
    let string_protocol = protocol.string_protocol();
    let Some(original_receiver) = receiver else {
        return match action {
            Action::AdoptSource => Ok(source.clone()),
            Action::NewString => {
                let bytes = ops.string(source, string_protocol)?;
                let value = ops.new_string(Rc::clone(&bytes));
                ops.set_string(
                    &value,
                    string_protocol,
                    Some((bytes, Storage::Allocated)),
                    None,
                    None,
                )?;
                Ok(value)
            }
            _ => unreachable!("missing receiver action"),
        };
    };
    let receiver = if prepare_receiver {
        ops.prepare_receiver(original_receiver, string_protocol)
    } else {
        original_receiver.clone()
    };
    match action {
        Action::PreserveReceiver => return Ok(receiver),
        Action::DuplicateSource => {
            ops.duplicate_into(&receiver, source);
            return Ok(receiver);
        }
        Action::AppendBinary => {
            let mut bytes = match &receiver_shape.as_ref().expect("receiver shape").cache {
                Cache::ByteArray { bytes, .. } => bytes.to_vec(),
                _ => Vec::new(),
            };
            let Cache::ByteArray { bytes: source, .. } = &source_shape.cache else {
                unreachable!("binary source action")
            };
            bytes.extend_from_slice(source);
            ops.set_binary(&receiver, string_protocol, Rc::from(bytes))?;
            return Ok(receiver);
        }
        Action::AppendString => {}
        Action::AdoptSource | Action::NewString => unreachable!("existing receiver action"),
    }
    let receiver_shape = receiver_shape.as_ref().expect("receiver shape");
    if string_protocol.is_jim084() {
        append_jim_string(ops, string_protocol, &receiver, source, receiver_shape)?;
    } else {
        append_c_string(
            ops,
            protocol,
            &receiver,
            source,
            receiver_shape,
            &source_shape,
        )?;
    }
    Ok(receiver)
}

fn append_jim_string<O: NativeAppendObjects>(
    ops: &O,
    string_protocol: NativeStringProtocol,
    receiver: &O::Value,
    source: &O::Value,
    receiver_shape: &NativeObjectSnapshot,
) -> Result<(), ValueError> {
    // Jim obtains the append operand before converting the receiver.
    let source_bytes = ops.string(source, string_protocol)?;
    let receiver_bytes = ops.string(receiver, string_protocol)?;
    let old_count = match &receiver_shape.cache {
        Cache::JimString { num_chars } => *num_chars,
        _ => None,
    };
    let count = old_count.map(|count| {
        count
            + RawString::from_bytes(Rc::clone(&source_bytes))
                .jim084_characters()
                .count()
    });
    let mut bytes = receiver_bytes.to_vec();
    bytes.extend_from_slice(&source_bytes);
    ops.set_string(
        receiver,
        string_protocol,
        Some((Rc::from(bytes), Storage::Allocated)),
        count,
        None,
    )?;
    Ok(())
}

fn append_c_unicode_source<O: NativeAppendObjects>(
    ops: &O,
    string_protocol: NativeStringProtocol,
    receiver: &O::Value,
    source: &O::Value,
    source_shape: &NativeObjectSnapshot,
    source_bytes: Option<Rc<[u8]>>,
    old_units: &[u32],
) -> Result<(), ValueError> {
    let codec =
        NativeTclUtf::for_version(string_protocol.tcl_version().expect("C append protocol"));
    let source_units = if matches!(source_shape.cache, Cache::String { .. }) {
        ops.unicode(source, string_protocol)?
    } else {
        let bytes = match source_bytes {
            Some(bytes) => bytes,
            None => ops.string(source, string_protocol)?,
        };
        Rc::from(codec.decode_units(&bytes))
    };
    if !source_units.is_empty() {
        let mut units = old_units.to_vec();
        units.extend_from_slice(&source_units);
        let count = units.len();
        ops.set_string(
            receiver,
            string_protocol,
            None,
            Some(count),
            Some(Rc::from(units)),
        )?;
    }
    Ok(())
}

fn append_c_string<O: NativeAppendObjects>(
    ops: &O,
    protocol: NativeObjectAppendProtocol,
    receiver: &O::Value,
    source: &O::Value,
    original_receiver_shape: &NativeObjectSnapshot,
    source_shape: &NativeObjectSnapshot,
) -> Result<(), ValueError> {
    let string_protocol = protocol.string_protocol();
    let version = string_protocol.tcl_version().expect("C append protocol");
    // SetStringFromAny converts the receiver before obtaining the append operand.
    let receiver_bytes = if let Cache::String {
        protocol: origin, ..
    } = &original_receiver_shape.cache
    {
        if *origin != string_protocol {
            return Err(ValueError::CommandProtocolUnavailable(
                "native append String cache origin",
            ));
        }
        None
    } else {
        let bytes = ops.string(receiver, string_protocol)?;
        let shape = ops.snapshot(receiver)?;
        ops.set_string(receiver, string_protocol, resident(&shape), None, None)?;
        Some(bytes)
    };
    let mut receiver_shape = ops.snapshot(receiver)?;
    // C9 reaches GetString(source) before its continuation-byte boundary test.
    let source_bytes = if version >= tcl_dialect::TclVersion::V9_0 {
        let bytes = ops.string(source, string_protocol)?;
        if protocol.forces_unicode_for_source(&bytes) {
            ops.unicode(receiver, string_protocol)?;
            receiver_shape = ops.snapshot(receiver)?;
        }
        Some(bytes)
    } else {
        None
    };
    if let Cache::String {
        unicode: Some(old_units),
        ..
    } = &receiver_shape.cache
    {
        append_c_unicode_source(
            ops,
            string_protocol,
            receiver,
            source,
            source_shape,
            source_bytes,
            old_units,
        )?;
        return Ok(());
    }
    append_c_byte_source(
        ops,
        protocol,
        receiver,
        source,
        &receiver_shape,
        receiver_bytes,
        source_bytes,
    )
}

fn append_c_byte_source<O: NativeAppendObjects>(
    ops: &O,
    protocol: NativeObjectAppendProtocol,
    receiver: &O::Value,
    source: &O::Value,
    receiver_shape: &NativeObjectSnapshot,
    receiver_bytes: Option<Rc<[u8]>>,
    source_bytes: Option<Rc<[u8]>>,
) -> Result<(), ValueError> {
    let string_protocol = protocol.string_protocol();
    let source_bytes = match source_bytes {
        Some(bytes) => bytes,
        None => ops.string(source, string_protocol)?,
    };
    if source_bytes.is_empty() {
        return Ok(());
    }
    let receiver_bytes = match receiver_bytes {
        Some(bytes) => bytes,
        None => ops.string(receiver, string_protocol)?,
    };
    let source_shape = ops.snapshot(source)?;
    let count = match (&receiver_shape.cache, &source_shape.cache) {
        (
            Cache::String {
                num_chars: Some(left),
                ..
            },
            Cache::String {
                protocol: origin,
                num_chars: Some(right),
                ..
            },
        ) if *origin == string_protocol
            && protocol.combines_character_counts(*right, source_bytes.len()) =>
        {
            Some(left + right)
        }
        _ => None,
    };
    let mut bytes = receiver_bytes.to_vec();
    bytes.extend_from_slice(&source_bytes);
    ops.set_string(
        receiver,
        string_protocol,
        Some((Rc::from(bytes), Storage::Allocated)),
        count,
        None,
    )?;
    Ok(())
}

/// Retained variable publication. Physical receiver borrowing belongs to the backend.
pub trait NativeAppendVariable {
    /// Command result after publication and reached observers.
    type Value;
    /// Backend completion or typed host refusal.
    type Error;
    /// Append one operand into the retained receiver, without read observers or publication.
    fn append_operand(&mut self, source: &Self::Value) -> Result<(), Self::Error>;
    /// Publish pending contents and run the selected write observers.
    fn publish(&mut self) -> Result<Self::Value, Self::Error>;
}

/// Apply C per-operand publication or Jim's single publication after all operands.
/// Inputs are nonempty; the command's no-operand read remains an explicit separate operation.
///
/// # Errors
/// Stops at the first conversion, store, or reached write-observer failure.
pub fn append_operands<O: NativeAppendVariable>(
    ops: &mut O,
    protocol: NativeObjectAppendProtocol,
    sources: &[O::Value],
) -> Result<O::Value, O::Error> {
    assert!(
        !sources.is_empty(),
        "native append operands require a source"
    );
    if protocol.string_protocol().is_jim084() {
        for source in sources {
            ops.append_operand(source)?;
        }
        return ops.publish();
    }
    let mut result = None;
    for source in sources {
        // Result ownership from the preceding publication must not alter the
        // original receiver's sharing at the next native COW decision.
        drop(result.take());
        ops.append_operand(source)?;
        result = Some(ops.publish()?);
    }
    Ok(result.expect("nonempty native append operands"))
}
