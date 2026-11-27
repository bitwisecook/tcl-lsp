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

//! Span extents of the root origin's declaration layouts.
//!
//! A reference query walks every root-origin layout site and keeps only the
//! sites one of whose words contains the queried span. This index answers
//! that containment first, so a query visits the enclosing sites only. It is
//! derived from one undetached inventory and never outlives it.

use super::{SourceCommandBindings, declaration_layout};
use std::sync::Arc;
use tcl_names::command::CommandAllocationSite;
use tcl_names::origin::SourceOriginId;

type LayoutRows = Vec<declaration_layout::DeclarationLayoutObservation>;

struct RootLayoutExtent {
    start: u32,
    end: u32,
    /// The furthest end of this and every extent that starts no later.
    end_through: u32,
    site: usize,
}

pub(super) struct RootLayoutExtentIndex {
    /// Root-origin sites in inventory order.
    sites: Vec<CommandAllocationSite>,
    /// One extent per site that has a word, ordered by `start`.
    extents: Vec<RootLayoutExtent>,
    /// Sites by the span of a word's sole variable substitution.
    sole_variables: rustc_hash::FxHashMap<tcl_lexer::Span, Vec<usize>>,
}

pub(super) type RootLayoutExtentCache = super::shared_inventory::InventoryIndexCache<
    declaration_layout::DeclarationLayouts,
    RootLayoutExtentIndex,
>;

impl RootLayoutExtentIndex {
    fn build(
        layouts: &declaration_layout::DeclarationLayouts,
        origin: &Arc<SourceOriginId>,
    ) -> Self {
        let mut sites = Vec::new();
        let mut extents = Vec::new();
        let mut sole_variables = rustc_hash::FxHashMap::<tcl_lexer::Span, Vec<usize>>::default();
        for (site, rows) in layouts {
            if &site.source != origin {
                continue;
            }
            let index = sites.len();
            sites.push(site.clone());
            let mut extent: Option<(u32, u32)> = None;
            for word in rows.iter().flat_map(|row| row.words.iter()) {
                let span = word.source().span;
                extent = Some(extent.map_or((span.start(), span.end()), |(start, end)| {
                    (start.min(span.start()), end.max(span.end()))
                }));
                if let Some((_, read)) = word.sole_variable_substitution() {
                    let indexed = sole_variables.entry(read.span).or_default();
                    if indexed.last() != Some(&index) {
                        indexed.push(index);
                    }
                }
            }
            if let Some((start, end)) = extent {
                extents.push(RootLayoutExtent {
                    start,
                    end,
                    end_through: end,
                    site: index,
                });
            }
        }
        extents.sort_by_key(|extent| extent.start);
        let mut through = 0;
        for extent in &mut extents {
            through = extent.end.max(through);
            extent.end_through = through;
        }
        Self {
            sites,
            extents,
            sole_variables,
        }
    }
}

impl SourceCommandBindings {
    /// The index, or `None` when a query must scan every layout site.
    fn root_layout_extent_index(&self) -> Option<Arc<RootLayoutExtentIndex>> {
        let origin = self.root_origin.as_ref()?;
        self.derived_indexes.get().root_layout_extents.get(
            &self.declaration_layouts,
            origin,
            |layouts| RootLayoutExtentIndex::build(layouts, origin),
        )
    }

    fn root_layout_rows(
        &self,
        index: &RootLayoutExtentIndex,
        mut sites: Vec<usize>,
    ) -> Vec<(&CommandAllocationSite, &LayoutRows)> {
        sites.sort_unstable();
        sites.dedup();
        sites
            .into_iter()
            .filter_map(|site| self.declaration_layouts.get_key_value(&index.sites[site]))
            .collect()
    }

