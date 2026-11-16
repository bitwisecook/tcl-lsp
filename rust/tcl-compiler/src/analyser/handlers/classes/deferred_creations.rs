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

//! Creation calls the walk could not classify, kept for a retry once every metaclass is known.

use super::super::super::state::Analyser;
use super::super::super::types::ClassDef;
use super::super::qualify;
use super::parameterised_creations::join_parameterised_class_observations;
use tcl_lexer::Token;

/// How many unclassifiable creation calls one document keeps for the
/// post-pass retry — see [`Analyser::defer_class_creation`].
///
/// Only a call whose head names nothing the document knows is kept, so a real
/// file holds a handful: the shape exists to catch a metaclass proved after
/// the walk, and a document proves few. The cap bounds a generated or
/// half-edited file whose calls are all to unknown commands; past it the
/// document behaves exactly as it did before this pass existed, which is the
/// abstaining direction.
const MAX_DEFERRED_CLASS_CREATIONS: usize = 4096;

impl Analyser {
    /// Record the classes a proc manufactures under a **computed name** whose
    /// value a literal call-site argument proves.
    ///
    /// A namespace-normalising factory commonly creates its metaclass as
    /// `Base create ${namespace}::class {…}` inside a procedure. The name
    /// word is written dynamically, so the ordinary walk abstains and the
    /// metaclass never enters the class-factory index. A literal load-time
    /// call can nevertheless prove that computed absolute name.
    ///
    /// This pass follows exactly that: a **literal argument at a load-time
    /// call site**, through the parameter binding, into the creation call's
    /// name word. It is the const/literal propagation the `foreach`
    /// simulation already does for a literal list
    /// ([`Analyser::simulate_remaining_foreach_iterations`]), with the call
    /// site rather than the list supplying the values. The ordinary walk's
    /// registry-classified placeholder supplies the already-parsed class
    /// structure; this pass changes only its proved identity and re-derives
    /// identity-dependent factory facts against the completed class lattice.
    ///
    /// **What it proves, and what it does not.** Each literal call site is
    /// independent evidence: `mkdialect ::T::D` really does create
    /// `::T::D::class`, and a *different* call site passing `$x` neither adds
    /// to nor subtracts from that. So a mixed set of call sites yields
    /// exactly the classes the literal ones prove, and nothing for the
    /// others — no guess anywhere. What is deliberately **not** claimed:
    ///
    /// * a call site the pass cannot read as naming this proc (a bare
    ///   relative spelling resolved through a `namespace path`, a
    ///   `{*}`-expanded argument list, an `interp alias`) contributes
    ///   nothing, so the class it would have proved stays unrecorded — the
    ///   abstaining direction;
    /// * only **load-time** call sites count. A call written inside another
    ///   proc's or class's body runs at *call* time, if ever, and the same
    ///   `offset_is_inside_any_definition_body` rule the load-level
    ///   destruction filter uses applies here;
    /// * the resolved name must be **absolute**. A relative one would have to
    ///   be homed against the call site's namespace, which this pass does not
    ///   model, so it abstains instead of homing it wrongly;
    /// * a name word carrying a command substitution, a backslash escape, or
    ///   an unbound interpolation resolves to nothing;
    /// * nested control-flow bodies are retained as paths. Typed registry
    ///   hooks select `if`/`switch` arms and the unconditional parts of
    ///   `try`; a dynamic condition, handler-only path, loop, or unsupported
    ///   control shape abstains instead of flattening its body into the
    ///   unconditional stream.
    ///
    /// No command is named here and no framework is special-cased: the
    /// manufacturer words come from the registry
    /// ([`tcl_registry::CommandRegistry::is_manufacturer_method`]) and the
    /// creation's structural template exists only when the ordinary generic
    /// handler classified it as a class creation.
    /// Keep a creation call the walk declined, in case a metaclass proved
    /// after the walk turns it into one.
    ///
    /// The gate is "nothing this document knows makes the head's verdict
    /// final": a registry command has its own classification, and a recorded
    /// **proc** is a command that is not a class, so neither can become a
    /// factory later and neither is kept.
    ///
    /// A recorded **class** deliberately *is* kept, even though the walk knew
    /// the name. A class can be recorded without a factory and gain one in the
    /// post-pass — an `oo::define` on a computed-name metaclass makes exactly
    /// such a stub, and the join then proves it — so "known" is not the same
    /// question as "judged".
    ///
    /// A call needs at least a manufacturer word and a name word to be a
    /// creation at all, so shorter ones are never kept. Buffering is bounded:
    /// past [`MAX_DEFERRED_CLASS_CREATIONS`] a document keeps the behaviour it
    /// had before this pass existed, which is the abstaining direction.
    pub(super) fn defer_class_creation(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
        cmd_tok: Token,
    ) {
        // Cheapest questions first: this runs for every command the generic
        // handler declines, which is most of a document.
        if args.len() < 2
            || arg_tokens.len() < 2
            || self.deferred_class_creations.len() >= MAX_DEFERRED_CLASS_CREATIONS
            || crate::naming::is_dynamic_word(cmd_name)
            || self
                .registry
                .as_deref()
                .is_some_and(|registry| registry.get(cmd_name).is_some())
        {
            return;
        }
        let namespace = self.command_resolution_namespace(scope_path);
        if crate::naming::bareword_resolution_candidates(&namespace, cmd_name)
            .into_iter()
            .any(|candidate| self.result.all_procs.contains_key(&candidate))
        {
            return;
        }
        self.deferred_class_creations
            .push(super::super::super::types::DeferredClassCreation {
                cmd_name: cmd_name.to_owned(),
                args: args.to_vec(),
                arg_tokens: arg_tokens.to_vec(),
                scope_path: scope_path.to_vec(),
                cmd_tok,
            });
    }

