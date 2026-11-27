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

//! Object systems: method dispatch keywords, manufacturer methods, definition-body grammars, default construction and handle bindings.

use super::{CommandRegistry, spec_pack_of};
use crate::arg_role::ArgRole;
use crate::definer::DefinitionBodyGrammar;
use crate::spec::{CommandSpec, SubCommand};
use crate::traits::Traits;
use std::hash::Hash;
use tcl_dialect::model::SurfaceQuery;

impl CommandRegistry {
    /// Which `TclOO` method-context keyword `head` is, if it is one.
    ///
    /// The single source of truth other consumers would otherwise duplicate as
    /// `head == "my"` / `matches!(head, "my" | "next" | "nextto")` literals.
    /// Every consumer that needs "is this word a method
    /// dispatch or introspection keyword, and which kind" asks here, so a
    /// dialect that gains or loses one of them propagates through the specs
    /// rather than through a walker edit.
    ///
    /// Only the bare, contextual spelling is a dispatch keyword. `TclOO` installs
    /// `my` in the receiving object's namespace and exposes `next` / `nextto` /
    /// `self` through that namespace's path; it does not install global
    /// `::my`, `::next`, `::nextto`, or `::self` commands. Qualified
    /// `::oo::Helpers::...` commands are separate registry entries with the
    /// dispatch traits removed. [`Self::get`] owns that distinction and does
    /// not apply its leading-`::` fallback to a method-context-only singleton.
    ///
    /// Dialect-aware through the registry instance itself: a registry built
    /// by `registry_for_dialect` / `registry_for_profile` answers at that
    /// profile's point, so all four keywords — every one of them 8.6-and-
    /// later — return `None` from a `tcl8.4` or `tcl8.5` registry. A
    /// profile-less registry (`CommandRegistry::build_default`) answers
    /// dialect-agnostically, exactly as [`Self::get`] does.
    ///
    /// `link` is **not** a keyword here and must not be added: it creates
    /// per-class bareword commands rather than dispatching one. Nor is
    /// `self`'s definer-grammar homonym — the `self` word
    /// inside an `oo::define` body is a member-grammar wrapper, resolved
    /// through [`crate::definer`], not a command head.
    #[must_use]
    pub fn method_dispatch_keyword(&self, head: &str) -> Option<MethodDispatchKind> {
        let traits = self.spec_for_this_registry(head)?.traits;
        if traits.contains(Traits::TCLOO_SELF_DISPATCH) {
            Some(MethodDispatchKind::SelfDispatch)
        } else if traits.contains(Traits::TCLOO_NEXT_CHAIN) {
            Some(MethodDispatchKind::NextChain)
        } else if traits.contains(Traits::TCLOO_INTROSPECTION) {
            Some(MethodDispatchKind::Introspection)
        } else {
            None
        }
    }

    /// Whether a bracketed command substitution `[cmd ?arg?]` denotes the
    /// current `TclOO` receiving object — so a consumer resolving what a
    /// dispatch head (`[cmd ?arg?] method`) means should treat it exactly
    /// like `my method`: the enclosing class, not a structurally-inferred
    /// type. `arg` is the substitution's own first word, `None` for a bare
    /// call (`[self]` as opposed to `[self object]`).
    ///
    /// Registry data via [`CommandSpec::self_receiver_words`]
    /// (`self`/`object` today) rather than name-matching `cmd` — a
    /// `TCLOO_INTROSPECTION` command answers `Introspection` from
    /// [`Self::method_dispatch_keyword`] (its argument is a closed
    /// subcommand set, never a method name) for every *other* word, since
    /// this is a narrower, additional fact about specific words of that
    /// same closed set, not a fourth [`MethodDispatchKind`] axis: unlike
    /// `my`, the value only dispatches once *substituted* as a command
    /// head, and unlike plain introspection, this one specific word's
    /// result is the receiver itself.
    #[must_use]
    pub fn is_self_receiver_call(&self, cmd: &str, arg: Option<&str>) -> bool {
        let Some(spec) = self.spec_for_this_registry(cmd) else {
            return false;
        };
        if spec.self_receiver_words.is_empty() {
            return false;
        }
        match arg {
            None => spec.arity.min == 0,
            Some(word) => spec.self_receiver_words.contains(&word),
        }
    }

