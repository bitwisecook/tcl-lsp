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

//! Creating classes and objects: the command publication, `oo_make_class`, the `create`/`new`/`createWithNamespace` factory and the instance namespace.

use super::super::{
    method_invocation, native_context, native_method_cache, wrong_args, CallStep, Class, DefTarget,
    MethodInvocation, Object, ObjectPublication, OriginalObjectPublication,
};
use crate::interp::{Code, Command, Interp};
use crate::namespace::NsId;
use crate::obj;
use crate::obj::TclObj;
use tcl_core_types::OoId;
use tcl_registry::native_tcloo_method_definition::{
    NativeTclooMethodPayloadType, NativeTclooMethodReportRole,
};

impl Interp {
    /// Select and validate one actual object-command publication before allocation.
    pub(in super::super) fn oo_publication(
        &mut self,
        original: &[u8],
    ) -> Result<ObjectPublication, Code> {
        let current = self.current_ns();
        let Some((namespace, simple)) = self
            .namespaces_mut()
            .oo_object_publication_at(current, original)
        else {
            return Err(self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "TclOO original object publication",
                )
                .into(),
            ));
        };
        self.retire_gate_hidden_object_in_slot(namespace, &simple);
        if self.has_visible_command_in_slot(namespace, &simple) {
            let Some(protocol) = self.name_policy_protocol() else {
                return Err(self.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "TclOO object collision naming issuer",
                    )
                    .into(),
                ));
            };
            let message = match protocol.recipe().oo_object_collision_message(original) {
                Ok(message) => message,
                Err(_) => {
                    return Err(self.report_cmd_error(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "TclOO object collision report",
                        )
                        .into(),
                    ));
                }
            };
            return Err(self.error(&message));
        }
        let report = self.namespaces().command_fqn_at(namespace, &simple);
        Ok(ObjectPublication {
            namespace,
            simple,
            report,
        })
    }

    /// Create a class at its selected slot, running its optional definition script.
    pub(in super::super) fn oo_make_class(
        &mut self,
        publication: ObjectPublication,
        script: Option<*mut TclObj>,
    ) -> Code {
        let scope =
            match crate::interp::native_operation_currency::NativeOperationScope::enter(self) {
                Ok(scope) => scope,
                Err(error) => return self.report_cmd_error(error.into()),
            };
        let fqn = &publication.report;
        let object = self.oo.borrow_mut().allocate(fqn.to_vec());
        let (object_root, class_root) = {
            let oo = self.oo.borrow();
            (oo.object_root, oo.class_root)
        };
        self.oo.borrow_mut().classes.insert(
            object,
            Class {
                supers: object_root.into_iter().collect(),
                ..Class::default()
            },
        );
        // A class is also an object (an instance of `::oo::class`), so it can
        // carry its own methods (`oo::define … self method`) — the TclOO
        // classes-as-objects model.
        let var_ns = self.fresh_native_oo_namespace();
        native_context::install_object_helpers(self, var_ns);
        let creation_id = self.oo_next_id();
        self.oo.borrow_mut().objects.insert(
            object,
            Object::new(
                class_root.expect("TclOO class root installed"),
                var_ns,
                creation_id,
            ),
        );
        let original = match OriginalObjectPublication::capture(self, object, scope.currency()) {
            Ok(original) => original,
            Err(error) => return self.oo_refused_publication(object, error),
        };
        self.bind_command_replacement(
            publication.namespace,
            &publication.simple,
            Command::OoObject(object),
        );
        let continued = original
            .check(self, scope.currency())
            .and_then(|()| self.oo_register_my(object));
        if let Err(error) = continued {
            return self.oo_refused_publication(object, error);
        }
        if let Some(script) = script {
            let code = self.oo_define_body(DefTarget::Class(object), script, None);
            if code != Code::Ok {
                // Roll back a failed definition so the name frees up (C destroys
                // a partially-created class whose definition script errors).
                self.mark_original_oo_record_torn_down(object);
                self.retire_original_oo_class_record(object);
                let _ = self.retire_native_properties(object);
                self.retire_original_oo_object_record(object);
                self.retire_oo_command_identity(object);
                self.oo.borrow_mut().names.remove(&object);
                self.delete_namespace_by_id(var_ns);
                return if self.check_entered_native_operation().is_err() {
                    Code::Error
                } else {
                    code
                };
            }
        }
        self.set_result(obj::new_string_bytes(fqn));
        Code::Ok
    }

    /// The class instantiation built-ins (`create`/`new`/`createWithNamespace`).
    /// Shared by external `$cls method …` dispatch and internal `my method …`
    /// dispatch, so a metaclass method can `my create …`. `args` are the
    /// arguments *after* the method name; `cmd` names the command for
    /// `wrong # args`. `block_unexported` honours an `unexport` of `create`/
    /// `new` (and the default-unexported `createWithNamespace`) for an external
    /// call; an internal call sees them all. Returns `None` when `method` is not
    /// an applicable built-in, so the caller falls through.
    pub(in super::super) fn oo_class_factory(
        &mut self,
        class: OoId,
        cmd: &[u8],
        method: &[u8],
        args: &[*mut TclObj],
        block_unexported: bool,
        original_argv: Option<&[*mut TclObj]>,
    ) -> Option<Code> {
        if !self.oo.borrow().classes.contains_key(&class) {
            return None;
        }
        // The metaclass `::oo::class` is a singleton: instantiating it builds a
        // *class* (`oo_make_class`), and `new` is unexported on it.
        let is_meta = Some(class) == self.oo.borrow().class_root;
        let cre_ok =
            !block_unexported || self.factory_export_record(class, b"create") != Some(false);
        let new_ok = !block_unexported || self.factory_export_record(class, b"new") != Some(false);
        // `createWithNamespace` is unexported by default; an external call needs
        // it exported, an internal call reaches it regardless.
        let cwn_ok = !block_unexported
            || self.factory_export_record(class, b"createWithNamespace") == Some(true);
        let applicable = match method {
            b"new" => !is_meta && new_ok,
            b"create" => cre_ok,
            b"createWithNamespace" => cwn_ok,
            _ => false,
        };
        if !applicable {
            return None;
        }
        Some(native_context::with_original_arguments(
            self,
            args,
            |interp, originals| {
                let class_owner = oo_checked!(
                    interp,
                    native_context::OriginalOoTarget::capture(interp, class, true)
                );
                let invocation = |interp: &Interp| {
                    method_invocation(interp, class, Some(cmd), method, args, false, original_argv)
                };
                match method {
                    b"new" => {
                        let invocation = oo_checked!(interp, invocation(interp));
                        oo_checked!(interp, class_owner.check(interp));
                        interp.oo_new(class, None, args, invocation)
                    }
                    b"createWithNamespace" => {
                        if args.len() < 2 {
                            let mut usage = cmd.to_vec();
                            usage.extend_from_slice(
                                b" createWithNamespace objectName namespaceName ?arg ...?",
                            );
                            return wrong_args(interp, &usage);
                        }
                        let raw = oo_original_bytes!(interp, originals, 0);
                        oo_checked!(interp, class_owner.check(interp));
                        if raw.is_empty() {
                            return interp.error(b"object name must not be empty");
                        }
                        let publication = match interp.oo_publication(&raw) {
                            Ok(publication) => publication,
                            Err(code) => return code,
                        };
                        let namespace = oo_original_bytes!(interp, originals, 1);
                        oo_checked!(interp, class_owner.check(interp));
                        if oo_checked!(interp, interp.resolve_namespace_name(&namespace)).is_some()
                        {
                            let error = oo_checked!(
                                interp,
                                interp.namespace_creation_conflict_error(&namespace)
                            );
                            return interp.report_cmd_error(error);
                        }
                        let invocation = oo_checked!(interp, invocation(interp));
                        oo_checked!(interp, class_owner.check(interp));
                        let namespace =
                            oo_checked!(interp, interp.ensure_namespace_address(&namespace));
                        oo_original_check!(interp, originals);
                        oo_checked!(interp, class_owner.check(interp));
                        interp.oo_new_ns(
                            class,
                            Some(publication),
                            Some(namespace),
                            &args[2..],
                            invocation,
                        )
                    }
                    b"create" => {
                        if args.is_empty() {
                            let mut usage = cmd.to_vec();
                            usage.extend_from_slice(b" create objectName ?arg ...?");
                            return wrong_args(interp, &usage);
                        }
                        let raw = oo_original_bytes!(interp, originals, 0);
                        oo_checked!(interp, class_owner.check(interp));
                        if raw.is_empty() {
                            return interp.error(b"object name must not be empty");
                        }
                        let publication = match interp.oo_publication(&raw) {
                            Ok(publication) => publication,
                            Err(code) => return code,
                        };
                        if is_meta {
                            interp.oo_make_class(publication, args.get(1).copied())
                        } else {
                            let invocation = oo_checked!(interp, invocation(interp));
                            oo_checked!(interp, class_owner.check(interp));
                            interp.oo_new(class, Some(publication), &args[1..], invocation)
                        }
                    }
                    _ => unreachable!("selected class factory"),
                }
            },
        ))
    }

    fn oo_new(
        &mut self,
        class: OoId,
        publication: Option<ObjectPublication>,
        args: &[*mut TclObj],
        invocation: MethodInvocation,
    ) -> Code {
        self.oo_new_ns(class, publication, None, args, invocation)
    }

    /// `oo_new` with an optional explicit instance-variable namespace
    /// (`createWithNamespace`); `None` allocates the native hidden namespace.
    pub(in super::super) fn fresh_native_oo_namespace(&mut self) -> NsId {
        let name = {
            let mut oo = self.oo.borrow_mut();
            let number = oo.counter;
            oo.counter += 1;
            format!("::oo::Obj{number}")
        };
        self.ensure_command_owned_namespace(name.as_bytes())
    }

    pub(in super::super) fn oo_new_ns(
        &mut self,
        class: OoId,
        publication: Option<ObjectPublication>,
        ns_override: Option<NsId>,
        args: &[*mut TclObj],
        invocation: MethodInvocation,
    ) -> Code {
        let scope =
            match crate::interp::native_operation_currency::NativeOperationScope::enter(self) {
                Ok(scope) => scope,
                Err(error) => return self.report_cmd_error(error.into()),
            };
        let anonymous = publication.is_none();
        let publication = match publication {
            Some(publication) => publication,
            None => {
                let name = format!("::oo::Obj{}", self.oo.borrow().counter);
                self.oo.borrow_mut().counter += 1;
                match self.oo_publication(name.as_bytes()) {
                    Ok(publication) => publication,
                    Err(code) => return code,
                }
            }
        };
        let fqn = publication.report.clone();
        let var_ns = match ns_override {
            Some(ns) => ns,
            None if anonymous => self
                .namespaces_mut()
                .ensure_command_owned_namespace_in_slot(publication.namespace, &publication.simple),
            None => self.fresh_native_oo_namespace(),
        };
        native_context::install_object_helpers(self, var_ns);
        let creation_id = self.oo_next_id();
        let object = self.oo.borrow_mut().allocate(fqn.clone());
        self.oo
            .borrow_mut()
            .objects
            .insert(object, Object::new(class, var_ns, creation_id));
        // Instantiating a *metaclass* (one whose MRO includes `::oo::class`)
        // produces a class: register the new instance in the class map too, so
        // it responds to `create`/`new` and is `info object isa class`.
        let mro = self.mro(class);
        let is_metaclass = mro.iter().any(|c| Some(*c) == self.oo.borrow().class_root);
        if is_metaclass {
            // A TIP 558 configurable class (instance of `::oo::configurable`)
            // mixes in the configurable support (for `configure`) and points its
            // instance definitions at the configurableobject namespace (so
            // `oo::objdefine $inst property …` resolves). Done before the
            // definition script runs, which still resolves class-name arguments
            // in the caller's namespace.
            let configurable = mro
                .iter()
                .any(|c| Some(*c) == self.oo.borrow().configurable_root);
            let configurable_support = oo_checked!(
                self,
                self.oo_resolve_object_bytes(b"::oo::configuresupport::configurable")
            );
            let object_root = self.oo.borrow().object_root.into_iter().collect();
            self.oo.borrow_mut().classes.insert(
                object,
                Class {
                    supers: object_root,
                    mixins: if configurable {
                        vec![configurable_support]
                    } else {
                        Vec::new()
                    },
                    def_ns: if configurable {
                        Some(b"::oo::configuresupport::configurableobject".to_vec())
                    } else {
                        None
                    },
                    ..Class::default()
                },
            );
        }
        let original = match OriginalObjectPublication::capture(self, object, scope.currency()) {
            Ok(original) => original,
            Err(error) => return self.oo_refused_publication(object, error),
        };
        self.bind_command_replacement(
            publication.namespace,
            &publication.simple,
            Command::OoObject(object),
        );
        let continued = original
            .check(self, scope.currency())
            .and_then(|()| self.oo_register_my(object));
        if let Err(error) = continued {
            return self.oo_refused_publication(object, error);
        }

        // Constructor dispatch runs along the *class* MRO (objects can't define
        // constructors), with the object as `self`; the chain is the
        // constructor-providing classes in MRO order (so `next` chains). For a
        // metaclass instance, `::oo::class` contributes a synthetic constructor
        // that applies the (optional) definition-script argument.
        let chain: Vec<CallStep> = mro
            .iter()
            .filter(|c| {
                self.class_has_ctor(**c)
                    || (is_metaclass && Some(**c) == self.oo.borrow().class_root)
            })
            .map(|c| CallStep {
                method_owner: None,
                is_object: false,
                is_filter: false,
                synthetic_type: (Some(*c) == self.oo.borrow().class_root
                    && !self.class_has_ctor(*c))
                .then_some(NativeTclooMethodPayloadType::ClassConstructor),
                declaring_facet: None,
                provider: *c,
                method: Vec::new(),
            })
            .collect();
        if !chain.is_empty() {
            let chain =
                native_method_cache::ReachedMethodChain::from(native_method_cache::retain_owners(
                    &self.oo.borrow(),
                    chain,
                    NativeTclooMethodReportRole::Constructor,
                ))
                .with_report_role(NativeTclooMethodReportRole::Constructor);
            let code = self.oo_run(object, chain, 0, b"", args, invocation);
            if code == Code::Error {
                // A failed constructor tears the partially-built object down,
                // running its destructor (C: the object is deleted, firing the
                // destructor chain), then re-raises the constructor's error.
                let snap = self.error_snapshot();
                if self.oo.borrow().objects.contains_key(&object) {
                    self.oo_destroy_bg(object);
                }
                // `oo_destroy_bg` removes the object; clean up the class facet
                // (a failed metaclass instantiation) and the command too.
                let _ = self.retire_native_properties(object);
                self.mark_original_oo_record_torn_down(object);
                self.retire_original_oo_object_record(object);
                self.retire_original_oo_class_record(object);
                self.retire_oo_command_identity(object);
                self.oo.borrow_mut().names.remove(&object);
                if let Err(first) = self.error_restore(snap) {
                    return self.refuse_native_execution(first);
                }
                let _ = self.check_entered_native_operation();
                return Code::Error;
            }
            // A constructor that destroys its own object (`[self] destroy`)
            // leaves nothing to return: `new`/`create` fails (oo-30.1/30.2,
            // Bug 2903011).
            if !self.oo.borrow().objects.contains_key(&object) {
                return self.error(b"object deleted in constructor");
            }
        }
        // With no constructor in the MRO, extra construction arguments are
        // silently ignored (matching tclsh9.0 — the default constructor is a
        // no-op that does not validate its argument count).
        self.set_result(obj::new_string_bytes(&fqn));
        Code::Ok
    }
}
