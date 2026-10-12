// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Selected list-assignment invocation grammar, separate from compilation.

use crate::{Arity, InvocationDialect, SemanticOperationId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ListAssignmentSignature {
    RequiredTarget,
    OptionalTargets,
    JimRequiredTarget,
}

/// The argument floor and diagnostic usage of an actual native handler.
/// This is signature metadata, not command lookup or compiler admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeListAssignmentInvocation {
    signature: ListAssignmentSignature,
}

impl NativeListAssignmentInvocation {
    /// Counts arguments after the command head, including the list operand.
    #[must_use]
    pub const fn arity(self) -> Arity {
        Arity::at_least(match self.signature {
            ListAssignmentSignature::OptionalTargets => 1,
            ListAssignmentSignature::RequiredTarget
            | ListAssignmentSignature::JimRequiredTarget => 2,
        })
    }

    /// Test the actual argv length without narrowing a potentially large count.
    #[must_use]
    pub fn accepts(self, argument_count: usize) -> bool {
        argument_count >= usize::from(self.arity().min)
    }

    /// Exact native usage tail, selected independently of catalogue presentation.
    #[must_use]
    pub const fn usage(self) -> &'static str {
        match self.signature {
            ListAssignmentSignature::RequiredTarget => "lassign list varName ?varName ...?",
            ListAssignmentSignature::OptionalTargets => "lassign list ?varName ...?",
            ListAssignmentSignature::JimRequiredTarget => "lassign varList list ?varName ...?",
        }
    }
}

impl InvocationDialect {
    /// Select only a measured native family and release. Tcl compatibility on
    /// a vendor point, an unknown point or a conflicting point grants no row.
    #[must_use]
    pub fn list_assignment_invocation(self) -> Option<NativeListAssignmentInvocation> {
        use tcl_dialect::model::{Family, Release};
        let point = self.execution_point()?;
        let signature = match (point.family(), point.tcl_version()) {
            (Family::Tcl, Some(tcl_dialect::TclVersion::V8_5)) => {
                ListAssignmentSignature::RequiredTarget
            }
            (
                Family::Tcl,
                Some(
                    tcl_dialect::TclVersion::V8_6
                    | tcl_dialect::TclVersion::V9_0
                    | tcl_dialect::TclVersion::V9_1,
                ),
            ) => ListAssignmentSignature::OptionalTargets,
            (Family::Jim, _) if point.release() == Release::JIM_0_84 => {
                ListAssignmentSignature::JimRequiredTarget
            }
            _ => return None,
        };
        Some(NativeListAssignmentInvocation { signature })
    }
}

pub(crate) fn operation_arity(
    operation: SemanticOperationId,
    dialect: Option<InvocationDialect>,
) -> Option<Arity> {
    if operation != SemanticOperationId::Intrinsic(crate::IntrinsicId::ListAssign) {
        return None;
    }
    Some(dialect?.list_assignment_invocation()?.arity())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::model::{DialectPoint, Family, Release};

    #[test]
    fn selected_handler_floor_and_usage_preserve_measured_native_rows() {
        // Native proof: naming.list.original-lassign-target-arity
        // docs/design/analysis/name-resolution-proofs/list-original-lassign-target-arity.md
        for (version, minimum, usage) in [
            (
                tcl_dialect::TclVersion::V8_5,
                2,
                "lassign list varName ?varName ...?",
            ),
            (
                tcl_dialect::TclVersion::V8_6,
                1,
                "lassign list ?varName ...?",
            ),
            (
                tcl_dialect::TclVersion::V9_0,
                1,
                "lassign list ?varName ...?",
            ),
            (
                tcl_dialect::TclVersion::V9_1,
                1,
                "lassign list ?varName ...?",
            ),
        ] {
            let dialect = InvocationDialect::for_version(version);
            let grammar = dialect.list_assignment_invocation().unwrap();
            assert_eq!(grammar.arity(), Arity::at_least(minimum));
            assert_eq!(grammar.usage(), usage);
            assert!(!grammar.accepts(usize::from(minimum - 1)));
            assert!(grammar.accepts(usize::from(minimum)));
            assert!(grammar.accepts(usize::MAX));
        }
        let jim = InvocationDialect::of_point(DialectPoint::canonical(Release::JIM_0_84));
        let grammar = jim.list_assignment_invocation().unwrap();
        assert_eq!(grammar.arity(), Arity::at_least(2));
        assert_eq!(grammar.usage(), "lassign varList list ?varName ...?");
        assert!(!grammar.accepts(1));
        assert!(grammar.accepts(2));
        assert!(
            InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4)
                .list_assignment_invocation()
                .is_none()
        );
        let mut unknown = InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        unknown.native_family = None;
        assert!(unknown.list_assignment_invocation().is_none());
        let mut foreign = InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        foreign.core_point = jim.core_point;
        assert!(foreign.list_assignment_invocation().is_none());
        let vendor =
            InvocationDialect::of_profile(tcl_dialect::DialectProfile::find("f5-irules").unwrap());
        assert_ne!(vendor.native_family, Some(Family::Tcl));
        assert!(vendor.list_assignment_invocation().is_none());
    }

    #[test]
    fn selected_and_legacy_signature_views_share_operation_arity() {
        // Native proof: naming.list.original-lassign-target-arity
        // docs/design/analysis/name-resolution-proofs/list-original-lassign-target-arity.md
        // This is Rust metadata coverage; renamed labels do not grant native lookup.
        let mut registry = crate::CommandRegistry::build_default();
        registry.insert(crate::CommandSpec {
            name: "assigned-label",
            semantic_operation: Some(SemanticOperationId::Intrinsic(
                crate::IntrinsicId::ListAssign,
            )),
            arity: Arity::at_least(1),
            ..crate::CommandSpec::DEFAULT
        });
        for (dialect, minimum) in [
            (
                InvocationDialect::for_version(tcl_dialect::TclVersion::V8_5),
                2,
            ),
            (
                InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
                1,
            ),
            (
                InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0),
                1,
            ),
            (
                InvocationDialect::for_version(tcl_dialect::TclVersion::V9_1),
                1,
            ),
            (
                InvocationDialect::of_point(DialectPoint::canonical(Release::JIM_0_84)),
                2,
            ),
        ] {
            let words =
                crate::InvocationWords::literals("assigned-label", &["A B"]).with_dialect(dialect);
            let selected = registry
                .resolve_structured_invocation(words, None)
                .resolved()
                .unwrap();
            assert_eq!(selected.semantics.arity, Arity::at_least(minimum));
            assert_eq!(selected.facts().arity, Arity::at_least(minimum));
            assert_eq!(
                selected.facts_after_success().arity,
                Arity::at_least(minimum)
            );
            let legacy = registry
                .resolve_call("assigned-label", &["A B"], None)
                .unwrap();
            assert_eq!(
                legacy.arity_for_arguments(words.arguments()),
                Arity::at_least(minimum)
            );
        }
        let without_point = registry
            .resolve_invocation("assigned-label", &["A B"], None)
            .unwrap();
        assert_eq!(without_point.semantics.arity, Arity::at_least(1));
        assert_eq!(
            operation_arity(
                SemanticOperationId::Invoke,
                Some(InvocationDialect::for_version(
                    tcl_dialect::TclVersion::V8_5
                ))
            ),
            None
        );
    }
}
