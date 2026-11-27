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

//! The recorded point that covers a source offset.
//!
//! A position query picks, among the points whose extent contains the
//! offset, the one that starts last (the later-recorded of two that start
//! together). This index answers that without reading every point.

use super::{SourceBindingPoint, SourceCommandBindings};

pub(super) struct PointExtentIndex {
    /// `(offset, end, position)` ordered by offset and then position.
    extents: Vec<(u32, u32, usize)>,
    /// The furthest end of each extent and every one ordered before it.
    end_through: Vec<u32>,
}

pub(super) type PointExtentCache =
    super::shared_inventory::InventoryIndexCache<Vec<SourceBindingPoint>, PointExtentIndex>;

impl PointExtentIndex {
    fn build(points: &[SourceBindingPoint]) -> Self {
        let mut extents = points
            .iter()
            .enumerate()
            .map(|(position, point)| (point.offset, point.end, position))
            .collect::<Vec<_>>();
        extents.sort_unstable_by_key(|(offset, _, position)| (*offset, *position));
        let mut through = 0;
        let end_through = extents
            .iter()
            .map(|(_, end, _)| {
                through = through.max(*end);
                through
            })
            .collect();
        Self {
            extents,
            end_through,
        }
    }

    fn covering(&self, offset: u32) -> Option<usize> {
        let upper = self
            .extents
            .partition_point(|(start, _, _)| *start <= offset);
        (0..upper)
            .rev()
            .take_while(|at| self.end_through[*at] >= offset)
            .find(|at| self.extents[*at].1 >= offset)
            .map(|at| self.extents[at].2)
    }
}

impl SourceCommandBindings {
    fn scanned_point_covering(&self, offset: u32) -> Option<&SourceBindingPoint> {
        self.points
            .iter()
            .filter(|point| point.offset <= offset && offset <= point.end)
            .max_by_key(|point| point.offset)
    }

    /// The point whose extent contains `offset` and starts last; of several
    /// that start together, the last recorded. Exactly the scan's choice.
    pub(super) fn point_covering(&self, offset: u32) -> Option<&SourceBindingPoint> {
        let Some(origin) = self.root_origin.as_ref() else {
            return self.scanned_point_covering(offset);
        };
        let Some(index) =
            self.derived_indexes
                .get()
                .point_extents
                .get(&self.points, origin, |points| {
                    PointExtentIndex::build(points)
                })
        else {
            return self.scanned_point_covering(offset);
        };
        index
            .covering(offset)
            .and_then(|position| self.points.get(position))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Dialect-invariant; run under Tcl 9.0.
    #[test]
    fn the_index_picks_the_point_the_scan_picks() {
        let source = "proc p {a b} {\n    set v [expr {$a + $b}]\n    \
            if {$v > 1} { set v [string length $a$b] }\n    return $v\n}\n\
            set x 1\nset y [p $x [string trim $x]]\nputs \"$x [p $y $x]\"\n";
        let registry = tcl_registry::CommandRegistry::build_default();
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            crate::command_binding::SourceAnalysisOptions {
                invocation_dialect: Some(tcl_native_model::InvocationDialect::of_profile(profile)),
                ..crate::command_binding::SourceAnalysisOptions::default()
            },
        );
        assert!(bindings.root_origin.is_some());
        let mut covered = 0;
        for offset in 0..=u32::try_from(source.len()).unwrap() + 1 {
            let scanned = bindings.scanned_point_covering(offset);
            // The second query of an unchanged inventory reads the index.
            let _ = bindings.point_covering(offset);
            let indexed = bindings.point_covering(offset);
            assert_eq!(
                indexed.map(std::ptr::from_ref),
                scanned.map(std::ptr::from_ref),
                "offset {offset}"
            );
            covered += usize::from(scanned.is_some());
        }
        assert!(covered > 0, "the fixture must record points");
    }
}
