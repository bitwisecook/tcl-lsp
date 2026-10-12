// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native append shape rules independent of object mutation and engine authority.
use crate::native_object::{
    NativeObjectCacheSnapshot as Cache, NativeObjectShapeUnavailable, NativeObjectSnapshot,
    NativeObjectStringEmptiness, native_c9_string_emptiness,
};
use crate::native_string::{NativeStringProtocol, NativeStringStorageIdentity};
use tcl_dialect::TclVersion;

/// Reached object operation, independent of variable publication and callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeObjectAppendAction {
    /// Missing C variable adopts the exact original source object.
    AdoptSource,
    /// Missing Jim variable constructs a string from the original source bytes.
    NewString,
    /// The source's selected native empty receipt leaves the receiver intact.
    PreserveReceiver,
    /// Duplicate the original source representation into the existing receiver.
    DuplicateSource,
    /// Append actual binary backing without native string materialisation.
    AppendBinary,
    /// Reach the selected String conversion and native unit/count append rules.
    AppendString,
}

/// Missing physical information required by an append action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeObjectAppendUnavailable {
    /// Canonical empty storage identity is unknown at a reached native branch.
    EmptyStorageIdentity,
    /// Snapshot fields cannot describe an original physical object.
    Shape(NativeObjectShapeUnavailable),
}

/// Pure native append shape recipe; engine authority belongs to its issuer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeObjectAppendProtocol {
    string: NativeStringProtocol,
}

impl NativeObjectAppendProtocol {
    /// Select a pure string recipe; this does not authenticate a native engine.
    #[must_use]
    pub const fn for_string_protocol(string: NativeStringProtocol) -> Self {
        Self { string }
    }

    /// Original native string recipe; this pure descriptor supplies no authority.
    #[must_use]
    pub const fn string_protocol(self) -> NativeStringProtocol {
        self.string
    }

    /// C8.4/8.5 convert the receiver before a counted zero-length append returns.
    /// Later releases return before reaching its primary representation.
    #[must_use]
    // Native proof: naming.namespace.original-counted-tail-compiler-and-runtime
    // docs/design/analysis/name-resolution-proofs/original-counted-tail-compiler-and-runtime.md
    pub const fn converts_empty_counted_receiver(self) -> bool {
        // naming.object.original-empty-counted-append
        // docs/design/analysis/name-resolution-proofs/original-empty-counted-append.md
        matches!(
            self.string.tcl_version(),
            Some(TclVersion::V8_4 | TclVersion::V8_5)
        )
    }

    /// C9 uses proper binary backing even when original string bytes are resident.
    #[must_use]
    pub fn binary_eligible(self, value: &NativeObjectSnapshot) -> bool {
        match (&value.cache, self.string.tcl_version()) {
            (Cache::ByteArray { .. }, Some(TclVersion::V8_6)) => value.resident.is_none(),
            (Cache::ByteArray { proper, .. }, Some(TclVersion::V9_0 | TclVersion::V9_1)) => *proper,
            _ => false,
        }
    }

    /// C9 forces the receiver to Unicode when the source starts with a continuation byte.
    #[must_use]
    pub fn forces_unicode_for_source(self, source: &[u8]) -> bool {
        self.string
            .tcl_version()
            .is_some_and(|version| version >= TclVersion::V9_0)
            && source.first().is_some_and(|byte| byte & 0xc0 == 0x80)
    }

    /// C8.4/8.5 retain a combined count only for one-byte source String units.
    #[must_use]
    pub fn combines_character_counts(self, source_count: usize, source_bytes: usize) -> bool {
        self.string
            .tcl_version()
            .is_some_and(|version| version >= TclVersion::V8_6 || source_count == source_bytes)
    }

