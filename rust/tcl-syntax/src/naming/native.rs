// SPDX-License-Identifier: AGPL-3.0-or-later
//! Purpose-specific native name inputs and reporting.
//!
//! A projection preserves original operands and selects an operation's byte
//! extent. It grants no namespace/frame identity, receiver lifetime, alias
//! closure, command generation, compiler permission or observer proof.

use super::{ends_with_separator, is_qualified, qualifier_segments, written_command_tail};
use crate::native_string::NativeStringProtocol;
use std::borrow::Cow;
use tcl_core_types::{ByteCommandSlot, ByteNamespacePath, NameBytes, c_string_extent};
use tcl_dialect::{
    TclVersion,
    model::{BuildProfileId, DialectPoint, Family, Release},
};

/// Audited native name recipe, independent of numeric engine availability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeNameProtocol {
    C(TclVersion),
    Jim084,
}

/// The issuer of a pure naming policy, distinct from live native attestation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NamePolicyAuthority {
    Native,
    AuthoredSimulation,
}

/// Explicit naming provider retained at capability ingress and in cache keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NamePolicyProtocol {
    recipe: NativeNameProtocol,
    authority: NamePolicyAuthority,
}
impl NamePolicyProtocol {
    /// Select an actual audited engine/build point, without vendor compatibility.
    #[must_use]
    pub fn for_native_point(point: DialectPoint) -> Option<Self> {
        NativeNameProtocol::for_point(point).map(|recipe| Self {
            recipe,
            authority: NamePolicyAuthority::Native,
        })
    }
    /// An authored logical C simulation; does not authenticate its physical host.
    #[must_use]
    pub const fn authored_tcl(version: TclVersion) -> Self {
        Self {
            recipe: NativeNameProtocol::C(version),
            authority: NamePolicyAuthority::AuthoredSimulation,
        }
    }
    /// An authored Jim 0.84 naming simulation; does not authenticate a host.
    #[must_use]
    pub const fn authored_jim084() -> Self {
        Self {
            recipe: NativeNameProtocol::Jim084,
            authority: NamePolicyAuthority::AuthoredSimulation,
        }
    }
    /// The selected pure recipe.
    #[must_use]
    pub const fn recipe(self) -> NativeNameProtocol {
        self.recipe
    }
    /// Provider authority, retained separately from source and physical engine.
    #[must_use]
    pub const fn authority(self) -> NamePolicyAuthority {
        self.authority
    }
    /// The recipe for this provider's native string materialisation.
    #[must_use]
    pub const fn string_protocol(self) -> NativeStringProtocol {
        self.recipe.string_protocol()
    }
}

/// Dictionary failure operation; key equality remains counted and independent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeDictionaryMissingKeyOperation {
    /// A missing key while reading a dictionary value.
    Get,
    /// A missing intermediate key during dictionary removal.
    UnsetIntermediate,
}

/// Native missing-key reporting, separate from dictionary key identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDictionaryMissingKeyError {
    /// Counted diagnostic bytes after the operation's native `CString` formatting.
    pub message: Vec<u8>,
    /// Explicit native tuple update, if the operation supplies one.
    pub error_code: Option<Vec<u8>>,
}

/// Format an audited dictionary failure without changing the original key.
/// C8.4 has no dictionary command and cannot donate a reporting recipe.
///
/// # Errors
/// Returns an unavailable purpose for C8.4.
pub fn report_native_dictionary_missing_key(
    protocol: NativeNameProtocol,
    operation: NativeDictionaryMissingKeyOperation,
    original: &[u8],
) -> Result<NativeDictionaryMissingKeyError, NameProjectionUnavailable> {
    if protocol == NativeNameProtocol::C(TclVersion::V8_4) {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    }
    let reported = c_string_extent(original);
    let mut message = b"key \"".to_vec();
    message.extend_from_slice(reported);
    message.extend_from_slice(b"\" not known in dictionary");
    let explicit_lookup = matches!(protocol, NativeNameProtocol::C(version) if version >= TclVersion::V9_0)
        || (!protocol.is_jim084()
            && operation == NativeDictionaryMissingKeyOperation::UnsetIntermediate);
    let error_code = explicit_lookup.then(|| {
        let mut tuple = b"TCL LOOKUP DICT ".to_vec();
        crate::list::append_list_element(&mut tuple, reported, false);
        tuple
    });
    Ok(NativeDictionaryMissingKeyError {
        message,
        error_code,
    })
}

/// Operation whose operand extent is selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeNamePurpose {
    CommandLookup,
    CommandPublication,
    CommandCApiPublication,
    RenameSource,
    RenameDestination,
    AliasPublication,
    NamespaceAddress,
    /// Jim helper canonicalisation before flat command enumeration.
    JimNamespaceCanonical,
    /// Namespace ensemble command publication, independently of proc creation.
    EnsemblePublication,
    /// Namespace command's native subcommand index operand.
    NamespaceSubcommand,
    /// Interpreter dispatch table index, separate from its command operand.
    InterpreterSubcommand,
    /// Native namespace-variable query extent, independent of scalar lookup.
    NamespaceVariableQuery,
    VariableRoot,
    /// Simple alias-local lookup without original-name cache conversion.
    VariableAliasLocal,
    ArrayElementCombined,
    ArrayElementSeparated,
    VariableTraceRegistration,
    VariableTraceQuery,
    PackageName,
    FormalEnumeration,
    FormalStorage,
    NamespaceUpvarLocal,
    NamespaceExportPattern,
    NamespaceImportPattern,
    NamespaceForgetPattern,
    HiddenToken,
    /// `TclOO` counted method-table key, separate from command publication.
    OoMethod,
    /// `TclOO` object command declaration in its actual namespace context.
    OoObjectPublication,
}

/// Qualification visible to the selected native operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeNameQualification {
    Unqualified,
    Relative,
    Absolute,
}

/// Exact source namespace of a purpose-selected namespace pattern.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NativeNamespacePatternSource {
    /// C namespace component geometry, independent of its display name.
    C(ByteNamespacePath),
    /// Jim's retained flat namespace-object bytes.
    Jim(NameBytes),
}

/// Original pattern extent, source namespace and final counted pattern tail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeNamespacePatternParts {
    pub original: NameBytes,
    pub selected: NameBytes,
    pub source: Option<NativeNamespacePatternSource>,
    pub tail: NameBytes,
    pub purpose: NativeNamePurpose,
}

/// Actual current namespace context; constructed paths must not be reparsed.
#[derive(Debug, Clone, Copy)]
pub struct NativeNameContext<'a> {
    pub namespace: &'a ByteNamespacePath,
    /// Jim's actual namespace object, which has its own flat composition rules.
    pub jim_namespace_object: Option<&'a [u8]>,
}
static ROOT_PATH: ByteNamespacePath = ByteNamespacePath::root();
impl<'a> NativeNameContext<'a> {
    /// A C namespace context without manufacturing a Jim namespace object.
    #[must_use]
    pub const fn new(namespace: &'a ByteNamespacePath) -> Self {
        Self {
            namespace,
            jim_namespace_object: None,
        }
    }
    /// Retain the actual Jim namespace object alongside its analytical path.
    #[must_use]
    pub const fn with_jim_namespace(namespace: &'a ByteNamespacePath, original: &'a [u8]) -> Self {
        Self {
            namespace,
            jim_namespace_object: Some(original),
        }
    }
    /// Both engines' root context.
    #[must_use]
    pub const fn root() -> NativeNameContext<'static> {
        NativeNameContext {
            namespace: &ROOT_PATH,
            jim_namespace_object: Some(b""),
        }
    }
}

/// Original-object construction selected by Jim's namespace canonicaliser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeJimNamespaceConstruction {
    /// Return the original relative operand when the actual namespace is root.
    RetainOriginal,
    /// Construct a fresh string from the selected absolute `CString` suffix.
    FreshString,
    /// Duplicate the actual namespace object, then append the separator and operand.
    DuplicateNamespaceAndAppend,
}

/// Select the original invoked name for C8.4 procedure compilation context.
/// The caller applies its native UTF byte-budget trimming after this extent.
///
/// # Errors
/// Refuses borrowing this C8.4 diagnostic recipe for another native release.
pub fn native_procedure_compilation_name_input(
    protocol: NativeNameProtocol,
    original: &[u8],
) -> Result<&[u8], NameProjectionUnavailable> {
    match protocol {
        NativeNameProtocol::C(TclVersion::V8_4) => Ok(c_string_extent(original)),
        _ => Err(NameProjectionUnavailable::PurposeNotModelled),
    }
}

/// Unsupported policy/context, rather than a native guest failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameProjectionUnavailable {
    MissingJimNamespaceObject,
    PurposeNotModelled,
}

impl std::fmt::Display for NameProjectionUnavailable {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::MissingJimNamespaceObject => "native Jim namespace object is unavailable",
            Self::PurposeNotModelled => "native name purpose is not modelled for this engine",
        })
    }
}
impl std::error::Error for NameProjectionUnavailable {}

/// Original operand and its purpose-selected input; neither is a live identity.
#[derive(Debug, Clone)]
pub struct NativeNameProjection<'a> {
    original: &'a [u8],
    selected: Cow<'a, [u8]>,
    protocol: NativeNameProtocol,
    purpose: NativeNamePurpose,
    qualification: NativeNameQualification,
    context: Option<NativeNameContext<'a>>,
}
impl<'a> NativeNameProjection<'a> {
    #[must_use]
    pub const fn original(&self) -> &'a [u8] {
        self.original
    }
    #[must_use]
    pub fn selected(&self) -> &[u8] {
        &self.selected
    }
    #[must_use]
    pub const fn protocol(&self) -> NativeNameProtocol {
        self.protocol
    }
    #[must_use]
    pub const fn purpose(&self) -> NativeNamePurpose {
        self.purpose
    }
    #[must_use]
    pub const fn qualification(&self) -> NativeNameQualification {
        self.qualification
    }
    #[must_use]
    pub const fn context(&self) -> Option<NativeNameContext<'_>> {
        self.context
    }
    /// Checked analytical Unicode view of the selected bytes, with no loss.
    ///
    /// # Errors
    /// Returns an error when a retained native component is not valid Rust UTF-8.
    pub fn try_utf8(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.selected)
    }
    /// Jim command table comparison key; stored publication spelling remains
    /// available separately in `selected`, including redundant root markers.
    #[must_use]
    pub fn jim_flat_key(&self) -> Option<&[u8]> {
        (self.protocol == NativeNameProtocol::Jim084 && is_command_purpose(self.purpose)).then(
            || {
                if self.purpose == NativeNamePurpose::AliasPublication {
                    self.selected.as_ref()
                } else {
                    strip_jim_root(&self.selected)
                }
            },
        )
    }
}

/// Original variable input form, needed for parsing and trace reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeVariableInputForm<'a> {
    Combined(&'a [u8]),
    Separate {
        root: &'a [u8],
        element: Option<&'a [u8]>,
    },
}
/// Selected root and optional element, without resolving an alias or cell.
#[derive(Debug, Clone)]
pub struct NativeVariableProjection<'a> {
    original: NativeVariableInputForm<'a>,
    root: NativeNameProjection<'a>,
    element: Option<NativeNameProjection<'a>>,
}
impl<'a> NativeVariableProjection<'a> {
    #[must_use]
    pub const fn original(&self) -> NativeVariableInputForm<'a> {
        self.original
    }
    #[must_use]
    pub const fn root(&self) -> &NativeNameProjection<'a> {
        &self.root
    }
    #[must_use]
    pub const fn element(&self) -> Option<&NativeNameProjection<'a>> {
        self.element.as_ref()
    }
}

