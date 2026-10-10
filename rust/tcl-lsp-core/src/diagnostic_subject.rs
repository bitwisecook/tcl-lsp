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

//! Versioned diagnostic subject transport for LSP, CLI and MCP consumers.
//!
//! These descriptions preserve bytes and reporting coordinates. Deserialising
//! one cannot recreate an original word, invocation, native issuer or edit grant.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tcl_compiler::analyser::{Diagnostic, DiagnosticSubject};
use tcl_syntax::naming::{NamePolicyAuthority, NamePolicyProtocol, NativeNameProtocol};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PolicyDescription {
    engine: String,
    version: String,
    authority: String,
}

impl From<NamePolicyProtocol> for PolicyDescription {
    fn from(policy: NamePolicyProtocol) -> Self {
        let (engine, version) = match policy.recipe() {
            NativeNameProtocol::C(version) => ("c-tcl", version.version_string()),
            NativeNameProtocol::Jim084 => ("jim", "0.84"),
        };
        Self {
            engine: engine.to_owned(),
            version: version.to_owned(),
            authority: match policy.authority() {
                NamePolicyAuthority::Native => "native",
                NamePolicyAuthority::AuthoredSimulation => "authored",
            }
            .to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum MathDispatchDescription {
    FixedTable,
    CommandTable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum MathClassificationDescription {
    Absent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum VisibilityObligationDescription {
    SuccessfulSourceOperations,
    RegistryInitialization,
    NoUnmodelledInterpreterMutation,
    NoObserverInterference,
    SourceBodyEntered,
    OriginalOperandMaterialization,
    UnknownExecutingReceiverNamespace,
    NoEarlierReceiverNamespaceBinding,
}
impl From<tcl_compiler::analyser::InterpreterVisibilityObligation>
    for VisibilityObligationDescription
{
    fn from(obligation: tcl_compiler::analyser::InterpreterVisibilityObligation) -> Self {
        use tcl_compiler::analyser::InterpreterVisibilityObligation as Original;
        match obligation {
            Original::SuccessfulSourceOperations => Self::SuccessfulSourceOperations,
            Original::RegistryInitialization => Self::RegistryInitialization,
            Original::NoUnmodelledInterpreterMutation => Self::NoUnmodelledInterpreterMutation,
            Original::NoObserverInterference => Self::NoObserverInterference,
            Original::SourceBodyEntered => Self::SourceBodyEntered,
            Original::OriginalOperandMaterialization => Self::OriginalOperandMaterialization,
            Original::UnknownExecutingReceiverNamespace => Self::UnknownExecutingReceiverNamespace,
            Original::NoEarlierReceiverNamespaceBinding => Self::NoEarlierReceiverNamespaceBinding,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum RegistryPurposeDescription {
    ClauseShape,
    ContextGate,
    ErrorCapture,
    LiteralArgument,
    VariableName,
    OptionTerminator,
    DisabledOption,
    Subcommand,
    DeprecatedCommand,
    Expression,
    Arity,
    ArgumentRelationship,
    Lifecycle,
    ChannelArgument,
    PackageRequirement,
    PackageOrdering,
    AppendList,
    CaseBody,
    OptionOnly,
    PatternSubstitution,
    IndexBounds,
    /// Original selected template operand, independent of its runtime value.
    TemplateSubstitution,
    /// Original selected script-reparse syntax, independent of execution.
    ScriptReparse,
    /// Original channel path; source/value advice supplies no opened channel.
    ChannelPath,
    /// Original source-file path; source/value advice supplies no file entry.
    SourceFilePath,
}
impl From<tcl_compiler::analyser::RegistrySourceDiagnosticKind> for RegistryPurposeDescription {
    fn from(kind: tcl_compiler::analyser::RegistrySourceDiagnosticKind) -> Self {
        use tcl_compiler::analyser::RegistrySourceDiagnosticKind as Kind;
        match kind {
            Kind::ClauseShape => Self::ClauseShape,
            Kind::ContextGate => Self::ContextGate,
            Kind::ErrorCapture => Self::ErrorCapture,
            Kind::LiteralArgument => Self::LiteralArgument,
            Kind::VariableName => Self::VariableName,
            Kind::OptionTerminator => Self::OptionTerminator,
            Kind::DisabledOption => Self::DisabledOption,
            Kind::Subcommand => Self::Subcommand,
            Kind::DeprecatedCommand => Self::DeprecatedCommand,
            Kind::Expression => Self::Expression,
            Kind::Arity => Self::Arity,
            Kind::ArgumentRelationship => Self::ArgumentRelationship,
            Kind::Lifecycle => Self::Lifecycle,
            Kind::ChannelArgument => Self::ChannelArgument,
            Kind::PackageRequirement => Self::PackageRequirement,
            Kind::PackageOrdering => Self::PackageOrdering,
            Kind::AppendList => Self::AppendList,
            Kind::CaseBody => Self::CaseBody,
            Kind::OptionOnly => Self::OptionOnly,
            Kind::PatternSubstitution => Self::PatternSubstitution,
            Kind::IndexBounds => Self::IndexBounds,
            Kind::TemplateSubstitution => Self::TemplateSubstitution,
            Kind::ScriptReparse => Self::ScriptReparse,
            Kind::ChannelPath => Self::ChannelPath,
            Kind::SourceFilePath => Self::SourceFilePath,
        }
    }
}
impl RegistryPurposeDescription {
    fn supports(self, code: &str) -> bool {
        match self {
            Self::ClauseShape => code == "E004",
            Self::ContextGate => code == "W142",
            Self::ErrorCapture => code == "W302",
            Self::LiteralArgument => code == "W146",
            Self::VariableName => code == "W212",
            Self::OptionTerminator => code == "W304",
            Self::DisabledOption => code == "W004" || code == "W145",
            Self::Subcommand => matches!(code, "W001" | "W002" | "W145"),
            Self::DeprecatedCommand => matches!(code, "IRULE2001" | "IRULE2002"),
            Self::Expression => code == "W100",
            Self::Arity => matches!(code, "E001" | "E002" | "E003" | "E005" | "W149"),
            Self::ArgumentRelationship => matches!(code, "W147" | "W152"),
            Self::Lifecycle => matches!(code, "W135" | "W136" | "W139" | "W144" | "W150"),
            Self::ChannelArgument => code == "W126",
            Self::PackageRequirement => code == "W120",
            Self::PackageOrdering => code == "H301",
            Self::AppendList => code == "W104",
            Self::CaseBody => code == "W106",
            Self::OptionOnly => code == "W217",
            Self::PatternSubstitution => code == "W306",
            Self::IndexBounds => matches!(code, "W230" | "W232"),
            Self::TemplateSubstitution => code == "W102",
            Self::ScriptReparse => matches!(code, "W101" | "W301" | "W309" | "W312"),
            Self::ChannelPath => code == "W103",
            Self::SourceFilePath => code == "W300",
        }
    }
}

/// Separate source value premise; deserialising it proves no native argument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SupplementalLiteralDescription {
    argument: usize,
    value: String,
}

/// Authored metadata key retains its independently selected Native recipe.
/// It cannot reconstruct a source package issuer or runtime package table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct NativePackageDescription {
    bytes: Vec<u8>,
    policy: PolicyDescription,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum DeclaredProvenanceDescription {
    BuiltIn,
    BundledPack,
    User,
    WorkspaceTrusted,
    WorkspaceUntrusted,
    StudioOverride,
    Document,
}
impl From<tcl_dialect::model::Provenance> for DeclaredProvenanceDescription {
    fn from(value: tcl_dialect::model::Provenance) -> Self {
        use tcl_dialect::model::Provenance;
        match value {
            Provenance::BuiltIn => Self::BuiltIn,
            Provenance::BundledPack => Self::BundledPack,
            Provenance::User => Self::User,
            Provenance::WorkspaceTrusted => Self::WorkspaceTrusted,
            Provenance::WorkspaceUntrusted => Self::WorkspaceUntrusted,
            Provenance::StudioOverride => Self::StudioOverride,
            Provenance::Document => Self::Document,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum DeclaredObligationDescription {
    DeferredLogicalBodyApplicability,
    DeclarationApplicability,
    LogicalSourceApplicability,
    UnknownEarlierMutation,
}
impl From<tcl_compiler::command_binding::OriginalDeclaredSourceObligation>
    for DeclaredObligationDescription
{
    fn from(value: tcl_compiler::command_binding::OriginalDeclaredSourceObligation) -> Self {
        use tcl_compiler::command_binding::OriginalDeclaredSourceObligation;
        match value {
            OriginalDeclaredSourceObligation::DeferredLogicalBodyApplicability => {
                Self::DeferredLogicalBodyApplicability
            }
            OriginalDeclaredSourceObligation::DeclarationApplicability => {
                Self::DeclarationApplicability
            }
            OriginalDeclaredSourceObligation::LogicalSourceApplicability => {
                Self::LogicalSourceApplicability
            }
            OriginalDeclaredSourceObligation::UnknownEarlierMutation => {
                Self::UnknownEarlierMutation
            }
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum DeclaredPurposeDescription {
    Expression,
    Arity,
    VariableName,
    ChannelArgument,
}
impl DeclaredPurposeDescription {
    fn supports(self, code: &str) -> bool {
        match self {
            Self::Expression => code == "W100",
            Self::Arity => matches!(code, "E002" | "E003" | "E005"),
            Self::VariableName => code == "W212",
            Self::ChannelArgument => code == "W126",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum CommandAvailabilityDescription {
    Unavailable,
    RuleLoaderRefused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum CommandAvailabilityDomainDescription {
    Native,
    Logical,
    Hosted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum RegisteredInstancePurposeDescription {
    MethodName,
    Arity,
    OptionRelation,
}
impl RegisteredInstancePurposeDescription {
    fn supports(self, code: &str) -> bool {
        match self {
            Self::MethodName => code == "W001",
            Self::Arity => matches!(code, "E002" | "E003"),
            Self::OptionRelation => matches!(code, "W147" | "W152"),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum RegisteredInstanceObligationDescription {
    WrittenTransitionApplicability,
    NativeBaselineSourceApplicability,
    OriginalProcedureBodyApplicability,
    FutureOriginalProcedureSourceApplicability,
    UnavailableActualLookup,
    UnknownEarlierMutation,
    ExplicitLogicalSourceApplicability,
    ConditionalDeclarationApplicability,
    DeferredLogicalBodyApplicability,
    ConditionalLogicalBodyApplicability,
    AuthoredCommandPrefixApplicability,
    ProducedCommandPrefixApplicability,
    RegisteredFactoryApplicability,
    RegisteredHandleBindingApplicability,
    OriginalCallbackTargetApplicability,
    UnavailableCallbackLookupFrame,
}
impl From<&tcl_compiler::command_binding::SourceCommandTransitionObligation>
    for RegisteredInstanceObligationDescription
{
    fn from(value: &tcl_compiler::command_binding::SourceCommandTransitionObligation) -> Self {
        use tcl_compiler::command_binding::SourceCommandTransitionObligation as Obligation;
        match value {
            Obligation::WrittenTransitionApplicability => Self::WrittenTransitionApplicability,
            Obligation::OriginalCallbackTargetApplicability => {
                Self::OriginalCallbackTargetApplicability
            }
            Obligation::UnavailableCallbackLookupFrame => Self::UnavailableCallbackLookupFrame,
            Obligation::NativeBaselineSourceApplicability => {
                Self::NativeBaselineSourceApplicability
            }
            Obligation::OriginalProcedureBodyApplicability => {
                Self::OriginalProcedureBodyApplicability
            }
            Obligation::FutureOriginalProcedureSourceApplicability => {
                Self::FutureOriginalProcedureSourceApplicability
            }
            Obligation::UnavailableActualLookup => Self::UnavailableActualLookup,
            Obligation::UnknownEarlierMutation => Self::UnknownEarlierMutation,
            Obligation::ExplicitLogicalSourceApplicability => {
                Self::ExplicitLogicalSourceApplicability
            }
            Obligation::ConditionalDeclarationApplicability => {
                Self::ConditionalDeclarationApplicability
            }
            Obligation::DeferredLogicalBodyApplicability => Self::DeferredLogicalBodyApplicability,
            Obligation::ConditionalLogicalBodyApplicability => {
                Self::ConditionalLogicalBodyApplicability
            }
            Obligation::AuthoredCommandPrefixApplicability => {
                Self::AuthoredCommandPrefixApplicability
            }
            Obligation::ProducedCommandPrefixApplicability => {
                Self::ProducedCommandPrefixApplicability
            }
            Obligation::RegisteredFactoryApplicability => Self::RegisteredFactoryApplicability,
            Obligation::RegisteredHandleBindingApplicability => {
                Self::RegisteredHandleBindingApplicability
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ObjectArityPurposeDescription {
    Constructor,
    LexicalNext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum CallbackPurposeDescription {
    Signature,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum CallbackScopeDescription {
    Unavailable,
    InvokingFrame,
    GlobalFrame,
    TriggerFrame,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ParameterGrammarDescription {
    Tcl,
    Jim,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum CallbackCountsDescription {
    Finite { counts: Vec<usize> },
    AtLeast { minimum: usize },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum CallbackIssueDescription {
    TooFew { supplied: usize, minimum: usize },
    TooMany { supplied: usize, maximum: usize },
    NoCompatibleSignature { supplied: usize },
}
impl CallbackIssueDescription {
    fn supports(&self, code: &str) -> bool {
        match self {
            Self::TooFew { .. } => code == "E002",
            Self::TooMany { .. } => code == "E003",
            Self::NoCompatibleSignature { .. } => code == "E005",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CallbackSignatureDescription {
    simple_name_bytes: Vec<u8>,
    namespace_components: Vec<Vec<u8>>,
    policy: PolicyDescription,
    declaration_start: u32,
    parameter_grammar: ParameterGrammarDescription,
    minimum: u16,
    maximum: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum CallbackTargetKindDescription {
    LocalProcedure,
    ExternalSourceName,
    IndependentExternalHeader,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum CallbackCaptureOriginDescription {
    ResolvedHead,
    BindingPrefix { ordinal: usize },
    Written { ordinal: usize },
    ExpandedElement { written: usize, element: usize },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CallbackCaptureDescription {
    origin: CallbackCaptureOriginDescription,
    expanded: bool,
    source_literal_bytes: Option<Vec<u8>>,
    source_channel: String,
    start: u32,
    end: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CallbackSlotDescription {
    simple_name_bytes: Vec<u8>,
    namespace_components: Vec<Vec<u8>>,
    policy: PolicyDescription,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CallbackTransitionDescription {
    descriptor: String,
    start: u32,
    end: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CallbackTargetDescription {
    kind: CallbackTargetKindDescription,
    registration_start: u32,
    slot: Option<CallbackSlotDescription>,
    captured_minimum: u16,
    captured_indeterminate: bool,
    captures: Vec<CallbackCaptureDescription>,
    lineage: Vec<CallbackTransitionDescription>,
    obligations: Vec<RegisteredInstanceObligationDescription>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum SubjectDescription {
    CallbackSourceArity {
        purpose: CallbackPurposeDescription,
        bytes: Vec<u8>,
        policy: PolicyDescription,
        scope: CallbackScopeDescription,
        #[serde(rename = "bakedArgumentCount")]
        baked_argument_count: usize,
        #[serde(rename = "appendedCounts")]
        appended_counts: CallbackCountsDescription,
        #[serde(rename = "sourceTarget")]
        source_target: Option<CallbackTargetDescription>,
        counts: CallbackCountsDescription,
        issue: CallbackIssueDescription,
        declarations: Vec<CallbackSignatureDescription>,
        #[serde(rename = "sourceChannel")]
        source_channel: String,
        start: u32,
        end: u32,
    },
    ObjectSourceArity {
        purpose: ObjectArityPurposeDescription,
        obligations: Vec<RegisteredInstanceObligationDescription>,
        #[serde(rename = "classSimpleNameBytes")]
        class_simple_name_bytes: Vec<u8>,
        #[serde(rename = "classNamespaceComponents")]
        class_namespace_components: Vec<Vec<u8>>,
        policy: PolicyDescription,
        #[serde(rename = "declarationStart")]
        declaration_start: u32,
        #[serde(rename = "sourceChannel")]
        source_channel: String,
        #[serde(rename = "minimumArgumentCount")]
        minimum_argument_count: u16,
        #[serde(rename = "indeterminateArgumentCount")]
        indeterminate_argument_count: bool,
        start: u32,
        end: u32,
    },
    CommandAvailability {
        command: String,
        classification: CommandAvailabilityDescription,
        domain: CommandAvailabilityDomainDescription,
        #[serde(rename = "sourceChannel")]
        source_channel: String,
        start: u32,
        end: u32,
    },
    RegisteredInstanceSource {
        purpose: RegisteredInstancePurposeDescription,
        #[serde(rename = "factoryCommand")]
        factory_command: String,
        #[serde(rename = "instanceNameBytes")]
        instance_name_bytes: Vec<u8>,
        #[serde(rename = "factoryStart")]
        factory_start: u32,
        #[serde(rename = "sourceChannel")]
        source_channel: String,
        obligations: Vec<RegisteredInstanceObligationDescription>,
        start: u32,
        end: u32,
    },
    DeclaredSource {
        purpose: DeclaredPurposeDescription,
        #[serde(rename = "declarationName")]
        declaration_name: String,
        provenance: DeclaredProvenanceDescription,
        obligations: Vec<DeclaredObligationDescription>,
        argument: Option<usize>,
        #[serde(rename = "sourceChannel")]
        source_channel: String,
        start: u32,
        end: u32,
    },
    RegistrySource {
        purpose: RegistryPurposeDescription,
        #[serde(
            rename = "supplementalLiterals",
            default,
            skip_serializing_if = "Vec::is_empty"
        )]
        supplemental_literals: Vec<SupplementalLiteralDescription>,
        command: String,
        argument: Option<usize>,
        #[serde(rename = "writtenArgument")]
        written_argument: Option<usize>,
        #[serde(rename = "sourceChannel")]
        source_channel: String,
        start: u32,
        end: u32,
        #[serde(
            rename = "requiredPackage",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        required_package: Option<String>,
        #[serde(
            rename = "nativePackage",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        native_package: Option<NativePackageDescription>,
    },
    ConditionalInterpreterVisibility {
        bytes: Vec<u8>,
        policy: PolicyDescription,
        #[serde(rename = "sourceChannel")]
        source_channel: String,
        start: u32,
        end: u32,
        #[serde(rename = "rootSlot")]
        root_slot: Vec<u8>,
        obligations: Vec<VisibilityObligationDescription>,
    },
    RequiredPackage {
        bytes: Vec<u8>,
        policy: PolicyDescription,
    },
    UnresolvedMathFunction {
        bytes: Vec<u8>,
        policy: PolicyDescription,
        #[serde(rename = "reportingName")]
        reporting_name: String,
        #[serde(rename = "sourceChannel")]
        source_channel: String,
        start: u32,
        end: u32,
        ordinal: usize,
        #[serde(rename = "argumentCount")]
        argument_count: usize,
        dispatch: MathDispatchDescription,
        classification: MathClassificationDescription,
    },
    UnresolvedCommand {
        bytes: Vec<u8>,
        policy: PolicyDescription,
        #[serde(rename = "reportingName")]
        reporting_name: String,
        #[serde(rename = "sourceChannel")]
        source_channel: String,
        start: u32,
        end: u32,
    },
}

/// Reporting data retained independently of a diagnostic's presentation.
///
/// Native names remain counted bytes. This data supplies no resolution,
/// allocation, execution, original-source ownership or writable-reference proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosticSubjectData {
    version: u8,
    code: String,
    subject: SubjectDescription,
}

fn command_availability_description(
    original: &tcl_compiler::registry_invocation::OriginalSourceCommandAvailability,
) -> SubjectDescription {
    use tcl_compiler::registry_invocation::CommandSourceAvailabilityDomain as Domain;
    use tcl_registry::model::CommandSourceUnavailabilityKind as Kind;
    SubjectDescription::CommandAvailability {
        command: original.descriptor().command().to_owned(),
        classification: match original.descriptor().kind() {
            Kind::Unavailable => CommandAvailabilityDescription::Unavailable,
            Kind::RuleLoaderRefused => CommandAvailabilityDescription::RuleLoaderRefused,
        },
        domain: match original.domain() {
            Domain::Native => CommandAvailabilityDomainDescription::Native,
            Domain::Logical => CommandAvailabilityDomainDescription::Logical,
            Domain::Hosted => CommandAvailabilityDomainDescription::Hosted,
        },
        source_channel: match original.original_head().image().channel() {
            tcl_lexer::SourceChannel::Document => "document",
            tcl_lexer::SourceChannel::NativeValue => "nativeValue",
        }
        .to_owned(),
        start: original.original_head().span().start(),
        end: original.original_head().span().end(),
    }
}

fn callback_capture_description(
    target: &tcl_compiler::command_binding::OriginalSourceCallbackProcedureTarget,
    ordinal: usize,
) -> Option<CallbackCaptureDescription> {
    use tcl_compiler::registry_invocation::InvocationWordOrigin as Origin;
    let word = target.argument_word(ordinal)?;
    Some(CallbackCaptureDescription {
        origin: match target.argument_origin(ordinal)? {
            Origin::ResolvedHead => CallbackCaptureOriginDescription::ResolvedHead,
            Origin::BindingPrefix(ordinal) => {
                CallbackCaptureOriginDescription::BindingPrefix { ordinal: *ordinal }
            }
            Origin::Written(ordinal) => {
                CallbackCaptureOriginDescription::Written { ordinal: *ordinal }
            }
            Origin::ExpandedElement { written, element } => {
                CallbackCaptureOriginDescription::ExpandedElement {
                    written: *written,
                    element: *element,
                }
            }
        },
        expanded: word.group().expand,
        source_literal_bytes: target
            .argument_input(ordinal)
            .map(|input| input.bytes().to_vec()),
        source_channel: match word.image().channel() {
            tcl_lexer::SourceChannel::Document => "document",
            tcl_lexer::SourceChannel::NativeValue => "nativeValue",
        }
        .to_owned(),
        start: word.span().start(),
        end: word.span().end(),
    })
}
fn callback_target_description(
    selection: &tcl_compiler::analyser::SourceCallbackSignatureLookup,
) -> Option<CallbackTargetDescription> {
    use tcl_compiler::command_binding::OriginalSourceCallbackProcedureTargetKind as Kind;
    let original = selection.original();
    let count = selection.captured_argument_count()?;
    let (kind, slot, captures, lineage) = if let Some(target) = original.target() {
        let slot = target.source_slot();
        let policy = target.target_input().native_input()?.policy();
        (
            match target.kind() {
                Kind::LocalProcedure => CallbackTargetKindDescription::LocalProcedure,
                Kind::ExternalSourceName => CallbackTargetKindDescription::ExternalSourceName,
            },
            Some(CallbackSlotDescription {
                simple_name_bytes: slot.simple.as_bytes().to_vec(),
                namespace_components: slot
                    .namespace
                    .as_segments()
                    .iter()
                    .map(|part| part.as_bytes().to_vec())
                    .collect(),
                policy: policy.into(),
            }),
            (0..target.captured_arguments().len())
                .map(|ordinal| callback_capture_description(target, ordinal))
                .collect::<Option<Vec<_>>>()?,
            target
                .lineage()
                .iter()
                .map(|transition| {
                    Some(CallbackTransitionDescription {
                        descriptor: transition.descriptor().to_owned(),
                        start: transition.original_words().first()?.span().start(),
                        end: transition.original_words().last()?.span().end(),
                    })
                })
                .collect::<Option<Vec<_>>>()?,
        )
    } else if original.permits_external_signature_lookup() {
        (
            CallbackTargetKindDescription::IndependentExternalHeader,
            None,
            Vec::new(),
            Vec::new(),
        )
    } else {
        return None;
    };
    Some(CallbackTargetDescription {
        kind,
        registration_start: original.registration().site().offset,
        slot,
        captured_minimum: count.minimum,
        captured_indeterminate: count.indeterminate,
        captures,
        lineage,
        obligations: original
            .obligations()
            .iter()
            .map(RegisteredInstanceObligationDescription::from)
            .collect(),
    })
}

fn callback_arity_description(
    original: &tcl_compiler::analyser::SourceCallbackAritySubject,
) -> Option<SubjectDescription> {
    use tcl_compiler::analyser::{
        SourceCallbackArgumentCounts as Counts, SourceCallbackArityIssue as Issue,
    };
    let prefix = original.prefix();
    let declarations = original
        .declarations()
        .iter()
        .map(|header| {
            let projection = header.formal_count_projection();
            let arity = projection.arity();
            Some(CallbackSignatureDescription {
                simple_name_bytes: header.name().slot().simple.as_bytes().to_vec(),
                namespace_components: header
                    .name()
                    .slot()
                    .namespace
                    .as_segments()
                    .iter()
                    .map(|component| component.as_bytes().to_vec())
                    .collect(),
                policy: header.name().policy().into(),
                declaration_start: header.declaration_offset(),
                parameter_grammar: match projection.parameter_grammar()? {
                    tcl_dialect::ParameterGrammar::Tcl => ParameterGrammarDescription::Tcl,
                    tcl_dialect::ParameterGrammar::Jim => ParameterGrammarDescription::Jim,
                },
                minimum: arity.min,
                maximum: (!arity.is_unlimited()).then_some(arity.max),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(SubjectDescription::CallbackSourceArity {
        purpose: CallbackPurposeDescription::Signature,
        bytes: prefix.name_input().bytes().to_vec(),
        policy: prefix.name_input().policy().into(),
        scope: match prefix.scope() {
            None => CallbackScopeDescription::Unavailable,
            Some(tcl_registry::ScriptLookupScope::InvokingFrame) => {
                CallbackScopeDescription::InvokingFrame
            }
            Some(tcl_registry::ScriptLookupScope::GlobalFrame) => {
                CallbackScopeDescription::GlobalFrame
            }
            Some(tcl_registry::ScriptLookupScope::TriggerFrame) => {
                CallbackScopeDescription::TriggerFrame
            }
        },
        baked_argument_count: prefix.baked_argument_count(),
        appended_counts: {
            let appended = prefix.appended_arity()?;
            if let Some(exact) = appended.exact_counts() {
                CallbackCountsDescription::Finite {
                    counts: exact.map(usize::from).collect(),
                }
            } else {
                CallbackCountsDescription::AtLeast {
                    minimum: usize::from(appended.min()),
                }
            }
        },
        source_target: if let Some(selection) = original.source_lookup() {
            Some(callback_target_description(selection)?)
        } else {
            None
        },
        counts: match original.argument_counts() {
            Counts::Finite(counts) => CallbackCountsDescription::Finite {
                counts: counts.clone(),
            },
            Counts::AtLeast(minimum) => CallbackCountsDescription::AtLeast { minimum: *minimum },
        },
        issue: match original.issue() {
            Issue::TooFew {
                supplied,
                expected_minimum,
            } => CallbackIssueDescription::TooFew {
                supplied,
                minimum: expected_minimum,
            },
            Issue::TooMany {
                supplied,
                expected_maximum,
            } => CallbackIssueDescription::TooMany {
                supplied,
                maximum: expected_maximum,
            },
            Issue::NoCompatibleSignature { supplied } => {
                CallbackIssueDescription::NoCompatibleSignature { supplied }
            }
        },
        declarations,
        source_channel: match original.source_channel() {
            tcl_lexer::SourceChannel::Document => "document",
            tcl_lexer::SourceChannel::NativeValue => "nativeValue",
        }
        .to_owned(),
        start: original.span().start(),
        end: original.span().end(),
    })
}

fn object_arity_description(
    original: &tcl_compiler::analyser::ObjectSourceAritySubject,
) -> Option<SubjectDescription> {
    match original {
        tcl_compiler::analyser::ObjectSourceAritySubject::Constructor(advice) => {
            let call = advice.call().original_call();
            let class = call.class_declaration().source_slot();
            let first = call.original_words().first()?;
            let count = advice.argument_count();
            Some(SubjectDescription::ObjectSourceArity {
                purpose: ObjectArityPurposeDescription::Constructor,
                obligations: call
                    .obligations()
                    .iter()
                    .map(RegisteredInstanceObligationDescription::from)
                    .collect(),
                class_simple_name_bytes: class.simple.as_bytes().to_vec(),
                class_namespace_components: class
                    .namespace
                    .as_segments()
                    .iter()
                    .map(|part| part.as_bytes().to_vec())
                    .collect(),
                policy: call
                    .class_declaration()
                    .name_input()
                    .native_input()?
                    .policy()
                    .into(),
                declaration_start: call.class_declaration().factory().site().offset,
                source_channel: match first.image().channel() {
                    tcl_lexer::SourceChannel::Document => "document",
                    tcl_lexer::SourceChannel::NativeValue => "nativeValue",
                }
                .to_owned(),
                minimum_argument_count: count.minimum,
                indeterminate_argument_count: count.indeterminate,
                start: first.span().start(),
                end: call.original_words().last()?.span().end(),
            })
        }
        tcl_compiler::analyser::ObjectSourceAritySubject::LexicalNext(next) => {
            let context = next.member_context();
            let class = context.class_publication();
            let first = next.original_words().first()?;
            let count = next.source_argument_count()?;
            Some(SubjectDescription::ObjectSourceArity {
                purpose: ObjectArityPurposeDescription::LexicalNext,
                obligations: Vec::new(),
                class_simple_name_bytes: class.slot().simple.as_bytes().to_vec(),
                class_namespace_components: class
                    .slot()
                    .namespace
                    .as_segments()
                    .iter()
                    .map(|component| component.as_bytes().to_vec())
                    .collect(),
                policy: class.policy().into(),
                declaration_start: context.declaration_site().offset,
                source_channel: match first.image().channel() {
                    tcl_lexer::SourceChannel::Document => "document",
                    tcl_lexer::SourceChannel::NativeValue => "nativeValue",
                }
                .to_owned(),
                minimum_argument_count: count.minimum,
                indeterminate_argument_count: count.indeterminate,
                start: first.span().start(),
                end: next.original_words().last()?.span().end(),
            })
        }
    }
}

impl DiagnosticSubjectData {
    /// Project the emitting owner's typed subject without parsing presentation.
    #[must_use]
    pub fn from_diagnostic(diagnostic: &Diagnostic) -> Option<Self> {
        let subject = match diagnostic.subject()? {
            DiagnosticSubject::CallbackSourceArity(original) => {
                if original.issue().code() != diagnostic.code {
                    return None;
                }
                callback_arity_description(original)?
            }
            DiagnosticSubject::ObjectSourceArity(original) => object_arity_description(original)?,
            DiagnosticSubject::CommandAvailability(original) => {
                command_availability_description(original)
            }
            DiagnosticSubject::RegisteredInstanceSource(original) => {
                let words = original.words();
                let word = words.original_words().first()?;
                SubjectDescription::RegisteredInstanceSource {
                    purpose: match original.kind() {
                        tcl_compiler::analyser::RegisteredInstanceSourceDiagnosticKind::MethodName => RegisteredInstancePurposeDescription::MethodName,
                        tcl_compiler::analyser::RegisteredInstanceSourceDiagnosticKind::Arity => RegisteredInstancePurposeDescription::Arity,
                        tcl_compiler::analyser::RegisteredInstanceSourceDiagnosticKind::OptionRelation => RegisteredInstancePurposeDescription::OptionRelation,
                    },
                    factory_command: words.instance().factory().command().to_owned(),
                    instance_name_bytes: words.instance().name_input().bytes().to_vec(),
                    factory_start: words.instance().factory().site().offset,
                    source_channel: match word.image().channel() {
                        tcl_lexer::SourceChannel::Document => "document",
                        tcl_lexer::SourceChannel::NativeValue => "nativeValue",
                    }.to_owned(),
                    obligations: words.obligations().iter().map(Into::into).collect(),
                    start: word.span().start(),
                    end: words.original_words().last()?.span().end(),
                }
            }
            DiagnosticSubject::DeclaredSource(original) => {
                let word = original.words().original_words().first()?;
                SubjectDescription::DeclaredSource {
                    purpose: match original.kind() {
                        tcl_compiler::analyser::DeclaredSourceDiagnosticKind::Expression => {
                            DeclaredPurposeDescription::Expression
                        }
                        tcl_compiler::analyser::DeclaredSourceDiagnosticKind::Arity => {
                            DeclaredPurposeDescription::Arity
                        }
                        tcl_compiler::analyser::DeclaredSourceDiagnosticKind::VariableName => {
                            DeclaredPurposeDescription::VariableName
                        }
                        tcl_compiler::analyser::DeclaredSourceDiagnosticKind::ChannelArgument => {
                            DeclaredPurposeDescription::ChannelArgument
                        }
                    },
                    declaration_name: original.words().descriptor().name.clone(),
                    provenance: original.words().descriptor().provenance().into(),
                    obligations: original
                        .words()
                        .obligations()
                        .iter()
                        .copied()
                        .map(Into::into)
                        .collect(),
                    argument: original.argument(),
                    source_channel: match word.image().channel() {
                        tcl_lexer::SourceChannel::Document => "document",
                        tcl_lexer::SourceChannel::NativeValue => "nativeValue",
                    }
                    .to_owned(),
                    start: original.span().start(),
                    end: original.span().end(),
                }
            }
            DiagnosticSubject::RegistrySource(original) => {
                let word = original.words().head_source()?.word()?;
                SubjectDescription::RegistrySource {
                    purpose: original.kind().into(),
                    supplemental_literals: original
                        .supplemental_literals()
                        .iter()
                        .map(|(argument, value)| SupplementalLiteralDescription {
                            argument: *argument,
                            value: value.clone(),
                        })
                        .collect(),
                    command: original.words().command().to_owned(),
                    argument: original.argument(),
                    written_argument: original.written_argument(),
                    source_channel: match word.image().channel() {
                        tcl_lexer::SourceChannel::Document => "document",
                        tcl_lexer::SourceChannel::NativeValue => "nativeValue",
                    }
                    .to_owned(),
                    start: original.span().start(),
                    end: original.span().end(),
                    required_package: original.required_package().map(str::to_owned),
                    native_package: original.required_package_key().map(|key| {
                        NativePackageDescription {
                            bytes: key.bytes().to_vec(),
                            policy: key.policy().into(),
                        }
                    }),
                }
            }
            DiagnosticSubject::ConditionalInterpreterVisibility(original) => {
                let key = original.name_input().original_word_key()?;
                SubjectDescription::ConditionalInterpreterVisibility {
                    bytes: original.name_input().bytes().to_vec(),
                    policy: original.name_input().policy().into(),
                    source_channel: match key.source_image().channel() {
                        tcl_lexer::SourceChannel::Document => "document",
                        tcl_lexer::SourceChannel::NativeValue => "nativeValue",
                    }
                    .to_owned(),
                    start: key.span().start(),
                    end: key.span().end(),
                    root_slot: original.slot().simple.as_bytes().to_vec(),
                    obligations: original
                        .obligations()
                        .iter()
                        .copied()
                        .map(Into::into)
                        .collect(),
                }
            }
            DiagnosticSubject::RequiredPackage(key) => SubjectDescription::RequiredPackage {
                bytes: key.bytes().to_vec(),
                policy: key.policy().into(),
            },
            DiagnosticSubject::UnresolvedMathFunction(original) => {
                let occurrence = original.occurrence();
                SubjectDescription::UnresolvedMathFunction {
                    bytes: occurrence.bytes().to_vec(),
                    policy: occurrence.name_policy().into(),
                    reporting_name: original.reporting_name().to_owned(),
                    source_channel: match occurrence.source_image().channel() {
                        tcl_lexer::SourceChannel::NativeValue => "nativeValue",
                        tcl_lexer::SourceChannel::Document => "document",
                    }
                    .to_owned(),
                    start: occurrence.span().start(),
                    end: occurrence.span().end(),
                    ordinal: occurrence.ordinal(),
                    argument_count: occurrence.argument_count(),
                    dispatch: match occurrence.dispatch() {
                        tcl_registry::mathfunc::NativeMathFunctionDispatch::FixedTable => {
                            MathDispatchDescription::FixedTable
                        }
                        tcl_registry::mathfunc::NativeMathFunctionDispatch::CommandTable => {
                            MathDispatchDescription::CommandTable
                        }
                    },
                    classification: MathClassificationDescription::Absent,
                }
            }
            DiagnosticSubject::UnresolvedCommand(original) => {
                let key = original.name_input();
                SubjectDescription::UnresolvedCommand {
                    bytes: key.bytes().to_vec(),
                    policy: key.policy().into(),
                    reporting_name: original.reporting_name().to_owned(),
                    source_channel: match key.source_image().channel() {
                        tcl_lexer::SourceChannel::NativeValue => "nativeValue",
                        tcl_lexer::SourceChannel::Document => "document",
                    }
                    .to_owned(),
                    start: key.span().start(),
                    end: key.span().end(),
                }
            }
        };
        Some(Self {
            version: 1,
            code: diagnostic.code.to_string(),
            subject,
        })
    }

    /// Decode supported reporting data for the expected diagnostic code.
    /// Missing, malformed and future payloads stay unknown; no message fallback.
    #[must_use]
    pub fn from_value(value: &Value, code: &str) -> Option<Self> {
        let data: Self = serde_json::from_value(value.clone()).ok()?;
        let supported_subject = matches!(
            (&data.subject, data.code.as_str()),
            (
                SubjectDescription::ObjectSourceArity { .. },
                "E002" | "E003"
            ) | (
                SubjectDescription::CommandAvailability {
                    classification: CommandAvailabilityDescription::Unavailable,
                    ..
                },
                "W002"
            ) | (
                SubjectDescription::CommandAvailability {
                    classification: CommandAvailabilityDescription::RuleLoaderRefused,
                    ..
                },
                "IRULE2004"
            ) | (SubjectDescription::RequiredPackage { .. }, "W120")
                | (SubjectDescription::UnresolvedCommand { .. }, "W123")
                | (SubjectDescription::UnresolvedMathFunction { .. }, "W123")
                | (
                    SubjectDescription::ConditionalInterpreterVisibility { .. },
                    "W129"
                )
        );
        let supported_subject = supported_subject
            || matches!(&data.subject, SubjectDescription::CallbackSourceArity { issue, .. } if issue.supports(&data.code))
            || matches!(&data.subject,
            SubjectDescription::RegistrySource { purpose, .. } if purpose.supports(&data.code))
            || matches!(&data.subject, SubjectDescription::DeclaredSource { purpose, .. } if purpose.supports(&data.code))
            || matches!(&data.subject, SubjectDescription::RegisteredInstanceSource { purpose, .. } if purpose.supports(&data.code));
        (data.version == 1 && data.code == code && supported_subject).then_some(data)
    }

    /// Encode the shared transport used by every protocol adapter.
    #[must_use]
    pub fn to_value(&self) -> Option<Value> {
        serde_json::to_value(self).ok()
    }

    /// Conditional child-source subject bytes; the payload cannot reconstruct
    /// a child interpreter, original invocation or selected hidden allocation.
    #[must_use]
    pub fn conditional_visibility_bytes(&self) -> Option<&[u8]> {
        match &self.subject {
            SubjectDescription::ConditionalInterpreterVisibility { bytes, .. } => Some(bytes),
            _ => None,
        }
    }

    /// Exact reporting package bytes, without reconstructing a naming issuer.
    #[must_use]
    pub fn package_bytes(&self) -> Option<&[u8]> {
        match &self.subject {
            SubjectDescription::RequiredPackage { bytes, .. } => Some(bytes),
            SubjectDescription::RegistrySource { native_package, .. } => {
                native_package.as_ref().map(|key| key.bytes.as_slice())
            }
            SubjectDescription::CallbackSourceArity { .. }
            | SubjectDescription::ObjectSourceArity { .. }
            | SubjectDescription::CommandAvailability { .. }
            | SubjectDescription::RegisteredInstanceSource { .. }
            | SubjectDescription::DeclaredSource { .. }
            | SubjectDescription::UnresolvedCommand { .. }
            | SubjectDescription::UnresolvedMathFunction { .. }
            | SubjectDescription::ConditionalInterpreterVisibility { .. } => None,
        }
    }

    /// Original emitting word's reporting label; it supplies no lookup proof.
    #[must_use]
    pub fn command_reporting_name(&self) -> Option<&str> {
        match &self.subject {
            SubjectDescription::UnresolvedCommand { reporting_name, .. } => Some(reporting_name),
            SubjectDescription::CallbackSourceArity { .. }
            | SubjectDescription::ObjectSourceArity { .. }
            | SubjectDescription::CommandAvailability { .. }
            | SubjectDescription::RegisteredInstanceSource { .. }
            | SubjectDescription::DeclaredSource { .. }
            | SubjectDescription::RegistrySource { .. }
            | SubjectDescription::RequiredPackage { .. }
            | SubjectDescription::UnresolvedMathFunction { .. }
            | SubjectDescription::ConditionalInterpreterVisibility { .. } => None,
        }
    }
    /// Original expression identifier reporting; no command word is recovered.
    #[must_use]
    pub fn math_function_reporting_name(&self) -> Option<&str> {
        match &self.subject {
            SubjectDescription::UnresolvedMathFunction { reporting_name, .. } => {
                Some(reporting_name)
            }
            SubjectDescription::CallbackSourceArity { .. }
            | SubjectDescription::ObjectSourceArity { .. }
            | SubjectDescription::CommandAvailability { .. }
            | SubjectDescription::RegisteredInstanceSource { .. }
            | SubjectDescription::DeclaredSource { .. }
            | SubjectDescription::RegistrySource { .. }
            | SubjectDescription::RequiredPackage { .. }
            | SubjectDescription::UnresolvedCommand { .. }
            | SubjectDescription::ConditionalInterpreterVisibility { .. } => None,
        }
    }
}

/// Shared serialized subject projection; absent owner data remains absent.
#[must_use]
pub fn diagnostic_subject_data(diagnostic: &Diagnostic) -> Option<Value> {
    DiagnosticSubjectData::from_diagnostic(diagnostic)?.to_value()
}

#[cfg(test)]
mod tests {
    #[test]
    fn callback_signature_transport_retains_original_counts_and_source_purpose() {
        // naming.diagnostic.original-callback-signature-subject
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-callback-signature-subject.md
        // Readonly source assessment; the callback is not executed.
        use std::sync::Arc;
        use tcl_compiler::analyser::{
            Analyser, Diagnostic, DiagnosticSubject, ItemTree, Severity, SourceCallbackAritySubject,
        };
        let source = "proc cb {a b} {return 0}\nlsort -command {cb fixed} {3 1 2}";
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse(source, "tcl9.0");
        let headers = ItemTree::from_analysis(&analysis, &analyser.ensemble_namespaces)
            .sigs()
            .into_iter()
            .filter_map(|sig| sig.original_declaration)
            .collect::<Vec<_>>();
        let prefix = analysis
            .command_invocations
            .iter()
            .find_map(|inv| inv.original_callback_prefix.as_ref())
            .unwrap();
        let subject =
            SourceCallbackAritySubject::from_source_signatures(Arc::clone(prefix), &headers)
                .unwrap();
        let diagnostic = Diagnostic::new(
            subject.issue().code(),
            subject.span(),
            "presentation".to_owned(),
            Severity::Error,
        )
        .with_subject(DiagnosticSubject::CallbackSourceArity(Arc::new(subject)));
        let data = super::diagnostic_subject_data(&diagnostic).unwrap();
        assert_eq!(data["subject"]["kind"], "callbackSourceArity");
        assert_eq!(data["subject"]["purpose"], "signature");
        assert_eq!(data["subject"]["scope"], "invokingFrame");
        assert_eq!(data["subject"]["counts"]["counts"], serde_json::json!([3]));
        assert_eq!(
            data["subject"]["declarations"][0]["parameterGrammar"],
            "tcl"
        );
        assert_eq!(data["subject"]["issue"]["kind"], "tooMany");
        assert!(super::DiagnosticSubjectData::from_value(&data, "E003").is_some());
        assert!(super::DiagnosticSubjectData::from_value(&data, "W123").is_none());
        let mut wrong_code = data.clone();
        wrong_code["code"] = serde_json::json!("E002");
        assert!(
            super::DiagnosticSubjectData::from_value(&wrong_code, "E002").is_none(),
            "typed too-many issue cannot become too-few by transport code"
        );
        let mut changed = diagnostic;
        changed.message = "unrelated text".to_owned();
        assert_eq!(super::diagnostic_subject_data(&changed), Some(data));
    }

    #[test]
    fn original_callback_transport_keeps_unavailable_frame_and_registration_premises() {
        // naming.source.original-callback-procedure-target
        // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
        // Genuine source signature advice; the callback is not executed and
        // its unknown frame is never replaced by the registration frame.
        use std::sync::Arc;
        use tcl_compiler::analyser::{
            Analyser, Diagnostic, DiagnosticSubject, Severity, SourceCallbackAritySubject,
        };
        let source = "# tcl-lsp: requires smtp\nproc cb {} {}\nsmtp::sendmessage message -tokenCallback ::cb";
        let analysis = Analyser::new().analyse(source, "tcl9.0");
        let selection = analysis
            .command_invocations
            .iter()
            .find_map(|row| row.original_callback_signature_lookup.as_ref())
            .expect("authentic rooted source callback signature lookup");
        assert_eq!(selection.prefix().scope(), None);
        assert!(selection.prefix().lookup().is_none());
        assert!(selection.prefix().source_registration().is_some());
        let subject =
            SourceCallbackAritySubject::from_source_lookup(Arc::clone(selection), &[]).unwrap();
        let diagnostic = Diagnostic::new(
            subject.issue().code(),
            subject.span(),
            "presentation".to_owned(),
            Severity::Error,
        )
        .with_subject(DiagnosticSubject::CallbackSourceArity(Arc::new(subject)));
        let data = super::diagnostic_subject_data(&diagnostic).unwrap();
        assert_eq!(data["subject"]["scope"], "unavailable");
        assert!(
            data["subject"]["sourceTarget"]["obligations"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("unavailableCallbackLookupFrame"))
        );
        assert!(super::DiagnosticSubjectData::from_value(&data, "E003").is_some());
        let mut changed = diagnostic;
        changed.message = "unrelated presentation".to_owned();
        assert_eq!(super::diagnostic_subject_data(&changed), Some(data));
    }

    #[test]
    fn constructor_source_subject_retains_count_and_purpose_without_messages() {
        // naming.source.original-class-constructor-call
        // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
        let source =
            "oo::class create C {constructor {a b} { }}\ninterp alias {} make {} C new fixed\nmake";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl9.0");
        let diagnostic = analysis.diagnostics.iter().find(|diagnostic| matches!(diagnostic.subject(), Some(tcl_compiler::analyser::DiagnosticSubject::ObjectSourceArity(subject)) if matches!(subject.as_ref(), tcl_compiler::analyser::ObjectSourceAritySubject::Constructor(_)))).expect("constructor source signature");
        let mut changed = diagnostic.clone();
        changed.message = "unrelated presentation".to_owned();
        let first = super::diagnostic_subject_data(diagnostic).unwrap();
        assert_eq!(super::diagnostic_subject_data(&changed).unwrap(), first);
        assert_eq!(first["subject"]["purpose"], "constructor");
        assert_eq!(first["subject"]["minimumArgumentCount"], 1);
        assert_eq!(&source[changed.span.as_range()], "make");
    }

    #[test]
    fn constructor_transport_retains_distinct_source_applicability_obligations() {
        // naming.source.original-class-constructor-call
        // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
        // Source inventory is conditional; the procedure is not invoked.
        let source = "proc make {} {C new 1}\noo::class create C {constructor {a b} { }}";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl9.0");
        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| {
                matches!(
                    diagnostic.object_source_arity(),
                    Some(tcl_compiler::analyser::ObjectSourceAritySubject::Constructor(_))
                )
            })
            .expect("deferred source constructor signature");
        let data = super::diagnostic_subject_data(diagnostic).unwrap();
        assert!(
            data["subject"]["obligations"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!(
                    "futureOriginalProcedureSourceApplicability"
                ))
        );
        use tcl_compiler::command_binding::SourceCommandTransitionObligation as Obligation;
        for (value, label) in [
            (
                Obligation::NativeBaselineSourceApplicability,
                "nativeBaselineSourceApplicability",
            ),
            (
                Obligation::OriginalProcedureBodyApplicability,
                "originalProcedureBodyApplicability",
            ),
            (
                Obligation::FutureOriginalProcedureSourceApplicability,
                "futureOriginalProcedureSourceApplicability",
            ),
        ] {
            assert_eq!(
                serde_json::to_value(super::RegisteredInstanceObligationDescription::from(&value))
                    .unwrap(),
                label
            );
        }
    }

    #[test]
    fn lexical_next_arity_transport_retains_declaration_and_effective_count() {
        // naming.tcloo.original-lexical-member-context
        // docs/design/analysis/name-resolution-proofs/tcloo-original-lexical-member-context.md
        let source = "oo::class create Base {method f {a b} {}}\noo::class create C {superclass Base; method f {} {::::next 1}}";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl9.0");
        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.object_source_arity().is_some())
            .unwrap();
        let data = super::diagnostic_subject_data(diagnostic).unwrap();
        assert_eq!(data["subject"]["kind"], "objectSourceArity");
        assert_eq!(
            data["subject"]["classSimpleNameBytes"],
            serde_json::json!([67])
        );
        assert_eq!(data["subject"]["minimumArgumentCount"], 1);
        assert_eq!(data["subject"]["indeterminateArgumentCount"], false);
        assert!(super::DiagnosticSubjectData::from_value(&data, "E002").is_some());
        assert!(super::DiagnosticSubjectData::from_value(&data, "W123").is_none());
        let mut changed = diagnostic.clone();
        changed.message = "other wording".to_owned();
        assert_eq!(super::diagnostic_subject_data(&changed), Some(data));
    }

    #[test]
    fn registered_instance_transport_retains_factory_and_applicability_without_message_identity() {
        // naming.source.original-registered-instance-words
        // docs/design/analysis/name-resolution-proofs/source-original-registered-instance-words.md
        let source = "ttk::treeview .t; .t bogus";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.registered_instance_source().is_some())
            .unwrap();
        let value = super::diagnostic_subject_data(diagnostic).unwrap();
        assert_eq!(value["subject"]["kind"], "registeredInstanceSource");
        assert_eq!(value["subject"]["factoryCommand"], "ttk::treeview");
        assert_eq!(
            value["subject"]["instanceNameBytes"],
            serde_json::json!([46, 116])
        );
        assert!(super::DiagnosticSubjectData::from_value(&value, "W001").is_some());
        let mut changed = diagnostic.clone();
        changed.message = "different wording".to_owned();
        assert_eq!(super::diagnostic_subject_data(&changed), Some(value));
    }

    use super::*;
    use tcl_compiler::analyser::Severity;
    use tcl_core_types::DiagCode;

    fn package(bytes: &[u8]) -> Diagnostic {
        Diagnostic::new(
            DiagCode::W120,
            tcl_lexer::Span::new(0, 1),
            "same message",
            Severity::Hint,
        )
        .with_subject(DiagnosticSubject::RequiredPackage(
            tcl_registry::native_package::NativePackageNameKey::from_native_units(
                bytes,
                NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6),
            ),
        ))
    }

    #[test]
    fn conditional_visibility_transport_preserves_assumptions_without_message_parsing() {
        // Implementation contract: naming.interpreter.original-source-visibility-advice
        // docs/design/analysis/name-resolution-proofs/interpreter-original-source-visibility-advice.md
        let source = "interp create -safe s\ninterp eval s {::source a.tcl}";
        let result = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
        let original = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.conditional_interpreter_visibility().is_some())
            .unwrap();
        let data = DiagnosticSubjectData::from_diagnostic(original).unwrap();
        assert_eq!(
            data.conditional_visibility_bytes(),
            Some(b"::source".as_slice())
        );
        let encoded = data.to_value().unwrap();
        assert_eq!(
            encoded["subject"]["rootSlot"],
            serde_json::json!([115, 111, 117, 114, 99, 101])
        );
        assert_eq!(
            encoded["subject"]["obligations"][0],
            "successfulSourceOperations"
        );
        assert_eq!(
            DiagnosticSubjectData::from_value(&encoded, "W129"),
            Some(data)
        );
        assert!(DiagnosticSubjectData::from_value(&encoded, "W123").is_none());
        let mut foreign = encoded.clone();
        foreign["subject"]["obligations"][0] = serde_json::json!("unknownFutureObligation");
        assert!(DiagnosticSubjectData::from_value(&foreign, "W129").is_none());
        let mut report = original.clone();
        report.message = "an arbitrary reporting sentence".to_owned();
        assert_eq!(
            DiagnosticSubjectData::from_diagnostic(&report)
                .unwrap()
                .to_value(),
            Some(encoded)
        );
    }

    #[test]
    fn package_transport_ignores_message_and_fixes_but_retains_opaque_bytes() {
        // Proof naming.diagnostic.typed-subject-reporting:
        // docs/design/analysis/name-resolution-proofs/diagnostic-typed-subject-reporting.md
        let mut diagnostic = package(b"quoted'pkg\xc0\x80\xed\xa0\x80");
        let first = diagnostic_subject_data(&diagnostic).unwrap();
        diagnostic.message = "different wording and unrelated quoted names".to_owned();
        diagnostic.fixes.clear();
        assert_eq!(diagnostic_subject_data(&diagnostic), Some(first.clone()));
        let decoded = DiagnosticSubjectData::from_value(&first, "W120").unwrap();
        assert_eq!(
            decoded.package_bytes(),
            Some(b"quoted'pkg\xc0\x80\xed\xa0\x80".as_slice())
        );
        assert_ne!(diagnostic_subject_data(&package(b"other")), Some(first));
    }

    #[test]
    fn absent_malformed_future_or_wrong_code_payload_never_recreates_subject() {
        // Proof naming.diagnostic.typed-subject-reporting:
        // docs/design/analysis/name-resolution-proofs/diagnostic-typed-subject-reporting.md
        let mut diagnostic = package(b"Tk");
        diagnostic.subject = None;
        assert!(diagnostic_subject_data(&diagnostic).is_none());
        assert!(DiagnosticSubjectData::from_value(&Value::Null, "W120").is_none());
        let mut payload = diagnostic_subject_data(&package(b"Tk")).unwrap();
        assert!(DiagnosticSubjectData::from_value(&payload, "W123").is_none());
        payload["code"] = "W123".into();
        assert!(DiagnosticSubjectData::from_value(&payload, "W123").is_none());
        payload["code"] = "W120".into();
        payload["version"] = 2.into();
        assert!(DiagnosticSubjectData::from_value(&payload, "W120").is_none());
    }

    #[test]
    fn unresolved_transport_uses_the_emitting_original_source_owner() {
        // Proof naming.diagnostic.typed-subject-reporting:
        // docs/design/analysis/name-resolution-proofs/diagnostic-typed-subject-reporting.md
        let source = r"missing\uD800 argument";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
        let mut diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == DiagCode::W123)
            .expect("actual unresolved command diagnostic")
            .clone();
        let Some(DiagnosticSubject::UnresolvedCommand(original)) = diagnostic.subject() else {
            panic!("emitter retains original source name");
        };
        assert_eq!(original.name_input().bytes(), b"missing\xed\xa0\x80");
        let payload = diagnostic_subject_data(&diagnostic).unwrap();
        assert_eq!(
            payload["subject"]["bytes"],
            serde_json::json!(b"missing\xed\xa0\x80")
        );
        assert_eq!(payload["subject"]["sourceChannel"], "document");
        diagnostic.message = "wording with misleading 'different' and `quoted` names".to_owned();
        assert_eq!(diagnostic_subject_data(&diagnostic), Some(payload.clone()));
        assert!(DiagnosticSubjectData::from_value(&payload, "W123").is_some());
        assert!(DiagnosticSubjectData::from_value(&payload, "W120").is_none());
    }

    #[test]
    fn shape_advice_transport_keeps_original_purpose_and_operand_axes() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        for (source, code, purpose, argument, written) in [
            (
                "interp alias {} add {} append out\nadd \" $item\"",
                tcl_core_types::DiagCode::W104,
                "appendList",
                Some(1),
                Some(0),
            ),
            (
                "interp alias {} choose {} switch -regexp subject\nchoose a $body",
                tcl_core_types::DiagCode::W106,
                "caseBody",
                Some(3),
                Some(1),
            ),
            (
                "interp alias {} clean {} unset -nocomplain\nclean",
                tcl_core_types::DiagCode::W217,
                "optionOnly",
                None,
                None,
            ),
        ] {
            let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
            let mut diagnostic = analysis
                .diagnostics
                .iter()
                .find(|diagnostic| diagnostic.code == code)
                .expect("owned shape advice")
                .clone();
            let payload = diagnostic_subject_data(&diagnostic).unwrap();
            assert_eq!(payload["subject"]["purpose"], purpose);
            assert_eq!(payload["subject"]["argument"], serde_json::json!(argument));
            assert_eq!(
                payload["subject"]["writtenArgument"],
                serde_json::json!(written)
            );
            assert!(DiagnosticSubjectData::from_value(&payload, code.as_str()).is_some());
            diagnostic.message = "translated misleading spelling".to_owned();
            diagnostic.fixes.clear();
            assert_eq!(diagnostic_subject_data(&diagnostic), Some(payload));
        }
    }

    #[test]
    fn math_transport_preserves_expression_kind_without_reconstructing_a_command() {
        // naming.diagnostic.original-math-function-subject
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-math-function-subject.md
        for (dialect, dispatch) in [
            ("tcl8.4", "fixedTable"),
            ("tcl8.5", "commandTable"),
            ("tcl8.6", "commandTable"),
            ("tcl9.0", "commandTable"),
            ("tcl9.1", "commandTable"),
            ("jim", "fixedTable"),
        ] {
            let analysis = tcl_compiler::analyser::Analyser::new().analyse("expr {Pi()}", dialect);
            let mut diagnostic = analysis
                .diagnostics
                .iter()
                .find(|diagnostic| diagnostic.unresolved_math_function().is_some())
                .unwrap_or_else(|| panic!("{dialect}: original math diagnostic missing; invocations={:?}; diagnostics={:?}", analysis.command_invocations, analysis.diagnostics))
                .clone();
            let payload = diagnostic_subject_data(&diagnostic).unwrap();
            assert_eq!(payload["subject"]["kind"], "unresolvedMathFunction");
            assert_eq!(payload["subject"]["bytes"], serde_json::json!(b"Pi"));
            assert_eq!(payload["subject"]["start"], 6);
            assert_eq!(payload["subject"]["end"], 8);
            assert_eq!(payload["subject"]["sourceChannel"], "document");
            assert_eq!(payload["subject"]["argumentCount"], 0);
            assert_eq!(payload["subject"]["ordinal"], 0);
            assert_eq!(payload["subject"]["dispatch"], dispatch);
            assert_eq!(payload["subject"]["classification"], "absent");
            let decoded = DiagnosticSubjectData::from_value(&payload, "W123").unwrap();
            assert_eq!(decoded.math_function_reporting_name(), Some("Pi"));
            assert!(decoded.command_reporting_name().is_none());
            assert!(decoded.package_bytes().is_none());
            assert!(DiagnosticSubjectData::from_value(&payload, "W120").is_none());
            diagnostic.message = "translated misleading 'Else'".to_owned();
            diagnostic.fixes.clear();
            assert_eq!(diagnostic_subject_data(&diagnostic), Some(payload.clone()));
            let mut future = payload.clone();
            future["version"] = 2.into();
            assert!(DiagnosticSubjectData::from_value(&future, "W123").is_none());
            let mut unknown_purpose = payload.clone();
            unknown_purpose["subject"]["dispatch"] = "future".into();
            assert!(DiagnosticSubjectData::from_value(&unknown_purpose, "W123").is_none());
            unknown_purpose = payload.clone();
            unknown_purpose["subject"]["classification"] = "unknown".into();
            assert!(DiagnosticSubjectData::from_value(&unknown_purpose, "W123").is_none());
            let mut malformed = payload;
            malformed["subject"]["start"] = "unknown".into();
            assert!(DiagnosticSubjectData::from_value(&malformed, "W123").is_none());
        }
    }
    #[test]
    fn declared_source_transport_preserves_contract_purpose_and_provenance() {
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
        let source = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub select {expression:expr}\n# tcl-lsp: stubs-end\nselect \"$value + 1\"";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
        let mut diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.declared_source().is_some())
            .expect("declared source diagnostic")
            .clone();
        let payload = diagnostic_subject_data(&diagnostic).unwrap();
        assert_eq!(payload["subject"]["kind"], "declaredSource");
        assert_eq!(payload["subject"]["purpose"], "expression");
        assert_eq!(payload["subject"]["declarationName"], "select");
        assert_eq!(payload["subject"]["provenance"], "document");
        assert!(
            payload["subject"]["obligations"]
                .as_array()
                .unwrap()
                .iter()
                .any(|obligation| obligation == "declarationApplicability")
        );
        diagnostic.message = "translated misleading 'expr'".to_owned();
        diagnostic.fixes.clear();
        assert_eq!(diagnostic_subject_data(&diagnostic), Some(payload.clone()));
        let decoded = DiagnosticSubjectData::from_value(&payload, "W100").unwrap();
        assert!(decoded.command_reporting_name().is_none());
        assert!(decoded.package_bytes().is_none());
        assert!(DiagnosticSubjectData::from_value(&payload, "W123").is_none());
    }
    #[test]
    fn unavailable_command_transport_retains_the_original_purpose_without_messages() {
        // naming.diagnostic.original-command-source-unavailability
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-command-source-unavailability.md
        for (source, dialect, code, command, class, domain) in [
            (
                "::dict get {a 1} a",
                "tcl8.4",
                DiagCode::W002,
                "dict",
                "unavailable",
                "native",
            ),
            (
                "when RULE_INIT {rename a b}",
                "f5-irules",
                DiagCode::Irule2004,
                "rename",
                "ruleLoaderRefused",
                "hosted",
            ),
        ] {
            let mut analysis = tcl_compiler::analyser::Analyser::new().analyse(source, dialect);
            let original = analysis
                .diagnostics
                .iter()
                .find(|diagnostic| diagnostic.code == code)
                .unwrap()
                .clone();
            assert!(
                original
                    .command_availability()
                    .unwrap()
                    .matches_analysis(&analysis)
            );
            let payload = DiagnosticSubjectData::from_diagnostic(&original)
                .unwrap()
                .to_value()
                .unwrap();
            assert_eq!(payload["subject"]["command"], command);
            assert_eq!(payload["subject"]["classification"], class);
            assert_eq!(payload["subject"]["domain"], domain);
            assert_eq!(payload["subject"]["sourceChannel"], "document");
            let mut report = original;
            report.message = "unrelated reporting text".into();
            analysis.command_invocations.clear();
            analysis.all_procs.clear();
            analysis.diagnostics.clear();
            assert!(
                report
                    .command_availability()
                    .unwrap()
                    .matches_analysis(&analysis)
            );
            assert_eq!(
                DiagnosticSubjectData::from_diagnostic(&report)
                    .unwrap()
                    .to_value()
                    .unwrap(),
                payload
            );
            let decoded = DiagnosticSubjectData::from_value(&payload, &code.to_string()).unwrap();
            assert!(decoded.package_bytes().is_none());
            assert!(decoded.command_reporting_name().is_none());
            assert!(decoded.math_function_reporting_name().is_none());
            assert!(DiagnosticSubjectData::from_value(&payload, "W123").is_none());
            for field in ["classification", "domain"] {
                let mut unsupported = payload.clone();
                unsupported["subject"][field] = Value::from("future");
                assert!(
                    DiagnosticSubjectData::from_value(&unsupported, &code.to_string()).is_none()
                );
            }
            let mut incorrect_purpose = payload;
            incorrect_purpose["subject"]["classification"] =
                Value::from(if class == "unavailable" {
                    "ruleLoaderRefused"
                } else {
                    "unavailable"
                });
            assert!(
                DiagnosticSubjectData::from_value(&incorrect_purpose, &code.to_string()).is_none()
            );
        }
    }
    #[test]
    fn declared_variable_name_transport_keeps_its_authored_purpose() {
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
        let source = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub store {destination:var value}\n# tcl-lsp: stubs-end\nstore $name payload";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
        let mut diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == DiagCode::W212)
            .expect("declaration-owned variable name")
            .clone();
        let payload = diagnostic_subject_data(&diagnostic).unwrap();
        assert_eq!(payload["subject"]["purpose"], "variableName");
        assert_eq!(payload["subject"]["argument"], 0);
        assert!(DiagnosticSubjectData::from_value(&payload, "W212").is_some());
        assert!(DiagnosticSubjectData::from_value(&payload, "W100").is_none());
        diagnostic.message = "unrelated translated report".to_owned();
        diagnostic.fixes.clear();
        assert_eq!(diagnostic_subject_data(&diagnostic), Some(payload));
    }
    #[test]
    fn original_package_source_subject_transport_retains_selected_purpose_and_package() {
        // naming.diagnostic.original-package-source-advice
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-package-source-advice.md
        let source = "csv::join {a b}";
        let result = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|d| d.code == DiagCode::W120)
            .unwrap();
        let data = DiagnosticSubjectData::from_diagnostic(diagnostic).unwrap();
        let encoded = data.to_value().unwrap();
        assert_eq!(encoded["subject"]["purpose"], "packageRequirement");
        assert_eq!(encoded["subject"]["requiredPackage"], "csv");
        assert_eq!(data.package_bytes(), Some(b"csv".as_slice()));
        assert!(DiagnosticSubjectData::from_value(&encoded, "H301").is_none());
        let mut changed = diagnostic.clone();
        changed.message = "unrelated reporting text".to_owned();
        changed.fixes.clear();
        assert_eq!(diagnostic_subject_data(&changed), Some(encoded));
    }
    #[test]
    fn callback_alias_transport_keeps_capture_baked_and_suffix_premises() {
        // naming.source.original-callback-procedure-target
        // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
        use std::sync::Arc;
        use tcl_compiler::analyser::{
            Analyser, Diagnostic, DiagnosticSubject, Severity, SourceCallbackAritySubject,
        };
        let source = "proc target {a b c d} {}\ninterp alias {} inner {} target INNER\ninterp alias {} outer {} inner OUTER\nlsort -command {outer BAKED} {2 1}";
        let analysis = Analyser::new().analyse(source, "tcl9.0");
        let selection = analysis
            .command_invocations
            .iter()
            .find_map(|row| {
                row.original_callback_signature_lookup
                    .as_ref()
                    .filter(|selection| {
                        selection.prefix().appended_arity()
                            == Some(tcl_registry::AppendedArity::Exactly(2))
                    })
            })
            .unwrap();
        let subject =
            SourceCallbackAritySubject::from_source_lookup(Arc::clone(selection), &[]).unwrap();
        let diagnostic = Diagnostic::new(
            subject.issue().code(),
            subject.span(),
            "presentation".to_owned(),
            Severity::Error,
        )
        .with_subject(DiagnosticSubject::CallbackSourceArity(Arc::new(subject)));
        let data = super::diagnostic_subject_data(&diagnostic).unwrap();
        let details = &data["subject"];
        assert_eq!(details["bakedArgumentCount"], 1);
        assert_eq!(details["appendedCounts"]["counts"], serde_json::json!([2]));
        assert_eq!(details["counts"]["counts"], serde_json::json!([5]));
        let target = &details["sourceTarget"];
        assert_eq!(target["kind"], "localProcedure");
        assert_eq!(target["capturedMinimum"], 2);
        assert_eq!(target["capturedIndeterminate"], false);
        assert_eq!(
            target["captures"][0]["sourceLiteralBytes"],
            serde_json::json!(b"INNER".to_vec())
        );
        assert_eq!(
            target["captures"][1]["sourceLiteralBytes"],
            serde_json::json!(b"OUTER".to_vec())
        );
        assert_eq!(target["captures"][0]["origin"]["kind"], "written");
        assert_eq!(target["captures"][0]["origin"]["ordinal"], 6);
        assert_eq!(target["lineage"].as_array().unwrap().len(), 2);
        let obligations = target["obligations"].as_array().unwrap();
        assert!(obligations.contains(&serde_json::json!("originalCallbackTargetApplicability")));
        assert!(obligations.contains(&serde_json::json!("writtenTransitionApplicability")));
        assert!(selection.original().obligations().contains(
            &tcl_compiler::command_binding::SourceCommandTransitionObligation::WrittenTransitionApplicability
        ));
        assert!(super::DiagnosticSubjectData::from_value(&data, "E003").is_some());
        let mut changed = diagnostic;
        changed.message = "unrelated presentation".to_owned();
        assert_eq!(super::diagnostic_subject_data(&changed), Some(data));
    }
    #[test]
    fn original_index_transport_preserves_captured_operand_purpose_without_message_names() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry(),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let source = "interp alias {} at {} string range {λé} 9; at 12";
        let result = tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, "tcl");
        let mut diagnostic = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == DiagCode::W232)
            .unwrap()
            .clone();
        let payload = diagnostic_subject_data(&diagnostic).unwrap();
        assert_eq!(payload["subject"]["purpose"], "indexBounds");
        assert_eq!(payload["subject"]["argument"], 2);
        assert_eq!(payload["subject"]["writtenArgument"], Value::Null);
        assert_eq!(payload["subject"]["start"], diagnostic.span.start());
        assert_eq!(payload["subject"]["end"], diagnostic.span.end());
        assert!(DiagnosticSubjectData::from_value(&payload, "W232").is_some());
        assert!(DiagnosticSubjectData::from_value(&payload, "W123").is_none());
        diagnostic.message =
            "translated text with a misleading command and variable name".to_owned();
        diagnostic.fixes.clear();
        assert_eq!(diagnostic_subject_data(&diagnostic), Some(payload));
    }
    #[test]
    fn original_template_transport_keeps_selected_prefix_purpose_and_ignores_presentation() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let source = "interp alias {} render {} subst -nocommands; render \"pré $tmpl\"";
        let result = tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, "tcl");
        let mut diagnostic = result
            .diagnostics
            .into_iter()
            .find(|diagnostic| diagnostic.code == DiagCode::W102)
            .expect("original source template warning");
        let payload = diagnostic_subject_data(&diagnostic).unwrap();
        assert_eq!(payload["subject"]["purpose"], "templateSubstitution");
        assert_eq!(payload["subject"]["argument"], 1);
        assert_eq!(payload["subject"]["writtenArgument"], 0);
        assert_eq!(&source[diagnostic.span.as_range()], "\"pré $tmpl\"");
        assert!(DiagnosticSubjectData::from_value(&payload, "W102").is_some());
        assert!(DiagnosticSubjectData::from_value(&payload, "W123").is_none());
        diagnostic.message = "translated text with misleading command and operand names".to_owned();
        diagnostic.fixes.clear();
        assert_eq!(diagnostic_subject_data(&diagnostic), Some(payload));
    }
    #[test]
    fn original_reparse_transport_keeps_both_selected_purposes_without_presentation_names() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let source = "interp alias {} run {} eval {set local}; interp alias {} render {} subst; run [render $tmpl]";
        let result = tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, "tcl");
        for code in [DiagCode::W101, DiagCode::W309] {
            let mut diagnostic = result
                .diagnostics
                .iter()
                .find(|diagnostic| diagnostic.code == code)
                .expect("original selected reparse warning")
                .clone();
            let payload = diagnostic_subject_data(&diagnostic).unwrap();
            assert_eq!(payload["subject"]["purpose"], "scriptReparse");
            assert_eq!(payload["subject"]["argument"], 1);
            assert_eq!(payload["subject"]["writtenArgument"], 0);
            assert_eq!(&source[diagnostic.span.as_range()], "[render $tmpl]");
            assert!(
                DiagnosticSubjectData::from_value(
                    &payload,
                    if code == DiagCode::W101 {
                        "W101"
                    } else {
                        "W309"
                    }
                )
                .is_some()
            );
            assert!(DiagnosticSubjectData::from_value(&payload, "W102").is_none());
            diagnostic.message =
                "translated text with misleading command and operand names".to_owned();
            diagnostic.fixes.clear();
            assert_eq!(diagnostic_subject_data(&diagnostic), Some(payload));
        }
    }
    #[test]
    fn original_path_transport_keeps_alias_ordinals_and_proven_value_premises() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let source = "interp alias {} load {} source -encoding utf-8; load \"rép/$path\"; proc f {} {set p {|literal}; open $p}";
        let result = tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, "tcl");
        for (code, purpose, argument, literal) in [
            (DiagCode::W300, "sourceFilePath", 2, None),
            (DiagCode::W103, "channelPath", 0, Some("|literal")),
        ] {
            let mut diagnostic = result
                .diagnostics
                .iter()
                .find(|finding| finding.code == code)
                .expect("original path finding")
                .clone();
            let payload = diagnostic_subject_data(&diagnostic).unwrap();
            assert_eq!(payload["subject"]["purpose"], purpose);
            assert_eq!(payload["subject"]["argument"], argument);
            assert_eq!(payload["subject"]["writtenArgument"], 0);
            if let Some(literal) = literal {
                assert_eq!(payload["subject"]["supplementalLiterals"][0]["argument"], 0);
                assert_eq!(
                    payload["subject"]["supplementalLiterals"][0]["value"],
                    literal
                );
            }
            assert!(DiagnosticSubjectData::from_value(&payload, code.as_str()).is_some());
            assert!(DiagnosticSubjectData::from_value(&payload, "W102").is_none());
            diagnostic.message = "translated 'open/source' with misleading values".to_owned();
            diagnostic.fixes.clear();
            assert_eq!(diagnostic_subject_data(&diagnostic), Some(payload));
        }
    }

    #[test]
    fn original_crossing_transport_keeps_captured_ordinals_and_ignores_presentation() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let source = "interp alias {} frame {} uplevel 1; interp alias {} child {} interp invokehidden {} -namespace ::N; frame \"pré $value\"; child $command";
        let result = tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, "tcl");
        for (code, argument, span) in [
            (DiagCode::W301, 1, "\"pré $value\""),
            (DiagCode::W312, 4, "$command"),
        ] {
            let mut diagnostic = result
                .diagnostics
                .iter()
                .find(|diagnostic| diagnostic.code == code)
                .expect("original crossing warning")
                .clone();
            let payload = diagnostic_subject_data(&diagnostic).unwrap();
            assert_eq!(payload["subject"]["purpose"], "scriptReparse");
            assert_eq!(payload["subject"]["argument"], argument);
            assert_eq!(payload["subject"]["writtenArgument"], 0);
            assert_eq!(&source[diagnostic.span.as_range()], span);
            assert!(
                DiagnosticSubjectData::from_value(
                    &payload,
                    if code == DiagCode::W301 {
                        "W301"
                    } else {
                        "W312"
                    }
                )
                .is_some()
            );
            assert!(DiagnosticSubjectData::from_value(&payload, "W102").is_none());
            diagnostic.message = "unrelated presentation names".to_owned();
            assert_eq!(diagnostic_subject_data(&diagnostic), Some(payload));
        }
    }
}