    /// Choose the native action before object string generation or mutation.
    /// Receiver copy-on-write and variable observers belong to concrete owners.
    ///
    /// # Errors
    /// Refuses physical storage identity needed by a reached native branch.
    pub fn action(
        self,
        receiver: Option<&NativeObjectSnapshot>,
        source: &NativeObjectSnapshot,
    ) -> Result<NativeObjectAppendAction, NativeObjectAppendUnavailable> {
        use NativeObjectAppendAction::{
            AdoptSource, AppendBinary, AppendString, DuplicateSource, NewString, PreserveReceiver,
        };
        let Some(receiver) = receiver else {
            return Ok(if self.string.is_jim084() {
                NewString
            } else {
                AdoptSource
            });
        };
        let Some(version) = self.string.tcl_version() else {
            return Ok(AppendString);
        };
        if version >= TclVersion::V9_0 {
            if native_c9_string_emptiness(source).map_err(NativeObjectAppendUnavailable::Shape)?
                == NativeObjectStringEmptiness::Empty
            {
                return Ok(PreserveReceiver);
            }
            if native_c9_string_emptiness(receiver).map_err(NativeObjectAppendUnavailable::Shape)?
                == NativeObjectStringEmptiness::Empty
            {
                return Ok(DuplicateSource);
            }
        } else if version == TclVersion::V8_6 {
            if source.storage == Some(NativeStringStorageIdentity::CanonicalEmpty) {
                return Ok(PreserveReceiver);
            }
            if source.storage == Some(NativeStringStorageIdentity::Unknown)
                && source
                    .resident
                    .as_ref()
                    .is_some_and(|bytes| bytes.is_empty())
            {
                return Err(NativeObjectAppendUnavailable::EmptyStorageIdentity);
            }
        }
        if self.binary_eligible(source) {
            if self.binary_eligible(receiver)
                || receiver.storage == Some(NativeStringStorageIdentity::CanonicalEmpty)
            {
                return Ok(AppendBinary);
            }
            if receiver.storage == Some(NativeStringStorageIdentity::Unknown)
                && receiver
                    .resident
                    .as_ref()
                    .is_some_and(|bytes| bytes.is_empty())
            {
                return Err(NativeObjectAppendUnavailable::EmptyStorageIdentity);
            }
        }
        Ok(AppendString)
    }
}

/// Native C9 concatenation representation, selected before any materialisation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeObjectCatMode {
    /// Proper binary payloads and known resident empty strings.
    Binary,
    /// Requested or forced native Unicode units.
    Unicode,
    /// Native string spelling with deferred updater evaluation.
    Bytes,
}

/// Pure `TclStringCat` recipe. Construction supplies no actual engine authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeObjectCatProtocol {
    string: NativeStringProtocol,
}
impl NativeObjectCatProtocol {
    /// Select only releases providing the audited C9 concatenation implementation.
    #[must_use]
    pub fn for_string_protocol(string: NativeStringProtocol) -> Option<Self> {
        string
            .tcl_version()
            .filter(|version| *version >= TclVersion::V9_0)?;
        Some(Self { string })
    }
    /// The separately retained native string recipe.
    #[must_use]
    pub const fn string_protocol(self) -> NativeStringProtocol {
        self.string
    }
    /// Select the native analysis branch without generating any operand string.
    #[must_use]
    pub fn mode(self, inputs: &[NativeObjectSnapshot]) -> NativeObjectCatMode {
        let mut binary = true;
        let mut allow_unicode = true;
        let mut request_unicode = false;
        let mut force_unicode = false;
        for (index, input) in inputs.iter().enumerate() {
            if matches!(input.cache, Cache::ByteArray { proper: true, .. }) {
                allow_unicode = false;
            } else if let Some(bytes) = input.resident.as_ref() {
                if !bytes.is_empty() {
                    binary = false;
                    if index > 0 && matches!(bytes[0], 0x80..=0xbf) {
                        force_unicode = true;
                    } else if !matches!(input.cache, Cache::None | Cache::String { .. }) {
                        allow_unicode = false;
                    }
                }
            } else {
                binary = false;
                if matches!(input.cache, Cache::String { .. }) {
                    request_unicode = true;
                } else {
                    allow_unicode = false;
                }
            }
            if !binary && !allow_unicode {
                break;
            }
        }
        if binary {
            NativeObjectCatMode::Binary
        } else if (allow_unicode && request_unicode) || force_unicode {
            NativeObjectCatMode::Unicode
        } else {
            NativeObjectCatMode::Bytes
        }
    }
}

/// Readonly data result for independently known source string components.
/// This is not the physical TclStringCat/append owner: representation-sensitive
/// non-ASCII or NUL cases require that owner and are deliberately unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceStringConcatenationProtocol {
    string: NativeStringProtocol,
}

impl SourceStringConcatenationProtocol {
    /// The data recipe is selected independently of engine or object authority.
    #[must_use]
    pub const fn for_string_protocol(string: NativeStringProtocol) -> Self {
        Self { string }
    }

    /// ASCII non-NUL source strings have the same data concatenation under each
    /// selected C/Jim recipe. Unknown binary/Unicode primaries cannot borrow it.
    #[must_use]
    pub fn concatenate(self, left: &[u8], right: &[u8]) -> Option<Vec<u8>> {
        match self.string {
            NativeStringProtocol::C(_) | NativeStringProtocol::Jim084 => {}
        }
        if left
            .iter()
            .chain(right)
            .any(|byte| !byte.is_ascii() || *byte == 0)
        {
            return None;
        }
        let mut bytes = Vec::with_capacity(left.len().checked_add(right.len())?);
        bytes.extend_from_slice(left);
        bytes.extend_from_slice(right);
        Some(bytes)
    }
}
