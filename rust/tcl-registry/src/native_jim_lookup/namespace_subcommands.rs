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

//! Original pinned namespace subcommand declaration, separate from source schema.

use std::rc::Rc;
use std::sync::{Arc, OnceLock};
use tcl_cmd_core::prefix::{Resolution, scan};
use tcl_core_types::{
    NativeIndexCache, NativeIndexTable, NativeIndexUnavailable, NativeJimSubcommandCache,
};

/// Actual namespace table operation, without a selected invocation or frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeJimNamespaceSubcommandKind {
    /// Original `canonical` table operation.
    Canonical,
    /// Original `code` table operation.
    Code,
    /// Original `current` table operation.
    Current,
    /// Original `delete` table operation.
    Delete,
    /// Original `ensemble` table operation.
    Ensemble,
    /// Original `eval` table operation.
    Eval,
    /// Original `export` table operation.
    Export,
    /// Original `import` table operation.
    Import,
    /// Original `inscope` table operation.
    Inscope,
    /// Original `origin` table operation.
    Origin,
    /// Original `parent` table operation.
    Parent,
    /// Original `qualifiers` table operation.
    Qualifiers,
    /// Original `tail` table operation.
    Tail,
    /// Original `upvar` table operation.
    Upvar,
    /// Original `which` table operation.
    Which,
}

/// One immutable original table declaration. Source-analysis metadata is separate.
#[derive(Debug, Clone, Copy)]
pub struct NativeJimNamespaceSubcommand {
    kind: NativeJimNamespaceSubcommandKind,
    name: &'static str,
    usage: &'static str,
    minimum: usize,
    maximum: Option<usize>,
}

impl NativeJimNamespaceSubcommand {
    /// Actual operation declared by this row.
    #[must_use]
    pub const fn kind(self) -> NativeJimNamespaceSubcommandKind {
        self.kind
    }
    /// Original canonical public selector.
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }
    /// Original public argument synopsis, independently of Proc formals.
    #[must_use]
    pub const fn usage(self) -> &'static str {
        self.usage
    }
    /// Apply the original public table's operand-count check.
    #[must_use]
    pub fn accepts_arguments(self, count: usize) -> bool {
        count >= self.minimum && self.maximum.is_none_or(|maximum| count <= maximum)
    }
    /// Authored helper family selected by this row, without any installed binding.
    #[must_use]
    pub fn scripted_helper_kind(self) -> Option<super::NativeJimNamespaceHelperKind> {
        use NativeJimNamespaceSubcommandKind as Kind;
        Some(match self.kind {
            Kind::Which => super::NativeJimNamespaceHelperKind::Which,
            Kind::Code => super::NativeJimNamespaceHelperKind::Code,
            Kind::Origin => super::NativeJimNamespaceHelperKind::Origin,
            Kind::Inscope => super::NativeJimNamespaceHelperKind::Inscope,
            Kind::Delete => super::NativeJimNamespaceHelperKind::Delete,
            Kind::Upvar => super::NativeJimNamespaceHelperKind::Upvar,
            Kind::Import => super::NativeJimNamespaceHelperKind::Import,
            Kind::Ensemble => super::NativeJimNamespaceHelperKind::Ensemble,
            _ => return None,
        })
    }
}

