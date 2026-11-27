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

//! TclOO's original object-name header and selected variable-frame ownership.

use super::{
    list, next_cmd, nextto_cmd_using, obj, self_cmd, CallStep, Code, Command, DefTarget, Interp,
    NsId, OoId, Rc, RefCell, TclObj, ValueOps, GLOBAL,
};
use crate::interp::native_arguments::OriginalArguments;

type OriginalOoCheck<'a> = &'a dyn Fn(&Interp) -> Result<(), tcl_cmd_core::CmdError>;
type OriginalOoHeadReader<'a> =
    &'a dyn Fn(&Interp, *mut TclObj) -> Result<Rc<[u8]>, tcl_cmd_core::CmdError>;

pub(super) fn with_original_arguments(
    interpreter: &mut Interp,
    argv: &[*mut TclObj],
    worker: impl FnOnce(&mut Interp, &OriginalArguments) -> Code,
) -> Code {
    let originals = match OriginalArguments::enter(interpreter, argv) {
        Ok(originals) => originals,
        Err(error) => return interpreter.report_cmd_error(error),
    };
    let code = worker(interpreter, &originals);
    originals.finish(code)
}

/// Read generated reporting operands through the same original getter owner.
/// The caller's generated head words retain their authored reporting purpose.
pub(super) fn original_word_bytes(
    interpreter: &Interp,
    argv: &[*mut TclObj],
) -> Result<Vec<Vec<u8>>, tcl_cmd_core::CmdError> {
    let originals = OriginalArguments::enter(interpreter, argv)?;
    let words = (0..argv.len())
        .map(|index| originals.bytes(index).map(|bytes| bytes.to_vec()))
        .collect();
    originals.check()?;
    words
}

/// Existing selected List conversion with the actual argument operation's
/// original currency. The explicit Logical simulation lane is the same one
/// already selected by Interp's ValueOps adapter, never a new Native issuer.
pub(super) fn original_list_members(
    interpreter: &Interp,
    original: *mut TclObj,
    originals: &OriginalArguments,
) -> Result<Vec<*mut TclObj>, tcl_cmd_core::CmdError> {
    originals.check()?;
    let protocol = interpreter
        .native_invocation_dialect()
        .native_string_materialization(Some(
            tcl_native_model::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation,
        ))
        .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native object list conversion",
        ))?
        .protocol();
    let association = interpreter.associate_native_jim_arguments(&[original]);
    originals.check()?;
    association?;
    let result =
        list::list_elements_native_with_currency(original, protocol, Some(originals.currency()));
    originals.check()?;
    result.map_err(Into::into)
}