impl NativeNameProtocol {
    /// Namespace subcommand indexing consumes `CString` bytes in the audited
    /// C and Jim dispatch tables, independently from the original argv object.
    #[must_use]
    pub fn namespace_subcommand_input(self, original: &[u8]) -> NativeNameProjection<'_> {
        projection(
            self,
            NativeNamePurpose::NamespaceSubcommand,
            original,
            Cow::Borrowed(c_string_extent(original)),
            None,
        )
    }
    /// Interpreter subcommand indexing consumes the native `CString` extent;
    /// the original typed argv remains available for invocation and reporting.
    #[must_use]
    pub fn interpreter_subcommand_input(self, original: &[u8]) -> NativeNameProjection<'_> {
        projection(
            self,
            NativeNamePurpose::InterpreterSubcommand,
            original,
            Cow::Borrowed(c_string_extent(original)),
            None,
        )
    }

    /// Pure recipe selection; does not authenticate a native command or object.
    #[must_use]
    pub const fn for_tcl_version(version: TclVersion) -> Self {
        Self::C(version)
    }
    /// Select actual engine/build identity; vendors and unknown points abstain.
    #[must_use]
    pub fn for_point(point: DialectPoint) -> Option<Self> {
        match (point.family(), point.build()) {
            (Family::Tcl, BuildProfileId::Canonical) => point.tcl_version().map(Self::C),
            (Family::Jim, BuildProfileId::Canonical | BuildProfileId::JimFull)
                if point.release() == Release::JIM_0_84 =>
            {
                Some(Self::Jim084)
            }
            _ => None,
        }
    }
    #[must_use]
    pub const fn tcl_version(self) -> Option<TclVersion> {
        match self {
            Self::C(version) => Some(version),
            Self::Jim084 => None,
        }
    }
    #[must_use]
    pub const fn is_jim084(self) -> bool {
        matches!(self, Self::Jim084)
    }
    #[must_use]
    pub const fn string_protocol(self) -> NativeStringProtocol {
        match self {
            Self::C(version) => NativeStringProtocol::C(version),
            Self::Jim084 => NativeStringProtocol::Jim084,
        }
    }

    /// Lookup input in the retained current context. Path/fallback lookup and
    /// binding selection remain runtime operations on actual namespace identities.
    ///
    /// # Errors
    /// Returns an error for a missing actual Jim namespace object or an unaudited engine operation.
    pub fn command_lookup_input<'a>(
        self,
        context: NativeNameContext<'a>,
        original: &'a [u8],
    ) -> Result<NativeNameProjection<'a>, NameProjectionUnavailable> {
        self.command_input(context, original, NativeNamePurpose::CommandLookup)
    }
    /// Script publication input, including procedure declarations.
    ///
    /// # Errors
    /// Returns an error for a missing actual Jim namespace object or an unaudited engine operation.
    pub fn command_publication_input<'a>(
        self,
        context: NativeNameContext<'a>,
        original: &'a [u8],
    ) -> Result<NativeNameProjection<'a>, NameProjectionUnavailable> {
        self.command_input(context, original, NativeNamePurpose::CommandPublication)
    }
    /// C ABI publication differs from script declarations: an unqualified name
    /// belongs to the global table; a qualified relative name uses current context.
    ///
    /// # Errors
    /// Returns an error for a missing actual Jim namespace object or an unaudited engine operation.
    pub fn command_c_api_publication_input<'a>(
        self,
        context: NativeNameContext<'a>,
        original: &'a [u8],
    ) -> Result<NativeNameProjection<'a>, NameProjectionUnavailable> {
        if self.is_jim084() {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        let context = if is_qualified(c_string_extent(original)) {
            context
        } else {
            NativeNameContext::root()
        };
        self.command_input(context, original, NativeNamePurpose::CommandCApiPublication)
    }
    ///
    /// # Errors
    /// Returns an error for a missing actual Jim namespace object or an unaudited engine operation.
    pub fn rename_source_input<'a>(
        self,
        context: NativeNameContext<'a>,
        original: &'a [u8],
    ) -> Result<NativeNameProjection<'a>, NameProjectionUnavailable> {
        self.command_input(context, original, NativeNamePurpose::RenameSource)
    }
    ///
    /// # Errors
    /// Returns an error for a missing actual Jim namespace object or an unaudited engine operation.
    pub fn rename_destination_input<'a>(
        self,
        context: NativeNameContext<'a>,
        original: &'a [u8],
    ) -> Result<NativeNameProjection<'a>, NameProjectionUnavailable> {
        self.command_input(context, original, NativeNamePurpose::RenameDestination)
    }
    /// Select the `CString` simple name reported by a refused alias rename.
    /// C8.4 reports the resolved source binding; C8.5+ reports its attempted
    /// destination. Both inputs must come from the selected binding slots,
    /// not from a command's display spelling. Jim has no C alias-loop purpose.
    #[must_use]
    pub fn rename_alias_loop_name<'a>(
        self,
        source_simple: &'a [u8],
        destination_simple: &'a [u8],
    ) -> Option<&'a [u8]> {
        match self {
            Self::C(TclVersion::V8_4) => Some(c_string_extent(source_simple)),
            Self::C(_) => Some(c_string_extent(destination_simple)),
            Self::Jim084 => None,
        }
    }
    /// Select the exact rename destination without procedure-publication
    /// qualification or display reparsing.
    ///
    /// # Errors
    /// Refuses an unavailable native operation or missing actual Jim namespace.
    pub fn rename_destination_slot(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<ByteCommandSlot, NameProjectionUnavailable> {
        slot_for_projection(&self.rename_destination_input(context, original)?)
    }
    ///
    /// # Errors
    /// Returns an error for a missing actual Jim namespace object or an unaudited engine operation.
    pub fn alias_publication_input<'a>(
        self,
        context: NativeNameContext<'a>,
        original: &'a [u8],
    ) -> Result<NativeNameProjection<'a>, NameProjectionUnavailable> {
        match self {
            Self::C(_) => {
                let context = if is_qualified(c_string_extent(original)) {
                    context
                } else {
                    NativeNameContext::root()
                };
                self.command_input(context, original, NativeNamePurpose::AliasPublication)
            }
            Self::Jim084 => Ok(projection(
                self,
                NativeNamePurpose::AliasPublication,
                original,
                Cow::Borrowed(original),
                Some(NativeNameContext::root()),
            )),
        }
    }

    /// Alias commands publish separately from procedure declarations. C keeps
    /// an unqualified name global and retains the current holder for relative
    /// qualified names. Jim registers the complete original object at its flat
    /// global table, including any original leading root marker.
    ///
    /// # Errors
    /// Refuses an unsupported native publication projection.
    pub fn alias_publication_slot(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<ByteCommandSlot, NameProjectionUnavailable> {
        if self.is_jim084() {
            return Ok(ByteCommandSlot::new(
                ByteNamespacePath::root(),
                original.into(),
            ));
        }
        slot_for_projection(&self.alias_publication_input(context, original)?)
    }

    fn command_input<'a>(
        self,
        context: NativeNameContext<'a>,
        original: &'a [u8],
        purpose: NativeNamePurpose,
    ) -> Result<NativeNameProjection<'a>, NameProjectionUnavailable> {
        let selected = match self {
            Self::C(_) => Cow::Borrowed(c_string_extent(original)),
            Self::Jim084 => {
                let namespace = context
                    .jim_namespace_object
                    .ok_or(NameProjectionUnavailable::MissingJimNamespaceObject)?;
                if namespace.is_empty() || original.starts_with(b"::") {
                    Cow::Borrowed(original)
                } else {
                    Cow::Owned(join_jim(namespace, c_string_extent(original)))
                }
            }
        };
        Ok(projection(self, purpose, original, selected, Some(context)))
    }

    /// Construct a publication slot once. Jim's primary table is flat; it is
    /// represented at root with its complete comparison key, without C reparsing.
    ///
    /// # Errors
    /// Returns an error for a missing actual Jim namespace object or an unaudited engine operation.
    pub fn command_publication_slot(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<ByteCommandSlot, NameProjectionUnavailable> {
        let slot = slot_for_projection(&self.command_publication_input(context, original)?)?;
        if matches!(self, Self::C(version) if version <= TclVersion::V8_5) {
            // TclProcObjCmd resolves its holder, renders holder->fullName plus
            // the simple name, then calls Tcl_CreateObjCommand. That C API
            // parses the constructed spelling again in these releases.
            let mut full_name = Vec::from(b"::".as_slice());
            for component in slot.namespace.as_segments() {
                full_name.extend_from_slice(component.as_bytes());
                full_name.extend_from_slice(b"::");
            }
            full_name.extend_from_slice(slot.simple.as_bytes());
            return self.command_c_api_publication_slot(NativeNameContext::root(), &full_name);
        }
        Ok(slot)
    }

    /// `TclOO` object commands select their `CString` name against the actual
    /// current holder and create missing qualifier namespaces. Method-table
    /// keys and original invocation words do not use this extent.
    ///
    /// # Errors
    /// Refuses engines without audited `TclOO` object creation.
    pub fn oo_object_publication_slot(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<ByteCommandSlot, NameProjectionUnavailable> {
        if !matches!(self, Self::C(version) if version >= TclVersion::V8_6) {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        slot_for_projection(&self.command_input(
            context,
            original,
            NativeNamePurpose::OoObjectPublication,
        )?)
    }

    /// Select a namespace ensemble command slot. The default names the actual
    /// namespace in its parent, rather than reparsing that namespace's display.
    /// Tcl 8.5 constructs and reparses a qualified command through its C API.
    ///
    /// # Errors
    /// Refuses unsupported engines or an absent actual Jim namespace object.
    pub fn ensemble_publication_slot(
        self,
        context: NativeNameContext<'_>,
        explicit: Option<&[u8]>,
    ) -> Result<ByteCommandSlot, NameProjectionUnavailable> {
        if let Some(original) = explicit {
            return self.command_publication_slot(context, original);
        }
        if self.is_jim084() {
            let original = context
                .jim_namespace_object
                .ok_or(NameProjectionUnavailable::MissingJimNamespaceObject)?;
            return Ok(ByteCommandSlot::new(
                ByteNamespacePath::root(),
                NameBytes::from(original),
            ));
        }
        if matches!(self, Self::C(version) if version < TclVersion::V8_5) {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        if self == Self::C(TclVersion::V8_5) {
            let mut written = b"::".to_vec();
            for (index, component) in context.namespace.as_segments().iter().enumerate() {
                if index != 0 {
                    written.extend_from_slice(b"::");
                }
                written.extend_from_slice(component.as_bytes());
            }
            return self.command_c_api_publication_slot(NativeNameContext::root(), &written);
        }
        let mut parent = context.namespace.clone();
        let simple = parent.pop().unwrap_or_default();
        Ok(ByteCommandSlot::new(parent, simple))
    }
    /// First C lookup candidate in the actual retained namespace context.
    /// Namespace-path and global fallback traversal remain runtime operations;
    /// Jim uses `jim_command_lookup_keys` and its flat command table instead.
    ///
    /// # Errors
    /// Returns an error when this engine uses a different command-table model.
    pub fn command_lookup_slot(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<ByteCommandSlot, NameProjectionUnavailable> {
        if self.is_jim084() {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        slot_for_projection(&self.command_lookup_input(context, original)?)
    }
    ///
    /// # Errors
    /// Returns an error for a missing actual Jim namespace object or an unaudited engine operation.
    pub fn command_c_api_publication_slot(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<ByteCommandSlot, NameProjectionUnavailable> {
        slot_for_projection(&self.command_c_api_publication_input(context, original)?)
    }
    /// Jim's current-namespace candidate and independent global fallback key.
    ///
    /// # Errors
    /// Returns an error for a missing actual Jim namespace object or an unaudited engine operation.
    pub fn jim_command_lookup_keys(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<Vec<NameBytes>, NameProjectionUnavailable> {
        if !self.is_jim084() {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        let projected = self.command_lookup_input(context, original)?;
        let mut keys = vec![NameBytes::from(
            projected.jim_flat_key().expect("Jim command projection"),
        )];
        if !original.starts_with(b"::") {
            let global = NameBytes::from(strip_jim_root(original));
            if keys[0] != global {
                keys.push(global);
            }
        }
        Ok(keys)
    }

    /// Namespace object construction is distinct from command qualification in Jim.
    ///
    /// # Errors
    /// Returns an error for a missing actual Jim namespace object or an unaudited engine operation.
    pub fn namespace_address_input<'a>(
        self,
        context: NativeNameContext<'a>,
        original: &'a [u8],
    ) -> Result<NativeNameProjection<'a>, NameProjectionUnavailable> {
        let selected = match self {
            Self::C(_) => Cow::Borrowed(c_string_extent(original)),
            Self::Jim084 => {
                let namespace = context
                    .jim_namespace_object
                    .ok_or(NameProjectionUnavailable::MissingJimNamespaceObject)?;
                if original.starts_with(b"::") {
                    Cow::Borrowed(strip_jim_root(c_string_extent(original)))
                } else if namespace.is_empty() {
                    Cow::Borrowed(original)
                } else {
                    Cow::Owned(join_jim(namespace, original))
                }
            }
        };
        Ok(projection(
            self,
            NativeNamePurpose::NamespaceAddress,
            original,
            selected,
            Some(context),
        ))
    }

    /// Select a C namespace address using retained context components.
    /// The original written operand alone is segmented; the context is never rendered.
    ///
    /// # Errors
    /// Jim namespace objects require their independent flat-object owner.
    pub fn namespace_address_path(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<ByteNamespacePath, NameProjectionUnavailable> {
        if self.is_jim084() {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        let selected = self.namespace_address_input(context, original)?;
        if selected.selected().is_empty()
            && selected.qualification() != NativeNameQualification::Absolute
            && !context.namespace.is_root()
        {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        let mut path = if selected.qualification() == NativeNameQualification::Absolute {
            ByteNamespacePath::root()
        } else {
            context.namespace.clone()
        };
        for segment in qualifier_segments(selected.selected()) {
            path.push(segment);
        }
        Ok(path)
    }

    /// Actual Jim procedure namespace from its complete selected flat command key.
    /// This counted declaration operation differs from `CString` qualifier reporting.
    ///
    /// # Errors
    /// C procedure namespaces are retained holder tokens, not flat-key projections.
    pub fn jim_procedure_namespace(
        self,
        selected_key: &[u8],
    ) -> Result<&[u8], NameProjectionUnavailable> {
        if !self.is_jim084() {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        let selected_key = strip_jim_root(selected_key);
        Ok(match selected_key.iter().rposition(|byte| *byte == b':') {
            Some(last) if last > 0 && selected_key[last - 1] == b':' => &selected_key[..last - 1],
            _ => b"",
        })
    }

    /// Namespace replacement selected by Jim's actual command relocation.
    /// An unqualified destination keeps the existing procedure namespace;
    /// a counted qualifier creates a new namespace object even for equal bytes.
    ///
    /// # Errors
    /// C command relocation uses its retained namespace token instead.
    pub fn jim_procedure_relocation_namespace(
        self,
        selected_key: &[u8],
    ) -> Result<Option<&[u8]>, NameProjectionUnavailable> {
        if !self.is_jim084() {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        let selected_key = strip_jim_root(selected_key);
        Ok(match selected_key.iter().rposition(|byte| *byte == b':') {
            Some(last) if last > 0 && selected_key[last - 1] == b':' => {
                Some(&selected_key[..last - 1])
            }
            _ => None,
        })
    }

    /// Native namespace-variable query input, distinct from ordinary scalar access.
    /// C `Tcl_FindNamespaceVar` takes a `CString`; Jim's helper returns a textual
    /// canonical namespace name without consulting variable cells.
    #[must_use]
    pub fn namespace_variable_query_input(self, original: &[u8]) -> NativeNameProjection<'_> {
        let selected = if self.is_jim084() {
            original
        } else {
            c_string_extent(original)
        };
        projection(
            self,
            NativeNamePurpose::NamespaceVariableQuery,
            original,
            Cow::Borrowed(selected),
            None,
        )
    }

    /// Jim's textual parent query does not require a namespace object to exist.
    ///
    /// # Errors
    /// C parent lookup requires an actual namespace token.
    pub fn jim_namespace_parent_bytes(
        self,
        original: &[u8],
    ) -> Result<Vec<u8>, NameProjectionUnavailable> {
        if !self.is_jim084() {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        if original.is_empty() || c_string_extent(original) == b"::" {
            return Ok(Vec::new());
        }
        let parent = self.namespace_qualifier_bytes(original);
        if parent.starts_with(b"::") {
            return Ok(parent.to_vec());
        }
        let mut bytes = b"::".to_vec();
        bytes.extend_from_slice(parent);
        Ok(bytes)
    }

    /// Native namespace-tail reporting. Jim tests only its last `CString` colon
    /// and returns the complete original object when that colon is not paired.
    #[must_use]
    pub fn namespace_tail_bytes(self, original: &[u8]) -> &[u8] {
        let prefix = c_string_extent(original);
        if !self.is_jim084() {
            return written_command_tail(prefix);
        }
        match prefix.iter().rposition(|byte| *byte == b':') {
            Some(last) if last > 0 && prefix[last - 1] == b':' => &prefix[last + 1..],
            _ => original,
        }
    }
    /// Native namespace-qualifier reporting, independent of namespace lookup.
    #[must_use]
    pub fn namespace_qualifier_bytes(self, original: &[u8]) -> &[u8] {
        let prefix = c_string_extent(original);
        if self.is_jim084() {
            return match prefix.iter().rposition(|byte| *byte == b':') {
                Some(last) if last > 0 && prefix[last - 1] == b':' => &prefix[..last - 1],
                _ => b"",
            };
        }
        let Some(mut start) = prefix.windows(2).rposition(|pair| pair == b"::") else {
            return b"";
        };
        while start > 0 && prefix[start - 1] == b':' {
            start -= 1;
        }
        &prefix[..start]
    }

    /// Root-variable receiver input. Qualification is tested before raw NUL in
    /// C; a later `::` must not turn a full unqualified scalar into a short key.
    #[must_use]
    pub fn variable_root_input(self, original: &[u8]) -> NativeNameProjection<'_> {
        let prefix = c_string_extent(original);
        let selected = match self {
            Self::C(version) if version == TclVersion::V8_4 || is_qualified(prefix) => prefix,
            _ => original,
        };
        let mut result = projection(
            self,
            NativeNamePurpose::VariableRoot,
            original,
            Cow::Borrowed(selected),
            None,
        );
        result.qualification = match self {
            Self::C(_) => qualification(prefix),
            Self::Jim084 if original.starts_with(b"::") => NativeNameQualification::Absolute,
            Self::Jim084 => NativeNameQualification::Unqualified,
        };
        result
    }
    /// Scalar receiver used by `TclLookupSimpleVar` for an alias's local side.
    /// Its extent follows the simple-name API; it does not authorise a parsed
    /// or indexed cache on the original operand.
    #[must_use]
    pub fn variable_alias_local_input(self, original: &[u8]) -> NativeNameProjection<'_> {
        let mut input = self.variable_root_input(original);
        input.purpose = NativeNamePurpose::VariableAliasLocal;
        input
    }
    /// Combined array spelling uses a native opening-parenthesis scanner and
    /// its own element extent. It does not strip a source `$` or `${...}`.
    #[must_use]
    pub fn combined_variable_input(self, original: &[u8]) -> NativeVariableProjection<'_> {
        let opening_input = match self {
            Self::C(version) if version < TclVersion::V9_0 => original,
            _ => c_string_extent(original),
        };
        let parts = if original.last() == Some(&b')') {
            opening_input
                .iter()
                .position(|byte| *byte == b'(')
                .map(|opening| {
                    (
                        &original[..opening],
                        &original[opening + 1..original.len() - 1],
                    )
                })
        } else {
            None
        };
        let (root, element) = match parts {
            Some((root, element)) => (
                self.variable_root_input(root),
                Some(self.element_input(element, NativeNamePurpose::ArrayElementCombined)),
            ),
            None => (self.variable_root_input(original), None),
        };
        NativeVariableProjection {
            original: NativeVariableInputForm::Combined(original),
            root,
            element,
        }
    }
    /// Separate native object operands preserve their input form. An already
    /// parsed array root plus a second element is a runtime guest error, not a
    /// reason for this pure selector to invent a different receiver.
    #[must_use]
    pub fn separate_variable_input<'a>(
        self,
        root: &'a [u8],
        element: Option<&'a [u8]>,
    ) -> NativeVariableProjection<'a> {
        NativeVariableProjection {
            original: NativeVariableInputForm::Separate { root, element },
            root: self.variable_root_input(root),
            element: element
                .map(|bytes| self.element_input(bytes, NativeNamePurpose::ArrayElementSeparated)),
        }
    }
    fn element_input(
        self,
        original: &[u8],
        purpose: NativeNamePurpose,
    ) -> NativeNameProjection<'_> {
        let truncate = match (self, purpose) {
            (Self::C(version), NativeNamePurpose::ArrayElementCombined) => {
                version < TclVersion::V9_0
            }
            (Self::C(TclVersion::V8_4), NativeNamePurpose::ArrayElementSeparated) => true,
            _ => false,
        };
        let selected = if truncate {
            c_string_extent(original)
        } else {
            original
        };
        let mut result = projection(self, purpose, original, Cow::Borrowed(selected), None);
        result.qualification = NativeNameQualification::Unqualified;
        result
    }

    /// C trace text extent is separate from the followed physical receiver.
    ///
    /// # Errors
    /// Returns `PurposeNotModelled` for an unaudited engine operation.
    pub fn trace_registration_input(
        self,
        original: &[u8],
    ) -> Result<NativeNameProjection<'_>, NameProjectionUnavailable> {
        self.trace_input(original, NativeNamePurpose::VariableTraceRegistration)
    }
    ///
    /// # Errors
    /// Returns `PurposeNotModelled` for an unaudited engine operation.
    pub fn trace_query_input(
        self,
        original: &[u8],
    ) -> Result<NativeNameProjection<'_>, NameProjectionUnavailable> {
        self.trace_input(original, NativeNamePurpose::VariableTraceQuery)
    }
    fn trace_input(
        self,
        original: &[u8],
        purpose: NativeNamePurpose,
    ) -> Result<NativeNameProjection<'_>, NameProjectionUnavailable> {
        if self.is_jim084() {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        Ok(projection(
            self,
            purpose,
            original,
            Cow::Borrowed(c_string_extent(original)),
            None,
        ))
    }
    /// Select a namespace command pattern before qualifier parsing or storage.
    /// C export/import/forget entry points receive C strings. Jim's import
    /// helper receives the same extent; its export/forget APIs have no C recipe.
    ///
    /// # Errors
    /// Refuses other purposes and Jim export/forget projections.
    pub fn namespace_pattern_input(
        self,
        original: &[u8],
        purpose: NativeNamePurpose,
    ) -> Result<NativeNameProjection<'_>, NameProjectionUnavailable> {
        if !matches!(
            purpose,
            NativeNamePurpose::NamespaceExportPattern
                | NativeNamePurpose::NamespaceImportPattern
                | NativeNamePurpose::NamespaceForgetPattern
        ) || (self.is_jim084() && purpose != NativeNamePurpose::NamespaceImportPattern)
        {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        Ok(projection(
            self,
            purpose,
            original,
            Cow::Borrowed(c_string_extent(original)),
            None,
        ))
    }

    /// Split a namespace pattern after its purpose selects the input extent.
    /// Constructed C context components are never rendered or reparsed; Jim
    /// uses its own namespace helper and original namespace object instead.
    ///
    /// # Errors
    /// Refuses unsupported purposes and missing actual Jim namespace context.
    pub fn namespace_pattern_parts(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
        purpose: NativeNamePurpose,
    ) -> Result<NativeNamespacePatternParts, NameProjectionUnavailable> {
        let mut input = self.namespace_pattern_input(original, purpose)?;
        let selected = NameBytes::from(input.selected());
        let (source, tail) = if !is_qualified(input.selected()) {
            (None, selected.clone())
        } else if self.is_jim084() {
            let qualifier = self.namespace_qualifier_bytes(input.selected());
            let source = self.jim_namespace_canonical_input(context, qualifier)?;
            (
                Some(NativeNamespacePatternSource::Jim(source.selected().into())),
                self.namespace_tail_bytes(input.selected()).into(),
            )
        } else {
            input.context = Some(context);
            let slot = slot_for_projection(&input)?;
            (
                Some(NativeNamespacePatternSource::C(slot.namespace)),
                slot.simple,
            )
        };
        Ok(NativeNamespacePatternParts {
            original: original.into(),
            selected,
            source,
            tail,
            purpose,
        })
    }

    /// Jim helper canonicalisation preserves relative object bytes, while an
    /// absolute operand strips leading colons and consumes `CString` extent.
    /// This differs from command publication and command lookup qualification.
    ///
    /// # Errors
    /// Refuses C recipes or an absent actual Jim namespace object.
    pub fn jim_namespace_canonical_input<'a>(
        self,
        context: NativeNameContext<'a>,
        original: &'a [u8],
    ) -> Result<NativeNameProjection<'a>, NameProjectionUnavailable> {
        if !self.is_jim084() {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        let namespace = context
            .jim_namespace_object
            .ok_or(NameProjectionUnavailable::MissingJimNamespaceObject)?;
        let selected = if original.starts_with(b"::") {
            Cow::Borrowed(c_string_extent(strip_jim_root(original)))
        } else if namespace.is_empty() {
            Cow::Borrowed(original)
        } else {
            let mut name = namespace.to_vec();
            name.extend_from_slice(b"::");
            name.extend_from_slice(original);
            Cow::Owned(name)
        };
        Ok(projection(
            self,
            NativeNamePurpose::JimNamespaceCanonical,
            original,
            selected,
            Some(context),
        ))
    }

    /// Select Jim's original-object canonicalisation branch before construction.
    /// Equal byte results do not imply that the same native object is returned.
    ///
    /// # Errors
    /// Refuses a non-Jim recipe or a missing actual namespace holder.
    pub fn jim_namespace_construction(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<NativeJimNamespaceConstruction, NameProjectionUnavailable> {
        if !self.is_jim084() {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        let namespace = context
            .jim_namespace_object
            .ok_or(NameProjectionUnavailable::MissingJimNamespaceObject)?;
        Ok(if original.starts_with(b"::") {
            NativeJimNamespaceConstruction::FreshString
        } else if namespace.is_empty() {
            NativeJimNamespaceConstruction::RetainOriginal
        } else {
            NativeJimNamespaceConstruction::DuplicateNamespaceAndAppend
        })
    }

    /// Package database keys use `CString` extent in both audited engines.
    #[must_use]
    pub fn package_key(self, original: &[u8]) -> NativeNameProjection<'_> {
        projection(
            self,
            NativeNamePurpose::PackageName,
            original,
            Cow::Borrowed(c_string_extent(original)),
            None,
        )
    }
    /// A parsed formal operand's native name key, independently of list parsing.
    #[must_use]
    pub fn formal_name_input(self, original: &[u8]) -> NativeNameProjection<'_> {
        self.formal_enumeration_name_input(original)
    }
    /// Native `info args`/`info default` name presentation and comparison.
    /// This input does not select the procedure's actual local storage key.
    #[must_use]
    pub fn formal_enumeration_name_input(self, original: &[u8]) -> NativeNameProjection<'_> {
        let selected = if self.is_jim084() {
            original
        } else {
            c_string_extent(original)
        };
        projection(
            self,
            NativeNamePurpose::FormalEnumeration,
            original,
            Cow::Borrowed(selected),
            None,
        )
    }
    /// Formal storage after the caller has parsed the native argument-list
    /// representation. C8.4/8.5 declarations use `CString` list splitting;
    /// C8.6+ and Jim retain the complete selected formal bytes.
    #[must_use]
    pub fn formal_storage_name_input(self, original: &[u8]) -> NativeNameProjection<'_> {
        let selected = if matches!(self, Self::C(version) if version <= TclVersion::V8_5) {
            c_string_extent(original)
        } else {
            original
        };
        projection(
            self,
            NativeNamePurpose::FormalStorage,
            original,
            Cow::Borrowed(selected),
            None,
        )
    }
    /// The local receiver published by `namespace upvar`. Its C argument is
    /// a `CString` even when ordinary `upvar`, `global` and `variable` locals
    /// retain a counted object. C8.4 does not provide this command purpose.
    ///
    /// # Errors
    /// Returns `PurposeNotModelled` for C8.4.
    pub fn namespace_upvar_local_input(
        self,
        original: &[u8],
    ) -> Result<NativeNameProjection<'_>, NameProjectionUnavailable> {
        if self == Self::C(TclVersion::V8_4) {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        let selected = if self.is_jim084() {
            original
        } else {
            c_string_extent(original)
        };
        Ok(projection(
            self,
            NativeNamePurpose::NamespaceUpvarLocal,
            original,
            Cow::Borrowed(selected),
            None,
        ))
    }
    /// `TclOO` method publication and dispatch retain counted original bytes.
    ///
    /// # Errors
    /// Returns an unavailable purpose for engines without audited `TclOO` support.
    pub fn oo_method_input(
        self,
        original: &[u8],
    ) -> Result<NativeNameProjection<'_>, NameProjectionUnavailable> {
        if !matches!(self, Self::C(version) if version >= TclVersion::V8_6) {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        Ok(projection(
            self,
            NativeNamePurpose::OoMethod,
            original,
            Cow::Borrowed(original),
            None,
        ))
    }

    /// C hidden-table tokens use their own `CString` namespace, without qualifiers.
    ///
    /// # Errors
    /// Returns `PurposeNotModelled` for an unaudited engine operation.
    pub fn hidden_token_input(
        self,
        original: &[u8],
    ) -> Result<NativeNameProjection<'_>, NameProjectionUnavailable> {
        if self.is_jim084() {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        Ok(projection(
            self,
            NativeNamePurpose::HiddenToken,
            original,
            Cow::Borrowed(c_string_extent(original)),
            None,
        ))
    }
}

