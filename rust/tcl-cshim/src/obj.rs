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

//! The shim's `Tcl_Obj`: a reference-counted, dual-representation value.
//!
//! An [`Obj`] carries an optional string representation and an internal
//! representation, exactly as C Tcl's does, so that an integer or a list
//! built by C code never takes a detour through text: `Tcl_NewIntObj(5)` is
//! `Rep::Int(5)` with no string until someone asks for one, and it crosses the
//! interface as [`Value::Int`]. The string rep is generated lazily and cached,
//! and the pointer `Tcl_GetString` hands out stays valid until the object is
//! mutated or freed — the same contract C Tcl gives.
//!
//! Which representation crosses the boundary is decided by `canonical`: when
//! a string rep exists that did *not* come from the internal rep (the object
//! was created from text and merely parsed), the text is authoritative and is
//! what the engine sees, so `0x10` stays `0x10`. When the string was rendered
//! from the rep, or there is no string at all, the typed rep crosses.
//!
//! Reference counts map onto Rust ownership through [`ObjRef`]: cloning one is
//! `Tcl_IncrRefCount`, dropping one is `Tcl_DecrRefCount`, and a count of zero
//! frees the object — including the C convention that a freshly created
//! object has count zero and is owned by whoever first takes a reference.

use std::cell::{Cell, RefCell};
use std::ffi::{c_char, c_void};
use std::ptr::NonNull;
use std::rc::{Rc, Weak};
use tcl_registry::native_stock_list::ObjectLengthDialect;

use tcl_engine_api::{EngineError, NativeStringStorageIdentity, Value};
use tcl_syntax::list::{self, ListError};
use tcl_syntax::number::{self, Number};
use tcl_syntax::scalar_getter::{
    NativeScalarCache, NativeScalarGetterError, NativeScalarGetterKind, NativeScalarGetterProtocol,
    NativeScalarGetterValue,
};

/// A Tcl error a value conversion raises: the message and the `-errorcode`,
/// worded as C Tcl words them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TclError {
    /// The error message.
    pub message: Vec<u8>,
    /// The `-errorcode` list, when C Tcl sets one.
    pub code: Option<Vec<u8>>,
    /// Retained actual primitive failure, independently of interpreter propagation.
    pub getter: Option<Box<NativeScalarGetterError>>,
    /// A physical object capability refusal, never a guest parse failure.
    pub host: Option<Box<EngineError>>,
}

impl TclError {
    /// An error with a code.
    #[must_use]
    pub fn with_code(message: impl AsRef<[u8]>, code: impl AsRef<[u8]>) -> Self {
        Self {
            message: message.as_ref().to_vec(),
            code: Some(code.as_ref().to_vec()),
            getter: None,
            host: None,
        }
    }

    pub(crate) fn host(error: EngineError) -> Self {
        Self {
            message: Vec::new(),
            code: None,
            getter: None,
            host: Some(Box::new(error)),
        }
    }

    /// Width extraction failure after the primitive wide conversion succeeds.
    /// This is not a new primitive parse/cache receipt.
    pub(crate) fn overflow() -> Self {
        Self::with_code(
            tcl_syntax::expr::errors::IOVERFLOW_MESSAGE,
            tcl_syntax::expr::errors::IOVERFLOW_CODE,
        )
    }

    fn list_bytes(error: ListError, original: &[u8]) -> Self {
        let kind = match error {
            ListError::UnmatchedBrace => "BRACE",
            ListError::UnmatchedQuote => "QUOTE",
            ListError::BraceFollowedByJunk | ListError::QuoteFollowedByJunk => "JUNK",
        };
        Self::with_code(
            error.full_message_bytes(original),
            format!("TCL VALUE LIST {kind}"),
        )
    }
}

/// A genuine native List header with one shared member table and live flag.
struct ListHeader(Rc<ListStorage>);
struct ListStorage {
    items: Vec<ObjRef>,
    canonical: Rc<Cell<bool>>,
    headers: Cell<usize>,
    authority: Option<tcl_engine_api::NativeListBacking>,
}
impl ListHeader {
    fn new(items: Vec<ObjRef>, canonical: bool) -> Self {
        Self(Rc::new(ListStorage {
            items,
            canonical: Rc::new(Cell::new(canonical)),
            headers: Cell::new(1),
            authority: None,
        }))
    }
    fn retained(items: Vec<ObjRef>, authority: tcl_engine_api::NativeListBacking) -> Self {
        Self(Rc::new(ListStorage {
            items,
            canonical: authority.canonical_state(),
            headers: Cell::new(1),
            authority: Some(authority),
        }))
    }
    fn is_shared(&self) -> bool {
        self.0.authority.as_ref().map_or(
            self.0.headers.get() > 1,
            tcl_engine_api::NativeListBacking::is_shared,
        )
    }
}
impl Clone for ListHeader {
    fn clone(&self) -> Self {
        self.0.headers.set(
            self.0
                .headers
                .get()
                .checked_add(1)
                .expect("native List headers exhausted"),
        );
        Self(Rc::clone(&self.0))
    }
}
impl Drop for ListHeader {
    fn drop(&mut self) {
        self.0.headers.set(
            self.0
                .headers
                .get()
                .checked_sub(1)
                .expect("native List header underflow"),
        );
    }
}
impl std::ops::Deref for ListHeader {
    type Target = Vec<ObjRef>;
    fn deref(&self) -> &Self::Target {
        &self.0.items
    }
}

/// The internal representation.
enum Rep {
    /// A pure string.
    None,
    /// Actual C9 String primary cache retaining its count and Unicode units.
    String(tcl_engine_api::NativeStringCache),
    /// Opaque actual engine command-node cache authority, with resident bytes.
    CommandName(tcl_engine_api::NativeCommandNameCache),
    /// Opaque actual engine namespace cache authority, with resident bytes.
    NamespaceName(tcl_engine_api::NativeNamespaceNameCache),
    Int(i64),
    Double(f64),
    List(ListHeader),
    /// The table entry `Tcl_GetIndexFromObj` resolved this value to, kept so
    /// `Tcl_WrongNumArgs` can print the full option name of an abbreviation.
    Index(tcl_core_types::NativeIndexCache),
    /// Binary payload, independent of any resident string.
    ByteArray(Box<[u8]>),
    /// Full native primitive cache, including non-wide magnitude and word Boolean.
    Scalar(NativeScalarCache),
    Dictionary {
        entries: Vec<(ObjRef, ObjRef)>,
        buckets: Option<usize>,
    },
}