/// The context actually selected after Next's operand/head stages. This weak
/// receipt adds no Method, chain, name or Native reference. Mutable context
/// payload is read by the existing worker after its reached class getter.
pub(super) struct OriginalOoNextContext {
    arguments: OriginalArguments,
    activation: u64,
    frame_birth: std::rc::Weak<()>,
    position: usize,
    object: OoId,
    name_owner: std::rc::Weak<RefCell<Option<obj::Owned>>>,
    steps: std::rc::Weak<Vec<CallStep>>,
}
impl OriginalOoNextContext {
    pub(super) fn capture(interpreter: &Interp) -> Result<Option<Self>, tcl_cmd_core::CmdError> {
        let arguments = OriginalArguments::enter(interpreter, &[])?;
        let (activation, frame_birth) = {
            let frames = interpreter.frames.borrow();
            (
                frames.current_activation(),
                frames.current_activation_owner(),
            )
        };
        let selected = {
            let state = interpreter.oo.borrow();
            state
                .call_stack
                .iter()
                .enumerate()
                .rev()
                .find(|(_, frame)| frame.activation == Some(activation))
                .map(|(position, frame)| Self {
                    arguments,
                    activation,
                    frame_birth,
                    position,
                    object: frame.object,
                    name_owner: Rc::downgrade(&frame.name_owner),
                    steps: frame.chain.observed_steps(),
                })
        };
        if let Some(owner) = &selected {
            owner.check(interpreter)?;
        }
        Ok(selected)
    }
    pub(super) fn check(&self, actual: &Interp) -> Result<(), tcl_cmd_core::CmdError> {
        self.arguments.check_for(actual)?;
        let frame_current = {
            let frames = actual.frames.borrow();
            self.frame_birth.strong_count() != 0
                && frames.current_activation() == self.activation
                && std::rc::Weak::ptr_eq(&frames.current_activation_owner(), &self.frame_birth)
        };
        let selected_current = {
            let state = actual.oo.borrow();
            state
                .call_stack
                .iter()
                .enumerate()
                .rev()
                .find(|(_, frame)| frame.activation == Some(self.activation))
                .is_some_and(|(position, frame)| {
                    position == self.position
                        && frame.object == self.object
                        && self.name_owner.strong_count() != 0
                        && std::rc::Weak::ptr_eq(
                            &Rc::downgrade(&frame.name_owner),
                            &self.name_owner,
                        )
                        && self.steps.strong_count() != 0
                        && std::rc::Weak::ptr_eq(&frame.chain.observed_steps(), &self.steps)
                        && frame.chain.get(frame.index).is_some()
                })
        };
        if !frame_current || !selected_current {
            return Err(self.arguments.retain_refusal(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original selected Next context or chain",
                )
                .into(),
            ));
        }
        Ok(())
    }
}

pub(super) fn next_original_bytes_using(
    interpreter: &Interp,
    originals: &OriginalArguments,
    index: usize,
    extra: Option<OriginalOoCheck<'_>>,
) -> Result<Rc<[u8]>, tcl_cmd_core::CmdError> {
    let Some(extra) = extra else {
        return originals.bytes(index);
    };
    let original = originals.original(index)?;
    let current = || {
        originals.check_original_for(interpreter, index, original)?;
        extra(interpreter)
    };
    let reached = interpreter.native_object_string_bytes_using(original, &current, &|error| {
        originals.retain_refusal(error.into())
    });
    current()?;
    reached
}

/// Read-only continuation of the actual selected OO target. It conveys no
/// command, frame or name lookup authority and never reselects by spelling.
pub(super) struct OriginalOoTarget {
    object: OoId,
    creation: u64,
    class_facet: Option<std::rc::Weak<()>>,
}
impl OriginalOoTarget {
    pub(super) fn capture(
        interpreter: &Interp,
        object: OoId,
        class: bool,
    ) -> Result<Self, tcl_cmd_core::CmdError> {
        let result = {
            let state = interpreter.oo.borrow();
            let creation = state
                .objects
                .get(&object)
                .map(|owner| owner.creation_id)
                .ok_or_else(Self::unavailable)?;
            let class_facet = if class {
                Some(Rc::downgrade(
                    &state
                        .classes
                        .get(&object)
                        .ok_or_else(Self::unavailable)?
                        .facet_birth,
                ))
            } else {
                None
            };
            Self {
                object,
                creation,
                class_facet,
            }
        };
        result.check(interpreter)?;
        Ok(result)
    }
    pub(super) fn for_definition(
        interpreter: &Interp,
        target: &DefTarget,
    ) -> Result<Self, tcl_cmd_core::CmdError> {
        match target {
            DefTarget::Class(object) => Self::capture(interpreter, *object, true),
            DefTarget::Object(object) => Self::capture(interpreter, *object, false),
        }
    }
    pub(super) fn check(&self, interpreter: &Interp) -> Result<(), tcl_cmd_core::CmdError> {
        let state = interpreter.oo.borrow();
        if !state
            .objects
            .get(&self.object)
            .is_some_and(|owner| owner.creation_id == self.creation && !owner.torn_down)
        {
            return Err(Self::unavailable());
        }
        if let Some(original) = &self.class_facet {
            if !state
                .classes
                .get(&self.object)
                .is_some_and(|current| original.ptr_eq(&Rc::downgrade(&current.facet_birth)))
            {
                return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original TclOO class facet",
                )
                .into());
            }
        }
        Ok(())
    }
    /// Join a selected instance namespace to the same original object and ledger.
    pub(super) fn check_instance_namespace(
        &self,
        interpreter: &Interp,
        namespace: NsId,
        holder: &tcl_runtime_api::native_namespace_name::NativeNamespaceNameToken,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        self.check(interpreter)?;
        let same_namespace = interpreter
            .oo
            .borrow()
            .objects
            .get(&self.object)
            .is_some_and(|owner| owner.var_ns == namespace);
        let namespaces = interpreter.namespaces();
        if same_namespace
            && namespaces.owns_namespace_name_token(holder)
            && holder.lifecycle()
                != tcl_syntax::native_namespace_name::NativeNamespaceLifecycle::Dead
        {
            Ok(())
        } else {
            Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original OO report namespace retired",
            )
            .into())
        }
    }
    fn unavailable() -> tcl_cmd_core::CmdError {
        tcl_cmd_core::CmdError::with_error_code_bytes(
            b"this command cannot be called when the object has been deleted".to_vec(),
            b"TCL OO MONKEY_BUSINESS".to_vec(),
        )
    }
}

