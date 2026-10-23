// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected native method declaration layout, visibility and rename verdicts.
//!
//! This pure argv recipe supplies no method table, call context, compiler
//! preparation, body object or native registration capability.

use crate::InvocationDialect;
use tcl_dialect::{TclVersion, model::Family};

/// Stock descriptor and visibility issued by the genuine root method installer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeTclooCoreMethod {
    /// Object cloning hook installed by the object root.
    Cloned,
    /// Public object destruction method.
    Destroy,
    /// Object namespace evaluation method.
    Eval,
    /// Unknown-method handler installed by the object root.
    Unknown,
    /// Object variable linking method.
    Variable,
    /// Object variable name reporting method.
    Varname,
    /// Public class object creation method.
    Create,
    /// Public anonymous object creation method.
    New,
    /// Class creation method with an explicit namespace.
    CreateWithNamespace,
    /// Public configurable object method.
    Configure,
}
impl NativeTclooCoreMethod {
    #[must_use]
    /// Original stock method selector used by the installed descriptor.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Cloned => "<cloned>",
            Self::Destroy => "destroy",
            Self::Eval => "eval",
            Self::Unknown => "unknown",
            Self::Variable => "variable",
            Self::Varname => "varname",
            Self::Create => "create",
            Self::New => "new",
            Self::CreateWithNamespace => "createWithNamespace",
            Self::Configure => "configure",
        }
    }
    #[must_use]
    /// Visibility supplied by the stock installer for this method.
    pub const fn public(self) -> bool {
        matches!(
            self,
            Self::Destroy | Self::Create | Self::New | Self::Configure
        )
    }
}

/// Current payload type issued by an actual method installer, independent of its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeTclooMethodPayloadType {
    /// Declared procedure method payload.
    Procedure,
    /// Original retained command-prefix forwarding payload.
    Forward,
    /// Property getter payload.
    PropertyGetter,
    /// Property setter payload.
    PropertySetter,
    /// Stock method payload with its installed descriptor.
    Core(NativeTclooCoreMethod),
    /// Slot operation payload with its installed slot name.
    SlotOperation(&'static str),
    /// Slot accessor payload with its installed slot and accessor names.
    SlotAccessor {
        /// Installed slot name used by this accessor descriptor.
        slot: &'static str,
        /// Installed accessor name used by this descriptor.
        accessor: &'static str,
    },
    /// Class root constructor payload.
    ClassConstructor,
}
impl NativeTclooMethodPayloadType {
    /// Actual object-root fallback selector, selected only after no override record.
    #[must_use]
    pub fn object_core_fallback(selector: &[u8]) -> Option<Self> {
        match selector {
            b"destroy" => Some(Self::Core(NativeTclooCoreMethod::Destroy)),
            b"eval" => Some(Self::Core(NativeTclooCoreMethod::Eval)),
            b"variable" => Some(Self::Core(NativeTclooCoreMethod::Variable)),
            b"varname" => Some(Self::Core(NativeTclooCoreMethod::Varname)),
            _ => None,
        }
    }
    /// Actual class-root fallback selector, selected only after no override record.
    #[must_use]
    pub fn class_core_fallback(selector: &[u8]) -> Option<Self> {
        match selector {
            b"create" => Some(Self::Core(NativeTclooCoreMethod::Create)),
            b"new" => Some(Self::Core(NativeTclooCoreMethod::New)),
            b"createWithNamespace" => Some(Self::Core(NativeTclooCoreMethod::CreateWithNamespace)),
            _ => None,
        }
    }
    /// Pure report descriptor; conveys no method record or invocation authority.
    #[must_use]
    pub fn name(self) -> Vec<u8> {
        match self {
            Self::Procedure => b"method".to_vec(),
            Self::Forward => b"forward".to_vec(),
            Self::PropertyGetter => b"PropertyGetter".to_vec(),
            Self::PropertySetter => b"PropertySetter".to_vec(),
            Self::Core(name) => format!("core method: \"{}\"", name.name()).into_bytes(),
            Self::SlotOperation(name) => format!("core method: {name} slot").into_bytes(),
            Self::SlotAccessor { slot, accessor } => {
                format!("core method: {slot} {accessor}").into_bytes()
            }
            Self::ClassConstructor => b"oo::class constructor".to_vec(),
        }
    }
}

