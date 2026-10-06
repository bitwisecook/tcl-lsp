// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Independently selected naming policy and bounded appliance observations.

use super::{
    NamePolicyProtocol, NameProjectionUnavailable, NativeVariableInputForm,
    NativeVariableProjection,
};

/// Exact software, execution engine and reached event of a name observation.
/// This is not an attestation of another running interpreter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MeasuredBigIpNameScope {
    /// BIG-IP 21.1.0.1 build 0.0.26, TMM `HTTP_REQUEST`, four reached units.
    BigIp21_1_0_1Build0_0_26TmmHttpRequest,
}

/// A measured variable operand purpose, independent of native C object APIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObservedVariableNamePurpose {
    /// The full dynamic scalar receiver used by the scalar read/write controls.
    ScalarReceiver,
    /// The full root of the array-root controls, including an embedded NUL.
    ArrayRoot,
    /// The combined dynamic `root(index)` operand of the array controls.
    CombinedElement,
}

/// An operation-bounded counted recipe; unsupported grammar and purposes remain unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObservedBigIpNamePolicy {
    scope: MeasuredBigIpNameScope,
}

impl ObservedBigIpNamePolicy {
    /// Select the exact measured scope explicitly. This grants no native host,
    /// object-header, compiler, namespace, static-cell or command-name recipe.
    #[must_use]
    pub const fn for_measured_scope(scope: MeasuredBigIpNameScope) -> Self {
        Self { scope }
    }

    #[must_use]
    pub const fn scope(self) -> MeasuredBigIpNameScope {
        self.scope
    }

    /// Project counted dynamic names within the observed unqualified operand grammar.
    /// The result supplies no actual cell, frame, alias, trace or presence proof.
    ///
    /// # Errors
    /// Separate forms, qualification, empty names and other grammars are unmeasured.
    pub fn variable_input<'a>(
        self,
        original: NativeVariableInputForm<'a>,
        purpose: ObservedVariableNamePurpose,
    ) -> Result<ExecutionVariableNameProjection<'a>, NameProjectionUnavailable> {
        let NativeVariableInputForm::Combined(written) = original else {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        };
        let (root, element) = match purpose {
            ObservedVariableNamePurpose::ScalarReceiver if measured_unqualified_root(written) => {
                (written, None)
            }
            ObservedVariableNamePurpose::ArrayRoot if measured_unqualified_root(written) => {
                (written, None)
            }
            ObservedVariableNamePurpose::CombinedElement => {
                let (root, element) = measured_combined_element(written)
                    .ok_or(NameProjectionUnavailable::PurposeNotModelled)?;
                (root, Some(element))
            }
            _ => return Err(NameProjectionUnavailable::PurposeNotModelled),
        };
        Ok(ExecutionVariableNameProjection(
            VariableProjection::Observed {
                original,
                root,
                element,
                policy: self,
                purpose,
            },
        ))
    }
}

// These operation boundaries preserve already-produced bytes. They do not
// select a byte producer or generalise qualification and input grammar.
fn measured_unqualified_root(root: &[u8]) -> bool {
    !root.is_empty() && !root.iter().any(|byte| matches!(byte, b':' | b'(' | b')'))
}

fn measured_combined_element(written: &[u8]) -> Option<(&[u8], &[u8])> {
    let open = written.iter().position(|byte| *byte == b'(')?;
    let root = &written[..open];
    let element = written.get(open + 1..written.len().checked_sub(1)?)?;
    (written.last() == Some(&b')')
        && measured_unqualified_root(root)
        && !element.is_empty()
        && !element.iter().any(|byte| matches!(byte, b'(' | b')')))
    .then_some((root, element))
}

/// Name semantics selected independently of producer-string and compiler policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutionNamePolicy {
    /// Actual audited native or explicitly authored simulation recipe.
    NativeRecipe(NamePolicyProtocol),
    /// Bounded appliance recipe with its own build, event and purpose boundary.
    ObservedBigIp(ObservedBigIpNamePolicy),
}

impl ExecutionNamePolicy {
    /// Preserve the original native/authored issuer without promoting observations.
    #[must_use]
    pub const fn native_recipe(self) -> Option<NamePolicyProtocol> {
        match self {
            Self::NativeRecipe(policy) => Some(policy),
            Self::ObservedBigIp(_) => None,
        }
    }

    /// Select a receiver projection. The purpose is required for bounded observations
    /// and cannot infer another API's input form from an equivalent display name.
    ///
    /// # Errors
    /// An unmeasured appliance purpose or input remains unavailable.
    pub fn variable_input(
        self,
        original: NativeVariableInputForm<'_>,
        observed_purpose: ObservedVariableNamePurpose,
    ) -> Result<ExecutionVariableNameProjection<'_>, NameProjectionUnavailable> {
        match self {
            Self::NativeRecipe(policy) => Ok(ExecutionVariableNameProjection(
                VariableProjection::Native(match original {
                    NativeVariableInputForm::Combined(written) => {
                        policy.recipe().combined_variable_input(written)
                    }
                    NativeVariableInputForm::Separate { root, element } => {
                        policy.recipe().separate_variable_input(root, element)
                    }
                }),
            )),
            Self::ObservedBigIp(policy) => policy.variable_input(original, observed_purpose),
        }
    }
}

