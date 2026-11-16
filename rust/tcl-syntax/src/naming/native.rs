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

//! Purpose-specific native name inputs and reporting.
//!
//! A projection preserves original operands and selects an operation's byte
//! extent. It grants no namespace/frame identity, receiver lifetime, alias
//! closure, command generation, compiler permission or observer proof.

use super::is_qualified;

mod routing;
use crate::native_string::NativeStringProtocol;
pub use routing::{NativeCommandNamespaceRoute, NativeCommandSlotProjection};
use std::borrow::Cow;
use tcl_core_types::{ByteNamespacePath, NameBytes, c_string_extent};
use tcl_dialect::{
    TclVersion,
    model::{BuildProfileId, DialectPoint, Family, Release},
};

mod alias_publication;
mod command_slots;
mod jim_canonical;
#[cfg(test)]
mod jim_namespace_tests;
mod name_diagnostics;
mod name_reports;
mod namespace_addresses;
#[cfg(test)]
mod namespace_reporting_tests;
#[cfg(test)]
mod publication_tests;
mod variable_diagnostics;
mod variable_inputs;

pub use name_diagnostics::{
    NativeCommandVisibilityDiagnostic, NativeCommandVisibilityFailure, NativeNamespaceLookupError,
    NativeNamespaceLookupOperation, NativeNamespaceOperationError, NativeRenameDiagnostic,
    NativeRenameFailure, report_native_namespace_creation_conflict,
    report_native_namespace_creation_error, report_native_namespace_lookup_error,
    report_native_namespace_operation_error,
};
pub use name_reports::{
    NativeNameReportPurpose, NativeNamespaceReportParts, NativeVariableTraceNames,
    NativeVariableTraceReportingInput, checked_command_slot_utf8, checked_namespace_path_utf8,
    constructed_command_address_bytes, native_command_full_name_bytes,
    native_command_source_spelling, native_command_source_word,
    native_jim_namespace_source_spelling, native_namespace_full_name_bytes,
    native_namespace_key_bytes, native_namespace_source_spelling,
    native_qualified_inventory_report_bytes, report_native_name_bytes,
    report_native_variable_access_trace_names,
};
pub use variable_diagnostics::{
    NativeVariableDiagnosticOperation, NativeVariableDiagnosticProjection,
    NativeVariableDiagnosticReason, NativeVariableFailureSite, native_constant_failure_verb,
    report_native_c_variable_value_name, report_native_variable_diagnostic,
    report_native_variable_diagnostic_at,
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
/// Selected import-control meaning of one complete authored Unicode value.
/// This source-value facet is not resident native bytes or execution authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthoredNamespaceImportControl<'source> {
    policy: NamePolicyProtocol,
    source_value: &'source str,
    forces: bool,
}
impl<'source> AuthoredNamespaceImportControl<'source> {
    /// Actual selected pure naming policy of this source operation.
    #[must_use]
    pub const fn policy(self) -> NamePolicyProtocol {
        self.policy
    }
    /// Whole source value; no native resident encoding is fabricated.
    #[must_use]
    pub const fn source_value(self) -> &'source str {
        self.source_value
    }
    /// Whether this source operand selects the supported leading control.
    #[must_use]
    pub const fn forces(self) -> bool {
        self.forces
    }
}

impl NamePolicyProtocol {
    /// Select the leading import control of a complete authored Unicode value.
    /// Source values retain their complete extent; native resident operands must
    /// use `NativeNameProtocol::namespace_import_forces` independently. In
    /// particular U+0000 is not reinterpreted as a resident C byte terminator.
    ///
    /// # Errors
    /// Jim's helper has no modelled C-style leading `-force` control.
    pub fn namespace_import_source_control(
        self,
        source_value: &str,
    ) -> Result<AuthoredNamespaceImportControl<'_>, NameProjectionUnavailable> {
        if self.recipe.is_jim084() {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        Ok(AuthoredNamespaceImportControl {
            policy: self,
            source_value,
            // registry-axis-ok: options — the native name policy's force flag; until the option table marks the force flag
            forces: source_value == "-force",
        })
    }

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
    /// Namespace command's native exact subcommand index operand.
    NamespaceSubcommand,
    /// Namespace command's independent abbreviation operand.
    NamespaceSubcommandPrefix,
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

    /// Namespace exact subcommand indexing consumes `CString` bytes,
    /// independently from the abbreviation stage and original argv object.
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
    /// Jim's namespace abbreviation stage uses the original counted extent;
    /// its earlier exact comparison remains [`Self::namespace_subcommand_input`].
    /// C namespace indexing retains its selected `CString` extent.
    #[must_use]
    pub fn namespace_subcommand_prefix_input(self, original: &[u8]) -> NativeNameProjection<'_> {
        projection(
            self,
            NativeNamePurpose::NamespaceSubcommandPrefix,
            original,
            Cow::Borrowed(if self.is_jim084() {
                original
            } else {
                c_string_extent(original)
            }),
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

#[cfg(test)]
mod tests;

#[cfg(test)]
mod command_source_word_tests;

#[cfg(test)]
mod coroutine_publication_tests;

#[cfg(test)]
mod original_command_key_tests;

#[cfg(test)]
mod rename_reporting_tests;

#[cfg(test)]
mod command_visibility_diagnostic_tests;

#[cfg(test)]
mod jim_command_hash_comparison_tests;

#[cfg(test)]
mod authored_namespace_import_phase_controls;
