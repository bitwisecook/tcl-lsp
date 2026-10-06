// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native C variable-name primary shapes, distinct from variable table keys.

use tcl_dialect::TclVersion;

/// Pinned C variable-name cache recipe. Live procedure/name objects belong to
/// the concrete interpreter; this value supplies no variable receiver authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeVariableNameProtocol {
    version: TclVersion,
}

/// The actual caller of an original-object variable lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeVariableNameLookupPurpose {
    /// Read without creating an undefined entry.
    Read,
    /// Write, creating root and element entries before observers.
    Write,
    /// Original write with native flags zero: observers run but variable errors
    /// do not publish a getter, result or trace frame.
    QuietWrite,
    /// Make an alias to an existing or newly created undefined entry.
    Link,
    /// Namespace declaration creates its root but never an array element.
    Define,
    /// Remove the selected entry without creating a missing entry.
    Unset,
    /// Original unset with native flags zero; lookup errors leave no message.
    QuietUnset,
}

impl NativeVariableNameLookupPurpose {
    /// Whether this lookup creates undefined variable entries.
    #[must_use]
    pub const fn creates_entries(self) -> bool {
        matches!(
            self,
            Self::Write | Self::QuietWrite | Self::Link | Self::Define
        )
    }
    /// Root creation and element creation are independent native flags.
    #[must_use]
    pub const fn creates_element_entries(self) -> bool {
        self.creates_entries() && !matches!(self, Self::Define)
    }
    /// Whether this caller requested the native variable-error presenter.
    #[must_use]
    pub const fn leaves_error_message(self) -> bool {
        !matches!(self, Self::QuietWrite | Self::QuietUnset)
    }
    /// Native lookup diagnostic verb, independent of later value observers.
    #[must_use]
    pub const fn diagnostic_verb(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write | Self::QuietWrite => "set",
            Self::Link => "access",
            Self::Define => "define",
            Self::Unset | Self::QuietUnset => "unset",
        }
    }
}

/// Original parts retained by a preparsed array name.
#[derive(Clone)]
pub enum NativeParsedVariableElement<V> {
    /// C8 stores an allocated byte buffer; copying duplicates its `CString` extent.
    Bytes(Box<[u8]>),
    /// C9 stores and retains the original counted element object.
    Object(V),
}

