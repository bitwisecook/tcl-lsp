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

//! The new core/environment model of the registry redesign (issue #1631,
//! `docs/design/dialect-and-package-registry-redesign.md` — P1 of §8).
//!
//! Four submodules carry the model's first layer and its algebra:
//!
//! - [`family`] — [`Family`] × [`Release`] × [`BuildProfileId`] core
//!   profiles ([`CoreProfileId`] / [`CoreProfile`]), with typed build
//!   capabilities ([`CapabilitySet`]).
//! - [`expr_grammar`] — the full §3.1 [`ExprGrammar`] contract as data.
//! - [`version_set`] — the axis-typed [`VersionSet`] algebra
//!   (differentially tested against `package vsatisfies`) and
//!   [`ItemHistory`].
//! - [`environment`] — [`EnvironmentDefinition`] / [`EnvironmentOverlay`]
//!   and the one-resolver [`EnvironmentRegistry`], seeded with every
//!   current catalogue name.
//!
//! This model lands **alongside** the old `DialectSet`/`DialectProfile`
//! types, which stay untouched until their consumers migrate; nothing
//! here wraps or shims them — old names are first-class data rows in the
//! new registry.

pub mod dynamic;
pub mod environment;
pub mod expr_grammar;
pub mod family;
pub mod version_set;

pub use dynamic::{
    DynamicCore, DynamicFamily, DynamicFamilyError, DynamicFamilyId, DynamicRegistration,
    DynamicRelease, dynamic_core_for, dynamic_core_grammar, dynamic_families, dynamic_generation,
    register_dynamic_families, reserved_family_name, resolve_dynamic_family,
};
pub use environment::{
    ConfigurationOrigin, CoreProfileSelector, DetectionFacts, EditorLanguageIdentityId,
    EnvironmentDefinition, EnvironmentId, EnvironmentIdentity, EnvironmentOverlay,
    EnvironmentOverlayError, EnvironmentPolicy, EnvironmentRegistry, EnvironmentRegistryError,
    FileExtensionClaim, KeyedAxis, PackageChanges, PackagePlacement, Placement, Provenance,
    TargetChanges, WorldPolicy, compiled_definitions,
};
pub use expr_grammar::{
    ExprArity, ExprGrammar, ExprSubstitution, MathFunc, MathFuncSet, PrecedenceTable, WordOperator,
    expr,
};
pub use family::{
    BuildProfileId, CapabilityAnswer, CapabilitySet, CoreProfile, CoreProfileId, Family, Release,
    ReleaseParseError, grammar,
};
pub use version_set::{
    HalfOpenRange, ItemHistory, ItemHistoryError, ItemState, Version, VersionAxisId, VersionSet,
    VersionSetError,
};
