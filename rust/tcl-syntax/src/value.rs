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

//! `ValueOps` — the value seam shared across Tcl runtimes.
//!
//! The evaluation parallel of [`crate::expr::ExprOps`], lifted from "evaluate an
//! `expr`" to "construct/inspect/shimmer a Tcl value". The pure command logic in
//! `tcl-cmd-core` (string/list/dict/…) is written **once**, generic over a
//! `ValueOps` implementor; each runtime plugs in its own value representation:
//!
//! - the **bytecode VM** over `Rc<Obj>` (cheap clone; copy-on-write list ops;
//!   `try_append_bytes_in_place` is always a no-op so the caller copies),
//! - the **WASM runtime** over `*mut TclObj` (24-byte C-ABI object; amortised
//!   in-place string growth via `try_append_bytes_in_place`).
//!
//! Two deliberate contract decisions:
//!
//! 1. **Checked Unicode.** [`ValueOps::try_as_str`] yields a UTF-8 `Rc<str>`
//!    or an operational Unicode access refusal, without replacing native bytes.
//!    Tcl 8 character operations use UTF-16-style code units while Tcl 9 uses
//!    Unicode scalars; [`string_char_len`] centralises that release split. A
//!    byte-oriented runtime conforms inside its own impl; the seam never exposes
//!    byte offsets. Byte-exact commands (`append`, `binary`) use the parallel
//!    [`ValueOps::as_bytes`]/[`ValueOps::new_bytes`] rung instead.
//! 2. **Copy-on-write is explicit, not implied.** The asymmetry between a runtime
//!    that can grow a buffer in place (when unshared) and one that always copies
//!    is encoded as the [`ValueOps::try_append_bytes_in_place`] /
//!    [`ValueOps::try_list_append_in_place`] capabilities (default: cannot),
//!    **not** as a hidden `strong_count` assumption.
//!
//! Coercion failures are a closed, runtime-agnostic set ([`ValueError`]) carrying
//! the canonical Tcl message, so a single shared body produces identical errors
//! across runtimes and `tcl-cmd-core` can lift them into its command error with
//! a plain `From`.

use std::rc::Rc;

use tcl_dialect::TclVersion;

use crate::number::Radix;
use crate::raw_string::{
    NativeFatalCondition, NativeMaterializationLimitError, NativeStringAccessError,
    NativeValueAccessRefusal, UnicodeAccessError,
};

/// Count the release-defined Tcl characters in a UTF-8 string value.
///
/// Rust strings cannot contain unpaired UTF-16 surrogates, but every Unicode
/// scalar has the same width C Tcl assigns it: one Tcl 9 character, and one or
/// two Tcl 8 `Tcl_UniChar` units depending on whether it is supplementary.
///
/// **Counting is the only versioned string operation.** Character *indexing*
/// (`string index`, `range`, `first`, `last`, …) addresses Unicode scalars in
/// both runtimes whatever the dialect, so on Tcl 8 a string holding a
/// supplementary character has a length that disagrees with those indices.
/// That case is unsupported, deliberately rather than accidentally: reproducing
/// it needs a value layer that can hold an unpaired surrogate, and anything
/// less approximates. See the Tcl 8 supplementary-character boundary in
/// `docs/design/compiler/semantic-aot-optimisation.md`.
#[must_use]
pub fn string_char_len(value: &str, version: TclVersion) -> usize {
    version.string_character_model().count(value)
}

/// A value-coercion / list-parse failure — the closed set Tcl reports with
/// canonical messages, independent of any runtime's value or error type.
///
/// Downstream command logic converts this into its command-level error (e.g.
/// `tcl_cmd_core::CmdError`) with a `From` impl; the canonical wording lives
/// here once via [`ValueError::message`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueError {
    /// Native fatal behavior remains an outer host refusal.
    NativeFatalCondition(NativeFatalCondition),
    /// Selected primitive failure retaining stage, cache origin, exact bytes and
    /// error-state update. The physical adapter already applied its cache.
    NativeScalarGetter(Box<crate::scalar_getter::NativeScalarGetterError>),
    /// The value is not a wide integer (`expected integer but got "…"`).
    NotInteger(String),
    /// Native integer failure retaining the actual non-Unicode input bytes.
    NotIntegerBytes(Vec<u8>),
    /// The value is not a float (`expected floating-point number but got "…"`).
    NotDouble(String),
    /// Native floating-point failure retaining actual input bytes.
    NotDoubleBytes(Vec<u8>),
    /// The value is not a boolean (`expected boolean value but got "…"`).
    NotBoolean(String),
    /// Native boolean failure retaining actual input bytes.
    NotBooleanBytes(Vec<u8>),
    /// The value is not a well-formed list; carries the verbatim parser message
    /// (e.g. `unmatched open brace in list`).
    BadList(String),
    /// A list parser message whose exact native bytes need not be Unicode.
    BadListBytes(Vec<u8>),
    /// Typed list syntax failure retaining original byte input.
    ListParse {
        /// Native parser failure identity.
        error: crate::list::ListError,
        /// Actual original list bytes.
        source: Vec<u8>,
    },
    /// Actual list converter failure with its independently selected object
    /// producer. This receipt does not grant a successful List conversion.
    NativeListParse {
        error: crate::list::ListError,
        source: Vec<u8>,
        protocol: crate::native_string::NativeStringProtocol,
    },
    /// Typed dictionary syntax failure using the shared element grammar.
    DictionaryParse {
        /// Native parser failure identity.
        error: crate::list::ListError,
        /// Actual original dictionary bytes.
        source: Vec<u8>,
    },
    /// A dictionary has an odd number of key/value elements.
    MissingDictionaryValue,
    /// A host operation requires Unicode that the native byte value cannot supply.
    /// This is operational refusal, never a guest coercion completion.
    UnicodeAccess(UnicodeAccessError),
    /// Native string access exceeded retained storage; a host-only refusal.
    NativeStringAccess(NativeStringAccessError),
    /// Eager host construction exceeded its capacity; never a guest coercion.
    NativeMaterialization(NativeMaterializationLimitError),
    /// A reached operation requires an expression engine absent from the host.
    ExpressionEngineUnavailable,
    /// A native character operation has no selected unit model.
    CharacterModelUnavailable,
    /// A scalar numeric getter has no selected actual-engine input policy.
    ScalarNumericInputUnavailable,
    /// A reached command lacks its independently selected native protocol.
    CommandProtocolUnavailable(&'static str),
    /// An integer operation overflowed the runtime's wide-integer range
    /// (`integer value too large to represent`). The bignum-capable runtime
    /// never raises this from [`ValueOps::int_add`] (it widens); the fixed-`i64`
    /// VM does.
    IntegerOverflow,
}