fn projection<'a>(
    protocol: NativeNameProtocol,
    purpose: NativeNamePurpose,
    original: &'a [u8],
    selected: Cow<'a, [u8]>,
    context: Option<NativeNameContext<'a>>,
) -> NativeNameProjection<'a> {
    let qualification = if matches!(
        purpose,
        NativeNamePurpose::PackageName
            | NativeNamePurpose::FormalEnumeration
            | NativeNamePurpose::FormalStorage
            | NativeNamePurpose::NamespaceUpvarLocal
            | NativeNamePurpose::HiddenToken
            | NativeNamePurpose::ArrayElementCombined
            | NativeNamePurpose::ArrayElementSeparated
    ) {
        NativeNameQualification::Unqualified
    } else {
        qualification(&selected)
    };
    NativeNameProjection {
        original,
        selected,
        protocol,
        purpose,
        qualification,
        context,
    }
}
fn qualification(bytes: &[u8]) -> NativeNameQualification {
    if bytes.starts_with(b"::") {
        NativeNameQualification::Absolute
    } else if is_qualified(bytes) {
        NativeNameQualification::Relative
    } else {
        NativeNameQualification::Unqualified
    }
}

pub(super) fn strip_jim_root(bytes: &[u8]) -> &[u8] {
    if bytes.starts_with(b"::") {
        &bytes[bytes
            .iter()
            .position(|byte| *byte != b':')
            .unwrap_or(bytes.len())..]
    } else {
        bytes
    }
}
fn join_jim(namespace: &[u8], name: &[u8]) -> Vec<u8> {
    let mut selected = Vec::with_capacity(namespace.len() + 2 + name.len());
    selected.extend_from_slice(namespace);
    selected.extend_from_slice(b"::");
    selected.extend_from_slice(name);
    selected
}
fn is_command_purpose(purpose: NativeNamePurpose) -> bool {
    matches!(
        purpose,
        NativeNamePurpose::CommandLookup
            | NativeNamePurpose::CommandPublication
            | NativeNamePurpose::CommandCApiPublication
            | NativeNamePurpose::RenameSource
            | NativeNamePurpose::RenameDestination
            | NativeNamePurpose::AliasPublication
            | NativeNamePurpose::OoObjectPublication
    )
}
fn slot_for_projection(
    input: &NativeNameProjection<'_>,
) -> Result<ByteCommandSlot, NameProjectionUnavailable> {
    if let Some(key) = input.jim_flat_key() {
        return Ok(ByteCommandSlot::new(
            ByteNamespacePath::root(),
            NameBytes::from(key),
        ));
    }
    let context = input
        .context
        .ok_or(NameProjectionUnavailable::PurposeNotModelled)?;
    let mut path = if input.qualification == NativeNameQualification::Absolute {
        ByteNamespacePath::root()
    } else {
        context.namespace.clone()
    };
    let segments = qualifier_segments(input.selected());
    let qualifier_count = if ends_with_separator(input.selected()) {
        segments.len()
    } else {
        segments.len().saturating_sub(1)
    };
    for segment in &segments[..qualifier_count] {
        path.push(*segment);
    }
    Ok(ByteCommandSlot::new(
        path,
        NameBytes::from(written_command_tail(input.selected())),
    ))
}