pub(super) fn install_object_helpers(interp: &mut Interp, namespace: NsId) {
    let helpers = interp.ensure_namespace(b"::oo::Helpers");
    interp.namespaces_mut().set_path(namespace, vec![helpers]);
}

pub(super) fn helper_self(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let live = interp
        .oo
        .borrow()
        .call_stack
        .iter()
        .any(|frame| frame.activation == Some(interp.frames.borrow().current_activation()));
    if !live {
        let Some(&original) = argv.first() else {
            return interp.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original self invocation head",
                )
                .into(),
            );
        };
        let head = match interp.native_string_bytes(&original) {
            Ok(head) => head,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        return interp.native_oo_context_required(&head);
    }
    self_cmd(interp, argv)
}

pub(super) fn install_unknown(interp: &mut Interp) {
    interp.ns_register(
        b"::oo::UnknownDefinition",
        Command::Builtin(unknown_definition),
    );
    let prefix = obj::Owned::fresh(obj::new_string_bytes(b"::oo::UnknownDefinition"));
    for name in [b"::oo::define".as_slice(), b"::oo::objdefine"] {
        if let Some(namespace) = interp.find_namespace_id(name) {
            let retired = interp
                .namespaces_mut()
                .set_unknown_handler(namespace, Some(prefix.clone()));
            drop(retired);
        }
    }
}

fn unknown_definition(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    with_original_arguments(interp, argv, |interp, originals| {
        if interp.active_def_target().is_none() {
            return monkey_business(interp);
        }
        let Some((_, operands)) = argv.get(1..).and_then(|args| args.split_first()) else {
            return interp.set_error(b"bad call of unknown handler");
        };
        let requested_bytes = match originals.bytes(1) {
            Ok(bytes) => bytes,
            Err(error) => return interp.report_cmd_error(error),
        };
        let Some(head) = interp.oo_definition_head(&requested_bytes) else {
            return interp.invalid_command(&requested_bytes);
        };
        let mut arguments = vec![head.as_ptr()];
        arguments.extend_from_slice(operands);
        interp.dispatch(&arguments)
    })
}

pub(super) fn monkey_business(interp: &mut Interp) -> Code {
    let code = crate::interp::error_code_list(&[b"TCL", b"OO", b"MONKEY_BUSINESS"]);
    interp.error_with_code(b"this command may only be called from within the context of an ::oo::define or ::oo::objdefine command", &code)
}

