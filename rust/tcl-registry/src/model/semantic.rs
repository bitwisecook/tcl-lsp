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

//! Generation-bound metadata contexts for semantic invocation selection.
//!
//! [`SemanticContext`] is a `Copy` handle to the interned, default-keyed,
//! un-overlaid [`ContextRegistry`] for one environment generation. The handle
//! carries that generation's availability view and command store. Its equality
//! compares the interned generation handle; it is not a runtime interpreter,
//! command token, original source owner or native compilation receipt.
//!
//! Function units share this small handle for repeated semantic queries.
//! [`ResolvedContext`] supports complete structural equality, but cloning and
//! comparing its environment, floors, packages and authoring scope for every
//! function unit would repeat work for a shared generation.
//!
//! A source analysis with its own keyed versions, overlays or package context
//! instead retains its actual [`ContextRegistry`]. Such callers use
//! [`crate::model::assembly::resolve_structured_invocation_in_resolved_context`]
//! with that owner's command store and borrowed availability view. A default
//! static handle cannot replace the independently retained analysis context.
//!
//! Both doors use the same structured selection owner. A supplied context must
//! admit the literal head under the selected availability realm before its
//! authored descriptor resolves. A computed head remains unresolved; absence
//! of a supplied context retains the command store's ordinary shape query.
//! Availability and metadata selection grant no live command presence,
//! original argument provenance, variable access, Normal result or native
//! compiler admission.

use tcl_dialect::{DialectProfile, TclVersion};

use crate::invocation_words::InvocationWords;
use crate::model::assembly::ContextRegistry;
use crate::model::context::ResolvedContext;
use crate::model::ingress::static_context_for;
use crate::registry::CommandRegistry;
use crate::resolved_invocation::StructuredInvocationResolution;

/// Default-keyed, un-overlaid metadata context for one environment generation.
/// Function units share the interned handle. Analyses with independently
/// retained keyed or overlaid contexts use their actual `ContextRegistry`.
/// This handle does not identify a runtime interpreter or original source.
#[derive(Clone, Copy)]
pub struct SemanticContext {
    /// Interned default metadata generation selected by [`static_context_for`].
    /// Equality retains this generation handle across environment reloads.
    generation: &'static ContextRegistry,
}

impl SemanticContext {
    /// The semantic context for the environment `name` resolves to — the one
    /// dialect-name ingress
    /// ([`crate::model::ingress::resolve_environment`]), then that
    /// environment's un-overlaid generation.
    #[must_use]
    pub fn for_environment(name: &str) -> Self {
        Self {
            generation: static_context_for(name),
        }
    }

    /// Select the default environment generation named by a resolved profile.
    /// This does not retain another analysis's keyed or overlaid context.
    #[must_use]
    pub fn for_profile(profile: &DialectProfile) -> Self {
        Self::for_environment(profile.name)
    }

    /// The environment id this context names.
    #[must_use]
    pub fn environment_id(self) -> &'static str {
        self.generation.context().environment.id.as_str()
    }

    /// The availability view every selection here is filtered by.
    #[must_use]
    pub fn context(self) -> &'static ResolvedContext {
        self.generation.context()
    }

    /// Read this default generation's immutable command store.
    /// Independently owned analysis contexts retain their own command store.
    #[must_use]
    pub fn commands(self) -> &'static CommandRegistry {
        self.generation.commands()
    }

    /// The environment's runtime release, when its core names one — the
    /// premise the guarded-intrinsic selection reads.
    ///
    /// Derived from the environment's own point
    /// (`EnvironmentDefinition::point`), the same derivation the ingress and
    /// the projected profile read, so a dialect with no catalogue row (`tk`,
    /// whose core is 8.6) answers with its core rather than `None`, and a
    /// non-Tcl family (`jim`) answers `None` because it has no rung on the
    /// Tcl ladder. `the_point_names_the_catalogue_runtime_base` pins that
    /// the environment's point and catalogue runtime base agree
    /// for every row that has one.
    #[must_use]
    pub fn runtime_version(self) -> Option<TclVersion> {
        super::ingress::environments()
            .resolve(self.environment_id())
            .and_then(|definition| definition.point())
            .and_then(tcl_dialect::model::DialectPoint::tcl_version)
    }
}