/// Native namespace lookup failure, with a complete current-context report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeNamespaceLookupError {
    /// Native diagnostic bytes.
    pub message: Vec<u8>,
    /// Native error code as Tcl list bytes.
    pub error_code: Vec<u8>,
}

/// Report a failed native namespace address lookup without using display as identity.
///
/// # Errors
/// Tcl 8.4 requires the caller's operation; Jim uses distinct script helpers.
pub fn report_native_namespace_lookup_error(
    protocol: NativeNameProtocol,
    original: &[u8],
    current_fullname: &[u8],
) -> Result<NativeNamespaceLookupError, NameProjectionUnavailable> {
    if protocol.is_jim084() || protocol == NativeNameProtocol::C(TclVersion::V8_4) {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    }
    Ok(namespace_lookup_error_parts(original, current_fullname))
}

fn namespace_lookup_error_parts(
    original: &[u8],
    current_fullname: &[u8],
) -> NativeNamespaceLookupError {
    let written = c_string_extent(original);
    let mut message = b"namespace \"".to_vec();
    message.extend_from_slice(written);
    message.extend_from_slice(b"\" not found");
    if !written.starts_with(b"::") {
        message.extend_from_slice(b" in \"");
        message.extend_from_slice(c_string_extent(current_fullname));
        message.push(b'"');
    }
    let mut error_code = b"TCL LOOKUP NAMESPACE ".to_vec();
    crate::list::append_list_element(&mut error_code, written, false);
    NativeNamespaceLookupError {
        message,
        error_code,
    }
}

/// The native namespace operation that selected a missing address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeNamespaceLookupOperation {
    /// Later-C `TclGetNamespaceFromObj` reporting used by namespace upvar/path.
    ObjectLookup,
    /// Namespace parent query.
    Parent,
    /// Namespace children query.
    Children,
    /// Namespace deletion.
    Delete,
    /// Namespace inscope target selection.
    Inscope,
}

/// Native namespace operation failure with its primitive error-code update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeNamespaceOperationError {
    /// Complete diagnostic bytes.
    pub message: Vec<u8>,
    /// Explicit native tuple; absence retains the primitive error-code state.
    pub error_code: Option<Vec<u8>>,
    /// The native append/format result owns an unknown-count String primary.
    /// This pure recipe does not itself authorise physical cache installation.
    pub string_protocol: crate::native_string::NativeStringProtocol,
}

/// Report a missing namespace using the actual operation and engine release.
///
/// # Errors
/// Refuses unaudited Jim failure operations and unavailable C84 object APIs.
pub fn report_native_namespace_operation_error(
    protocol: NativeNameProtocol,
    operation: NativeNamespaceLookupOperation,
    original: &[u8],
    current_fullname: &[u8],
) -> Result<NativeNamespaceOperationError, NameProjectionUnavailable> {
    use NativeNamespaceLookupOperation as Operation;
    if protocol.is_jim084()
        || (protocol == NativeNameProtocol::C(TclVersion::V8_4)
            && operation == Operation::ObjectLookup)
    {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    }
    let report = namespace_lookup_error_parts(original, current_fullname);
    let legacy = protocol == NativeNameProtocol::C(TclVersion::V8_4);
    let message = if legacy || operation == Operation::Delete {
        let mut message = b"unknown namespace \"".to_vec();
        message.extend_from_slice(c_string_extent(original));
        message.extend_from_slice(b"\" in ");
        if operation == Operation::Inscope {
            message.extend_from_slice(b"inscope namespace command");
        } else {
            message.extend_from_slice(b"namespace ");
            message.extend_from_slice(match operation {
                Operation::Parent => b"parent",
                Operation::Children => b"children",
                Operation::Delete => b"delete",
                Operation::ObjectLookup | Operation::Inscope => unreachable!("selected above"),
            });
            message.extend_from_slice(b" command");
        }
        message
    } else {
        report.message
    };
    Ok(NativeNamespaceOperationError {
        message,
        error_code: (!legacy).then_some(report.error_code),
        string_protocol: crate::native_string::NativeStringProtocol::C(match protocol {
            NativeNameProtocol::C(version) => version,
            NativeNameProtocol::Jim084 => unreachable!("Jim operation refused above"),
        }),
    })
}

/// The variable operation whose diagnostic retains original operand parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeVariableDiagnosticOperation {
    Read,
    Write,
    Unset,
    /// Native increment constant-write failure; reached reads use `Read`.
    Increment,
    /// Native C9 constant declaration failure on the selected variable cell.
    MakeConstant,
    /// Namespace declaration lookup, independently of a following value write.
    Define,
}

/// The actual variable failure selected by the physical runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeVariableDiagnosticReason {
    NoSuchVariable,
    NoSuchElement,
    NotArray,
    IsArray,
    MissingParentNamespace,
    Constant,
    DetachedElement,
    RetiredNamespace,
    /// A constant declaration targets an existing non-constant variable.
    AlreadyExists,
    /// A constant or namespace declaration targets an array element.
    ArrayElement,
}

impl NativeVariableDiagnosticReason {
    /// Native reason text after operation-specific selection.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::NoSuchVariable => "no such variable",
            Self::NoSuchElement => "no such element in array",
            Self::NotArray => "variable isn't array",
            Self::IsArray => "variable is array",
            Self::MissingParentNamespace => "parent namespace doesn't exist",
            Self::Constant => "variable is a constant",
            Self::DetachedElement => "upvar refers to element in deleted array",
            Self::RetiredNamespace => "upvar refers to variable in deleted namespace",
            Self::AlreadyExists => "variable already exists",
            Self::ArrayElement => "name refers to an element in an array",
        }
    }
}

/// Failure site retained by the physical variable operation. Identical reason
/// text at two sites does not imply identical reporting or error-code tuples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeVariableFailureSite {
    NameLookup,
    ValueRead,
    ValueWrite,
    ValueUnset,
}

/// Purpose-selected diagnostic output, independent of cell identity and argv.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeVariableDiagnosticProjection {
    /// Operation-selected name spelling for the diagnostic.
    pub name: Vec<u8>,
    /// Effective reason, including Jim's unset-on-scalar dictionary behaviour.
    pub reason: NativeVariableDiagnosticReason,
    /// Explicit native error-code words. `None` means this operation does not
    /// issue a tuple; the interpreter's error initialisation owns its default.
    pub error_code: Option<Vec<Vec<u8>>>,
    /// Compatibility lookup-root view, only for actual LOOKUP VARNAME tuples.
    pub missing_lookup_root: Option<Vec<u8>>,
}

/// Report a failure occurring while selecting a variable name-table binding.
/// Late cell reads/writes must use [`report_native_variable_diagnostic_at`].
///
/// # Errors
/// Returns `PurposeNotModelled` for an unmeasured failure/input combination.
pub fn report_native_variable_diagnostic(
    protocol: NativeNameProtocol,
    operation: NativeVariableDiagnosticOperation,
    reason: NativeVariableDiagnosticReason,
    input: NativeVariableInputForm<'_>,
) -> Result<NativeVariableDiagnosticProjection, NameProjectionUnavailable> {
    report_native_variable_diagnostic_at(
        protocol,
        operation,
        reason,
        NativeVariableFailureSite::NameLookup,
        input,
    )
}

/// Select the measured C9 constant failure verb at its physical site.
/// Early binding failures report `const`; late receiver failures report
/// `make constant`. This does not select a variable receiver or caller opcode.
///
/// # Errors
/// Refuses other engines and unsupported failure sites.
pub fn native_constant_failure_verb(
    protocol: NativeNameProtocol,
    site: NativeVariableFailureSite,
) -> Result<&'static str, NameProjectionUnavailable> {
    if !matches!(protocol, NativeNameProtocol::C(version) if version >= TclVersion::V9_0) {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    }
    match site {
        NativeVariableFailureSite::NameLookup => Ok("const"),
        NativeVariableFailureSite::ValueWrite => Ok("make constant"),
        _ => Err(NameProjectionUnavailable::PurposeNotModelled),
    }
}