/// Actual key producer selected after parsing an original combined array name.
pub enum NativeElementTableKey<'a, V> {
    /// A legacy `CString` index creates a fresh native string header.
    FreshString(&'a [u8]),
    /// Counted parser storage already owns the original native index header.
    Original(&'a V),
}

/// A parsed name owns parts, never a selected variable or namespace.
#[derive(Clone)]
pub struct NativeParsedVariableName<V> {
    /// Original release that produced this primary.
    pub protocol: NativeVariableNameProtocol,
    /// None is the native scalar NULL/NULL primary.
    pub array: Option<(V, NativeParsedVariableElement<V>)>,
}

/// An indexed local name retains either the actual C8.4 procedure or a later
/// canonical original name. The concrete owner authenticates both identities.
#[derive(Clone)]
pub enum NativeLocalVariableOwner<P, V> {
    /// C8.4 owns its procedure, independently of the current activation.
    Procedure(P),
    /// Later C caches own the canonical name; None is its native self-cache.
    Name(Option<V>),
}

/// A native indexed lookup descriptor, never a byte-derived variable receiver.
#[derive(Clone)]
pub struct NativeLocalVariableName<P, V> {
    /// Actual producing release.
    pub protocol: NativeVariableNameProtocol,
    /// Actual compiled local index.
    pub index: usize,
    /// The native retained procedure or canonical original name.
    pub owner: NativeLocalVariableOwner<P, V>,
}

impl NativeVariableNameProtocol {
    /// Exact pure recipe for a supported C release, without authenticating a host.
    #[must_use]
    pub const fn for_tcl_version(version: TclVersion) -> Self {
        Self { version }
    }
    /// The physical descriptor's retained release.
    #[must_use]
    pub const fn version(self) -> TclVersion {
        self.version
    }
    /// Native simple scalar lookup for an alias local, without cache adoption.
    #[must_use]
    pub fn alias_local_input(self, original: &[u8]) -> crate::naming::NativeNameProjection<'_> {
        crate::naming::NativeNameProtocol::C(self.version).variable_alias_local_input(original)
    }

    /// `ObjMakeUpvar` checks array shape through its `CString` before the counted
    /// simple-name lookup. Raw bytes beyond the first NUL do not affect this test.
    #[must_use]
    pub fn alias_local_is_element(self, original: &[u8]) -> bool {
        crate::naming::NativeNameProtocol::C(self.version)
            .combined_variable_input(tcl_core_types::c_string_extent(original))
            .element()
            .is_some()
    }
    /// C85+ inspect matching local and parsed caches before requiring name bytes.
    /// C84 obtains its root string before the local-cache comparison.
    #[must_use]
    pub const fn cache_precedes_string_getter(self) -> bool {
        !matches!(self.version, TclVersion::V8_4)
    }

    /// C85+ variable diagnostics use the native String result producer;
    /// C84 retains the legacy NULL primary produced by `Tcl_AppendResult`.
    #[must_use]
    pub const fn diagnostic_string_protocol(
        self,
    ) -> Option<crate::native_string::NativeStringProtocol> {
        match self.version {
            TclVersion::V8_4 => None,
            _ => Some(crate::native_string::NativeStringProtocol::C(self.version)),
        }
    }

    /// Tcl regexp/regsub pass flags zero through C85 and `LEAVE_ERR_MSG` later.
    #[must_use]
    pub const fn regex_write_purpose(self) -> NativeVariableNameLookupPurpose {
        match self.version {
            TclVersion::V8_4 | TclVersion::V8_5 => NativeVariableNameLookupPurpose::QuietWrite,
            _ => NativeVariableNameLookupPurpose::Write,
        }
    }

    /// Physical free-slot selection before a simple-name receiver lookup.
    /// C8.4 only clears descriptors with a free hook; later C8 clears all.
    #[must_use]
    pub const fn retires_before_simple_lookup(self, has_free_hook: bool) -> bool {
        match self.version {
            TclVersion::V8_4 => has_free_hook,
            TclVersion::V8_5 | TclVersion::V8_6 => true,
            TclVersion::V9_0 | TclVersion::V9_1 => false,
        }
    }
    /// Hash entry key birth from actual parser storage, independent of the
    /// variable-name primary's retained bytes or object.
    #[must_use]
    pub fn parsed_element_table_key<V>(
        self,
        element: &NativeParsedVariableElement<V>,
    ) -> Option<NativeElementTableKey<'_, V>> {
        match (self.version, element) {
            (TclVersion::V8_5 | TclVersion::V8_6, NativeParsedVariableElement::Bytes(bytes)) => {
                let end = bytes
                    .iter()
                    .position(|byte| *byte == 0)
                    .unwrap_or(bytes.len());
                Some(NativeElementTableKey::FreshString(&bytes[..end]))
            }
            (
                TclVersion::V9_0 | TclVersion::V9_1,
                NativeParsedVariableElement::Object(original),
            ) => Some(NativeElementTableKey::Original(original)),
            _ => None,
        }
    }
    /// Separate element operands are already the actual table's object keys.
    #[must_use]
    pub const fn element_table_retains_original(self) -> bool {
        !matches!(self.version, TclVersion::V8_4)
    }
    /// C84 clears the undefined flag after its element unset trace; later C
    /// clears it before that callback while retaining the table entry.
    #[must_use]
    pub const fn element_unset_preserves_definition_during_trace(self) -> bool {
        matches!(self.version, TclVersion::V8_4)
    }
    /// C9 preparsed names have no string updater. C8 only updates array names.
    #[must_use]
    pub const fn has_parsed_array_updater(self) -> bool {
        matches!(
            self.version,
            TclVersion::V8_4 | TclVersion::V8_5 | TclVersion::V8_6
        )
    }
    /// C8.4's indexed cache owns its actual procedure; later caches own names.
    #[must_use]
    pub const fn local_cache_owns_procedure(self) -> bool {
        matches!(self.version, TclVersion::V8_4)
    }
    /// C9 also installs the canonical local-name object's self cache.
    #[must_use]
    pub const fn installs_canonical_local_self_cache(self) -> bool {
        matches!(self.version, TclVersion::V9_0 | TclVersion::V9_1)
    }

    /// Original counted parser parts before any table-key or diagnostic projection.
    #[must_use]
    pub fn parsed_array_parts(self, original: &[u8]) -> Option<(&[u8], &[u8])> {
        let parsed =
            crate::naming::NativeNameProtocol::C(self.version).combined_variable_input(original);
        let element = parsed.element()?;
        Some((parsed.root().original(), element.original()))
    }
    /// C8 stores the complete original element buffer on first parsing.
    #[must_use]
    pub fn new_element_bytes(self, original: &[u8]) -> Option<Box<[u8]>> {
        self.has_parsed_array_updater().then(|| original.into())
    }
    /// Native C8 duplicate uses strlen rather than the original allocation length.
    #[must_use]
    pub fn duplicate_element_bytes(self, original: &[u8]) -> Option<Box<[u8]>> {
        self.has_parsed_array_updater()
            .then(|| tcl_core_types::c_string_extent(original).into())
    }
    /// C8 array-name updater uses counted root bytes and `CString` element bytes.
    #[must_use]
    pub fn parsed_array_string(self, root: &[u8], element: &[u8]) -> Option<Vec<u8>> {
        self.has_parsed_array_updater()
            .then(|| [root, b"(", tcl_core_types::c_string_extent(element), b")"].concat())
    }
}