impl PartialEq for SemanticContext {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.generation, other.generation)
    }
}

impl Eq for SemanticContext {}

impl std::hash::Hash for SemanticContext {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::from_ref(self.generation).hash(state);
    }
}

impl std::fmt::Debug for SemanticContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SemanticContext")
            .field(&self.environment_id())
            .finish()
    }
}

/// Resolve structured source words to target-neutral registry semantics **in
/// context** — the executable-IR face of the C7/I4 selection primitive.
///
/// `commands` is the store the caller reads; `context` is the resolved
/// context the invocation executes under, when the caller has one.
///
/// **Invariant I4** — a carried context is a proof obligation: the literal
/// head must resolve to a spec's declaration under the document's environment
/// ([`ResolvedContext::resolve_spec`] — availability-filtered, not merely mask
/// membership). A head nothing provides here is
/// [`crate::model::BindingKnowledge::Absent`], recorded as the same
/// [`crate::InvocationResolutionUnresolved::UnknownLiteralHead`] an absent store spec
/// produces, so the executable IR keeps its typed decline rather than gaining
/// a second "present but unavailable" shape. Subcommand and form selection
/// then proceed under the same environment's authoring mask, so a
/// gate-excluded subcommand or form cannot be selected either.
///
/// No context means the caller carries no environment — the obligation is
/// `NotRequired` and the ordinary command-store shape selection applies.
#[must_use]
pub fn resolve_structured_invocation_in_context<'r, 'w>(
    commands: &'r CommandRegistry,
    context: Option<SemanticContext>,
    words: InvocationWords<'w>,
) -> StructuredInvocationResolution<'r, 'w> {
    if context.is_none() {
        return commands.resolve_structured_invocation(words, None);
    }
    resolve_structured_invocation_in_realm(
        commands,
        context,
        words,
        tcl_dialect::model::InvocationRealm::RuleLoader,
    )
}

