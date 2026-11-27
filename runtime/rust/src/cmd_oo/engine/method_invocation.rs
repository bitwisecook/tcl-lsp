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

//! Invoking a method on an object: export visibility, filters, the method chain, and the unknown-method error.

use super::super::introspection::{original_core_report_step, retain_original_terminal_report};
use super::super::{
    method_invocation, native_context, native_method_cache, wrong_args, CallStep, MethodDispatch,
    OriginalOoInvocationView, MAX_MRO_VISITS,
};
use crate::interp::{Code, Interp};
use crate::obj;
use crate::obj::TclObj;
use std::collections::BTreeSet;
use std::rc::Rc;
use tcl_core_types::OoId;
use tcl_native_model::native_tcloo_method_definition::NativeTclooMethodReportRole;

impl Interp {
    /// `unknown method "X": must be <public methods + built-ins>` — the C TclOO
    /// error, listing the callable methods (sorted, `a, b or c` style).
    pub(in super::super) fn oo_unknown_method(&mut self, obj: OoId, requested: &[u8]) -> Code {
        // `destroy` is a real (overridable, unexportable) method on every
        // object; it is visible unless unexported anywhere in the method chain
        // (e.g. `::oo::Slot`) without a re-export.
        let (destroy_exp, destroy_unexp, _) = self.method_visibility_flags(obj, b"destroy");
        let destroy_hidden = destroy_unexp && !destroy_exp;
        let mut names: BTreeSet<Vec<u8>> = BTreeSet::new();
        if self.oo.borrow().classes.contains_key(&obj) {
            // A class object responds to the class built-ins. `::oo::class`
            // is a singleton with `new` unexported, so it lists only `create`.
            // A class that unexports its own `create`/`new` (e.g. a metaclass
            // with a custom factory) hides them from an external miss.
            let ext = self.oo.borrow().unknown_external;
            let cre_unexp = self.factory_export_record(obj, b"create") == Some(false);
            let new_unexp = self.factory_export_record(obj, b"new") == Some(false);
            if !(ext && cre_unexp) {
                names.insert(b"create".to_vec());
            }
            if Some(obj) != self.oo.borrow().class_root && !(ext && new_unexp) {
                names.insert(b"new".to_vec());
            }
        }
        if !destroy_hidden {
            names.insert(b"destroy".to_vec());
        }
        // The miss's origin: an internal (`my m`) call also lists unexported
        // methods and the `oo::object` built-ins; an external one lists only
        // public (exported) methods.
        let external = self.oo.borrow().unknown_external;
        // An internal call reaches the unexported `oo::object` built-ins too.
        if !external {
            for b in [
                &b"<cloned>"[..],
                b"eval",
                b"unknown",
                b"variable",
                b"varname",
            ] {
                names.insert(b.to_vec());
            }
        }
        // Collect method names from each provider: public always; unexported
        // (non-private) only for an internal call; explicitly-exported names
        // (a promoted built-in) always.
        let collect = |methods: &native_method_cache::MethodTable,
                       unexp: &BTreeSet<Vec<u8>>,
                       exp: &BTreeSet<Vec<u8>>,
                       priv_set: &BTreeSet<Vec<u8>>,
                       names: &mut BTreeSet<Vec<u8>>| {
            for m in methods.keys() {
                if priv_set.contains(m) {
                    continue; // private: handled separately (scope-gated)
                }
                if !unexp.contains(m) || !external {
                    names.insert(m.clone());
                }
            }
            // An explicitly-exported name is listed only if it is a real method
            // or a promotable built-in — not a name merely `export`ed without a
            // backing implementation (oo-4.3).
            const PROMOTABLE: &[&[u8]] =
                &[b"eval", b"variable", b"varname", b"unknown", b"<cloned>"];
            for m in exp {
                if methods.contains_key(m) || PROMOTABLE.contains(&m.as_slice()) {
                    names.insert(m.clone());
                }
            }
        };
        // A private method named is visible (and so listed) only from its own
        // declaring scope — the scope captured at the original invocation (the
        // `unknown` handler's own frame must not mask it; TIP 500). From
        // non-method code there is no scope, so none are listed.
        let caller_scope = self.oo.borrow().unknown_scope;
        for (p, is_object) in self.method_chain_faceted(obj) {
            let in_scope = caller_scope == Some((p, is_object));
            let oo = self.oo.borrow();
            if is_object {
                if let Some(o) = oo.objects.get(&p) {
                    collect(
                        &o.methods,
                        &o.unexported,
                        &o.exported,
                        &o.private,
                        &mut names,
                    );
                    if in_scope {
                        names.extend(o.private.iter().cloned());
                    }
                }
            } else if let Some(c) = oo.classes.get(&p) {
                collect(
                    &c.methods,
                    &c.unexported,
                    &c.exported,
                    &c.private,
                    &mut names,
                );
                if in_scope {
                    names.extend(c.private.iter().cloned());
                }
            }
        }
        let names: Vec<Vec<u8>> = names.into_iter().collect();
        if names.is_empty() {
            // C: an object with no callable methods reports differently.
            let mut msg = b"object \"".to_vec();
            msg.extend_from_slice(&self.oo_name(obj));
            msg.extend_from_slice(b"\" has no visible methods");
            return self.error(&msg);
        }
        let mut msg = b"unknown method \"".to_vec();
        msg.extend_from_slice(requested);
        msg.extend_from_slice(b"\": must be ");
        // TclOO's method-list style: no Oxford comma before the final `or`,
        // unlike the option tables. The rule lives in the shared owner.
        msg.extend_from_slice(&tcl_cmd_core::prefix::tcloo_choice_list_bytes(&names));
        self.error(&msg)
    }
}

