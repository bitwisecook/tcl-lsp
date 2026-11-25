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

//! Source property declarations and the accessor methods a family generates for them.

use super::{DefinitionBodyGrammar, MemberKind};
use crate::arg_role::ArgRole;
use tcl_dialect::model::{SurfaceQuery, surface_admits};

/// Source-advisory layout of the configurable property accessor. Selection
/// of an actually entered method, receiver and property table is independent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourcePropertyAccessorMode {
    /// No arguments enumerate readable properties.
    ListReadable,
    /// One argument reads a readable property.
    Read,
    /// Name/value pairs set writable properties.
    WritePairs,
}

/// Candidate property kind for a genuine existing or immediately next
/// argument position. This is completion advice, not a valid invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourcePropertyCompletionRole {
    /// Existing one-name query proposes readable properties.
    Readable,
    /// An existing or prospective write pair proposes writable properties.
    Writable,
    /// A fresh first property can become a read or a write pair.
    ReadOrWrite,
}

/// Property-name argument ordinals under one selected accessor layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourcePropertyAccessorLayout {
    mode: SourcePropertyAccessorMode,
    arguments: usize,
}

impl SourcePropertyAccessorLayout {
    /// Read/list versus write naming axis, independent of execution.
    #[must_use]
    pub const fn mode(self) -> SourcePropertyAccessorMode {
        self.mode
    }
    /// Zero-based ordinals after the selected method word. Values never
    /// become property names solely because their bytes resemble an option.
    pub fn property_ordinals(self) -> impl Iterator<Item = usize> {
        (0..self.arguments).step_by(2)
    }
}

/// Immutable source accessor advice issued by the selected grammar and name
/// recipe. Its role is separate from an installed or actually entered method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourcePropertyAccessorAdvice {
    methods: &'static [&'static str],
    pub(super) recipe: tcl_syntax::naming::NativeNameProtocol,
    configurable: bool,
}

impl SourcePropertyAccessorAdvice {
    /// Authored generated method candidates, without installation authority.
    #[must_use]
    pub const fn methods(self) -> &'static [&'static str] {
        self.methods
    }
    /// Exact argument layout for the retained source accessor protocol.
    #[must_use]
    pub fn layout(self, selector: &[u8], arguments: usize) -> Option<SourcePropertyAccessorLayout> {
        if !self.configurable
            || !self
                .methods
                .iter()
                .any(|name| *name == "configure" && name.as_bytes() == selector)
        {
            return None;
        }
        let mode = match arguments {
            0 => SourcePropertyAccessorMode::ListReadable,
            1 => SourcePropertyAccessorMode::Read,
            count if count % 2 == 0 => SourcePropertyAccessorMode::WritePairs,
            _ => return None,
        };
        Some(SourcePropertyAccessorLayout { mode, arguments })
    }
    /// Source candidate role at a real argument or the immediate trailing
    /// gap. Value positions and skipped future operands supply no name role.
    #[must_use]
    pub fn completion_name_role(
        self,
        selector: &[u8],
        ordinal: usize,
        arguments: usize,
    ) -> Option<SourcePropertyCompletionRole> {
        self.layout(selector, 0)?;
        if ordinal > arguments || !ordinal.is_multiple_of(2) {
            return None;
        }
        Some(if ordinal == 0 && arguments == 0 {
            SourcePropertyCompletionRole::ReadOrWrite
        } else if ordinal == 0 && arguments == 1 {
            SourcePropertyCompletionRole::Readable
        } else {
            SourcePropertyCompletionRole::Writable
        })
    }
    /// Independently selected recipe, without physical engine attestation.
    #[must_use]
    pub const fn recipe(self) -> tcl_syntax::naming::NativeNameProtocol {
        self.recipe
    }
}

/// One property declaration's selected original argument layout. Indices
/// address the complete argument vector after the member keyword; each name
/// retains its own option values. This is grammar advice, not target admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourcePropertyDeclaration {
    pub(super) name: usize,
    getter: Option<usize>,
    setter: Option<usize>,
    pub(super) kind: crate::commands::tcl::TclOoPropertyKind,
}

