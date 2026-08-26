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

//! The registry on the new core/environment model (P1-E of the redesign,
//! `docs/design/dialect-and-package-registry-redesign.md` §4 and
//! `docs/design/dialect-and-package-registry-centralisation.md` §1).
//!
//! Four submodules, layered exactly as the design's availability chapter
//! draws them:
//!
//! - [`surface`] — [`SurfaceDeclaration`]: provider + axis-typed
//!   [`tcl_dialect::model::VersionSet`] applicability + predicate +
//!   history, and the **mechanical translation** from today's
//!   [`crate::CommandSpec`] fields ([`declarations_for_spec`]).
//! - [`context`] — [`ResolvedContext`] (environment + per-axis floor map)
//!   and the [`ContextQueries`] **assistance view** (§1.2 R-c/R-d split):
//!   `is_available`, `available_at_targets`.
//! - [`assembly`] — [`ContextRegistry`]: per-environment registry
//!   generations assembled by provider filtering instead of bit loading,
//!   cached by `(environment identity, keyed-versions hash)`.
//! - [`binding`] — the [`BindingKnowledge`] **semantic view** types
//!   (I3–I5) plus the package realm vocabulary
//!   ([`PackageStateMap`], [`PackageTransition`]); realm integration
//!   itself is P1a.
//!
//! Everything here lands **alongside** the old `DialectSet`-mask registry:
//! nothing existing is wrapped or shimmed, and the equivalence sweeps in
//! [`assembly`] pin new-model visibility to the old model's answers for
//! every compiled spec under every catalogue profile.

pub mod assembly;
pub mod binding;
pub mod context;
pub mod surface;

pub use assembly::{ContextRegistry, registry_for_environment};
pub use binding::{BindingKnowledge, PackageState, PackageStateMap, PackageTransition, SpecKey};
pub use context::{ContextQueries, FloorMap, KeyedVersions, ResolvedContext, specificity_breadth};
pub use surface::{
    BuildCapability, CapabilityPredicate, PackageId, Provider, SurfaceDeclaration,
    declarations_for_spec,
};