/// `oo::object`'s built-in `unknown` method: the standard "unknown method"
/// error. It is the end of the `unknown` call chain, so a user `unknown` that
/// does `next` lands here. `args[0]` is the originally-requested method name.
/// With *no* args it was reached via a no-method invocation (`$obj`), whose
/// default outcome is the `wrong # args` usage (C's `FORCE_UNKNOWN` path).
pub(in super::super) fn oo_object_unknown(
    interp: &mut Interp,
    obj: OoId,
    args: &[*mut TclObj],
) -> Code {
    native_context::with_original_arguments(interp, args, |interp, originals| {
        let Some(_) = args.first() else {
            let mut u = interp.oo_name(obj);
            u.extend_from_slice(b" method ?arg ...?");
            return wrong_args(interp, &u);
        };
        let method = oo_original_bytes!(interp, originals, 0);
        interp.oo_unknown_method(obj, &method)
    })
}

impl Interp {
    /// Invoke method `method` on `obj`. `external` enforces export visibility.
    pub(in super::super) fn oo_invoke(
        &mut self,
        obj: OoId,
        method: &[u8],
        args: &[*mut TclObj],
        external: bool,
        invoked: Option<&[u8]>,
    ) -> Code {
        self.oo_invoke_with_original(obj, method, args, external, invoked, None)
    }

    pub(in super::super) fn oo_invoke_with_original(
        &mut self,
        obj: OoId,
        method: &[u8],
        args: &[*mut TclObj],
        external: bool,
        invoked: Option<&[u8]>,
        original_argv: Option<&[*mut TclObj]>,
    ) -> Code {
        self.oo_invoke_with_role(
            obj,
            method,
            args,
            MethodDispatch {
                external,
                unknown: false,
            },
            invoked,
            original_argv,
        )
    }

    fn oo_invoke_with_role(
        &mut self,
        obj: OoId,
        method: &[u8],
        args: &[*mut TclObj],
        dispatch: MethodDispatch,
        invoked: Option<&[u8]>,
        original_argv: Option<&[*mut TclObj]>,
    ) -> Code {
        if !self.oo.borrow().objects.contains_key(&obj) {
            return self.invalid_command(&self.oo_name(obj));
        }
        // A torn-down object (its destructor finished, structures dismantled, but
        // command still resolvable) has no call chain for anything — C's
        // `TclOOGetCallContext` returns NULL. This is what a nested-owned child's
        // destructor hits when it calls back into the parent mid-teardown
        // (oo-35.7.1/2, oo-11.8).
        if self
            .oo
            .borrow()
            .objects
            .get(&obj)
            .is_some_and(|o| o.torn_down)
        {
            let mut m = b"impossible to invoke method \"".to_vec();
            m.extend_from_slice(method);
            m.extend_from_slice(b"\": no defined method or unknown method");
            let mut code = b"TCL LOOKUP METHOD ".to_vec();
            code.extend_from_slice(method);
            return self.error_with_code(&m, &code);
        }
        let mut original_objects = original_argv.unwrap_or_default().to_vec();
        original_objects.extend_from_slice(args);
        native_context::with_original_arguments(self, &original_objects, |interp, originals| {
            interp.oo_invoke_original_entered(
                obj,
                method,
                args,
                dispatch,
                OriginalOoInvocationView {
                    invoked,
                    original_argv,
                },
                originals,
            )
        })
    }