impl SourcePropertyDeclaration {
    /// Original property-name argument ordinal.
    #[must_use]
    pub const fn name_index(self) -> usize {
        self.name
    }
    /// Original custom getter body ordinal, when supplied.
    #[must_use]
    pub const fn getter_index(self) -> Option<usize> {
        self.getter
    }
    /// Original custom setter body ordinal, when supplied.
    #[must_use]
    pub const fn setter_index(self) -> Option<usize> {
        self.setter
    }
    /// Independently selected kind for this property only.
    #[must_use]
    pub const fn kind(self) -> crate::commands::tcl::TclOoPropertyKind {
        self.kind
    }

    pub(super) fn selected_indices_where(self, admit: &dyn Fn(ArgRole) -> bool) -> Vec<usize> {
        let mut indices = Vec::new();
        if admit(ArgRole::Name) {
            indices.push(self.name);
        }
        if admit(ArgRole::Body) {
            indices.extend(self.getter);
            indices.extend(self.setter);
        }
        indices.sort_unstable();
        indices.dedup();
        indices
    }
}

impl DefinitionBodyGrammar {
    /// Per-property layout selected from original counted argument values.
    /// Option and kind matching use their C Index string purposes, separately
    /// from the original property token. No body is evaluated by this query.
    #[must_use]
    pub fn source_property_declarations_bytes<S: AsRef<[u8]>>(
        &self,
        args: &[S],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<Vec<SourcePropertyDeclaration>> {
        self.source_property_declarations_where(
            args.len(),
            &|index| args.get(index).map(AsRef::as_ref),
            dialect,
            false,
        )
    }

    pub(super) fn source_property_declarations_where<'w>(
        &self,
        count: usize,
        word_at: &dyn Fn(usize) -> Option<&'w [u8]>,
        dialect: Option<SurfaceQuery<'_>>,
        source_layout: bool,
    ) -> Option<Vec<SourcePropertyDeclaration>> {
        use crate::commands::tcl::{
            TclOoPropertyKind, TclOoPropertyOption, resolve_tcloo_property_kind,
            resolve_tcloo_property_option,
        };
        let dialect = if source_layout { None } else { Some(dialect?) };
        let member = self.member("property")?;
        if !self.profile.has_native_definition_workers()
            || member.kind != MemberKind::FlagKeyed
            || (!source_layout
                && member
                    .surface
                    .is_some_and(|surface| !surface_admits(surface, dialect.as_ref())))
        {
            return None;
        }
        let mut declarations = Vec::new();
        let mut next = 0;
        while next < count {
            let mut declaration = SourcePropertyDeclaration {
                name: next,
                getter: None,
                setter: None,
                kind: TclOoPropertyKind::ReadWrite,
            };
            next += 1;
            while next < count {
                let option = word_at(next)?;
                if option.first() != Some(&b'-') {
                    break;
                }
                let option = resolve_tcloo_property_option(option).ok()?;
                if next + 1 >= count {
                    return None;
                }
                match option {
                    TclOoPropertyOption::Get => declaration.getter = Some(next + 1),
                    TclOoPropertyOption::Set => declaration.setter = Some(next + 1),
                    TclOoPropertyOption::Kind => {
                        declaration.kind = resolve_tcloo_property_kind(word_at(next + 1)?).ok()?;
                    }
                }
                next += 2;
            }
            declarations.push(declaration);
        }
        Some(declarations)
    }

    /// Retain authored accessor candidates and the independently selected
    /// configurable accessor protocol as immutable structural source advice.
    #[must_use]
    pub fn source_property_accessor_advice(
        &self,
        recipe: tcl_syntax::naming::NativeNameProtocol,
    ) -> SourcePropertyAccessorAdvice {
        SourcePropertyAccessorAdvice {
            methods: self.property_accessor_methods,
            recipe,
            configurable: self.profile.has_native_definition_workers()
                && recipe.oo_property_option_name(b"").is_ok(),
        }
    }

    /// Source-only argument layout for a property accessor declared by this
    /// selected grammar, without installed or entered method authority.
    #[must_use]
    pub fn source_property_accessor_layout(
        &self,
        selector: &[u8],
        recipe: tcl_syntax::naming::NativeNameProtocol,
        arguments: usize,
    ) -> Option<SourcePropertyAccessorLayout> {
        self.source_property_accessor_advice(recipe)
            .layout(selector, arguments)
    }
}
