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

//! **`PackSurfaceRoster` → [`InheritedSurface`]** — the Q6 rider on the
//! `dialect`-block conversion beside it ([`crate::dialect_conversion`]).
//!
//! Several `include from … into …` rows for one `(target, source)` pair
//! are one roster: that is how a roster with per-release windows is
//! written without repeating the pair on every line. Merging them here —
//! rather than letting each row register separately — is what makes the
//! model's "last roster for a pair wins" rule a *replacement* rule
//! between packs instead of an accident of row order inside one.

use std::collections::BTreeMap;
use std::sync::Arc;

use tcl_dialect::model::{
    Family, InheritedSurface, Provenance, VersionAxisId, VersionSet,
};

use crate::loader::{PackSurfaceRoster, family_named};

/// Fold every roster row in `rows` into one [`InheritedSurface`] per
/// `(target, source)` pair, at `provenance`.
///
/// A row naming a family this build does not have is dropped — the loader
/// already refused it once, so reaching here means a caller built the row
/// itself. A later row restating a name replaces its window, which is the
/// same last-wins rule the pair itself follows.
#[must_use]
pub fn to_inherited_surfaces(
    rows: &[PackSurfaceRoster],
    provenance: Provenance,
) -> Vec<InheritedSurface> {
    let mut merged: BTreeMap<(Family, Family), BTreeMap<Arc<str>, VersionSet>> = BTreeMap::new();
    for row in rows {
        let (Some(target), Some(source)) = (family_named(&row.target), family_named(&row.source))
        else {
            continue;
        };
        let axis = VersionAxisId::core(target);
        let names = merged.entry((target, source)).or_default();
        for entry in &row.names {
            let window = if entry.window.is_empty() {
                VersionSet::from_requirements(axis.clone(), &["0-"])
            } else {
                VersionSet::from_requirements(axis.clone(), &entry.window)
            };
            let Ok(window) = window else {
                continue;
            };
            names.insert(Arc::from(entry.name.as_str()), window);
        }
    }
    merged
        .into_iter()
        .map(|((target, source), names)| InheritedSurface {
            target,
            source,
            names,
            provenance,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use tcl_dialect::model::{Family};
    use super::*;
    use crate::loader::{PackRosterName, PackSurfaceRoster};

    fn row(names: &[(&str, &[&str])]) -> PackSurfaceRoster {
        PackSurfaceRoster {
            source: "tcl".to_owned(),
            target: "jim".to_owned(),
            names: names
                .iter()
                .map(|(name, window)| PackRosterName {
                    name: (*name).to_owned(),
                    window: window.iter().map(|w| (*w).to_owned()).collect(),
                })
                .collect(),
            line: 1,
        }
    }

    #[test]
    fn rows_for_one_pair_merge_into_one_roster() {
        let surfaces = to_inherited_surfaces(
            &[row(&[("set", &[]), ("proc", &[])]), row(&[("interp", &["0.77-"])])],
            Provenance::BuiltIn,
        );
        let [surface] = surfaces.as_slice() else {
            panic!("one pair, one roster: {surfaces:?}");
        };
        assert_eq!(surface.target, Family::Jim);
        assert_eq!(surface.source, Family::Tcl);
        assert_eq!(surface.names.len(), 3);
        assert!(surface.admits("set", None));
        assert!(surface.admits("interp", None));
        assert!(!surface.admits("coroutine", None));
    }

    #[test]
    fn a_row_naming_an_unknown_family_is_dropped() {
        let mut unknown = row(&[("set", &[])]);
        unknown.target = "nonesuch".to_owned();
        assert!(to_inherited_surfaces(&[unknown], Provenance::BuiltIn).is_empty());
    }
}
