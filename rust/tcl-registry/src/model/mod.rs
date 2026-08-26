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
//! - [`ingress`] — the one dialect-**name** ingress seam
//!   ([`resolve_environment`]): every user-written dialect string in the
//!   toolchain resolves to a [`DocumentEnvironment`] here, and derives its
//!   registry generation and interop profile from the resolved
//!   environment (centralisation R-a, ledger C2/F2/F3/F9).
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
pub mod ingress;
pub mod surface;

pub use assembly::{
    ContextRegistry, registry_for_environment, registry_for_environment_if_built,
    resolve_call_in_context, resolve_invocation_in_context, side_effect_hints_in_context,
};
pub use binding::{BindingKnowledge, PackageState, PackageStateMap, PackageTransition, SpecKey};
pub use context::{ContextQueries, FloorMap, KeyedVersions, ResolvedContext, specificity_breadth};
pub use ingress::{
    DocumentEnvironment, context_for_profile, environments, irules_context,
    is_known_environment_name, resolve_environment, resolve_known_environment, static_context_for,
    static_context_for_profile, static_document_context_for, static_document_context_for_profile,
};
pub use surface::{
    BuildCapability, CapabilityPredicate, PackageId, Provider, SurfaceDeclaration,
    declarations_for_spec,
};

/// The old-model **oracle** the parity sweeps compare against for one
/// catalogue profile — the profile itself for every environment except
/// the two the F5 reclassification corrected.
///
/// measurements §4a (`docs/design/bigip-irule-parser-measurements.md`,
/// F5 evidence review F2): the `f5-iapps`/`f5-tmsh` 8.5-core hypothesis
/// is **falsified** — both contexts report patchlevel 8.4.6 and fail
/// every 8.4/8.5 discriminator — so their environments now ride the
/// `f5-tcl` trunk (fork of Tcl at 8.4.6). The derived context facts
/// therefore differ from the *unfixed* old profiles in exactly three
/// fields, and the sweeps compare against this reclassified twin instead
/// of adjusting thousands of per-spec expectations by hand:
///
/// - `availability_mask`: the embedded core admits the **8.4** line, not
///   the falsified 8.5 one (`TCL85|vendor` → `TCL84|vendor`);
/// - `version_ceiling`: `8.5` → `8.4` (all sixteen measured 8.5
///   discriminators behave as 8.4);
/// - `operators_as_commands`: `true` → `false` (`::tcl::mathop` is
///   measured absent in both contexts).
///
/// The old catalogue rows themselves are P1-G's to retire; this twin is
/// the documented translation of the measurement onto the old vocabulary.
/// (`DialectProfile` deliberately stays `!Clone` — a `Clone` impl would
/// silently re-resolve `.clone()`/`.to_owned()` on `&'static` profiles
/// across the workspace — so the twin copies every field explicitly.)
#[cfg(test)]
pub(crate) fn f5_reclassified_oracle(
    profile: &'static tcl_dialect::DialectProfile,
) -> tcl_dialect::DialectProfile {
    use tcl_dialect::{DialectProfile, DialectSet, TclVersion};
    let reclassified = matches!(profile.name, "f5-iapps" | "f5-tmsh");
    let vendor = match profile.name {
        "f5-iapps" => DialectSet::IAPPS,
        "f5-tmsh" => DialectSet::TMSH,
        _ => DialectSet::empty(),
    };
    DialectProfile {
        name: profile.name,
        aliases: profile.aliases,
        display_name: profile.display_name,
        short_name: profile.short_name,
        editor_language_id: profile.editor_language_id,
        filenames: profile.filenames,
        file_extensions: profile.file_extensions,
        vendor_bit: profile.vendor_bit,
        availability_mask: if reclassified {
            DialectSet::TCL84.union(vendor)
        } else {
            profile.availability_mask
        },
        base_layers: profile.base_layers,
        grammar_union: profile.grammar_union,
        version_ceiling: if reclassified {
            Some(TclVersion::V8_4)
        } else {
            profile.version_ceiling
        },
        signature_base: profile.signature_base,
        runtime_base: profile.runtime_base,
        leading_zero_is_octal: profile.leading_zero_is_octal,
        expr_grammar_base: profile.expr_grammar_base,
        grammar: profile.grammar,
        operators_as_commands: if reclassified {
            false
        } else {
            profile.operators_as_commands
        },
        tcloo: profile.tcloo,
        has_fixed_ensembles: profile.has_fixed_ensembles,
        vm_runtime_version: profile.vm_runtime_version,
        libraries: profile.libraries,
        help_terms: profile.help_terms,
    }
}