fn import_cache(
    cache: &tcl_engine_api::NativeScalarCache,
) -> Result<NativeScalarCache, EngineError> {
    use tcl_engine_api::{NativeCVersion, NativeIntegerRadix as R, NativeScalarCache as A};
    use tcl_syntax::number::Radix;
    let number = match cache {
        A::Integer(value) => Number::Int(*value),
        A::Double(value) => Number::Double(*value),
        A::Nan { negative, payload } => Number::Nan {
            negative: *negative,
            payload: *payload,
        },
        A::BigInteger {
            negative,
            radix,
            digits,
        } => Number::Big {
            negative: *negative,
            radix: match radix {
                R::Binary => Radix::Bin,
                R::Octal => Radix::Oct,
                R::Decimal => Radix::Dec,
                R::Hexadecimal => Radix::Hex,
            },
            digits: digits.to_string(),
        },
        A::WordBoolean {
            value,
            origin: NativeCVersion::V9_0,
        } => return Ok(NativeScalarCache::WordBoolean(*value)),
        A::WordBoolean { .. } | A::JimCoercedInteger(_) | A::Tcl84Long(_) | A::Tcl84Wide(_) => {
            return Err(EngineError::ExecutionRefusal(
                "native scalar cache belongs to a different engine descriptor".into(),
            ));
        }
    };
    if let Number::Big { radix, digits, .. } = &number
        && num_bigint::BigInt::parse_bytes(digits.as_bytes(), *radix as u32).is_none()
    {
        return Err(EngineError::ExecutionRefusal(
            "invalid transported integer magnitude".into(),
        ));
    }
    Ok(NativeScalarCache::Number(number))
}
fn export_cache(cache: &NativeScalarCache) -> tcl_engine_api::NativeScalarCache {
    use tcl_engine_api::{NativeCVersion, NativeIntegerRadix as R, NativeScalarCache as A};
    use tcl_syntax::number::Radix;
    match cache {
        NativeScalarCache::Number(Number::Int(value)) => A::Integer(*value),
        NativeScalarCache::Number(Number::Double(value)) => A::Double(*value),
        NativeScalarCache::Number(Number::Nan { negative, payload }) => A::Nan {
            negative: *negative,
            payload: *payload,
        },
        NativeScalarCache::Number(Number::Big {
            negative,
            radix,
            digits,
        }) => A::BigInteger {
            negative: *negative,
            radix: match radix {
                Radix::Bin => R::Binary,
                Radix::Oct => R::Octal,
                Radix::Dec => R::Decimal,
                Radix::Hex => R::Hexadecimal,
            },
            digits: std::rc::Rc::from(digits.as_str()),
        },
        NativeScalarCache::WordBoolean(value) => A::WordBoolean {
            value: *value,
            origin: NativeCVersion::V9_0,
        },
        NativeScalarCache::JimCoercedInteger(value) => A::JimCoercedInteger(*value),
        NativeScalarCache::Tcl84Long(value) => A::Tcl84Long(*value),
    }
}

/// A Tcl value: `Tcl_Obj` on the C side of the header.
///
/// The first five fields are the layout `runtime/rust/include/tcl.h` declares
/// for `Tcl_Obj` (`refCount`, `bytes`, `length`, `typePtr`, `internalRep`), at
/// the offsets the header's reference-count macros and an extension's reads of
/// `objPtr->bytes` use; the shim's own state follows that prefix, which C never
/// sees. Those fields are cells because C writes `refCount` directly and the
/// shim keeps `bytes` and `length` in step with the string rep it owns.
#[repr(C)]
pub struct Obj {
    /// `refCount`: one unit per holder. `Tcl_IncrRefCount` and `Tcl_DecrRefCount`
    /// are macros over this field.
    refcount: Cell<isize>,
    /// `bytes`: the first byte of the string rep, which is NUL-terminated at
    /// `length`, or null while there is none.
    bytes: Cell<*mut c_char>,
    /// `length`: the bytes of the string rep, the terminator excluded.
    length: Cell<isize>,
    /// `typePtr`: always null. The shim keeps its internal representation to
    /// itself, so C sees an object with no type.
    type_ptr: Cell<*const c_void>,
    /// `internalRep`: room for the union's two pointers, unused.
    internal_rep: Cell<[usize; 2]>,
    lifetime_pins: Cell<usize>,
    retired: Cell<bool>,
    /// The string rep as NUL-terminated bytes, once generated.
    string: RefCell<Option<Box<[u8]>>>,
    rep: RefCell<Rep>,
    /// Whether `string` was rendered from `rep` (so the typed rep may cross the
    /// boundary) rather than `rep` being parsed from `string`.
    canonical: Cell<bool>,
    storage: Cell<NativeStringStorageIdentity>,
    original: Option<(
        Rc<dyn tcl_engine_api::OriginalObject>,
        Weak<crate::state::InterpState>,
    )>,
    refusal: RefCell<Option<EngineError>>,
    string_mutation: Cell<tcl_engine_api::ResidentStringMutation>,
    lifetime: Rc<()>,
}

/// Explicit Unicode construction for the shim's C9 object protocol.
/// Exact resident input uses `from_bytes` and never passes through this encoder.
fn encode_text(text: &str) -> Box<[u8]> {
    let mut bytes = Vec::with_capacity(text.len() + 1);
    let units =
        tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(tcl_dialect::TclVersion::V9_0);
    for character in text.chars() {
        units
            .encode_unit(u32::from(character), &mut bytes)
            .expect("C9 Unicode construction");
    }
    bytes.push(0);
    bytes.into_boxed_slice()
}

impl Obj {
    fn with_rep(rep: Rep, string: Option<Box<[u8]>>, canonical: bool) -> Self {
        let storage = if string.is_some() {
            NativeStringStorageIdentity::Allocated
        } else {
            NativeStringStorageIdentity::Unknown
        };
        let obj = Self {
            refcount: Cell::new(0),
            lifetime_pins: Cell::new(0),
            retired: Cell::new(false),
            bytes: Cell::new(std::ptr::null_mut()),
            length: Cell::new(0),
            type_ptr: Cell::new(std::ptr::null()),
            internal_rep: Cell::new([0; 2]),
            string: RefCell::new(None),
            rep: RefCell::new(rep),
            canonical: Cell::new(canonical),
            storage: Cell::new(storage),
            original: None,
            refusal: RefCell::new(None),
            string_mutation: Cell::new(tcl_engine_api::ResidentStringMutation::Preserve),
            lifetime: Rc::new(()),
        };
        obj.set_string(string);
        obj
    }

    /// Replace the string rep, keeping the `bytes` and `length` fields C reads
    /// in step with it: null and zero while there is none.
    fn set_string(&self, string: Option<Box<[u8]>>) {
        if let Some(bytes) = &string {
            self.bytes.set(bytes.as_ptr().cast_mut().cast::<c_char>());
            self.length
                .set(isize::try_from(bytes.len() - 1).unwrap_or(isize::MAX));
        } else {
            self.bytes.set(std::ptr::null_mut());
            self.length.set(0);
        }
        *self.string.borrow_mut() = string;
    }

