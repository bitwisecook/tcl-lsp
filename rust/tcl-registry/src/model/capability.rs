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

//! What a pack's distance from the workspace root lets it declare.
//!
//! `docs/design/compiler/registry-consumer-contracts.md` § *Dialects and
//! packages*. Two gates decide whether a pack's declaration that changes
//! emitted code, or claims how a command runs, reaches a registry, and a
//! declaration must pass both:
//!
//! - the **provenance gate** — where the pack was found, and whether the
//!   editor trusts the folder (`tcl_spectcl::stamps::stamps_admitted_from`
//!   for a codegen-axis stamp);
//! - the **capability gate** here — how far the *package* that ships the pack
//!   sits from the workspace root ([`DependencyTier`]). That is a fact the
//!   discovery tier cannot express: a pack found beside a `tclpkg.tcl` is a
//!   workspace-tier file whether the author wrote it or a dependency of a
//!   dependency shipped it.
//!
//! [`CodegenCapability::for_tier`] is the matrix. A pack no package ships has
//! no tier and is not narrowed by it.

pub use tcl_dialect::model::DependencyTier;

/// What the packs of one [`DependencyTier`] may declare.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodegenCapability {
    /// The tier this is the capability of.
    pub tier: DependencyTier,
    /// May name a member of a closed code-generation catalogue
    /// (`codegen_hook`, `inline_codegen_hook`, `semantic_operation
    /// {Intrinsic …}`).
    pub codegen_stamps: bool,
    /// May declare a `runtime_backing` other than `none`.
    pub runtime_backing: bool,
    /// May declare `alias_of`, which is what a site recorded against a
    /// shipped builtin rests on.
    pub builtin_alias: bool,
    /// Which reference bodies it may supply: whether a `runtime_backing` that
    /// is a Tcl body — which the compiler inlines into the code of whatever
    /// calls the command — survives the load from a pack at this tier.
    pub reference_body: ReferenceBodies,
}

/// Which reference bodies the packs of a tier may supply.
///
/// The design names "which `BodySource`" here, and no tier is yet allowed
/// one source and not the other, so the two states are the two answers the
/// matrix gives; a tier that gains one source without the other adds its
/// variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceBodies {
    /// No reference body.
    Forbidden,
    /// A reference body from either source: the package's own installed
    /// source, or text carried in the pack.
    AnySource,
}

impl CodegenCapability {
    /// The capability of `tier`. The workspace's own package may declare
    /// everything, a direct dependency may declare a backing and an alias but
    /// no stamp and no body, and a transitive or development dependency may
    /// declare none of them, so a package deep in a dependency graph cannot
    /// change what the workspace emits.
    #[must_use]
    pub const fn for_tier(tier: DependencyTier) -> Self {
        let (codegen_stamps, runtime_backing, builtin_alias, reference_body) = match tier {
            DependencyTier::Root => (true, true, true, ReferenceBodies::AnySource),
            DependencyTier::Direct => (false, true, true, ReferenceBodies::Forbidden),
            DependencyTier::Transitive | DependencyTier::Development => {
                (false, false, false, ReferenceBodies::Forbidden)
            }
        };
        Self {
            tier,
            codegen_stamps,
            runtime_backing,
            builtin_alias,
            reference_body,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ReferenceBodies::{AnySource, Forbidden};
    use super::*;

    const TIERS: [DependencyTier; 4] = [
        DependencyTier::Root,
        DependencyTier::Direct,
        DependencyTier::Transitive,
        DependencyTier::Development,
    ];

    /// The matrix as the design page states it, one row per tier.
    #[test]
    fn the_matrix_is_the_pages() {
        let row = |tier| {
            let capability = CodegenCapability::for_tier(tier);
            (
                capability.codegen_stamps,
                capability.runtime_backing,
                capability.builtin_alias,
                capability.reference_body,
            )
        };
        assert_eq!(row(DependencyTier::Root), (true, true, true, AnySource));
        assert_eq!(row(DependencyTier::Direct), (false, true, true, Forbidden));
        assert_eq!(
            row(DependencyTier::Transitive),
            (false, false, false, Forbidden)
        );
        assert_eq!(
            row(DependencyTier::Development),
            (false, false, false, Forbidden)
        );
    }

    #[test]
    fn a_capability_names_its_own_tier() {
        for tier in TIERS {
            assert_eq!(CodegenCapability::for_tier(tier).tier, tier);
        }
    }

    /// A tier further from the root never gains what a nearer one lacks:
    /// each capability of a dependency is one its package's parent has.
    #[test]
    fn distance_never_widens_a_capability() {
        let holds: [fn(&CodegenCapability) -> bool; 4] = [
            |c| c.codegen_stamps,
            |c| c.runtime_backing,
            |c| c.builtin_alias,
            |c| c.reference_body == ReferenceBodies::AnySource,
        ];
        for pick in holds {
            for (nearer, further) in [
                (DependencyTier::Root, DependencyTier::Direct),
                (DependencyTier::Direct, DependencyTier::Transitive),
                (DependencyTier::Direct, DependencyTier::Development),
            ] {
                assert!(
                    pick(&CodegenCapability::for_tier(nearer))
                        || !pick(&CodegenCapability::for_tier(further)),
                    "{further:?} holds what {nearer:?} lacks"
                );
            }
        }
    }
}
