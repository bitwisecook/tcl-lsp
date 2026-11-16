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

//! Session storage of issued declaration receipts, the wire handle that locates one and host publication stamps.

use super::{OriginalDeclarationDocument, OriginalDeclarationIdentity};
use crate::wire_session::EditorWireSession;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use tcl_compiler::analyser::AnalysisResult;

/// Compact editor data locating a sealed readonly receipt. Decoding the token
/// supplies no declaration or native authority until the registry reselects it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OriginalDeclarationHandle {
    pub(super) schema: u8,
    pub(super) token: u64,
    // Lossless JSON bytes identify this registry allocation only. Legacy
    // Native/TYPE wire objects retain exactly their existing two fields.
    pub(super) session: Option<EditorWireSession>,
}

impl Serialize for OriginalDeclarationHandle {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Wire {
            schema: u8,
            token: u64,
            #[serde(skip_serializing_if = "Option::is_none")]
            session: Option<EditorWireSession>,
            #[serde(rename = "wirePurpose", skip_serializing_if = "Option::is_none")]
            purpose: Option<&'static str>,
        }
        Wire {
            schema: self.schema,
            token: self.token,
            session: self.session,
            purpose: self.session.map(|_| {
                if self.schema == 5 {
                    "originalSourceViewRelationship"
                } else {
                    "logicalSourceRelationship"
                }
            }),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for OriginalDeclarationHandle {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Legacy {
            schema: u8,
            token: u64,
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Session {
            schema: u8,
            token: u64,
            #[serde(rename = "session")]
            allocation: EditorWireSession,
            #[serde(rename = "wirePurpose")]
            purpose: RelationshipPurpose,
        }
        #[derive(Deserialize)]
        enum RelationshipPurpose {
            #[serde(rename = "logicalSourceRelationship")]
            LogicalSourceRelationship,
            #[serde(rename = "originalSourceViewRelationship")]
            OriginalSourceViewRelationship,
        }
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Session(Session),
            Legacy(Legacy),
        }
        match Wire::deserialize(deserializer)? {
            Wire::Legacy(wire) => Ok(Self {
                schema: wire.schema,
                token: wire.token,
                session: None,
            }),
            // The explicit transport variant is independent of the claimed
            // semantic schema. Wrong schemas decode as inert locators; every
            // actual resolver still enforces its own kind and retained owner.
            Wire::Session(wire) => {
                let purpose_matches = match wire.purpose {
                    RelationshipPurpose::OriginalSourceViewRelationship => wire.schema == 5,
                    RelationshipPurpose::LogicalSourceRelationship => wire.schema != 5,
                };
                if !purpose_matches {
                    return Err(serde::de::Error::custom(
                        "source relationship wire purpose mismatch",
                    ));
                }
                Ok(Self {
                    schema: wire.schema,
                    token: wire.token,
                    session: Some(wire.allocation),
                })
            }
        }
    }
}

/// Session-only host publication payload. This stores and compares an actual
/// host-owned typed stamp; it cannot issue semantic/source authority and is
/// never serialised into editor data. Different host stamp types never match.
#[derive(Debug, Clone)]
pub struct LogicalCallHostPublication(std::sync::Arc<dyn HostPublicationValue>);

pub(super) trait HostPublicationValue: std::fmt::Debug + Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;
    fn same_value(&self, other: &dyn HostPublicationValue) -> bool;
}

impl<T: std::fmt::Debug + Eq + Send + Sync + 'static> HostPublicationValue for T {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn same_value(&self, other: &dyn HostPublicationValue) -> bool {
        other
            .as_any()
            .downcast_ref::<T>()
            .is_some_and(|other| self == other)
    }
}

impl LogicalCallHostPublication {
    /// Retain a host-owned stamp independently of the source selection. This
    /// constructor checks no host currency; its actual issuer must do so under
    /// the host's publication lease before issuing or resolving a readonly handle.
    #[must_use]
    pub fn retain<T: std::fmt::Debug + Eq + Send + Sync + 'static>(stamp: T) -> Self {
        Self(std::sync::Arc::new(stamp))
    }
}

impl PartialEq for LogicalCallHostPublication {
    fn eq(&self, other: &Self) -> bool {
        self.0.same_value(other.0.as_ref())
    }
}

impl Eq for LogicalCallHostPublication {}

/// A source view's typed host publication stamp. The host must validate its
/// actual physical parent, path, source and publication leases independently.
/// This equality-only payload grants no semantic or Native authority and is
/// never serialized into editor data.
#[derive(Debug, Clone)]
pub struct OriginalSourceViewPublication(std::sync::Arc<dyn HostPublicationValue>);

