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

//! The spec-pack facts a specialised site rests on, as the claims codegen
//! records beside its bindings (`tcl_runtime_api::SiteClaim`) —
//! `docs/design/compiler/registry-consumer-contracts.md` § *What the
//! artefact records per rung*.
//!
//! A spec an installed pack supplied carries a [`PackOrigin`] in the
//! registry it was installed into; any site whose emitted code rests on
//! such a spec records the origin's [`PackFactStamp`], stamped with the
//! registry's overlay generation and the building thread's evaluator
//! revision. [`pack_fact_stamp`] is the one construction: an embedder's held
//! facts for a pack set (`tcl_spectcl::PackSet::fact_stamps`) are built by
//! it too, so a site and the VM that admits it agree by construction.

use tcl_registry::CommandRegistry;
use tcl_registry::pack_origin::PackOrigin;
use tcl_registry::registry::ResolvedCall;
use tcl_runtime_api::{CommandBindingIdentity, PackFactStamp, SiteClaim};

/// The stamp of `origin`'s facts under an overlay generation and an
/// evaluator revision.
#[must_use]
pub fn pack_fact_stamp(
    origin: &PackOrigin,
    overlay_generation: u64,
    evaluator_revision: u64,
) -> PackFactStamp {
    PackFactStamp {
        pack: origin.pack.clone(),
        content_hash: origin.content_hash,
        vocabulary_version: origin.vocabulary_version.clone(),
        overlay_generation,
        evaluator_revision,
    }
}

/// The building thread's evaluator revision: the generation of the host
/// that serves declared implementations
/// ([`tcl_registry::pack_hooks::evaluator_generation`]).
#[must_use]
pub fn evaluator_revision() -> u64 {
    u64::from(tcl_registry::pack_hooks::evaluator_generation().0)
}

/// The stamp a site compiled against `registry` records for a spec from
/// `origin`.
fn site_stamp(registry: &CommandRegistry, origin: &PackOrigin) -> PackFactStamp {
    pack_fact_stamp(
        origin,
        registry.overlay_generation().unwrap_or(0),
        evaluator_revision(),
    )
}

/// Rung 1: the claim a site makes when a pack-supplied `spec` answered it at
/// compile time — a constant its `const_fold` computed. `None` for a spec no
/// pack supplied.
#[must_use]
pub fn pack_facts_claim(
    registry: &CommandRegistry,
    spec: &tcl_registry::CommandSpec,
) -> Option<SiteClaim> {
    registry
        .pack_origin(spec)
        .map(|origin| SiteClaim::PackFacts(site_stamp(registry, origin)))
}

/// Rung 3: the pack facts behind a pack command's declared backing, for a site
/// that inlines the definition the backing names. `None` for a spec no pack
/// supplied.
#[must_use]
pub fn reference_body_facts(
    registry: &CommandRegistry,
    spec: &tcl_registry::CommandSpec,
) -> Option<PackFactStamp> {
    registry
        .pack_origin(spec)
        .map(|origin| site_stamp(registry, origin))
}

/// Rung 2: the claim a site makes when its `binding` reached a builtin
/// through the resolved pack command's `alias_of` — its identity is not the
/// command's own name. `None` for every other binding.
#[must_use]
pub fn builtin_alias_claim(
    registry: &CommandRegistry,
    resolved: &ResolvedCall<'_>,
    binding: &CommandBindingIdentity,
) -> Option<SiteClaim> {
    if binding.identity == resolved.spec.name {
        return None;
    }
    let origin = registry.pack_origin(resolved.spec)?;
    Some(SiteClaim::BuiltinAlias {
        binding: binding.clone(),
        facts: site_stamp(registry, origin),
    })
}
