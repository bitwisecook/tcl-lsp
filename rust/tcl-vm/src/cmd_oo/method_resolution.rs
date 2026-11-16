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

//! Method resolution order: class linearisation, object precedence, visibility and the method chain of one invocation.

use super::{
    Method, OoState, Step, method_report_bytes, native_context, native_method_cache, object_message,
};
use crate::interp::{Vm, err};
use crate::value::Value;
use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;
use tcl_registry::native_tcloo_method_definition::NativeTclooMethodReportRole;
use tcl_runtime_api::{Completion, OoId};
use tcl_syntax::mro::{MroError, tcloo_linearise};

impl OoState {
    /// Build the `(supers, mixins)` maps `tcloo_linearise` consumes from the
    /// live class registry, injecting `oo::object` as the implicit root of any
    /// class with no declared superclass (mirroring `TclOO`, where every class
    /// ultimately derives from `oo::object`).
    fn linearise_maps(&self) -> (HashMap<OoId, Vec<OoId>>, HashMap<OoId, Vec<OoId>>) {
        let mut supers = HashMap::with_capacity(self.classes.len());
        let mut mixins = HashMap::with_capacity(self.classes.len());
        for (name, c) in &self.classes {
            let mut sup = c.supers.clone();
            if sup.is_empty()
                && Some(*name) != self.object_root
                && let Some(root) = self.object_root
            {
                sup.push(root);
            }
            supers.insert(*name, sup);
            mixins.insert(*name, c.mixins.clone());
        }
        (supers, mixins)
    }

    /// The class-side linearisation, delegated to the compiler's faithful
    /// `TclOO` algorithm ([`tcloo_linearise`]: the two-pass mixin/super split with
    /// late-placement dedup from `tclOOCall.c`, correct for nested mixins and
    /// diamonds where a naive per-class DFS is not). Used for constructor /
    /// destructor chains and `info class properties -all`.
    pub(super) fn class_linear_of(&self, class_key: OoId) -> Vec<Step> {
        let (supers, mixins) = self.linearise_maps();
        class_steps(tcloo_linearise(&class_key, &supers, &mixins), class_key)
    }

    /// The full method-search precedence for an object: its per-object mixins
    /// (each linearised as a class), then the object's own methods, then its
    /// class MRO — keep-last deduped so a shared class defers to its
    /// most-derived position.
    pub(super) fn object_precedence(&self, obj_key: OoId) -> Vec<Step> {
        let Some(obj) = self.objects.get(&obj_key) else {
            return Vec::new();
        };
        let (supers, mixins) = self.linearise_maps();
        let mut out: Vec<Step> = Vec::new();
        for mx in &obj.mixins {
            out.extend(class_steps(tcloo_linearise(mx, &supers, &mixins), *mx));
        }
        out.push(Step {
            method_owner: None,
            provider: obj_key,
            is_object: true,
        });
        out.extend(class_steps(
            tcloo_linearise(&obj.class, &supers, &mixins),
            obj.class,
        ));
        keep_last(&out)
    }

    /// The `Method` a step provides for `name`, if any.
    pub(super) fn step_method(&self, step: &Step, name: &str) -> Option<Method> {
        if step.is_object {
            self.objects.get(&step.provider)?.methods.get(name)
        } else {
            self.classes.get(&step.provider)?.methods.get(name)
        }
    }

    /// Whether `name` is exported for external dispatch at `step` (the
    /// per-provider `export`/`unexport` overrides beat the method's default).
    pub(super) fn step_exported(&self, step: &Step, name: &str) -> bool {
        let (exported, unexported, dflt) = if step.is_object {
            let o = self.objects.get(&step.provider);
            (
                o.is_some_and(|o| o.exported.contains(name)),
                o.is_some_and(|o| o.unexported.contains(name)),
                o.and_then(|o| o.methods.get(name)).map(|m| m.exported),
            )
        } else {
            let c = self.classes.get(&step.provider);
            (
                c.is_some_and(|c| c.exported.contains(name)),
                c.is_some_and(|c| c.unexported.contains(name)),
                c.and_then(|c| c.methods.get(name)).map(|m| m.exported),
            )
        };
        if unexported {
            return false;
        }
        exported || dflt.unwrap_or(false)
    }

