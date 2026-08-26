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

//! The spec studio's dialect ingress — its face of the one shared seam,
//! [`tcl_registry::model::ingress`] (centralisation contract R-a; P1-F
//! wave 4, alongside the CLIs and the MCP server).
//!
//! Every dialect **name** the studio threads — the picker's selection, a
//! draft's `SOURCE_DIALECT_KEY`, the fixed `spectcl` the pack formatter
//! uses, the corpus scanner's target — resolves here, once, and the
//! command store it then browses is the resolved environment's registry
//! generation.
//!
//! Nothing here changes what the studio shows. The catalogue names the
//! picker offers map to their same-named environments, whose `unit_profile`
//! is the profile the retired `by_name`/`find` returned and whose
//! generation store is the very `Arc` the old `(profile, overlay)` cache
//! owns.
//!
//! Ledger row T7 (P2) is untouched by this: the studio's `DIALECT_BITS`
//! editor, its dialect-string APIs, `SOURCE_DIALECT_KEY`, and the
//! dialect-as-language-id client are *payload* the row retires, not
//! ingresses this wave ports.

use tcl_dialect::DialectProfile;
use tcl_registry::CommandRegistry;

/// The dialect the picker starts on, and the sink a name with no catalogue
/// profile falls back to.
pub const DEFAULT_DIALECT: &str = "tcl9.0";

/// Resolve a dialect **name** to the profile the studio threads — the
/// environment-model form of `DialectProfile::by_name`.
///
/// P1-G: the profile retires and the studio reads its labels off the
/// environment instead.
#[must_use]
pub fn profile_for_dialect(name: &str) -> &'static DialectProfile {
    tcl_registry::model::resolve_environment(name).unit_profile()
}

/// The **catalogue** name `name` selects, as a `&'static str`, falling back
/// to [`DEFAULT_DIALECT`] when it names no catalogue entry.
///
/// The environment-model form of `DialectProfile::find(name).map_or(…)`:
/// aliases canonicalise onto the catalogue name, and the model-only ids
/// (the lenient `tcl`, the additive `tk`) have no catalogue entry and so
/// land on the picker's starting dialect exactly as an unknown spelling
/// always did.
#[must_use]
pub fn catalogue_dialect_or_default(name: &str) -> &'static str {
    tcl_registry::model::resolve_environment(name)
        .catalogue_profile()
        .map_or(DEFAULT_DIALECT, |profile| profile.name)
}

/// The command **store** for a dialect name — the resolved environment's
/// registry generation, replacing `tcl_registry::cache::registry_for_dialect`.
#[must_use]
pub fn store_for_dialect(name: &str) -> &'static CommandRegistry {
    tcl_registry::model::static_context_for(name).commands()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_names_canonicalise_and_unknown_names_take_the_default() {
        for profile in DialectProfile::all() {
            assert_eq!(catalogue_dialect_or_default(profile.name), profile.name);
            for &alias in profile.aliases {
                assert_eq!(catalogue_dialect_or_default(alias), profile.name, "{alias}");
            }
        }
        for name in ["", "tcl", "tk", "wish", "not-a-real-dialect"] {
            assert_eq!(
                catalogue_dialect_or_default(name),
                DEFAULT_DIALECT,
                "{name}"
            );
        }
    }

    /// The generation store is the very allocation the retired name-keyed
    /// cache published, so nothing the studio browses moves.
    #[test]
    fn the_generation_store_is_the_cached_one() {
        for name in ["spectcl", "tcl9.0", "f5-irules"] {
            assert!(std::ptr::eq(
                store_for_dialect(name),
                tcl_registry::cache::registry_for_dialect(name)
            ));
        }
    }
}