/// Call-chain flags issued by the actual chain/filter/unknown producers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeTclooCallReportKind {
    /// Call step issued by a filter chain.
    Filter,
    /// Call step issued by the unknown-method chain.
    Unknown,
    /// Call step whose actual current method record is private.
    Private,
    /// Ordinary method call step.
    Method,
}
impl NativeTclooCallReportKind {
    /// Original `RenderCallChain` precedence; selector spelling is not an input.
    #[must_use]
    pub const fn from_flags(filter: bool, unknown: bool, private: bool) -> Self {
        if filter {
            Self::Filter
        } else if unknown {
            Self::Unknown
        } else if private {
            Self::Private
        } else {
            Self::Method
        }
    }
    #[must_use]
    /// Original report label for the selected call-chain flags.
    pub const fn name(self) -> &'static [u8] {
        match self {
            Self::Filter => b"filter",
            Self::Unknown => b"unknown",
            Self::Private => b"private",
            Self::Method => b"method",
        }
    }
}

const EXPORT_MODES: &[&str] = &["-export", "-private", "-unexport"];

/// Reached command-head String stage for the selected class-variable worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeTclooClassVariableHeadStage {
    /// C9.0 reads the head only to report a missing method context.
    MissingContext,
    /// C9.1 reads the head before checking the current method context.
    BeforeContext,
}

/// Original local-name validation failure before any class-variable links.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeTclooClassVariableNameFailure {
    /// The local scalar name has array-element shape.
    ArrayElement,
    /// The local scalar name includes a namespace separator.
    NamespaceSeparator,
}
impl NativeTclooClassVariableNameFailure {
    /// Original `CString` diagnostic, independent of the counted lookup key.
    #[must_use]
    pub fn message(self, original: &[u8]) -> Vec<u8> {
        let reason: &[u8] = match self {
            Self::ArrayElement => b"scalar variable that looks like an array element",
            Self::NamespaceSeparator => b"local variable with a namespace separator in it",
        };
        [
            b"bad variable name \"".as_slice(),
            tcl_core_types::c_string_extent(original),
            b"\": can't create a ",
            reason,
        ]
        .concat()
    }
    /// Both native shape failures use the same local-element error code.
    #[must_use]
    pub const fn error_code(self) -> &'static [u8] {
        b"TCL UPVAR LOCAL_ELEMENT"
    }
}

/// Pure C9 class-variable operand schedule; supplies no frame or class owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeTclooClassVariableProtocol {
    version: TclVersion,
}
impl NativeTclooClassVariableProtocol {
    /// Core classvariable is available in the inspected C9 workers.
    #[must_use]
    pub fn select(dialect: InvocationDialect) -> Option<Self> {
        let version = dialect.tcl_version?;
        (dialect.family() == Some(Family::Tcl) && version >= TclVersion::V9_0)
            .then_some(Self { version })
    }
    /// Native release-specific head getter stage, before operand validation.
    #[must_use]
    pub const fn head_stage(self) -> NativeTclooClassVariableHeadStage {
        if matches!(self.version, TclVersion::V9_1) {
            NativeTclooClassVariableHeadStage::BeforeContext
        } else {
            NativeTclooClassVariableHeadStage::MissingContext
        }
    }
    /// Check every original name before any target lookup or local link.
    ///
    /// # Errors
    /// Returns the first array-shape or namespace-shape rejection.
    pub fn validate_name(self, original: &[u8]) -> Result<(), NativeTclooClassVariableNameFailure> {
        let names = tcl_syntax::native_variable_name::NativeVariableNameProtocol::for_tcl_version(
            self.version,
        );
        if names.alias_local_is_element(original) {
            return Err(NativeTclooClassVariableNameFailure::ArrayElement);
        }
        if names
            .alias_local_input(tcl_core_types::c_string_extent(original))
            .qualification()
            != tcl_syntax::naming::NativeNameQualification::Unqualified
        {
            return Err(NativeTclooClassVariableNameFailure::NamespaceSeparator);
        }
        Ok(())
    }
}