    /// A string value from raw bytes (no NUL terminator in `bytes`).
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut owned = Vec::with_capacity(bytes.len() + 1);
        owned.extend_from_slice(bytes);
        owned.push(0);
        let result = Self::with_rep(Rep::None, Some(owned.into_boxed_slice()), false);
        if bytes.is_empty() {
            result
                .storage
                .set(NativeStringStorageIdentity::CanonicalEmpty);
        }
        result
    }

    /// A string value from text.
    #[must_use]
    pub fn from_text(text: &str) -> Self {
        if text.is_empty() {
            return Self::from_bytes(b"");
        }
        Self::with_rep(Rep::None, Some(encode_text(text)), false)
    }

    /// An integer, with no string rep until one is asked for.
    #[must_use]
    pub fn int(value: i64) -> Self {
        Self::with_rep(Rep::Int(value), None, true)
    }

    /// A double, likewise.
    #[must_use]
    pub fn double(value: f64) -> Self {
        Self::with_rep(Rep::Double(value), None, true)
    }

    /// Native C9 List construction; zero elements retain canonical NULL-type empty storage.
    #[must_use]
    pub fn list(items: Vec<ObjRef>) -> Self {
        if items.is_empty() {
            return Self::from_bytes(b"");
        }
        Self::with_rep(Rep::List(ListHeader::new(items, false)), None, true)
    }

    /// An already-selected Dictionary cache retaining exact member objects.
    pub(crate) fn dictionary_cache(entries: Vec<(ObjRef, ObjRef)>) -> Self {
        Self::with_rep(
            Rep::Dictionary {
                entries,
                buckets: None,
            },
            None,
            true,
        )
    }

    /// The reference count, as `Tcl_IsShared` reads it.
    #[must_use]
    pub fn refcount(&self) -> usize {
        usize::try_from(self.refcount.get()).unwrap_or(0)
    }

    /// Whether more than one reference holds this object.
    #[must_use]
    pub fn is_shared(&self) -> bool {
        self.refcount() > 1
            || self
                .original
                .as_ref()
                .is_some_and(|(original, _)| original.is_shared())
    }

    /// Render native list bytes without a Unicode round trip.
    fn render(&self) -> Box<[u8]> {
        let rep = self.rep.borrow();
        let mut bytes = match &*rep {
            Rep::Int(value) => value.to_string().into_bytes(),
            Rep::Double(value) => number::format_double(*value).into_bytes(),
            Rep::List(items) => {
                let elements: Vec<Vec<u8>> = items.iter().map(|item| item.get().bytes()).collect();
                tcl_syntax::list_result::NativeListResultSerialization::Tcl85Plus.render(&elements)
            }
            Rep::Dictionary { entries, .. } => {
                let elements: Vec<Vec<u8>> = entries
                    .iter()
                    .flat_map(|(key, value)| [key, value])
                    .map(|item| item.get().bytes())
                    .collect();
                tcl_syntax::list_result::NativeListResultSerialization::Tcl85Plus.render(&elements)
            }
            Rep::ByteArray(bytes) => {
                let protocol = tcl_syntax::native_string::NativeStringProtocol::for_tcl_version(
                    tcl_dialect::TclVersion::V9_0,
                );
                protocol
                    .materialize(tcl_syntax::native_string::NativeStringInput::PureByteArray(
                        bytes,
                    ))
                    .expect("the C shim byte-array protocol is modeled")
                    .into_owned()
            }
            Rep::None => Vec::new(),
            Rep::String(tcl_engine_api::NativeStringCache::C {
                unicode: Some(units),
                ..
            }) => {
                if let Some(bytes) = tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(
                    tcl_dialect::TclVersion::V9_0,
                )
                .encode_units(units)
                {
                    bytes
                } else {
                    self.refuse_original(EngineError::ExecutionRefusal(
                        "native String Unicode units are unavailable".into(),
                    ));
                    Vec::new()
                }
            }
            Rep::String(_) => {
                self.refuse_original(EngineError::ExecutionRefusal(
                    "native String updater storage is unavailable".into(),
                ));
                Vec::new()
            }
            Rep::CommandName(_) => {
                self.refuse_original(EngineError::ExecutionRefusal(
                    "native command-name descriptor has no string updater".into(),
                ));
                Vec::new()
            }
            Rep::NamespaceName(_) => {
                self.refuse_original(EngineError::ExecutionRefusal(
                    "native namespace-name descriptor has no C9 string updater".into(),
                ));
                Vec::new()
            }
            Rep::Index(cache) => {
                if let Ok(word) = cache.word() {
                    word.to_vec()
                } else {
                    self.refuse_original(EngineError::ExecutionRefusal(
                        "retained native Index table entry is unavailable".into(),
                    ));
                    Vec::new()
                }
            }
            Rep::Scalar(cache) => match cache {
                NativeScalarCache::Number(Number::Int(value))
                | NativeScalarCache::Tcl84Long(value)
                | NativeScalarCache::JimCoercedInteger(value) => value.to_string().into_bytes(),
                NativeScalarCache::Number(Number::Double(value)) => {
                    number::format_double(*value).into_bytes()
                }
                NativeScalarCache::Number(Number::Nan { .. }) => b"NaN".to_vec(),
                NativeScalarCache::Number(Number::Big {
                    negative,
                    radix,
                    digits,
                }) => {
                    let integer = num_bigint::BigInt::parse_bytes(digits.as_bytes(), *radix as u32)
                        .expect("validated full native integer cache");
                    (if *negative { -integer } else { integer })
                        .to_string()
                        .into_bytes()
                }
                NativeScalarCache::WordBoolean(_) => {
                    unreachable!("word-Boolean always has resident bytes")
                }
            },
        };
        bytes.push(0);
        bytes.into_boxed_slice()
    }

    fn refuse_original(&self, error: EngineError) {
        if let Some((_, state)) = &self.original
            && let Some(state) = state.upgrade()
        {
            state.refuse_host(error.clone());
        }
        *self.refusal.borrow_mut() = Some(error);
    }

    fn publish_original(&self) {
        if let Some((original, _)) = &self.original
            && self.refusal.borrow().is_none()
        {
            match self
                .representation_result()
                .and_then(|result| original.apply(&result, self.string_mutation.get()))
            {
                Ok(()) => self
                    .string_mutation
                    .set(tcl_engine_api::ResidentStringMutation::Preserve),
                Err(error) => self.refuse_original(error),
            }
        }
    }

    fn from_original_string_cache(
        original: &dyn tcl_engine_api::OriginalObject,
        cache: tcl_engine_api::NativeStringCache,
    ) -> Result<Self, EngineError> {
        if !matches!(
            &cache,
            tcl_engine_api::NativeStringCache::C {
                origin: tcl_engine_api::NativeCVersion::V9_0,
                ..
            }
        ) {
            return Err(EngineError::ExecutionRefusal(
                "foreign native String descriptor".into(),
            ));
        }
        if original.resident_string().is_none()
            && !matches!(
                &cache,
                tcl_engine_api::NativeStringCache::C {
                    unicode: Some(_),
                    ..
                }
            )
        {
            return Err(EngineError::ExecutionRefusal(
                "native String updater storage is unavailable".into(),
            ));
        }
        Ok(Self::with_rep(Rep::String(cache), None, false))
    }

    /// Bind a supported scalar/string/binary snapshot to its original object.
    /// Compound and opaque caches need their own live member capabilities.
    /// # Errors
    /// Refuses unsupported cache/storage before native code receives the object.
    pub(crate) fn from_original(
        original: Rc<dyn tcl_engine_api::OriginalObject>,
        state: &Rc<crate::state::InterpState>,
    ) -> Result<Self, EngineError> {
        if original.native_c_version() != Some(tcl_engine_api::NativeCVersion::V9_0) {
            return Err(EngineError::ExecutionRefusal(
                "original object issuer differs from the C9.0 shim ABI".into(),
            ));
        }
        let mut object = if let Some(cache) = original.command_name_cache() {
            if cache.origin() != tcl_engine_api::NativeCVersion::V9_0
                || cache.scope_identity() != original.scope_identity()
                || original.resident_string().is_none()
            {
                return Err(EngineError::ExecutionRefusal(
                    "foreign native command-name cache or missing resident storage".into(),
                ));
            }
            Self::with_rep(Rep::CommandName(cache), None, false)
        } else if let Some(cache) = original.namespace_name_cache() {
            if cache.origin() != tcl_engine_api::NativeCVersion::V9_0
                || cache.scope_identity() != original.scope_identity()
                || original.resident_string().is_none()
            {
                return Err(EngineError::ExecutionRefusal(
                    "foreign native namespace-name cache or missing resident storage".into(),
                ));
            }
            Self::with_rep(Rep::NamespaceName(cache), None, false)
        } else if let Some((cache, origin)) = original.index_cache() {
            if origin != tcl_engine_api::NativeCVersion::V9_0 {
                return Err(EngineError::ExecutionRefusal(
                    "foreign native Index descriptor".into(),
                ));
            }
            Self::with_rep(Rep::Index(cache), None, false)
        } else if let Some(cache) = original.string_cache() {
            Self::from_original_string_cache(original.as_ref(), cache)?
        } else if let Some((members, canonical)) = original.list_members()? {
            let items = members
                .into_iter()
                .map(|member| state.import_original(member))
                .collect::<Result<Vec<_>, _>>()?;
            {
                let authority = original.list_backing()?.ok_or_else(|| {
                    EngineError::ExecutionRefusal("original List backing unavailable".into())
                })?;
                if authority.origin() != tcl_engine_api::NativeCVersion::V9_0
                    || authority.scope_identity() != original.scope_identity()
                    || authority.member_count() != items.len()
                    || authority.canonical_state().get() != canonical
                    || !original.list_backing_matches(&authority)?
                {
                    return Err(EngineError::ExecutionRefusal(
                        "original List backing scope or shape mismatch".into(),
                    ));
                }
                Self::with_rep(
                    Rep::List(ListHeader::retained(items, authority)),
                    None,
                    true,
                )
            }
        } else if let Some((members, buckets)) = original.dictionary_members()? {
            let entries = members
                .into_iter()
                .map(|(key, value)| {
                    Ok((state.import_original(key)?, state.import_original(value)?))
                })
                .collect::<Result<Vec<_>, EngineError>>()?;
            Self::with_rep(Rep::Dictionary { entries, buckets }, None, true)
        } else {
            Self::from_value(&original.snapshot()?)?
        };
        if let Some((string, storage)) = original.resident_string() {
            if storage == NativeStringStorageIdentity::CanonicalEmpty && !string.is_empty() {
                return Err(EngineError::ExecutionRefusal(
                    "invalid canonical empty original storage".into(),
                ));
            }
            let mut bytes = string.to_vec();
            bytes.push(0);
            object.set_string(Some(bytes.into_boxed_slice()));
            object.storage.set(storage);
        }
        object.original = Some((original, Rc::downgrade(state)));
        Ok(object)
    }

    /// Refresh a retained mirror from the current authoritative original object.
    pub(crate) fn refresh_original(
        &self,
        state: &Rc<crate::state::InterpState>,
    ) -> Result<(), EngineError> {
        let Some((original, _)) = &self.original else {
            return Ok(());
        };
        let retained = {
            let rep = self.rep.borrow();
            if let Rep::List(items) = &*rep {
                items
                    .0
                    .authority
                    .as_ref()
                    .map(|authority| original.list_backing_matches(authority))
                    .transpose()?
            } else {
                None
            }
        };
        if retained == Some(true) {
            let resident = original.resident_string();
            let string = resident.as_ref().map(|(bytes, _)| {
                let mut bytes = bytes.to_vec();
                bytes.push(0);
                bytes.into_boxed_slice()
            });
            let storage =
                resident.map_or(NativeStringStorageIdentity::Unknown, |(_, storage)| storage);
            if self.string.borrow().as_deref() != string.as_deref() || self.storage.get() != storage
            {
                self.set_string(string);
            }
            self.storage.set(storage);
            self.canonical.set(true);
            return Ok(());
        }
        let mut donor = Self::from_original(Rc::clone(original), state)?;
        let string = donor.string.get_mut().take();
        // Repeated reads preserve the C pointer while resident storage is unchanged.
        if self.string.borrow().as_deref() != string.as_deref()
            || self.storage.get() != donor.storage.get()
        {
            self.set_string(string);
        }
        self.storage.set(donor.storage.get());
        *self.rep.borrow_mut() = std::mem::replace(donor.rep.get_mut(), Rep::None);
        self.canonical.set(donor.canonical.get());
        Ok(())
    }

    fn representation_result(&self) -> Result<tcl_engine_api::OriginalObjectResult, EngineError> {
        // This root can also be a stack-owned Obj; child ObjRefs retain all
        // heap objects while the graph traversal runs.
        OriginalObjectGraph::default().export(self, true)
    }

    /// Recover this original or one owned shared node in the callback graph.
    #[cfg(test)]
    pub(crate) fn original_result(
        &self,
    ) -> Result<tcl_engine_api::OriginalObjectResult, EngineError> {
        OriginalObjectGraph::default().export(self, false)
    }

    /// Ensure the string rep exists, then run `with` over its bytes (including
    /// the terminator).
    fn with_string<T>(&self, with: impl FnOnce(&[u8]) -> T) -> T {
        if self.string.borrow().is_none() {
            let rendered = self.render();
            self.set_string(Some(rendered));
            self.string_mutation
                .set(tcl_engine_api::ResidentStringMutation::Replace);
            self.storage.set(
                if self
                    .string
                    .borrow()
                    .as_ref()
                    .is_some_and(|bytes| bytes.len() == 1)
                {
                    NativeStringStorageIdentity::CanonicalEmpty
                } else {
                    NativeStringStorageIdentity::Allocated
                },
            );
            if let Rep::List(items) = &*self.rep.borrow() {
                let protocol = tcl_syntax::native_string::NativeStringProtocol::for_tcl_version(
                    tcl_dialect::TclVersion::V9_0,
                );
                items.0.canonical.set(
                    protocol.updated_list_canonical(items.0.canonical.get(), items.is_shared()),
                );
            }
            self.canonical.set(true);
            self.publish_original();
        }
        if self.storage.get() == NativeStringStorageIdentity::CanonicalEmpty {
            // This target-native singleton is distinct from allocated empty buffers.
            static EMPTY: [u8; 1] = [0];
            return with(&EMPTY);
        }
        let string = self.string.borrow();
        with(string.as_deref().unwrap_or(&[0]))
    }

    /// The string rep as a C pointer plus its length in bytes (terminator
    /// excluded). Valid until the object is mutated or freed.
    pub fn c_string(&self) -> (*const c_char, usize) {
        self.with_string(|bytes| (bytes.as_ptr().cast::<c_char>(), bytes.len() - 1))
    }

    /// Exact resident string bytes, materialising the actual typed payload.
    #[must_use]
    pub fn bytes(&self) -> Vec<u8> {
        self.with_string(|bytes| bytes[..bytes.len() - 1].to_vec())
    }

    /// Checked Unicode view; this never changes native bytes or identity.
    pub fn text(&self) -> Result<String, std::str::Utf8Error> {
        self.with_string(|bytes| std::str::from_utf8(&bytes[..bytes.len() - 1]).map(str::to_owned))
    }

    /// Forget the string rep after a mutation of the internal rep.
    fn invalidate_string(&self) {
        self.string_mutation
            .set(tcl_engine_api::ResidentStringMutation::Discard);
        self.set_string(None);
        self.storage.set(NativeStringStorageIdentity::Unknown);
        self.canonical.set(true);
    }

    /// Execute the actual C9 primitive getter on this original object.
    fn scalar_getter(
        &self,
        kind: NativeScalarGetterKind,
    ) -> Result<NativeScalarGetterValue, TclError> {
        let protocol = NativeScalarGetterProtocol::for_tcl_version(tcl_dialect::TclVersion::V9_0);
        let cache = match &*self.rep.borrow() {
            Rep::Int(value) => Some(NativeScalarCache::Number(Number::Int(*value))),
            Rep::Double(value) => Some(NativeScalarCache::Number(Number::Double(*value))),
            Rep::Scalar(cache) => Some(cache.clone()),
            _ => None,
        };
        let conversion = cache
            .as_ref()
            .map(|cache| protocol.cached_conversion(kind, cache, None))
            .transpose()
            .map_err(|_| {
                TclError::host(EngineError::Unsupported(
                    "native scalar target integer layout",
                ))
            })?
            .flatten()
            .unwrap_or_else(|| {
                protocol
                    .fresh_conversion(kind, &self.bytes())
                    .expect("C9 primitive grammar has no unavailable Jim range state")
            });
        let (materialize, cache, outcome) = conversion.into_parts();
        if materialize {
            let _ = self.bytes();
        }
        if let Some(cache) = cache {
            match cache {
                NativeScalarCache::Number(Number::Int(value)) => {
                    self.set_parsed_rep(Rep::Int(value));
                }
                NativeScalarCache::Number(Number::Double(value)) => {
                    self.set_parsed_rep(Rep::Double(value));
                }
                cache => self.set_parsed_rep(Rep::Scalar(cache)),
            }
        }
        if let Some(error) = self.refusal.borrow().clone() {
            return Err(TclError::host(error));
        }
        outcome.map_err(|failure| {
            let record = match protocol.failure_requires_original_string(kind, failure) {
                Some(true) => {
                    let original = self.bytes();
                    if let Some(error) = self.refusal.borrow().clone() {
                        return TclError::host(error);
                    }
                    protocol.failure_presentation(kind, failure, &original)
                }
                Some(false) => protocol.failure_presentation_without_original_string(kind, failure),
                None => None,
            };
            let Some(record) = record else {
                return TclError::host(EngineError::Unsupported(
                    "native scalar failure presentation",
                ));
            };
            let code = match record.error_code_update() {
                tcl_syntax::scalar_getter::NativeScalarGetterErrorCode::Unchanged => None,
                tcl_syntax::scalar_getter::NativeScalarGetterErrorCode::Set(bytes) => {
                    Some(bytes.clone())
                }
            };
            TclError {
                message: record.message_bytes().to_vec(),
                code,
                getter: Some(Box::new(record)),
                host: None,
            }
        })
    }

    /// The value as a native C integer, with its own cache and width protocol.
    ///
    /// # Errors
    /// Returns the selected primitive integer failure after its cache effects.
    pub fn get_int(&self) -> Result<i64, TclError> {
        match self.scalar_getter(NativeScalarGetterKind::Int)? {
            NativeScalarGetterValue::Wide(value) => Ok(value),
            _ => unreachable!("integer getter result"),
        }
    }

    /// The value as a wide integer — `Tcl_GetWideIntFromObj`.
    pub fn get_wide(&self) -> Result<i64, TclError> {
        match self.scalar_getter(NativeScalarGetterKind::Wide)? {
            NativeScalarGetterValue::Wide(value) => Ok(value),
            _ => unreachable!("selected Wide getter"),
        }
    }

    /// The value as a double — `Tcl_GetDoubleFromObj`, not expression arithmetic.
    pub fn get_double(&self) -> Result<f64, TclError> {
        match self.scalar_getter(NativeScalarGetterKind::Double)? {
            NativeScalarGetterValue::Double(value) => Ok(value),
            _ => unreachable!("selected Double getter"),
        }
    }

    /// Primitive Boolean extraction, independently of expression truthiness.
    pub fn get_boolean(&self) -> Result<bool, TclError> {
        match self.scalar_getter(NativeScalarGetterKind::Boolean)? {
            NativeScalarGetterValue::Boolean(value) => Ok(value.is_true()),
            _ => unreachable!("selected Boolean getter"),
        }
    }

    /// Install a rep parsed from the string rep: the string stays
    /// authoritative.
    fn set_parsed_rep(&self, rep: Rep) {
        *self.rep.borrow_mut() = rep;
        self.canonical.set(false);
        self.publish_original();
    }

    /// Ensure the list rep exists, parsing the string rep if needed.
    fn ensure_list(&self) -> Result<(), TclError> {
        if matches!(&*self.rep.borrow(), Rep::List(_)) {
            return Ok(());
        }
        if self.string.borrow().is_none() {
            let members = match &*self.rep.borrow() {
                Rep::Dictionary { entries, .. } => Some(
                    entries
                        .iter()
                        .flat_map(|(key, value)| [key.clone(), value.clone()])
                        .collect(),
                ),
                _ => None,
            };
            if let Some(members) = members {
                self.set_parsed_rep(Rep::List(ListHeader::new(members, false)));
                if let Some(error) = self.refusal.borrow().clone() {
                    return Err(TclError::host(error));
                }
                return Ok(());
            }
        }
        let bytes = self.bytes();
        let elements = list::split_list_bytes_in(
            &bytes,
            tcl_dialect::ListParse::Strict,
            tcl_dialect::EscapeSyntax::Tcl90,
        )
        .map_err(|error| TclError::list_bytes(error, &bytes))?;
        let items = elements
            .into_iter()
            .map(|element| ObjRef::new(Obj::from_bytes(&element)))
            .collect();
        self.set_parsed_rep(Rep::List(ListHeader::new(items, false)));
        if let Some((original, state)) = &self.original
            && let (Some(state), Some((members, _))) = (
                state.upgrade(),
                original.list_members().map_err(TclError::host)?,
            )
        {
            let members = members
                .into_iter()
                .map(|member| state.import_original(member))
                .collect::<Result<_, _>>()
                .map_err(TclError::host)?;
            let authority = original
                .list_backing()
                .map_err(TclError::host)?
                .ok_or_else(|| {
                    TclError::host(EngineError::ExecutionRefusal(
                        "converted original List backing unavailable".into(),
                    ))
                })?;
            *self.rep.borrow_mut() = Rep::List(ListHeader::retained(members, authority));
        }
        if let Some(error) = self.refusal.borrow().clone() {
            return Err(TclError::host(error));
        }
        Ok(())
    }

    /// Execute the selected C9 stock Length operation without unnecessary conversion.
    pub fn list_len(&self) -> Result<usize, TclError> {
        use tcl_registry::native_stock_list::{
            NativeObjectLengthAction as Action, NativeStockListInputClass as Class,
        };
        let protocol =
            tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0)
                .native_object_length_protocol()
                .expect("the shim implements actual C9");
        let class = match &*self.rep.borrow() {
            Rep::None | Rep::String(_) => Class::String,
            Rep::List(_) => Class::List,
            Rep::Dictionary { .. } => Class::Dictionary,
            Rep::ByteArray(_) => Class::ByteArray,
            Rep::Scalar(NativeScalarCache::WordBoolean(_)) => Class::Boolean,
            Rep::Int(_) | Rep::Double(_) | Rep::Scalar(_) => Class::Numeric,
            Rep::Index(_) => Class::Unknown,
            Rep::CommandName(_) => Class::CommandName,
            Rep::NamespaceName(_) => Class::NamespaceName,
        };
        let canonical = self.storage.get() == NativeStringStorageIdentity::CanonicalEmpty;
        let unknown_empty = self.storage.get() == NativeStringStorageIdentity::Unknown
            && self
                .string
                .borrow()
                .as_ref()
                .is_some_and(|bytes| bytes.len() == 1);
        if unknown_empty && protocol.action(class, true) != protocol.action(class, false) {
            let error = EngineError::ExecutionRefusal(
                "native list Length empty allocation identity is unavailable".into(),
            );
            self.refuse_original(error.clone());
            return Err(TclError::host(error));
        }
        match protocol.action(class, canonical) {
            Some(Action::Constant(length)) => Ok(length),
            Some(Action::CachedList | Action::ConvertToList) => self.with_list(<[ObjRef]>::len),
            None => {
                let error = EngineError::ExecutionRefusal(
                    "native object Length cache capability is unavailable".into(),
                );
                self.refuse_original(error.clone());
                Err(TclError::host(error))
            }
        }
    }

    /// Run `with` over the list elements — `Tcl_ListObjGetElements`.
    pub fn with_list<T>(&self, with: impl FnOnce(&[ObjRef]) -> T) -> Result<T, TclError> {
        self.ensure_list()?;
        let rep = self.rep.borrow();
        match &*rep {
            Rep::List(items) => Ok(with(items)),
            Rep::None
            | Rep::CommandName(_)
            | Rep::NamespaceName(_)
            | Rep::String(_)
            | Rep::Int(_)
            | Rep::Double(_)
            | Rep::Index(_)
            | Rep::ByteArray(_)
            | Rep::Scalar(_)
            | Rep::Dictionary { .. } => {
                unreachable!("ensure_list installed a list rep")
            }
        }
    }

    /// Append an element — `Tcl_ListObjAppendElement`. The caller has checked
    /// the object is unshared.
    pub fn append_element(&self, element: ObjRef) -> Result<(), TclError> {
        self.ensure_list()?;
        if let Rep::List(items) = &mut *self.rep.borrow_mut() {
            let protocol = tcl_syntax::native_string::NativeStringProtocol::for_tcl_version(
                tcl_dialect::TclVersion::V9_0,
            );
            let canonical = if items.is_shared() {
                protocol.copied_list_canonical(items.0.canonical.get())
            } else {
                items.0.canonical.get()
            };
            let mut members = items.0.items.clone();
            members.push(element);
            *items = ListHeader::new(members, canonical);
        }
        self.invalidate_string();
        self.publish_original();
        if let Some(error) = self.refusal.borrow().clone() {
            return Err(TclError::host(error));
        }
        Ok(())
    }

    /// Install an independently retained native table cache.
    pub fn set_index_cache(
        &self,
        cache: tcl_core_types::NativeIndexCache,
    ) -> Result<(), EngineError> {
        // Rendering first keeps the invariant that an index rep always sits
        // beside a string rep, which is what `render` relies on.
        self.with_string(|_| ());
        *self.rep.borrow_mut() = Rep::Index(cache);
        self.canonical.set(false);
        self.publish_original();
        self.refusal.borrow().clone().map_or(Ok(()), Err)
    }

    pub(crate) fn static_extension(&self) -> Option<Rc<crate::state::StaticExtensionLifetime>> {
        self.original
            .as_ref()
            .and_then(|(_, state)| state.upgrade())
            .and_then(|state| state.static_extension())
    }

    pub(crate) fn cached_index(&self, table: usize, stride: usize) -> Option<usize> {
        match &*self.rep.borrow() {
            Rep::Index(cache) if cache.table_identity() == table && cache.stride() == stride => {
                Some(cache.index())
            }
            _ => None,
        }
    }

    /// Read the canonical word from the original retained native table.
    #[must_use]
    pub fn index_entry(&self) -> Option<Vec<u8>> {
        match &*self.rep.borrow() {
            Rep::Index(cache) => {
                if let Ok(word) = cache.word() {
                    Some(word.to_vec())
                } else {
                    self.refuse_original(EngineError::ExecutionRefusal(
                        "retained native Index word is unavailable".into(),
                    ));
                    None
                }
            }
            Rep::None
            | Rep::CommandName(_)
            | Rep::NamespaceName(_)
            | Rep::String(_)
            | Rep::Int(_)
            | Rep::Double(_)
            | Rep::List(_)
            | Rep::ByteArray(_)
            | Rep::Scalar(_)
            | Rep::Dictionary { .. } => None,
        }
    }

    /// A fresh, unshared copy — `Tcl_DuplicateObj`.
    #[must_use]
    pub fn duplicate(&self) -> Self {
        let rep = match &*self.rep.borrow() {
            Rep::None => Rep::None,
            Rep::String(cache) => Rep::String(cache.clone()),
            Rep::Int(value) => Rep::Int(*value),
            Rep::Double(value) => Rep::Double(*value),
            Rep::List(items) => Rep::List(items.clone()),
            Rep::Index(entry) => Rep::Index(entry.clone()),
            Rep::CommandName(cache) => Rep::CommandName(cache.clone()),
            Rep::NamespaceName(cache) => Rep::NamespaceName(cache.clone()),
            Rep::ByteArray(bytes) => Rep::ByteArray(bytes.clone()),
            Rep::Scalar(cache) => Rep::Scalar(cache.clone()),
            Rep::Dictionary { entries, buckets } => Rep::Dictionary {
                entries: entries.clone(),
                buckets: *buckets,
            },
        };
        let mut result = Self::with_rep(rep, self.string.borrow().clone(), self.canonical.get());
        result.storage.set(self.storage.get());
        if let Some((original, state)) = &self.original {
            match original.duplicate_native_header() {
                Ok(duplicate) => result.original = Some((duplicate, Weak::clone(state))),
                Err(error) => {
                    result.refusal.replace(Some(error.clone()));
                    self.refuse_original(error);
                }
            }
        }
        result
    }

    /// Retain the typed cache together with any authoritative resident spelling.
    #[must_use]
    pub fn to_value(&self) -> Value {
        let typed_is_authoritative = self.string.borrow().is_none() || self.canonical.get();
        let payload = match &*self.rep.borrow() {
            Rep::ByteArray(bytes) => Some(Value::byte_array(bytes.to_vec())),
            Rep::Int(value) if typed_is_authoritative => Some(Value::Int(*value)),
            Rep::Double(value) if typed_is_authoritative => Some(Value::Double(*value)),
            Rep::Int(value) => Some(Value::NativeScalar(
                tcl_engine_api::NativeScalarCache::Integer(*value),
            )),
            Rep::Double(value) => Some(Value::NativeScalar(
                tcl_engine_api::NativeScalarCache::Double(*value),
            )),
            Rep::Scalar(cache) => Some(Value::NativeScalar(export_cache(cache))),
            Rep::List(items) => Some(Value::list(items.iter().map(|item| item.get().to_value()))),
            Rep::Dictionary { entries, .. } => {
                Some(Value::dict(entries.iter().map(|(key, value)| {
                    (key.get().to_value(), value.get().to_value())
                })))
            }
            _ => None,
        };
        if let Some(payload) = payload {
            return self
                .string
                .borrow()
                .as_ref()
                .map_or(payload.clone(), |bytes| {
                    payload.with_resident_string_storage(
                        bytes[..bytes.len() - 1].to_vec(),
                        self.storage.get(),
                    )
                });
        }
        Value::string_bytes(self.bytes())
    }

    /// An object from an interface value, keeping its structure.
    /// # Errors
    /// Refuses a foreign cache descriptor or unavailable physical storage identity.
    pub fn from_value(value: &Value) -> Result<Self, EngineError> {
        Self::from_value_with_storage(value, None)
    }

    fn from_value_with_storage(
        value: &Value,
        resident: Option<(&[u8], NativeStringStorageIdentity)>,
    ) -> Result<Self, EngineError> {
        let result = match value {
            Value::Empty => Self::from_text(""),
            Value::Str(text) => Self::from_bytes(text.as_bytes()),
            Value::StringBytes(bytes) => Self::from_bytes(bytes),
            Value::ByteArray(bytes) => Self::with_rep(
                Rep::ByteArray(bytes.to_vec().into_boxed_slice()),
                None,
                true,
            ),
            Value::Resident {
                value,
                string,
                storage,
            } => return Self::from_value_with_storage(value, Some((string, *storage))),
            Value::NativeScalar(cache) => {
                let cache = import_cache(cache)?;
                if matches!(cache, NativeScalarCache::WordBoolean(_)) && resident.is_none() {
                    return Err(EngineError::ExecutionRefusal(
                        "C word-Boolean cache requires its resident string".into(),
                    ));
                }
                Self::with_rep(Rep::Scalar(cache), None, true)
            }
            Value::Int(number) => Self::int(*number),
            Value::Double(number) => Self::double(*number),
            Value::List(items) => Self::list(
                items
                    .iter()
                    .map(|item| Self::from_value(item).map(ObjRef::new))
                    .collect::<Result<_, _>>()?,
            ),
            Value::Dict(entries) => Self::with_rep(
                Rep::Dictionary {
                    entries: entries
                        .iter()
                        .map(|(key, value)| {
                            Ok((
                                ObjRef::new(Self::from_value(key)?),
                                ObjRef::new(Self::from_value(value)?),
                            ))
                        })
                        .collect::<Result<_, EngineError>>()?,
                    buckets: None,
                },
                None,
                true,
            ),
        };
        if let Some((string, storage)) = resident {
            if storage == NativeStringStorageIdentity::CanonicalEmpty && !string.is_empty() {
                return Err(EngineError::ExecutionRefusal(
                    "canonical empty-string storage has nonempty bytes".into(),
                ));
            }
            let mut bytes = string.to_vec();
            bytes.push(0);
            result.set_string(Some(bytes.into_boxed_slice()));
            result.storage.set(storage);
            result.canonical.set(true);
        }
        Ok(result)
    }
}