pub(super) fn definition_worker(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    class: bool,
    handler: crate::interp::BuiltinFn,
) -> Code {
    let Some(target) = interp.active_def_target() else {
        return handler(interp, argv);
    };
    let object = match target {
        DefTarget::Class(object) | DefTarget::Object(object) => object,
    };
    if class && !interp.oo.borrow().classes.contains_key(&object) {
        let code = crate::interp::error_code_list(&[b"TCL", b"OO", b"MONKEY_BUSINESS"]);
        return interp.error_with_code(b"attempt to misuse API", &code);
    }
    let index = interp
        .oo_definition_index()
        .expect("selected definition activation");
    interp.oo.borrow_mut().def_stack[index].0 = if class {
        DefTarget::Class(object)
    } else {
        DefTarget::Object(object)
    };
    let code = handler(interp, argv);
    interp.oo.borrow_mut().def_stack[index].0 = target;
    code
}

impl Interp {
    pub(super) fn oo_outer_namespace_name(
        &mut self,
        original: *mut TclObj,
        originals: &OriginalArguments,
    ) -> Result<Option<Vec<u8>>, tcl_cmd_core::CmdError> {
        originals.check_for(self)?;
        let namespace = self.oo_outer_namespace();
        let selected = self.native_namespace_object_lookup_at_using(
            namespace,
            original,
            &|| originals.check(),
            &|error| originals.retain_refusal(error.into()),
        )?;
        originals.check_for(self)?;
        Ok(selected.map(|selected| self.namespaces().qualified_name(selected)))
    }

    pub(super) fn oo_outer_namespace(&self) -> NsId {
        let mut activation = self.frames.borrow().current_activation();
        let mut namespace = self.current_ns();
        let state = self.oo.borrow();
        while let Some((_, _, _, caller, caller_activation)) = state
            .def_stack
            .iter()
            .rev()
            .find(|(_, active, _, _, _)| *active == activation)
        {
            namespace = *caller;
            activation = *caller_activation;
        }
        namespace
    }

    fn oo_definition_index(&self) -> Option<usize> {
        let activation = self.frames.borrow().current_activation();
        self.oo
            .borrow()
            .def_stack
            .iter()
            .rposition(|(_, active, _, _, _)| *active == activation)
    }

    fn oo_definition_head(&self, requested: &[u8]) -> Option<obj::Owned> {
        if requested.is_empty() || tcl_syntax::naming::is_qualified(requested) {
            return None;
        }
        let names = self.visible_command_names_in(self.current_ns());
        let selected = if let Some(exact) = names.iter().find(|name| name.as_slice() == requested) {
            exact
        } else {
            let mut candidates = names.iter().filter(|name| name.starts_with(requested));
            match (candidates.next(), candidates.next()) {
                (Some(name), None) => name,
                _ => return None,
            }
        };
        let namespaces = self.namespaces();
        let namespace = self.current_ns();
        namespaces.command_generation(namespace, selected)?;
        let slot = tcl_core_types::NativeByteCommandSlot {
            namespace: namespaces.native_context_path(namespace)?,
            simple: selected.as_slice().into(),
        };
        let full = tcl_syntax::naming::constructed_command_address_bytes(&slot);
        drop(namespaces);
        Some(obj::Owned::fresh(obj::new_string_bytes(&full)))
    }

    pub(super) fn oo_magic_definition_invoke(
        &mut self,
        requested: *mut TclObj,
        operands: &[*mut TclObj],
    ) -> Code {
        let mut original_argv = vec![requested];
        original_argv.extend_from_slice(operands);
        with_original_arguments(self, &original_argv, |interp, originals| {
            let bytes = match originals.bytes(0) {
                Ok(bytes) => bytes,
                Err(error) => return interp.report_cmd_error(error),
            };
            let head = interp
                .oo_definition_head(&bytes)
                .unwrap_or_else(|| obj::Owned::retain(requested));
            let mut arguments = vec![head.as_ptr()];
            arguments.extend_from_slice(operands);
            interp.dispatch(&arguments)
        })
    }