    fn oo_invoke_original_entered(
        &mut self,
        obj: OoId,
        method: &[u8],
        args: &[*mut TclObj],
        dispatch: MethodDispatch,
        OriginalOoInvocationView {
            invoked,
            original_argv,
        }: OriginalOoInvocationView<'_>,
        originals: &crate::interp::native_arguments::OriginalArguments,
    ) -> Code {
        let MethodDispatch { external, unknown } = dispatch;
        let target_owner = oo_checked!(
            self,
            native_context::OriginalOoTarget::capture(self, obj, false)
        );
        // True-private scope is independent of public visibility. An external
        // same-object call still cannot select a merely unexported method.
        let caller_context = {
            let activation = self.frames.borrow().current_activation();
            self.oo
                .borrow()
                .call_stack
                .iter()
                .rev()
                .find(|frame| frame.activation == Some(activation))
                .and_then(|frame| {
                    frame
                        .chain
                        .get(frame.index)
                        .map(|step| (frame.object, step.provider, step.is_object))
                })
        };
        let caller_is_self = caller_context.is_some_and(|(caller, _, _)| caller == obj);
        let enforce = external;
        // Native class hierarchies containing true-private methods never stash a chain.
        let selected_original =
            if let Some(original) = original_argv.and_then(|words| words.get(1).copied()) {
                let bytes = oo_original_bytes!(self, originals, 1);
                oo_checked!(self, target_owner.check(self));
                (bytes == method).then_some(original)
            } else {
                None
            };
        let providers = self.method_chain_faceted(obj);
        let cache_original = selected_original
            .filter(|_| !self.oo.borrow().filter_handling)
            .filter(|_| {
                let state = self.oo.borrow();
                let caller_private = caller_is_self
                    && state
                        .objects
                        .get(&obj)
                        .is_some_and(|object| !object.private.is_empty());
                !caller_private
                    && !providers.iter().any(|(provider, object)| {
                        !object
                            && state
                                .classes
                                .get(provider)
                                .is_some_and(|class| !class.private.is_empty())
                    })
            });
        if let Some(original) = cache_original {
            let cached = self.lookup_native_method_chain(obj, method, original, external);
            oo_original_check!(self, originals);
            oo_checked!(self, target_owner.check(self));
            let cached = oo_checked!(self, cached);
            if let Some(chain) = cached {
                let invocation = oo_checked!(
                    self,
                    method_invocation(self, obj, invoked, method, args, external, original_argv)
                );
                oo_checked!(self, target_owner.check(self));
                let chain = retain_original_terminal_report(self, &providers, method, chain)
                    .with_unknown(unknown);
                return self.oo_run(obj, chain, 0, method, args, invocation);
            }
        }
        // An `export` of the method anywhere in the chain (e.g. on the object)
        // makes every step callable, overriding a class-level unexport.
        let exported_anywhere = self.method_exported(obj, method);
        // TIP 500: a TRUE_PRIVATE method is scoped to its declaring class/object.
        // The general call chain skips private methods (C's
        // `AddSimpleClassChainToCallContext`: `if (!IS_PRIVATE(mPtr))`); a
        // private step is added only when the caller's context scope — the
        // declaring entity of the currently-running method — is that same
        // provider. From non-method (external) code there is no scope, so all
        // private methods are invisible.
        let caller_scope = caller_context.and_then(|(caller, provider, is_object)| {
            (!is_object || caller == obj).then_some((provider, is_object))
        });
        // The target-method steps: every provider that defines `method`. For an
        // external call, skip steps the provider unexports (unless overridden by
        // an export) — so a public override still runs while a private one is
        // skipped.
        let mut steps: Vec<CallStep> = providers
            .iter()
            .filter(|(p, is_obj)| {
                let is_obj = *is_obj;
                if !self.oo_has_method(*p, method, is_obj) {
                    return false;
                }
                // A private method is in scope only from its own declarer.
                if self.method_is_private(*p, method, is_obj) {
                    return caller_scope == Some((*p, is_obj));
                }
                !(enforce && !exported_anywhere && self.method_unexported(*p, method, is_obj))
            })
            .map(|(p, is_obj)| CallStep {
                method_owner: None,
                provider: *p,
                method: method.to_vec(),
                is_object: *is_obj,
                is_filter: false,
                synthetic_type: None,
                declaring_facet: None,
            })
            .collect();
        // A caller-scope private shadows any public override further down the
        // chain, so move it to the front (it is the most-specific step).
        if let Some((c, is_obj)) = caller_scope {
            if self.oo_has_method(c, method, is_obj) && self.method_is_private(c, method, is_obj) {
                if let Some(pos) = steps.iter().position(|s| s.provider == c) {
                    let s = steps.remove(pos);
                    steps.insert(0, s);
                }
            }
        }
        if steps.is_empty() {
            // An active object/class filter wraps even a built-in target (e.g.
            // `destroy`, `eval`): C's filters sit ahead of the real method, which
            // for these is the `oo::object` built-in. Route the filter chain
            // through `oo_run`; the filter's `next` reaches the built-in terminus
            // in `next_cmd` (oo-12.2/12.3). Only when the built-in is reachable —
            // otherwise the call still misses and falls to the unknown handler.
            if external {
                let destroy_ok = method == b"destroy"
                    && (exported_anywhere || !self.method_visibility_flags(obj, b"destroy").1);
                let objbuiltin_ok =
                    matches!(method, b"eval" | b"variable" | b"varname") && exported_anywhere;
                if destroy_ok || objbuiltin_ok {
                    let filters = self.active_filters(obj, &providers);
                    if !filters.is_empty() {
                        let invocation = oo_checked!(
                            self,
                            method_invocation(
                                self,
                                obj,
                                invoked,
                                method,
                                args,
                                external,
                                original_argv,
                            )
                        );
                        oo_checked!(self, target_owner.check(self));
                        let filters = native_method_cache::retain_owners(
                            &self.oo.borrow(),
                            filters,
                            NativeTclooMethodReportRole::Named,
                        );
                        let root = self
                            .oo
                            .borrow()
                            .object_root
                            .expect("original object builtin provider");
                        let terminal = original_core_report_step(self, root, method)
                            .expect("selected object builtin report");
                        let filters = native_method_cache::ReachedMethodChain::from(filters)
                            .with_terminal_report(terminal);
                        return self.oo_run(obj, filters, 0, method, args, invocation);
                    }
                }
            }
            // `destroy` is a built-in (overridable, PUBLIC) method on every
            // object; with no user override in the chain it tears the object
            // down. Reachable both externally (`$obj destroy`) and internally
            // (`my destroy`). When a class unexports it (e.g. `::oo::Slot`), an
            // external call misses and falls through to the unknown handler.
            if method == b"destroy"
                && (!enforce
                    || exported_anywhere
                    || !self.method_visibility_flags(obj, b"destroy").1)
            {
                return self.oo_destroy(obj);
            }
            // The class instantiation built-ins (`create`/`new`/
            // `createWithNamespace`) are reachable here when `obj` is a class
            // — notably via an internal `my create …` from a metaclass method
            // (oo-7.4/7.5). External calls honour the class's export state.
            if let Some(code) = self.oo_class_factory(
                obj,
                &self.oo_name(obj),
                method,
                args,
                enforce && !exported_anywhere,
                original_argv,
            ) {
                return code;
            }
            // The object built-ins (`variable`/`varname`/`eval`) are not in any
            // method table; they are unexported by default (reachable internally
            // via `my`), but a public call reaches them too once explicitly
            // `export`ed.
            if !external || exported_anywhere {
                if let Some(code) =
                    self.oo_builtin_method(obj, method, args, external, original_argv)
                {
                    return code;
                }
            }
            // Record the originating scope for the unknown-method error so the
            // `unknown` handler's own frame does not mask it (restored after).
            let saved_scope = self.oo.borrow_mut().unknown_scope;
            self.oo.borrow_mut().unknown_scope = caller_scope;
            let saved_external = self.oo.borrow().unknown_external;
            self.oo.borrow_mut().unknown_external = external;
            // A user-defined `unknown` method handles the miss (the method name
            // is prepended to the args), e.g. `oo::Slot`'s default-operation.
            // An *external* call of a hidden `unknown` (e.g. `$slot unknown`)
            // also routes here; only the internal fallback invocation (where the
            // handler itself missed) bypasses it, so we never recurse.
            let code = if (method != b"unknown" || external)
                && providers
                    .iter()
                    .any(|(p, is_obj)| self.oo_has_method(*p, b"unknown", *is_obj))
            {
                // The missed selector is an original value, including slot
                // data whose String bytes are not its object identity.
                let head = selected_original.map_or_else(
                    || obj::Owned::fresh(obj::new_string_bytes(method)),
                    obj::Owned::retain,
                );
                let mut uargs: Vec<*mut TclObj> = vec![head.as_ptr()];
                for &a in args {
                    unsafe { obj::incr_ref_count(a) };
                    uargs.push(a);
                }
                // `unknown` is itself usually unexported, so dispatch it
                // internally (it is the object's own fallback handler).
                let code = self.oo_invoke_with_role(
                    obj,
                    b"unknown",
                    &uargs,
                    MethodDispatch {
                        external: false,
                        unknown: true,
                    },
                    None,
                    original_argv,
                );
                for &a in &uargs[1..] {
                    unsafe { obj::decr_ref_count(a) };
                }
                code
            } else {
                self.oo_unknown_method(obj, method)
            };
            self.oo.borrow_mut().unknown_scope = saved_scope;
            self.oo.borrow_mut().unknown_external = saved_external;
            return code;
        }
        steps = native_method_cache::retain_owners(
            &self.oo.borrow(),
            steps,
            NativeTclooMethodReportRole::Named,
        );
        // Filters wrap a method call (public or `my`) — prepend each active
        // filter as its own step — unless we're already handling a filter (C's
        // `FILTER_HANDLING`): a filter's own synchronous calls are not
        // re-wrapped, which avoids infinite recursion and matches the call-chain
        // model where the wrapped method (reached via `next`) re-enables filters
        // for *its* calls only (oo-12.x).
        if !self.oo.borrow().filter_handling {
            let filters = self.active_filters(obj, &providers);
            if !filters.is_empty() {
                let mut chain: Vec<CallStep> = native_method_cache::retain_owners(
                    &self.oo.borrow(),
                    filters,
                    NativeTclooMethodReportRole::Named,
                );
                chain.append(&mut steps);
                let invocation = oo_checked!(
                    self,
                    method_invocation(self, obj, invoked, method, args, external, original_argv)
                );
                let chain = match cache_original {
                    Some(original) => {
                        let result = self.remember_native_method_chain(
                            obj,
                            method,
                            original,
                            external,
                            Rc::new(chain),
                        );
                        oo_original_check!(self, originals);
                        oo_checked!(self, target_owner.check(self));
                        match result {
                            Ok(chain) => chain,
                            Err(error) => return self.report_cmd_error(error),
                        }
                    }
                    None => native_method_cache::ReachedMethodChain::from(chain),
                };
                oo_checked!(self, target_owner.check(self));
                let chain = retain_original_terminal_report(self, &providers, method, chain)
                    .with_unknown(unknown);
                return self.oo_run(obj, chain, 0, method, args, invocation);
            }
        }
        let steps = match cache_original {
            Some(original) => {
                let result = self.remember_native_method_chain(
                    obj,
                    method,
                    original,
                    external,
                    Rc::new(steps),
                );
                oo_original_check!(self, originals);
                oo_checked!(self, target_owner.check(self));
                match result {
                    Ok(chain) => chain,
                    Err(error) => return self.report_cmd_error(error),
                }
            }
            None => native_method_cache::ReachedMethodChain::from(steps),
        };
        let invocation = oo_checked!(
            self,
            method_invocation(self, obj, invoked, method, args, external, original_argv)
        );
        oo_checked!(self, target_owner.check(self));
        let steps =
            retain_original_terminal_report(self, &providers, method, steps).with_unknown(unknown);
        self.oo_run(obj, steps, 0, method, args, invocation)
    }