    /// Whether `step` holds a non-private record or an export override for
    /// `name`; C's `unexport` and `export` create such a record.
    fn step_declares_visibility(&self, step: &Step, name: &str) -> bool {
        let (methods, exported, unexported) = if step.is_object {
            let Some(object) = self.objects.get(&step.provider) else {
                return false;
            };
            (&object.methods, &object.exported, &object.unexported)
        } else {
            let Some(class) = self.classes.get(&step.provider) else {
                return false;
            };
            (&class.methods, &class.exported, &class.unexported)
        };
        exported.contains(name)
            || unexported.contains(name)
            || methods
                .owner(name)
                .is_some_and(|method| !method.borrow().private)
    }

    /// Whether an external call may name `name`: the first record met, the
    /// object's own before any class's, decides, even one that only records
    /// visibility (tclOOCall.c `AddSimpleChainToCallContext`).
    fn publicly_visible(&self, obj_key: OoId, precedence: &[Step], name: &str) -> bool {
        precedence
            .iter()
            .find(|step| {
                step.is_object
                    && step.provider == obj_key
                    && self.step_declares_visibility(step, name)
            })
            .or_else(|| {
                precedence
                    .iter()
                    .find(|step| self.step_declares_visibility(step, name))
            })
            .is_some_and(|step| self.step_exported(step, name))
    }

    /// Names callable on `obj_key` (for the unknown-method error), honouring
    /// export state. `internal` includes unexported names.
    pub(super) fn visible_methods(&self, obj_key: OoId, internal: bool) -> BTreeSet<String> {
        let mut names = BTreeSet::new();
        let precedence = self.object_precedence(obj_key);
        for step in &precedence {
            let provided: Vec<String> = if step.is_object {
                self.objects
                    .get(&step.provider)
                    .map_or_default(|o| o.methods.keys().cloned().collect())
            } else {
                self.classes
                    .get(&step.provider)
                    .map_or_default(|c| c.methods.keys().cloned().collect())
            };
            for name in provided {
                if self
                    .step_method(step, &name)
                    .is_some_and(|method| !method.visibility_only && !method.private)
                    && (internal || self.publicly_visible(obj_key, &precedence, &name))
                {
                    names.insert(name);
                }
            }
        }
        if let Some(object) = self.objects.get(&obj_key) {
            names.retain(|name| !object.unexported.contains(name));
        }
        names
    }
}

/// Map a [`tcloo_linearise`] result into class-facet [`Step`]s, degrading to the
/// class itself when linearisation fails (a superclass cycle involving it, or a
/// pathological depth `tcloo_linearise` refuses to expand).
fn class_steps(chain: Result<Vec<OoId>, MroError>, fallback: OoId) -> Vec<Step> {
    match chain {
        Ok(classes) => classes
            .into_iter()
            .map(|c| Step {
                method_owner: None,
                provider: c,
                is_object: false,
            })
            .collect(),
        Err(_) => vec![Step {
            method_owner: None,
            provider: fallback,
            is_object: false,
        }],
    }
}

/// Keep-last dedup of a step list on `(provider, is_object)`: each entry
/// survives only at its last position (deferring shared bases past their
/// descendants), preserving order otherwise.
fn keep_last(steps: &[Step]) -> Vec<Step> {
    let mut out: Vec<Step> = Vec::new();
    for (i, s) in steps.iter().enumerate() {
        let later = steps[i + 1..]
            .iter()
            .any(|t| t.provider == s.provider && t.is_object == s.is_object);
        if !later {
            out.push(s.clone());
        }
    }
    out
}