    /// Form the deferred verdicts, now that
    /// [`Self::record_literal_parameter_definitions`] has settled the
    /// parameterised-class join.
    ///
    /// Replayed in document order, so a chain settles in one pass: a class
    /// this replay records is in the index before the next call is retried,
    /// which is what lets `Meta create ::A::sub` and then `::A::sub create
    /// ::A::leaf` both land when `Meta` itself was only proved post-pass.
    ///
    /// Each call goes back through the *same* handler the walk used, so a
    /// replayed creation is recorded by one code path with the walk's own
    /// rules — the manufacturer layout, the injected members, the body walk,
    /// the W314/W315 emissions. The retry is gated on the head now naming a
    /// factory, so a head that stayed unknown costs one lookup and records
    /// nothing.
    pub(in super::super::super) fn replay_deferred_class_creations(&mut self) {
        if self.deferred_class_creations.is_empty() {
            return;
        }
        for deferred in std::mem::take(&mut self.deferred_class_creations) {
            // What the walk already recorded under the name this call
            // creates — an `oo::define` stub it made *because* the creation
            // was invisible. Captured before the replay, which overwrites it.
            let prior = self.deferred_creation_prior(&deferred);
            if !self.handle_oo_class_command(
                &deferred.cmd_name,
                &deferred.args,
                &deferred.arg_tokens,
                &deferred.scope_path,
                deferred.cmd_tok,
            ) {
                continue;
            }
            if let Some((qualified, prior)) = prior {
                self.rejoin_replayed_class(&qualified, &prior, &deferred.scope_path);
            }
        }
        // A replayed call has had its verdict, whichever way it went. The
        // handler's decline path buffers again — it cannot tell a first
        // hearing from a second — so the buffer is emptied here rather than
        // left to be replayed once more against the same evidence.
        self.deferred_class_creations.clear();
    }

    /// The class name a deferred call would record, paired with whatever the
    /// walk already holds under it.
    ///
    /// `None` when the head still names no factory, when the layout puts no
    /// name where this call has a word, or — the ordinary case — when nothing
    /// was recorded under that name at all.
    fn deferred_creation_prior(
        &self,
        deferred: &super::super::super::types::DeferredClassCreation,
    ) -> Option<(String, ClassDef)> {
        let resolved =
            self.resolve_class_creation(&deferred.args, &deferred.arg_tokens, deferred.cmd_tok)?;
        let raw_name = deferred.args.get(resolved.layout.name_arg)?;
        let namespace = self.command_resolution_namespace(&deferred.scope_path);
        let qualified = qualify(&namespace, raw_name);
        Some((
            qualified.clone(),
            self.result.all_classes.get(&qualified)?.clone(),
        ))
    }

    /// Fold a record the walk made for a class whose creation call it could
    /// not see back into the replayed creation.
    ///
    /// Because the creation was invisible, an `oo::define` on the same class
    /// made a *stub* — and that stub is where its members went. The replay
    /// then records the creation under the same name, so without this the
    /// members the walk did record would be lost: strictly worse than before
    /// the replay existed.
    ///
    /// The union is the one [`join_parameterised_class_observations`] already
    /// owns, used exactly as the parameterised-class post-pass uses it, so a
    /// stub joins a creation by one rule wherever the two meet. Two
    /// observations that contradict each other publish the abstaining record
    /// the house rule calls for — no factory, inheritance unknown — while
    /// still keeping both sides' declarations, which are declarations of this
    /// class either way.
    fn rejoin_replayed_class(&mut self, qualified: &str, prior: &ClassDef, scope_path: &[usize]) {
        let Some(observed) = self.result.all_classes.get(qualified) else {
            return;
        };
        let joined = join_parameterised_class_observations(prior, observed).unwrap_or_else(|| {
            let mut abstaining = observed.clone();
            abstaining.factory = None;
            abstaining.inheritance_unknown = true;
            abstaining.absorb_declarations(prior);
            abstaining
        });
        let simple = joined.name.clone();
        self.result
            .all_classes
            .insert(qualified.to_owned(), joined.clone());
        if let Some(scope) =
            super::super::super::scope::scope_at_mut(&mut self.result.global_scope, scope_path)
        {
            scope.classes.insert(simple, joined);
        }
    }
}