impl ValueError {
    /// Exact guest error bytes; host refusal is represented separately.
    #[must_use]
    pub fn message_bytes(&self) -> Vec<u8> {
        match self {
            Self::NativeScalarGetter(error) => error.message_bytes().to_vec(),
            Self::BadListBytes(bytes) => bytes.clone(),
            Self::ListParse { error, source } | Self::NativeListParse { error, source, .. } => {
                error.full_message_bytes(source)
            }
            Self::DictionaryParse { error, source } => dictionary_parse_message(*error, source),
            Self::MissingDictionaryValue => b"missing value to go with key".to_vec(),
            Self::NotIntegerBytes(bytes) => quoted_byte_error(b"expected integer but got ", bytes),
            Self::NotDoubleBytes(bytes) => {
                quoted_byte_error(b"expected floating-point number but got ", bytes)
            }
            Self::NotBooleanBytes(bytes) => {
                quoted_byte_error(b"expected boolean value but got ", bytes)
            }
            _ => self.message().into_bytes(),
        }
    }

    /// Operational native value access failure, never a guest completion.
    #[must_use]
    pub fn native_access_refusal(&self) -> Option<NativeValueAccessRefusal> {
        match self {
            Self::NativeFatalCondition(error) => {
                Some(NativeValueAccessRefusal::FatalCondition(*error))
            }
            Self::UnicodeAccess(error) => Some((*error).into()),
            Self::NativeStringAccess(error) => Some((*error).into()),
            Self::NativeMaterialization(error) => Some((*error).into()),
            Self::ExpressionEngineUnavailable => {
                Some(NativeValueAccessRefusal::ExpressionEngineUnavailable)
            }
            Self::ScalarNumericInputUnavailable => {
                Some(NativeValueAccessRefusal::ScalarNumericInputUnavailable)
            }
            Self::CharacterModelUnavailable => {
                Some(NativeValueAccessRefusal::CharacterModelUnavailable)
            }
            Self::CommandProtocolUnavailable(command) => Some(
                NativeValueAccessRefusal::CommandProtocolUnavailable(command),
            ),
            _ => None,
        }
    }

    /// Operational Unicode refusal, which adapters must keep outside guest completion.
    #[must_use]
    pub fn unicode_refusal(&self) -> Option<UnicodeAccessError> {
        match self {
            Self::UnicodeAccess(error) => Some(*error),
            _ => None,
        }
    }

    /// The canonical Tcl error message for this coercion failure.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            ValueError::NativeScalarGetter(error) => std::str::from_utf8(error.message_bytes())
                .map_or_else(
                    |_| format!("native byte getter error: {:?}", error.message_bytes()),
                    str::to_owned,
                ),
            ValueError::NotInteger(s) => {
                format!(
                    "expected integer but got {}",
                    crate::list::describe_bad_value(s)
                )
            }
            ValueError::NotDouble(s) => format!(
                "expected floating-point number but got {}",
                crate::list::describe_bad_value(s)
            ),
            ValueError::NotBoolean(s) => {
                format!(
                    "expected boolean value but got {}",
                    crate::list::describe_bad_value(s)
                )
            }
            ValueError::NotIntegerBytes(bytes)
            | ValueError::NotDoubleBytes(bytes)
            | ValueError::NotBooleanBytes(bytes) => {
                format!("native byte coercion error: {bytes:?}")
            }
            ValueError::ListParse { error, .. }
            | ValueError::NativeListParse { error, .. }
            | ValueError::DictionaryParse { error, .. } => error.message().to_owned(),
            ValueError::MissingDictionaryValue => "missing value to go with key".to_owned(),
            ValueError::BadList(msg) => msg.clone(),
            ValueError::BadListBytes(bytes) => format!("native byte list error: {bytes:?}"),
            ValueError::NativeFatalCondition(error) => error.to_string(),
            ValueError::NativeStringAccess(error) => error.to_string(),
            ValueError::NativeMaterialization(error) => error.to_string(),
            ValueError::ExpressionEngineUnavailable => {
                NativeValueAccessRefusal::ExpressionEngineUnavailable.to_string()
            }
            ValueError::ScalarNumericInputUnavailable => {
                NativeValueAccessRefusal::ScalarNumericInputUnavailable.to_string()
            }
            ValueError::CharacterModelUnavailable => {
                NativeValueAccessRefusal::CharacterModelUnavailable.to_string()
            }
            ValueError::CommandProtocolUnavailable(command) => {
                NativeValueAccessRefusal::CommandProtocolUnavailable(command).to_string()
            }
            ValueError::UnicodeAccess(error) => {
                format!("host Unicode access refused at byte {}", error.valid_up_to)
            }
            ValueError::IntegerOverflow => "integer value too large to represent".to_string(),
        }
    }
}

/// Dictionary wording of an actual typed list-element parse failure.
#[must_use]
pub fn dictionary_parse_message(error: crate::list::ListError, source: &[u8]) -> Vec<u8> {
    let message = error.full_message_bytes(source);
    let (prefix, replacement): (&[u8], &[u8]) = match error {
        crate::list::ListError::UnmatchedBrace => (
            b"unmatched open brace in list",
            b"unmatched open brace in dict",
        ),
        crate::list::ListError::UnmatchedQuote => (
            b"unmatched open quote in list",
            b"unmatched open quote in dict",
        ),
        _ => (b"list element in ", b"dict element in "),
    };
    let mut output = replacement.to_vec();
    output.extend_from_slice(&message[prefix.len()..]);
    output
}

fn quoted_byte_error(prefix: &[u8], bytes: &[u8]) -> Vec<u8> {
    let mut message = prefix.to_vec();
    message.push(b'"');
    message.extend_from_slice(bytes);
    message.push(b'"');
    message
}

impl core::fmt::Display for ValueError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.message())
    }
}

impl std::error::Error for ValueError {}

impl From<NativeStringAccessError> for ValueError {
    fn from(error: NativeStringAccessError) -> Self {
        Self::NativeStringAccess(error)
    }
}
impl From<NativeValueAccessRefusal> for ValueError {
    fn from(error: NativeValueAccessRefusal) -> Self {
        match error {
            NativeValueAccessRefusal::FatalCondition(error) => Self::NativeFatalCondition(error),
            NativeValueAccessRefusal::Unicode(error) => error.into(),
            NativeValueAccessRefusal::StringAccess(error) => error.into(),
            NativeValueAccessRefusal::Materialization(error) => Self::NativeMaterialization(error),
            NativeValueAccessRefusal::ExpressionEngineUnavailable => {
                Self::ExpressionEngineUnavailable
            }
            NativeValueAccessRefusal::CharacterModelUnavailable => Self::CharacterModelUnavailable,
            NativeValueAccessRefusal::ScalarNumericInputUnavailable => {
                Self::ScalarNumericInputUnavailable
            }
            NativeValueAccessRefusal::CommandProtocolUnavailable(command) => {
                Self::CommandProtocolUnavailable(command)
            }
        }
    }
}