/// Distinct native method flags; unexported is not true-private.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeTclooMethodVisibility {
    /// Public method, eligible for an external invocation.
    Public,
    /// Ordinary protected method, eligible for `my` dispatch.
    Unexported,
    /// C9 method restricted to the declaring class or object context.
    Private,
}

/// A rename verdict over actual entries in the retained native method table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeTclooMethodRenameFailure {
    /// The original source lookup found no declaration.
    MissingSource,
    /// The destination is the same actual table entry as the source.
    SameEntry,
    /// A different declaration occupies the destination entry.
    OccupiedDestination,
}
impl NativeTclooMethodRenameFailure {
    /// Original worker diagnostic; `%s` presentation has a C string extent.
    #[must_use]
    pub fn message(self, original_name: &[u8]) -> Vec<u8> {
        let name = tcl_core_types::c_string_extent(original_name);
        match self {
            Self::MissingSource => [b"method ".as_slice(), name, b" does not exist"].concat(),
            Self::SameEntry => b"cannot rename method to itself".to_vec(),
            Self::OccupiedDestination => {
                [b"method called ".as_slice(), name, b" already exists"].concat()
            }
        }
    }
    /// Error-code fields for the reached stock worker failure. A name field is
    /// diagnostic presentation, independent of the full counted table key.
    #[must_use]
    pub fn error_code(self, original_name: &[u8]) -> Vec<&[u8]> {
        match self {
            Self::MissingSource => vec![
                b"TCL",
                b"LOOKUP",
                b"METHOD",
                tcl_core_types::c_string_extent(original_name),
            ],
            Self::SameEntry => vec![b"TCL", b"OO", b"RENAME_TO_SELF"],
            Self::OccupiedDestination => vec![b"TCL", b"OO", b"RENAME_OVER"],
        }
    }
}

/// The reached call-context role issued by a genuine native entry producer.
/// A method's name text cannot establish a constructor or destructor role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeTclooMethodReportRole {
    /// Ordinary named method, including a literal sentinel-looking name.
    Named,
    /// Actual constructor call context, inherited unchanged by `next`.
    Constructor,
    /// Actual destructor call context, inherited unchanged by `next`.
    Destructor,
}

/// Selected pure report semantics, independent of physical Foundation headers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeTclooMethodReportProtocol;
impl NativeTclooMethodReportProtocol {
    /// Select through the existing actual `TclOO` declaration availability owner.
    #[must_use]
    pub fn select(dialect: InvocationDialect) -> Option<Self> {
        NativeTclooMethodDefinitionProtocol::select(dialect).map(|_| Self)
    }
    /// Literal for the actual Foundation's separately owned special-role header.
    /// Named records own their own name and have no Foundation report header.
    #[must_use]
    pub const fn foundation_name(self, role: NativeTclooMethodReportRole) -> Option<&'static [u8]> {
        match role {
            NativeTclooMethodReportRole::Named => None,
            NativeTclooMethodReportRole::Constructor => Some(b"<constructor>"),
            NativeTclooMethodReportRole::Destructor => Some(b"<destructor>"),
        }
    }
}

/// Post-head operand positions selected by the actual declaration argc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeTclooMethodLayout {
    /// Original method name operand.
    pub name: usize,
    /// Original C9 export flag operand, when present.
    pub option: Option<usize>,
    /// Original formal parameter-list operand.
    pub parameters: usize,
    /// Original body operand.
    pub body: usize,
}