/// An owning reference to an [`Obj`]: one unit of its reference count.
///
/// `#[repr(transparent)]` over the pointer so a `Vec<ObjRef>` is exactly the
/// `Tcl_Obj **` array `Tcl_ListObjGetElements` hands to C.
/// One callback's result/options graph, with original aliases retained.
#[derive(Default)]
pub(crate) struct OriginalObjectGraph {
    memo: std::collections::BTreeMap<usize, Rc<tcl_engine_api::OriginalObjectResult>>,
}

enum ExportShape {
    List(usize, bool),
    Dictionary(usize, Option<usize>),
}
enum ExportWork {
    Visit(ObjLifetimeLease),
    Build(
        usize,
        ExportShape,
        Option<(Rc<[u8]>, NativeStringStorageIdentity)>,
    ),
}

impl OriginalObjectGraph {
    fn visit(
        &mut self,
        object: &Obj,
        force_representation: bool,
        active: &mut std::collections::BTreeSet<usize>,
        work: &mut Vec<ExportWork>,
        values: &mut Vec<tcl_engine_api::OriginalObjectResult>,
    ) -> Result<(), EngineError> {
        use tcl_engine_api::OriginalObjectResult as R;
        let identity = std::ptr::from_ref(object) as usize;
        if let Some(error) = object.refusal.borrow().clone() {
            return Err(error);
        }
        if active.contains(&identity) {
            return Err(EngineError::ExecutionRefusal(
                "cyclic native callback object graph".into(),
            ));
        }
        if !force_representation && let Some((original, _)) = &object.original {
            values.push(R::Original(Rc::clone(original)));
            return Ok(());
        }
        if let Some(value) = self.memo.get(&identity) {
            values.push(R::Shared(Rc::clone(value)));
            return Ok(());
        }
        let resident = object
            .string
            .borrow()
            .as_ref()
            .map(|bytes| (Rc::from(&bytes[..bytes.len() - 1]), object.storage.get()));
        let rep = object.rep.borrow();
        match &*rep {
            Rep::List(items) => {
                if let Some(authority) = &items.0.authority {
                    let value = Rc::new(wrap_export_resident(
                        R::RetainedList(authority.clone()),
                        resident,
                    ));
                    self.memo.insert(identity, Rc::clone(&value));
                    values.push(R::Shared(value));
                    return Ok(());
                }
                active.insert(identity);
                work.push(ExportWork::Build(
                    identity,
                    ExportShape::List(items.len(), items.0.canonical.get()),
                    resident,
                ));
                work.extend(
                    items
                        .iter()
                        .rev()
                        .map(|item| ExportWork::Visit(ObjLifetimeLease::new(item))),
                );
                return Ok(());
            }
            Rep::Dictionary { entries, buckets } => {
                active.insert(identity);
                work.push(ExportWork::Build(
                    identity,
                    ExportShape::Dictionary(entries.len(), *buckets),
                    resident,
                ));
                for (key, value) in entries.iter().rev() {
                    work.push(ExportWork::Visit(ObjLifetimeLease::new(value)));
                    work.push(ExportWork::Visit(ObjLifetimeLease::new(key)));
                }
                return Ok(());
            }
            _ => {}
        }
        let value = match &*rep {
            Rep::Index(cache) => R::Index {
                cache: cache.clone(),
                origin: tcl_engine_api::NativeCVersion::V9_0,
            },
            Rep::String(cache) => R::String(cache.clone()),
            Rep::CommandName(cache) => R::CommandName(cache.clone()),
            Rep::NamespaceName(cache) => R::NamespaceName(cache.clone()),
            _ => {
                drop(rep);
                let value = Rc::new(R::Value(object.to_value()));
                self.memo.insert(identity, Rc::clone(&value));
                values.push(R::Shared(value));
                return Ok(());
            }
        };
        let value = Rc::new(wrap_export_resident(value, resident));
        self.memo.insert(identity, Rc::clone(&value));
        values.push(R::Shared(value));
        Ok(())
    }

