// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected native method declaration layout, visibility and rename verdicts.
//!
//! This pure argv recipe supplies no method table, call context, compiler
//! preparation, body object or native registration capability.

use crate::InvocationDialect;
use tcl_dialect::{TclVersion, model::Family};

const EXPORT_MODES: &[&str] = &["-export", "-private", "-unexport"];

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
    /// Select through the existing actual TclOO declaration availability owner.
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
        // docs/design/analysis/name-resolution-proofs/tcloo-original-method-rename-operands-and-records.md
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
        // Native proof: naming.tcloo.method-original-option-layout
        // docs/design/analysis/name-resolution-proofs/method-original-option-layout.md
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
