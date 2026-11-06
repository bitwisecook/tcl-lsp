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

//! Nameless lifecycle source declarations, separate from named method routes.

use super::{MemberSide, MethodDef};
use crate::command_binding::CommandAllocationSite;
use tcl_lexer::NativeWord;
use tcl_registry::definer::DefinitionSpecialMemberKind;

/// An independently selected constructor/destructor declaration. Whole source
/// words and their site supply source navigation, without an editable method
/// name, executed body, native installation or closed own-member absence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceSpecialMemberMetadata {
    site: CommandAllocationSite,
    keyword: NativeWord,
    parameters: Option<NativeWord>,
    body: NativeWord,
    side: MemberSide,
    kind: DefinitionSpecialMemberKind,
    metadata: MethodDef,
}

impl OriginalSourceSpecialMemberMetadata {
    pub(crate) fn new(
        allocation_site: CommandAllocationSite,
        keyword: NativeWord,
        parameters: Option<NativeWord>,
        body: NativeWord,
        side: MemberSide,
        kind: DefinitionSpecialMemberKind,
        metadata: MethodDef,
    ) -> Option<Self> {
        if keyword.image() != allocation_site.source.source_image()
            || allocation_site.offset != keyword.group().span.start()
            || metadata.body_span != body.span()
            || metadata.name_span != keyword.span()
            || parameters.iter().chain(std::iter::once(&body)).any(|word| {
                word.image() != keyword.image()
                    || word.config() != keyword.config()
                    || word.group().expand
            })
        {
            return None;
        }
        Some(Self {
            site: allocation_site,
            keyword,
            parameters,
            body,
            side,
            kind,
            metadata,
        })
    }
    /// Actual original worker command, independent of enclosing class site.
    #[must_use]
    pub const fn site(&self) -> &CommandAllocationSite {
        &self.site
    }
    /// Original nameless lifecycle keyword; no name producer is fabricated.
    #[must_use]
    pub const fn keyword_word(&self) -> &NativeWord {
        &self.keyword
    }
    /// Whole original formal-list argument, when the selected role has one.
    /// Retaining it supplies no parameter binding or activation receipt.
    #[must_use]
    pub const fn parameters_word(&self) -> Option<&NativeWord> {
        self.parameters.as_ref()
    }
    /// Whole original body argument, without a script execution projection.
    #[must_use]
    pub const fn body_word(&self) -> &NativeWord {
        &self.body
    }
    /// Selected definition receiver axis; source metadata only.
    #[must_use]
    pub const fn side(&self) -> MemberSide {
        self.side
    }
    /// Independently selected Registry lifecycle declaration role.
    #[must_use]
    pub const fn kind(&self) -> DefinitionSpecialMemberKind {
        self.kind
    }
    /// Signature/body reporting metadata; labels do not supply identity.
    #[must_use]
    pub const fn metadata(&self) -> &MethodDef {
        &self.metadata
    }
}

/// Canonical declarations observed in source walk order. This inventory has
/// no dispatch/absence query: even an empty body stays an original declaration.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OriginalSourceSpecialMemberLedger {
    declarations: Vec<OriginalSourceSpecialMemberMetadata>,
}

impl OriginalSourceSpecialMemberLedger {
    pub(crate) fn declare(&mut self, declaration: OriginalSourceSpecialMemberMetadata) {
        self.declarations.push(declaration);
    }
    pub(crate) fn absorb(&mut self, other: &Self, other_ran_second: bool) {
        let mut declarations = if other_ran_second {
            std::mem::take(&mut self.declarations)
        } else {
            other.declarations.clone()
        };
        let tail = if other_ran_second {
            &other.declarations
        } else {
            &self.declarations
        };
        for declaration in tail {
            if !declarations.contains(declaration) {
                declarations.push(declaration.clone());
            }
        }
        self.declarations = declarations;
    }
    /// Every canonical declaration, including a later replacement/removal.
    pub fn declarations(&self) -> impl Iterator<Item = &OriginalSourceSpecialMemberMetadata> {
        self.declarations.iter()
    }
}