    pub(crate) fn export(
        &mut self,
        root: &Obj,
        force_representation: bool,
    ) -> Result<tcl_engine_api::OriginalObjectResult, EngineError> {
        use tcl_engine_api::OriginalObjectResult as R;
        let mut work = Vec::new();
        let mut values = Vec::new();
        let mut active = std::collections::BTreeSet::new();
        // The root is borrowed, not retained through a manufactured raw pointer.
        // Every queued descendant is retained by an authentic ObjRef.
        self.visit(
            root,
            force_representation,
            &mut active,
            &mut work,
            &mut values,
        )?;
        while let Some(next) = work.pop() {
            match next {
                ExportWork::Visit(object) => {
                    self.visit(object.get(), false, &mut active, &mut work, &mut values)?;
                }
                ExportWork::Build(identity, shape, resident) => {
                    let value = match shape {
                        ExportShape::List(count, canonical) => {
                            let start = values
                                .len()
                                .checked_sub(count)
                                .expect("visited List members");
                            R::List {
                                items: values.split_off(start),
                                canonical,
                            }
                        }
                        ExportShape::Dictionary(count, buckets) => {
                            let start = values
                                .len()
                                .checked_sub(count * 2)
                                .expect("visited Dictionary members");
                            let mut children = values.split_off(start).into_iter();
                            let entries = (0..count)
                                .map(|_| {
                                    (
                                        children.next().expect("key"),
                                        children.next().expect("value"),
                                    )
                                })
                                .collect();
                            R::Dictionary { entries, buckets }
                        }
                    };
                    let value = wrap_export_resident(value, resident);
                    let value = Rc::new(value);
                    self.memo.insert(identity, Rc::clone(&value));
                    active.remove(&identity);
                    values.push(R::Shared(value));
                }
            }
        }
        Ok(values.pop().expect("visited original result"))
    }
}

