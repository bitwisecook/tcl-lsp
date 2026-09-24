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

//! **Design Q6's trust floor**, at the registration seam.
//!
//! Its own test binary, and deliberately: the roster store is
//! process-wide, so a test that replaces it cannot share a process with
//! the ones in `jim_surface_roster.rs` that ask what a `jim` generation
//! offers. Cargo gives each integration file its own process, which is
//! the isolation this needs.

/// The trust floor, at the seam a real workspace pack would reach: a
/// roster narrows a compiled family's surface, so an untrusted tier's is
/// refused and the compiled-in one still stands.
#[test]
fn a_workspace_roster_cannot_narrow_a_compiled_family() {
    use tcl_dialect::model::Provenance;

    let pack = tcl_spectcl::loader::evaluate_pack(
        "speclib hostile 2.0 {\n include from tcl into jim {set}\n}",
    );
    assert_eq!(
        pack.surface_rosters.len(),
        1,
        "the row itself is well-formed"
    );

    let refused = tcl_spectcl::surface_roster_conversion::to_inherited_surfaces(
        &pack.surface_rosters,
        Provenance::WorkspaceUntrusted,
    );
    let outcome = tcl_dialect::model::register_inherited_surfaces(refused);
    assert_eq!(outcome.rosters, 0, "{:?}", outcome.rejected);
    assert_eq!(outcome.rejected.len(), 1);

    // …and the compiled-in roster is what the process goes back to.
    let restored = tcl_dialect::model::register_inherited_surfaces(
        tcl_spectcl::core_surfaces::builtin_rosters(),
    );
    assert_eq!(restored.rosters, 1);
}

/// #2139: the tier a pack was discovered from does not decide its trust
/// class — the provenance it maps to does, and one predicate answers.
///
/// `EvalOptions::tier`'s doc comment used to call `Tier::Workspace` and
/// `Tier::StudioOverride` "the untrusted provenance classes". It was wrong
/// about the first: the discovery location never decided a workspace pack's
/// class — the editor's Workspace Trust state does, and
/// `PackEnvironmentTier::provenance` maps `Workspace` to `WorkspaceTrusted`
/// or `WorkspaceUntrusted` by it. This pins the chain the comment
/// misdescribed, for both states.
#[test]
fn a_tier_decides_trust_only_through_its_provenance_issue_2139() {
    use tcl_dialect::model::{Provenance, WorkspaceTrust};
    use tcl_spectcl::discovery::Tier;
    use tcl_spectcl::loader::PackEnvironmentTier;

    let class =
        |tier: Tier, trust: WorkspaceTrust| PackEnvironmentTier::of(tier, trust).provenance();

    for trust in [WorkspaceTrust::Trusted, WorkspaceTrust::Untrusted] {
        assert_eq!(class(Tier::Bundled, trust), Provenance::BundledPack);
        assert_eq!(class(Tier::User, trust), Provenance::User);
        assert_eq!(
            class(Tier::StudioOverride, trust),
            Provenance::StudioOverride
        );
        assert!(class(Tier::StudioOverride, trust).is_untrusted());
        assert!(!class(Tier::Bundled, trust).is_untrusted());
        assert!(!class(Tier::User, trust).is_untrusted());
    }
    assert_eq!(
        class(Tier::Workspace, WorkspaceTrust::Trusted),
        Provenance::WorkspaceTrusted
    );
    assert_eq!(
        class(Tier::Workspace, WorkspaceTrust::Untrusted),
        Provenance::WorkspaceUntrusted
    );
    assert!(
        !class(Tier::Workspace, WorkspaceTrust::Trusted).is_untrusted(),
        "a workspace the editor trusts, or says nothing about, is trusted"
    );
    assert!(class(Tier::Workspace, WorkspaceTrust::Untrusted).is_untrusted());
}