pub(super) static NAMESPACE_SUBCOMMANDS: &[NativeJimNamespaceSubcommand] = &[
    NativeJimNamespaceSubcommand {
        kind: NativeJimNamespaceSubcommandKind::Canonical,
        name: "canonical",
        usage: "?current? ?name?",
        minimum: 0,
        maximum: Some(2),
    },
    NativeJimNamespaceSubcommand {
        kind: NativeJimNamespaceSubcommandKind::Code,
        name: "code",
        usage: "arg",
        minimum: 1,
        maximum: Some(1),
    },
    NativeJimNamespaceSubcommand {
        kind: NativeJimNamespaceSubcommandKind::Current,
        name: "current",
        usage: "",
        minimum: 0,
        maximum: Some(0),
    },
    NativeJimNamespaceSubcommand {
        kind: NativeJimNamespaceSubcommandKind::Delete,
        name: "delete",
        usage: "?name ...?",
        minimum: 0,
        maximum: None,
    },
    NativeJimNamespaceSubcommand {
        kind: NativeJimNamespaceSubcommandKind::Ensemble,
        name: "ensemble",
        usage: "subcommand ?arg ...?",
        minimum: 1,
        maximum: None,
    },
    NativeJimNamespaceSubcommand {
        kind: NativeJimNamespaceSubcommandKind::Eval,
        name: "eval",
        usage: "name arg ?arg ...?",
        minimum: 2,
        maximum: None,
    },
    NativeJimNamespaceSubcommand {
        kind: NativeJimNamespaceSubcommandKind::Export,
        name: "export",
        usage: "?...?",
        minimum: 0,
        maximum: None,
    },
    NativeJimNamespaceSubcommand {
        kind: NativeJimNamespaceSubcommandKind::Import,
        name: "import",
        usage: "?...?",
        minimum: 0,
        maximum: None,
    },
    NativeJimNamespaceSubcommand {
        kind: NativeJimNamespaceSubcommandKind::Inscope,
        name: "inscope",
        usage: "name arg ?arg ...?",
        minimum: 2,
        maximum: None,
    },
    NativeJimNamespaceSubcommand {
        kind: NativeJimNamespaceSubcommandKind::Origin,
        name: "origin",
        usage: "name",
        minimum: 1,
        maximum: Some(1),
    },
    NativeJimNamespaceSubcommand {
        kind: NativeJimNamespaceSubcommandKind::Parent,
        name: "parent",
        usage: "?name?",
        minimum: 0,
        maximum: Some(1),
    },
    NativeJimNamespaceSubcommand {
        kind: NativeJimNamespaceSubcommandKind::Qualifiers,
        name: "qualifiers",
        usage: "string",
        minimum: 1,
        maximum: Some(1),
    },
    NativeJimNamespaceSubcommand {
        kind: NativeJimNamespaceSubcommandKind::Tail,
        name: "tail",
        usage: "string",
        minimum: 1,
        maximum: Some(1),
    },
    NativeJimNamespaceSubcommand {
        kind: NativeJimNamespaceSubcommandKind::Upvar,
        name: "upvar",
        usage: "ns ?arg ...?",
        minimum: 1,
        maximum: None,
    },
    NativeJimNamespaceSubcommand {
        kind: NativeJimNamespaceSubcommandKind::Which,
        name: "which",
        usage: "?-command|-variable? name",
        minimum: 1,
        maximum: Some(2),
    },
];

/// Selected original supported-backend table, with its actual static lifetime.
#[derive(Debug, Clone, Copy)]
pub struct NativeJimNamespaceSubcommandTable {
    owner: &'static Arc<()>,
}

impl NativeJimNamespaceSubcommandTable {
    /// Original rows in native ordinal order; no mutable source-schema table is borrowed.
    #[must_use]
    pub const fn entries(self) -> &'static [NativeJimNamespaceSubcommand] {
        NAMESPACE_SUBCOMMANDS
    }
    /// Borrow the exact original row at a validated ordinal.
    #[must_use]
    pub fn entry(self, index: usize) -> Option<&'static NativeJimNamespaceSubcommand> {
        self.entries().get(index)
    }
    /// Equality key for this actual immutable declaration, never pointer authority.
    #[must_use]
    pub fn declaration_identity(self) -> usize {
        self.entries().as_ptr() as usize
    }
    /// Consume only this table's retained original cache.
    #[must_use]
    pub fn cached_entry(
        self,
        cache: &NativeJimSubcommandCache,
    ) -> Option<&'static NativeJimNamespaceSubcommand> {
        (cache.table_owner_is(self.owner) && cache.table_identity() == self.declaration_identity())
            .then(|| self.entry(cache.index()))
            .flatten()
    }
    /// Retain the selected entry from this actual table. This supplies no object header.
    #[must_use]
    pub fn cache(self, index: usize) -> Option<NativeJimSubcommandCache> {
        self.entry(index)?;
        Some(NativeJimSubcommandCache::new(
            NativeIndexCache::new(
                Rc::new(self),
                std::mem::size_of::<NativeJimNamespaceSubcommand>(),
                index,
            ),
            Arc::clone(self.owner),
        ))
    }
    /// Original diagnostic-list helper's evaluated command head.
    #[must_use]
    pub const fn diagnostic_sort_head(self) -> &'static [u8] {
        b"lsort"
    }
    /// Counted-prefix progress through the actual reached table row.
    /// The physical parser performs its original immediate comparison first;
    /// an earlier ambiguity must finish before a later comparison is reached.
    #[must_use]
    fn counted_prefix_through(self, original: &[u8], index: usize) -> Option<Resolution> {
        self.entry(index)?;
        let names: Vec<_> = self.entries()[..=index]
            .iter()
            .map(|entry| entry.name)
            .collect();
        let input = tcl_syntax::naming::NativeNameProtocol::Jim084
            .namespace_subcommand_prefix_input(original);
        Some(scan(&names, input.selected(), false))
    }
    /// Resolve exact CString then counted abbreviation through the shared matcher.
    /// Physical cache access and each original getter remain with the caller.
    ///
    /// # Errors
    /// Reports the shared matcher's original ambiguous or missing outcome.
    pub fn lookup(self, original: &[u8]) -> Result<NativeJimSubcommandCache, Resolution> {
        let exact =
            tcl_syntax::naming::NativeNameProtocol::Jim084.namespace_subcommand_input(original);
        match self.lookup_using(original, &mut |index| {
            Ok::<bool, std::convert::Infallible>(
                exact.selected() == self.entries()[index].name.as_bytes(),
            )
        }) {
            Ok(outcome) => outcome,
            Err(never) => match never {},
        }
    }
    /// Select the actual row through reached original immediate comparisons.
    /// Only their misses reach counted-prefix progress; an earlier ambiguity
    /// finishes before any later comparison. This issues no physical primary.
    ///
    /// # Errors
    /// Preserves the comparison owner's refusal separately from a native table miss.
    pub fn lookup_using<E>(
        self,
        original: &[u8],
        compare: &mut impl FnMut(usize) -> Result<bool, E>,
    ) -> Result<Result<NativeJimSubcommandCache, Resolution>, E> {
        let mut progress = Resolution::NoMatch;
        for index in 0..self.entries().len() {
            if compare(index)? {
                return Ok(self.cache(index).ok_or(Resolution::NoMatch));
            }
            progress = self
                .counted_prefix_through(original, index)
                .unwrap_or(Resolution::NoMatch);
            if matches!(progress, Resolution::Ambiguous) {
                return Ok(Err(progress));
            }
        }
        Ok(match progress {
            Resolution::Exact(index) | Resolution::UniquePrefix(index) => {
                self.cache(index).ok_or(Resolution::NoMatch)
            }
            failure => Err(failure),
        })
    }
}