    pub(super) fn oo_enter_definition(
        &mut self,
        target: &DefTarget,
        argv: &[*mut TclObj],
    ) -> Result<NsId, Code> {
        let originals =
            OriginalArguments::enter(self, argv).map_err(|error| self.report_cmd_error(error))?;
        let default = match target {
            DefTarget::Class(_) => b"::oo::define".as_slice(),
            DefTarget::Object(_) => b"::oo::objdefine",
        };
        let name = self
            .definition_namespace_for(target)
            .unwrap_or_else(|| default.to_vec());
        let namespace = match tcl_runtime_api::Namespaces::find_namespace_bytes_checked(
            self,
            tcl_runtime_api::ROOT_NS,
            &name,
        ) {
            Ok(Some(namespace)) => namespace.0 as usize,
            Ok(None) => {
                return Err(
                    self.set_error(b"cannot process definitions; support namespace deleted")
                );
            }
            Err(error) => return Err(self.report_cmd_error(error.into())),
        };
        let caller = self.current_ns();
        let caller_activation = self.frames.borrow().current_activation();
        self.set_current_ns(namespace);
        self.enter_namespace_activation(namespace);
        self.frames.borrow_mut().push_namespace(namespace);
        let installed = self
            .frames
            .borrow_mut()
            .install_original_error_stack_argv(argv);
        if let Err(error) = installed {
            let popped = self.pop_native_call_frame();
            self.set_current_ns(caller);
            self.leave_namespace_activation(popped);
            return Err(self.report_cmd_error(error.into()));
        }
        if let Err(error) = originals.check() {
            let popped = self.pop_native_call_frame();
            self.set_current_ns(caller);
            self.leave_namespace_activation(popped);
            return Err(self.report_cmd_error(error));
        }
        let object = match target {
            DefTarget::Class(object) | DefTarget::Object(object) => *object,
        };
        let creation = self
            .oo
            .borrow()
            .objects
            .get(&object)
            .map(|owner| owner.creation_id);
        let activation = self.frames.borrow().current_activation();
        self.oo.borrow_mut().def_stack.push((
            target.clone(),
            activation,
            creation,
            caller,
            caller_activation,
        ));
        Ok(caller)
    }

    pub(super) fn oo_leave_definition(&mut self, caller: NsId) {
        self.oo.borrow_mut().def_stack.pop();
        let popped = self.pop_native_call_frame();
        self.set_current_ns(caller);
        self.leave_namespace_activation(popped);
    }

    pub(crate) fn oo_bind_method_activation(&self) {
        let activation = self.frames.borrow().current_activation();
        if let Some(frame) = self.oo.borrow_mut().call_stack.last_mut() {
            frame.activation = Some(activation);
        }
    }

    pub(super) fn oo_name_owner(&self, object: OoId) -> Rc<RefCell<Option<obj::Owned>>> {
        Rc::clone(&self.oo.borrow().objects[&object].cached_name)
    }

    pub(super) fn oo_original_name(&self, object: OoId) -> *mut TclObj {
        let state = self.oo.borrow();
        let alive = state.objects.contains_key(&object);
        let owner = state
            .objects
            .get(&object)
            .map(|owner| &owner.cached_name)
            .or_else(|| {
                state
                    .call_stack
                    .iter()
                    .rev()
                    .find(|frame| frame.object == object)
                    .map(|frame| &frame.name_owner)
            })
            .expect("object-name getter retains the actual OO owner");
        let mut cached = owner.borrow_mut();
        cached
            .get_or_insert_with(|| {
                let bytes = if alive { state.name(object) } else { b"" };
                let value = obj::Owned::fresh(obj::new_string_bytes(bytes));
                if alive {
                    if let Some(protocol) = self
                        .invocation_string_protocol()
                        .filter(|protocol| protocol.tcl_version().is_some())
                    {
                        obj::set_native_append_string(
                            value.as_ptr(),
                            protocol,
                            Some((
                                Rc::<[u8]>::from(bytes),
                                tcl_syntax::native_string::NativeStringStorageIdentity::Allocated,
                            )),
                            None,
                            None,
                        )
                        .expect("fresh original command-full-name append String");
                    }
                }
                value
            })
            .as_ptr()
    }
}

