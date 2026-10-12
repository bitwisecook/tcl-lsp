// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Purpose-selected variable root geometry without a Unicode bridge.

use super::{NativeNameContext, NativeNameProtocol, NativeNameQualification};
use tcl_core_types::{ByteNamespacePath, NameBytes};

/// Name geometry selected by the runtime variable receiver purpose.
/// This carries bytes and component boundaries, never a table or slot receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeVariableRootGeometry {
    /// An unqualified C root or a relative Jim name in the current frame.
    Local(NameBytes),
    /// C namespace table components and its selected simple key.
    CNamespace {
        /// Constructed namespace path, independent of its display.
        namespace: ByteNamespacePath,
        /// Exact selected table key.
        simple: NameBytes,
    },
    /// Jim's root-flat global variable key from an absolute name.
    JimAbsolute(NameBytes),
}

/// A combined-name bridge cannot preserve the selected input form.
/// This is a spelling limitation, not a variable lookup or storage result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeVariableBridgeUnavailable {
    /// A first operand already denotes an element and also has a second index.
    Part1IsElement,
    /// Combining or rooting the operands changes a selected root or element.
    Unrepresentable,
}

impl NativeNameProtocol {
    /// Spell original variable operands for a bridge that accepts one combined
    /// name. The native selector validates the resulting root and element;
    /// rooting must not clip a counted root or reinterpret a literal array name.
    /// This supplies no cell, namespace token, observer or lookup authority.
    ///
    /// # Errors
    /// Refuses a first operand already parsed as an element beside a second
    /// index, or a spelling whose re-projection changes the selected operands.
    pub fn variable_combined_bridge_input(
        self,
        original: super::NativeVariableInputForm<'_>,
        global_only: bool,
    ) -> Result<Vec<u8>, NativeVariableBridgeUnavailable> {
        let (selected, mut combined) = match original {
            super::NativeVariableInputForm::Combined(bytes)
            | super::NativeVariableInputForm::Separate {
                root: bytes,
                element: None,
            } => (self.combined_variable_input(bytes), bytes.to_vec()),
            super::NativeVariableInputForm::Separate {
                root,
                element: Some(element),
            } => {
                if self.combined_variable_input(root).element().is_some() {
                    return Err(NativeVariableBridgeUnavailable::Part1IsElement);
                }
                let mut combined = root.to_vec();
                combined.push(b'(');
                combined.extend_from_slice(element);
                combined.push(b')');
                (self.separate_variable_input(root, Some(element)), combined)
            }
        };
        let add_root =
            global_only && selected.root().qualification() != NativeNameQualification::Absolute;
        if add_root {
            combined.splice(..0, b"::".iter().copied());
        }
        let reproduced = self.combined_variable_input(&combined);
        let reproduced_root = if add_root {
            reproduced.root().selected().get(2..)
        } else {
            Some(reproduced.root().selected())
        };
        if reproduced_root != Some(selected.root().selected())
            || reproduced
                .element()
                .map(super::NativeNameProjection::selected)
                != selected
                    .element()
                    .map(super::NativeNameProjection::selected)
            || global_only && reproduced.root().qualification() != NativeNameQualification::Absolute
        {
            return Err(NativeVariableBridgeUnavailable::Unrepresentable);
        }
        Ok(combined)
    }
}