impl OriginalSourceViewPublication {
    /// Retain a host-owned stamp without asserting that it is current.
    #[must_use]
    pub fn retain<T: std::fmt::Debug + Eq + Send + Sync + 'static>(stamp: T) -> Self {
        Self(std::sync::Arc::new(stamp))
    }
}

impl PartialEq for OriginalSourceViewPublication {
    fn eq(&self, other: &Self) -> bool {
        self.0.same_value(other.0.as_ref())
    }
}

impl Eq for OriginalSourceViewPublication {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RetainedDeclaration {
    Original(OriginalDeclarationIdentity),
    OriginalInSourceView {
        identity: OriginalDeclarationIdentity,
        publication: OriginalSourceViewPublication,
    },
    LogicalClass(Box<crate::type_hierarchy::LogicalHierarchyClass>),
    LogicalCall {
        identity: crate::call_hierarchy::LogicalCallIdentity,
        publication: LogicalCallHostPublication,
    },
    LogicalLens {
        identity: crate::code_lens::LogicalLensIdentity,
        publication: LogicalCallHostPublication,
    },
}

/// Bounded session-owned declaration receipt storage. Tokens never alias an
/// evicted record and full source equality is checked on every resolution.
#[derive(Debug)]
pub struct OriginalDeclarationRegistry {
    pub(super) next: u64,
    pub(super) rows: BTreeMap<u64, RetainedDeclaration>,
    pub(super) order: VecDeque<u64>,
    // Real OS/Web Crypto allocation randomness, never source/name authority.
    // Failure withholds the new session-scoped kinds independently of legacy
    // Native/TYPE issuance and resolution.
    pub(super) session: Option<EditorWireSession>,
}

impl Default for OriginalDeclarationRegistry {
    fn default() -> Self {
        Self {
            next: 0,
            rows: BTreeMap::new(),
            order: VecDeque::new(),
            session: EditorWireSession::allocate(),
        }
    }
}

impl OriginalDeclarationRegistry {
    /// Retain this issued source identity and return its readonly wire locator.
    #[must_use]
    pub fn issue(
        &mut self,
        identity: OriginalDeclarationIdentity,
    ) -> Option<OriginalDeclarationHandle> {
        self.issue_retained(RetainedDeclaration::Original(identity), 1)
    }
    /// Bind a genuine readonly declaration to its independently checked host
    /// source view. Issue only inside the host's current publication lease.
    #[must_use]
    pub fn issue_in_source_view(
        &mut self,
        identity: OriginalDeclarationIdentity,
        publication: OriginalSourceViewPublication,
    ) -> Option<OriginalDeclarationHandle> {
        self.issue_retained(
            RetainedDeclaration::OriginalInSourceView {
                identity,
                publication,
            },
            5,
        )
    }

    /// Read the saved typed host stamp to select its exact live publication.
    /// The returned stamp is inert: the host must revalidate it and call
    /// `resolve_in_source_view` before using a declaration. No URI/text fallback
    /// may renew a missing or retired saved publication.
    #[must_use]
    pub fn source_view_publication<T: 'static>(
        &self,
        handle: OriginalDeclarationHandle,
    ) -> Option<&T> {
        if handle.schema != 5 || !self.matches_logical_session(handle) {
            return None;
        }
        let RetainedDeclaration::OriginalInSourceView { publication, .. } =
            self.rows.get(&handle.token)?
        else {
            return None;
        };
        publication.0.as_any().downcast_ref::<T>()
    }

    /// Reselect this exact current source-view publication and original owner.
    /// A legacy locator cannot cast to this kind; neither can CALL/TYPE/LENS.
    #[must_use]
    pub fn resolve_in_source_view(
        &self,
        handle: OriginalDeclarationHandle,
        uri: &str,
        source: &str,
        analysis: &AnalysisResult,
        current_publication: &OriginalSourceViewPublication,
    ) -> Option<OriginalDeclarationIdentity> {
        if handle.schema != 5 || !self.matches_logical_session(handle) {
            return None;
        }
        let RetainedDeclaration::OriginalInSourceView {
            identity,
            publication,
        } = self.rows.get(&handle.token)?
        else {
            return None;
        };
        (publication == current_publication && identity.is_current(uri, source, analysis))
            .then(|| identity.clone())
    }

