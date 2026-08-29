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

//! The **executable-IR vocabulary**: one generation-bound handle naming the
//! context every semantic invocation resolves under (centralisation ledger
//! row C1, redesign §11.2 D1).
//!
//! Before this module the semantic-analysis and executable-IR path spoke
//! `SpecSurface`: `SemanticAnalysisBundle` carried a `dialect: Option<SurfaceQuery<'_>>`
//! field, `build_linear_executable_ir` took a mask, and a bundle whose mask
//! did not name exactly one profile recorded a `DialectUnavailable` decline.
//! That was the last dialect vocabulary in the tree beside
//! [`ResolvedContext`], and D1 rules it "one change or none — a partial
//! re-key leaves two dialect vocabularies".
//!
//! # Why a handle rather than a [`ResolvedContext`] value
//!
//! `SemanticAnalysisBundle` is a field of `FunctionUnit`, which is a field of
//! `CompilationUnit`, which salsa memoises on `PartialEq`. A
//! [`ResolvedContext`] is an `Arc<EnvironmentDefinition>` plus four owned
//! vectors and is neither `Eq` nor `Copy`: cloning one per function unit on
//! every keystroke, and structurally comparing one per memo probe, would put
//! allocation and a deep compare on the per-edit path for a value that is
//! **the same object** for every unit of a document. Standing principle P-B
//! forbids exactly that.
//!
//! [`SemanticContext`] is therefore the generation-bound handle
//! [`crate::model::ingress::static_context_for`] already publishes: a
//! `&'static ContextRegistry` for one environment's un-overlaid,
//! default-keyed generation. It is `Copy`, its equality is pointer equality
//! (the promotion interns exactly one view per environment id, so pointer
//! equality *is* environment identity), and it carries both halves a
//! resolution needs — the [`ResolvedContext`] availability view and the
//! generation's command store — without a second lookup. Resolving one costs
//! a name ingress; the compiler resolves it **once per module build** and
//! threads the handle, where the retired `DialectSet` projection ran per
//! function unit.
//!
//! # The selection rule is C7/I4's, not a second one
//!
//! [`resolve_structured_invocation_in_context`] is the structured-words face
//! of the selection primitive
//! [`crate::model::assembly::resolve_invocation_in_context`] already
//! implements for the lowering-hook and side-effect paths: a carried context
//! is a binding-proof obligation ([`ResolvedContext::resolve_spec`]), and the
//! proved spec is the selected spec because both sides are `get_for_surface`
//! under the same authoring mask. No context means the caller carries no
//! environment — a unit harness or a shape-only query — and the
//! dialect-blind store selection stands, exactly as `None`
//! behaved.

use tcl_dialect::{DialectProfile, TclVersion};

use crate::invocation_words::InvocationWords;
use crate::model::assembly::ContextRegistry;
use crate::model::context::ResolvedContext;
use crate::model::ingress::static_context_for;
use crate::registry::CommandRegistry;
use crate::resolved_invocation::{InvocationResolutionUnresolved, StructuredInvocationResolution};

/// The context one function unit's executable IR was resolved under — a
/// generation-bound handle on the environment's un-overlaid registry
/// generation.
///
/// Replaces the `SpecSurface` the semantic-analysis path carried. Where the
/// mask could be empty, a combinator, or a bit whose name no profile owned,
/// this either names exactly one resolved environment or is absent
/// (`Option<SemanticContext>`), so the "no one explicit dialect profile"
/// decline the mask model needed becomes the ordinary absent-context case.
#[derive(Clone, Copy)]
pub struct SemanticContext {
    /// The interned un-overlaid generation for one environment id. There is
    /// one view per id (see [`static_context_for`]), so pointer equality is
    /// environment identity.
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

    /// [`Self::for_environment`] keyed by an already-resolved profile — the
    /// surviving interned-profile interop (retired with the profile itself
    /// under ledger F1 / redesign §11.2 D5). A profile's canonical name **is**
    /// a canonical environment id, so this is an id-keyed lookup and never a
    /// re-parse of a user string.
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

    /// The generation's own command store.
    ///
    /// Executable-IR callers pass the store they already hold (a unit
    /// registry, the analyser's generation), so the re-key changes *which
    /// context* filters a selection, never *which specs exist*; this door is
    /// for callers that have a context and no store of their own.
    #[must_use]
    pub fn commands(self) -> &'static CommandRegistry {
        self.generation.commands()
    }

    /// The environment's runtime release, when its core names one — the
    /// premise the guarded-intrinsic selection reads.
    ///
    /// Still projected through the interned catalogue profile: the runtime
    /// base is a [`DialectProfile`] field with no environment-model twin yet
    /// (redesign §11.2 D5's remaining boundary). The projection is id-keyed,
    /// so it answers for exactly the environment this handle names.
    #[must_use]
    pub fn runtime_version(self) -> Option<TclVersion> {
        DialectProfile::find(self.environment_id()).and_then(DialectProfile::runtime_version)
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
/// [`InvocationResolutionUnresolved::UnknownLiteralHead`] an absent store spec
/// produces, so the executable IR keeps its typed decline rather than gaining
/// a second "present but unavailable" shape. Subcommand and form selection
/// then proceed under the same environment's authoring mask, so a
/// gate-excluded subcommand or form cannot be selected either.
///
/// No context means the caller carries no environment — the obligation is
/// `NotRequired` and the dialect-blind store selection stands, exactly as the
/// retired `None` argument behaved.
#[must_use]
pub fn resolve_structured_invocation_in_context<'r, 'w>(
    commands: &'r CommandRegistry,
    context: Option<SemanticContext>,
    words: InvocationWords<'w>,
) -> StructuredInvocationResolution<'r, 'w> {
    let Some(context) = context else {
        return commands.resolve_structured_invocation(words, None);
    };
    let Some(name) = words.head_literal() else {
        // A computed head selects nothing in either model; report it through
        // the ordinary path so the decline names the word kind rather than a
        // missing spec.
        return commands
            .resolve_structured_invocation(words, Some(context.context().authoring_query()));
    };
    if context.context().resolve_spec(commands, name).is_none() {
        return StructuredInvocationResolution::from_unresolved(
            InvocationResolutionUnresolved::UnknownLiteralHead { spelling: name },
        );
    }
    commands.resolve_structured_invocation(words, Some(context.context().authoring_query()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_generation_per_environment_id_so_equality_is_identity() {
        let a = SemanticContext::for_environment("tcl8.6");
        let b = SemanticContext::for_environment("tcl8.6");
        assert_eq!(a, b);
        assert_eq!(a.environment_id(), "tcl8.6");
        assert_ne!(a, SemanticContext::for_environment("tcl9.0"));
        // Aliases resolve to the canonical environment, so they share the one
        // generation rather than interning a second view.
        assert_eq!(
            SemanticContext::for_environment("irules"),
            SemanticContext::for_environment("f5-irules")
        );
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
        // The same words resolve with no context carried, which is the
        // dialect-blind store selection the retired empty mask performed.
        assert!(
            resolve_structured_invocation_in_context(
                commands,
                None,
                InvocationWords::literals("tk_popup", &args),
            )
            .resolved()
            .is_some()
        );
    }
}