impl NativeNameProtocol {
    /// Select the runtime root's qualification before resolving a cell.
    /// A separator after a raw NUL is governed by the root input policy and
    /// cannot be recovered by reparsing a display or a compiler-local key.
    #[must_use]
    pub fn variable_root_geometry(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> NativeVariableRootGeometry {
        let selected = self.variable_root_input(original);
        if self.is_jim084() {
            return if selected.qualification() == NativeNameQualification::Absolute {
                NativeVariableRootGeometry::JimAbsolute(
                    super::jim_global_variable_key_bytes(b"::", selected.selected()).into(),
                )
            } else {
                NativeVariableRootGeometry::Local(selected.selected().into())
            };
        }
        if selected.qualification() == NativeNameQualification::Unqualified {
            return NativeVariableRootGeometry::Local(selected.selected().into());
        }
        let mut namespace = if selected.qualification() == NativeNameQualification::Absolute {
            ByteNamespacePath::root()
        } else {
            context.namespace.clone()
        };
        let simple = super::written_command_tail(selected.selected());
        let parts = super::qualifier_segments(selected.selected());
        let qualifier_count = parts.len().saturating_sub(usize::from(!simple.is_empty()));
        for qualifier in &parts[..qualifier_count] {
            namespace.push(*qualifier);
        }
        NativeVariableRootGeometry::CNamespace {
            namespace,
            simple: simple.into(),
        }
    }
}

impl NativeNameProtocol {
    /// Replace the written root tail selected by this native input form. The
    /// original qualifier spelling, ignored suffix and combined element bytes
    /// remain exact. This partitions the supplied original root; it does not
    /// infer a namespace relationship from Jim's flat stored variable key.
    /// The caller independently checks the old and proposed symbol owners.
    #[must_use]
    pub fn replace_variable_root_tail(
        self,
        original: super::NativeVariableInputForm<'_>,
        new_tail: &[u8],
    ) -> Option<Vec<u8>> {
        let proposed = self.combined_variable_input(new_tail);
        if proposed.element().is_some()
            || proposed.root().selected() != new_tail
            || super::is_qualified(new_tail)
        {
            return None;
        }
        let extent = self.variable_root_tail_extent(original)?;
        let bytes = match original {
            super::NativeVariableInputForm::Combined(bytes) => bytes,
            super::NativeVariableInputForm::Separate { root, .. } => root,
        };
        let mut replacement = bytes[..extent.start].to_vec();
        replacement.extend_from_slice(new_tail);
        replacement.extend_from_slice(&bytes[extent.end..]);
        Some(replacement)
    }