impl crate::native_index_lookup::NativeImmediateLiteralTable for NativeJimNamespaceSubcommandTable {
    fn immediate_literal(&self, index: usize) -> Option<&'static [u8]> {
        self.entry(index).map(|entry| entry.name.as_bytes())
    }
    fn immediate_cache(&self, index: usize) -> Option<NativeIndexCache> {
        self.entry(index)?;
        Some(NativeIndexCache::new(
            Rc::new(*self),
            std::mem::size_of::<NativeJimNamespaceSubcommand>(),
            index,
        ))
    }
}

impl NativeIndexTable for NativeJimNamespaceSubcommandTable {
    fn identity(&self) -> usize {
        self.declaration_identity()
    }
    fn word(&self, index: usize, stride: usize) -> Result<Rc<[u8]>, NativeIndexUnavailable> {
        if stride != std::mem::size_of::<NativeJimNamespaceSubcommand>() {
            return Err(NativeIndexUnavailable);
        }
        self.entry(index)
            .map(|entry| Rc::from(entry.name.as_bytes()))
            .ok_or(NativeIndexUnavailable)
    }
}

impl crate::InvocationDialect {
    /// Select the actual original Jim namespace table; source schemas cannot issue it.
    #[must_use]
    pub fn native_jim_namespace_subcommand_table(
        self,
    ) -> Option<NativeJimNamespaceSubcommandTable> {
        self.native_jim_lookup_protocol()?;
        static OWNER: OnceLock<Arc<()>> = OnceLock::new();
        Some(NativeJimNamespaceSubcommandTable {
            owner: OWNER.get_or_init(|| Arc::new(())),
        })
    }
    /// Original ParseSubCmd special literals, separate from selected table caches.
    #[must_use]
    pub fn native_jim_namespace_parser_comparison_table(
        self,
    ) -> Option<crate::native_index_lookup::NativeStaticIndexTable> {
        self.native_jim_lookup_protocol()?;
        static SPECIAL: &[&str] = &["-help", "-commands"];
        Some(crate::native_index_lookup::NativeStaticIndexTable::supported_backend(SPECIAL))
    }
    /// Original immediate root-literal comparison, separate from namespace lookup.
    #[must_use]
    pub fn native_jim_namespace_parent_root_comparison_table(
        self,
    ) -> Option<crate::native_index_lookup::NativeStaticIndexTable> {
        self.native_jim_lookup_protocol()?;
        static ROOT: &[&str] = &["::"];
        Some(crate::native_index_lookup::NativeStaticIndexTable::supported_backend(ROOT))
    }
}