fn wrap_export_resident(
    value: tcl_engine_api::OriginalObjectResult,
    resident: Option<(Rc<[u8]>, NativeStringStorageIdentity)>,
) -> tcl_engine_api::OriginalObjectResult {
    match resident {
        Some((string, storage)) => tcl_engine_api::OriginalObjectResult::Resident {
            value: Box::new(value),
            string,
            storage,
        },
        None => value,
    }
}

#[repr(transparent)]
pub struct ObjRef(NonNull<Obj>);

pub(crate) struct WeakObjRef {
    pointer: NonNull<Obj>,
    lifetime: Weak<()>,
}
impl WeakObjRef {
    pub(crate) fn is_alive(&self) -> bool {
        self.lifetime.strong_count() != 0
            // SAFETY: the live allocation token keeps the pointer valid.
            && !unsafe { self.pointer.as_ref() }.retired.get()
    }
    pub(crate) fn upgrade(&self) -> Option<ObjRef> {
        let _lifetime = self.lifetime.upgrade()?;
        // SAFETY: the upgraded allocation token keeps this header address valid.
        if unsafe { self.pointer.as_ref() }.retired.get() {
            return None;
        }
        // SAFETY: this single-threaded Rc token is owned by the live Obj. No
        // callbacks run between checking it and taking the object reference.
        Some(unsafe { ObjRef::adopt(self.pointer.as_ptr()) })
    }
}