/// Selected names without any promotion to physical storage or native caches.
#[derive(Debug, Clone)]
pub struct ExecutionVariableNameProjection<'a>(VariableProjection<'a>);

#[derive(Debug, Clone)]
enum VariableProjection<'a> {
    Native(NativeVariableProjection<'a>),
    Observed {
        original: NativeVariableInputForm<'a>,
        root: &'a [u8],
        element: Option<&'a [u8]>,
        policy: ObservedBigIpNamePolicy,
        purpose: ObservedVariableNamePurpose,
    },
}

impl<'a> ExecutionVariableNameProjection<'a> {
    /// Original native projection only when a native/authored recipe selected it.
    /// Measured byte facts cannot manufacture native qualifier or trace metadata.
    #[must_use]
    pub const fn native_projection(&self) -> Option<&NativeVariableProjection<'a>> {
        match &self.0 {
            VariableProjection::Native(projection) => Some(projection),
            VariableProjection::Observed { .. } => None,
        }
    }
    #[must_use]
    pub fn original(&self) -> NativeVariableInputForm<'a> {
        match &self.0 {
            VariableProjection::Native(projection) => projection.original(),
            VariableProjection::Observed { original, .. } => *original,
        }
    }

    #[must_use]
    pub fn root(&self) -> &[u8] {
        match &self.0 {
            VariableProjection::Native(projection) => projection.root().selected(),
            VariableProjection::Observed { root, .. } => root,
        }
    }

    #[must_use]
    pub fn element(&self) -> Option<&[u8]> {
        match &self.0 {
            VariableProjection::Native(projection) => projection
                .element()
                .map(super::NativeNameProjection::selected),
            VariableProjection::Observed { element, .. } => *element,
        }
    }

    /// Independently retained observed scope and purpose, absent for native recipes.
    #[must_use]
    pub fn observed_issuer(
        &self,
    ) -> Option<(ObservedBigIpNamePolicy, ObservedVariableNamePurpose)> {
        match &self.0 {
            VariableProjection::Observed {
                policy, purpose, ..
            } => Some((*policy, *purpose)),
            VariableProjection::Native(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counted_observations_keep_full_original_keys_and_decline_other_bytes_and_forms() {
        let policy = ObservedBigIpNamePolicy::for_measured_scope(
            MeasuredBigIpNameScope::BigIp21_1_0_1Build0_0_26TmmHttpRequest,
        );
        for written in [
            b"relocated".as_slice(),
            b"renamed\0key",
            b"raw\xe9",
            b"raw\xc3\xa9",
            b"raw\x01",
        ] {
            for purpose in [
                ObservedVariableNamePurpose::ScalarReceiver,
                ObservedVariableNamePurpose::ArrayRoot,
            ] {
                let projection = policy
                    .variable_input(NativeVariableInputForm::Combined(written), purpose)
                    .unwrap();
                assert_eq!(projection.root(), written);
                assert_eq!(projection.element(), None);
                assert!(projection.native_projection().is_none());
            }
        }
        let projection = policy
            .variable_input(
                NativeVariableInputForm::Combined(b"other\0root(index\0byte)"),
                ObservedVariableNamePurpose::CombinedElement,
            )
            .unwrap();
        assert_eq!(projection.root(), b"other\0root");
        assert_eq!(projection.element(), Some(b"index\0byte".as_slice()));
        for written in [
            b"".as_slice(),
            b"::static::A\0B",
            b"qualified::root",
            b"colon:root",
            b"root()",
            b"root((index))",
            b"(index)",
            b"root(index)tail",
        ] {
            assert!(
                policy
                    .variable_input(
                        NativeVariableInputForm::Combined(written),
                        ObservedVariableNamePurpose::CombinedElement
                    )
                    .is_err()
            );
            assert!(
                policy
                    .variable_input(
                        NativeVariableInputForm::Combined(written),
                        ObservedVariableNamePurpose::ScalarReceiver
                    )
                    .is_err()
            );
        }
        assert!(
            policy
                .variable_input(
                    NativeVariableInputForm::Separate {
                        root: b"root",
                        element: Some(b"index")
                    },
                    ObservedVariableNamePurpose::CombinedElement
                )
                .is_err()
        );
        assert_eq!(
            ExecutionNamePolicy::ObservedBigIp(policy).native_recipe(),
            None
        );
    }
}