    /// Whether `head`'s **bare** spelling resolves only from inside a
    /// `TclOO` method context; see [`Traits::TCLOO_METHOD_CONTEXT`] for the oracle
    /// transcripts.
    ///
    /// Consumers pair this with their own "is this call site inside a
    /// `TclOO` method body?" fact
    /// (`tcl_compiler::analyser::scope::innermost_scope_reaches_oo_helpers`)
    /// — the registry knows *which* commands are scoped, the call site
    /// knows *where* it is, and neither needs the other's command names.
    ///
    /// Dialect-aware exactly like [`Self::method_dispatch_keyword`]: a
    /// registry built for a profile answers at that profile's point, so a
    /// `tcl8.4` registry — which has no `TclOO` at all — answers `false`
    /// for every one of them.
    ///
    /// A qualified spelling (`oo::Helpers::link`, `::oo::Helpers::link`) is
    /// a separate, unscoped spec, so it answers `false`: the command really
    /// does exist globally, and calling it outside a method is a runtime
    /// error rather than an unknown command.
    #[must_use]
    pub fn resolves_only_in_method_context(&self, head: &str) -> bool {
        self.spec_for_this_registry(head)
            .is_some_and(|spec| spec.traits.contains(Traits::TCLOO_METHOD_CONTEXT))
    }

    /// Whether **calling** `head` needs a real `TclOO` method invocation,
    /// not merely a frame that can resolve it — see
    /// [`Traits::TCLOO_REQUIRES_METHOD_FRAME`] for the oracle transcript.
    ///
    /// The narrower companion to [`Self::resolves_only_in_method_context`],
    /// and the two answer differently in exactly one place: a Tcl 9 class
    /// `initialise` / `initialize` body, where the whole family resolves
    /// (so `W123` must stay silent) but only `my` actually runs (so
    /// completion and hover must offer only `my`).
    ///
    /// A consumer deciding "may I offer this word here / does it hover"
    /// wants **both**: the word must resolve at the call site *and*, when
    /// this answers `true`, the call site must be a method frame rather
    /// than a bare object frame. `tcl_lsp_core::oo_dispatch` pairs them
    /// once so no consumer re-derives the rule.
    ///
    /// Dialect-aware exactly like [`Self::resolves_only_in_method_context`].
    #[must_use]
    pub fn requires_oo_method_frame(&self, head: &str) -> bool {
        self.spec_for_this_registry(head)
            .is_some_and(|spec| spec.traits.contains(Traits::TCLOO_REQUIRES_METHOD_FRAME))
    }