/// Order of reached export/unexport operand reads and flag effects.
/// The compatibility schedule carries no selected native chronology claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeTclooMethodExportOperandOrder {
    /// Existing unmeasured adapter reads all operands before applying effects.
    CompatibilityReadAll,
    /// Selected C9.1 reads and applies each operand before reaching the next.
    OperandThenEffect,
}

/// Independently selected C method-definition argv semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeTclooMethodDefinitionProtocol {
    options: bool,
    export_order: NativeTclooMethodExportOperandOrder,
}

impl NativeTclooMethodDefinitionProtocol {
    /// Select only an actual C release with `TclOO` support.
    #[must_use]
    pub fn select(dialect: InvocationDialect) -> Option<Self> {
        let version = dialect.tcl_version?;
        (dialect.family() == Some(Family::Tcl) && version >= TclVersion::V8_6).then_some(Self {
            options: version >= TclVersion::V9_0,
            export_order: if version >= TclVersion::V9_1 {
                NativeTclooMethodExportOperandOrder::OperandThenEffect
            } else {
                NativeTclooMethodExportOperandOrder::CompatibilityReadAll
            },
        })
    }

    /// Pure reached operand schedule; supplies no method record or flag authority.
    #[must_use]
    pub const fn export_operand_order(self) -> NativeTclooMethodExportOperandOrder {
        self.export_order
    }

    /// The native option-free form is exactly three operands. C9 adds one
    /// option after the name; a dash in a three-operand formal list is data.
    #[must_use]
    pub const fn layout(self, count: usize) -> Option<NativeTclooMethodLayout> {
        match count {
            3 => Some(NativeTclooMethodLayout {
                name: 0,
                option: None,
                parameters: 1,
                body: 2,
            }),
            4 if self.options => Some(NativeTclooMethodLayout {
                name: 0,
                option: Some(1),
                parameters: 2,
                body: 3,
            }),
            _ => None,
        }
    }

    /// C9 flags use the original-object native index owner, flags zero.
    #[must_use]
    pub const fn export_modes(self) -> Option<&'static [&'static str]> {
        if self.options {
            Some(EXPORT_MODES)
        } else {
            None
        }
    }

    /// Interpret the selected native export-mode ordinal.
    #[must_use]
    pub const fn option_visibility(self, index: usize) -> Option<NativeTclooMethodVisibility> {
        if !self.options {
            return None;
        }
        match index {
            0 => Some(NativeTclooMethodVisibility::Public),
            1 => Some(NativeTclooMethodVisibility::Private),
            2 => Some(NativeTclooMethodVisibility::Unexported),
            _ => None,
        }
    }

    /// The default also applies to a forward declaration. An explicit C9
    /// export option overrides the enclosing private definition frame.
    #[must_use]
    pub fn default_visibility(
        self,
        name: &[u8],
        private_frame: bool,
    ) -> NativeTclooMethodVisibility {
        if self.options && private_frame {
            NativeTclooMethodVisibility::Private
        } else if name.first().is_some_and(u8::is_ascii_lowercase) {
            NativeTclooMethodVisibility::Public
        } else {
            NativeTclooMethodVisibility::Unexported
        }
    }

    /// Check the actual destination entry after the source lookup. This pure
    /// policy cannot establish either entry, its lifetime or lookup authority.
    ///
    /// # Errors
    /// Returns the reached native self/occupied destination verdict.
    pub const fn rename_destination(
        self,
        same_entry: bool,
        occupied: bool,
    ) -> Result<(), NativeTclooMethodRenameFailure> {
        if same_entry {
            Err(NativeTclooMethodRenameFailure::SameEntry)
        } else if occupied {
            Err(NativeTclooMethodRenameFailure::OccupiedDestination)
        } else {
            Ok(())
        }
    }

    /// Native wrong-argument usage including the worker name.
    #[must_use]
    pub const fn usage(self) -> &'static str {
        if self.options {
            "method name ?option? args body"
        } else {
            "method name args body"
        }
    }
}