    /// Retain a sealed Logical class selection under a separate wire schema.
    #[must_use]
    pub fn issue_logical_class(
        &mut self,
        identity: crate::type_hierarchy::LogicalHierarchyClass,
    ) -> Option<OriginalDeclarationHandle> {
        self.issue_retained(RetainedDeclaration::LogicalClass(Box::new(identity)), 2)
    }
    /// Retain a genuine Logical CALL selection under its own wire kind.
    #[must_use]
    pub fn issue_logical_call(
        &mut self,
        identity: crate::call_hierarchy::LogicalCallIdentity,
        publication: LogicalCallHostPublication,
    ) -> Option<OriginalDeclarationHandle> {
        self.issue_retained(
            RetainedDeclaration::LogicalCall {
                identity,
                publication,
            },
            3,
        )
    }
    /// Retain a genuine Logical source lens under its own wire schema.
    /// The equality-only host payload must be minted inside the actual source,
    /// provider and index publication lease; it supplies no semantic authority.
    #[must_use]
    pub fn issue_logical_lens(
        &mut self,
        identity: crate::code_lens::LogicalLensIdentity,
        publication: LogicalCallHostPublication,
    ) -> Option<OriginalDeclarationHandle> {
        self.issue_retained(
            RetainedDeclaration::LogicalLens {
                identity,
                publication,
            },
            4,
        )
    }
    fn issue_retained(
        &mut self,
        identity: RetainedDeclaration,
        schema: u8,
    ) -> Option<OriginalDeclarationHandle> {
        let session = if matches!(schema, 3..=5) {
            Some(self.session?)
        } else {
            None
        };
        if let Some((token, _)) = self.rows.iter().find(|(_, row)| *row == &identity) {
            return Some(OriginalDeclarationHandle {
                schema,
                token: *token,
                session,
            });
        }
        let token = self.next.checked_add(1)?;
        self.next = token;
        while self.rows.len() >= 4096 {
            self.rows.remove(&self.order.pop_front()?);
        }
        self.rows.insert(token, identity);
        self.order.push_back(token);
        Some(OriginalDeclarationHandle {
            schema,
            token,
            session,
        })
    }
    /// Reselect the authentic current metadata by its retained full receipt.
    #[must_use]
    pub fn resolve(
        &self,
        handle: OriginalDeclarationHandle,
        uri: &str,
        source: &str,
        analysis: &AnalysisResult,
    ) -> Option<OriginalDeclarationIdentity> {
        if handle.schema != 1 || handle.session.is_some() {
            return None;
        }
        let RetainedDeclaration::Original(identity) = self.rows.get(&handle.token)? else {
            return None;
        };
        identity
            .is_current(uri, source, analysis)
            .then(|| identity.clone())
    }
    /// Reselect one current Logical class; Native and forged wire schemas refuse.
    #[must_use]
    pub fn resolve_logical_class(
        &self,
        handle: OriginalDeclarationHandle,
        uri: &str,
        source: &str,
        analysis: &AnalysisResult,
    ) -> Option<crate::type_hierarchy::LogicalHierarchyClass> {
        if handle.schema != 2 || handle.session.is_some() {
            return None;
        }
        let RetainedDeclaration::LogicalClass(identity) = self.rows.get(&handle.token)? else {
            return None;
        };
        identity
            .is_current(uri, source, analysis)
            .then(|| identity.as_ref().clone())
    }
    /// Session allocation is a transport discriminator, independent of source
    /// equality and genuine publication revalidation. Missing entropy refuses
    /// before any row can be selected, including in a fresh registry.
    fn matches_logical_session(&self, handle: OriginalDeclarationHandle) -> bool {
        self.session.is_some() && handle.session == self.session
    }

    /// Reborrow the current source CALL receipt. Native and TYPE tokens cannot
    /// cast to CALL, and client labels/ranges do not participate in selection.
    #[must_use]
    pub fn resolve_logical_call(
        &self,
        handle: OriginalDeclarationHandle,
        uri: &str,
        documents: &[OriginalDeclarationDocument<'_>],
        current_publication: &LogicalCallHostPublication,
    ) -> Option<crate::call_hierarchy::LogicalCallIdentity> {
        if handle.schema != 3 || !self.matches_logical_session(handle) {
            return None;
        }
        let RetainedDeclaration::LogicalCall {
            identity,
            publication,
        } = self.rows.get(&handle.token)?
        else {
            return None;
        };
        (publication == current_publication
            && identity.uri() == uri
            && identity.is_current_in(documents))
        .then(|| identity.clone())
    }
    /// Reselect a current Logical lens without reading client labels/ranges.
    /// Native, TYPE and CALL handles cannot cast to this separate schema.
    #[must_use]
    pub fn resolve_logical_lens(
        &self,
        handle: OriginalDeclarationHandle,
        uri: &str,
        documents: &[OriginalDeclarationDocument<'_>],
        current_publication: &LogicalCallHostPublication,
    ) -> Option<crate::code_lens::LogicalLensIdentity> {
        if handle.schema != 4 || !self.matches_logical_session(handle) {
            return None;
        }
        let RetainedDeclaration::LogicalLens {
            identity,
            publication,
        } = self.rows.get(&handle.token)?
        else {
            return None;
        };
        (publication == current_publication
            && identity.uri() == uri
            && identity.is_current_in(documents))
        .then(|| identity.clone())
    }
}