fn special_variable_failure_projection(
    protocol: NativeNameProtocol,
    operation: NativeVariableDiagnosticOperation,
    reason: NativeVariableDiagnosticReason,
    site: NativeVariableFailureSite,
    input: NativeVariableInputForm<'_>,
) -> Result<Option<NativeVariableDiagnosticProjection>, NameProjectionUnavailable> {
    use NativeVariableDiagnosticOperation::MakeConstant;
    use NativeVariableDiagnosticReason::{
        IsArray, MissingParentNamespace, NoSuchElement, NotArray,
    };
    use NativeVariableFailureSite::{NameLookup, ValueWrite};
    if operation == NativeVariableDiagnosticOperation::Define {
        if !matches!(protocol, NativeNameProtocol::C(_)) || site != NameLookup {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        if reason == NativeVariableDiagnosticReason::ArrayElement {
            let NativeVariableInputForm::Combined(original) = input else {
                return Err(NameProjectionUnavailable::PurposeNotModelled);
            };
            return Ok(Some(NativeVariableDiagnosticProjection {
                name: c_string_extent(original).to_vec(),
                reason,
                error_code: Some(vec![
                    b"TCL".to_vec(),
                    b"UPVAR".to_vec(),
                    b"LOCAL_ELEMENT".to_vec(),
                ]),
                missing_lookup_root: None,
            }));
        }
    }
    if operation == MakeConstant {
        native_constant_failure_verb(protocol, site)?;
        if site == NameLookup && matches!(reason, MissingParentNamespace | NotArray | NoSuchElement)
        {
            // Binding failures use the ordinary independently split name parts.
        } else if site == ValueWrite
            && matches!(
                reason,
                IsArray
                    | NativeVariableDiagnosticReason::AlreadyExists
                    | NativeVariableDiagnosticReason::ArrayElement
            )
        {
            let name = match input {
                NativeVariableInputForm::Combined(original) => c_string_extent(original).to_vec(),
                NativeVariableInputForm::Separate { root, element } => {
                    let mut name = c_string_extent(root).to_vec();
                    if let Some(element) = element {
                        name.push(b'(');
                        name.extend_from_slice(c_string_extent(element));
                        name.push(b')');
                    }
                    name
                }
            };
            return Ok(Some(NativeVariableDiagnosticProjection {
                name,
                reason,
                error_code: Some(vec![b"TCL".to_vec(), b"LOOKUP".to_vec(), b"CONST".to_vec()]),
                missing_lookup_root: None,
            }));
        } else {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
    }
    Ok(None)
}

fn native_variable_failure_code(
    protocol: NativeNameProtocol,
    operation: NativeVariableDiagnosticOperation,
    reason: NativeVariableDiagnosticReason,
    site: NativeVariableFailureSite,
    root: &[u8],
    element: Option<&[u8]>,
) -> Result<Option<Vec<Vec<u8>>>, NameProjectionUnavailable> {
    use NativeVariableDiagnosticOperation::Increment;
    use NativeVariableDiagnosticReason::{
        Constant, DetachedElement, IsArray, MissingParentNamespace, NoSuchElement, NoSuchVariable,
        NotArray, RetiredNamespace,
    };
    use NativeVariableFailureSite::{NameLookup, ValueRead, ValueUnset, ValueWrite};
    let error_code = if matches!(protocol, NativeNameProtocol::C(version) if version >= TclVersion::V8_6)
    {
        let words: &[&[u8]] = match (site, reason) {
            (NameLookup, NoSuchVariable | MissingParentNamespace | NotArray) => {
                &[b"TCL", b"LOOKUP", b"VARNAME", root]
            }
            (NameLookup | ValueUnset, NoSuchElement) if element.is_some() => &[
                b"TCL",
                b"LOOKUP",
                b"ELEMENT",
                element.expect("selected element"),
            ],
            (ValueRead, NoSuchVariable | NoSuchElement | IsArray | DetachedElement) => {
                &[b"TCL", b"READ", b"VARNAME"]
            }
            (ValueWrite, Constant) if operation == Increment => &[b"TCL", b"WRITE", b"CONST"],
            (ValueWrite, NoSuchVariable | Constant | DetachedElement | RetiredNamespace) => {
                &[b"TCL", b"WRITE", b"VARNAME"]
            }
            (ValueUnset, Constant) if matches!(protocol, NativeNameProtocol::C(version) if version >= TclVersion::V9_0) => {
                &[b"TCL", b"UNSET", b"CONST"]
            }
            _ => return Err(NameProjectionUnavailable::PurposeNotModelled),
        };
        Some(words.iter().map(|word| word.to_vec()).collect::<Vec<_>>())
    } else {
        None
    };
    Ok(error_code)
}

/// Project the original input at the actual physical failure site. C lookup
/// diagnostics format `CString` root/element parts independently. A late read
/// uses the original combined `CString` operand, so an embedded NUL can remove
/// its closing parenthesis. Separate object inputs retain their own boundaries.
///
/// # Errors
/// Returns `PurposeNotModelled` for an unaudited site, operation or Jim input form.
pub fn report_native_variable_diagnostic_at(
    protocol: NativeNameProtocol,
    operation: NativeVariableDiagnosticOperation,
    reason: NativeVariableDiagnosticReason,
    site: NativeVariableFailureSite,
    input: NativeVariableInputForm<'_>,
) -> Result<NativeVariableDiagnosticProjection, NameProjectionUnavailable> {
    use NativeVariableDiagnosticOperation::{Increment, Read, Unset, Write};
    use NativeVariableDiagnosticReason::{Constant, NoSuchElement, NoSuchVariable, NotArray};
    use NativeVariableFailureSite::{ValueRead, ValueUnset, ValueWrite};
    if let Some(projection) =
        special_variable_failure_projection(protocol, operation, reason, site, input)?
    {
        return Ok(projection);
    }
    if matches!(
        reason,
        NativeVariableDiagnosticReason::AlreadyExists
            | NativeVariableDiagnosticReason::ArrayElement
    ) {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    }
    if matches!(
        (site, operation),
        (ValueRead, Write | Unset | Increment)
            | (ValueWrite, Read | Unset)
            | (ValueUnset, Read | Write | Increment)
    ) {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    }
    if operation == Increment
        && !(site == ValueWrite
            && reason == Constant
            && matches!(protocol, NativeNameProtocol::C(version) if version >= TclVersion::V9_0))
    {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    }
    let parts = match input {
        NativeVariableInputForm::Combined(original) => protocol.combined_variable_input(original),
        NativeVariableInputForm::Separate { root, element } => {
            protocol.separate_variable_input(root, element)
        }
    };
    let root = c_string_extent(parts.root().original());
    let element = parts.element().map(|part| c_string_extent(part.original()));
    let reconstructed = || {
        let mut name = root.to_vec();
        if let Some(element) = element {
            name.push(b'(');
            name.extend_from_slice(element);
            name.push(b')');
        }
        name
    };
    let mut effective_reason = reason;
    let name = if protocol.is_jim084() {
        match input {
            NativeVariableInputForm::Separate { .. } => match (operation, reason) {
                (Read, NoSuchVariable) => root.to_vec(),
                (Read, NoSuchElement | NotArray) if element.is_some() => reconstructed(),
                _ => return Err(NameProjectionUnavailable::PurposeNotModelled),
            },
            NativeVariableInputForm::Combined(original) => match (operation, reason) {
                (Read, NoSuchVariable) if element.is_some() => root.to_vec(),
                (Read, NoSuchElement | NotArray) => reconstructed(),
                (Read | Write | Unset, NoSuchVariable)
                | (Write, NotArray)
                | (Unset, NoSuchElement) => c_string_extent(original).to_vec(),
                (Unset, NotArray) => {
                    effective_reason = NoSuchElement;
                    c_string_extent(original).to_vec()
                }
                _ => return Err(NameProjectionUnavailable::PurposeNotModelled),
            },
        }
    } else if matches!(site, ValueRead | ValueWrite) {
        match input {
            NativeVariableInputForm::Combined(original) => c_string_extent(original).to_vec(),
            NativeVariableInputForm::Separate { .. } => reconstructed(),
        }
    } else {
        reconstructed()
    };
    let error_code =
        native_variable_failure_code(protocol, operation, reason, site, root, element)?;
    let missing_lookup_root = error_code
        .as_ref()
        .filter(|words| words.len() == 4 && words[1] == b"LOOKUP" && words[2] == b"VARNAME")
        .map(|words| words[3].clone());
    Ok(NativeVariableDiagnosticProjection {
        name,
        reason: effective_reason,
        error_code,
        missing_lookup_root,
    })
}

/// Audited native reporting operation, distinct from lookup or receiver identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeNameReportPurpose {
    TraceCallbackName1,
    TraceCallbackName2,
    /// Missing command formatter, independently of command-table extent.
    CommandLookupError,
    /// C interpreter-handle lookup diagnostic uses its `CString` operand.
    InterpreterLookupError,
    /// `TclOO` method enumeration formats the counted table key as `CString`.
    OoMethodEnumeration,
    /// `TclOO` unknown-method diagnostics format the counted operand as `CString`.
    OoMethodLookupError,
    /// Ensemble default-target miss exposed under the caller's subcommand.
    EnsembleMissingSubcommand,
}
/// Project a supplied native callback-name part. The caller supplies the actual
/// operation's part boundaries; combined array reporting is not reconstructed
/// from a selected root/element or an alias target.
///
/// # Errors
/// Returns `PurposeNotModelled` for an unaudited engine operation.
pub fn report_native_name_bytes(
    protocol: NativeNameProtocol,
    purpose: NativeNameReportPurpose,
    original_operation_part: &[u8],
) -> Result<&[u8], NameProjectionUnavailable> {
    if matches!(
        purpose,
        NativeNameReportPurpose::OoMethodEnumeration | NativeNameReportPurpose::OoMethodLookupError
    ) && !matches!(protocol, NativeNameProtocol::C(version) if version >= TclVersion::V8_6)
    {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    }
    if protocol.is_jim084() && purpose == NativeNameReportPurpose::InterpreterLookupError {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    }
    if protocol.is_jim084()
        && matches!(
            purpose,
            NativeNameReportPurpose::TraceCallbackName1
                | NativeNameReportPurpose::TraceCallbackName2
        )
    {
        Err(NameProjectionUnavailable::PurposeNotModelled)
    } else {
        Ok(c_string_extent(original_operation_part))
    }
}

/// Retained input parts plus the followed receiver's actual element key.
/// The key is reporting metadata from an already resolved cell, not a name
/// from which this pure function may infer that cell or its lifetime.
#[derive(Debug, Clone, Copy)]
pub struct NativeVariableTraceReportingInput<'a> {
    pub original_part1: &'a [u8],
    pub original_part2: Option<&'a [u8]>,
    pub actual_element_key: Option<&'a [u8]>,
}

/// Exact reported name parts for a reached variable access trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeVariableTraceNames<'a> {
    pub name1: &'a [u8],
    pub name2: &'a [u8],
}

/// Native access-trace name reporting. This is separate from trace registration,
/// cell resolution and unset/retirement reporting. C9 fills a missing second
/// name from the already followed array-element cell; older C does not.
///
/// # Errors
/// Returns `PurposeNotModelled` for an unaudited engine operation.
pub fn report_native_variable_access_trace_names(
    protocol: NativeNameProtocol,
    input: NativeVariableTraceReportingInput<'_>,
) -> Result<NativeVariableTraceNames<'_>, NameProjectionUnavailable> {
    let NativeNameProtocol::C(version) = protocol else {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    };
    let mut name1 = c_string_extent(input.original_part1);
    let mut name2 = input.original_part2.map(c_string_extent);
    if name2.is_none()
        && name1.last() == Some(&b')')
        && let Some(opening) = name1.iter().position(|byte| *byte == b'(')
    {
        name2 = Some(&name1[opening + 1..name1.len() - 1]);
        name1 = &name1[..opening];
    }
    if name2.is_none() && version >= TclVersion::V9_0 {
        name2 = input.actual_element_key.map(c_string_extent);
    }
    Ok(NativeVariableTraceNames {
        name1,
        name2: name2.unwrap_or(b""),
    })
}

/// Checked Unicode components of an already constructed path. This preserves
/// segment identity; it does not certify an equivalent written source spelling.
///
/// # Errors
/// Returns an error when a retained native component is not valid Rust UTF-8.
pub fn checked_namespace_path_utf8(
    path: &ByteNamespacePath,
) -> Result<Vec<String>, std::str::Utf8Error> {
    path.as_segments()
        .iter()
        .map(|part| part.try_utf8().map(str::to_owned))
        .collect()
}

/// Checked analytical slot without joining/reparsing its components. A caller
/// may length-frame these components for internal string maps. Opaque native
/// rows remain in their byte/token tables if this optional view is unavailable.
///
/// # Errors
/// Returns an error when a retained native component is not valid Rust UTF-8.
pub fn checked_command_slot_utf8(
    slot: &ByteCommandSlot,
) -> Result<tcl_core_types::CommandSlot<String, Vec<String>>, std::str::Utf8Error> {
    Ok(tcl_core_types::CommandSlot::new(
        checked_namespace_path_utf8(&slot.namespace)?,
        slot.simple.try_utf8()?.to_owned(),
    ))
}

/// A globally addressable written source spelling only when the selected
/// native recipe round-trips to this exact structured slot. Valid UTF-8 alone
/// does not certify native extent or injective colon boundaries. Failure is
/// absence of an analytical alias, never absence of the actual byte binding.
#[must_use]
pub fn native_command_source_spelling(
    protocol: NativeNameProtocol,
    slot: &ByteCommandSlot,
) -> Option<String> {
    let projected = checked_command_slot_utf8(slot).ok()?;
    let mut spelling = String::from("::");
    for part in &projected.namespace {
        spelling.push_str(part);
        spelling.push_str("::");
    }
    spelling.push_str(&projected.simple);
    let selected = protocol
        .command_publication_slot(NativeNameContext::root(), spelling.as_bytes())
        .ok()?;
    if selected != *slot {
        return None;
    }
    let lookup_agrees = if protocol.is_jim084() {
        slot.namespace.is_root()
            && protocol
                .jim_command_lookup_keys(NativeNameContext::root(), spelling.as_bytes())
                .ok()?
                .as_slice()
                == std::slice::from_ref(&slot.simple)
    } else {
        protocol
            .command_lookup_slot(NativeNameContext::root(), spelling.as_bytes())
            .ok()?
            == *slot
    };
    lookup_agrees.then_some(spelling)
}

/// `TclGetCommandFullName` reporting bytes from an already selected native slot.
/// Literal colons and non-text bytes remain data; this projection supplies no
/// source spelling, lookup key or command identity.
#[must_use]
pub fn native_command_full_name_bytes(slot: &ByteCommandSlot) -> Vec<u8> {
    let mut report = b"::".to_vec();
    for component in slot.namespace.as_segments() {
        report.extend_from_slice(component.as_bytes());
        report.extend_from_slice(b"::");
    }
    report.extend_from_slice(slot.simple.as_bytes());
    report
}