    /// The active filter steps for `obj` (object filters, then class filters
    /// along the MRO), each resolved to the chain provider defining it.
    fn active_filters(&self, obj: OoId, providers: &[(OoId, bool)]) -> Vec<CallStep> {
        let mut names: Vec<Vec<u8>> = Vec::new();
        let add = |n: &Vec<u8>, names: &mut Vec<Vec<u8>>| {
            if !names.contains(n) {
                names.push(n.clone());
            }
        };
        if let Some(o) = self.oo.borrow().objects.get(&obj) {
            for f in &o.filters {
                add(f, &mut names);
            }
        }
        for (p, _) in providers {
            if let Some(c) = self.oo.borrow().classes.get(p) {
                for f in &c.filters {
                    add(f, &mut names);
                }
            }
        }
        // Each filter name contributes a step for *every* provider in the
        // precedence that implements it (in precedence order), so a filter
        // method overridden along the chain wraps once per implementation —
        // e.g. `Emix.f` then `B.f` (oo-21.3).
        names
            .iter()
            .flat_map(|fname| {
                providers
                    .iter()
                    .filter(|(p, is_obj)| self.oo_has_method(*p, fname, *is_obj))
                    .map(|(p, is_obj)| CallStep {
                        method_owner: None,
                        provider: *p,
                        method: fname.clone(),
                        is_object: *is_obj,
                        is_filter: true,
                        synthetic_type: None,
                        declaring_facet: None,
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// The method-resolution (precedence) chain for `obj`: a depth-first walk —
    /// the object's mixins, then the object itself, then its class's
    /// linearisation (each class: its mixins, then the class, then its
    /// superclasses) — with each provider kept at its **last** occurrence. This
    /// "keep-last" dedup defers shared bases (a diamond's apex, a mixin's
    /// superclass shared with the class chain) to after everything that derives
    /// from them, matching C's `TclOOComputePrecedenceList` / call-chain order
    /// (oo-21.x C3 ordering, oo-14.x mixins).
    pub(in super::super) fn method_chain(&self, obj: OoId) -> Vec<OoId> {
        self.method_chain_faceted(obj)
            .into_iter()
            .map(|(p, _)| p)
            .collect()
    }

    /// Like [`method_chain`] but each provider is tagged with its facet
    /// (`is_object`): the object's own methods vs a class's instance methods.
    /// The object facet and a self-mixed class facet share the FQN but are
    /// distinct steps, so the dedup keeps each `(provider, facet)` pair (not
    /// just each name) — needed to dispatch a `self mixin <self>` method.
    pub(in super::super) fn method_chain_faceted(&self, obj: OoId) -> Vec<(OoId, bool)> {
        let (obj_mixins, cls) = {
            let oo = self.oo.borrow();
            match oo.objects.get(&obj) {
                Some(o) => (o.mixins.clone(), Some(o.class)),
                None => return vec![(obj, true)],
            }
        };
        let mut seq: Vec<(OoId, bool)> = Vec::new();
        let mut path: Vec<OoId> = Vec::new();
        // One visit budget shared across every mixin and the class chain below
        // (see `MAX_MRO_VISITS`): the cap is on *this whole dispatch's* total
        // linearisation work, not on each sub-walk independently.
        let mut budget = MAX_MRO_VISITS;
        // Mixins and the class hierarchy contribute *class*-facet steps; the
        // object itself sits between the object mixins and its class chain.
        for mx in &obj_mixins {
            let mut tmp: Vec<OoId> = Vec::new();
            self.linearize_class(*mx, &mut tmp, &mut path, 0, &mut budget);
            seq.extend(tmp.into_iter().map(|c| (c, false)));
        }
        seq.push((obj, true));
        if let Some(cls) = cls {
            let mut tmp: Vec<OoId> = Vec::new();
            self.linearize_class(cls, &mut tmp, &mut path, 0, &mut budget);
            seq.extend(tmp.into_iter().map(|c| (c, false)));
        }
        // Keep each (provider, facet) at its last occurrence (drop earlier dups).
        let mut chain: Vec<(OoId, bool)> = Vec::with_capacity(seq.len());
        for (i, c) in seq.iter().enumerate() {
            if !seq[i + 1..].iter().any(|x| x == c) {
                chain.push(*c);
            }
        }
        chain
    }
}