impl Interp {
    pub(crate) fn execute_native_oo_self(&mut self, namespace: bool) -> Code {
        let owner = self
            .oo
            .borrow()
            .call_stack
            .iter()
            .rev()
            .find(|frame| frame.activation == Some(self.frames.borrow().current_activation()))
            .map(|frame| frame.object);
        let Some(object) = owner else {
            return self.native_oo_context_required(b"self");
        };
        let original = obj::Owned::retain(self.oo_original_name(object));
        if namespace {
            // Native SELF first checks the frame and obtains its original name.
            drop(original);
            return match tcl_cmd_core::namespace::current_original(self) {
                Ok(value) => {
                    self.set_result(value);
                    Code::Ok
                }
                Err(error) => self.report_cmd_error(error),
            };
        } else {
            self.set_result(original.as_ptr());
        }
        Code::Ok
    }

    pub(crate) fn execute_native_oo_next(
        &mut self,
        words: &[*mut TclObj],
        class: bool,
        version: tcl_dialect::TclVersion,
        check: &dyn Fn(&Interp) -> Result<(), tcl_cmd_core::CmdError>,
        read_head: OriginalOoHeadReader<'_>,
    ) -> Code {
        if let Err(first) = check(self) {
            return self.report_cmd_error(first);
        }
        let head = if version == tcl_dialect::TclVersion::V9_1 {
            let Some(&original) = words.first() else {
                return self.set_error(b"missing native helper head");
            };
            let reached = read_head(self, original);
            if let Err(error) = &reached {
                if let Some(first) = error.native_execution_refusal() {
                    self.refuse_native_execution(first.clone());
                }
            }
            if let Err(first) = check(self) {
                return self.report_cmd_error(first);
            }
            let bytes = match reached {
                Ok(bytes) => bytes,
                Err(error) => return self.report_cmd_error(error),
            };
            tcl_core_types::c_string_extent(&bytes).to_vec()
        } else if class {
            b"nextto".to_vec()
        } else {
            b"next".to_vec()
        };
        // C9.1 reaches the head getter before selecting its actual context.
        // Earlier compiled C opcodes select here without that getter.
        let selected = match OriginalOoNextContext::capture(self) {
            Ok(Some(selected)) => selected,
            Ok(None) => return self.native_oo_context_required(&head),
            Err(first) => return self.report_cmd_error(first),
        };
        let current = |actual: &Interp| {
            check(actual)?;
            selected.check(actual)
        };
        if let Err(first) = current(self) {
            return self.report_cmd_error(first);
        }
        if class {
            nextto_cmd_using(self, words, Some(&current))
        } else {
            next_cmd(self, words)
        }
    }

