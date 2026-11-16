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

//! Namespace names: addresses and lambda namespaces, Jim procedure namespaces, the parent, tail and qualifier text, and import, export and command patterns.

use super::super::{is_qualified, qualifier_segments, written_command_tail};
use super::{
    NameProjectionUnavailable, NativeJimNamespaceParentResult, NativeNameContext,
    NativeNameProjection, NativeNameProtocol, NativeNamePurpose, NativeNameQualification,
    NativeNamespacePatternParts, NativeNamespacePatternSource, NativeNamespaceTextResult,
    jim_namespace_qualifier, join_jim, projection, routing, strip_jim_root,
};
use std::borrow::Cow;
use tcl_core_types::{ByteNamespacePath, NameBytes, c_string_extent};
use tcl_dialect::TclVersion;

impl NativeNameProtocol {
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
        if self.is_jim084() && !tail && jim_namespace_qualifier(original).is_none() {
            return NativeNamespaceTextResult::SharedEmpty;
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
        super::super::written_separator_extent(prefix)
            .map_or(&prefix[..0], |separator| &prefix[..separator.start])
    }

    /// Test the sole leading export control after selecting its native input extent.
    /// This supplies no operand visit order, clear effect or namespace owner.
    ///
    /// # Errors
    /// Jim export is a no-op and has no C export control declaration.
    pub fn namespace_export_clears(
        self,
        original: &[u8],
    ) -> Result<bool, NameProjectionUnavailable> {
        self.namespace_pattern_input(original, NativeNamePurpose::NamespaceExportPattern)
            .map(|input| input.selected() == b"-clear")
    }

    /// Select C's sole leading import control before per-pattern effects.
    /// The first original operand uses the same C-string extent as import.
    ///
    /// # Errors
    /// Jim's original helper has no C `-force` control declaration.
    pub fn namespace_import_forces(
        self,
        original: &[u8],
    ) -> Result<bool, NameProjectionUnavailable> {
        if self.is_jim084() {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        self.namespace_pattern_input(original, NativeNamePurpose::NamespaceImportPattern)
            .map(|input| input.selected() == b"-force")
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
}
