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

//! Running one step of a call chain: the method frame, forwards, native handlers and the retirement of the call frame.

use super::super::{
    native_context, native_method_cache, native_properties, native_variables, oo_display,
    variable_mapping, DefTarget, Method, MethodInvocation, OoFrame,
};
use crate::interp::{CallMeta, Code, Interp, MethodFrameWhat, ProcFrame};
use crate::obj::TclObj;
use tcl_core_types::OoId;
use tcl_native_model::native_tcloo_method_definition::{
    NativeTclooMethodPayloadType, NativeTclooMethodReportRole,
};
use tcl_syntax::value::ValueOps;

impl Interp {
    /// Run the call-chain step at `index` on `obj`. `target` is the originally
    /// invoked method (empty for a constructor), recorded for `self target`.
    pub(in super::super) fn oo_run(
        &mut self,
        obj: OoId,
        chain: impl Into<native_method_cache::ReachedMethodChain>,
        index: usize,
        target: &[u8],
        args: &[*mut TclObj],
        invocation: MethodInvocation,
    ) -> Code {
        let scope =
            match crate::interp::native_operation_currency::NativeOperationScope::enter(self) {
                Ok(scope) => scope,
                Err(first) => {
                    self.refuse_native_execution(first.clone());
                    drop(chain);
                    drop(invocation);
                    return self.report_cmd_error(first.into());
                }
            };
        let reached = self.oo_run_entered(
            obj,
            chain.into(),
            index,
            target,
            args,
            (invocation, scope.currency()),
        );
        match scope.currency().ensure_current_or_refuse() {
            Ok(()) => reached,
            Err(first) => self.report_cmd_error(first.into()),
        }
    }

    /// Extract the actual active context before releasing its owning roles.
    /// Its caller retains this original currency through mandatory restoration.
    pub(super) fn retire_oo_call_frame(
        &mut self,
        currency: &crate::interp::native_operation_currency::NativeOperationCurrency,
    ) {
        let _ = currency.ensure_current_or_refuse();
        let retired = { self.oo.borrow_mut().call_stack.pop() };
        drop(retired);
        let _ = currency.ensure_current_or_refuse();
    }