/// Written C namespace spelling only when its native address round-trips to
/// the exact constructed path. Jim's flat namespace object must instead be
/// supplied to `native_jim_namespace_source_spelling`; a path does not recover
/// that independently retained object. The root is shared by both engines.
#[must_use]
pub fn native_namespace_source_spelling(
    protocol: NativeNameProtocol,
    path: &ByteNamespacePath,
) -> Option<String> {
    if path.is_root() {
        return Some(String::from("::"));
    }
    if protocol.is_jim084() {
        return None;
    }
    let components = checked_namespace_path_utf8(path).ok()?;
    let spelling = format!("::{}", components.join("::"));
    let projected = protocol
        .namespace_address_input(NativeNameContext::root(), spelling.as_bytes())
        .ok()?;
    let selected = ByteNamespacePath::from_segments(qualifier_segments(projected.selected()));
    (selected == *path).then_some(spelling)
}

/// A globally addressable Jim namespace spelling from its actual flat object,
/// independently of C-style analytical paths and Jim's flat command keys.
#[must_use]
pub fn native_jim_namespace_source_spelling(actual_namespace_object: &[u8]) -> Option<String> {
    let namespace = std::str::from_utf8(actual_namespace_object).ok()?;
    let spelling = format!("::{namespace}");
    let projected = NativeNameProtocol::Jim084
        .namespace_address_input(NativeNameContext::root(), spelling.as_bytes())
        .ok()?;
    (projected.selected() == actual_namespace_object).then_some(spelling)
}

#[cfg(test)]
mod tests {
    use super::*;
    const VERSIONS: [TclVersion; 5] = [
        TclVersion::V8_4,
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ];

    #[test]
    fn command_variable_trace_package_formal_and_hidden_extents_are_distinct() {
        let full = b"k\0z";
        for version in VERSIONS {
            let protocol = NativeNameProtocol::C(version);
            assert_eq!(
                protocol
                    .command_lookup_input(NativeNameContext::root(), full)
                    .unwrap()
                    .selected(),
                b"k"
            );
            assert_eq!(
                protocol.variable_root_input(full).selected(),
                if version == TclVersion::V8_4 {
                    b"k".as_slice()
                } else {
                    full
                }
            );
            assert_eq!(
                protocol.trace_registration_input(full).unwrap().selected(),
                b"k"
            );
            assert_eq!(protocol.package_key(full).selected(), b"k");
            assert_eq!(protocol.formal_name_input(full).selected(), b"k");
            assert_eq!(protocol.hidden_token_input(full).unwrap().selected(), b"k");
            assert_eq!(
                report_native_name_bytes(
                    protocol,
                    NativeNameReportPurpose::TraceCallbackName1,
                    full
                )
                .unwrap(),
                b"k"
            );
            assert_eq!(
                protocol.variable_root_input(b"k\xc0\x80z").selected(),
                b"k\xc0\x80z"
            );
            let late = protocol.variable_root_input(b"k\0z::c");
            assert_eq!(late.qualification(), NativeNameQualification::Unqualified);
            assert_eq!(
                late.selected(),
                if version == TclVersion::V8_4 {
                    b"k".as_slice()
                } else {
                    b"k\0z::c"
                }
            );
            assert_eq!(
                protocol.variable_root_input(b"::ns::k\0z").selected(),
                b"::ns::k"
            );
            assert_eq!(
                protocol
                    .command_lookup_input(NativeNameContext::root(), b"k\xff")
                    .unwrap()
                    .selected(),
                b"k\xff"
            );
        }
        let jim = NativeNameProtocol::Jim084;
        assert_eq!(
            jim.command_lookup_input(NativeNameContext::root(), full)
                .unwrap()
                .jim_flat_key(),
            Some(full.as_slice())
        );
        assert_eq!(jim.variable_root_input(full).selected(), full);
        assert_eq!(jim.formal_name_input(full).selected(), full);
        assert_eq!(jim.package_key(full).selected(), b"k");
        assert_eq!(
            jim.trace_registration_input(full).unwrap_err(),
            NameProjectionUnavailable::PurposeNotModelled
        );
        assert_eq!(
            jim.hidden_token_input(full).unwrap_err(),
            NameProjectionUnavailable::PurposeNotModelled
        );
    }

    #[test]
    fn formal_storage_enumeration_and_namespace_upvar_locals_have_distinct_purposes() {
        for version in VERSIONS {
            let protocol = NativeNameProtocol::C(version);
            assert_eq!(
                protocol.formal_enumeration_name_input(b"k\0z").selected(),
                b"k"
            );
            assert_eq!(
                protocol.formal_storage_name_input(b"k\0z").selected(),
                if version <= TclVersion::V8_5 {
                    b"k".as_slice()
                } else {
                    b"k\0z"
                }
            );
            assert_eq!(
                protocol.formal_storage_name_input(b"k\xc0\x80z").selected(),
                b"k\xc0\x80z"
            );
            if version == TclVersion::V8_4 {
                assert!(protocol.namespace_upvar_local_input(b"a\0z").is_err());
            } else {
                assert_eq!(
                    protocol
                        .namespace_upvar_local_input(b"a\0z")
                        .unwrap()
                        .selected(),
                    b"a"
                );
                assert_eq!(protocol.variable_root_input(b"a\0z").selected(), b"a\0z");
                assert_eq!(
                    protocol
                        .namespace_upvar_local_input(b"a\xff")
                        .unwrap()
                        .selected(),
                    b"a\xff"
                );
            }
        }
        let jim = NativeNameProtocol::Jim084;
        assert_eq!(jim.formal_storage_name_input(b"k\0z").selected(), b"k\0z");
        assert_eq!(
            jim.formal_enumeration_name_input(b"k\0z").selected(),
            b"k\0z"
        );
        assert_eq!(
            jim.namespace_upvar_local_input(b"a\0z").unwrap().selected(),
            b"a\0z"
        );
    }

    #[test]
    fn constant_binding_failures_have_separate_parts_codes_and_verbs() {
        use NativeVariableDiagnosticOperation::MakeConstant;
        use NativeVariableDiagnosticReason::{MissingParentNamespace, NoSuchElement, NotArray};
        use NativeVariableFailureSite::NameLookup;
        for version in [TclVersion::V9_0, TclVersion::V9_1] {
            let protocol = NativeNameProtocol::C(version);
            assert_eq!(
                native_constant_failure_verb(protocol, NameLookup).unwrap(),
                "const"
            );
            assert_eq!(
                native_constant_failure_verb(protocol, NativeVariableFailureSite::ValueWrite)
                    .unwrap(),
                "make constant"
            );
            for (original, reason, expected, kind, key) in [
                (
                    b"::missing::k\0z".as_slice(),
                    MissingParentNamespace,
                    b"::missing::k".as_slice(),
                    b"VARNAME".as_slice(),
                    b"::missing::k".as_slice(),
                ),
                (
                    b"a(k\0z)".as_slice(),
                    NotArray,
                    b"a(k)".as_slice(),
                    b"VARNAME".as_slice(),
                    b"a".as_slice(),
                ),
                (
                    b"a(k\0z)".as_slice(),
                    NoSuchElement,
                    b"a(k)".as_slice(),
                    b"ELEMENT".as_slice(),
                    b"k".as_slice(),
                ),
                (
                    b"a(k\xc0\x80z)".as_slice(),
                    NoSuchElement,
                    b"a(k\xc0\x80z)".as_slice(),
                    b"ELEMENT".as_slice(),
                    b"k\xc0\x80z".as_slice(),
                ),
            ] {
                let result = report_native_variable_diagnostic_at(
                    protocol,
                    MakeConstant,
                    reason,
                    NameLookup,
                    NativeVariableInputForm::Combined(original),
                )
                .unwrap();
                assert_eq!(result.name, expected);
                assert_eq!(
                    result.error_code,
                    Some(vec![
                        b"TCL".to_vec(),
                        b"LOOKUP".to_vec(),
                        kind.to_vec(),
                        key.to_vec()
                    ])
                );
            }
        }
    }

    #[test]
    fn constant_declaration_reports_original_operand_and_its_own_tuple() {
        use NativeVariableDiagnosticOperation::MakeConstant;
        use NativeVariableDiagnosticReason::{AlreadyExists, ArrayElement, IsArray};
        use NativeVariableFailureSite::ValueWrite;
        for version in [TclVersion::V9_0, TclVersion::V9_1] {
            for (original, reason, expected) in [
                (b"a(k\0z)".as_slice(), ArrayElement, b"a(k".as_slice()),
                (b"a\0z".as_slice(), AlreadyExists, b"a".as_slice()),
                (b"a\xc0\x80z".as_slice(), IsArray, b"a\xc0\x80z".as_slice()),
                (b"a\xff".as_slice(), AlreadyExists, b"a\xff".as_slice()),
            ] {
                let result = report_native_variable_diagnostic_at(
                    NativeNameProtocol::C(version),
                    MakeConstant,
                    reason,
                    ValueWrite,
                    NativeVariableInputForm::Combined(original),
                )
                .unwrap();
                assert_eq!(result.name, expected);
                assert_eq!(
                    result.error_code,
                    Some(vec![b"TCL".to_vec(), b"LOOKUP".to_vec(), b"CONST".to_vec()])
                );
                assert_eq!(result.missing_lookup_root, None);
            }
        }
        for protocol in [
            NativeNameProtocol::C(TclVersion::V8_6),
            NativeNameProtocol::Jim084,
        ] {
            assert!(
                report_native_variable_diagnostic_at(
                    protocol,
                    MakeConstant,
                    AlreadyExists,
                    ValueWrite,
                    NativeVariableInputForm::Combined(b"a")
                )
                .is_err()
            );
        }
        assert!(
            report_native_variable_diagnostic_at(
                NativeNameProtocol::C(TclVersion::V9_0),
                MakeConstant,
                AlreadyExists,
                NativeVariableFailureSite::NameLookup,
                NativeVariableInputForm::Combined(b"a")
            )
            .is_err()
        );
    }

    #[test]
    fn increment_constant_failure_has_its_own_write_disposition() {
        for version in [TclVersion::V9_0, TclVersion::V9_1] {
            let projection = report_native_variable_diagnostic_at(
                NativeNameProtocol::C(version),
                NativeVariableDiagnosticOperation::Increment,
                NativeVariableDiagnosticReason::Constant,
                NativeVariableFailureSite::ValueWrite,
                NativeVariableInputForm::Combined(b"k\0suffix"),
            )
            .unwrap();
            assert_eq!(projection.name, b"k");
            assert_eq!(
                projection.error_code,
                Some(vec![b"TCL".to_vec(), b"WRITE".to_vec(), b"CONST".to_vec()])
            );
        }
        assert!(
            report_native_variable_diagnostic_at(
                NativeNameProtocol::C(TclVersion::V8_6),
                NativeVariableDiagnosticOperation::Increment,
                NativeVariableDiagnosticReason::Constant,
                NativeVariableFailureSite::ValueWrite,
                NativeVariableInputForm::Combined(b"k"),
            )
            .is_err()
        );
    }

    fn assert_constant_variable_diagnostics(protocol: NativeNameProtocol) {
        use NativeVariableDiagnosticOperation::{Unset, Write};
        use NativeVariableDiagnosticReason::Constant;
        use NativeVariableFailureSite::{ValueUnset, ValueWrite};
        let write = report_native_variable_diagnostic_at(
            protocol,
            Write,
            Constant,
            ValueWrite,
            NativeVariableInputForm::Combined(b"k\0z"),
        )
        .unwrap();
        let unset = report_native_variable_diagnostic_at(
            protocol,
            Unset,
            Constant,
            ValueUnset,
            NativeVariableInputForm::Combined(b"k\0z"),
        )
        .unwrap();
        assert_eq!(write.name, b"k");
        assert_eq!(
            write.error_code,
            Some(vec![
                b"TCL".to_vec(),
                b"WRITE".to_vec(),
                b"VARNAME".to_vec()
            ])
        );
        assert_eq!(
            unset.error_code,
            Some(vec![b"TCL".to_vec(), b"UNSET".to_vec(), b"CONST".to_vec()])
        );
    }

    #[test]
    fn variable_diagnostics_retain_original_input_and_actual_failure_site() {
        use NativeVariableDiagnosticOperation::{Read, Unset};
        use NativeVariableDiagnosticReason::{NoSuchElement, NotArray};
        use NativeVariableFailureSite::{NameLookup, ValueRead};
        for version in VERSIONS {
            let protocol = NativeNameProtocol::C(version);
            let input = NativeVariableInputForm::Combined(b"a(k\0z)");
            let read = report_native_variable_diagnostic_at(
                protocol,
                Read,
                NoSuchElement,
                ValueRead,
                input,
            )
            .unwrap();
            let unset = report_native_variable_diagnostic_at(
                protocol,
                Unset,
                NoSuchElement,
                NameLookup,
                input,
            )
            .unwrap();
            let scalar =
                report_native_variable_diagnostic_at(protocol, Read, NotArray, NameLookup, input)
                    .unwrap();
            assert_eq!(read.name, b"a(k");
            assert_eq!(unset.name, b"a(k)");
            assert_eq!(scalar.name, b"a(k)");
            if version >= TclVersion::V8_6 {
                assert_eq!(
                    read.error_code,
                    Some(vec![b"TCL".to_vec(), b"READ".to_vec(), b"VARNAME".to_vec()])
                );
                assert_eq!(
                    unset.error_code,
                    Some(vec![
                        b"TCL".to_vec(),
                        b"LOOKUP".to_vec(),
                        b"ELEMENT".to_vec(),
                        b"k".to_vec()
                    ])
                );
                assert_eq!(scalar.missing_lookup_root, Some(b"a".to_vec()));
            } else {
                assert_eq!(read.error_code, None);
            }
            let control = report_native_variable_diagnostic_at(
                protocol,
                Read,
                NoSuchElement,
                ValueRead,
                NativeVariableInputForm::Combined(b"a(k\xc0\x80z)"),
            )
            .unwrap();
            assert_eq!(control.name, b"a(k\xc0\x80z)");
            let separate = report_native_variable_diagnostic_at(
                protocol,
                Read,
                NoSuchElement,
                ValueRead,
                NativeVariableInputForm::Separate {
                    root: b"a",
                    element: Some(b"k\0z"),
                },
            )
            .unwrap();
            assert_eq!(separate.name, b"a(k)");
            if version >= TclVersion::V9_0 {
                assert_constant_variable_diagnostics(protocol);
            }
        }
        let jim = NativeNameProtocol::Jim084;
        let input = NativeVariableInputForm::Combined(b"a(k\0z)");
        let read = report_native_variable_diagnostic_at(jim, Read, NoSuchElement, ValueRead, input)
            .unwrap();
        let unset =
            report_native_variable_diagnostic_at(jim, Unset, NotArray, NameLookup, input).unwrap();
        assert_eq!(read.name, b"a(k)");
        assert_eq!(unset.name, b"a(k");
        assert_eq!(unset.reason, NoSuchElement);
        assert_eq!(unset.error_code, None);
    }