    /// Exact byte extent of the selected written variable root tail. Combined
    /// and separate-root forms retain their independent parsing boundaries.
    /// This partitions the original input, never a rendered variable-table key.
    #[must_use]
    pub fn variable_root_tail_extent(
        self,
        original: super::NativeVariableInputForm<'_>,
    ) -> Option<std::ops::Range<usize>> {
        let projection = match original {
            super::NativeVariableInputForm::Combined(bytes) => self.combined_variable_input(bytes),
            super::NativeVariableInputForm::Separate { root, element } => {
                self.separate_variable_input(root, element)
            }
        };
        let bytes = match original {
            super::NativeVariableInputForm::Combined(bytes) => bytes,
            super::NativeVariableInputForm::Separate { root, .. } => root,
        };
        let root = projection.root().selected();
        let tail = if self.variable_root_input(root).qualification()
            == super::NativeNameQualification::Unqualified
            || self.is_jim084()
                && self.variable_root_input(root).qualification()
                    != super::NativeNameQualification::Absolute
        {
            root
        } else {
            super::written_command_tail(root)
        };
        (bytes.get(..root.len()) == Some(root)).then_some(())?;
        Some(root.len().checked_sub(tail.len())?..root.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::TclVersion;

    #[test]
    fn combined_bridge_preserves_selected_parts_and_refuses_lost_counted_roots() {
        // Software bridge contract over the shared selectors. Native variable
        // input extents are recorded independently in variable-table-actual-abi-inputs.md.
        // No cell, host API execution or physical variable identity is asserted.
        use super::super::NativeVariableInputForm::{Combined, Separate};
        let protocol = NativeNameProtocol::C(TclVersion::V9_0);
        for (input, global, expected) in [
            (Combined(b"x"), false, b"x".as_slice()),
            (Combined(b"ns::v"), true, b"::ns::v".as_slice()),
            (Combined(b"::::v(k)"), true, b"::::v(k)".as_slice()),
            (Combined(b"x\0tail"), false, b"x\0tail".as_slice()),
            (
                Separate {
                    root: b"a)",
                    element: Some(b"k\0tail"),
                },
                false,
                b"a)(k\0tail)".as_slice(),
            ),
            (
                Separate {
                    root: "café".as_bytes(),
                    element: Some(b""),
                },
                true,
                "::café()".as_bytes(),
            ),
        ] {
            assert_eq!(
                protocol.variable_combined_bridge_input(input, global),
                Ok(expected.to_vec())
            );
        }
        assert_eq!(
            protocol.variable_combined_bridge_input(
                Separate {
                    root: b"a(k)",
                    element: Some(b"j")
                },
                false
            ),
            Err(NativeVariableBridgeUnavailable::Part1IsElement)
        );
        for (input, global) in [
            (Combined(b"x\0tail"), true),
            (
                Separate {
                    root: b"a(",
                    element: Some(b"j"),
                },
                false,
            ),
            (
                Separate {
                    root: b"a\0tail",
                    element: Some(b"j"),
                },
                false,
            ),
        ] {
            assert_eq!(
                protocol.variable_combined_bridge_input(input, global),
                Err(NativeVariableBridgeUnavailable::Unrepresentable)
            );
        }
        // The selected C8.4 scalar extent is independently different from C9.
        assert_eq!(
            NativeNameProtocol::C(TclVersion::V8_4)
                .variable_combined_bridge_input(Combined(b"x\0tail"), true),
            Ok(b"::x\0tail".to_vec())
        );
    }

    #[test]
    fn original_variable_tail_replacement_preserves_qualifiers_and_selected_index_form() {
        use super::super::NativeVariableInputForm;
        for version in TclVersion::ALL {
            let protocol = NativeNameProtocol::C(version);
            assert_eq!(
                protocol.variable_root_tail_extent(NativeVariableInputForm::Combined(
                    b"::N:::v(k\0suffix)"
                )),
                Some(6..7)
            );
            assert_eq!(
                protocol.variable_root_tail_extent(NativeVariableInputForm::Separate {
                    root: b"a(b)",
                    element: Some(b"k")
                }),
                Some(0..4)
            );
            assert_eq!(
                protocol.variable_root_tail_extent(NativeVariableInputForm::Combined(b"a(b)")),
                Some(0..1)
            );
            assert_eq!(
                protocol.replace_variable_root_tail(
                    NativeVariableInputForm::Combined(b"::N:::v(k\0suffix)"),
                    b"new"
                ),
                Some(b"::N:::new(k\0suffix)".to_vec())
            );
            assert_eq!(
                protocol.replace_variable_root_tail(
                    NativeVariableInputForm::Separate {
                        root: b"a(b)",
                        element: Some(b"k")
                    },
                    b"new"
                ),
                Some(b"new".to_vec())
            );
            assert_eq!(
                protocol
                    .replace_variable_root_tail(NativeVariableInputForm::Combined(b"a(b)"), b"new"),
                Some(b"new(b)".to_vec())
            );
            assert!(
                protocol
                    .replace_variable_root_tail(
                        NativeVariableInputForm::Combined(b"v"),
                        b"other::new"
                    )
                    .is_none()
            );
            assert!(
                protocol
                    .replace_variable_root_tail(NativeVariableInputForm::Combined(b"v"), b"new(k)")
                    .is_none()
            );
        }
        let protocol = NativeNameProtocol::Jim084;
        assert_eq!(
            protocol.variable_root_tail_extent(NativeVariableInputForm::Combined(b"relative::v")),
            Some(0..11)
        );
        assert_eq!(
            protocol
                .variable_root_tail_extent(NativeVariableInputForm::Combined(b"::N::v\0tail(k)")),
            Some(5..14)
        );
        assert_eq!(
            protocol.replace_variable_root_tail(
                NativeVariableInputForm::Combined(b"relative::v"),
                b"new"
            ),
            Some(b"new".to_vec())
        );
        assert_eq!(
            protocol.replace_variable_root_tail(
                NativeVariableInputForm::Combined(b"::N::v\0tail(k)"),
                b"new"
            ),
            Some(b"::N::new".to_vec())
        );
    }

    #[test]
    fn runtime_root_geometry_preserves_counted_units_and_selected_qualification() {
        let current = ByteNamespacePath::from_segments([b"N".as_slice()]);
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let protocol = NativeNameProtocol::C(version);
            assert_eq!(
                protocol.variable_root_geometry(NativeNameContext::new(&current), b"v\0tail::x"),
                NativeVariableRootGeometry::Local(
                    if version == TclVersion::V8_4 {
                        b"v".as_slice()
                    } else {
                        b"v\0tail::x".as_slice()
                    }
                    .into()
                )
            );
            assert_eq!(
                protocol.variable_root_geometry(NativeNameContext::new(&current), b"::N::v\xff"),
                NativeVariableRootGeometry::CNamespace {
                    namespace: current.clone(),
                    simple: b"v\xff".into()
                }
            );
        }
        assert_eq!(
            NativeNameProtocol::Jim084
                .variable_root_geometry(NativeNameContext::new(&current), b"N::v\0tail"),
            NativeVariableRootGeometry::Local(b"N::v\0tail".into())
        );
        assert_eq!(
            NativeNameProtocol::Jim084
                .variable_root_geometry(NativeNameContext::new(&current), b"::::v\0tail"),
            NativeVariableRootGeometry::JimAbsolute(b"v\0tail".into())
        );
    }
}
