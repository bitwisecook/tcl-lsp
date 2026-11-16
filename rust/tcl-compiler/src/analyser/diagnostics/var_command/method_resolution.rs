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

//! Whether a method is callable on a class through its hierarchy, and whether a call's argument count fits it.

use crate::analyser::state::Analyser;

impl Analyser {
    /// Hold a proved `$obj method …` dispatch's argument count to the
    /// providing class's own method definition; a method binds its
    /// arguments exactly as a procedure does (tclsh 8.6.18 and 9.0.4:
    /// `wrong # args: should be "::oo::Obj12 bar x y"`).
    ///
    /// A `forward NAME TARGET ?ARG…?` takes its target's arity shifted by
    /// the prepended arguments. `forward NAME my OTHER ?ARG…?` re-dispatches
    /// on the same instance, so `OTHER` resolves through the receiver's
    /// method resolution order; a bare method name is not a command and is
    /// left alone, as is a target neither this file nor the registry
    /// describes.
    pub(super) fn method_arity_diagnostic(
        &self,
        site: &crate::analyser::state::VarCommandSite,
        method_name: &str,
        receiver_class: &str,
        hierarchy: &super::super::class_hierarchy::ClassHierarchy,
    ) -> Option<super::super::types::Diagnostic> {
        const MAX_HOPS: u8 = 8;
        if site.has_expand {
            return None;
        }
        let providing_class = hierarchy.method_target(receiver_class, method_name)?;
        let class_def = self.result.all_classes.get(providing_class)?;
        // Only an instance method is reachable from an instance: a class
        // method is called on the class object.
        let mut method_def = class_def.methods.get(method_name)?;
        let mut prepended_total: u16 = 0;
        let mut arity = None;
        for _ in 0..MAX_HOPS {
            let Some((target, prepended)) = method_def.forward_target.as_ref() else {
                // A forward whose target could not be read has no body of its
                // own, and so no formals to hold the call to.
                if method_def.body_span == method_def.name_span {
                    return None;
                }
                arity = Some(method_def.arity());
                break;
            };
            if self.registry.as_deref().is_some_and(|registry| {
                registry.method_dispatch_keyword(target)
                    == Some(tcl_registry::MethodDispatchKind::SelfDispatch)
            }) {
                let (next_method, rest) = prepended.split_first()?;
                let next_provider = hierarchy.method_target(receiver_class, next_method)?;
                method_def = self
                    .result
                    .all_classes
                    .get(next_provider)
                    .and_then(|class| class.methods.get(next_method))?;
                prepended_total =
                    prepended_total.saturating_add(u16::try_from(rest.len()).unwrap_or(u16::MAX));
                continue;
            }
            prepended_total =
                prepended_total.saturating_add(u16::try_from(prepended.len()).unwrap_or(u16::MAX));
            if !self.command_slot_is_present(target, site.cmd_span.start()) {
                return None;
            }
            arity = self
                .result
                .all_procs
                .get(target)
                .or_else(|| {
                    self.result
                        .all_procs
                        .get(&tcl_syntax::naming::root_unrooted_key(target))
                })
                .map(super::super::types::ProcDef::arity)
                .or_else(|| {
                    if tcl_syntax::naming::is_qualified(target.as_bytes()) {
                        return None;
                    }
                    self.registry
                        .as_deref()
                        .and_then(|registry| registry.get(target))
                        .map(|spec| spec.arity)
                });
            break;
        }
        let arity = super::super::validity::shift_arity(arity?, prepended_total);
        // `argc` counts the method-name word too.
        let nargs = site.argc.saturating_sub(1);
        super::super::validity::arity_verdict(
            &format!("{} {method_name}", site.var_name),
            arity,
            nargs,
            false,
            site.cmd_span,
            None,
            None,
        )
    }

    /// Resolve a possibly-bare class name to its fully-qualified form keyed
    /// in `result.all_classes` — the shared call-site resolver
    /// ([`super::super::class_hierarchy::resolve_written_class_name`]), so this
    /// keying can never diverge from the LSP's. Missing source identity withdraws.
    pub(super) fn canonicalise_class_name(&self, name: &str) -> Option<String> {
        super::super::class_hierarchy::resolve_written_class_name(name, &self.result.all_classes)
    }

    /// Decide whether `method` is callable on `class_name` through a
    /// dispatch of the given `reach`, consulting the class hierarchy, the
    /// class's local method tables, and the class system's own built-in
    /// object methods.
    ///
    /// A method is OK when the class's MRO produces a concrete provider, or
    /// the class is external (no local `ClassDef`), or the method is one the
    /// *class system itself* supplies (see
    /// [`Self::builtin_object_method_reachable`]), or the class declares an
    /// `unknown` method, or the class extends or mixes in an external class
    /// we can't introspect.
    ///
    /// `reach` matters only for the built-in set, and it is the difference
    /// between a true positive and a false one: `my variable v` reaches
    /// `oo::object`'s unexported `variable`, while `$obj variable v` and
    /// `[self] variable v` do not and really are errors.
    pub(super) fn validate_method_on_class(
        &self,
        class_name: &str,
        method: &str,
        cd: Option<&super::super::types::ClassDef>,
        hierarchy: Option<&super::super::class_hierarchy::ClassHierarchy>,
        reach: tcl_registry::definer::MethodReach,
    ) -> bool {
        if hierarchy.is_some_and(|h| h.method_target(class_name, method).is_some()) {
            return true;
        }
        let Some(cd) = cd else {
            // External class — can't validate.
            return true;
        };
        if cd.methods.contains_key(method) || cd.class_methods.contains_key(method) {
            return true;
        }
        if self.builtin_object_method_reachable(cd, method, reach) {
            return true;
        }
        // A method the class system generates from declared properties —
        // written by no `method` body, so neither the member tables nor the
        // MRO above can see it.
        if hierarchy
            .is_some_and(|h| h.is_property_accessor(self.registry.as_deref(), class_name, method))
        {
            return true;
        }
        if let Some(fallback) = self
            .class_definer_grammar(class_name)
            .and_then(|grammar| grammar.unknown_dispatch_method)
        {
            if cd.methods.contains_key(fallback) {
                return true;
            }
            if hierarchy.is_some_and(|h| h.method_target(class_name, fallback).is_some()) {
                return true;
            }
        }
        if self
            .class_definer_grammar(class_name)
            .is_some_and(|grammar| grammar.dynamic_method_dispatch)
        {
            return true;
        }
        // Unknowable method set (external base, opaque inheritance, or
        // reflectively-installed members) ⇒ skip W308.
        self.method_set_unknowable(cd)
    }

