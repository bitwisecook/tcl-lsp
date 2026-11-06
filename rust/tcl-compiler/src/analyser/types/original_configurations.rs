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

//! Independently targeted original class-configuration metadata.

use super::{
    OriginalSourceClassRelationLedger, OriginalSourceMemberLedger, OriginalSourcePropertyLedger,
    OriginalSourceSpecialMemberLedger,
};
use crate::command_binding::OriginalSourceClassConfigurationTarget;

/// Own member and relation delta at one original configuration. Its target
/// must independently join a class declaration; a report `QName` supplies no join.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceClassConfiguration {
    target: OriginalSourceClassConfigurationTarget,
    members: OriginalSourceMemberLedger,
    relations: OriginalSourceClassRelationLedger,
    properties: OriginalSourcePropertyLedger,
    special_members: OriginalSourceSpecialMemberLedger,
}

impl OriginalSourceClassConfiguration {
    pub(crate) fn new(
        target: OriginalSourceClassConfigurationTarget,
        members: OriginalSourceMemberLedger,
        relations: OriginalSourceClassRelationLedger,
    ) -> Self {
        Self {
            target,
            members,
            relations,
            properties: OriginalSourcePropertyLedger::default(),
            special_members: OriginalSourceSpecialMemberLedger::default(),
        }
    }
    pub(crate) fn with_properties(mut self, properties: OriginalSourcePropertyLedger) -> Self {
        self.properties = properties;
        self
    }
    pub(crate) fn with_special_members(
        mut self,
        members: OriginalSourceSpecialMemberLedger,
    ) -> Self {
        self.special_members = members;
        self
    }
    /// Canonical constructor/destructor source declarations in this delta.
    /// An empty inventory supplies no lifecycle absence or dispatch proof.
    #[must_use]
    pub const fn special_members(&self) -> &OriginalSourceSpecialMemberLedger {
        &self.special_members
    }
    /// Own original property declarations; target/provider selection is separate.
    #[must_use]
    pub const fn properties(&self) -> &OriginalSourcePropertyLedger {
        &self.properties
    }
    /// Exact target operand, lookup point and separately retained local allocation.
    #[must_use]
    pub const fn target(&self) -> &OriginalSourceClassConfigurationTarget {
        &self.target
    }
    /// This configuration's own counted member declarations and effects.
    #[must_use]
    pub const fn members(&self) -> &OriginalSourceMemberLedger {
        &self.members
    }
    /// This configuration's own ordered relation effects, with no initial class grant.
    #[must_use]
    pub const fn relations(&self) -> &OriginalSourceClassRelationLedger {
        &self.relations
    }
}

/// Own-object delta, keyed by an independently reached bounded allocation.
/// Its class's class-object table and any reporting object `QName` stay separate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceObjectConfiguration {
    target: crate::command_binding::OriginalSourceObjectConfigurationTarget,
    members: OriginalSourceMemberLedger,
    relations: OriginalSourceClassRelationLedger,
    properties: OriginalSourcePropertyLedger,
    special_members: OriginalSourceSpecialMemberLedger,
}

impl OriginalSourceObjectConfiguration {
    pub(crate) fn new(
        target: crate::command_binding::OriginalSourceObjectConfigurationTarget,
        members: OriginalSourceMemberLedger,
        relations: OriginalSourceClassRelationLedger,
    ) -> Self {
        Self {
            target,
            members,
            relations,
            properties: OriginalSourcePropertyLedger::default(),
            special_members: OriginalSourceSpecialMemberLedger::default(),
        }
    }
    pub(crate) fn with_properties(mut self, properties: OriginalSourcePropertyLedger) -> Self {
        self.properties = properties;
        self
    }
    pub(crate) fn with_special_members(
        mut self,
        members: OriginalSourceSpecialMemberLedger,
    ) -> Self {
        self.special_members = members;
        self
    }
    /// Canonical constructor/destructor source declarations in this delta.
    /// An empty inventory supplies no lifecycle absence or dispatch proof.
    #[must_use]
    pub const fn special_members(&self) -> &OriginalSourceSpecialMemberLedger {
        &self.special_members
    }
    /// Own original property declarations; target/provider selection is separate.
    #[must_use]
    pub const fn properties(&self) -> &OriginalSourcePropertyLedger {
        &self.properties
    }
    /// Actual object operand and bounded allocation at the configuration point.
    #[must_use]
    pub const fn target(&self) -> &crate::command_binding::OriginalSourceObjectConfigurationTarget {
        &self.target
    }
    /// This object's own original declarations and effects only.
    #[must_use]
    pub const fn members(&self) -> &OriginalSourceMemberLedger {
        &self.members
    }
    /// This object's own ordered mixin effects, without an instance MRO grant.
    #[must_use]
    pub const fn relations(&self) -> &OriginalSourceClassRelationLedger {
        &self.relations
    }
}