impl From<UnicodeAccessError> for ValueError {
    fn from(error: UnicodeAccessError) -> Self {
        Self::UnicodeAccess(error)
    }
}

/// Tcl's dict canonicalisation rule, as slot indices — **the** implementation of
/// "first-occurrence key position, last value wins".
///
/// `keys` is one key string per key/value slot, in source order: slot `i` is
/// elements `2i` / `2i + 1` of the flat list rep, or arguments `2i` / `2i + 1`
/// of `dict create`. The result is one `(key_slot, value_slot)` entry per
/// *surviving* key, in canonical order — the slot whose key spelling and
/// position the dict keeps, and the slot whose value it keeps. With no
/// duplicate key that is `[(0, 0), (1, 1), …]`; for `a 1 b 2 a 3` it is
/// `[(0, 2), (1, 1)]`.
///
/// This is `SetDictFromAny` (`tmp/tcl9.0.4/generic/tclDictObj.c:589`) walking
/// the list rep and `Tcl_DictObjPut`ing each pair: the hash entry for a
/// repeated key already exists, so the *value* is overwritten while the entry
/// keeps its original chain position. `DictCreateCmd` walks its arguments the
/// same way, which is why one function answers for both.
///
/// Working in indices rather than values is what lets every layer share it:
/// the runtime seam ([`ValueOps::dict_pairs`]) maps them back onto its own
/// value handles, and the compile-time folders (`tcl_registry::const_fold`'s
/// `parse_dict` / `fold_dict_create`, `tcl_compiler`'s `fold_dict_create_cmd`)
/// map them onto their own strings — none of them re-derives the rule.
///
/// That mattered: the rule existed in three independent copies plus one place
/// it had been *missed*, where `dict get {a 1 a 2} a` folded to `1` while both
/// tclsh oracles say `2`. Callers must not re-implement the walk; the
/// cross-crate parity gate `dict_canonicalisation_parity` fails when a copy
/// reappears and diverges.
///
/// The odd-length check belongs to the caller: what an unpaired trailing
/// element means (an error, or a declined fold) differs per layer.
///
/// The key type is borrowed and only `Eq + Hash`, so a byte-oriented value
/// model can bind this with `&[u8]` keys as readily as a string one does with
/// `&str` — keys are compared by their exact rep either way, which is what
/// `Tcl_DictObjPut`'s hash does.
///
/// The WASM runtime's *native* dict rep (`runtime/rust/src/dict.rs`)
/// deliberately does not call this: it maintains a live key index across
/// mutation, so it canonicalises incrementally (one `Tcl_DictObjPut` per
/// insert) rather than in one walk. Its agreement with this rule is pinned by
/// its own parity test rather than by construction.
#[must_use]
pub fn canonical_dict_slots<'a, K>(keys: impl IntoIterator<Item = &'a K>) -> Vec<(usize, usize)>
where
    K: ?Sized + Eq + std::hash::Hash + 'a,
{
    // Key → its slot's position in `slots`, so a duplicate is O(1) to find: a
    // linear re-scan per element made this O(N²) on every dict operation (D3).
    // Both containers are sized from the iterator up front — this runs on every
    // VM dict opcode, so the growth reallocations are worth avoiding.
    let keys = keys.into_iter();
    let expected = keys.size_hint().0;
    let mut first_seen: std::collections::HashMap<&'a K, usize> =
        std::collections::HashMap::with_capacity(expected);
    let mut slots: Vec<(usize, usize)> = Vec::with_capacity(expected);
    for (slot, key) in keys.enumerate() {
        if let Some(&position) = first_seen.get(key) {
            slots[position].1 = slot; // last value wins, first position kept
        } else {
            first_seen.insert(key, slots.len());
            slots.push((slot, slot));
        }
    }
    slots
}

/// The value operations a Tcl command core supplies its runtime.
///
/// The shared command bodies in `tcl-cmd-core` drive these; construction,
/// shimmer caching, interning, and result-object building stay the runtime's
/// business (hence `&mut self`). Monomorphises per implementor — zero dynamic
/// dispatch, exactly like [`crate::expr::ExprOps`].
/// The canonical ordered key/value pairs of a dict, or a [`ValueError`] when the
/// backing list is odd-length. The pair type is the implementor's `Value`, so it
/// is parameterised here to keep [`ValueOps::dict_pairs`]'s signature readable.
pub type DictPairs<V> = Result<Vec<(V, V)>, ValueError>;

/// An integer's sign and magnitude for arbitrary-precision formatting.
///
/// The magnitude has no sign or radix prefix and uses lowercase digits. The
/// command core owns conversion prefixes, precision, and padding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegerMagnitude {
    /// Whether the integer is negative.
    pub negative: bool,
    /// The unsigned digits in the requested radix.
    pub digits: String,
}

/// Representation evidence independent of a value's known string contents.
/// A list and an equal string may take different native operations in Jim.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ValueRepresentation {
    /// Known native ordinary string representation.
    String,
    /// Known native list internal representation, retaining element values.
    List,
    /// Known native dictionary internal representation, retaining key/value objects.
    Dict,
    /// Native representation is not established by the available value facts.
    #[default]
    Unknown,
}

/// Selected ordinary integer or effect-free safe integer-expression conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntegerOperandGrammar {
    /// Convert an ordinary integer with the selected native tower.
    Integer,
    /// Try an ordinary integer, then the shared safe-expression evaluator.
    SafeIntegerExpression,
}

/// Integer operand preparation retains numeric and safe-expression errors separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegerOperandError {
    /// An ordinary native integer conversion failed.
    Integer(ValueError),
    /// The selected safe expression did not produce an integer.
    SafeExpression,
}

impl IntegerOperandGrammar {
    /// Normalize through the selected value adapter without reading a variable.
    /// The caller chooses the authored validation phase before calling this.
    pub fn prepare<O: ValueOps>(
        self,
        ops: &mut O,
        amount: &O::Value,
    ) -> Result<O::Value, IntegerOperandError> {
        match ops.int_add(None, amount) {
            Ok(value) => Ok(value),
            Err(error) if self == Self::Integer || error.native_access_refusal().is_some() => {
                Err(IntegerOperandError::Integer(error))
            }
            Err(_) => {
                // The safe numeric grammar rejects a raw non-Unicode token;
                // its diagnostic still contains the original operand bytes.
                let source = ops
                    .try_as_str(amount)
                    .map_err(|_| IntegerOperandError::SafeExpression)?;
                let value = ops.eval_index_expression(&source).map_err(|error| {
                    if error.native_access_refusal().is_some() {
                        IntegerOperandError::Integer(error)
                    } else {
                        IntegerOperandError::SafeExpression
                    }
                })?;
                Ok(ops.new_int(value))
            }
        }
    }
}