impl<V: Clone> NativeParsedVariableName<V> {
    /// Duplicate native retained parts, including C8's allocated `CString` copy.
    #[must_use]
    pub fn duplicate(&self) -> Option<Self> {
        Some(Self {
            protocol: self.protocol,
            array: self
                .array
                .as_ref()
                .map(|(root, element)| {
                    Some((
                        root.clone(),
                        match element {
                            NativeParsedVariableElement::Bytes(bytes) => {
                                NativeParsedVariableElement::Bytes(
                                    self.protocol.duplicate_element_bytes(bytes)?,
                                )
                            }
                            NativeParsedVariableElement::Object(object)
                                if !self.protocol.has_parsed_array_updater() =>
                            {
                                NativeParsedVariableElement::Object(object.clone())
                            }
                            NativeParsedVariableElement::Object(_) => return None,
                        },
                    ))
                })
                .map_or(Some(None), |value| value.map(Some))?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn alias_local_shape_and_lookup_keep_their_native_extents() {
        for version in TclVersion::ALL {
            let protocol = NativeVariableNameProtocol::for_tcl_version(version);
            assert!(!protocol.alias_local_is_element(b"n\0(k)"));
            assert!(protocol.alias_local_is_element(b"n(k)\0tail"));
            let input = protocol.alias_local_input(b"n\0(k)");
            assert_eq!(
                input.purpose(),
                crate::naming::NativeNamePurpose::VariableAliasLocal
            );
            assert_eq!(
                input.selected(),
                if version == TclVersion::V8_4 {
                    b"n".as_slice()
                } else {
                    b"n\0(k)"
                }
            );
            assert_eq!(
                input.qualification(),
                crate::naming::NativeNameQualification::Unqualified
            );
        }
    }

    #[test]
    fn parsed_cache_parts_and_duplicate_extents_are_separate_from_keys() {
        for version in TclVersion::ALL {
            let protocol = NativeVariableNameProtocol::for_tcl_version(version);
            assert_eq!(
                protocol.cache_precedes_string_getter(),
                version != TclVersion::V8_4
            );
            assert_eq!(
                protocol.parsed_array_parts(b"a(k\0z)"),
                Some((b"a".as_slice(), b"k\0z".as_slice()))
            );
            assert_eq!(
                protocol.duplicate_element_bytes(b"k\0z").as_deref(),
                (version < TclVersion::V9_0).then_some(b"k".as_slice())
            );
            assert_eq!(
                protocol.parsed_array_parts(b"a\0z(k)"),
                (version < TclVersion::V9_0).then_some((b"a\0z".as_slice(), b"k".as_slice()))
            );
            assert_eq!(
                protocol.has_parsed_array_updater(),
                version < TclVersion::V9_0
            );
        }
    }
}
