// SPDX-License-Identifier: AGPL-3.0-or-later
//! Purpose-specific native name inputs and reporting.
//!
//! A projection preserves original operands and selects an operation's byte
//! extent. It grants no namespace/frame identity, receiver lifetime, alias
//! closure, command generation, compiler permission or observer proof.

use super::{is_qualified, qualifier_segments, written_command_tail};

mod routing;
use crate::native_string::NativeStringProtocol;
pub use routing::{NativeCommandNamespaceRoute, NativeCommandSlotProjection};
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

/// Selected direct namespace text result path, without a live object capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeNamespaceTextResult<'a> {
    /// Jim qualifiers with no paired colon return the actual interpreter empty object.
    SharedEmpty,
    /// Jim's unchanged input object, preserving its actual header and identity.
    Original,
    /// C8.4 appends the selected bytes to the fresh interpreter result.
    Append(&'a [u8]),
    /// A fresh counted byte object selected by C8.5+ or Jim.
    Counted(&'a [u8]),
}

/// Jim's textual parent result branch, without a namespace lookup or object role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeJimNamespaceParentResult<'a> {
    /// The primitive leaves the interpreter's current result untouched.
    Unchanged,
    /// Return the original counted qualifier bytes in a fresh String object.
    CountedQualifier(&'a [u8]),
    /// Publish a root prefix, then append a fresh counted qualifier object.
    RootedQualifier(&'a [u8]),
    /// Publish a root prefix, then append the interpreter's actual empty object.
    RootedSharedEmpty,
}

fn jim_namespace_qualifier(original: &[u8]) -> Option<&[u8]> {
    let prefix = c_string_extent(original);
    match prefix.iter().rposition(|byte| *byte == b':') {
        Some(last) if last > 0 && prefix[last - 1] == b':' => Some(&prefix[..last - 1]),
        _ => None,
    }
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
    /// Variable table/listing semantics of this sealed naming purpose.
    /// The provider's authority remains independent of this pure projection.
    #[must_use]
    pub const fn variable_lookup_policy(self) -> tcl_dialect::VariableLookupPolicy {
        self.recipe.variable_lookup_policy()
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
    /// Coroutine declaration in an existing current-namespace holder.
    CoroutinePublication,
    CommandCApiPublication,
    /// C visibility restoration into the actual global command table.
    ExposedCommandPublication,
    RenameSource,
    RenameDestination,
    AliasPublication,
    /// C alias-record lookup, independent of the command's movable slot.
    AliasRegistrationQuery,
    /// Child-interpreter alias extension registration, including Jim's C-string constructor.
    ChildAliasPublication,
    NamespaceAddress,
    /// C lambda namespace object construction before namespace lookup.
    LambdaNamespace,
    /// Counted Jim lambda namespace object, without lookup or canonicalisation.
    JimLambdaNamespace,
    /// Jim helper canonicalisation before flat command enumeration.
    JimNamespaceCanonical,
    /// Namespace ensemble command publication, independently of proc creation.
    EnsemblePublication,
    /// Namespace command's native subcommand index operand.
    NamespaceSubcommand,
    /// Interpreter dispatch table index, separate from its command operand.
    InterpreterSubcommand,
    /// A C interpreter child-table key; colon bytes are ordinary name bytes.
    InterpreterChild,
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
    /// Formal declaration list splitting, independently of decoded name storage.
    FormalParameterList,
    FormalStorage,
    NamespaceUpvarLocal,
    NamespaceExportPattern,
    NamespaceImportPattern,
    NamespaceForgetPattern,
    HiddenToken,
    /// `TclOO` counted method-table key, separate from command publication.
    OoMethod,
    /// Formatted configurable property option, separate from counted names.
    OoPropertyOption,
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
    /// Qualification selected from this purpose's original input extent.
    pub qualification: NativeNameQualification,
    pub source: Option<NativeNamespacePatternSource>,
    pub tail: NameBytes,
    pub purpose: NativeNamePurpose,
}

/// A fresh Jim namespace-command-query operand and its independent report.
/// Neither spelling grants a command token or a publication address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeJimNamespaceCommandQueryCandidate {
    pub lookup: NameBytes,
    pub reported: NameBytes,
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

/// A name projection refusal, including an independently selected invalid address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameProjectionUnavailable {
    MissingJimNamespaceObject,
    PurposeNotModelled,
    /// A selected empty relative C address cannot denote a non-root namespace.
    EmptyNamespaceOutsideRoot,
}

impl std::fmt::Display for NameProjectionUnavailable {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::MissingJimNamespaceObject => "native Jim namespace object is unavailable",
            Self::PurposeNotModelled => "native name purpose is not modelled for this engine",
            Self::EmptyNamespaceOutsideRoot => "empty namespace name outside the root namespace",
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
    /// Borrowed selected extent of the original input. A projection that
    /// constructs new bytes cannot supply original operand geometry.
    #[must_use]
    pub fn borrowed_selected(&self) -> Option<&'a [u8]> {
        match &self.selected {
            Cow::Borrowed(bytes) => Some(*bytes),
            Cow::Owned(_) => None,
        }
    }
    /// Report extent retained independently of the actual registration slot.
    /// C interp aliases keeps the original counted alias-record name; Jim's
    /// child extension reports its newly constructed registration spelling.
    /// This projection supplies bytes only, never an alias object or inventory.
    #[must_use]
    pub fn alias_registration_report_bytes(&self) -> Option<&[u8]> {
        if !matches!(
            self.purpose,
            NativeNamePurpose::AliasPublication | NativeNamePurpose::ChildAliasPublication
        ) {
            return None;
        }
        Some(if self.protocol.is_jim084() {
            self.selected()
        } else {
            self.original()
        })
    }

    /// C8.4's alias-record coordinate, independent of the command's current slot.
    /// Later C releases keep distinct registration tokens; Jim has no C table.
    ///
    /// # Errors
    /// Refuses operands that were not selected for alias publication.
    pub fn alias_registration_replacement_key(
        &self,
    ) -> Result<Option<&[u8]>, NameProjectionUnavailable> {
        if !matches!(
            self.purpose,
            NativeNamePurpose::AliasPublication | NativeNamePurpose::ChildAliasPublication
        ) {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        Ok((self.protocol == NativeNameProtocol::C(TclVersion::V8_4))
            .then(|| c_string_extent(self.original)))
    }

    /// C alias-record coordinate for registration or query. Jim has no C table.
    ///
    /// # Errors
    /// Refuses an operand selected for another naming purpose.
    pub fn alias_registration_key(&self) -> Result<Option<&[u8]>, NameProjectionUnavailable> {
        if !matches!(
            self.purpose,
            NativeNamePurpose::AliasPublication
                | NativeNamePurpose::ChildAliasPublication
                | NativeNamePurpose::AliasRegistrationQuery
        ) {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        Ok(matches!(self.protocol, NativeNameProtocol::C(_))
            .then(|| c_string_extent(self.original)))
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
        is_command_purpose(self.purpose)
            .then(|| {
                self.protocol
                    .jim_command_table_comparison(&self.selected)
                    .ok()
            })
            .flatten()
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
    /// Variable namespace/activation selection for an already selected recipe.
    /// Catalogue compatibility supplies no recipe or execution authority.
    #[must_use]
    pub const fn variable_lookup_policy(self) -> tcl_dialect::VariableLookupPolicy {
        match self {
            Self::C(_) => tcl_dialect::VariableLookupPolicy::Tcl,
            Self::Jim084 => tcl_dialect::VariableLookupPolicy::Jim,
        }
    }

    /// Construct a C ensemble map-prefix head from its original counted bytes.
    /// A rooted head is unchanged; a relative head appends to the actual
    /// namespace report, with a separator only when that namespace has a parent.
    /// Parent presence is supplied by the live owner, never inferred from the
    /// report's spelling. Construction grants no command slot or later lookup.
    ///
    /// # Errors
    /// Refuses C8.4 and Jim, whose original commands do not supply this C purpose.
    pub fn ensemble_map_prefix_input<'a>(
        self,
        namespace_report: &[u8],
        namespace_has_parent: bool,
        original: &'a [u8],
    ) -> Result<Cow<'a, [u8]>, NameProjectionUnavailable> {
        if !matches!(self, Self::C(version) if version >= TclVersion::V8_5) {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        if original.starts_with(b"::") {
            return Ok(Cow::Borrowed(original));
        }
        let mut qualified = namespace_report.to_vec();
        if namespace_has_parent {
            qualified.extend_from_slice(b"::");
        }
        qualified.extend_from_slice(original);
        Ok(Cow::Owned(qualified))
    }

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

    /// C child-interpreter tables use the `CString` extent of an independently
    /// decoded path element. This supplies no interpreter existence or command
    /// publication; Jim's handle factory has a separate protocol.
    ///
    /// # Errors
    /// Returns an unavailable purpose for Jim's different interpreter API.
    pub fn interpreter_child_input(
        self,
        original: &[u8],
    ) -> Result<NativeNameProjection<'_>, NameProjectionUnavailable> {
        if self.is_jim084() {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        Ok(projection(
            self,
            NativeNamePurpose::InterpreterChild,
            original,
            Cow::Borrowed(c_string_extent(original)),
            None,
        ))
    }

    /// Pure recipe selection; does not authenticate a native command or object.
    #[must_use]
    pub const fn for_tcl_version(version: TclVersion) -> Self {
        Self::C(version)
    }
    /// Pure C command-table teardown order for this selected name policy.
    /// Jim has no C namespace command-table recipe; no holder is issued here.
    #[must_use]
    pub const fn namespace_command_teardown(
        self,
    ) -> Option<crate::native_namespace_name::NativeNamespaceCommandTeardown> {
        match self.tcl_version() {
            Some(version) => Some(
                crate::native_namespace_name::NativeNamespaceNameRecipe::for_tcl_version(version)
                    .command_teardown(),
            ),
            None => None,
        }
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
        self.rename_destination_projection(context, original)
            .map(NativeCommandSlotProjection::into_slot)
    }
    /// C8.4/8.5's original `interp alias` reaches the target String getter
    /// before retaining the alias operands. Later C recipes retain and publish
    /// first, then reach the target during loop lookup. Jim's core alias does
    /// not read the target at definition. This is stage data, not admission.
    #[must_use]
    pub const fn alias_target_string_precedes_capture(self) -> bool {
        matches!(self, Self::C(TclVersion::V8_4 | TclVersion::V8_5))
    }

    /// The empty-target pretest belongs only to the selected C8.4/8.5 stage.
    /// `None` means that operation is absent, not a guessed nonempty target.
    #[must_use]
    pub fn alias_pretested_target_is_empty(self, original: &[u8]) -> Option<bool> {
        self.alias_target_string_precedes_capture()
            .then(|| c_string_extent(original).is_empty())
    }

    /// Every C child-handle alias reads the target before capture. This
    /// selected entry is separate from the `interp alias` ensemble split.
    #[must_use]
    pub const fn child_alias_target_string_precedes_capture(self) -> bool {
        matches!(self, Self::C(_))
    }

    /// The child-handle empty-target check consumes the selected C String
    /// extent. Jim's child alias has no such pretest.
    #[must_use]
    pub fn child_alias_pretested_target_is_empty(self, original: &[u8]) -> Option<bool> {
        self.child_alias_target_string_precedes_capture()
            .then(|| c_string_extent(original).is_empty())
    }

    /// C child-handle aliases return the original registration name. The
    /// Jim child extension leaves its empty result in place.
    #[must_use]
    pub const fn child_alias_returns_original_registration_name(self) -> bool {
        matches!(self, Self::C(_))
    }

    /// C alias records own the original registration name. Jim owns its
    /// command-table key separately, and its child String-name constructor
    /// does not retain the original caller name.
    #[must_use]
    pub const fn alias_record_retains_original_registration_name(self) -> bool {
        matches!(self, Self::C(_))
    }

    /// C reaches the original target after publication to reject alias loops;
    /// Jim admits loops and leaves all target/prefix Strings lazy until use.
    #[must_use]
    pub const fn alias_publication_checks_original_target(self) -> bool {
        matches!(self, Self::C(_))
    }

    /// C query creates a fresh List retaining the original members. Jim's
    /// `info alias` returns its stored List header and therefore its own cache.
    #[must_use]
    pub const fn alias_query_returns_original_list(self) -> bool {
        matches!(self, Self::Jim084)
    }

    /// Jim sets the original name result before registering the command;
    /// C returns its retained registration token after successful publication.
    #[must_use]
    pub const fn alias_result_precedes_name_registration(self) -> bool {
        matches!(self, Self::Jim084)
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

    /// Select the original C alias-table operand without namespace resolution.
    ///
    /// # Errors
    /// Jim's alias queries use its independent command-object lookup.
    pub fn alias_registration_query_input(
        self,
        original: &[u8],
    ) -> Result<NativeNameProjection<'_>, NameProjectionUnavailable> {
        match self {
            Self::C(_) => Ok(projection(
                self,
                NativeNamePurpose::AliasRegistrationQuery,
                original,
                Cow::Borrowed(c_string_extent(original)),
                None,
            )),
            Self::Jim084 => Err(NameProjectionUnavailable::PurposeNotModelled),
        }
    }

    /// Construct a later-C alias record token after an occupied registration.
    /// The complete getter result survives; its C-string table key is separate.
    ///
    /// # Errors
    /// C8.4 deletes the colliding token and Jim has no C alias table.
    pub fn alias_registration_collision_token(
        self,
        original: &[u8],
    ) -> Result<Vec<u8>, NameProjectionUnavailable> {
        if !matches!(self, Self::C(version) if version >= TclVersion::V8_5) {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        let mut token = Vec::with_capacity(original.len().saturating_add(2));
        token.extend_from_slice(b"::");
        token.extend_from_slice(original);
        Ok(token)
    }

    /// Child alias extensions construct their command name independently of the
    /// standalone alias command. Jim's Jim_RegisterCmd receives Jim_String and
    /// builds a new -1-length string at the child root; original counted input
    /// remains separate from the selected C-string extent and report spelling.
    /// C uses its existing alias-registration rule in the selected child realm.
    ///
    /// # Errors
    /// Refuses an unavailable registration purpose or context.
    pub fn child_alias_publication_input<'a>(
        self,
        context: NativeNameContext<'a>,
        original: &'a [u8],
    ) -> Result<NativeNameProjection<'a>, NameProjectionUnavailable> {
        match self {
            Self::C(_) => {
                let mut selected = self.alias_publication_input(context, original)?;
                selected.purpose = NativeNamePurpose::ChildAliasPublication;
                Ok(selected)
            }
            Self::Jim084 => Ok(projection(
                self,
                NativeNamePurpose::ChildAliasPublication,
                original,
                Cow::Borrowed(c_string_extent(original)),
                Some(NativeNameContext::root()),
            )),
        }
    }

    /// Alias commands publish separately from procedure declarations. C keeps
    /// an unqualified name global and retains the current holder for relative
    /// qualified names. Jim retains the complete original naming object at its
    /// global table, while shared table comparison strips leading root colons.
    ///
    /// # Errors
    /// Refuses an unsupported native publication projection.
    pub fn alias_publication_slot(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<ByteCommandSlot, NameProjectionUnavailable> {
        self.alias_publication_projection(context, original)
            .map(NativeCommandSlotProjection::into_slot)
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
        self.command_publication_projection(context, original)
            .map(NativeCommandSlotProjection::into_slot)
    }

    /// Publication geometry of Jim's factory-generated two-word commands.
    /// The provider interpolates the original counted class-name value, one
    /// space and the selected member units before the independently selected
    /// procedure or alias publication. This
    /// pure slot grants no class identity, installed command or live method.
    ///
    /// # Errors
    /// Refuses other engines, unsupported publication purposes and a missing
    /// actual Jim namespace object for procedure publication.
    pub fn jim_two_word_member_publication_slot(
        self,
        context: NativeNameContext<'_>,
        class_name: &[u8],
        member: &[u8],
        purpose: NativeNamePurpose,
    ) -> Result<ByteCommandSlot, NameProjectionUnavailable> {
        if !self.is_jim084() {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        let mut name = class_name.to_vec();
        name.push(b' ');
        name.extend_from_slice(member);
        match purpose {
            NativeNamePurpose::CommandPublication => self.command_publication_slot(context, &name),
            NativeNamePurpose::AliasPublication => self.alias_publication_slot(context, &name),
            _ => Err(NameProjectionUnavailable::PurposeNotModelled),
        }
    }

    /// Coroutine publication selects a `CString` command name in the actual
    /// current holder. Runtime traversal must refuse missing qualifiers and
    /// retain exact namespace tokens; this pure slot creates no namespace.
    /// Empty names and trailing separators select the holder's empty command.
    ///
    /// # Errors
    /// Refuses releases without the audited C coroutine command.
    pub fn coroutine_publication_slot(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<ByteCommandSlot, NameProjectionUnavailable> {
        if !matches!(self, Self::C(version) if version >= TclVersion::V8_6) {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        self.coroutine_publication_projection(context, original)
            .map(NativeCommandSlotProjection::into_slot)
    }

    /// Report refusal of a coroutine's missing qualifier namespace, retaining
    /// the original name separately from the native formatter's `CString`.
    ///
    /// # Errors
    /// Refuses releases without the audited C coroutine command.
    pub fn coroutine_unknown_namespace_error(
        self,
        original: &[u8],
    ) -> Result<NativeNamespaceLookupError, NameProjectionUnavailable> {
        if !matches!(self, Self::C(version) if version >= TclVersion::V8_6) {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        let mut message = b"can't create procedure \"".to_vec();
        message.extend_from_slice(c_string_extent(original));
        message.extend_from_slice(b"\": unknown namespace");
        Ok(NativeNamespaceLookupError {
            message,
            error_code: b"TCL LOOKUP NAMESPACE".to_vec(),
        })
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
        self.oo_object_publication_projection(context, original)
            .map(NativeCommandSlotProjection::into_slot)
    }

    /// Public collision message for an original TclOO object publication.
    /// The caller independently supplies the actual occupied slot and result producer.
    ///
    /// # Errors
    /// Refuses engines without audited TclOO object publication.
    pub fn oo_object_collision_message(
        self,
        original: &[u8],
    ) -> Result<Vec<u8>, NameProjectionUnavailable> {
        if !matches!(self, Self::C(version) if version >= TclVersion::V8_6) {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        let mut message = b"can't create object \"".to_vec();
        message.extend_from_slice(c_string_extent(original));
        message.extend_from_slice(b"\": command already exists with that name");
        Ok(message)
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
        let mut parent = context.namespace.clone();
        parent.pop();
        self.ensemble_publication_projection(context, explicit, Some(&parent))
            .map(NativeCommandSlotProjection::into_slot)
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
        self.command_lookup_projection(context, original)
            .map(NativeCommandSlotProjection::into_slot)
    }
    ///
    /// # Errors
    /// Returns an error for a missing actual Jim namespace object or an unaudited engine operation.
    pub fn command_c_api_publication_slot(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<ByteCommandSlot, NameProjectionUnavailable> {
        self.command_c_api_publication_projection(context, original)
            .map(NativeCommandSlotProjection::into_slot)
    }
    /// Jim's command hash comparison keeps counted bytes and removes leading root markers.
    /// This supplies no live header, namespace or table-entry authority.
    ///
    /// # Errors
    /// Returns unavailable for an engine without Jim's command hash protocol.
    pub fn jim_command_table_comparison(
        self,
        original: &[u8],
    ) -> Result<&[u8], NameProjectionUnavailable> {
        if !self.is_jim084() {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        Ok(strip_jim_root(original))
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

    /// C creation without a delete callback returns its selected trailing parent.
    /// The caller must independently retain that actual namespace token; this
    /// recipe supplies neither its existence nor creation or frame authority.
    ///
    /// # Errors
    /// Jim and a C namespace delete callback require independent creation recipes.
    pub fn namespace_creation_reuses_parent(
        self,
        original: &[u8],
        has_delete_callback: bool,
    ) -> Result<bool, NameProjectionUnavailable> {
        if self.is_jim084() || has_delete_callback {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        Ok(c_string_extent(original).ends_with(b"::"))
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

    /// Retain the counted original Jim lambda namespace object's bytes.
    /// The actual List child and procedure/frame object remain independent;
    /// this projection neither looks up nor creates a namespace.
    ///
    /// # Errors
    /// Refuses C lambda constructors with their independent namespace recipe.
    pub fn jim_lambda_namespace_input(
        self,
        original: &[u8],
    ) -> Result<NativeNameProjection<'_>, NameProjectionUnavailable> {
        if !self.is_jim084() {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        Ok(projection(
            self,
            NativeNamePurpose::JimLambdaNamespace,
            original,
            Cow::Borrowed(original),
            None,
        ))
    }

    /// Original C lambda namespace object bytes. An unrooted value receives
    /// an exact :: prefix before lookup; its counted suffix remains retained.
    /// Namespace lookup applies its independent C-string extent afterwards.
    /// This recipe supplies no object, namespace existence or entered frame.
    ///
    /// # Errors
    /// C Tcl 8.4 has no Apply entry; Jim requires its own original object owner.
    pub fn lambda_namespace_input(
        self,
        original: &[u8],
    ) -> Result<NativeNameProjection<'_>, NameProjectionUnavailable> {
        if !matches!(self, Self::C(version) if version >= TclVersion::V8_5) {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        let selected = if original.starts_with(b"::") {
            Cow::Borrowed(original)
        } else {
            let mut rooted = Vec::from(b"::".as_slice());
            rooted.extend_from_slice(original);
            Cow::Owned(rooted)
        };
        Ok(projection(
            self,
            NativeNamePurpose::LambdaNamespace,
            original,
            selected,
            Some(NativeNameContext::root()),
        ))
    }

    /// C lambda namespace address after its own object-construction recipe.
    /// The original counted input remains separate from the lookup extent.
    ///
    /// # Errors
    /// Refuses providers without the audited C lambda namespace recipe.
    pub fn lambda_namespace_path(
        self,
        original: &[u8],
    ) -> Result<ByteNamespacePath, NameProjectionUnavailable> {
        let input = self.lambda_namespace_input(original)?;
        self.namespace_address_path(NativeNameContext::root(), input.selected())
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
            return Err(NameProjectionUnavailable::EmptyNamespaceOutsideRoot);
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

    /// Select Jim's textual parent producer without consulting namespace tables.
    /// Original object getters and the immediate root-literal comparison remain
    /// independent reached operations of the consumer.
    ///
    /// # Errors
    /// C parent lookup requires an actual namespace token.
    pub fn jim_namespace_parent_result(
        self,
        original: &[u8],
    ) -> Result<NativeJimNamespaceParentResult<'_>, NameProjectionUnavailable> {
        if !self.is_jim084() {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        if original.is_empty() || c_string_extent(original) == b"::" {
            return Ok(NativeJimNamespaceParentResult::Unchanged);
        }
        let Some(parent) = jim_namespace_qualifier(original) else {
            return Ok(NativeJimNamespaceParentResult::RootedSharedEmpty);
        };
        Ok(if parent.starts_with(b"::") {
            NativeJimNamespaceParentResult::CountedQualifier(parent)
        } else {
            NativeJimNamespaceParentResult::RootedQualifier(parent)
        })
    }

    /// Jim's textual parent query does not require a namespace object to exist.
    ///
    /// # Errors
    /// C parent lookup requires an actual namespace token.
    pub fn jim_namespace_parent_bytes(
        self,
        original: &[u8],
    ) -> Result<Vec<u8>, NameProjectionUnavailable> {
        match self.jim_namespace_parent_result(original)? {
            NativeJimNamespaceParentResult::Unchanged => Ok(Vec::new()),
            NativeJimNamespaceParentResult::CountedQualifier(bytes) => Ok(bytes.to_vec()),
            NativeJimNamespaceParentResult::RootedSharedEmpty => Ok(b"::".to_vec()),
            NativeJimNamespaceParentResult::RootedQualifier(bytes) => {
                let mut rooted = b"::".to_vec();
                rooted.extend_from_slice(bytes);
                Ok(rooted)
            }
        }
    }

    /// Native namespace-tail reporting. Jim tests only its last `CString` colon
    /// and returns the complete original object when that colon is not paired.
    #[must_use]
    pub fn namespace_tail_bytes(self, original: &[u8]) -> &[u8] {
        match self.namespace_text_result(original, true) {
            NativeNamespaceTextResult::Original => original,
            NativeNamespaceTextResult::SharedEmpty => b"",
            NativeNamespaceTextResult::Append(bytes)
            | NativeNamespaceTextResult::Counted(bytes) => bytes,
        }
    }

    /// Direct worker result production, separate from compiled Unicode arithmetic.
    /// C8.4 uses counted append; later C releases use a fresh byte object. Jim's
    /// unpaired Tail returns the original object and never reconstructs it.
    #[must_use]
    // Native proof: naming.namespace.original-counted-tail-compiler-and-runtime
    // docs/design/analysis/name-resolution-proofs/original-counted-tail-compiler-and-runtime.md
    pub fn namespace_text_result(
        self,
        original: &[u8],
        tail: bool,
    ) -> NativeNamespaceTextResult<'_> {
        if self.is_jim084() && !tail {
            if jim_namespace_qualifier(original).is_none() {
                return NativeNamespaceTextResult::SharedEmpty;
            }
        }
        let bytes = if tail {
            let prefix = c_string_extent(original);
            if self.is_jim084() {
                let Some(last) = prefix.iter().rposition(|byte| *byte == b':') else {
                    return NativeNamespaceTextResult::Original;
                };
                if last == 0 || prefix[last - 1] != b':' {
                    return NativeNamespaceTextResult::Original;
                }
                &prefix[last + 1..]
            } else {
                written_command_tail(prefix)
            }
        } else {
            self.namespace_qualifier_bytes(original)
        };
        let append_reached = if tail {
            !c_string_extent(original).is_empty()
        } else {
            !bytes.is_empty()
        };
        if self == Self::C(TclVersion::V8_4) && append_reached {
            NativeNamespaceTextResult::Append(bytes)
        } else {
            NativeNamespaceTextResult::Counted(bytes)
        }
    }
    /// Native namespace-qualifier reporting, independent of namespace lookup.
    #[must_use]
    pub fn namespace_qualifier_bytes(self, original: &[u8]) -> &[u8] {
        let prefix = c_string_extent(original);
        if self.is_jim084() {
            return jim_namespace_qualifier(original).unwrap_or(&prefix[..0]);
        }
        super::written_separator_extent(prefix)
            .map_or(&prefix[..0], |separator| &prefix[..separator.start])
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
            let slot = routing::command_slot_projection(&input)?.into_slot();
            (
                Some(NativeNamespacePatternSource::C(slot.namespace)),
                slot.simple,
            )
        };
        Ok(NativeNamespacePatternParts {
            original: original.into(),
            selected,
            qualification: input.qualification(),
            source,
            tail,
            purpose,
        })
    }

    /// Select Jim's absolute canonicalisation without reading namespace bytes.
    /// Relative operands return `None` and require the actual namespace object.
    /// This byte projection supplies no namespace holder or frame authority.
    ///
    /// # Errors
    /// Refuses a non-Jim recipe.
    pub fn jim_namespace_absolute_canonical_input(
        self,
        original: &[u8],
    ) -> Result<Option<NativeNameProjection<'_>>, NameProjectionUnavailable> {
        if !self.is_jim084() {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        Ok(original.starts_with(b"::").then(|| {
            projection(
                self,
                NativeNamePurpose::JimNamespaceCanonical,
                original,
                Cow::Borrowed(c_string_extent(strip_jim_root(original))),
                None,
            )
        }))
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
        let selected =
            if let Some(absolute) = self.jim_namespace_absolute_canonical_input(original)? {
                absolute.selected
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

    /// Jim `namespace which -command` tries fresh rooted canonical and original
    /// operands in that order, reporting the successful candidate's bytes.
    ///
    /// # Errors
    /// Refuses C recipes or an absent actual Jim namespace object.
    pub fn jim_namespace_command_query_candidates(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<[NativeJimNamespaceCommandQueryCandidate; 2], NameProjectionUnavailable> {
        let canonical = self.jim_namespace_canonical_input(context, original)?;
        self.jim_namespace_command_query_candidates_from_canonical(&canonical)
    }

    /// Construct Jim's query candidates from its selected canonical projection.
    /// Namespace and entered-frame authentication remain with the caller.
    ///
    /// # Errors
    /// Refuses a projection from another engine or naming purpose.
    pub fn jim_namespace_command_query_candidates_from_canonical(
        self,
        canonical: &NativeNameProjection<'_>,
    ) -> Result<[NativeJimNamespaceCommandQueryCandidate; 2], NameProjectionUnavailable> {
        if !self.is_jim084()
            || canonical.protocol() != self
            || canonical.purpose() != NativeNamePurpose::JimNamespaceCanonical
        {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        let candidate = |name: &[u8]| {
            let mut rooted = b"::".to_vec();
            rooted.extend_from_slice(name);
            NativeJimNamespaceCommandQueryCandidate {
                lookup: NameBytes::from(rooted.as_slice()),
                reported: NameBytes::from(rooted.as_slice()),
            }
        };
        Ok([
            candidate(canonical.selected()),
            candidate(canonical.original()),
        ])
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

    /// Select the actual Jim command-table key constructor. An occupied slot
    /// keeps its original key; this selects only the incoming constructor role.
    /// Alias registration uses the supplied object, while the child/public
    /// CString API constructs a fresh object. Procedure and rename qualification
    /// duplicate the genuine namespace object before appending CString units.
    ///
    /// # Errors
    /// Refuses other engines, unsupported purposes or a missing namespace owner.
    pub fn jim_command_key_construction(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
        purpose: NativeNamePurpose,
    ) -> Result<NativeJimNamespaceConstruction, NameProjectionUnavailable> {
        if !self.is_jim084() {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        match purpose {
            NativeNamePurpose::AliasPublication => {
                Ok(NativeJimNamespaceConstruction::RetainOriginal)
            }
            NativeNamePurpose::ChildAliasPublication
            | NativeNamePurpose::CommandCApiPublication => {
                Ok(NativeJimNamespaceConstruction::FreshString)
            }
            NativeNamePurpose::CommandPublication
            | NativeNamePurpose::RenameSource
            | NativeNamePurpose::RenameDestination => {
                let namespace = context
                    .jim_namespace_object
                    .ok_or(NameProjectionUnavailable::MissingJimNamespaceObject)?;
                Ok(if namespace.is_empty() || original.starts_with(b"::") {
                    NativeJimNamespaceConstruction::RetainOriginal
                } else {
                    NativeJimNamespaceConstruction::DuplicateNamespaceAndAppend
                })
            }
            _ => Err(NameProjectionUnavailable::PurposeNotModelled),
        }
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
    /// Input to each formal declaration list level. C8.4/8.5 split a
    /// `CString`; modern C and Jim retain counted list objects. This pure
    /// extent recipe grants neither successful parsing nor an activation.
    #[must_use]
    pub fn formal_parameter_list_input(self, original: &[u8]) -> NativeNameProjection<'_> {
        let selected = if matches!(self, Self::C(version) if version <= TclVersion::V8_5) {
            c_string_extent(original)
        } else {
            original
        };
        projection(
            self,
            NativeNamePurpose::FormalParameterList,
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

    /// C9 property declarations form their option name with `"-%s"` from
    /// the declaration's C string. The original counted name stays separate.
    /// This recipe grants no property installation, accessor or lookup cache.
    ///
    /// # Errors
    /// Returns an unavailable purpose outside the audited C9 property releases.
    pub fn oo_property_option_name(
        self,
        original: &[u8],
    ) -> Result<NativeNameProjection<'_>, NameProjectionUnavailable> {
        if !matches!(self, Self::C(version) if version >= TclVersion::V9_0) {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        let mut option = vec![b'-'];
        option.extend_from_slice(c_string_extent(original));
        Ok(projection(
            self,
            NativeNamePurpose::OoPropertyOption,
            original,
            Cow::Owned(option),
            None,
        ))
    }

    /// Global C visibility destination, preserving an empty hash key.
    /// Qualification is rejected before publication; Jim has no exposed table.
    ///
    /// # Errors
    /// Refuses unaudited engines and namespace-qualified destinations.
    pub fn exposed_command_publication_slot(
        self,
        original: &[u8],
    ) -> Result<tcl_core_types::ByteCommandSlot, NameProjectionUnavailable> {
        if self.is_jim084() || is_qualified(c_string_extent(original)) {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        let selected = projection(
            self,
            NativeNamePurpose::ExposedCommandPublication,
            original,
            Cow::Borrowed(c_string_extent(original)),
            None,
        );
        Ok(tcl_core_types::ByteCommandSlot::new(
            tcl_core_types::ByteNamespacePath::root(),
            tcl_core_types::NameBytes::from(selected.selected()),
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
            | NativeNamePurpose::FormalParameterList
            | NativeNamePurpose::FormalStorage
            | NativeNamePurpose::JimLambdaNamespace
            | NativeNamePurpose::NamespaceUpvarLocal
            | NativeNamePurpose::HiddenToken
            | NativeNamePurpose::InterpreterChild
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
            | NativeNamePurpose::CoroutinePublication
            | NativeNamePurpose::CommandCApiPublication
            | NativeNamePurpose::RenameSource
            | NativeNamePurpose::RenameDestination
            | NativeNamePurpose::AliasPublication
            | NativeNamePurpose::ChildAliasPublication
            | NativeNamePurpose::OoObjectPublication
    )
}

/// The failed stage selected by the actual rename operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeRenameFailure {
    /// The source was absent when requesting a nonempty destination.
    MissingSource,
    /// The source was absent when requesting deletion.
    MissingDeleteSource,
    /// The destination already has a command binding.
    TargetExists,
}

/// Selected rename result bytes and independently selected explicit error code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeRenameDiagnostic {
    /// Exact native diagnostic bytes.
    pub message: Vec<u8>,
    /// Explicit native tuple; absence uses the adapter's neutral error path.
    pub error_code: Option<Vec<u8>>,
}

impl NativeNameProtocol {
    /// Project an actual rename refusal without selecting or mutating bindings.
    /// Original counted operands remain independent of Jim's report extent.
    #[must_use]
    pub fn rename_diagnostic(
        self,
        failure: NativeRenameFailure,
        original: &[u8],
    ) -> NativeRenameDiagnostic {
        let reported = if self.is_jim084() {
            c_string_extent(original)
        } else {
            original
        };
        let (prefix, suffix) = match failure {
            NativeRenameFailure::MissingSource => (
                b"can't rename \"".as_slice(),
                b"\": command doesn't exist".as_slice(),
            ),
            NativeRenameFailure::MissingDeleteSource => (
                b"can't delete \"".as_slice(),
                b"\": command doesn't exist".as_slice(),
            ),
            NativeRenameFailure::TargetExists => (
                b"can't rename to \"".as_slice(),
                b"\": command already exists".as_slice(),
            ),
        };
        let message = [prefix, reported, suffix].concat();
        let error_code = if matches!(self, Self::C(version) if version >= TclVersion::V8_6) {
            Some(match failure {
                NativeRenameFailure::MissingSource | NativeRenameFailure::MissingDeleteSource => {
                    let mut code = b"TCL LOOKUP COMMAND ".to_vec();
                    crate::list::append_list_element(&mut code, original, false);
                    code
                }
                NativeRenameFailure::TargetExists => b"TCL OPERATION RENAME TARGET_EXISTS".to_vec(),
            })
        } else {
            None
        };
        NativeRenameDiagnostic {
            message,
            error_code,
        }
    }
}

/// Actual native hide/expose failure, independent of command lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCommandVisibilityFailure {
    /// Hidden destination token contains a namespace qualifier.
    InvalidHiddenToken,
    /// Selected command to hide is outside the global namespace.
    HideNonGlobal,
    /// Requested exposed destination contains a namespace qualifier.
    ExposeNonGlobal,
    /// Actual hidden source is outside the global namespace.
    HiddenSourceNonGlobal,
    /// Visible source command was absent.
    MissingCommand,
    /// Hidden source token was absent during exposure.
    MissingHiddenToken,
    /// Hidden invocation token was absent.
    InvokeHiddenMissing,
    /// Hidden destination token was occupied.
    AlreadyHidden,
    /// Visible destination command was occupied.
    CommandExists,
}

/// Native visibility diagnostic bytes with the selected explicit tuple.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeCommandVisibilityDiagnostic {
    /// Exact native diagnostic bytes.
    pub message: Vec<u8>,
    /// Explicit native tuple; absence uses the adapter's neutral error path.
    pub error_code: Option<Vec<u8>>,
}

impl NativeNameProtocol {
    /// Project an actual C hide/expose refusal without selecting any binding.
    ///
    /// # Errors
    /// Jim has no C hidden-command operation.
    pub fn command_visibility_diagnostic(
        self,
        failure: NativeCommandVisibilityFailure,
        original: &[u8],
    ) -> Result<NativeCommandVisibilityDiagnostic, NameProjectionUnavailable> {
        let Self::C(version) = self else {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        };
        let name = c_string_extent(original);
        let (message, code) = match failure {
            NativeCommandVisibilityFailure::InvalidHiddenToken => (
                b"cannot use namespace qualifiers in hidden command token (rename)".to_vec(),
                b"TCL VALUE HIDDENTOKEN".as_slice(),
            ),
            NativeCommandVisibilityFailure::HideNonGlobal => (
                b"can only hide global namespace commands (use rename then hide)".to_vec(),
                b"TCL HIDE NON_GLOBAL".as_slice(),
            ),
            NativeCommandVisibilityFailure::ExposeNonGlobal => (
                if version == TclVersion::V8_4 {
                    b"can not expose to a namespace (use expose to toplevel, then rename)".to_vec()
                } else {
                    b"cannot expose to a namespace (use expose to toplevel, then rename)".to_vec()
                },
                b"TCL EXPOSE NON_GLOBAL".as_slice(),
            ),
            NativeCommandVisibilityFailure::HiddenSourceNonGlobal => (
                if version <= TclVersion::V8_5 {
                    b"trying to expose a non global command name space command".to_vec()
                } else {
                    b"trying to expose a non-global command namespace command".to_vec()
                },
                b"NONE".as_slice(),
            ),
            NativeCommandVisibilityFailure::MissingCommand => (
                [b"unknown command \"".as_slice(), name, b"\""].concat(),
                b"TCL LOOKUP COMMAND".as_slice(),
            ),
            NativeCommandVisibilityFailure::MissingHiddenToken => (
                [b"unknown hidden command \"".as_slice(), name, b"\""].concat(),
                b"TCL LOOKUP HIDDENTOKEN".as_slice(),
            ),
            NativeCommandVisibilityFailure::InvokeHiddenMissing => (
                [b"invalid hidden command name \"".as_slice(), name, b"\""].concat(),
                b"TCL LOOKUP HIDDENTOKEN".as_slice(),
            ),
            NativeCommandVisibilityFailure::AlreadyHidden => (
                [
                    b"hidden command named \"".as_slice(),
                    name,
                    b"\" already exists",
                ]
                .concat(),
                b"TCL HIDE ALREADY_HIDDEN".as_slice(),
            ),
            NativeCommandVisibilityFailure::CommandExists => (
                [b"exposed command \"".as_slice(), name, b"\" already exists"].concat(),
                b"TCL EXPOSE COMMAND_EXISTS".as_slice(),
            ),
        };
        let error_code = ((version >= TclVersion::V8_6
            || (version == TclVersion::V8_5
                && failure == NativeCommandVisibilityFailure::MissingCommand))
            && failure != NativeCommandVisibilityFailure::HiddenSourceNonGlobal)
            .then(|| {
                let mut tuple = code.to_vec();
                if matches!(
                    failure,
                    NativeCommandVisibilityFailure::MissingCommand
                        | NativeCommandVisibilityFailure::MissingHiddenToken
                        | NativeCommandVisibilityFailure::InvokeHiddenMissing
                ) {
                    tuple.push(b' ');
                    crate::list::append_list_element(&mut tuple, name, false);
                }
                tuple
            });
        Ok(NativeCommandVisibilityDiagnostic {
            message,
            error_code,
        })
    }
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

/// Report the selected invalid C namespace-creation address.
///
/// # Errors
/// Refuses unsupported recipes and failures without this exact address reason.
pub fn report_native_namespace_creation_error(
    protocol: NativeNameProtocol,
    reason: NameProjectionUnavailable,
) -> Result<NativeNamespaceOperationError, NameProjectionUnavailable> {
    let NativeNameProtocol::C(version) = protocol else {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    };
    if reason != NameProjectionUnavailable::EmptyNamespaceOutsideRoot {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    }
    Ok(NativeNamespaceOperationError {
        message: b"can't create namespace \"\": only global namespace can have empty name".to_vec(),
        error_code: (version >= TclVersion::V8_6)
            .then(|| b"TCL OPERATION NAMESPACE CREATEGLOBAL".to_vec()),
        string_protocol: crate::native_string::NativeStringProtocol::C(version),
    })
}

/// Report C namespace-creation occupancy using its original C-string operand.
///
/// # Errors
/// Refuses unaudited Jim namespace-creation diagnostics.
pub fn report_native_namespace_creation_conflict(
    protocol: NativeNameProtocol,
    original: &[u8],
) -> Result<NativeNamespaceOperationError, NameProjectionUnavailable> {
    let NativeNameProtocol::C(version) = protocol else {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    };
    Ok(NativeNamespaceOperationError {
        message: [
            b"can't create namespace \"".as_slice(),
            c_string_extent(original),
            b"\": already exists",
        ]
        .concat(),
        error_code: (version >= TclVersion::V8_6)
            .then(|| b"TCL OPERATION NAMESPACE CREATEEXISTING".to_vec()),
        string_protocol: crate::native_string::NativeStringProtocol::C(version),
    })
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
    /// Early element lookup by an actually admitted indexed local instruction.
    IndexedNameLookup,
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

/// Original operand name for a reached C variable value access. Combined
/// inputs retain their original diagnostic extent; separate operands retain
/// their own extents before parentheses are added for presentation only.
/// This selects no variable cell, alias or native execution capability.
///
/// # Errors
/// Returns `PurposeNotModelled` for a non-C protocol.
pub fn report_native_c_variable_value_name(
    protocol: NativeNameProtocol,
    input: NativeVariableInputForm<'_>,
) -> Result<Vec<u8>, NameProjectionUnavailable> {
    if !matches!(protocol, NativeNameProtocol::C(_)) {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    }
    Ok(match input {
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
    })
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
    use NativeVariableFailureSite::{
        IndexedNameLookup, NameLookup, ValueRead, ValueUnset, ValueWrite,
    };
    let error_code = if matches!(protocol, NativeNameProtocol::C(version) if version >= TclVersion::V8_6)
    {
        let words: &[&[u8]] = match (site, reason) {
            (IndexedNameLookup, RetiredNamespace)
                if operation == NativeVariableDiagnosticOperation::Write && element.is_some() =>
            {
                &[b"TCL", b"LOOKUP", b"VARNAME"]
            }
            (NameLookup, RetiredNamespace)
                if operation == NativeVariableDiagnosticOperation::Write && element.is_some() =>
            {
                &[b"TCL", b"LOOKUP", b"VARNAME", root]
            }
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
            (
                ValueWrite,
                NoSuchVariable | IsArray | Constant | DetachedElement | RetiredNamespace,
            ) => &[b"TCL", b"WRITE", b"VARNAME"],
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
    use NativeVariableFailureSite::{IndexedNameLookup, ValueRead, ValueUnset, ValueWrite};
    if site == IndexedNameLookup
        && !(matches!(protocol, NativeNameProtocol::C(_))
            && operation == Write
            && reason == NativeVariableDiagnosticReason::RetiredNamespace)
    {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    }
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
    if site == IndexedNameLookup && element.is_none() {
        return Err(NameProjectionUnavailable::PurposeNotModelled);
    }
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
        report_native_c_variable_value_name(protocol, input)?
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
    /// Rename missing-source and occupied-destination formatters, separate from lookup keys.
    RenameCommandError,
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
    if purpose == NativeNameReportPurpose::RenameCommandError && !protocol.is_jim084() {
        return Ok(original_operation_part);
    }
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

/// One callable original source word for an exact structured slot. Reporting
/// bytes are only a candidate: publication and lookup must independently select
/// the same slot before the source-channel renderer can preserve their value.
/// This supplies no command existence, compiler preparation or edit geometry.
#[must_use]
pub fn native_command_source_word(
    protocol: NativeNameProtocol,
    slot: &ByteCommandSlot,
    channel: tcl_lexer::SourceChannel,
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    let value = native_command_full_name_bytes(slot);
    if protocol
        .command_publication_slot(NativeNameContext::root(), &value)
        .ok()?
        != *slot
    {
        return None;
    }
    if protocol.is_jim084() {
        if !slot.namespace.is_root()
            || protocol
                .jim_command_lookup_keys(NativeNameContext::root(), &value)
                .ok()?
                .as_slice()
                != std::slice::from_ref(&slot.simple)
        {
            return None;
        }
    } else if protocol
        .command_lookup_slot(NativeNameContext::root(), &value)
        .ok()?
        != *slot
    {
        return None;
    }
    crate::backslash::native_literal_source_word(
        &value,
        channel,
        config,
        protocol.string_protocol(),
    )
}

/// `TclGetCommandFullName` reporting bytes from an already selected native slot.
/// Literal colons and non-text bytes remain data; this projection supplies no
/// source spelling, lookup key or command identity.
#[must_use]
pub fn native_command_full_name_bytes(slot: &ByteCommandSlot) -> Vec<u8> {
    constructed_command_address_bytes(slot)
}

/// Counted absolute command input from retained namespace components and a tail.
/// This constructs an address; lookup, table identity and original object headers
/// remain independent. Literal colons and non-text bytes are not reparsed.
#[must_use]
pub fn constructed_command_address_bytes(slot: &ByteCommandSlot) -> Vec<u8> {
    let mut report = native_namespace_full_name_bytes(&slot.namespace);
    if !slot.namespace.is_root() {
        report.extend_from_slice(b"::");
    }
    report.extend_from_slice(slot.simple.as_bytes());
    report
}

/// Counted full-name bytes of an already constructed C namespace path.
/// Component boundaries remain owned by the path; this reporting projection
/// supplies no written address, namespace existence or native token.
#[must_use]
pub fn native_namespace_full_name_bytes(path: &ByteNamespacePath) -> Vec<u8> {
    let mut report = b"::".to_vec();
    for (index, component) in path.as_segments().iter().enumerate() {
        if index != 0 {
            report.extend_from_slice(b"::");
        }
        report.extend_from_slice(component.as_bytes());
    }
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
    fn ensemble_map_prefix_construction_preserves_counted_heads_before_lookup() {
        // naming.ensemble.original-counted-map-prefix-construction
        // docs/design/analysis/name-resolution-proofs/ensemble-original-counted-map-prefix-construction.md
        // Native312 public map heads/calls are separate from this pure API
        // projection; a namespace report alone never selects a physical holder.
        for version in [
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let protocol = NativeNameProtocol::C(version);
            let relative = protocol
                .ensemble_map_prefix_input(b"::N", true, b"p\0tail")
                .unwrap();
            assert_eq!(relative.as_ref(), b"::N::p\0tail");
            assert_eq!(
                protocol
                    .ensemble_map_prefix_input(b"::", false, b"p\0tail")
                    .unwrap()
                    .as_ref(),
                b"::p\0tail"
            );
            let rooted = b"::N::p\0tail";
            assert!(
                matches!(protocol.ensemble_map_prefix_input(b"::Q", true, rooted).unwrap(), Cow::Borrowed(value) if value == rooted)
            );
            assert_eq!(
                protocol
                    .command_lookup_input(NativeNameContext::root(), &relative)
                    .unwrap()
                    .selected(),
                b"::N::p"
            );
        }
    }

    #[test]
    fn ensemble_map_prefix_requires_its_selected_purpose_and_actual_parent_fact() {
        // Authored pure API negatives, not native detached-holder observations.
        for protocol in [
            NativeNameProtocol::C(TclVersion::V8_4),
            NativeNameProtocol::Jim084,
        ] {
            assert_eq!(
                protocol
                    .ensemble_map_prefix_input(b"::N", true, b"p")
                    .unwrap_err(),
                NameProjectionUnavailable::PurposeNotModelled
            );
        }
        let protocol = NativeNameProtocol::C(TclVersion::V9_1);
        assert_eq!(
            protocol
                .ensemble_map_prefix_input(b"::N", false, b"p")
                .unwrap()
                .as_ref(),
            b"::Np"
        );
        assert_eq!(
            protocol
                .ensemble_map_prefix_input(b"::N", true, b"p")
                .unwrap()
                .as_ref(),
            b"::N::p"
        );
    }

    #[test]
    fn trace_subject_registration_keeps_escaped_zero_distinct_from_raw_zero() {
        // Native proof: naming.variable.trace-subject-counted-zero-address
        // docs/design/analysis/name-resolution-proofs/trace-subject-counted-zero-address.md
        let raw = b"v\0tail(k)";
        let escaped = b"v\xc0\x80tail(k)";
        for version in VERSIONS {
            let protocol = NativeNameProtocol::C(version);
            assert_eq!(
                protocol.trace_registration_input(raw).unwrap().selected(),
                b"v"
            );
            assert_eq!(protocol.trace_query_input(raw).unwrap().selected(), b"v");
            assert_eq!(
                protocol
                    .trace_registration_input(escaped)
                    .unwrap()
                    .selected(),
                escaped,
            );
            assert_eq!(
                protocol.trace_query_input(escaped).unwrap().selected(),
                escaped,
            );
            assert_eq!(
                protocol.variable_root_input(raw).selected(),
                if version == TclVersion::V8_4 {
                    b"v".as_slice()
                } else {
                    raw
                },
            );
        }
        assert!(
            NativeNameProtocol::Jim084
                .trace_registration_input(raw)
                .is_err()
        );
    }

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
            let list = protocol.formal_parameter_list_input(b"k\0z second");
            assert_eq!(list.purpose(), NativeNamePurpose::FormalParameterList);
            assert_eq!(list.original(), b"k\0z second");
            assert_eq!(
                list.selected(),
                if version <= TclVersion::V8_5 {
                    b"k".as_slice()
                } else {
                    b"k\0z second"
                }
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
    fn retired_namespace_write_sites_match_retained_original_c_rows() {
        use NativeVariableDiagnosticOperation::Write;
        use NativeVariableDiagnosticReason::RetiredNamespace;
        use NativeVariableFailureSite::{IndexedNameLookup, NameLookup, ValueWrite};
        let inputs = [
            (
                TclVersion::V8_4,
                include_str!(
                    "../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/capture-01/8.4.20/stdout.txt"
                ),
                include_str!(
                    "../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/capture-02/8.4.20/stdout.txt"
                ),
            ),
            (
                TclVersion::V8_5,
                include_str!(
                    "../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/capture-01/8.5.19/stdout.txt"
                ),
                include_str!(
                    "../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/capture-02/8.5.19/stdout.txt"
                ),
            ),
            (
                TclVersion::V8_6,
                include_str!(
                    "../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/capture-01/8.6.18/stdout.txt"
                ),
                include_str!(
                    "../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/capture-02/8.6.18/stdout.txt"
                ),
            ),
            (
                TclVersion::V9_0,
                include_str!(
                    "../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/capture-01/9.0.4/stdout.txt"
                ),
                include_str!(
                    "../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/capture-02/9.0.4/stdout.txt"
                ),
            ),
            (
                TclVersion::V9_1,
                include_str!(
                    "../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/capture-01/9.1.0/stdout.txt"
                ),
                include_str!(
                    "../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/capture-02/9.1.0/stdout.txt"
                ),
            ),
        ];
        for (version, compiled, object_command) in inputs {
            let protocol = crate::native_string::NativeStringProtocol::C(version);
            for (capture, label, site, original) in [
                (compiled, "literal-scalar", ValueWrite, b"foo".as_slice()),
                (
                    compiled,
                    "literal-element",
                    IndexedNameLookup,
                    b"foo(3)".as_slice(),
                ),
                (
                    object_command,
                    "variable-head-element",
                    NameLookup,
                    b"foo(3)".as_slice(),
                ),
            ] {
                let prefix = format!("ROW {label} ");
                let line = capture
                    .lines()
                    .find(|line| line.starts_with(&prefix))
                    .unwrap();
                let row = crate::list::split_native_list_bytes(line.as_bytes(), protocol).unwrap();
                let expected = crate::list::split_native_list_bytes(&row[3], protocol).unwrap();
                let projection = report_native_variable_diagnostic_at(
                    NativeNameProtocol::C(version),
                    Write,
                    RetiredNamespace,
                    site,
                    NativeVariableInputForm::Combined(original),
                )
                .unwrap();
                let mut message = b"can't set \"".to_vec();
                message.extend_from_slice(&projection.name);
                message.extend_from_slice(b"\": ");
                message.extend_from_slice(projection.reason.message().as_bytes());
                assert_eq!(message.as_slice(), expected[3].as_ref());
                let code = if expected[4].as_ref() == b"NONE" {
                    None
                } else {
                    Some(
                        crate::list::split_native_list_bytes(&expected[4], protocol)
                            .unwrap()
                            .into_iter()
                            .map(|word| word.into_owned())
                            .collect::<Vec<_>>(),
                    )
                };
                assert_eq!(projection.error_code, code);
                assert_eq!(
                    projection.missing_lookup_root.as_deref(),
                    (version >= TclVersion::V8_6 && site == NameLookup)
                        .then_some(b"foo".as_slice())
                );
            }
        }
    }

    #[test]
    fn indexed_retired_lookup_rejects_missing_or_incompatible_purpose() {
        use NativeVariableDiagnosticOperation::{Read, Write};
        use NativeVariableDiagnosticReason::{IsArray, RetiredNamespace};
        use NativeVariableFailureSite::IndexedNameLookup;
        for version in VERSIONS {
            let protocol = NativeNameProtocol::C(version);
            assert!(
                report_native_variable_diagnostic_at(
                    protocol,
                    Write,
                    RetiredNamespace,
                    IndexedNameLookup,
                    NativeVariableInputForm::Combined(b"foo")
                )
                .is_err()
            );
            for (operation, reason) in [(Read, RetiredNamespace), (Write, IsArray)] {
                assert!(
                    report_native_variable_diagnostic_at(
                        protocol,
                        operation,
                        reason,
                        IndexedNameLookup,
                        NativeVariableInputForm::Combined(b"foo(3)")
                    )
                    .is_err()
                );
            }
        }
        assert!(
            report_native_variable_diagnostic_at(
                NativeNameProtocol::Jim084,
                Write,
                RetiredNamespace,
                IndexedNameLookup,
                NativeVariableInputForm::Combined(b"foo(3)")
            )
            .is_err()
        );
    }

    #[test]
    fn c_array_root_write_failure_uses_the_measured_value_store_tuple() {
        // Native proof: naming.variable.original-array-root-write-diagnostic
        // docs/design/analysis/name-resolution-proofs/variable-original-array-root-write-diagnostic.md
        // The source observations identify output; this checks the selected Rust site.
        use NativeVariableDiagnosticOperation::{Read, Write};
        use NativeVariableDiagnosticReason::IsArray;
        use NativeVariableFailureSite::{NameLookup, ValueWrite};
        let input = NativeVariableInputForm::Combined(b"a");
        for version in VERSIONS {
            let protocol = NativeNameProtocol::C(version);
            let projection =
                report_native_variable_diagnostic_at(protocol, Write, IsArray, ValueWrite, input)
                    .unwrap();
            assert_eq!(projection.name, b"a");
            assert_eq!(projection.reason, IsArray);
            let code = (version >= TclVersion::V8_6)
                .then(|| vec![b"TCL".to_vec(), b"WRITE".to_vec(), b"VARNAME".to_vec()]);
            assert_eq!(projection.error_code, code);
            assert!(projection.missing_lookup_root.is_none());
            assert!(
                report_native_variable_diagnostic_at(protocol, Read, IsArray, ValueWrite, input)
                    .is_err()
            );
            if version >= TclVersion::V8_6 {
                assert!(
                    report_native_variable_diagnostic_at(
                        protocol, Write, IsArray, NameLookup, input
                    )
                    .is_err()
                );
            }
        }
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
    fn original_lambda_namespace_preserves_counted_object_and_lookup_purposes() {
        // naming.procedure.original-root-and-colon-holder-publication
        // docs/design/analysis/name-resolution-proofs/procedure-original-root-and-colon-holder-publication.md
        // naming.lambda.original-namespace-constructor-and-getter
        // docs/design/analysis/name-resolution-proofs/lambda-original-namespace-constructor-and-getter.md
        // SDK raw String00 and ByteArray materialised C080 remain independent.
        // These projections retain selected getter bytes without claiming an
        // original object header or namespace existence from the byte recipe.
        for version in [
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let recipe = NativeNameProtocol::C(version);
            for (original, selected, components) in [
                (
                    b":ns".as_slice(),
                    b":::ns".as_slice(),
                    vec![b"ns".as_slice()],
                ),
                (b":::ns", b":::ns", vec![b"ns".as_slice()]),
                (b"", b"::", vec![]),
                (b":ns\0suffix", b":::ns\0suffix", vec![b"ns".as_slice()]),
                (b"::ns\0suffix", b"::ns\0suffix", vec![b"ns".as_slice()]),
                (
                    b"\xc0\x80ns",
                    b"::\xc0\x80ns",
                    vec![b"\xc0\x80ns".as_slice()],
                ),
            ] {
                let input = recipe.lambda_namespace_input(original).unwrap();
                assert_eq!(input.original(), original);
                assert_eq!(input.selected(), selected);
                assert_eq!(input.purpose(), NativeNamePurpose::LambdaNamespace);
                assert_eq!(
                    recipe.lambda_namespace_path(original).unwrap(),
                    ByteNamespacePath::from_segments(components)
                );
            }
        }
        for recipe in [
            NativeNameProtocol::C(TclVersion::V8_4),
            NativeNameProtocol::Jim084,
        ] {
            assert!(recipe.lambda_namespace_input(b":ns").is_err());
            assert!(recipe.lambda_namespace_path(b":ns").is_err());
        }
    }

    #[test]
    fn namespace_creation_parent_route_uses_its_original_cstring_extent() {
        for version in TclVersion::ALL {
            let protocol = NativeNameProtocol::C(version);
            for (input, parent) in [
                (b"retiring".as_slice(), false),
                (b"retiring::", true),
                (b"retiring::::", true),
                (b"retiring::Missing", false),
                (b"retiring::Missing::", true),
                (b"retiring\0::::", false),
                (b"retiring::\0Tail", true),
                (b"retiring:", false),
            ] {
                assert_eq!(
                    protocol.namespace_creation_reuses_parent(input, false),
                    Ok(parent)
                );
                assert!(
                    protocol
                        .namespace_creation_reuses_parent(input, true)
                        .is_err()
                );
                assert!(
                    NativeNameProtocol::Jim084
                        .namespace_creation_reuses_parent(input, false)
                        .is_err()
                );
            }
        }
    }

    #[test]
    fn jim_lambda_namespace_keeps_the_counted_original_context_object() {
        for original in [
            b"AbsentLambda".as_slice(),
            b"::AbsentLambda",
            b"::::AbsentLambda",
            b":AbsentLambda",
            b"Absent:Lambda",
            b"Absent\0Tail",
            b"",
            b"Absent::Child",
        ] {
            let input = NativeNameProtocol::Jim084
                .jim_lambda_namespace_input(original)
                .unwrap();
            assert_eq!(input.original(), original);
            assert_eq!(input.selected(), original);
            assert_eq!(input.purpose(), NativeNamePurpose::JimLambdaNamespace);
            assert_eq!(input.qualification(), NativeNameQualification::Unqualified);
            for version in TclVersion::ALL {
                assert!(
                    NativeNameProtocol::C(version)
                        .jim_lambda_namespace_input(original)
                        .is_err()
                );
            }
        }
    }

    #[test]
    fn original_root_procedure_publication_preserves_a_literal_colon() {
        // naming.procedure.original-root-and-colon-holder-publication
        // docs/design/analysis/name-resolution-proofs/procedure-original-root-and-colon-holder-publication.md
        // C84/85 TclProcObjCmd only prepends a non-global holder's fullName.
        // CLI source observations are independent of these pure slot assertions.
        for version in VERSIONS {
            let recipe = NativeNameProtocol::C(version);
            let context = NativeNameContext::root();
            assert_eq!(
                recipe
                    .command_publication_slot(context, b":source")
                    .unwrap(),
                ByteCommandSlot::new(ByteNamespacePath::root(), NameBytes::from(":source"))
            );
            assert_eq!(
                recipe
                    .command_publication_slot(context, b":::source")
                    .unwrap(),
                ByteCommandSlot::new(ByteNamespacePath::root(), NameBytes::from("source"))
            );
        }
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
fn jim_absolute_canonical_projection_needs_no_namespace_bytes() {
    let jim = NativeNameProtocol::Jim084;
    for (original, expected) in [
        (b":::p\0q".as_slice(), b"p".as_slice()),
        (b"::::".as_slice(), b"".as_slice()),
        (b"::p\xff".as_slice(), b"p\xff".as_slice()),
    ] {
        let canonical = jim
            .jim_namespace_absolute_canonical_input(original)
            .unwrap()
            .unwrap();
        assert_eq!(canonical.original(), original);
        assert_eq!(canonical.selected(), expected);
        assert_eq!(
            canonical.purpose(),
            NativeNamePurpose::JimNamespaceCanonical
        );
        assert!(canonical.context().is_none());
        let candidates = jim
            .jim_namespace_command_query_candidates_from_canonical(&canonical)
            .unwrap();
        let mut expected_rooted = b"::".to_vec();
        expected_rooted.extend_from_slice(expected);
        assert_eq!(candidates[0].lookup.as_bytes(), expected_rooted);
        assert_eq!(candidates[0].reported, candidates[0].lookup);
        let mut original_rooted = b"::".to_vec();
        original_rooted.extend_from_slice(original);
        assert_eq!(candidates[1].lookup.as_bytes(), original_rooted);
        assert_eq!(candidates[1].reported, candidates[1].lookup);
        // The existing contextual entry still requires an actual namespace.
        assert!(
            jim.jim_namespace_canonical_input(
                NativeNameContext::new(&ByteNamespacePath::root()),
                original,
            )
            .is_err()
        );
    }
    for relative in [b"p".as_slice(), b":p", b"", b"p\0q"] {
        assert!(
            jim.jim_namespace_absolute_canonical_input(relative)
                .unwrap()
                .is_none()
        );
    }
    let wrong_purpose = jim.package_key(b"::p");
    assert!(
        jim.jim_namespace_command_query_candidates_from_canonical(&wrong_purpose)
            .is_err()
    );
    for version in TclVersion::ALL {
        let c = NativeNameProtocol::C(version);
        assert!(c.jim_namespace_absolute_canonical_input(b"::p").is_err());
        let canonical = jim
            .jim_namespace_absolute_canonical_input(b"::p")
            .unwrap()
            .unwrap();
        assert!(
            c.jim_namespace_command_query_candidates_from_canonical(&canonical)
                .is_err()
        );
    }
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
    // naming.class.jim-source-rooted-factory-initialiser-replacement
    // docs/design/analysis/name-resolution-proofs/class-jim-source-rooted-factory-initialiser-replacement.md
    // Native rooted factory observations motivate table-key comparison; extent
    // controls preserve original/reporting producer bytes independently.
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
        assert_eq!(slot.simple.as_bytes(), strip_jim_root(name));
        let publication = jim.alias_publication_input(context, name).unwrap();
        assert_eq!(publication.original(), name);
        assert_eq!(publication.selected(), name);
        assert_eq!(publication.jim_flat_key(), Some(strip_jim_root(name)));
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
fn empty_namespace_creation_diagnostic_requires_its_selected_address_reason() {
    for version in TclVersion::ALL {
        let report = report_native_namespace_creation_error(
            NativeNameProtocol::C(version),
            NameProjectionUnavailable::EmptyNamespaceOutsideRoot,
        )
        .unwrap();
        assert_eq!(
            report.message,
            b"can't create namespace \"\": only global namespace can have empty name"
        );
        assert_eq!(
            report.error_code.as_deref(),
            (version >= TclVersion::V8_6)
                .then_some(b"TCL OPERATION NAMESPACE CREATEGLOBAL".as_slice())
        );
        assert_eq!(
            report.string_protocol,
            crate::native_string::NativeStringProtocol::C(version)
        );
        assert!(
            report_native_namespace_creation_error(
                NativeNameProtocol::C(version),
                NameProjectionUnavailable::PurposeNotModelled
            )
            .is_err()
        );
    }
    assert!(
        report_native_namespace_creation_error(
            NativeNameProtocol::Jim084,
            NameProjectionUnavailable::EmptyNamespaceOutsideRoot
        )
        .is_err()
    );
}

#[test]
fn namespace_creation_conflict_keeps_original_spelling_and_selected_extent() {
    for version in TclVersion::ALL {
        for original in [
            b"retiring".as_slice(),
            b"::retiring",
            b"::::retiring",
            b"retiring:tail",
            b"retiring\0ignored",
        ] {
            let report =
                report_native_namespace_creation_conflict(NativeNameProtocol::C(version), original)
                    .unwrap();
            let selected = if original == b"retiring\0ignored" {
                b"retiring".as_slice()
            } else {
                original
            };
            assert_eq!(
                report.message,
                [
                    b"can't create namespace \"".as_slice(),
                    selected,
                    b"\": already exists"
                ]
                .concat(),
            );
            assert_eq!(
                report.error_code.as_deref(),
                (version >= TclVersion::V8_6)
                    .then_some(b"TCL OPERATION NAMESPACE CREATEEXISTING".as_slice()),
            );
            assert_eq!(
                report.string_protocol,
                crate::native_string::NativeStringProtocol::C(version),
            );
        }
    }
    assert!(
        report_native_namespace_creation_conflict(NativeNameProtocol::Jim084, b"retiring").is_err()
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
            assert_eq!(
                recipe.namespace_address_path(NativeNameContext::new(&parent), written),
                Err(NameProjectionUnavailable::EmptyNamespaceOutsideRoot)
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

#[cfg(test)]
mod command_source_word_tests {
    use super::*;

    #[test]
    fn command_source_word_preserves_opaque_slots_and_original_channel() {
        let config = tcl_lexer::LexerConfig::from_grammar(tcl_dialect::grammar_of_dialect_name(
            Some("tcl8.6"),
        ));
        let protocol = NativeNameProtocol::C(TclVersion::V8_6);
        let first =
            ByteCommandSlot::new(ByteNamespacePath::root(), NameBytes::from(b"p\xed\xa0\x80"));
        let second =
            ByteCommandSlot::new(ByteNamespacePath::root(), NameBytes::from(b"p\xed\xa0\x81"));
        let first_word = native_command_source_word(
            protocol,
            &first,
            tcl_lexer::SourceChannel::Document,
            config,
        )
        .unwrap();
        let second_word = native_command_source_word(
            protocol,
            &second,
            tcl_lexer::SourceChannel::Document,
            config,
        )
        .unwrap();
        assert_ne!(first_word, second_word);
        for (slot, word) in [(first, first_word), (second, second_word)] {
            let produced = crate::backslash::native_source_string_bytes_channel_in(
                &word.as_bytes()[1..word.len() - 1],
                tcl_lexer::SourceChannel::Document,
                config.escapes,
                protocol.string_protocol(),
            )
            .unwrap();
            assert_eq!(
                protocol
                    .command_lookup_slot(NativeNameContext::root(), &produced)
                    .unwrap(),
                slot
            );
        }
        let raw_zero =
            ByteCommandSlot::new(ByteNamespacePath::root(), NameBytes::from(b"p\0suffix"));
        assert!(
            native_command_source_word(
                protocol,
                &raw_zero,
                tcl_lexer::SourceChannel::NativeValue,
                config
            )
            .is_none()
        );
        let ambiguous_colon = ByteCommandSlot::new(
            ByteNamespacePath::from_segments([b"a:".as_slice()]),
            NameBytes::from(b"p"),
        );
        assert!(
            native_command_source_word(
                protocol,
                &ambiguous_colon,
                tcl_lexer::SourceChannel::Document,
                config
            )
            .is_none()
        );
    }
}

#[test]
fn original_property_option_extent_is_separate_from_counted_declaration_names() {
    // Implementation contract: naming.tcloo.original-property-accessor-source-advice
    // docs/design/analysis/name-resolution-proofs/tcloo-original-property-accessor-source-advice.md
    for version in [TclVersion::V9_0, TclVersion::V9_1] {
        let recipe = NativeNameProtocol::C(version);
        for (original, expected) in [
            (b"p\0tail".as_slice(), b"-p".as_slice()),
            (b"p\xc0\x80tail", b"-p\xc0\x80tail"),
            (b"p\xed\xa0\x80", b"-p\xed\xa0\x80"),
        ] {
            let option = recipe.oo_property_option_name(original).unwrap();
            assert_eq!(option.original(), original);
            assert_eq!(option.selected(), expected);
            assert_eq!(option.purpose(), NativeNamePurpose::OoPropertyOption);
            assert_eq!(
                recipe.oo_method_input(original).unwrap().selected(),
                original
            );
        }
    }
    for recipe in [
        NativeNameProtocol::C(TclVersion::V8_4),
        NativeNameProtocol::C(TclVersion::V8_5),
        NativeNameProtocol::C(TclVersion::V8_6),
        NativeNameProtocol::Jim084,
    ] {
        assert!(recipe.oo_property_option_name(b"p").is_err());
    }
}

#[cfg(test)]
mod coroutine_publication_tests {
    use super::*;

    type CoroutinePublicationCase<'a> = (&'a [u8], &'a [&'a [u8]], &'a [u8]);

    #[test]
    fn coroutine_slots_keep_current_holders_and_native_byte_extents() {
        // Native proof: naming.coroutine.original-publication-boundaries
        // docs/design/analysis/name-resolution-proofs/coroutine-original-publication-boundaries.md
        let current = ByteNamespacePath::from_segments([b"N".as_slice()]);
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            let protocol = NativeNameProtocol::C(version);
            let cases: &[CoroutinePublicationCase<'_>] = &[
                (b"c\0::Missing::tail", &[b"N"], b"c"),
                (b"c\xc0\x80z", &[b"N"], b"c\xc0\x80z"),
                (b"c\xffz", &[b"N"], b"c\xffz"),
                (b"c\xed\xa0\x80z", &[b"N"], b"c\xed\xa0\x80z"),
                (b"Q::c", &[b"N", b"Q"], b"c"),
                (b"::Q::c", &[b"Q"], b"c"),
                (b"", &[b"N"], b""),
                (b"::", &[], b""),
                (b"Q::", &[b"N", b"Q"], b""),
            ];
            for &(original, holders, simple) in cases {
                let slot = protocol
                    .coroutine_publication_slot(NativeNameContext::new(&current), original)
                    .unwrap();
                assert_eq!(
                    slot.namespace,
                    ByteNamespacePath::from_segments(holders.iter().copied())
                );
                assert_eq!(slot.simple.as_bytes(), simple);
            }
            let error = protocol
                .coroutine_unknown_namespace_error(b"Missing::c\0z")
                .unwrap();
            assert_eq!(
                error.message,
                b"can't create procedure \"Missing::c\": unknown namespace"
            );
            assert_eq!(error.error_code, b"TCL LOOKUP NAMESPACE");
        }
        for protocol in [
            NativeNameProtocol::C(TclVersion::V8_4),
            NativeNameProtocol::C(TclVersion::V8_5),
            NativeNameProtocol::Jim084,
        ] {
            assert_eq!(
                protocol
                    .coroutine_publication_slot(NativeNameContext::new(&current), b"c")
                    .unwrap_err(),
                NameProjectionUnavailable::PurposeNotModelled
            );
        }
    }
    #[test]
    fn c_value_report_name_keeps_original_compound_parts() {
        // Native proof: naming.variable.recreated-element-independent-read-trace
        // docs/design/analysis/name-resolution-proofs/variable-recreated-element-independent-read-trace.md
        // The native source proves the ::a(k) name; extent controls are Rust ownership coverage.
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let protocol = NativeNameProtocol::C(version);
            assert_eq!(
                report_native_c_variable_value_name(
                    protocol,
                    NativeVariableInputForm::Separate {
                        root: b"::a",
                        element: Some(b"k")
                    }
                )
                .unwrap(),
                b"::a(k)"
            );
            let input = NativeVariableInputForm::Separate {
                root: b"a\0tail",
                element: Some(b"k\0tail"),
            };
            let name = report_native_c_variable_value_name(protocol, input).unwrap();
            assert_eq!(name, b"a(k)");
            assert_eq!(
                report_native_variable_diagnostic_at(
                    protocol,
                    NativeVariableDiagnosticOperation::Read,
                    NativeVariableDiagnosticReason::NoSuchElement,
                    NativeVariableFailureSite::ValueRead,
                    input
                )
                .unwrap()
                .name,
                name
            );
            assert_eq!(
                report_native_c_variable_value_name(
                    protocol,
                    NativeVariableInputForm::Combined(b"a\0(k)")
                )
                .unwrap(),
                b"a"
            );
        }
        assert_eq!(
            report_native_c_variable_value_name(
                NativeNameProtocol::Jim084,
                NativeVariableInputForm::Separate {
                    root: b"a",
                    element: Some(b"k")
                }
            ),
            Err(NameProjectionUnavailable::PurposeNotModelled)
        );
    }
}

#[test]
fn jim_two_word_factory_publication_uses_original_units_and_existing_publication_recipe() {
    // naming.source.original-class-reference
    // docs/design/analysis/name-resolution-proofs/source-original-class-reference.md
    // Extent cases are pure owner coverage, not additional native probe claims.
    let jim = NativeNameProtocol::Jim084;
    for (class, member, expected) in [
        (b"C".as_slice(), b"get".as_slice(), b"C get".as_slice()),
        (b"::C", b"method", b"C method"),
        (b"C extra", b"get", b"C extra get"),
        (b"C\0tail", b"get", b"C\0tail get"),
    ] {
        let slot = jim
            .jim_two_word_member_publication_slot(
                NativeNameContext::root(),
                class,
                member,
                NativeNamePurpose::CommandPublication,
            )
            .unwrap();
        assert!(slot.namespace.is_root());
        assert_eq!(slot.simple.as_bytes(), expected);
    }
    assert_eq!(
        jim.jim_two_word_member_publication_slot(
            NativeNameContext::new(&ByteNamespacePath::root()),
            b"C",
            b"get",
            NativeNamePurpose::CommandPublication,
        )
        .unwrap_err(),
        NameProjectionUnavailable::MissingJimNamespaceObject
    );
    for version in [
        TclVersion::V8_4,
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        assert_eq!(
            NativeNameProtocol::C(version)
                .jim_two_word_member_publication_slot(
                    NativeNameContext::root(),
                    b"C",
                    b"get",
                    NativeNamePurpose::CommandPublication
                )
                .unwrap_err(),
            NameProjectionUnavailable::PurposeNotModelled
        );
    }
    let alias = jim
        .jim_two_word_member_publication_slot(
            NativeNameContext::root(),
            b"::C",
            b"constructor",
            NativeNamePurpose::AliasPublication,
        )
        .unwrap();
    assert_eq!(alias.simple.as_bytes(), b"C constructor");
    assert_eq!(
        jim.jim_two_word_member_publication_slot(
            NativeNameContext::root(),
            b"C",
            b"get",
            NativeNamePurpose::CommandLookup
        )
        .unwrap_err(),
        NameProjectionUnavailable::PurposeNotModelled
    );
}

#[test]
fn child_alias_publication_has_its_own_counted_input_and_c_string_registration() {
    // naming.alias.original-child-publication-and-inventory
    // docs/design/analysis/name-resolution-proofs/alias-original-child-publication-and-inventory.md
    // Native298 separately measures child invocation/inventories. This pure
    // projection grants no installed command, original header or parent realm.
    let context_path = ByteNamespacePath::from_segments([b"N".as_slice()]);
    let context = NativeNameContext::new(&context_path);
    for protocol in TclVersion::ALL
        .map(NativeNameProtocol::C)
        .into_iter()
        .chain([NativeNameProtocol::Jim084])
    {
        for input in [
            b"::::r2286".as_slice(),
            b"r2286\0_tail",
            b"r2286_tail\0",
            b"r2286_\xc3\xa9",
            b"r2286_\xff",
        ] {
            let selected = protocol
                .child_alias_publication_input(context, input)
                .unwrap();
            assert_eq!(selected.original(), input);
            assert_eq!(selected.selected(), c_string_extent(input));
            assert_eq!(selected.purpose(), NativeNamePurpose::ChildAliasPublication);
            assert_eq!(
                selected.alias_registration_report_bytes(),
                Some(if protocol.is_jim084() {
                    c_string_extent(input)
                } else {
                    input
                })
            );
            let slot = protocol
                .child_alias_publication_projection(context, input)
                .unwrap();
            assert!(slot.slot().namespace.is_root());
            if protocol.is_jim084() {
                let table = super::NativeJimCommandTableKey::from_projection(&selected).unwrap();
                assert_eq!(table.report_bytes(), c_string_extent(input));
                assert_eq!(
                    table.comparison_bytes(),
                    strip_jim_root(c_string_extent(input))
                );
                assert_eq!(slot.slot().simple.as_bytes(), table.comparison_bytes());
                assert_eq!(
                    protocol
                        .alias_publication_input(context, input)
                        .unwrap()
                        .selected(),
                    input
                );
            }
        }
    }
}

#[cfg(test)]
mod original_command_key_tests {
    use super::*;

    #[test]
    fn original_jim_command_key_constructors_keep_independent_capture_purposes() {
        // naming.alias.original-command-key-replacement-child-capture-and-rename
        // docs/design/analysis/name-resolution-proofs/alias-original-command-key-replacement-child-capture-and-rename.md
        // Constructor selection is a pure recipe, not object ownership or a
        // grant to install a command, evaluate a prefix or recover a context.
        let path = ByteNamespacePath::root();
        let nested = NativeNameContext::with_jim_namespace(&path, b"actual_namespace");
        let jim = NativeNameProtocol::Jim084;
        assert_eq!(
            jim.jim_command_key_construction(nested, b"name", NativeNamePurpose::AliasPublication),
            Ok(NativeJimNamespaceConstruction::RetainOriginal)
        );
        for purpose in [
            NativeNamePurpose::ChildAliasPublication,
            NativeNamePurpose::CommandCApiPublication,
        ] {
            assert_eq!(
                jim.jim_command_key_construction(nested, b"name", purpose),
                Ok(NativeJimNamespaceConstruction::FreshString)
            );
        }
        for purpose in [
            NativeNamePurpose::CommandPublication,
            NativeNamePurpose::RenameSource,
            NativeNamePurpose::RenameDestination,
        ] {
            assert_eq!(
                jim.jim_command_key_construction(nested, b"name", purpose),
                Ok(NativeJimNamespaceConstruction::DuplicateNamespaceAndAppend)
            );
            assert_eq!(
                jim.jim_command_key_construction(nested, b"::name", purpose),
                Ok(NativeJimNamespaceConstruction::RetainOriginal)
            );
            assert_eq!(
                jim.jim_command_key_construction(NativeNameContext::root(), b"name", purpose),
                Ok(NativeJimNamespaceConstruction::RetainOriginal)
            );
            assert_eq!(
                jim.jim_command_key_construction(NativeNameContext::new(&path), b"name", purpose),
                Err(NameProjectionUnavailable::MissingJimNamespaceObject)
            );
            assert_eq!(
                NativeNameProtocol::C(tcl_dialect::TclVersion::V9_0)
                    .jim_command_key_construction(nested, b"name", purpose),
                Err(NameProjectionUnavailable::PurposeNotModelled)
            );
        }
        assert_eq!(
            jim.jim_command_key_construction(nested, b"name", NativeNamePurpose::PackageName),
            Err(NameProjectionUnavailable::PurposeNotModelled)
        );
    }
}

#[cfg(test)]
#[test]
fn jim_namespace_command_query_uses_two_fresh_report_candidates() {
    let jim = NativeNameProtocol::Jim084;
    let path = ByteNamespacePath::root().with_child(b"__nh_context");
    let context = NativeNameContext::with_jim_namespace(&path, b"__nh_context");
    for (original, first, second) in [
        (
            b"__nh_shared".as_slice(),
            b"::__nh_context::__nh_shared".as_slice(),
            b"::__nh_shared".as_slice(),
        ),
        (
            b"::__nh_shared".as_slice(),
            b"::__nh_shared".as_slice(),
            b"::::__nh_shared".as_slice(),
        ),
        (
            b":__nh_literal".as_slice(),
            b"::__nh_context:::__nh_literal".as_slice(),
            b":::__nh_literal".as_slice(),
        ),
    ] {
        let candidates = jim
            .jim_namespace_command_query_candidates(context, original)
            .unwrap();
        assert_eq!(candidates[0].lookup.as_bytes(), first);
        assert_eq!(candidates[1].lookup.as_bytes(), second);
        for candidate in candidates {
            assert_eq!(candidate.lookup, candidate.reported);
        }
    }
    let same = jim
        .jim_namespace_command_query_candidates(NativeNameContext::root(), b"__nh_shared")
        .unwrap();
    assert_eq!(same[0], same[1]);
    assert_eq!(
        jim.jim_namespace_command_query_candidates(NativeNameContext::new(&path), b"name"),
        Err(NameProjectionUnavailable::MissingJimNamespaceObject)
    );
    assert_eq!(
        NativeNameProtocol::C(TclVersion::V9_0)
            .jim_namespace_command_query_candidates(context, b"name"),
        Err(NameProjectionUnavailable::PurposeNotModelled)
    );
}

#[cfg(test)]
mod rename_reporting_tests {
    use super::*;

    #[test]
    fn rename_diagnostic_codes_follow_original_c_releases_and_jim() {
        let original = b"::missing\0suffix";
        for protocol in TclVersion::ALL
            .into_iter()
            .map(NativeNameProtocol::C)
            .chain([NativeNameProtocol::Jim084])
        {
            for (failure, prefix, suffix) in [
                (
                    NativeRenameFailure::MissingSource,
                    b"can't rename \"".as_slice(),
                    b"\": command doesn't exist".as_slice(),
                ),
                (
                    NativeRenameFailure::MissingDeleteSource,
                    b"can't delete \"".as_slice(),
                    b"\": command doesn't exist".as_slice(),
                ),
                (
                    NativeRenameFailure::TargetExists,
                    b"can't rename to \"".as_slice(),
                    b"\": command already exists".as_slice(),
                ),
            ] {
                let error = protocol.rename_diagnostic(failure, original);
                let extent = if protocol.is_jim084() {
                    b"::missing".as_slice()
                } else {
                    original
                };
                assert_eq!(error.message, [prefix, extent, suffix].concat());
                let expected = match (protocol, failure) {
                    (NativeNameProtocol::C(version), _) if version <= TclVersion::V8_5 => None,
                    (NativeNameProtocol::Jim084, _) => None,
                    (_, NativeRenameFailure::TargetExists) => {
                        Some(b"TCL OPERATION RENAME TARGET_EXISTS".as_slice())
                    }
                    _ => Some(b"TCL LOOKUP COMMAND ::missing\0suffix".as_slice()),
                };
                assert_eq!(error.error_code.as_deref(), expected);
            }
        }
    }

    #[test]
    fn rename_diagnostics_preserve_counted_lookup_operands_and_selected_report_extent() {
        let original = b"::alias\0old";
        for protocol in tcl_dialect::TclVersion::ALL
            .into_iter()
            .map(NativeNameProtocol::C)
            .chain([NativeNameProtocol::Jim084])
        {
            assert_eq!(
                report_native_name_bytes(
                    protocol,
                    NativeNameReportPurpose::RenameCommandError,
                    original
                )
                .unwrap(),
                if protocol.is_jim084() {
                    b"::alias".as_slice()
                } else {
                    original
                }
            );
            let lookup = protocol
                .rename_source_input(NativeNameContext::root(), original)
                .unwrap();
            assert_eq!(lookup.original(), original);
            if protocol.is_jim084() {
                assert_eq!(lookup.selected(), original);
            }
        }
    }
}

#[cfg(test)]
mod command_visibility_diagnostic_tests {
    use super::*;

    fn hex(text: &str) -> Vec<u8> {
        text.as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    #[test]
    fn command_visibility_diagnostics_match_original_eight_reason_c_releases() {
        for (version, original) in [
            (
                TclVersion::V8_4,
                include_str!("native/original_visibility/c8_4.stdout"),
            ),
            (
                TclVersion::V8_5,
                include_str!("native/original_visibility/c8_5.stdout"),
            ),
            (
                TclVersion::V8_6,
                include_str!("native/original_visibility/c8_6.stdout"),
            ),
            (
                TclVersion::V9_0,
                include_str!("native/original_visibility/c9_0.stdout"),
            ),
            (
                TclVersion::V9_1,
                include_str!("native/original_visibility/c9_1.stdout"),
            ),
        ] {
            let mut count = 0;
            for row in original.lines() {
                let fields: Vec<_> = row.split_whitespace().collect();
                assert_eq!(fields.len(), 4);
                assert_eq!(fields[1], "1");
                let (failure, name) = match fields[0] {
                    "invalid-token" => (
                        NativeCommandVisibilityFailure::InvalidHiddenToken,
                        b"N::held".as_slice(),
                    ),
                    "hide-non-global" => (
                        NativeCommandVisibilityFailure::HideNonGlobal,
                        b"N::p".as_slice(),
                    ),
                    "expose-non-global" => (
                        NativeCommandVisibilityFailure::ExposeNonGlobal,
                        b"::destination".as_slice(),
                    ),
                    "missing-command" => (
                        NativeCommandVisibilityFailure::MissingCommand,
                        b"missing".as_slice(),
                    ),
                    "missing-hidden" => (
                        NativeCommandVisibilityFailure::MissingHiddenToken,
                        b"N::missing".as_slice(),
                    ),
                    "already-hidden" => (
                        NativeCommandVisibilityFailure::AlreadyHidden,
                        b"held".as_slice(),
                    ),
                    "command-exists" => (
                        NativeCommandVisibilityFailure::CommandExists,
                        b"other".as_slice(),
                    ),
                    "invoke-hidden-missing" => (
                        NativeCommandVisibilityFailure::InvokeHiddenMissing,
                        b"N::missing".as_slice(),
                    ),
                    unknown => panic!("unexpected original reason {unknown}"),
                };
                let error = NativeNameProtocol::C(version)
                    .command_visibility_diagnostic(failure, name)
                    .unwrap();
                assert_eq!(error.message, hex(fields[2]), "{version:?} {}", fields[0]);
                assert_eq!(
                    error.error_code.as_deref().unwrap_or(b"NONE"),
                    hex(fields[3]),
                    "{version:?} {}",
                    fields[0]
                );
                count += 1;
            }
            assert_eq!(count, 8);
        }
        assert_eq!(
            NativeNameProtocol::Jim084.command_visibility_diagnostic(
                NativeCommandVisibilityFailure::MissingCommand,
                b"missing"
            ),
            Err(NameProjectionUnavailable::PurposeNotModelled)
        );
    }
}

#[cfg(test)]
mod jim_command_hash_comparison_tests {
    use super::*;

    #[test]
    fn original_jim_hash_comparison_retains_counted_zero_bytes_and_root_geometry() {
        let jim = NativeNameProtocol::Jim084;
        for (original, key) in [
            (b"p".as_slice(), b"p".as_slice()),
            (b"::::p".as_slice(), b"p".as_slice()),
            (b"p\0x".as_slice(), b"p\0x".as_slice()),
            (b"::p\0x".as_slice(), b"p\0x".as_slice()),
            (b"p\0y".as_slice(), b"p\0y".as_slice()),
        ] {
            assert_eq!(jim.jim_command_table_comparison(original).unwrap(), key);
        }
        assert_ne!(
            jim.jim_command_table_comparison(b"p\0x"),
            jim.jim_command_table_comparison(b"p\0y")
        );
        for version in TclVersion::ALL {
            assert_eq!(
                NativeNameProtocol::C(version).jim_command_table_comparison(b"p"),
                Err(NameProjectionUnavailable::PurposeNotModelled)
            );
        }
    }
}

#[test]
fn original_jim_parent_producer_distinguishes_unchanged_counted_and_rooted_branches() {
    use NativeJimNamespaceParentResult::{
        CountedQualifier, RootedQualifier, RootedSharedEmpty, Unchanged,
    };
    let jim = NativeNameProtocol::Jim084;
    for input in [b"".as_slice(), b"::", b"::\0tail"] {
        assert_eq!(jim.jim_namespace_parent_result(input).unwrap(), Unchanged);
    }
    assert_eq!(
        jim.jim_namespace_parent_result(b"::absent::child").unwrap(),
        CountedQualifier(b"::absent")
    );
    assert_eq!(
        jim.jim_namespace_parent_result(b"absent::child").unwrap(),
        RootedQualifier(b"absent")
    );
    assert_eq!(
        jim.jim_namespace_parent_result(b"84").unwrap(),
        RootedSharedEmpty
    );
    assert_eq!(
        jim.jim_namespace_parent_result(b"::child").unwrap(),
        RootedQualifier(b"")
    );
    assert_eq!(
        jim.jim_namespace_parent_result(b"unpaired:").unwrap(),
        RootedSharedEmpty
    );
    for version in TclVersion::ALL {
        assert!(
            NativeNameProtocol::C(version)
                .jim_namespace_parent_result(b"absent")
                .is_err()
        );
    }
}

#[test]
fn jim_qualifiers_shared_empty_is_distinct_from_a_paired_root_counted_empty() {
    let jim = NativeNameProtocol::Jim084;
    for original in [b"".as_slice(), b"plain", b":", b"plain\0::tail"] {
        assert_eq!(
            jim.namespace_text_result(original, false),
            NativeNamespaceTextResult::SharedEmpty
        );
        assert_eq!(
            jim.namespace_text_result(original, true),
            NativeNamespaceTextResult::Original
        );
    }
    for original in [b"::".as_slice(), b"::\0tail"] {
        assert_eq!(
            jim.namespace_text_result(original, false),
            NativeNamespaceTextResult::Counted(b"")
        );
        assert_eq!(
            jim.namespace_text_result(original, true),
            NativeNamespaceTextResult::Counted(b"")
        );
    }
    for version in TclVersion::ALL {
        assert_ne!(
            NativeNameProtocol::C(version).namespace_text_result(b"plain", false),
            NativeNamespaceTextResult::SharedEmpty
        );
    }
}