    #[test]
    fn jim_evaluated_element_read_failures_keep_the_selected_root_and_key() {
        use NativeVariableDiagnosticOperation::{Read, Write};
        use NativeVariableDiagnosticReason::{NoSuchElement, NoSuchVariable, NotArray};
        use NativeVariableFailureSite::NameLookup;
        let input = NativeVariableInputForm::Separate {
            root: b"arr",
            element: Some(b"missing"),
        };
        for (reason, expected) in [
            (NoSuchElement, b"arr(missing)".as_slice()),
            (NotArray, b"arr(missing)".as_slice()),
            (NoSuchVariable, b"arr".as_slice()),
        ] {
            let projection = report_native_variable_diagnostic_at(
                NativeNameProtocol::Jim084,
                Read,
                reason,
                NameLookup,
                input,
            )
            .unwrap();
            assert_eq!(projection.name, expected);
            assert_eq!(projection.reason, reason);
            assert_eq!(projection.error_code, None);
        }
        assert!(
            report_native_variable_diagnostic_at(
                NativeNameProtocol::Jim084,
                Write,
                NotArray,
                NameLookup,
                input,
            )
            .is_err()
        );
    }

    #[test]
    fn combined_and_separate_elements_keep_native_input_form_and_release() {
        for version in VERSIONS {
            let protocol = NativeNameProtocol::C(version);
            let combined = protocol.combined_variable_input(b"a(k\0z)");
            let separated = protocol.separate_variable_input(b"a", Some(b"k\0z"));
            assert_eq!(combined.root().selected(), b"a");
            assert_eq!(
                combined.element().unwrap().selected(),
                if version < TclVersion::V9_0 {
                    b"k".as_slice()
                } else {
                    b"k\0z"
                }
            );
            assert_eq!(
                separated.element().unwrap().selected(),
                if version == TclVersion::V8_4 {
                    b"k".as_slice()
                } else {
                    b"k\0z"
                }
            );
            assert_eq!(
                combined.original(),
                NativeVariableInputForm::Combined(b"a(k\0z)")
            );
            assert_eq!(
                separated.original(),
                NativeVariableInputForm::Separate {
                    root: b"a",
                    element: Some(b"k\0z")
                }
            );
        }
    }

    #[test]
    fn c_api_publication_and_script_declaration_retain_different_contexts() {
        let path = ByteNamespacePath::from_segments([b"n".as_slice()]);
        for version in VERSIONS {
            let protocol = NativeNameProtocol::C(version);
            assert_eq!(
                protocol
                    .command_publication_slot(NativeNameContext::new(&path), b"x")
                    .unwrap(),
                ByteCommandSlot::new(path.clone(), NameBytes::from(b"x"))
            );
            assert_eq!(
                protocol
                    .command_c_api_publication_slot(NativeNameContext::new(&path), b"x\0z")
                    .unwrap(),
                ByteCommandSlot::new(ByteNamespacePath::root(), NameBytes::from(b"x"))
            );
            assert_eq!(
                protocol
                    .command_c_api_publication_slot(NativeNameContext::new(&path), b"child::x")
                    .unwrap(),
                ByteCommandSlot::new(path.with_child(b"child"), NameBytes::from(b"x"))
            );
        }
    }

    #[test]
    fn jim_context_composition_does_not_borrow_c_namespace_parsing() {
        let path = ByteNamespacePath::from_segments([b"src".as_slice()]);
        let context = NativeNameContext::with_jim_namespace(&path, b"src");
        let jim = NativeNameProtocol::Jim084;
        let relative = jim.command_publication_input(context, b"k\0z").unwrap();
        assert_eq!(relative.original(), b"k\0z");
        assert_eq!(relative.selected(), b"src::k");
        let absolute = jim
            .command_publication_input(context, b"::src::k\0z")
            .unwrap();
        assert_eq!(absolute.selected(), b"::src::k\0z");
        assert_eq!(absolute.jim_flat_key(), Some(b"src::k\0z".as_slice()));
        assert_eq!(
            jim.jim_command_lookup_keys(context, b"k\0z").unwrap(),
            vec![NameBytes::from(b"src::k"), NameBytes::from(b"k\0z")]
        );
        assert_eq!(
            jim.namespace_address_input(context, b"k\0z")
                .unwrap()
                .selected(),
            b"src::k\0z"
        );
        assert_eq!(
            jim.namespace_address_input(context, b"::k\0z")
                .unwrap()
                .selected(),
            b"k"
        );
        assert_eq!(
            jim.command_publication_input(NativeNameContext::new(&path), b"x")
                .unwrap_err(),
            NameProjectionUnavailable::MissingJimNamespaceObject
        );
    }

    #[test]
    fn analytical_slot_projection_never_reconstructs_opaque_identity() {
        let colon = ByteCommandSlot::new(
            ByteNamespacePath::from_segments([b"a:".as_slice()]),
            NameBytes::from(b"p"),
        );
        assert_eq!(
            checked_command_slot_utf8(&colon).unwrap().namespace,
            vec!["a:"]
        );
        assert_eq!(native_command_full_name_bytes(&colon), b"::a:::p");
        assert_eq!(
            native_command_source_spelling(NativeNameProtocol::C(TclVersion::V9_0), &colon),
            None
        );
        let raw = ByteCommandSlot::new(ByteNamespacePath::root(), NameBytes::from(b"k\xff"));
        assert_eq!(native_command_full_name_bytes(&raw), b"::k\xff");
        assert!(checked_command_slot_utf8(&raw).is_err());
        let nul = ByteCommandSlot::new(ByteNamespacePath::root(), NameBytes::from(b"k\0z"));
        assert_eq!(native_command_full_name_bytes(&nul), b"::k\0z");
        assert!(checked_command_slot_utf8(&nul).is_ok());
        assert_eq!(
            native_command_source_spelling(NativeNameProtocol::C(TclVersion::V9_0), &nul),
            None
        );
        assert_eq!(
            native_command_source_spelling(NativeNameProtocol::Jim084, &nul).as_deref(),
            Some("::k\0z")
        );
        let ordinary = ByteCommandSlot::new(
            ByteNamespacePath::from_segments([b"n".as_slice()]),
            NameBytes::from(b"p"),
        );
        assert_eq!(
            native_command_source_spelling(NativeNameProtocol::C(TclVersion::V9_0), &ordinary)
                .as_deref(),
            Some("::n::p")
        );
    }

    #[test]
    fn namespace_reports_and_source_aliases_do_not_replace_constructed_identity() {
        for version in VERSIONS {
            let c = NativeNameProtocol::C(version);
            assert_eq!(c.namespace_tail_bytes(b"::k:z"), b"k:z");
            assert_eq!(c.namespace_qualifier_bytes(b"::k:z"), b"");
            assert_eq!(c.namespace_tail_bytes(b"k\0z::c"), b"k");
            assert_eq!(c.namespace_qualifier_bytes(b"a:::b"), b"a");
            assert_eq!(
                native_namespace_source_spelling(
                    c,
                    &ByteNamespacePath::from_segments([b"a:".as_slice()])
                ),
                Some(String::from("::a:"))
            );
            assert_eq!(
                native_namespace_source_spelling(
                    c,
                    &ByteNamespacePath::from_segments([b"a".as_slice()])
                ),
                Some(String::from("::a"))
            );
        }
        let jim = NativeNameProtocol::Jim084;
        assert_eq!(jim.namespace_tail_bytes(b"::k:z"), b"::k:z");
        assert_eq!(jim.namespace_tail_bytes(b"k\0z::c"), b"k\0z::c");
        assert_eq!(jim.namespace_qualifier_bytes(b"a:::b"), b"a:");
        assert_eq!(native_jim_namespace_source_spelling(b"a\0z"), None);
        assert_eq!(
            native_jim_namespace_source_spelling(b"a:::b"),
            Some(String::from("::a:::b"))
        );
        assert_eq!(
            native_namespace_source_spelling(jim, &ByteNamespacePath::root()),
            Some(String::from("::"))
        );
    }

    #[test]
    fn array_receiver_selection_and_trace_reporting_keep_original_and_cell_parts() {
        for version in VERSIONS {
            let c = NativeNameProtocol::C(version);
            let input = c.combined_variable_input(b"a\0z(k)");
            if version < TclVersion::V9_0 {
                assert_eq!(
                    input.root().selected(),
                    if version == TclVersion::V8_4 {
                        b"a".as_slice()
                    } else {
                        b"a\0z"
                    }
                );
                assert_eq!(input.element().unwrap().selected(), b"k");
            } else {
                assert_eq!(input.root().selected(), b"a\0z(k)");
                assert!(input.element().is_none());
            }
            let trace = report_native_variable_access_trace_names(
                c,
                NativeVariableTraceReportingInput {
                    original_part1: b"a(k\0z)",
                    original_part2: None,
                    actual_element_key: Some(b"k\0z"),
                },
            )
            .unwrap();
            assert_eq!(trace.name1, b"a(k");
            assert_eq!(
                trace.name2,
                if version < TclVersion::V9_0 {
                    b"".as_slice()
                } else {
                    b"k"
                }
            );
            let separated = report_native_variable_access_trace_names(
                c,
                NativeVariableTraceReportingInput {
                    original_part1: b"a",
                    original_part2: Some(b"k\0z"),
                    actual_element_key: Some(b"k\0z"),
                },
            )
            .unwrap();
            assert_eq!(
                separated,
                NativeVariableTraceNames {
                    name1: b"a",
                    name2: b"k"
                }
            );
            let modified = report_native_variable_access_trace_names(
                c,
                NativeVariableTraceReportingInput {
                    original_part1: b"a(k\xc0\x80z)",
                    original_part2: None,
                    actual_element_key: Some(b"k\xc0\x80z"),
                },
            )
            .unwrap();
            assert_eq!(
                modified,
                NativeVariableTraceNames {
                    name1: b"a",
                    name2: b"k\xc0\x80z"
                }
            );
        }
        assert!(
            NativeNameProtocol::Jim084
                .combined_variable_input(b"a\0z(k)")
                .element()
                .is_none()
        );
    }

    #[test]
    fn native_terminal_colon_namespace_keeps_lookup_context_and_release_publication() {
        let namespace = ByteNamespacePath::from_segments([b"a:".as_slice()]);
        let context = NativeNameContext::new(&namespace);
        for version in VERSIONS {
            let c = NativeNameProtocol::C(version);
            assert_eq!(
                c.command_lookup_slot(context, b"p").unwrap(),
                ByteCommandSlot::new(namespace.clone(), NameBytes::from("p"))
            );
            let published = c.command_publication_slot(context, b"p").unwrap();
            assert_eq!(
                published.namespace,
                if version <= TclVersion::V8_5 {
                    ByteNamespacePath::from_segments(["a"])
                } else {
                    namespace.clone()
                }
            );
            assert_eq!(published.simple, "p");
            assert_eq!(
                c.command_c_api_publication_slot(context, b"p").unwrap(),
                ByteCommandSlot::new(ByteNamespacePath::root(), NameBytes::from("p"))
            );
        }
    }

    #[test]
    fn variable_diagnostics_report_parts_and_keep_lookup_keys_separate() {
        for version in VERSIONS {
            let c = NativeNameProtocol::C(version);
            for operation in [
                NativeVariableDiagnosticOperation::Read,
                NativeVariableDiagnosticOperation::Unset,
            ] {
                for (input, expected, key) in [
                    (b"k\0z".as_slice(), b"k".as_slice(), b"k".as_slice()),
                    (b"a(k\0z)", b"a(k)", b"a"),
                    (b"N\0z::k", b"N", b"N"),
                    (b"N\xff::k", b"N\xff::k", b"N\xff::k"),
                ] {
                    let reported = report_native_variable_diagnostic(
                        c,
                        operation,
                        NativeVariableDiagnosticReason::NoSuchVariable,
                        NativeVariableInputForm::Combined(input),
                    )
                    .unwrap();
                    assert_eq!(reported.name, expected);
                    assert_eq!(
                        reported.missing_lookup_root,
                        (version >= TclVersion::V8_6).then(|| key.to_vec())
                    );
                }
            }
        }
        let jim_read = report_native_variable_diagnostic(
            NativeNameProtocol::Jim084,
            NativeVariableDiagnosticOperation::Read,
            NativeVariableDiagnosticReason::NoSuchVariable,
            NativeVariableInputForm::Combined(b"a(k\0z)"),
        )
        .unwrap();
        assert_eq!(jim_read.name, b"a");
        let jim_unset = report_native_variable_diagnostic(
            NativeNameProtocol::Jim084,
            NativeVariableDiagnosticOperation::Unset,
            NativeVariableDiagnosticReason::NotArray,
            NativeVariableInputForm::Combined(b"a(k\0z)"),
        )
        .unwrap();
        assert_eq!(jim_unset.name, b"a(k");
    }

