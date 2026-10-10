// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected native method declaration layout and visibility.
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

/// Independently selected C method-definition argv semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeTclooMethodDefinitionProtocol {
    options: bool,
}

impl NativeTclooMethodDefinitionProtocol {
    /// Select only an actual C release with `TclOO` support.
    #[must_use]
    pub fn select(dialect: InvocationDialect) -> Option<Self> {
        let version = dialect.tcl_version?;
        (dialect.family() == Some(Family::Tcl) && version >= TclVersion::V8_6).then_some(Self {
            options: version >= TclVersion::V9_0,
        })
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
mod tests {
    use super::*;

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