pub(super) fn invocation_method_chain(
    vm: &mut Vm,
    obj_key: OoId,
    method: &str,
    original_method: &Value,
    external: bool,
) -> Result<native_method_cache::ReachedMethodChain, Completion<Value>> {
    let caller_scope = native_context::current_method(vm).and_then(|frame| {
        frame
            .chain
            .get(frame.index)
            .filter(|step| !step.is_object || frame.object == obj_key)
            .map(|step| (step.provider, step.is_object))
    });
    let precedence = vm.oo.object_precedence(obj_key);
    let cacheable = !precedence.iter().any(|step| {
        let methods = if step.is_object {
            vm.oo
                .objects
                .get(&step.provider)
                .map(|object| &object.methods)
        } else {
            vm.oo
                .classes
                .get(&step.provider)
                .map(|class| &class.methods)
        };
        methods.is_some_and(|methods| methods.values().any(|method| method.borrow().private))
    });
    let cached = if cacheable {
        native_method_cache::lookup(vm, obj_key, method, original_method, external)
            .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?
    } else {
        None
    };
    Ok(if let Some(chain) = cached {
        chain
    } else {
        let public = vm.oo.publicly_visible(obj_key, &precedence, method);
        let mut chain: Vec<Step> = precedence
            .into_iter()
            .filter(|step| {
                vm.oo.step_method(step, method).is_some_and(|method| {
                    !method.visibility_only
                        && (!method.private
                            || caller_scope == Some((step.provider, step.is_object)))
                })
            })
            .collect();
        if let Some(position) = chain.iter().position(|step| {
            caller_scope == Some((step.provider, step.is_object))
                && vm
                    .oo
                    .step_method(step, method)
                    .is_some_and(|method| method.private)
        }) {
            let private = chain.remove(position);
            chain.insert(0, private);
        }

        if chain.is_empty() {
            return Err(unknown_method(vm, obj_key, method, external));
        }
        // External calls require the first visibility record to be exported.
        if external
            && (vm
                .oo
                .objects
                .get(&obj_key)
                .is_some_and(|object| object.unexported.contains(method))
                || !public)
            && !vm
                .oo
                .step_method(&chain[0], method)
                .is_some_and(|method| method.private)
        {
            return Err(unknown_method(vm, obj_key, method, external));
        }
        let chain = native_method_cache::retain_owners(
            &vm.oo,
            chain,
            method,
            NativeTclooMethodReportRole::Named,
        );
        if cacheable {
            match native_method_cache::remember(
                vm,
                obj_key,
                method,
                original_method,
                external,
                Rc::new(chain),
            ) {
                Ok(chain) => chain,
                Err(error) => {
                    return Err(crate::command::completion_from_cmd_error(vm, error));
                }
            }
        } else {
            chain.into()
        }
    })
}

/// Whether an object built-in (`variable`/`varname`/`eval`) is exported — they
/// are unexported by default, so only reachable externally after `self export`.
pub(super) fn is_exported_builtin(vm: &Vm, obj_key: OoId, method: &str) -> bool {
    vm.oo
        .objects
        .get(&obj_key)
        .is_some_and(|o| o.exported.contains(method))
}

/// The `unknown method "X": must be …` error (or "no visible methods").
fn unknown_method(vm: &Vm, obj_key: OoId, method: &str, external: bool) -> Completion<Value> {
    let names = vm.oo.visible_methods(obj_key, !external);
    if names.is_empty() {
        return err(object_message(
            vm,
            obj_key,
            b"object \"",
            b"\" has no visible methods",
        ));
    }
    let mut names: Vec<_> = names
        .iter()
        .map(|name| method_report_bytes(vm, name))
        .collect();
    names.sort();
    let mut message = b"unknown method \"".to_vec();
    message.extend_from_slice(&method_report_bytes(vm, method));
    message.extend_from_slice(b"\": must be ");
    message.extend_from_slice(&tcl_cmd_core::prefix::tcloo_choice_list_bytes(&names));
    err(message)
}
