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

//! Jim's namespace canonicalisation: canonical inputs and reports, command-query candidates and namespace and command-key construction.

use super::{
    NameProjectionUnavailable, NativeJimNamespaceCommandQueryCandidate,
    NativeJimNamespaceConstruction, NativeNameContext, NativeNameProjection, NativeNameProtocol,
    NativeNamePurpose, projection, strip_jim_root,
};
use std::borrow::Cow;
use tcl_core_types::{NameBytes, c_string_extent};

impl NativeNameProtocol {
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

    /// Rooted variable-name report from the same selected Jim canonical input.
    /// The counted canonical bytes remain data; this grants no cell, lookup,
    /// original result header, namespace holder or successful operation.
    ///
    /// # Errors
    /// Refuses another engine, protocol or naming purpose.
    pub fn jim_namespace_canonical_report_bytes(
        self,
        canonical: &NativeNameProjection<'_>,
    ) -> Result<Vec<u8>, NameProjectionUnavailable> {
        if !self.is_jim084()
            || canonical.protocol() != self
            || canonical.purpose() != NativeNamePurpose::JimNamespaceCanonical
        {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        let mut report = b"::".to_vec();
        report.extend_from_slice(canonical.selected());
        Ok(report)
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
    /// `CString` API constructs a fresh object. Procedure and rename qualification
    /// duplicate the genuine namespace object before appending `CString` units.
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
}