    /// Resolve `word` against the registry-declared universal object-command
    /// surface.
    ///
    /// `TclOO` receiver commands are runtime values, so their source head is not
    /// itself a static registry command.  This query gives common consumers
    /// the inherited method metadata without encoding the registry entry that
    /// owns that surface.  A dialect may supply at most one such surface; an
    /// accidental duplicate is treated conservatively as no unique answer.
    #[must_use]
    pub fn object_command_method(&self, word: &str) -> Option<&'static SubCommand> {
        let surfaces = self.query_surfaces();
        let mut matches = surfaces
            .object_command_surfaces
            .iter()
            .filter_map(|spec| spec.resolve_subcommand(word));
        let method = matches.next()?;
        matches.next().is_none().then_some(method)
    }

    /// Whether `word` is a destructive operation on the universal object
    /// command surface.
    #[must_use]
    pub fn is_destructive_object_method(&self, word: &str) -> bool {
        self.object_command_method(word)
            .is_some_and(|method| method.destructive)
    }

    /// Whether `word` is a **manufacturer method** of any definer family the
    /// registry models — `create` / `new` / `createWithNamespace` for
    /// `TclOO`, `create` for snit.
    ///
    /// The union over every definer grammar, for the one consumer that has a
    /// class name but not the family it belongs to: a *pure consumer*
    /// document holding `set w [Widget create x]` where `Widget` is declared
    /// in another file. A consumer that knows the family must
    /// ask its grammar
    /// ([`crate::definer::DefinitionBodyGrammar::manufacturer`]) instead —
    /// that answer is exact, this one is a union.
    #[must_use]
    pub fn is_manufacturer_method(&self, word: &str) -> bool {
        self.manufacturer_methods(word)
            .any(|method| method.visibility == crate::definer::MemberVisibility::Exported)
    }

    /// Every definer family's declaration of the manufacturer method `word`
    /// — the iterator behind [`Self::is_manufacturer_method`], for a consumer
    /// that needs the layout (which argument names the instance) and not just
    /// the yes/no.
    pub fn manufacturer_methods<'a>(
        &'a self,
        word: &'a str,
    ) -> impl Iterator<Item = &'static crate::definer::ManufacturerMethod> + 'a {
        let definers = self.query_surfaces().definers.clone();
        definers.into_iter().filter_map(move |spec| {
            spec.manufacturer_methods
                .iter()
                .find(|method| method.keyword == word)
                .or_else(|| {
                    spec.definition_body
                        .and_then(|grammar| grammar.manufacturer(word))
                })
        })
    }

    /// The exact manufacturer descriptor for registered command `head` and
    /// method `word`. This covers both definition-body commands and package
    /// class factories carrying standalone manufacturer data.
    #[must_use]
    pub fn manufacturer_method(
        &self,
        head: &str,
        word: &str,
    ) -> Option<&'static crate::definer::ManufacturerMethod> {
        Self::manufacturer_method_for_descriptor(self.get(head)?, word)
    }

    /// Manufacturer vocabulary of one independently selected descriptor.
    /// The caller retains availability and original invocation ownership.
    /// A command's own manufacturer rows take precedence over the defaults
    /// its definition grammar assigns to manufactured classes. This descriptor
    /// supplies layout and default visibility, never Native entry or execution.
    #[must_use]
    pub fn manufacturer_method_for_descriptor(
        spec: &CommandSpec,
        word: &str,
    ) -> Option<&'static crate::definer::ManufacturerMethod> {
        if !spec.manufacturer_methods.is_empty() {
            return spec
                .manufacturer_methods
                .iter()
                .find(|method| method.keyword == word);
        }
        spec.definition_body
            .and_then(|grammar| grammar.manufacturer(word))
    }

    /// The exact manufacturer descriptor when ordinary external dispatch
    /// through registered command `head` may reach it.
    ///
    /// This is the call-classification counterpart of
    /// [`Self::manufacturer_method`]. Consumers examining a real command
    /// invocation should normally use this method; definition-body analysis
    /// that models `self export` / `self unexport` needs the unfiltered
    /// descriptor and applies the class-local visibility changes itself.
    #[must_use]
    pub fn exported_manufacturer_method(
        &self,
        head: &str,
        word: &str,
    ) -> Option<&'static crate::definer::ManufacturerMethod> {
        self.manufacturer_method(head, word)
            .filter(|method| method.visibility == crate::definer::MemberVisibility::Exported)
    }

    /// Definition grammar entered by an outer literal script form.
    /// Configuration operations select their target from the typed transition
    /// owner: an immediately following script body is a definition region,
    /// while a later method body has ordinary Tcl command context. This is
    /// structural assistance, without target existence or runtime admission.
    #[must_use]
    pub fn outer_definition_body_grammar(
        &self,
        command: &str,
        args: &[&str],
    ) -> Option<&'static DefinitionBodyGrammar> {
        let grammar = self.get(command)?.definition_body?;
        let invocation = self.resolve_invocation(command, args, None)?;
        let transitions = invocation.state_transitions();
        let target = transitions
            .facts()
            .iter()
            .find_map(|fact| match &fact.transition {
                crate::StateTransition::ObjectDispatch(
                    crate::ObjectDispatchTransition::Configure { target, .. },
                ) => target.argument_index(),
                _ => None,
            });
        if let Some(target) = target {
            return self
                .arg_indices_for_role(command, args, ArgRole::Body)
                .contains(&target.checked_add(1)?)
                .then_some(grammar);
        }
        Some(grammar)
    }

    /// Nested source vocabulary of an admitted member of this store's selected
    /// closed document grammar. The current and nested grammars must share its
    /// actual family; ordinary command execution and state transitions are not
    /// premises for a document declaration. This supplies source grammar only,
    /// without runtime dispatch, Native naming or editing authority.
    #[must_use]
    pub fn authored_document_member_grammar(
        &self,
        current: &DefinitionBodyGrammar,
        keyword: &str,
    ) -> Option<&'static DefinitionBodyGrammar> {
        self.authored_document_member_descriptor(current, keyword)?
            .definition_body
    }

    /// Descriptor reborrow through this Registry's actual closed document
    /// member relation. This grants source vocabulary, never global lookup.
    #[must_use]
    pub fn authored_document_member_descriptor(
        &self,
        current: &DefinitionBodyGrammar,
        keyword: &str,
    ) -> Option<&'static CommandSpec> {
        let document = self.document_grammar()?;
        if current.family != document.family || !current.is_member(keyword) {
            return None;
        }
        let descriptor = self.get(keyword)?;
        let nested = descriptor.definition_body?;
        (nested.family == document.family).then_some(descriptor)
    }

    /// The first constructor-payload argument for manufacturer `word`, when
    /// every definer family declaring that word agrees on the layout.
    ///
    /// Consumers that know the exact family should query its grammar
    /// directly. Scope-blind flow analyses use this conservative union: a
    /// future family reusing a keyword with a different layout makes the
    /// answer `None`, so parameter flow abstains instead of skipping the
    /// wrong structural words.
    #[must_use]
    pub fn uniform_manufacturer_constructor_args_from(&self, word: &str) -> Option<usize> {
        let mut methods = self.manufacturer_methods(word);
        let first = usize::from(methods.next()?.constructor_args_from);
        methods
            .all(|method| usize::from(method.constructor_args_from) == first)
            .then_some(first)
    }

    /// The instance-name argument for manufacturer `word`, when every
    /// definer family declaring that word agrees on the layout.
    ///
    /// `None` is deliberately ambiguous: the method may generate its own
    /// name, or different families may use different layouts. Consumers
    /// that need to distinguish those cases should use
    /// [`Self::manufacturer_methods`] and require agreement themselves.
    #[must_use]
    pub fn uniform_manufacturer_names_instance_at(&self, word: &str) -> Option<usize> {
        let mut methods = self.manufacturer_methods(word);
        let first = methods.next()?.names_instance_at?;
        methods
            .all(|method| method.names_instance_at == Some(first))
            .then_some(usize::from(first))
    }

    /// Whether `word` can conservatively identify a constructor call when a
    /// consumer knows only that the head names some class, not which definer
    /// family created it.
    ///
    /// Named manufacturer methods and family-specific bare-word naming hints
    /// are both registry data. Exact class-aware consumers should use the
    /// class grammar instead of this union.
    #[must_use]
    pub fn is_possible_class_construction_word(&self, word: &str) -> bool {
        self.is_manufacturer_method(word)
            || self.query_surfaces().definers.iter().any(|spec| {
                spec.definition_body.is_some_and(|grammar| {
                    match grammar.bare_word_construction_source_hint {
                        Some(hint) => hint.recognises(word),
                        None => grammar
                            .bare_word_construction_hint
                            .is_some_and(|recognises| recognises(word)),
                    }
                })
            })
    }

    /// Whether `head` binds bareword aliases for methods of the current
    /// object — `TclOO`'s `link`; see [`Traits::TCLOO_BINDS_METHOD_ALIAS`].
    ///
    /// The registry-first replacement for a `texts[0] == "link"` literal an
    /// analyser's class-body walk would otherwise carry, and dialect-aware
    /// for the same reason [`Self::method_dispatch_keyword`]
    /// is: `link` is 9.0-core / 8.6-via-`ooutil`, so an 8.5 registry answers
    /// `false`.
    #[must_use]
    pub fn binds_method_alias(&self, head: &str) -> bool {
        self.spec_for_this_registry(head)
            .is_some_and(|spec| spec.traits.contains(Traits::TCLOO_BINDS_METHOD_ALIAS))
    }

    /// Audited native root factory whose default construction has no user
    /// hook graph. A document binding, authored override or arbitrary `TclOO`
    /// metaclass cannot inherit this admission by sharing a grammar.
    #[must_use]
    pub fn default_construction_grammar(
        &self,
        command: &str,
    ) -> Option<&'static crate::definer::DefinitionBodyGrammar> {
        self.selected_default_construction_grammar(command, self.own_surface_query())
    }

    /// Native root-factory descriptor under the retained invocation axes.
    /// This selects metadata only; live factory, allocation and effect
    /// dependencies must independently close before construction is inferred.
    #[must_use]
    pub fn native_default_construction_grammar(
        &self,
        command: &str,
        dialect: tcl_native_model::InvocationDialect,
        realm: tcl_dialect::model::InvocationRealm,
    ) -> Option<&'static crate::definer::DefinitionBodyGrammar> {
        if dialect.family() != Some(tcl_dialect::model::Family::Tcl)
            || dialect.tcl_version.is_none()
        {
            return None;
        }
        self.selected_default_construction_grammar(
            command,
            Some(dialect.authoring_query()?.with_realm(realm)),
        )
    }

    /// Original stock class-factory constructor/bootstrap recipe under the
    /// actual retained invocation axes. Current command/support allocations,
    /// body execution and successful completion are independent obligations.
    #[must_use]
    pub fn native_class_factory_recipe(
        &self,
        command: &str,
        dialect: tcl_native_model::InvocationDialect,
        realm: tcl_dialect::model::InvocationRealm,
    ) -> Option<crate::native_tcloo_bootstrap::NativeClassFactoryRecipe> {
        let spec =
            self.get_for_surface(command, Some(dialect.authoring_query()?.with_realm(realm)))?;
        if spec_pack_of(spec) != Some("tcl")
            || !spec.traits.contains(Traits::IS_OO_METACLASS)
            || spec.definition_body.is_none()
        {
            return None;
        }
        crate::native_tcloo_bootstrap::NativeClassFactoryRecipe::select(spec.name, dialect)
    }

    fn selected_default_construction_grammar(
        &self,
        command: &str,
        query: Option<SurfaceQuery<'_>>,
    ) -> Option<&'static crate::definer::DefinitionBodyGrammar> {
        let spec = self.get_for_surface(command, query)?;
        if spec.name != "oo::class" || spec_pack_of(spec) != Some("tcl") {
            return None;
        }
        spec.definition_body
    }

    /// Every instance method some registered metaclass **generates** from a
    /// class's declared `property` members — the union of
    /// [`DefinitionBodyGrammar::property_accessor_methods`] over the
    /// metaclasses that configure by property
    /// ([`Traits::CONFIGURES_BY_PROPERTY`]).
    ///
    /// The dialect-wide answer, for a consumer holding a class whose own
    /// metaclass it could not resolve to a grammar — a user metaclass derived
    /// from `oo::configurable` — but which demonstrably declares `property`
    /// members. Prefer the class's own grammar
    /// (`CommandSpec::definition_body`) whenever it resolves; this is the
    /// fallback that keeps such a class from being told its generated
    /// accessor does not exist, without any consumer spelling
    /// `configure` itself.
    ///
    /// Sorted and deduplicated. Empty for a dialect with no such metaclass
    /// (every pre-9.0 Tcl, where `oo::configurable` does not exist).
    ///
    /// [`DefinitionBodyGrammar::property_accessor_methods`]: crate::definer::DefinitionBodyGrammar::property_accessor_methods
    #[must_use]
    pub fn property_accessor_methods(&self) -> Vec<&'static str> {
        let mut out: Vec<&'static str> = self
            .commands_with_trait(Traits::CONFIGURES_BY_PROPERTY)
            .into_iter()
            .filter_map(|name| self.get(name))
            .filter_map(|spec| spec.definition_body)
            .flat_map(|grammar| grammar.property_accessor_methods.iter().copied())
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// How a call to `name` binds a variable to an **object handle**, when it
    /// does — [`CommandSpec::binds_handle`], resolved through [`Self::get`] so
    /// the explicitly global spelling (`::set`) answers identically to the
    /// bare one.
    ///
    /// The member-body-only installers a class system injects (snit's
    /// `install NAME using TYPE …`) are **not** here: they are not global
    /// commands, and live on
    /// [`crate::definer::DefinitionBodyGrammar::member_body_commands`] —
    /// enumerate them with [`Self::member_body_handle_bindings`].
    ///
    /// [`CommandSpec::binds_handle`]: crate::spec::CommandSpec::binds_handle
    #[must_use]
    pub fn handle_binding(
        &self,
        name: &str,
    ) -> Option<&'static crate::handle_binding::HandleBindingSpec> {
        self.get(name).and_then(|spec| spec.binds_handle)
    }

    /// Every member-body command that binds an object handle, as
    /// `(word, layout)` pairs collected from the definition-body grammars this
    /// registry's definers carry.
    ///
    /// These words exist only inside a class system's member bodies (snit's
    /// `install`), so they deliberately have no global `CommandSpec` — a
    /// consumer builds a small lookup from this list once per document instead
    /// of naming the keyword.  Deduplicated: the snit `type` and `widget`
    /// grammars share one member-body command set.
    #[must_use]
    pub fn member_body_handle_bindings(
        &self,
    ) -> Vec<(&'static str, crate::handle_binding::HandleBindingSpec)> {
        let mut out: Vec<_> = self
            .by_name
            .values()
            .filter_map(|specs| specs.last())
            .filter_map(|spec| spec.definition_body)
            .flat_map(|grammar| grammar.member_body_commands.iter())
            .filter_map(|cmd| cmd.binds_handle.map(|layout| (cmd.name, layout)))
            .collect();
        out.sort_unstable_by_key(|(name, _)| *name);
        out.dedup_by_key(|(name, _)| *name);
        out
    }
}

/// Which `TclOO` method-context keyword a command word is — the answer
/// [`CommandRegistry::method_dispatch_keyword`] returns.
///
/// The three variants are three different axes, deliberately not merged: a
/// consumer that wants "does the next word name a method" wants
/// [`Self::SelfDispatch`] alone, and one that wants "is this a call into the
/// method chain" wants `SelfDispatch | NextChain` but never
/// [`Self::Introspection`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MethodDispatchKind {
    /// `my` — dispatches a method on the current object; the following word
    /// is a method name on the enclosing class
    /// ([`Traits::TCLOO_SELF_DISPATCH`]).
    SelfDispatch,
    /// `next` / `nextto` — invokes the next implementation of the
    /// *currently executing* method along the receiver's method resolution
    /// order, so no word names a method. `nextto`'s first word names the
    /// *class* to resume from, marked [`ArgRole::Name`] on the spec
    /// ([`Traits::TCLOO_NEXT_CHAIN`]).
    NextChain,
    /// `self` — introspects the current invocation and dispatches nothing;
    /// its argument is a closed subcommand set, never a method name
    /// ([`Traits::TCLOO_INTROSPECTION`]).
    Introspection,
}