/// Transport-only graph lease; native sharing excludes this retained handle.
struct ObjLifetimeLease(NonNull<Obj>);
impl ObjLifetimeLease {
    fn new(value: &ObjRef) -> Self {
        value.get().lifetime_pins.set(
            value
                .get()
                .lifetime_pins
                .get()
                .checked_add(1)
                .expect("native object pins exhausted"),
        );
        Self(value.0)
    }
    fn get(&self) -> &Obj {
        // SAFETY: this memory-only lease pins the allocation independently of C's native references.
        unsafe { self.0.as_ref() }
    }
}
impl Drop for ObjLifetimeLease {
    fn drop(&mut self) {
        let object = self.get();
        let remaining = object
            .lifetime_pins
            .get()
            .checked_sub(1)
            .expect("native object pin underflow");
        object.lifetime_pins.set(remaining);
        if remaining == 0 && object.refcount.get() <= 0 {
            // SAFETY: no native reference or allocation lease remains.
            unsafe { Obj::free(self.0.as_ptr()) };
        }
    }
}

impl ObjRef {
    /// Allocate `obj` and take the first reference to it.
    #[must_use]
    pub fn new(obj: Obj) -> Self {
        // SAFETY: the pointer is freshly allocated and non-null.
        unsafe { Self::adopt(Obj::into_raw(obj)) }
    }