    #[test]
    fn authored_provider_never_authenticates_vendor_native_name_recipes() {
        let vendor =
            DialectPoint::of_name_and_release(Some("f5-irules"), None).expect("known vendor");
        assert_eq!(NativeNameProtocol::for_point(vendor), None);
        assert_eq!(NativeStringProtocol::for_point(vendor), None);
        assert_eq!(NamePolicyProtocol::for_native_point(vendor), None);
        let logical = NamePolicyProtocol::authored_tcl(TclVersion::V8_4);
        assert_eq!(logical.authority(), NamePolicyAuthority::AuthoredSimulation);
        assert_eq!(logical.recipe(), NativeNameProtocol::C(TclVersion::V8_4));
        assert_eq!(
            NamePolicyProtocol::for_native_point(DialectPoint::for_tcl_version(TclVersion::V9_1))
                .unwrap()
                .authority(),
            NamePolicyAuthority::Native
        );
    }
}
#[test]
fn jim_helper_canonicalisation_keeps_relative_object_extent() {
    let path = ByteNamespacePath::root();
    let context = NativeNameContext::with_jim_namespace(&path, b"n\0z");
    let canonical = NativeNameProtocol::Jim084
        .jim_namespace_canonical_input(context, b"p\0q")
        .unwrap();
    assert_eq!(canonical.selected(), b"n\0z::p\0q");
    let absolute = NativeNameProtocol::Jim084
        .jim_namespace_canonical_input(context, b":::p\0q")
        .unwrap();
    assert_eq!(absolute.selected(), b"p");
    let published = NativeNameProtocol::Jim084
        .command_publication_input(context, b"p\0q")
        .unwrap();
    assert_ne!(canonical.selected(), published.selected());
    assert!(
        NativeNameProtocol::C(TclVersion::V9_0)
            .jim_namespace_canonical_input(context, b"p")
            .is_err()
    );
}

#[test]
fn ensemble_defaults_keep_actual_parent_after_85() {
    let path = ByteNamespacePath::from_segments([b"a:".as_slice(), b"q".as_slice()]);
    let context = NativeNameContext::new(&path);
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        let slot = NativeNameProtocol::C(version)
            .ensemble_publication_slot(context, None)
            .unwrap();
        assert_eq!(
            slot.namespace,
            ByteNamespacePath::from_segments([b"a:".as_slice()])
        );
        assert_eq!(slot.simple.as_bytes(), b"q");
    }
    let old = NativeNameProtocol::C(TclVersion::V8_5)
        .ensemble_publication_slot(context, None)
        .unwrap();
    assert_ne!(
        old.namespace,
        ByteNamespacePath::from_segments([b"a:".as_slice()])
    );
    assert!(
        NativeNameProtocol::C(TclVersion::V8_4)
            .ensemble_publication_slot(context, None)
            .is_err()
    );
}

#[test]
fn namespace_query_reporting_and_procedure_context_keep_distinct_extents() {
    for version in TclVersion::ALL {
        let c = NativeNameProtocol::C(version);
        assert_eq!(c.namespace_variable_query_input(b"k\0z").selected(), b"k");
        if version == TclVersion::V8_4 {
            assert!(report_native_namespace_lookup_error(c, b"k\0z", b"::n").is_err());
        } else {
            let report = report_native_namespace_lookup_error(c, b"k\0z", b"::n\xff").unwrap();
            assert_eq!(report.message, b"namespace \"k\" not found in \"::n\xff\"");
            assert_eq!(report.error_code, b"TCL LOOKUP NAMESPACE k");
            let absolute = report_native_namespace_lookup_error(c, b"::k\0z", b"::n").unwrap();
            assert_eq!(absolute.message, b"namespace \"::k\" not found");
        }
        assert_eq!(
            c.namespace_variable_query_input(b"k\xc0\x80z").selected(),
            b"k\xc0\x80z"
        );
    }
    let jim = NativeNameProtocol::Jim084;
    assert_eq!(
        jim.namespace_variable_query_input(b"k\0z").selected(),
        b"k\0z"
    );
    assert_eq!(jim.namespace_qualifier_bytes(b"x\0z::p"), b"");
    assert_eq!(jim.jim_procedure_namespace(b"x\0z::p").unwrap(), b"x\0z");
    assert_eq!(
        jim.jim_namespace_parent_bytes(b"absent::child").unwrap(),
        b"::absent"
    );
    assert_eq!(jim.jim_namespace_parent_bytes(b"absent").unwrap(), b"::");
    assert!(report_native_namespace_lookup_error(jim, b"absent", b"::").is_err());
}

#[test]
fn namespace_operation_failures_keep_release_and_operation_reporting() {
    use NativeNamespaceLookupOperation as Operation;
    for version in TclVersion::ALL {
        let c = NativeNameProtocol::C(version);
        let parent =
            report_native_namespace_operation_error(c, Operation::Parent, b"n\0z", b"::ctx")
                .unwrap();
        if version == TclVersion::V8_4 {
            assert_eq!(
                parent.message,
                b"unknown namespace \"n\" in namespace parent command"
            );
            assert_eq!(parent.error_code, None);
        } else {
            assert_eq!(parent.message, b"namespace \"n\" not found in \"::ctx\"");
            assert_eq!(
                parent.error_code.as_deref(),
                Some(b"TCL LOOKUP NAMESPACE n".as_slice())
            );
        }
        let deleted =
            report_native_namespace_operation_error(c, Operation::Delete, b"n\0z", b"::ctx")
                .unwrap();
        assert_eq!(
            deleted.message,
            b"unknown namespace \"n\" in namespace delete command"
        );
    }
    assert_eq!(
        NativeNameProtocol::Jim084
            .jim_namespace_parent_bytes(b"::\0z")
            .unwrap(),
        b""
    );
}

#[test]
fn dictionary_key_diagnostics_do_not_project_counted_identity() {
    use NativeDictionaryMissingKeyOperation::{Get, UnsetIntermediate};
    for version in [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        let recipe = NativeNameProtocol::C(version);
        for operation in [Get, UnsetIntermediate] {
            for (original, reported) in [
                (b"a\0z".as_slice(), b"a".as_slice()),
                (b"a\xc0\x80z".as_slice(), b"a\xc0\x80z".as_slice()),
                (b"a\xffz".as_slice(), b"a\xffz".as_slice()),
            ] {
                let report =
                    report_native_dictionary_missing_key(recipe, operation, original).unwrap();
                let mut expected = b"key \"".to_vec();
                expected.extend_from_slice(reported);
                expected.extend_from_slice(b"\" not known in dictionary");
                assert_eq!(report.message, expected);
                assert_eq!(
                    report.error_code.is_some(),
                    version >= TclVersion::V9_0 || operation == UnsetIntermediate
                );
                if let Some(tuple) = report.error_code {
                    let mut expected = b"TCL LOOKUP DICT ".to_vec();
                    crate::list::append_list_element(&mut expected, reported, false);
                    assert_eq!(tuple, expected);
                }
            }
        }
    }
    assert!(
        report_native_dictionary_missing_key(NativeNameProtocol::C(TclVersion::V8_4), Get, b"a")
            .is_err()
    );
    let jim =
        report_native_dictionary_missing_key(NativeNameProtocol::Jim084, Get, b"a\0z").unwrap();
    assert_eq!(jim.message, b"key \"a\" not known in dictionary");
    assert!(jim.error_code.is_none());
}

#[test]
fn native_oo_counted_method_and_cstring_object_publication_are_distinct() {
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        let recipe = NativeNameProtocol::C(version);
        for original in [
            b"a\0z".as_slice(),
            b"a\xffz".as_slice(),
            b"a\xc0\x80z".as_slice(),
        ] {
            let method = recipe.oo_method_input(original).unwrap();
            assert_eq!(method.original(), original);
            assert_eq!(method.selected(), original);
            assert_eq!(method.purpose(), NativeNamePurpose::OoMethod);
            let object = recipe
                .oo_object_publication_slot(NativeNameContext::root(), original)
                .unwrap();
            assert_eq!(object.simple.as_bytes(), c_string_extent(original));
            assert_eq!(
                report_native_name_bytes(
                    recipe,
                    NativeNameReportPurpose::OoMethodEnumeration,
                    original
                )
                .unwrap(),
                c_string_extent(original)
            );
        }
        let path = ByteNamespacePath::from_segments([NameBytes::from(b"a:".as_slice())]);
        let object = recipe
            .oo_object_publication_slot(NativeNameContext::new(&path), b"p")
            .unwrap();
        assert_eq!(object.namespace, path);
    }
    for recipe in [
        NativeNameProtocol::C(TclVersion::V8_4),
        NativeNameProtocol::C(TclVersion::V8_5),
        NativeNameProtocol::Jim084,
    ] {
        assert!(recipe.oo_method_input(b"m").is_err());
        assert!(
            recipe
                .oo_object_publication_slot(NativeNameContext::root(), b"O")
                .is_err()
        );
        assert!(
            report_native_name_bytes(recipe, NativeNameReportPurpose::OoMethodEnumeration, b"m")
                .is_err()
        );
    }
}

#[test]
fn alias_publication_retains_its_distinct_native_entry_point() {
    let namespace = ByteNamespacePath::from_segments(["n"]);
    let context = NativeNameContext::new(&namespace);
    for version in TclVersion::ALL {
        let c = NativeNameProtocol::C(version);
        assert_eq!(
            c.alias_publication_slot(context, b"x").unwrap(),
            ByteCommandSlot::new(ByteNamespacePath::root(), NameBytes::from("x"))
        );
        assert_eq!(
            c.alias_publication_slot(context, b"q::x").unwrap(),
            ByteCommandSlot::new(
                ByteNamespacePath::from_segments(["n", "q"]),
                NameBytes::from("x")
            )
        );
        assert_eq!(
            c.alias_publication_slot(context, b"::q::x").unwrap(),
            ByteCommandSlot::new(
                ByteNamespacePath::from_segments(["q"]),
                NameBytes::from("x")
            )
        );
        assert_eq!(
            c.alias_publication_slot(context, b"x\0z")
                .unwrap()
                .simple
                .as_bytes(),
            b"x"
        );
        assert_eq!(
            c.alias_publication_slot(context, b"x\xc0\x80z")
                .unwrap()
                .simple
                .as_bytes(),
            b"x\xc0\x80z"
        );
    }
    let jim = NativeNameProtocol::Jim084;
    for name in [b"x".as_slice(), b"q::x", b"::q::x", b"x\0z", b"x\xff"] {
        let slot = jim.alias_publication_slot(context, name).unwrap();
        assert!(slot.namespace.is_root());
        assert_eq!(slot.simple.as_bytes(), name);
        assert_eq!(
            jim.alias_publication_input(context, name)
                .unwrap()
                .jim_flat_key(),
            Some(name)
        );
    }
}

#[test]
fn jim_namespace_construction_retains_original_object_only_for_root_relative_names() {
    use NativeJimNamespaceConstruction::{
        DuplicateNamespaceAndAppend, FreshString, RetainOriginal,
    };
    let jim = NativeNameProtocol::Jim084;
    for name in [&b"a\0z"[..], &b"a\xffz"[..], &b""[..]] {
        assert_eq!(
            jim.jim_namespace_construction(NativeNameContext::root(), name)
                .unwrap(),
            RetainOriginal
        );
    }
    assert_eq!(
        jim.jim_namespace_construction(NativeNameContext::root(), b"::a\0z")
            .unwrap(),
        FreshString
    );
    let path = ByteNamespacePath::root();
    let context = NativeNameContext::with_jim_namespace(&path, b"n\0z");
    assert_eq!(
        jim.jim_namespace_construction(context, b"a").unwrap(),
        DuplicateNamespaceAndAppend
    );
    assert_eq!(
        jim.jim_namespace_construction(context, b"::a").unwrap(),
        FreshString
    );
}

#[test]
fn rename_alias_loop_diagnostic_uses_selected_release_and_original_slot() {
    for version in TclVersion::ALL {
        let selected = NativeNameProtocol::C(version)
            .rename_alias_loop_name(b"source\0OPAQUE", b"destination\0OPAQUE");
        assert_eq!(
            selected,
            Some(if version == TclVersion::V8_4 {
                b"source".as_slice()
            } else {
                b"destination".as_slice()
            })
        );
    }
    assert_eq!(
        NativeNameProtocol::Jim084.rename_alias_loop_name(b"source", b"destination"),
        None
    );
}

#[test]
fn empty_relative_namespace_address_does_not_select_nonroot_caller() {
    let mut parent = ByteNamespacePath::root();
    parent.push(b"outer");
    for version in [
        TclVersion::V8_4,
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        let recipe = NativeNameProtocol::C(version);
        for written in [b"".as_slice(), b"\0suffix"] {
            assert!(
                recipe
                    .namespace_address_path(NativeNameContext::new(&parent), written)
                    .is_err()
            );
            assert_eq!(
                recipe
                    .namespace_address_path(NativeNameContext::root(), written)
                    .unwrap(),
                ByteNamespacePath::root()
            );
        }
        assert_eq!(
            recipe
                .namespace_address_path(NativeNameContext::new(&parent), b"::outer::")
                .unwrap(),
            parent
        );
    }
}