    pub(super) fn native_oo_context_required(&mut self, head: &[u8]) -> Code {
        let mut message = head.to_vec();
        message.extend_from_slice(b" may only be called from inside a method");
        let code = crate::interp::error_code_list(&[b"TCL", b"OO", b"CONTEXT_REQUIRED"]);
        self.error_with_code(&message, &code)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_forward_prefix_tests;

#[cfg(test)]
mod original_definition_frame_tests;

#[cfg(test)]
mod residual_original_argument_tests;

impl Interp {
    /// Resolve an actual original object-name operand through the existing
    /// command-name and imported-token owners. Jim's alias compatibility path
    /// retains the stored List; it does not reconstruct member spellings.
    pub(super) fn oo_resolve_original_object(
        &mut self,
        original: *mut TclObj,
    ) -> Result<OoId, tcl_cmd_core::CmdError> {
        self.oo_resolve_original_object_using(original, None)
    }

    pub(super) fn oo_resolve_original_object_using(
        &mut self,
        original: *mut TclObj,
        extra: Option<OriginalOoCheck<'_>>,
    ) -> Result<OoId, tcl_cmd_core::CmdError> {
        let originals = OriginalArguments::enter(self, &[original])?;
        if let Some(check) = extra {
            check(self)?;
        }
        let lookup = self.oo_outer_namespace();
        let source_names = self.dialect_profile().namespace_import_binding()
            == Some(tcl_dialect::NamespaceImportBinding::SourceName);
        let mut prefix_owners: Vec<Rc<crate::interp::NativeAliasHeader>> = Vec::new();
        let mut visited = std::collections::BTreeSet::new();
        let mut operand = original;
        let mut namespace = lookup;
        let result: Result<OoId, tcl_cmd_core::CmdError> = (|| loop {
            let stage = |actual: &Interp| {
                for owner in &prefix_owners {
                    owner.checked_ptr()?;
                }
                originals
                    .check_for(actual)
                    .and_then(|()| extra.map_or(Ok(()), |check| check(actual)))
                    .map_err(|error| {
                        originals.retain_refusal(error);
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original selected OO lookup stage",
                        )
                    })
            };
            let reached = self.resolve_original_command_at_using(
                namespace,
                operand,
                extra.map(|_| {
                    &stage as &dyn Fn(&Interp) -> Result<(), tcl_syntax::value::ValueError>
                }),
            );
            if extra.is_some() {
                if let Err(error) = &reached {
                    originals.retain_refusal(error.clone().into());
                }
            }
            originals.check()?;
            if let Some(check) = extra {
                check(self)?;
            }
            let Some((command, generation)) = reached? else {
                return Ok(OoId(0));
            };
            match command {
                Command::OoObject(object) => return Ok(object),
                Command::Alias {
                    original_prefix, ..
                } if source_names => {
                    let generation = generation.ok_or(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original OO alias command generation",
                        ),
                    )?;
                    if !visited.insert(generation) {
                        return Ok(OoId(0));
                    }
                    operand = original_prefix.checked_ptr()?;
                    prefix_owners.push(original_prefix);
                    namespace = GLOBAL;
                }
                Command::Imported { .. } => {
                    let generation = generation.ok_or(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original OO imported command generation",
                        ),
                    )?;
                    let origin = self.original_import_origin_command(generation);
                    originals.check()?;
                    if let Some(check) = extra {
                        check(self)?;
                    }
                    return Ok(match origin {
                        Some(Command::OoObject(object)) => object,
                        _ => OoId(0),
                    });
                }
                _ => return Ok(OoId(0)),
            }
        })();
        if let Err(error) = &result {
            if let Some(first) = error.native_execution_refusal() {
                self.refuse_native_execution(first.clone());
            }
        }
        // Any final stored-prefix retirement is outside namespace/OO borrows.
        drop(prefix_owners);
        originals.check()?;
        if let Some(check) = extra {
            check(self)?;
        }
        result
    }

    /// Explicit provided-byte naming APIs create a fresh operand and delegate
    /// the same resolver. They convey no pre-existing original-header cache.
    pub(super) fn oo_resolve_object_bytes(
        &mut self,
        name: &[u8],
    ) -> Result<OoId, tcl_cmd_core::CmdError> {
        let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(self)?;
        let original = obj::Owned::fresh(obj::new_string_bytes(name));
        let result = self.oo_resolve_original_object(original.as_ptr());
        if let Err(error) = &result {
            if let Some(first) = error.native_execution_refusal() {
                self.refuse_native_execution(first.clone());
            }
        }
        drop(original);
        scope.currency().ensure_current_or_refuse()?;
        result
    }
}

#[cfg(test)]
impl Interp {
    /// Existing positive fixture convenience, delegating actual checked lookup.
    /// A failed lookup remains an assertion failure, never a production miss.
    pub(super) fn oo_resolve_object(&self, name: &[u8]) -> OoId {
        let mut original = self.clone();
        original
            .oo_resolve_object_bytes(name)
            .expect("supported original OO fixture lookup")
    }
}

#[cfg(test)]
#[path = "native_context/original_next_stages_tests.rs"]
mod original_next_stages_tests;