impl IntegerOperandError {
    /// Exact safe-expression presentation, preserving the original byte spelling.
    /// Ordinary numeric diagnostics are presented by their numeric owner.
    #[must_use]
    pub fn safe_expression_message_bytes(&self, amount: &[u8]) -> Option<Vec<u8>> {
        if !matches!(self, Self::SafeExpression) {
            return None;
        }
        let mut message = b"expected integer expression but got \"".to_vec();
        message.extend_from_slice(amount);
        message.push(b'"');
        Some(message)
    }
}

/// Outcome from a selected same-original static option lookup.
/// The adapter retains the native table/cache authority; callers retain the
/// original value and receive byte-exact guest diagnostics separately from
/// operational host refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OriginalOptionLookup {
    Index(usize),
    Failure {
        message: Vec<u8>,
        error_code: Vec<u8>,
        string_result: Option<crate::native_string::NativeStringProtocol>,
    },
}

/// Nonconverting shape of the same original at native concatenation.
/// This metadata does not license a List getter, header copy or mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeConcatListShape {
    /// Actual ordinary List flag and existing byte length.
    List {
        /// Actual member count, without parsing or inspecting any child.
        length: usize,
        /// Whether the original backing has its canonical flag set.
        canonical: bool,
        /// Existing resident length; None does not invoke an updater.
        resident_length: Option<usize>,
    },
    /// Actual abstract List with the selected native index procedure.
    Indexed,
    /// Another original representation, without conversion.
    Other,
}

/// Native concat's reached first-member lookup and its independent temporary.
/// Ordinary members are borrowed; an abstract index hook can return a fresh
/// refcount-zero original that must be bounced after append or fallback.
pub struct NativeConcatFirstElement<T> {
    /// Bytes produced by the selected getter on that exact original child.
    pub bytes: Rc<[u8]>,
    /// Actual fresh abstract child; None adds no member reference.
    pub temporary: Option<T>,
}

/// Backend value operations consumed by shared command implementations.
pub trait ValueOps {
    /// The runtime's value type (a cheap-to-clone handle).
    type Value: Clone;