#[cfg(test)]
mod export_order_tests {
    use super::*;
    #[test]
    fn selected_export_operand_order_does_not_grant_uninspected_releases() {
        // naming.tcloo.original-definition-getter-and-effect-owner

        for version in TclVersion::ALL {
            let selected = NativeTclooMethodDefinitionProtocol::select(
                InvocationDialect::for_version(version),
            );
            assert_eq!(selected.is_some(), version >= TclVersion::V8_6);
            if let Some(selected) = selected {
                assert_eq!(
                    selected.export_operand_order(),
                    if version >= TclVersion::V9_1 {
                        NativeTclooMethodExportOperandOrder::OperandThenEffect
                    } else {
                        NativeTclooMethodExportOperandOrder::CompatibilityReadAll
                    }
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_method_rename_verdict_keeps_entry_identity_and_diagnostic_extent_separate() {
        // naming.tcloo.original-method-rename-operands-and-records

        // Pure verdict over supplied actual entry facts; no table or invocation admission.
        use NativeTclooMethodRenameFailure as Failure;
        for version in TclVersion::ALL {
            let selected = NativeTclooMethodDefinitionProtocol::select(
                InvocationDialect::for_version(version),
            );
            assert_eq!(selected.is_some(), version >= TclVersion::V8_6);
            let Some(selected) = selected else {
                continue;
            };
            assert_eq!(
                selected.rename_destination(true, true),
                Err(Failure::SameEntry)
            );
            assert_eq!(
                selected.rename_destination(false, true),
                Err(Failure::OccupiedDestination)
            );
            assert_eq!(selected.rename_destination(false, false), Ok(()));
        }
        let jim = InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").unit_profile(),
        );
        assert!(NativeTclooMethodDefinitionProtocol::select(jim).is_none());
        assert_eq!(
            Failure::MissingSource.message(b"missing\0tail"),
            b"method missing does not exist"
        );
        assert_eq!(
            Failure::MissingSource.error_code(b"missing\0tail"),
            vec![b"TCL".as_slice(), b"LOOKUP", b"METHOD", b"missing"]
        );
        assert_eq!(
            Failure::OccupiedDestination.message(b"o\0x"),
            b"method called o already exists"
        );
        assert_eq!(
            Failure::SameEntry.error_code(b"counted\0name"),
            vec![b"TCL".as_slice(), b"OO", b"RENAME_TO_SELF"]
        );
    }

    #[test]
    fn method_argc_and_visibility_keep_private_separate_from_unexported() {
        for version in TclVersion::ALL {
            let selected = NativeTclooMethodDefinitionProtocol::select(
                InvocationDialect::for_version(version),
            );
            assert_eq!(selected.is_some(), version >= TclVersion::V8_6);
            let Some(selected) = selected else {
                continue;
            };
            assert_eq!(selected.layout(3).unwrap().option, None);
            assert_eq!(selected.layout(4).is_some(), version >= TclVersion::V9_0);
            assert!(selected.layout(2).is_none());
            assert!(selected.layout(5).is_none());
            assert_eq!(
                selected.default_visibility(b"lower", false),
                NativeTclooMethodVisibility::Public
            );
            assert_eq!(
                selected.default_visibility(b"Upper", false),
                NativeTclooMethodVisibility::Unexported
            );
            if version >= TclVersion::V9_0 {
                assert_eq!(
                    selected.default_visibility(b"lower", true),
                    NativeTclooMethodVisibility::Private
                );
                assert_eq!(
                    selected.option_visibility(0),
                    Some(NativeTclooMethodVisibility::Public)
                );
                assert_eq!(
                    selected.option_visibility(1),
                    Some(NativeTclooMethodVisibility::Private)
                );
                assert_eq!(
                    selected.option_visibility(2),
                    Some(NativeTclooMethodVisibility::Unexported)
                );
            }
        }
        let jim = InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").unit_profile(),
        );
        assert!(NativeTclooMethodDefinitionProtocol::select(jim).is_none());
    }
}

#[cfg(test)]
mod classvariable_tests {
    use super::*;

    #[test]
    fn selected_classvariable_schedule_keeps_availability_and_cstring_shapes_distinct() {
        // naming.tcloo.original-classvariable-preflight-and-link-owner

        for version in TclVersion::ALL {
            let selected =
                NativeTclooClassVariableProtocol::select(InvocationDialect::for_version(version));
            assert_eq!(selected.is_some(), version >= TclVersion::V9_0);
            let Some(selected) = selected else { continue };
            assert_eq!(
                selected.head_stage(),
                if version == TclVersion::V9_1 {
                    NativeTclooClassVariableHeadStage::BeforeContext
                } else {
                    NativeTclooClassVariableHeadStage::MissingContext
                },
            );
            assert_eq!(selected.validate_name(b"plain"), Ok(()));
            assert_eq!(selected.validate_name(b"plain\0::tail(k)"), Ok(()));
            assert_eq!(
                selected.validate_name(b"a(k)\0tail"),
                Err(NativeTclooClassVariableNameFailure::ArrayElement)
            );
            assert_eq!(
                selected.validate_name(b"ns::a(k)"),
                Err(NativeTclooClassVariableNameFailure::ArrayElement)
            );
            assert_eq!(
                selected.validate_name(b"ns::a"),
                Err(NativeTclooClassVariableNameFailure::NamespaceSeparator)
            );
        }
        let jim = InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").unit_profile(),
        );
        assert!(NativeTclooClassVariableProtocol::select(jim).is_none());
        assert_eq!(
            NativeTclooClassVariableNameFailure::NamespaceSeparator.message(b"ns::a\0tail"),
            b"bad variable name \"ns::a\": can't create a local variable with a namespace separator in it",
        );
        assert_eq!(
            NativeTclooClassVariableNameFailure::NamespaceSeparator.error_code(),
            b"TCL UPVAR LOCAL_ELEMENT"
        );
    }
}

#[cfg(test)]
mod call_report_tests {
    use super::{NativeTclooCallReportKind as Kind, NativeTclooMethodPayloadType as Type};
    #[test]
    fn original_call_report_flags_and_payload_types_do_not_classify_selector_spellings() {
        // naming.tcloo.original-entered-call-chain-report-record-and-flags

        assert_eq!(Kind::from_flags(true, true, true), Kind::Filter);
        assert_eq!(Kind::from_flags(false, true, true), Kind::Unknown);
        assert_eq!(Kind::from_flags(false, false, true), Kind::Private);
        assert_eq!(Kind::from_flags(false, false, false), Kind::Method);
        assert_eq!(Type::Forward.name(), b"forward");
        assert_eq!(Type::Procedure.name(), b"method");
        assert_eq!(Type::PropertyGetter.name(), b"PropertyGetter");
        assert_eq!(Type::PropertySetter.name(), b"PropertySetter");
        assert_eq!(
            Type::Core(super::NativeTclooCoreMethod::Destroy).name(),
            b"core method: \"destroy\""
        );
        assert_eq!(
            Type::SlotOperation("-append").name(),
            b"core method: -append slot"
        );
        assert_eq!(
            Type::SlotAccessor {
                slot: "define::mixin",
                accessor: "Resolver"
            }
            .name(),
            b"core method: define::mixin Resolver"
        );
        assert_eq!(Type::ClassConstructor.name(), b"oo::class constructor");
        assert!(Type::object_core_fallback(b"<constructor>").is_none());
        assert!(Type::object_core_fallback(b"").is_none());
        assert!(Type::class_core_fallback(b"<destructor>").is_none());
    }
}