    /// Whether `method` is one `cd`'s **class system** gives every object,
    /// reachable by a dispatch of this `reach`.
    ///
    /// Registry data, not a name list here: the class's recorded metaclass
    /// is itself a definer command, whose
    /// [`DefinitionBodyGrammar`](tcl_registry::definer::DefinitionBodyGrammar)
    /// carries `builtin_object_methods` — so `TclOO` gets `oo::object`'s
    /// inherited surface, snit gets `configure` / `cget` / `info`, itcl gets
    /// its own, and a class system the registry gains later is covered
    /// without touching this function.
    ///
    /// # Deliberate abstentions
    ///
    /// * **Receiver kind is not filtered.** `new` / `create` are declared
    ///   [`BuiltinMethodReceiver::ClassObject`](tcl_registry::definer::BuiltinMethodReceiver::ClassObject)
    ///   and really are absent from an instance (tclsh 9.0.4: `[C new] new`
    ///   → `unknown method "new"`), but a receiver here may equally be a
    ///   *class* command held in a variable (`set cls C; $cls new`), which
    ///   the analyser cannot always tell apart from one of its instances.
    ///   Accepting both is the abstention; narrowing it needs receiver-kind
    ///   facts this pass does not have.
    /// * **`oo::configurable`'s `configure` is not in the shared `TclOO`
    ///   grammar**, because it is contributed by a specific metaclass rather
    ///   than by `oo::object` — every `TclOO` metaclass shares the one
    ///   grammar, so listing it here would hand it to plain `oo::class`
    ///   instances too, which really do fail with `unknown method
    ///   "configure"`. It rides on `oo::configurable`'s own
    ///   `TCLOO_CONFIGURABLE_GRAMMAR` instead, and is settled one level up in
    ///   [`Self::validate_method_on_class`] via
    ///   [`ClassHierarchy::is_property_accessor`](super::super::class_hierarchy::ClassHierarchy::is_property_accessor),
    ///   which asks the *metaclass command's* registry traits; a class that
    ///   inherits from an *unindexed* configurable base is covered by
    ///   [`Self::method_set_unknowable`].
    fn builtin_object_method_reachable(
        &self,
        cd: &super::super::types::ClassDef,
        method: &str,
        reach: tcl_registry::definer::MethodReach,
    ) -> bool {
        self.class_definer_grammar(&cd.qualified_name)
            .is_some_and(|grammar| grammar.builtin_object_method(method, reach).is_some())
    }

    /// True when `cd`'s callable method set cannot be enumerated from what
    /// the analyser recorded, so W308 must abstain rather than call a method
    /// missing.  Three independent reasons, all "the tables are a lower
    /// bound", never "this method does not exist":
    ///
    /// * a superclass **or mixin** outside the local class index (the
    ///   `TclOO` bases in [`OO_BASE`] excepted) — `TclOO` mixins contribute
    ///   to the MRO exactly as superclasses do, so an unresolvable name in
    ///   either list hides an unknown set of inherited methods;
    /// * [`ClassDef::inheritance_unknown`] — manufactured by a user
    ///   metaclass whose `create` override could not be read, so the
    ///   spliced superclass list itself is unknown;
    /// * [`ClassDef::member_set_incomplete`] — the class's own body installs
    ///   members reflectively.
    ///
    /// Used by the class-definition assistance path; source-owned instance
    /// receipts retain their independent method-inventory completeness.
    ///
    /// [`ClassDef::inheritance_unknown`]: super::super::types::ClassDef::inheritance_unknown
    /// [`ClassDef::member_set_incomplete`]: super::super::types::ClassDef::member_set_incomplete
    pub(super) fn method_set_unknowable(&self, cd: &super::super::types::ClassDef) -> bool {
        // A class manufactured by a user-defined metaclass whose `create`
        // override could not be read has an unknown superclass list, which is
        // the same situation as a superclass outside the index: a method it
        // inherits is not a method it is missing.
        //
        // A class whose own body installs members through reflection
        // (`constructor {*}[info class constructor ::Base]`, `foreach m {…}
        // { method $m … }`) is the same judgement one level in: the recorded
        // member tables are a lower bound, so a method that is absent from
        // them is not thereby missing from the class.
        cd.inheritance_unknown
            || cd.member_set_incomplete
            || cd
                .superclasses
                .iter()
                .chain(&cd.mixins)
                .any(|s| !self.result.all_classes.contains_key(s) && !OO_BASE.contains(&s.as_str()))
    }
}

/// External OO base classes that aren't in the per-document
/// ``ClassDef`` index but are recognised as legitimate
/// superclasses / mixins for W308 / W308-related gates.
const OO_BASE: [&str; 2] = ["oo::object", "oo::class"];