    /// Resolve an actual original object against an immutable declaration table.
    /// `None` selects the portable matcher for non-native models or Jim; a
    /// concrete adapter without its physical protocol must return a refusal.
    fn original_option_index(
        &mut self,
        _original: &Self::Value,
        _words: &'static [&'static str],
        _exact: bool,
        _noun: &'static str,
    ) -> Result<Option<OriginalOptionLookup>, ValueError> {
        Ok(None)
    }

    /// Actual or explicitly authored native string/name policy issuer.
    /// Purpose-specific owners select their own operation from this receipt.
    fn name_policy_protocol(&self) -> Option<crate::naming::NamePolicyProtocol> {
        None
    }

    /// Publish an array-existence Boolean through the backend's selected
    /// result producer. Native interpreter constants require an actual owning
    /// environment; the default refuses rather than creating a fresh substitute.
    fn array_existence_result(&mut self, _present: bool) -> Result<Self::Value, ValueError> {
        Err(ValueError::CommandProtocolUnavailable(
            "array existence result producer",
        ))
    }

    /// Compare retained physical object identities without converting values.
    fn same_object(&self, _left: &Self::Value, _right: &Self::Value) -> Option<bool> {
        None
    }

    /// Inspect the actual primary representation without materialization.
    fn native_object_snapshot(
        &self,
        _value: &Self::Value,
    ) -> Result<crate::native_object::NativeObjectSnapshot, ValueError> {
        Err(ValueError::CommandProtocolUnavailable(
            "native physical object snapshot",
        ))
    }

    /// Retire the actual primary cache after a selected native temporary lookup.
    /// The already-resident exact string and storage identity stay unchanged.
    fn discard_native_internal_representation(
        &mut self,
        _value: &Self::Value,
    ) -> Result<(), ValueError> {
        Err(ValueError::CommandProtocolUnavailable(
            "native cache retirement",
        ))
    }

    /// Reach actual C Unicode preparation on the original object.
    fn native_unicode_units(&mut self, _value: &Self::Value) -> Result<Rc<[u32]>, ValueError> {
        Err(ValueError::CommandProtocolUnavailable(
            "native original object Unicode",
        ))
    }

    /// Construct a fresh native C String from selected Unicode units. Concrete
    /// object owners independently validate their actual interpreter protocol.
    fn native_unicode_string_result(
        &mut self,
        _units: Rc<[u32]>,
        _version: tcl_dialect::TclVersion,
    ) -> Result<Self::Value, ValueError> {
        Err(ValueError::CommandProtocolUnavailable(
            "native Unicode result constructor",
        ))
    }

    /// Construct the actual C external-UTF8 result as binary storage with its
    /// updater recipe retained before allocation. The supplied version and
    /// octets do not confer a native producer or string-residency receipt.
    /// Concrete adapters authenticate the actual C interpreter independently;
    /// unknown adapters and Jim's separately available extensions refuse.
    fn native_external_utf8_result(
        &mut self,
        _bytes: &[u8],
        _version: tcl_dialect::TclVersion,
    ) -> Result<Self::Value, ValueError> {
        Err(ValueError::CommandProtocolUnavailable("external UTF-8 binary result issuer"))
    }

    /// Selected native concat protocol; unknown adapters abstain.
    fn concat_policy(&self) -> Option<tcl_dialect::ConcatPolicy> {
        None
    }

    /// Whether this value currently has a native list representation.
    /// This query must not coerce or shimmer the value.
    fn has_list_representation(&self, _value: &Self::Value) -> bool {
        false
    }

    /// Inspect the original native concat List shape without conversion.
    /// Concrete adapters authenticate the actual physical string issuer.
    fn native_concat_list_shape(
        &self,
        _value: &Self::Value,
    ) -> Result<NativeConcatListShape, ValueError> {
        Err(ValueError::CommandProtocolUnavailable(
            "native concat List shape",
        ))
    }

    /// Create the actual selected empty List constructor result.
    fn native_concat_empty_list(&mut self) -> Result<Self::Value, ValueError> {
        Err(ValueError::CommandProtocolUnavailable(
            "native concat List constructor",
        ))
    }

    /// Apply C8.5+ `TclListObjCopy` to the original: ordinary Lists share
    /// backing; actual abstract length-hook headers duplicate before `GetElements`.
    fn native_concat_copy_list(&mut self, _value: &Self::Value) -> Result<Self::Value, ValueError> {
        Err(ValueError::CommandProtocolUnavailable(
            "native concat List copy",
        ))
    }

    /// Append the same original List members to the unshared result header.
    /// The result's backing owner performs the native child-reference COW.
    fn native_concat_append_list(
        &mut self,
        _result: &Self::Value,
        _source: &Self::Value,
    ) -> Result<(), ValueError> {
        Err(ValueError::CommandProtocolUnavailable(
            "native concat List append",
        ))
    }

    /// Get only the first original List member's native string.
    /// A missing member stays distinct from an empty member.
    fn native_concat_first_bytes(
        &mut self,
        _value: &Self::Value,
    ) -> Result<Option<NativeConcatFirstElement<Self::Value>>, ValueError> {
        Err(ValueError::CommandProtocolUnavailable(
            "native concat first member",
        ))
    }

    /// Retire the actual fresh abstract Index result at its native bounce point.
    /// Ordinary borrowed members have no temporary to release.
    fn release_native_concat_first(&mut self, first: NativeConcatFirstElement<Self::Value>) {
        drop(first);
    }

    /// Materialize one original through the physical concat string updater.
    fn native_concat_string_bytes(&mut self, value: &Self::Value) -> Result<Rc<[u8]>, ValueError> {
        self.native_string_bytes(value)
    }

    /// Construct the real release-selected string fallback result primary.
    fn native_concat_string_result(&mut self, _bytes: &[u8]) -> Result<Self::Value, ValueError> {
        Err(ValueError::CommandProtocolUnavailable(
            "native concat string result",
        ))
    }

    /// Retire an abandoned fresh concat result, including its member owners.
    fn discard_native_concat_result(&mut self, value: Self::Value) {
        drop(value);
    }

    /// Complete selected container index policy. Unknown adapters abstain.
    fn index_syntax(&self) -> Option<tcl_dialect::IndexSyntax> {
        None
    }

    /// Physical String construction for the reached C bad-index error producer.
    /// The selected index grammar remains independent of this original runtime
    /// result issuer. Portable adapters supply no physical representation claim.
    fn index_error_string_protocol(
        &self,
    ) -> Result<Option<crate::native_string::NativeStringProtocol>, ValueError> {
        Ok(None)
    }

    /// Evaluate Jim's safe integer expression without invoking public commands.
    /// Engines reject variable, script and interpolated-word requests.
    fn eval_index_expression(&mut self, source: &str) -> Result<i64, ValueError> {
        Err(ValueError::NotInteger(source.to_owned()))
    }

    // -- construction / shimmer --

    /// A value from a borrowed string (copies).
    fn new_str(&mut self, s: &str) -> Self::Value;
    /// A value from an owned string (may avoid a copy).
    fn new_string(&mut self, s: String) -> Self::Value {
        self.new_str(&s)
    }
    /// The empty string value.
    fn empty(&mut self) -> Self::Value {
        self.new_str("")
    }
    /// A wide-integer value.
    fn new_int(&mut self, n: i64) -> Self::Value;
    /// A double value.
    fn new_double(&mut self, f: f64) -> Self::Value;
    /// A boolean value (string side canonicalises to `"0"`/`"1"`).
    fn new_bool(&mut self, b: bool) -> Self::Value;
    /// A list value from element handles.
    fn new_list(&mut self, items: Vec<Self::Value>) -> Self::Value;

    /// Keep a borrowed value handle alive across later runtime callbacks.
    ///
    /// Owning value models need no extra work. Pointer-based runtimes override
    /// this with their object reference-count increment; every successful pin
    /// must be paired with [`Self::unpin_value`], including error exits.
    fn pin_value(&mut self, _value: &Self::Value) {}

    /// Release one transient hold established by [`Self::pin_value`].
    fn unpin_value(&mut self, _value: &Self::Value) {}

    // -- string access (UTF-8; char-indexed downstream) --

    /// Independently selected native character-unit protocol. Unknown adapters
    /// retain checked Unicode access rather than assuming a byte interpreter.
    fn string_character_model(&self) -> Option<tcl_dialect::StringCharacterModel> {
        None
    }

    /// Checked Unicode projection of the actual native bytes. Failure preserves
    /// the value and must propagate as a host refusal, outside guest completion.
    fn try_as_str(&mut self, v: &Self::Value) -> Result<Rc<str>, UnicodeAccessError> {
        let bytes = self.as_bytes(v);
        std::str::from_utf8(&bytes)
            .map(Rc::from)
            .map_err(|error| UnicodeAccessError {
                valid_up_to: error.valid_up_to(),
                error_len: error.error_len(),
            })
    }

    /// Checked character count for adapters that require Unicode. Native byte
    /// character engines may override this without asking for a Unicode view.
    fn try_char_len(&mut self, v: &Self::Value) -> Result<usize, UnicodeAccessError> {
        Ok(self.try_as_str(v)?.chars().count())
    }

    /// Reach a checked native character count, retaining original cache effects.
    /// Unicode adapters use their checked projection; physical native adapters
    /// authenticate their own string recipe and preserve capability refusals.
    fn native_char_len(&mut self, v: &Self::Value) -> Result<usize, ValueError> {
        self.try_char_len(v).map_err(Into::into)
    }

    // -- numeric / boolean coercion (closed error set) --

    /// As a wide integer (`Tcl_GetWideIntFromObj`).
    fn as_int(&mut self, v: &Self::Value) -> Result<i64, ValueError>;

    /// The integer's sign and magnitude in `radix` for a bignum format path.
    ///
    /// Fixed-width value models use the wide-integer default. Bignum-capable
    /// runtimes override it so `format %llx` never narrows through `i64`.
    fn integer_magnitude(
        &mut self,
        v: &Self::Value,
        radix: Radix,
        _syntax: tcl_dialect::NumberSyntax,
    ) -> Result<IntegerMagnitude, ValueError> {
        let value = self.as_int(v)?;
        let digits = match radix {
            Radix::Bin => format!("{:b}", value.unsigned_abs()),
            Radix::Oct => format!("{:o}", value.unsigned_abs()),
            Radix::Dec => value.unsigned_abs().to_string(),
            Radix::Hex => format!("{:x}", value.unsigned_abs()),
        };
        Ok(IntegerMagnitude {
            negative: value.is_negative(),
            digits,
        })
    }

    /// Parse a `string compare`/`string equal` `-length` argument.
    ///
    /// A non-positive value disables the length limit. The default uses the
    /// runtime's wide-integer coercion; a runtime with a wider integer tower
    /// can override this to retain its distinct overflow diagnostic.
    fn string_compare_length(&mut self, v: &Self::Value) -> Result<Option<usize>, ValueError> {
        Ok(usize::try_from(self.as_int(v)?).ok())
    }

    /// As a double (`Tcl_GetDoubleFromObj`).
    fn as_double(&mut self, v: &Self::Value) -> Result<f64, ValueError>;
    /// As a boolean (`Tcl_GetBooleanFromObj`).
    fn as_bool(&mut self, v: &Self::Value) -> Result<bool, ValueError>;

    // -- integer arithmetic (the value-model boundary made explicit) --

    /// Integer sum `a + b` as a fresh value (`incr`'s arithmetic step), where a
    /// `None` left operand denotes an **absent value treated as zero** — `incr`
    /// of an unset variable starts at 0.
    ///
    /// The compatibility default reports fixed-wide overflow. Production
    /// adapters override this seam with the selected native arithmetic policy:
    /// Tcl 8.4 and Jim wrap, while Tcl 8.5 and later promote to arbitrary precision.
    /// The absent-to-zero rule stays here so adapters own their transient values.
    fn int_add(
        &mut self,
        a: Option<&Self::Value>,
        b: &Self::Value,
    ) -> Result<Self::Value, ValueError> {
        let x = match a {
            Some(v) => self.as_int(v)?,
            None => 0,
        };
        let y = self.as_int(b)?;
        let sum = x.checked_add(y).ok_or(ValueError::IntegerOverflow)?;
        Ok(self.new_int(sum))
    }

    // -- list (copy-on-write) --

    /// The list elements (`Tcl_ListObjGetElements`), parsing+caching on first
    /// call. Element handles are cheap clones.
    fn list_elements(&mut self, v: &Self::Value) -> Result<Vec<Self::Value>, ValueError>;

    /// The list length (`llength`). Default walks [`ValueOps::list_elements`];
    /// impls with an O(1) length override.
    fn list_len(&mut self, v: &Self::Value) -> Result<usize, ValueError> {
        Ok(self.list_elements(v)?.len())
    }

    /// The element at `i` (`lindex`), or `None` if out of range. Default clones
    /// the element vector; impls with random access override.
    fn list_index(&mut self, v: &Self::Value, i: usize) -> Result<Option<Self::Value>, ValueError> {
        Ok(self.list_elements(v)?.into_iter().nth(i))
    }

    /// Append one element (`lappend` step), copy-on-write: the default rebuilds;
    /// an impl that owns the backing vector uniquely may mutate in place.
    fn list_append(
        &mut self,
        list: Self::Value,
        item: Self::Value,
    ) -> Result<Self::Value, ValueError> {
        let mut items = self.list_elements(&list)?;
        items.push(item);
        Ok(self.new_list(items))
    }

    // -- dict (a dict is an even-length list; keys compared by string rep) --

    /// The dict's **canonical** ordered key/value pairs (`Tcl_DictObjFirst`
    /// order: first-occurrence position, last value winning on a duplicate key).
    ///
    /// The default derives this from [`ValueOps::list_elements`] + the string
    /// rep — correct for any value model (the VM's list-backed dict and the WASM
    /// runtime's `TclDict`, which shimmers to a list). An impl with a native dict
    /// rep may override for efficiency. Errors with the canonical "missing value
    /// to go with key" when the list is odd-length.
    fn dict_pairs(&mut self, v: &Self::Value) -> DictPairs<Self::Value> {
        let elems = self.list_elements(v)?;
        if elems.len() % 2 != 0 {
            return Err(ValueError::MissingDictionaryValue);
        }
        // The canonicalisation rule itself lives in `canonical_dict_slots` —
        // this method binds it to a value model, it does not restate it.
        let keys: Vec<Rc<[u8]>> = elems
            .as_chunks::<2>()
            .0
            .iter()
            .map(|chunk| self.as_bytes(&chunk[0]))
            .collect();
        Ok(canonical_dict_slots(keys.iter().map(AsRef::as_ref))
            .into_iter()
            .map(|(key_slot, value_slot)| {
                (
                    elems[key_slot * 2].clone(),
                    elems[value_slot * 2 + 1].clone(),
                )
            })
            .collect())
    }

    /// Retained bucket-array size for a native dict representation.
    ///
    /// Tcl hash tables grow but do not shrink, and `dict info` exposes that
    /// history. String/list-only value models return `None`; runtimes with a
    /// native dict rep override this after validating/shimmering `v`.
    fn dict_hash_bucket_count(&mut self, _v: &Self::Value) -> Result<Option<usize>, ValueError> {
        Ok(None)
    }

    /// Build a dict value from canonical key/value pairs. The default interleaves
    /// them into a list value (a dict *is* an even-length list); an impl with a
    /// native dict rep may override.
    fn new_dict(&mut self, pairs: Vec<(Self::Value, Self::Value)>) -> Self::Value {
        let mut items = Vec::with_capacity(pairs.len() * 2);
        for (k, v) in pairs {
            items.push(k);
            items.push(v);
        }
        self.new_list(items)
    }

    /// Build a native dict starting with a copied table's bucket-array size.
    ///
    /// The default has no typed hash-table representation and ignores the
    /// hint. Native dict owners override it for copy-then-transform commands
    /// such as `dict remove`, whose growth remains observable through `info`.
    fn new_dict_with_hash_bucket_count(
        &mut self,
        pairs: Vec<(Self::Value, Self::Value)>,
        _bucket_count: usize,
    ) -> Self::Value {
        self.new_dict(pairs)
    }

    /// Construct native dictionary keys through checked original-object access.
    /// Unicode-only value models may use their ordinary dictionary constructor.
    ///
    /// # Errors
    /// Returns an unavailable native key updater or issuer.
    fn new_dict_checked(
        &mut self,
        pairs: Vec<(Self::Value, Self::Value)>,
    ) -> Result<Self::Value, ValueError> {
        Ok(self.new_dict(pairs))
    }

    /// Checked dictionary construction retaining the copied table bucket count.
    ///
    /// # Errors
    /// Returns the same original-key failures as `new_dict_checked`.
    fn new_dict_with_hash_bucket_count_checked(
        &mut self,
        pairs: Vec<(Self::Value, Self::Value)>,
        bucket_count: usize,
    ) -> Result<Self::Value, ValueError> {
        Ok(self.new_dict_with_hash_bucket_count(pairs, bucket_count))
    }

    // -- bytes (byte-exact; the value-representation seam for append/binary) --

    /// The value's exact raw bytes. Every adapter must preserve invalid Unicode.
    fn as_bytes(&mut self, v: &Self::Value) -> Rc<[u8]>;

    /// Materialise the native string representation before a byte consumer
    /// selects its own input extent. The default requires an adapter whose
    /// `as_bytes` already supplies an authoritative resident string. Adapters
    /// with pure binary storage override this with their independently selected
    /// native storage recipe; absence is an operational host refusal.
    fn native_string_bytes(&mut self, v: &Self::Value) -> Result<Rc<[u8]>, ValueError> {
        Ok(self.as_bytes(v))
    }

    /// Construct a native value retaining every supplied byte. There is no
    /// Unicode fallback; a string-only test model must keep its inputs bounded.
    fn new_bytes(&mut self, bytes: &[u8]) -> Self::Value;

    /// Native Jim string constructor with the character-count receipt authored
    /// by its range operation. Concrete Jim adapters retain this string intrep
    /// until a later conversion replaces it; equal bytes alone do not recover
    /// the receipt. Non-native fixture adapters may use the ordinary byte value.
    fn new_jim_string(&mut self, bytes: &[u8], _character_count: usize) -> Self::Value {
        self.new_bytes(bytes)
    }

    /// Apply native Jim trim cuts through the actual physical object owner.
    /// Concrete Jim adapters preserve an unshared suffix cut's cached count,
    /// copy shared cuts, and perform the required string-intrep conversion.
    /// Non-native fixture models may construct the ordinary byte value.
    fn jim_string_trim_result(
        &mut self,
        value: &Self::Value,
        plan: crate::raw_string::JimStringTrimPlan,
    ) -> Self::Value {
        let bytes = self.as_bytes(value);
        if plan.byte_start() == 0 && plan.byte_end() == bytes.len() {
            value.clone()
        } else {
            self.new_bytes(&bytes[plan.byte_start()..plan.byte_end()])
        }
    }

    // -- copy-on-write escape hatches (amortised in-place growth) --

    /// Try to append `bytes` to `v`'s value **in place** (amortised growth),
    /// returning whether it happened. A runtime whose object can be grown when
    /// unshared (the WASM `*mut TclObj`) overrides this; the default (the
    /// `Rc`-handle VM) returns `false`, signalling the caller to build a fresh
    /// value. This makes the COW asymmetry an explicit capability rather than a
    /// hidden `strong_count` assumption — and keeps `append` amortised O(1) per
    /// byte rather than O(n²) over a building loop.
    fn try_append_bytes_in_place(&mut self, _v: &mut Self::Value, _bytes: &[u8]) -> bool {
        false
    }

    /// Try to append `item` to `list`'s list value **in place** (the `lappend`
    /// analogue of [`try_append_bytes_in_place`](Self::try_append_bytes_in_place)),
    /// returning whether it happened. The default returns `Ok(false)` (the VM
    /// rebuilds); a runtime owning the backing vector uniquely overrides it.
    /// A selected conversion or storage refusal remains an error, so the caller
    /// cannot replace an unavailable native operation with a fresh value.
    fn try_list_append_in_place(
        &mut self,
        _list: &mut Self::Value,
        _item: &Self::Value,
    ) -> Result<bool, ValueError> {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A string-backed `ValueOps` model: every value is an `Rc<str>`; lists are
    /// whitespace-separated words. Enough to drive every default-method body in
    /// the seam. Expected results below were cross-checked against `tclsh9.0`
    /// (and `tclsh8.6` where the two agree) so the shared logic matches C Tcl.
    #[derive(Default)]
    struct Strs;

    impl ValueOps for Strs {
        type Value = Rc<str>;

        fn new_str(&mut self, s: &str) -> Rc<str> {
            Rc::from(s)
        }
        fn new_int(&mut self, n: i64) -> Rc<str> {
            Rc::from(n.to_string().as_str())
        }
        fn new_double(&mut self, f: f64) -> Rc<str> {
            Rc::from(f.to_string().as_str())
        }
        fn new_bool(&mut self, b: bool) -> Rc<str> {
            Rc::from(if b { "1" } else { "0" })
        }
        fn new_list(&mut self, items: Vec<Rc<str>>) -> Rc<str> {
            Rc::from(
                items
                    .iter()
                    .map(std::convert::AsRef::as_ref)
                    .collect::<Vec<_>>()
                    .join(" ")
                    .as_str(),
            )
        }
        fn as_bytes(&mut self, v: &Rc<str>) -> std::rc::Rc<[u8]> {
            std::rc::Rc::from(v.as_bytes())
        }
        fn new_bytes(&mut self, bytes: &[u8]) -> Self::Value {
            self.new_str(std::str::from_utf8(bytes).expect("Unicode-only fixture input"))
        }

        fn as_int(&mut self, v: &Rc<str>) -> Result<i64, ValueError> {
            v.parse::<i64>()
                .map_err(|_| ValueError::NotInteger(v.to_string()))
        }
        fn as_double(&mut self, v: &Rc<str>) -> Result<f64, ValueError> {
            v.parse::<f64>()
                .map_err(|_| ValueError::NotDouble(v.to_string()))
        }
        fn as_bool(&mut self, v: &Rc<str>) -> Result<bool, ValueError> {
            // The boolean-context acceptor (words by prefix, else any
            // number vs zero) — the mock must honour the real contract.
            crate::boolean::truthiness(v).ok_or_else(|| ValueError::NotBoolean(v.to_string()))
        }
        fn list_elements(&mut self, v: &Rc<str>) -> Result<Vec<Rc<str>>, ValueError> {
            Ok(v.split_whitespace().map(Rc::from).collect())
        }
    }

    #[test]
    fn an_absent_expression_engine_remains_a_host_refusal() {
        let refusal = NativeValueAccessRefusal::ExpressionEngineUnavailable;
        let error = ValueError::from(refusal);
        assert_eq!(error.native_access_refusal(), Some(refusal));
        assert_eq!(error.unicode_refusal(), None);
    }

    #[test]
    fn an_unselected_command_protocol_remains_a_typed_host_refusal() {
        let refusal = NativeValueAccessRefusal::CommandProtocolUnavailable("namespace code");
        let error = ValueError::from(refusal);
        assert_eq!(error.native_access_refusal(), Some(refusal));
        assert_eq!(error.unicode_refusal(), None);
        assert_eq!(
            error.message(),
            "namespace code native handler policy is not selected"
        );
    }

    #[test]
    fn array_existence_requires_an_explicit_result_producer() {
        // This string-only adapter has an ordinary Boolean constructor but no
        // retained interpreter environment or selected array-result issuer.
        let mut ops = Strs;
        assert_eq!(ops.new_bool(false).as_ref(), "0");
        assert_eq!(
            ops.array_existence_result(false),
            Err(ValueError::CommandProtocolUnavailable(
                "array existence result producer"
            ))
        );
    }

    #[test]
    fn construction_defaults() {
        let mut o = Strs;
        // new_string defaults through new_str; empty is "".
        assert_eq!(o.new_string("hi".to_string()).as_ref(), "hi");
        assert_eq!(o.empty().as_ref(), "");
        assert_eq!(o.new_int(42).as_ref(), "42");
        assert_eq!(o.new_bool(true).as_ref(), "1");
        assert_eq!(o.new_bool(false).as_ref(), "0");
    }

    #[test]
    fn char_len_counts_code_points() {
        // tclsh9.0: `string length héllo` == 5 (code points), not bytes.
        let mut o = Strs;
        let v = o.new_str("héllo");
        assert_eq!(o.try_char_len(&v).unwrap(), 5);
        let ascii = o.new_str("abc");
        assert_eq!(o.try_char_len(&ascii).unwrap(), 3);
    }

    #[test]
    fn release_length_distinguishes_supplementary_characters() {
        let value = "A\u{1f600}B";
        assert_eq!(string_char_len(value, TclVersion::V8_6), 4);
        assert_eq!(string_char_len(value, TclVersion::V9_0), 3);
    }

    #[test]
    fn int_add_default_and_unset_is_zero() {
        let mut o = Strs;
        let three = o.new_int(3);
        let four = o.new_int(4);
        // 3 + 4 == 7.
        assert_eq!(o.int_add(Some(&three), &four).unwrap().as_ref(), "7");
        // `incr` of an unset variable starts at 0: None + 4 == 4.
        assert_eq!(o.int_add(None, &four).unwrap().as_ref(), "4");
    }

    #[test]
    fn int_add_overflow_is_reported() {
        let mut o = Strs;
        let max = o.new_int(i64::MAX);
        let one = o.new_int(1);
        // i64::MAX + 1 overflows the fixed-width tower.
        assert_eq!(
            o.int_add(Some(&max), &one),
            Err(ValueError::IntegerOverflow)
        );
    }

    #[test]
    fn int_add_propagates_coercion_error() {
        let mut o = Strs;
        let bad = o.new_str("zzz");
        let one = o.new_int(1);
        assert!(matches!(
            o.int_add(Some(&bad), &one),
            Err(ValueError::NotInteger(_))
        ));
    }

    #[test]
    fn list_len_index_append_defaults() {
        let mut o = Strs;
        let list = o.new_str("a b c");
        // llength {a b c} == 3.
        assert_eq!(o.list_len(&list).unwrap(), 3);
        // lindex {a b c} 1 == b; out-of-range is None.
        assert_eq!(o.list_index(&list, 1).unwrap().unwrap().as_ref(), "b");
        assert!(o.list_index(&list, 9).unwrap().is_none());
        // lappend {a b} c == "a b c".
        let two = o.new_str("a b");
        let c = o.new_str("c");
        assert_eq!(o.list_append(two, c).unwrap().as_ref(), "a b c");
    }

    #[test]
    fn dict_pairs_even_odd_and_dedup() {
        let mut o = Strs;
        // Even-length list → pairs in first-occurrence order.
        let d = o.new_str("a 1 b 2");
        let pairs = o.dict_pairs(&d).unwrap();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].0.as_ref(), "a");
        assert_eq!(pairs[0].1.as_ref(), "1");
        assert_eq!(pairs[1].0.as_ref(), "b");
        // Duplicate key: last value wins, original position kept (tclsh
        // `dict create a 1 a 2` → `a 2`).
        let dup = o.new_str("a 1 a 2");
        let dp = o.dict_pairs(&dup).unwrap();
        assert_eq!(dp.len(), 1);
        assert_eq!(dp[0].0.as_ref(), "a");
        assert_eq!(dp[0].1.as_ref(), "2");
        // Odd-length list → the canonical "missing value to go with key" error.
        let odd = o.new_str("a 1 b");
        let error = o.dict_pairs(&odd).unwrap_err();
        assert_eq!(error, ValueError::MissingDictionaryValue);
        assert_eq!(error.message_bytes(), b"missing value to go with key");
    }

    #[test]
    fn new_dict_interleaves_pairs() {
        let mut o = Strs;
        let pairs = vec![
            (o.new_str("k1"), o.new_str("v1")),
            (o.new_str("k2"), o.new_str("v2")),
        ];
        assert_eq!(o.new_dict(pairs).as_ref(), "k1 v1 k2 v2");
    }

    #[test]
    fn bytes_defaults_round_trip_utf8() {
        let mut o = Strs;
        let v = o.new_str("abc");
        assert_eq!(o.as_bytes(&v).as_ref(), b"abc");
        // new_bytes routes through new_string (UTF-8).
        assert_eq!(o.new_bytes(b"xyz").as_ref(), "xyz");
    }

    #[test]
    fn cow_escape_hatches_default_false() {
        let mut o = Strs;
        let mut v = o.new_str("a");
        let extra = o.new_str("b");
        assert!(!o.try_append_bytes_in_place(&mut v, b"bc"));
        let mut list = o.new_str("a b");
        assert!(!o.try_list_append_in_place(&mut list, &extra).unwrap());
    }

    #[test]
    fn value_error_messages_match_tclsh() {
        // Verified against tclsh8.6 / tclsh9.0 (identical):
        //   incr of non-int       → expected integer but got "zzz"
        //   double("xx")          → expected floating-point number but got "xx"
        //   if {"notbool"}        → expected boolean value but got "notbool"
        //   llength "{"           → unmatched open brace in list
        //   i64 overflow          → integer value too large to represent
        assert_eq!(
            ValueError::NotInteger("zzz".to_string()).message(),
            "expected integer but got \"zzz\""
        );
        assert_eq!(
            ValueError::NotDouble("xx".to_string()).message(),
            "expected floating-point number but got \"xx\""
        );
        assert_eq!(
            ValueError::NotBoolean("notbool".to_string()).message(),
            "expected boolean value but got \"notbool\""
        );
        assert_eq!(
            ValueError::BadList("unmatched open brace in list".to_string()).message(),
            "unmatched open brace in list"
        );
        assert_eq!(
            ValueError::IntegerOverflow.message(),
            "integer value too large to represent"
        );
        // Display routes through message().
        assert_eq!(
            format!("{}", ValueError::IntegerOverflow),
            "integer value too large to represent"
        );
    }
}
