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

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::CommandSpec;
    use tcl_registry::hooks::InlineCodegenHookId;

    fn origin() -> PackOrigin {
        PackOrigin {
            pack: "vendor".to_owned(),
            content_hash: 7,
            vocabulary_version: "2".to_owned(),
        }
    }

    /// Insert `spec` as a pack's installer does: indexed, with its origin.
    fn install(registry: &mut CommandRegistry, spec: CommandSpec) {
        let spec: &'static CommandSpec = Box::leak(Box::new(spec));
        registry.insert_static(spec);
        registry.insert_pack_origin(spec, origin());
    }

    fn compile(source: &str, registry: &CommandRegistry) -> tcl_bytecode::ModuleAsm {
        let config = tcl_lexer::LexerConfig::default();
        let ir = crate::lowering::lower_script_module_for_bytecode(
            source, "", registry, config, None, false,
        );
        let cfg = crate::cfg_builder::build_cfg_codegen_with_registry_and_config(
            &ir, false, registry, config,
        );
        crate::codegen::codegen_module(&cfg, &ir, registry)
    }

    fn stamp(registry: &CommandRegistry) -> PackFactStamp {
        pack_fact_stamp(
            &origin(),
            registry.overlay_generation().unwrap_or(0),
            evaluator_revision(),
        )
    }

    fn double(args: &[&str]) -> Option<String> {
        let n: i64 = args.first()?.parse().ok()?;
        Some((n * 2).to_string())
    }

    /// Rung 1: a pack spec's `const_fold` answering a call at compile time
    /// claims the pack's facts; the same fold from a spec no pack supplied
    /// claims nothing.
    #[test]
    fn a_pack_fold_claims_the_pack_s_facts() {
        let spec = CommandSpec {
            name: "vendor::double",
            const_fold: Some(double),
            ..CommandSpec::DEFAULT
        };
        let mut registry = CommandRegistry::build_default();
        install(&mut registry, spec.clone());
        let module = compile("set x [vendor::double 21]\nset x", &registry);
        assert_eq!(
            module.top_level.site_claims,
            vec![SiteClaim::PackFacts(stamp(&registry))],
            "{:#?}",
            module.top_level
        );

        let mut embedder = CommandRegistry::build_default();
        embedder.insert(spec);
        let module = compile("set x [vendor::double 21]\nset x", &embedder);
        assert!(module.top_level.site_claims.is_empty());
    }

    /// Rung 2 on the inline path: a pack command whose `alias_of lindex`
    /// carries `lindex`'s own inline hook records the target's binding and
    /// the claim beside it.
    #[test]
    fn an_inline_alias_site_claims_the_builtin() {
        let mut registry = CommandRegistry::build_default();
        install(
            &mut registry,
            CommandSpec {
                name: "vendor::nth",
                alias_of: Some("lindex"),
                inline_codegen_hook: Some(InlineCodegenHookId::Lindex),
                ..CommandSpec::DEFAULT
            },
        );
        let module = compile("set l {a b c}\nset x [vendor::nth $l 1]\nset x", &registry);
        let binding = CommandBindingIdentity::new("vendor::nth", "lindex");
        assert!(
            module.top_level.command_bindings.contains(&binding),
            "{:#?}",
            module.top_level.command_bindings
        );
        assert_eq!(
            module.top_level.site_claims,
            vec![SiteClaim::BuiltinAlias {
                binding,
                facts: stamp(&registry),
            }]
        );
    }
}