    /// Layout sites in inventory order, including every root-origin site with
    /// one word that starts at or before `start` and ends at or after `end`.
    /// Other sites may be included; the caller applies its own exact tests.
    pub(super) fn root_layouts_spanning(
        &self,
        start: u32,
        end: u32,
    ) -> Vec<(&CommandAllocationSite, &LayoutRows)> {
        let Some(index) = self.root_layout_extent_index() else {
            return self.declaration_layouts.iter().collect();
        };
        let upper = index
            .extents
            .partition_point(|extent| extent.start <= start);
        let sites = index.extents[..upper]
            .iter()
            .rev()
            .take_while(|extent| extent.end_through >= end)
            .filter(|extent| extent.end >= end)
            .map(|extent| extent.site)
            .collect();
        self.root_layout_rows(&index, sites)
    }

    /// Layout sites in inventory order, including every root-origin site with
    /// a word whose sole variable substitution is written at `span`. Other
    /// sites may be included; the caller applies its own exact tests.
    pub(super) fn root_layouts_with_sole_variable_at(
        &self,
        span: tcl_lexer::Span,
    ) -> Vec<(&CommandAllocationSite, &LayoutRows)> {
        let Some(index) = self.root_layout_extent_index() else {
            return self.declaration_layouts.iter().collect();
        };
        let sites = index.sole_variables.get(&span).cloned().unwrap_or_default();
        self.root_layout_rows(&index, sites)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bindings(source: &str) -> SourceCommandBindings {
        let registry = tcl_registry::CommandRegistry::build_default();
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            crate::command_binding::SourceAnalysisOptions {
                invocation_dialect: Some(tcl_native_model::InvocationDialect::of_profile(profile)),
                ..crate::command_binding::SourceAnalysisOptions::default()
            },
        )
    }

    /// Dialect-invariant; run under Tcl 9.0. Every site a full scan would
    /// accept is offered, in inventory order, for every span of the source.
    #[test]
    fn the_index_offers_every_containing_site_in_inventory_order() {
        let source = "proc p {a b} {\n    set v [expr {$a + $b}]\n    \
            if {$v > 1} {\n        set v [string length $a$b]\n    }\n    return $v\n}\n\
            proc q {c} {\n    set w $c\n    return $w\n}\n";
        let bindings = bindings(source);
        let origin = bindings.root_origin.clone().expect("root origin");
        let length = u32::try_from(source.len()).unwrap();
        // The first query of an inventory scans; later ones read the index.
        assert_eq!(
            bindings.root_layouts_spanning(0, 0).len(),
            bindings.declaration_layouts.len()
        );
        let mut contained = 0;
        for start in 0..=length {
            for end in start..=length {
                let offered = bindings.root_layouts_spanning(start, end);
                let order = offered.iter().map(|(site, _)| *site).collect::<Vec<_>>();
                assert!(order.is_sorted(), "{start}..{end}");
                for (site, rows) in &bindings.declaration_layouts {
                    let contains = site.source == origin
                        && rows.iter().flat_map(|row| row.words.iter()).any(|word| {
                            let span = word.source().span;
                            span.start() <= start && end <= span.end()
                        });
                    if contains {
                        contained += 1;
                        assert!(order.contains(&site), "{start}..{end} at {}", site.offset);
                    }
                }
            }
        }
        assert!(contained > 0, "the fixture must retain declaration layouts");
    }

    /// Dialect-invariant; run under Tcl 9.0.
    #[test]
    fn the_index_offers_every_site_of_a_sole_variable_word() {
        let source = "proc p {a b} {\n    set v $a\n    set w \"$b\"\n    return $v\n}\n";
        let bindings = bindings(source);
        let origin = bindings.root_origin.clone().expect("root origin");
        let _ = bindings.root_layouts_spanning(0, 0);
        let mut reads = 0;
        for (site, rows) in &bindings.declaration_layouts {
            if site.source != origin {
                continue;
            }
            for word in rows.iter().flat_map(|row| row.words.iter()) {
                let Some((_, read)) = word.sole_variable_substitution() else {
                    continue;
                };
                reads += 1;
                let offered = bindings.root_layouts_with_sole_variable_at(read.span);
                assert!(offered.len() < bindings.declaration_layouts.len());
                assert!(offered.iter().any(|(offered, _)| *offered == site));
            }
        }
        assert!(reads > 0, "the fixture must retain sole variable words");
    }
}