    fn oo_run_entered(
        &mut self,
        obj: OoId,
        chain: native_method_cache::ReachedMethodChain,
        index: usize,
        target: &[u8],
        args: &[*mut TclObj],
        entered: (
            MethodInvocation,
            &crate::interp::native_operation_currency::NativeOperationCurrency,
        ),
    ) -> Code {
        let (invocation, currency) = entered;
        let report_role = chain.report_role();
        let MethodInvocation {
            external,
            level_words,
            original_argv,
        } = invocation;
        let prov = chain[index].provider;
        let method = chain[index].method.clone();
        // Object-vs-class facet of this step (a class mixed into its own instance
        // contributes both, so use the step's recorded facet, not `prov == obj`).
        let is_object = chain[index].is_object;
        // The `::oo::class` synthetic constructor: instantiating a metaclass
        // runs the (optional) definition-script argument on the new class. C's
        // `oo::class` constructor is `{{definitionScript ""}}`.
        if chain[index].synthetic_type == Some(NativeTclooMethodPayloadType::ClassConstructor) {
            return native_context::with_original_arguments(self, args, |interp, originals| {
                let owner = oo_checked!(
                    interp,
                    native_context::OriginalOoTarget::capture(interp, obj, true)
                );
                let body = if args.is_empty() {
                    Vec::new()
                } else {
                    oo_original_bytes!(interp, originals, 0)
                };
                oo_checked!(interp, owner.check(interp));
                if body.is_empty() {
                    interp.set_result_bytes(b"");
                    return Code::Ok;
                }
                interp.oo_define_body(DefTarget::Class(obj), args[0], None)
            });
        }
        // Reached steps use only their retained exact record; a removed name
        // cannot revive a declaration from a changed table.
        let m = chain[index]
            .method_owner
            .as_ref()
            .map(|owner| owner.borrow().clone());
        let Some(m) = m else {
            return self.error(b"no such method");
        };

        if let Method::Property(property) = &m {
            let name_owner = self.oo_name_owner(obj);
            self.oo.borrow_mut().call_stack.push(OoFrame {
                name_owner,
                activation: None,
                object: obj,
                chain,
                index,
                target: target.to_vec(),
                external,
            });
            let code = native_properties::invoke_default(
                self,
                obj,
                property,
                args,
                original_argv.as_deref(),
            );
            self.retire_oo_call_frame(currency);
            return code;
        }

        // A native method runs without a Tcl call frame (the OO context frame is
        // already pushed by the caller for `self`/`my`).
        if let Method::Builtin { handler, .. } = m {
            let name_owner = self.oo_name_owner(obj);
            self.oo.borrow_mut().call_stack.push(OoFrame {
                name_owner,
                activation: None,
                object: obj,
                chain,
                index,
                target: target.to_vec(),
                external,
            });
            let code = handler(self, obj, args);
            self.retire_oo_call_frame(currency);
            return code;
        }

        // A forward: build `prefix + args` and dispatch (with the OO context so a
        // forwarded `my`/`self` still works).
        if let Method::Forward { prefix } = &m {
            let members = self.list_elements(&prefix.as_ptr());
            if let Err(first) = self.check_entered_native_operation() {
                return self
                    .report_cmd_error(tcl_cmd_core::CmdError::from_execution_refusal(first));
            }
            let members = match members {
                Ok(members) => members,
                Err(error) => return self.report_cmd_error(error.into()),
            };
            let namespace = {
                self.oo
                    .borrow()
                    .objects
                    .get(&obj)
                    .map(|object| object.var_ns)
            };
            let Some(namespace) = namespace else {
                return self.set_error(b"forwarded object has been deleted");
            };
            let mut new_argv = Vec::with_capacity(members.len() + args.len());
            new_argv.extend_from_slice(&members);
            new_argv.extend_from_slice(args);
            // The original List owns its members. This borrowed invocation adds
            // no native child references and preserves the exact member headers.
            let originals =
                match crate::interp::native_arguments::OriginalArguments::enter(self, &new_argv) {
                    Ok(originals) => originals,
                    Err(error) => return self.report_cmd_error(error),
                };
            // The method this forward (possibly via a chain of forwards /
            // ensembles) leads to reports `wrong # args` against the *original*
            // invocation, not the forwarded command (C's ensemble rewrite). The
            // first forward in the chain is the root: record the invocation words
            // (`obj method ?args...?`); a downstream `wrong # args` rewrites
            // against them. `fwd_usage` keeps the simple single-step prefix for
            // the immediate next body's `info`-level naming.
            let head = if external {
                oo_display(self, obj)
            } else {
                b"my".to_vec()
            };
            let mut fwd = head.clone();
            fwd.push(b' ');
            fwd.extend_from_slice(target);
            self.oo.borrow_mut().fwd_usage = Some(fwd);
            let mut source = vec![
                crate::obj::Owned::fresh(crate::interp::new_string(&head)),
                crate::obj::Owned::fresh(crate::interp::new_string(target)),
            ];
            source.extend(args.iter().map(|&word| crate::obj::Owned::retain(word)));
            // `obj method` (2 words) is replaced by the forward `prefix`.
            let is_root = self.begin_ensemble_rewrite(source, 2, members.len());
            let name_owner = self.oo_name_owner(obj);
            self.oo.borrow_mut().call_stack.push(OoFrame {
                name_owner,
                activation: None,
                object: obj,
                chain,
                index,
                target: target.to_vec(),
                external,
            });
            let code = self.dispatch_in_lookup_namespace(namespace, &new_argv);
            self.retire_oo_call_frame(currency);
            let cleared = if is_root {
                self.clear_ensemble_rewrite_checked()
            } else {
                Ok(())
            };
            // Clear the rewrite if the forwarded command was not a method body
            // (which would have consumed it).
            self.oo.borrow_mut().fwd_usage = None;
            drop(m);
            if let Err(first) = cleared {
                self.refuse_native_execution(first);
            }
            return originals.finish(code);
        }

        let Method::Body { procedure, .. } = m else {
            unreachable!("forward handled above");
        };
        let method_client_data = procedure;
        let procedure = method_client_data.declaration();
        let body = match procedure
            .body
            .checked_ptr()
            .and_then(|original| ValueOps::native_string_bytes(self, &original))
        {
            Ok(body) => body,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        let params = &procedure.params;
        let Some(var_ns) = self.oo.borrow().objects.get(&obj).map(|o| o.var_ns) else {
            let mut m = b"object \"".to_vec();
            m.extend_from_slice(&self.oo_name(obj));
            m.extend_from_slice(b"\" has been deleted");
            return self.error(&m);
        };
        // The declared instance variables visible to the method, as `(local,
        // storage)` links: public variables (the object's own plus the union
        // over its full class hierarchy) link name→name; TIP 500 private
        // variables of the *declaring* provider link name→a per-class mangled
        // storage name (`"<creationEpoch> : name"`, C's PRIVATE_VARIABLE_PATTERN),
        // so identically-named private vars in different classes don't collide.
        let mut vars: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        let push_public = |v: &Vec<u8>, vars: &mut Vec<(Vec<u8>, Vec<u8>)>| {
            if !vars.iter().any(|(l, _)| l == v) {
                vars.push((v.clone(), v.clone()));
            }
        };
        // Private vars first, so a same-named public var down the chain does not
        // shadow the declaring provider's private mapping.
        {
            let oo = self.oo.borrow();
            let privates = if is_object {
                oo.objects.get(&prov).map(|o| {
                    o.private_variables
                        .iter()
                        .map(variable_mapping::observation)
                        .collect::<Vec<_>>()
                })
            } else {
                oo.classes.get(&prov).map(|c| {
                    c.private_variables
                        .iter()
                        .map(variable_mapping::observation)
                        .collect::<Vec<_>>()
                })
            };
            // A class is also an object, so its creation id lives in `objects`.
            let epoch = oo.objects.get(&prov).map(|o| o.creation_id).unwrap_or(0);
            if let Some(pv) = privates {
                for variable in pv {
                    if let Err(error) = variable.check_originals() {
                        drop(oo);
                        return self.report_cmd_error(error.into());
                    }
                    let v = variable.name;
                    if !vars.iter().any(|(l, _)| *l == v) {
                        let storage =
                            variable.full_name.map(|full| full.name).unwrap_or_else(|| {
                                let mut storage = format!("{epoch} : ").into_bytes();
                                storage.extend_from_slice(&v);
                                storage
                            });
                        vars.push((v, storage));
                    }
                }
            }
        }
        // Public `variable` declarations are scoped to the *declaring* provider
        // (C links `clsPtr->variables` / `oPtr->variables` of the method's
        // declarer, not the whole hierarchy): a class method sees its class's
        // declared vars, a per-object method sees the object's (oo-38.4).
        {
            let oo = self.oo.borrow();
            let declared = if is_object {
                oo.objects.get(&prov).map(|o| {
                    o.variables
                        .iter()
                        .map(variable_mapping::observation)
                        .collect::<Vec<_>>()
                })
            } else {
                oo.classes.get(&prov).map(|c| {
                    c.variables
                        .iter()
                        .map(variable_mapping::observation)
                        .collect::<Vec<_>>()
                })
            };
            if let Some(vs) = declared {
                for v in &vs {
                    if let Err(error) = v.check_originals() {
                        drop(oo);
                        return self.report_cmd_error(error.into());
                    }
                    push_public(&v.name, &mut vars);
                }
            }
        }
        // TclOO's automatic instance-variable resolver yields to a method
        // formal of the same name. Explicit `my variable X` still uses the
        // ordinary link path and reports a collision.
        vars.retain(|(local, _)| !params.iter().any(|param| param.name == *local));
        let name = if report_role == NativeTclooMethodReportRole::Constructor {
            b"<constructor>".to_vec()
        } else {
            method.clone()
        };
        // The method-body errorInfo frame (`CommonMethErrorHandler`): named by
        // the *declaring* entity — `object`/`class` per whether the provider is
        // the object itself or one of its classes.
        let kind: &[u8] = if is_object { b"object" } else { b"class" };
        let what = if report_role == NativeTclooMethodReportRole::Constructor {
            MethodFrameWhat::Constructor
        } else if report_role == NativeTclooMethodReportRole::Destructor {
            MethodFrameWhat::Destructor
        } else {
            MethodFrameWhat::Named(&method)
        };
        let name_owner = self.oo_name_owner(obj);
        let is_filter_step = chain[index].is_filter;
        self.oo.borrow_mut().call_stack.push(OoFrame {
            name_owner,
            activation: None,
            object: obj,
            chain,
            index,
            target: target.to_vec(),
            external,
        });
        // A method defined while sourcing a file carries that file + the body's
        // line base, so `info frame` reports file-absolute lines (TIP 280).
        let source = procedure.source.clone();
        let body_line_base = procedure.body_line_base;
        // `wrong # args` prefix: a forward's rewritten original invocation, else
        // the invoking `obj method` (external) / `my method` (internal). Only
        // regular methods (constructors/destructors keep their synthetic name).
        let forwarded_usage = self.oo.borrow_mut().fwd_usage.take();
        let usage_prefix: Option<Vec<u8>> = if report_role != NativeTclooMethodReportRole::Named {
            None
        } else if let Some(u) = forwarded_usage {
            Some(u)
        } else {
            let mut p = if external {
                oo_display(self, obj)
            } else {
                b"my".to_vec()
            };
            p.push(b' ');
            p.extend_from_slice(&method);
            Some(p)
        };
        // A constructor's `info level 0` reports the originating `create`/`new`
        // invocation (e.g. `oo::object create foo`), captured by the dispatcher,
        // rather than the synthetic `<constructor>` name (oo-2.1).
        // An actual filter step runs with
        // `filter_handling` set, so its own `my` calls — and everything they
        // call — are not re-wrapped by the same filters. A `next` from the filter
        // clears it again for the wrapped method (handled in `next_cmd`). Saved
        // and restored so nested filter chains compose.
        let saved_fh = self.oo.borrow().filter_handling;
        if is_filter_step {
            self.oo.borrow_mut().filter_handling = true;
        }
        let code = self.run_proc(
            params,
            &body,
            var_ns,
            args,
            &name,
            CallMeta {
                original_argv: original_argv.as_deref(),
                err: ProcFrame::Method {
                    kind,
                    owner: &self.oo_name(prov),
                    what,
                },
                fqn: None,
                source,
                body_line_base,
                link_vars: &vars,
                oo_variable_resolver: Some(native_variables::NativeOoVariableResolver::new(
                    &self.oo, prov, is_object, var_ns,
                )),
                // Property accessors propagate break/continue to `configure`.
                keep_loop_codes: method.starts_with(b"<ReadProp-")
                    || method.starts_with(b"<WriteProp-"),
                // A non-first chain step is reached via `next`: it shares the
                // level of the original method invocation (step 0).
                same_level: index > 0,
                usage_prefix,
                level_words,
                quote_name: false,
                native: None,
                statics: None,
                c_procedure: Some(&procedure),
                c_method_client_data: Some(&method_client_data),
                jim_parameters: None,
                jim_body: None,
                jim_namespace: None,
            },
        );
        self.oo.borrow_mut().filter_handling = saved_fh;
        self.retire_oo_call_frame(currency);
        code
    }
}