    /// Take a reference to an object C created or handed over
    /// (`Tcl_IncrRefCount` in Rust ownership terms).
    ///
    /// # Safety
    ///
    /// `raw` must point to a live object allocated by [`Obj::into_raw`].
    pub unsafe fn adopt(raw: *mut Obj) -> Self {
        let pointer = NonNull::new(raw).expect("a Tcl_Obj pointer is never null");
        // SAFETY: the caller guarantees the object is live.
        let obj = unsafe { pointer.as_ref() };
        obj.refcount.set(obj.refcount.get() + 1);
        Self(pointer)
    }

    pub(crate) fn downgrade(&self) -> WeakObjRef {
        WeakObjRef {
            pointer: self.0,
            lifetime: Rc::downgrade(&self.get().lifetime),
        }
    }

    /// The raw pointer, for handing to C.
    #[must_use]
    pub fn as_ptr(&self) -> *mut Obj {
        self.0.as_ptr()
    }

    /// The object.
    #[must_use]
    pub fn get(&self) -> &Obj {
        // SAFETY: an `ObjRef` holds one reference, so the object is live.
        unsafe { self.0.as_ref() }
    }
}

impl Clone for ObjRef {
    fn clone(&self) -> Self {
        // SAFETY: this reference keeps the object live.
        unsafe { Self::adopt(self.as_ptr()) }
    }
}

impl Drop for ObjRef {
    fn drop(&mut self) {
        // SAFETY: this reference is one unit of the count being released.
        unsafe { Obj::decr_ref_count(self.as_ptr()) }
    }
}

impl Obj {
    /// Move the object to the heap with a reference count of zero: the C
    /// convention for a value nobody holds yet.
    #[must_use]
    pub fn into_raw(self) -> *mut Self {
        Box::into_raw(Box::new(self))
    }

    /// Release one reference and free the object when none remain, as the
    /// header's `Tcl_DecrRefCount` macro does: a count of zero going down frees
    /// too.
    ///
    /// # Safety
    ///
    /// `raw` must point to a live object allocated by [`Obj::into_raw`]; it is
    /// dangling afterwards if this released the last reference.
    pub unsafe fn decr_ref_count(raw: *mut Self) {
        // SAFETY: the caller guarantees the object is live.
        let remaining = unsafe { &*raw }.refcount.get() - 1;
        // SAFETY: as above.
        unsafe { &*raw }.refcount.set(remaining);
        if remaining <= 0 {
            // SAFETY: no reference remains; the allocation came from `into_raw`.
            unsafe { Self::free(raw) };
        }
    }

    /// `TclFreeObj`: free an object whose count has fallen to zero. The header's
    /// `Tcl_DecrRefCount` macro lowers the count itself and calls this when the
    /// count was one.
    ///
    /// # Safety
    ///
    /// `raw` must point to a live object allocated by [`Obj::into_raw`] that no
    /// reference holds; it is dangling afterwards.
    pub unsafe fn free(raw: *mut Self) {
        // Native references and graph transport leases are separate roles.
        // A transport lease cannot contribute to C's `Tcl_IsShared` count.
        // SAFETY: the caller guarantees this is a live shim allocation.
        unsafe { &*raw }.retired.set(true);
        if unsafe { &*raw }.lifetime_pins.get() != 0 {
            return;
        }
        // SAFETY: neither role retains this allocation.
        drop(unsafe { Box::from_raw(raw) });
    }
}

/// Counts the objects destroyed on this thread, for the tests that need to see
/// an object freed.
#[cfg(test)]
impl Drop for Obj {
    fn drop(&mut self) {
        tests::DESTROYED.with(|destroyed| destroyed.set(destroyed.get() + 1));
    }
}

#[cfg(test)]
mod tests;