/// Resolve an invocation under its explicitly retained availability phase.
/// The phase changes availability filtering, never runtime command identity
/// or native compiler admission evidence.
#[must_use]
pub fn resolve_structured_invocation_in_realm<'r, 'w>(
    commands: &'r CommandRegistry,
    context: Option<SemanticContext>,
    words: InvocationWords<'w>,
    realm: tcl_dialect::model::InvocationRealm,
) -> StructuredInvocationResolution<'r, 'w> {
    crate::model::assembly::resolve_structured_invocation_in_resolved_context(
        commands,
        context.map(SemanticContext::context),
        words,
        realm,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InvocationResolutionUnresolved;

    #[test]
    fn one_generation_per_environment_id_so_equality_is_identity() {
        // Environment registrations deliberately advance the live generation
        // and registry tests exercise that channel concurrently.  Sample both
        // handles inside one stable generation so this test measures the
        // promotion invariant rather than racing a legitimate reload.
        let stable_pair = |left: &str, right: &str| loop {
            let before = crate::model::ingress::environments().generation();
            let a = SemanticContext::for_environment(left);
            let b = SemanticContext::for_environment(right);
            if crate::model::ingress::environments().generation() == before {
                break (a, b);
            }
        };

        let (a, b) = stable_pair("tcl8.6", "tcl8.6");
        assert_eq!(a, b);
        assert_eq!(a.environment_id(), "tcl8.6");
        assert_ne!(a, SemanticContext::for_environment("tcl9.0"));
        // Aliases resolve to the canonical environment, so they share the one
        // generation rather than interning a second view.
        let (alias, canonical) = stable_pair("irules", "f5-irules");
        assert_eq!(alias, canonical);
    }

    #[test]
    fn an_unknown_name_sinks_to_the_lenient_environment() {
        // The ingress contract: unknown and unstated names resolve to `tcl`.
        // Under the retired mask vocabulary they produced `None`
        // and the bundle declined outright; here they name a real context.
        for name in ["", "tcl", "no-such-dialect"] {
            assert_eq!(
                SemanticContext::for_environment(name).environment_id(),
                "tcl",
                "{name}"
            );
        }
    }

    /// **The D1 re-key equivalence sweep** (ledger C1). The retired semantic
    /// key was `tcl_dialect::DialectProfile::find(profile.name).map(tcl_dialect::DialectProfile::surface_query)` — the *exact* bit a profile's
    /// canonical name parses to, not the wider set of releases that dialect
    /// can reach. This pins what changed when the executable-IR path moved
    /// onto the resolved context, over every command name in every catalogue
    /// environment's store:
    ///
    /// - **nothing is ever lost.** No environment resolves fewer names than
    ///   the retired bit did, so no executable fact the old key produced can
    ///   disappear;
    /// - **the single-bit environments are byte-identical.** For the five
    ///   `tclN.N` ladder environments plus `f5-irules` and `f5-bigip` — whose
    ///   authoring mask *is* the bit their name parses to — the two answers
    ///   agree name for name. `tcl8.6` is the session default, so the
    ///   mainline LSP path is unchanged;
    /// - everything else **widens**, and the enumeration lives in the
    ///   redesign's §11.2 D1 row.
    #[test]
    fn context_resolution_refines_the_point_and_agrees_on_the_single_surface_ladder() {
        const IDENTICAL: &[&str] = &[
            "tcl8.4",
            "tcl8.5",
            "tcl8.6",
            "tcl9.0",
            "tcl9.1",
            "f5-irules",
            "f5-bigip",
        ];
        for profile in DialectProfile::all() {
            let context = SemanticContext::for_environment(profile.name);
            let commands = context.commands();
            let point = Some(profile.surface_query());
            let identical = IDENTICAL.contains(&profile.name);
            let names: Vec<&'static str> = commands.command_names().collect();
            for name in names {
                let by_point = commands.get_for_surface(name, point);
                let in_context = context.context().resolve_spec(commands, name);
                // The context *refines* the point: it also proves the
                // command's package can be hosted here, so it may refuse
                // what the point alone admits (`tk_popup` under `bpf`) but
                // can never admit what the point refuses.
                assert!(
                    in_context.is_none() || by_point.is_some(),
                    "{}: `{name}` resolved in context but not at the environment's point",
                    profile.name
                );
                if identical {
                    assert_eq!(
                        by_point.map(std::ptr::from_ref),
                        in_context.map(std::ptr::from_ref),
                        "{}: `{name}` must select the same spec either way",
                        profile.name
                    );
                }
            }
        }
    }

    #[test]
    fn a_head_the_environment_does_not_provide_is_an_unknown_literal_head() {
        // I4: the binding proof, not merely mask membership. `tk_popup` is in
        // the iRules generation's store (the store is shared) but nothing
        // declares it for the iRules environment, so an iRules context must
        // decline it exactly as an absent store spec would.
        let context = SemanticContext::for_environment("f5-irules");
        let commands = context.commands();
        let args: Vec<&str> = vec![".m", "1", "2"];
        let resolution = resolve_structured_invocation_in_context(
            commands,
            Some(context),
            InvocationWords::literals("tk_popup", &args),
        );
        assert!(
            matches!(
                resolution.unresolved(),
                Some(InvocationResolutionUnresolved::UnknownLiteralHead {
                    spelling: "tk_popup"
                })
            ),
            "expected an unknown-literal-head decline, got {resolution:?}"
        );
        // Omitting the context retains the attached store's selected profile;
        // it cannot manufacture a Tk provider in the iRules environment.
        assert!(
            resolve_structured_invocation_in_context(
                commands,
                None,
                InvocationWords::literals("tk_popup", &args),
            )
            .resolved()
            .is_none()
        );
        // The explicitly ambient Tk provider admits the same availability
        // query. This is catalogue availability, not an execution body proof.
        let loaded = SemanticContext::for_environment("tk");
        assert!(
            resolve_structured_invocation_in_context(
                loaded.commands(),
                Some(loaded),
                InvocationWords::literals("tk_popup", &args),
            )
            .resolved()
            .is_some()
        );
    }
}
