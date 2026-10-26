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

use super::*;
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
            tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation,
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
                        .native_invocation_dialect()
                        .native_string_protocol()
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
mod tests {
    use super::*;

    #[test]
    fn definition_head_uses_current_constructed_namespace_and_selected_tail() {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("tcl9.0"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(interp.eval_str(b"proc choice {} {}; namespace eval N {proc literal: {} {}; proc choice {} {}; proc chosen {} {}}"), Code::Ok);
        let root = interp.oo_definition_head(b"choice").unwrap();
        assert_eq!(obj::bytes_of(root.as_ptr()), b"::choice");
        let namespace = interp.find_namespace_id(b"::N").unwrap();
        interp.set_current_ns(namespace);
        let exact = interp.oo_definition_head(b"choice").unwrap();
        assert_eq!(obj::bytes_of(exact.as_ptr()), b"::N::choice");
        let literal = interp.oo_definition_head(b"lit").unwrap();
        assert_eq!(obj::bytes_of(literal.as_ptr()), b"::N::literal:");
        assert!(interp.oo_definition_head(b"cho").is_none());
        assert!(interp.oo_definition_head(b"").is_none());
        assert!(interp.oo_definition_head(b"::choice").is_none());
        interp.set_current_ns(crate::namespace::GLOBAL);
        let restored = interp.oo_definition_head(b"choice").unwrap();
        assert_eq!(obj::bytes_of(restored.as_ptr()), b"::choice");
        assert!(!interp.host_refusal_pending());
    }

    #[test]
    fn self_call_uses_the_selected_uplevel_activation_and_restores_its_caller() {
        for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(version),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            assert_eq!(
                interp.eval_str(
                    br"oo::class create Outer {method run {peer} {$peer inspect}}
oo::class create Inner {method inspect {} {
    set direct [self call]
    set caller [uplevel 1 {::oo::Helpers::self call}]
    list $direct $caller [self call]
}}
Outer create outer
Inner create inner
outer run inner"
                ),
                Code::Ok,
                "{version}: {:?}",
                interp.result_bytes()
            );
            assert_eq!(
                interp.result_bytes(),
                b"{{{method inspect ::Inner method}} 0} {{{method run ::Outer method}} 0} {{{method inspect ::Inner method}} 0}",
                "{version}"
            );
            assert_eq!(
                interp.eval_str(b"proc ordinary {} {::oo::Helpers::self call}; ordinary"),
                Code::Error,
                "{version}"
            );
            assert_eq!(interp.error_code(), b"TCL OO CONTEXT_REQUIRED", "{version}");
            assert_eq!(
                interp.result_bytes(),
                b"::oo::Helpers::self may only be called from inside a method",
                "{version}"
            );
            assert_eq!(interp.eval_str(b"namespace current"), Code::Ok);
            assert_eq!(interp.error_code(), b"NONE", "{version}");
        }
    }

    #[test]
    fn dynamic_self_subcommands_use_the_original_native_index_table() {
        for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(version),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            assert_eq!(
                interp.eval_str(b"oo::class create C {method query {which} {self $which}}; C create obj; obj query o"),
                Code::Ok
            );
            assert_eq!(interp.result_bytes(), b"::obj");
            assert_eq!(
                interp.eval_str(b"string match ::oo::Obj* [obj query nam]"),
                Code::Ok
            );
            assert_eq!(interp.result_bytes(), b"1");
            for (script, error_code) in [
                (
                    b"obj query c".as_slice(),
                    b"TCL LOOKUP INDEX subcommand c".as_slice(),
                ),
                (b"obj query n", b"TCL LOOKUP INDEX subcommand n"),
                (b"obj query {}", b"TCL LOOKUP INDEX subcommand {}"),
                (b"obj query missing", b"TCL LOOKUP INDEX subcommand missing"),
            ] {
                assert_eq!(
                    interp.eval_str(script),
                    Code::Error,
                    "{version}: {script:?}"
                );
                assert_eq!(interp.error_code(), error_code, "{version}: {script:?}");
            }
        }
    }

    #[test]
    fn helper_artifact_executes_only_the_actual_method_frame_and_original_words() {
        for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(version),
                tcl_registry::special_vars::NativeBootstrapInputs {
                    package_path: Vec::new(),
                    default_library: None,
                },
            )
            .unwrap();
            assert_eq!(interp.eval_str(br"oo::class create B {method m args {list BASE {*}$args}; method by args {list BASE {*}$args}; method static args {list STATIC {*}$args}}
oo::class create C {
 superclass B
 method m args {next {*}$args}
 method by args {nextto B {*}$args}
 method static {} {nextto {*}{B ONE}}
 method object {} {self o}
 method space {} {self n}
 method nested {} {ordinary}
}
proc ordinary {} {::oo::Helpers::self}
C create obj
list [obj m A B] [obj by C D] [obj static] [obj object] [string match ::oo::Obj* [obj space]] [catch {obj nested}] [llength [info commands ::self]]
"),Code::Ok,"{version}: {:?}",interp.result_bytes());
            assert_eq!(
                interp.result_bytes(),
                b"{BASE A B} {BASE C D} {STATIC ONE} ::obj 1 1 0",
                "{version}"
            );
            assert_eq!(interp.eval_str(b"obj nested"), Code::Error);
            assert_eq!(interp.error_code(), b"TCL OO CONTEXT_REQUIRED");
            assert_eq!(
                interp.eval_str(
                    b"oo::define C method evalContext {} {my eval {self object}}; obj evalContext"
                ),
                Code::Ok
            );
            assert_eq!(interp.result_bytes(), b"::obj");
        }
    }

    #[test]
    fn definition_namespace_owns_its_scope_and_original_workers() {
        let mut interp = Interp::new();
        let script = b"proc ::method args {error GLOBAL_WORKER}; namespace eval caller {proc build {} {set private CALLER; oo::class create ::C {set scope [namespace current]; set inherited [info exists private]; set local [uplevel 1 {set private}]; proc born {} {return HERE}; meth m {} {return OK}}}}; caller::build; C create obj; list [obj m] $::oo::define::scope $::oo::define::inherited $::oo::define::local [::oo::define::born]";
        assert_eq!(
            interp.eval_str(script),
            Code::Ok,
            "{:?}",
            interp.result_bytes()
        );
        assert_eq!(interp.result_bytes(), b"OK ::oo::define 0 CALLER HERE");
        assert_eq!(
            interp.eval_str(b"::oo::define::method m {} {}"),
            Code::Error
        );
        assert_eq!(interp.error_code(), b"TCL OO MONKEY_BUSINESS");
        assert_eq!(interp.current_ns(), GLOBAL);
    }

    #[test]
    fn nested_definitions_retain_actual_caller_frames_and_qualified_workers() {
        for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(version),
                tcl_registry::special_vars::NativeBootstrapInputs {
                    package_path: Vec::new(),
                    default_library: None,
                },
            )
            .unwrap();
            assert_eq!(interp.eval_str(br"namespace eval ::outer {
 oo::class create B {}
 oo::class create C {}
 oo::class create D {}
 proc build {} {
  oo::define C {oo::define ::outer::D {superclass B; uplevel 1 {::oo::define::method resumed {} {return OUTER}}}}
 }
 build
}
::outer::C create obj
set chained [list [info class superclasses ::outer::D] [obj resumed]]
oo::class create ::oo::define::B {}
proc ::oo::define::bridge {} {oo::define ::outer::D {superclass B}}
oo::define ::outer::C {bridge}
set ordinary [info class superclasses ::outer::D]
proc ::inspect args {error GLOBAL}
proc ::oo::define::inspect args {list [lindex [info level 0] 0] {*}$args}
set magic [list [oo::define ::outer::C insp FIRST SECOND] [oo::define ::outer::C inspect THIRD]]
set outside [catch {::oo::define::method outside {} {}} result options]
list $chained $ordinary $magic [namespace current] $outside [dict get $options -errorcode]"), Code::Ok, "{version}: {:?}", interp.result_bytes());
            assert_eq!(interp.result_bytes(), b"{::outer::B OUTER} ::oo::define::B {{::oo::define::inspect FIRST SECOND} {::oo::define::inspect THIRD}} :: 1 {TCL OO MONKEY_BUSINESS}");
            assert!(interp.oo.borrow().def_stack.is_empty());
        }
    }

    #[test]
    fn cached_self_name_is_the_same_header_and_nested_procs_have_no_context() {
        let mut interp = Interp::new();
        assert_eq!(interp.eval_str(b"oo::class create C {method object {} {self object}; method nested {} {p}; method shifted {} {uplevel #0 {::oo::Helpers::self}}}; proc p {} {::oo::Helpers::self}; C create obj; obj object"), Code::Ok);
        let original = obj::Owned::retain(interp.get_obj_result());
        list::list_elements(original.as_ptr()).unwrap();
        assert_eq!(interp.eval_str(b"obj object"), Code::Ok);
        assert_eq!(original.as_ptr(), interp.get_obj_result());
        assert!(core::ptr::eq(
            obj::obj_type_ptr(interp.get_obj_result()),
            &list::TCL_LIST_TYPE
        ));
        assert_eq!(interp.eval_str(b"obj nested"), Code::Error);
        assert_eq!(interp.eval_str(b"obj shifted"), Code::Error);
        assert_eq!(
            interp.eval_str(b"rename obj renamed; renamed object"),
            Code::Ok
        );
        assert_ne!(original.as_ptr(), interp.get_obj_result());
        let renamed = obj::Owned::retain(interp.get_obj_result());
        assert_eq!(
            interp.eval_str(
                b"oo::define C method deleted {} {rename [self] {}; self}; renamed deleted"
            ),
            Code::Ok
        );
        assert_eq!(interp.result_bytes(), b"");
        assert_eq!(obj_bytes(original.as_ptr()), b"::obj");
        assert_eq!(obj_bytes(renamed.as_ptr()), b"::renamed");
    }
}

#[cfg(test)]
mod original_forward_prefix_tests {
    use super::*;
    use std::cell::Cell;

    thread_local! {
        static OWNER: RefCell<Option<crate::interp::WeakInterp>> = const { RefCell::new(None) };
        static UPDATES: Cell<usize> = const { Cell::new(0) };
        static EXPECTED: Cell<usize> = const { Cell::new(0) };
        static MODE: Cell<u8> = const { Cell::new(0) };
        static REACHED_RESULT: Cell<usize> = const { Cell::new(0) };
        static CAPTURED: RefCell<Vec<(usize, isize, bool)>> = const { RefCell::new(Vec::new()) };
    }
    extern "C" fn update(original: *mut TclObj) {
        UPDATES.with(|calls| calls.set(calls.get() + 1));
        let mut interpreter =
            OWNER.with(|owner| owner.borrow().as_ref().unwrap().upgrade().unwrap());
        match MODE.with(Cell::get) {
            1 => {
                interpreter.refuse_host_command("first original forward member updater");
            }
            2 => {
                let original = interpreter.runtime_context();
                let mut changed = original.clone();
                changed
                    .packages
                    .push(("oo-prefix-stage".to_owned(), "1.0".to_owned()));
                interpreter.pin_context(&changed).unwrap();
                interpreter.pin_context(&original).unwrap();
            }
            _ => {}
        }
        unsafe { obj::set_native_updater_string_rep(original, b"prefix-data", false) };
    }
    static MEMBER: obj::TclObjType = obj::TclObjType {
        name: c"originalForwardMember".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(update),
        set_from_any_proc: None,
    };
    fn word(bytes: &[u8]) -> obj::Owned {
        obj::Owned::fresh(obj::new_string_bytes(bytes))
    }
    fn capture(interpreter: &mut Interp, argv: &[*mut TclObj]) -> Code {
        assert_eq!(argv.len(), 2);
        assert_eq!(argv[1] as usize, EXPECTED.with(Cell::get));
        let before = (
            UPDATES.with(Cell::get),
            unsafe { (*argv[1]).ref_count },
            obj::has_string_rep(argv[1]),
        );
        CAPTURED.with(|rows| rows.borrow_mut().push(before));
        if MODE.with(Cell::get) != 0 {
            interpreter.set_result_bytes(b"REACHED WORKER RESULT");
            REACHED_RESULT.with(|result| result.set(interpreter.get_obj_result() as usize));
        }
        let bytes = match interpreter.native_object_string_bytes(argv[1]) {
            Ok(bytes) => bytes,
            Err(error) => return interpreter.report_cmd_error(error.into()),
        };
        assert_eq!(bytes.as_ref(), b"prefix-data");
        let after = (
            UPDATES.with(Cell::get),
            unsafe { (*argv[1]).ref_count },
            obj::has_string_rep(argv[1]),
        );
        CAPTURED.with(|rows| rows.borrow_mut().push(after));
        interpreter.set_result_bytes(b"CAPTURE");
        Code::Ok
    }
    fn native(version: &str) -> Interp {
        let mut interpreter = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(version),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(
            interpreter.eval_str(b"oo::class create PrefixClass; PrefixClass create PrefixObject"),
            Code::Ok
        );
        interpreter.register_builtin(b"capture", capture);
        OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
        UPDATES.with(|calls| calls.set(0));
        MODE.with(|mode| mode.set(0));
        CAPTURED.with(|rows| rows.borrow_mut().clear());
        interpreter
    }
    fn define(interpreter: &mut Interp, member: *mut TclObj) -> OoId {
        let target = interpreter.oo_resolve_object(b"PrefixClass");
        let context = [word(b"oo::define"), word(b"PrefixClass"), word(b"")];
        let original: Vec<_> = context.iter().map(obj::Owned::as_ptr).collect();
        let caller = interpreter
            .oo_enter_definition(&DefTarget::Class(target), &original)
            .unwrap();
        let argv = [word(b"forward"), word(b"go"), word(b"capture")];
        let mut pointers: Vec<_> = argv.iter().map(obj::Owned::as_ptr).collect();
        pointers.push(member);
        assert_eq!(def_forward(interpreter, &pointers), Code::Ok);
        interpreter.oo_leave_definition(caller);
        target
    }
    fn stream(version: &str) -> &'static str {
        match version {
            "tcl8.6" => include_str!("../../../../rust/tcl-registry/tests/data/native_oo_operand_stages_original/capture/8.6.18/execute.stdout"),
            "tcl9.0" => include_str!("../../../../rust/tcl-registry/tests/data/native_oo_operand_stages_original/capture/9.0.4/execute.stdout"),
            "tcl9.1" => include_str!("../../../../rust/tcl-registry/tests/data/native_oo_operand_stages_original/capture/9.1.0/execute.stdout"),
            _ => unreachable!(),
        }
    }
    fn fields(version: &str, case: usize, stage: &str) -> Vec<(&'static str, &'static str)> {
        stream(version)
            .lines()
            .filter(|line| line.starts_with("ROW\t"))
            .map(|line| {
                line.split('\t')
                    .skip(1)
                    .filter_map(|field| field.split_once('='))
                    .collect::<Vec<_>>()
            })
            .find(|fields| {
                fields
                    .iter()
                    .any(|&(key, value)| key == "case" && value == case.to_string())
                    && fields
                        .iter()
                        .any(|&(key, value)| key == "stage" && value == stage)
            })
            .expect("one exact complete original ROW")
    }
    fn original_snapshot(version: &str, case: usize, stage: &str) -> (usize, isize, bool) {
        let fields = fields(version, case, stage);
        let get = |wanted| {
            fields
                .iter()
                .find_map(|&(key, value)| (key == wanted).then_some(value))
                .unwrap()
        };
        assert_eq!(get("status"), "0");
        (
            get("r0_update").parse().unwrap(),
            get("r0_refs").parse().unwrap(),
            get("r0_string") == "1",
        )
    }
    fn snapshot(original: *mut TclObj) -> (usize, isize, bool) {
        (
            UPDATES.with(Cell::get),
            unsafe { (*original).ref_count },
            obj::has_string_rep(original),
        )
    }
    fn prefix(interpreter: &Interp, target: OoId) -> obj::Owned {
        let Some(Method::Forward { prefix }) =
            interpreter.oo.borrow().classes[&target].methods.get(b"go")
        else {
            panic!("retained original forward");
        };
        prefix
    }

    #[test]
    fn original_forward_definition_and_raw_report_match_original_member_stages() {
        // Fixed C86/C90/C91 ROW comparisons; presentation is an explicit later read.
        for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interpreter = native(version);
            let member = obj::Owned::fresh(obj::alloc_typed(&MEMBER, 0));
            assert_eq!(
                snapshot(member.as_ptr()),
                original_snapshot(version, 0, "initial")
            );
            let target = define(&mut interpreter, member.as_ptr());
            assert_eq!(
                snapshot(member.as_ptr()),
                original_snapshot(version, 0, "forward-defined")
            );
            let stored = prefix(&interpreter, target);
            let argv: Vec<_> = [
                b"info".as_slice(),
                b"class",
                b"forward",
                b"PrefixClass",
                b"go",
            ]
            .iter()
            .map(|bytes| word(bytes))
            .collect();
            let pointers: Vec<_> = argv.iter().map(obj::Owned::as_ptr).collect();
            assert_eq!(info_class(&mut interpreter, &pointers), Code::Ok);
            assert_eq!(interpreter.get_obj_result(), stored.as_ptr());
            assert_eq!(
                snapshot(member.as_ptr()),
                original_snapshot(version, 0, "info-forward-returned")
            );
            let members = interpreter.list_elements(&stored.as_ptr()).unwrap();
            assert_eq!(members.len(), 2);
            assert_eq!(members[1], member.as_ptr());
            assert_eq!(
                snapshot(member.as_ptr()),
                original_snapshot(version, 0, "info-forward-list-accessed")
            );
            assert!(stream(version)
                .lines()
                .any(|line| line == "PREFIX\tcase=0\tstatus=0\tcount=2\toriginal_member=1"));
            let bytes = interpreter
                .native_object_string_bytes(stored.as_ptr())
                .unwrap();
            assert_eq!(bytes.as_ref(), b"capture prefix-data");
            assert_eq!(
                snapshot(member.as_ptr()),
                original_snapshot(version, 0, "info-forward-presented")
            );
        }
    }

    #[test]
    fn original_forward_invocation_passes_exact_member_before_its_worker_getter() {
        for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interpreter = native(version);
            let member = obj::Owned::fresh(obj::alloc_typed(&MEMBER, 0));
            define(&mut interpreter, member.as_ptr());
            EXPECTED.with(|expected| expected.set(member.as_ptr() as usize));
            let argv = [word(b"PrefixObject"), word(b"go")];
            assert_eq!(
                interpreter.dispatch(&argv.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>()),
                Code::Ok
            );
            assert_eq!(
                CAPTURED.with(|rows| rows.borrow().clone()),
                vec![
                    original_snapshot(version, 1, "capture-before-read"),
                    original_snapshot(version, 1, "capture-after-read")
                ]
            );
            assert_eq!(
                snapshot(member.as_ptr()),
                original_snapshot(version, 1, "forward-invoked-without-report")
            );
            assert!(stream(version)
                .lines()
                .any(|line| line == "CALL\tcase=1\tkind=forward\targc=2\toriginal_member=1"));
            assert_eq!(interpreter.entered_native_operation_count(), 0);
        }
    }

    #[test]
    fn forward_declaration_clone_shares_the_retained_list_without_member_reference_donation() {
        // Software sharing control for original CloneForwardMethod's source recipe;
        // original nine-case rows contain no independent native clone observation.
        let mut interpreter = native("tcl8.6");
        let member = obj::Owned::fresh(obj::alloc_typed(&MEMBER, 0));
        let target = define(&mut interpreter, member.as_ptr());
        let stored = prefix(&interpreter, target);
        let copied = interpreter.oo.borrow().classes[&target]
            .methods
            .duplicate_declarations()
            .unwrap();
        let Some(Method::Forward { prefix: cloned }) = copied.get(b"go") else {
            panic!("forward clone");
        };
        assert_eq!(cloned.as_ptr(), stored.as_ptr());
        assert_eq!(snapshot(member.as_ptr()), (0, 2, false));
        let removed = interpreter
            .oo
            .borrow_mut()
            .classes
            .get_mut(&target)
            .unwrap()
            .methods
            .remove(b"go");
        drop(removed);
        assert_eq!(snapshot(member.as_ptr()), (0, 2, false));
        drop(copied);
        drop(stored);
        assert_eq!(snapshot(member.as_ptr()), (0, 2, false));
        drop(cloned);
        assert_eq!(snapshot(member.as_ptr()), (0, 1, false));
    }

    #[test]
    fn forward_worker_member_refusal_keeps_reached_bytes_and_stops_result_publication() {
        for mode in [1, 2] {
            let mut interpreter = native("tcl8.6");
            let member = obj::Owned::fresh(obj::alloc_typed(&MEMBER, 0));
            define(&mut interpreter, member.as_ptr());
            EXPECTED.with(|expected| expected.set(member.as_ptr() as usize));
            interpreter.set_result_bytes(b"PRIOR");
            MODE.with(|selected| selected.set(mode));
            let argv = [word(b"PrefixObject"), word(b"go")];
            assert_eq!(
                interpreter.dispatch(&argv.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>()),
                Code::Error
            );
            assert!(interpreter.native_execution_refusal().is_some());
            assert_eq!(
                interpreter.get_obj_result() as usize,
                REACHED_RESULT.with(Cell::get)
            );
            assert_eq!(
                obj_bytes(interpreter.get_obj_result()),
                b"REACHED WORKER RESULT"
            );
            assert_eq!(snapshot(member.as_ptr()), (1, 2, true));
            assert_eq!(interpreter.entered_native_operation_count(), 0);
        }
    }
}

#[cfg(test)]
mod original_definition_frame_tests {
    use super::*;
    use std::cell::Cell;
    use tcl_runtime_api::Introspect;

    thread_local! {
        static OWNER: RefCell<Option<crate::interp::WeakInterp>> = const { RefCell::new(None) };
        static UPDATES: Cell<usize> = const { Cell::new(0) };
        static FREES: Cell<usize> = const { Cell::new(0) };
    }
    extern "C" fn update(original: *mut TclObj) {
        UPDATES.with(|calls| calls.set(calls.get() + 1));
        let interpreter = OWNER.with(|owner| owner.borrow().as_ref().unwrap().upgrade().unwrap());
        assert!(interpreter.frames.try_borrow_mut().is_ok());
        unsafe { obj::set_native_updater_string_rep(original, b"frame-data", false) };
    }
    extern "C" fn free(_original: *mut TclObj) {
        FREES.with(|calls| calls.set(calls.get() + 1));
    }
    static MEMBER: obj::TclObjType = obj::TclObjType {
        name: c"originalDefinitionFrameMember".as_ptr(),
        free_int_rep_proc: Some(free),
        dup_int_rep_proc: None,
        update_string_proc: Some(update),
        set_from_any_proc: None,
    };
    fn native(version: &str) -> Interp {
        let mut interpreter = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(version),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(
            interpreter.eval_str(b"oo::class create FrameClass"),
            Code::Ok
        );
        OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
        UPDATES.with(|calls| calls.set(0));
        FREES.with(|calls| calls.set(0));
        interpreter
    }
    fn word(bytes: &[u8]) -> obj::Owned {
        obj::Owned::fresh(obj::new_string_bytes(bytes))
    }
    fn stream(version: &str) -> &'static str {
        match version {
            "tcl8.6" => include_str!("../../../../rust/tcl-registry/tests/data/native_oo_operand_stages_original/capture/8.6.18/execute.stdout"),
            "tcl9.0" => include_str!("../../../../rust/tcl-registry/tests/data/native_oo_operand_stages_original/capture/9.0.4/execute.stdout"),
            "tcl9.1" => include_str!("../../../../rust/tcl-registry/tests/data/native_oo_operand_stages_original/capture/9.1.0/execute.stdout"),
            _ => unreachable!(),
        }
    }
    fn row(version: &str, case: usize, stage: &str) -> (usize, isize, bool) {
        let fields = stream(version)
            .lines()
            .filter(|line| line.starts_with("ROW\t"))
            .map(|line| {
                line.split('\t')
                    .skip(1)
                    .filter_map(|field| field.split_once('='))
                    .collect::<Vec<_>>()
            })
            .find(|fields| {
                fields.contains(&("case", case.to_string().as_str()))
                    && fields.contains(&("stage", stage))
            })
            .expect("one exact complete native ROW");
        let field = |name| {
            fields
                .iter()
                .find_map(|&(key, value)| (key == name).then_some(value))
                .unwrap()
        };
        assert_eq!(field("status"), "0");
        (
            field("r0_update").parse().unwrap(),
            field("r0_refs").parse().unwrap(),
            field("r0_string") == "1",
        )
    }
    fn snapshot(original: *mut TclObj) -> (usize, isize, bool) {
        (
            UPDATES.with(Cell::get),
            unsafe { (*original).ref_count },
            obj::has_string_rep(original),
        )
    }

    #[test]
    fn original_definition_entry_waits_for_a_report_and_preserves_raw_list_members() {
        // Match String/update phases and the report's native member +1 delta.
        // The direct software frame fixture has no public EvalObjv argv +1 role.
        for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            for case in [2, 3, 4] {
                let mut interpreter = native(version);
                let target = interpreter.oo_resolve_object(b"FrameClass");
                let argv = [
                    word(b"oo::define"),
                    word(b"FrameClass"),
                    word(b"observer"),
                    obj::Owned::fresh(obj::alloc_typed(&MEMBER, 0)),
                ];
                let original: Vec<_> = argv.iter().map(obj::Owned::as_ptr).collect();
                let member = original[3];
                let initial = snapshot(member);
                let operation = OriginalArguments::enter(&interpreter, &original).unwrap();
                let caller = interpreter
                    .oo_enter_definition(&DefTarget::Class(target), &original)
                    .unwrap();
                let entered = snapshot(member);
                let expected_entered = row(version, case, "definition-entered");
                assert_eq!(
                    (entered.0, entered.2),
                    (expected_entered.0, expected_entered.2)
                );
                assert_eq!(
                    entered.1, initial.1,
                    "frame tape owns allocation safety, not a native reference"
                );
                assert!(interpreter.frames.borrow().words_at(1).unwrap().is_empty());
                if case == 2 {
                    interpreter.oo_leave_definition(caller);
                    let returned = row(version, case, "definition-returned");
                    assert_eq!(
                        (snapshot(member).0, snapshot(member).2),
                        (returned.0, returned.2)
                    );
                    assert_eq!(snapshot(member).1, initial.1);
                } else {
                    let report = obj::Owned::fresh(
                        Introspect::level_argv(&interpreter, 1).unwrap().unwrap(),
                    );
                    let expected_report = row(version, case, "definition-info-level-returned");
                    let reported = snapshot(member);
                    assert_eq!(
                        (reported.0, reported.2),
                        (expected_report.0, expected_report.2)
                    );
                    assert_eq!(
                        reported.1 - entered.1,
                        expected_report.1 - expected_entered.1
                    );
                    let members = interpreter.list_elements(&report.as_ptr()).unwrap();
                    assert_eq!(members, original);
                    assert!(stream(version).lines().any(|line| line
                        == format!("REPORT\tcase={case}\tstatus=0\tcount=4\toriginal_member=1")));
                    if case == 4 {
                        let bytes = interpreter
                            .native_object_string_bytes(report.as_ptr())
                            .unwrap();
                        assert_eq!(bytes.as_ref(), b"oo::define FrameClass observer frame-data");
                        let expected = row(version, case, "definition-info-presented");
                        assert_eq!(
                            (snapshot(member).0, snapshot(member).2),
                            (expected.0, expected.2)
                        );
                    }
                    interpreter.oo_leave_definition(caller);
                    assert_eq!(
                        snapshot(member).1,
                        initial.1 + 1,
                        "only the returned List owns a native member reference"
                    );
                    drop(report);
                    assert_eq!(snapshot(member).1, initial.1);
                }
                assert!(operation.check().is_ok());
                drop(operation);
                assert_eq!(interpreter.entered_native_operation_count(), 0);
            }
        }
    }

    #[test]
    fn original_frame_allocation_lease_cannot_revive_a_retired_member() {
        // Allocation-only frame retention is independent of native header liveness.
        let interpreter = native("tcl8.6");
        let member = obj::Owned::fresh(obj::alloc_typed(&MEMBER, 0));
        let pointer = member.as_ptr();
        interpreter
            .frames
            .borrow_mut()
            .push(crate::namespace::GLOBAL);
        interpreter
            .frames
            .borrow_mut()
            .install_original_error_stack_argv(&[pointer])
            .unwrap();
        assert_eq!(snapshot(pointer), (0, 1, false));
        drop(member);
        assert_eq!(FREES.with(Cell::get), 1);
        assert!(obj::check_native_liveness(pointer).is_err());
        let error = match Introspect::level_argv(&interpreter, 1) {
            Err(error) => error,
            Ok(_) => panic!("a lease cannot revive the retired original header"),
        };
        assert_eq!(
            error,
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "retired native object"
                )
            )
        );
        interpreter.frames.borrow_mut().pop();
        assert_eq!(FREES.with(Cell::get), 1);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }

    #[test]
    fn shared_info_level_report_refusal_stops_before_bad_level_string_conversion() {
        // The genuine shared command receives the role's outer failure; only
        // an absent inner frame may reach bad-level String formatting.
        let mut interpreter = native("tcl8.6");
        let member = obj::Owned::fresh(obj::alloc_typed(&MEMBER, 0));
        interpreter
            .frames
            .borrow_mut()
            .push(crate::namespace::GLOBAL);
        interpreter
            .frames
            .borrow_mut()
            .install_original_error_stack_argv(&[member.as_ptr()])
            .unwrap();
        drop(member);
        let index = obj::Owned::fresh(obj::new_wide_int_obj(0));
        assert!(!obj::has_string_rep(index.as_ptr()));
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        let error = match tcl_cmd_core::info::level(&mut interpreter, Some(&index.as_ptr())) {
            Err(error) => error,
            Ok(_) => panic!("retired original report must refuse"),
        };
        let expected = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "retired native object",
            ),
        );
        assert_eq!(error.native_execution_refusal(), Some(&expected));
        assert!(
            !obj::has_string_rep(index.as_ptr()),
            "outer report failure must not format the level argument"
        );
        assert_eq!(interpreter.report_cmd_error(error), Code::Error);
        assert_eq!(interpreter.native_execution_refusal(), Some(expected));
        assert_eq!(interpreter.get_obj_result(), prior);
        interpreter.frames.borrow_mut().pop();
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

#[cfg(test)]
mod residual_original_argument_tests {
    use super::*;
    use std::cell::Cell;

    thread_local! {
        static OWNER: RefCell<Option<crate::interp::WeakInterp>> = const { RefCell::new(None) };
        static MODE: Cell<u8> = const { Cell::new(0) };
        static UPDATES: Cell<usize> = const { Cell::new(0) };
        static PAYLOAD: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    }
    fn native(version: &str) -> Interp {
        let mut interpreter = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(version),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(
            interpreter.eval_str(b"oo::class create OriginalResidualClass"),
            Code::Ok
        );
        OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
        MODE.with(|mode| mode.set(0));
        UPDATES.with(|calls| calls.set(0));
        interpreter
    }
    fn word(bytes: &[u8]) -> obj::Owned {
        obj::Owned::fresh(obj::new_string_bytes(bytes))
    }
    extern "C" fn update(original: *mut TclObj) {
        UPDATES.with(|calls| calls.set(calls.get() + 1));
        let mut interpreter =
            OWNER.with(|owner| owner.borrow().as_ref().unwrap().upgrade().unwrap());
        match MODE.with(Cell::get) {
            1 => {
                interpreter.refuse_host_command("first residual OO original getter");
            }
            2 => {
                let original = interpreter.runtime_context();
                let mut changed = original.clone();
                changed
                    .packages
                    .push(("residual-oo-stage".to_owned(), "1".to_owned()));
                interpreter.pin_context(&changed).unwrap();
                interpreter.pin_context(&original).unwrap();
            }
            3 => {
                assert_eq!(
                    interpreter.eval_str(b"OriginalResidualClass destroy"),
                    Code::Ok
                );
            }
            _ => {}
        }
        let bytes = PAYLOAD.with(|payload| payload.borrow().clone());
        unsafe { obj::set_native_updater_string_rep(original, &bytes, false) };
    }
    static ORIGINAL: obj::TclObjType = obj::TclObjType {
        name: c"residualOriginalOoArgument".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(update),
        set_from_any_proc: None,
    };
    fn original(payload: &[u8], mode: u8) -> obj::Owned {
        MODE.with(|selected| selected.set(mode));
        PAYLOAD.with(|selected| *selected.borrow_mut() = payload.to_vec());
        obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0))
    }
    fn definition(interpreter: &mut Interp) -> (OoId, NsId) {
        let target = interpreter.oo_resolve_object(b"OriginalResidualClass");
        let words = [
            word(b"oo::define"),
            word(b"OriginalResidualClass"),
            word(b""),
        ];
        let pointers: Vec<_> = words.iter().map(obj::Owned::as_ptr).collect();
        let caller = interpreter
            .oo_enter_definition(&DefTarget::Class(target), &pointers)
            .unwrap();
        (target, caller)
    }

    #[test]
    fn class_factory_original_name_refusal_stops_namespace_and_later_operand_effects() {
        // Software original getter/currency control; no stock factory-order observation.
        for mode in [1, 2] {
            let mut interpreter = native("tcl8.6");
            let class = interpreter.oo_resolve_object(b"OriginalResidualClass");
            let name = original(b"CreatedResidual", mode);
            let later = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
            let head = word(b"OriginalResidualClass");
            let selector = word(b"create");
            let tape = [
                head.as_ptr(),
                selector.as_ptr(),
                name.as_ptr(),
                later.as_ptr(),
            ];
            interpreter.set_result_bytes(b"PRIOR");
            let prior = interpreter.get_obj_result();
            assert_eq!(
                interpreter.oo_class_factory(
                    class,
                    b"OriginalResidualClass",
                    b"create",
                    &tape[2..],
                    false,
                    Some(&tape)
                ),
                Some(Code::Error)
            );
            assert_eq!(UPDATES.with(Cell::get), 1);
            assert!(!obj::has_string_rep(later.as_ptr()));
            assert_eq!(interpreter.get_obj_result(), prior);
            assert!(interpreter.native_execution_refusal().is_some());
            assert!(!interpreter
                .oo
                .borrow()
                .names
                .values()
                .any(|name| name == b"::CreatedResidual"));
            assert_eq!(unsafe { (*name.as_ptr()).ref_count }, 1);
            assert_eq!(unsafe { (*later.as_ptr()).ref_count }, 1);
            assert_eq!(interpreter.entered_native_operation_count(), 0);
        }
    }

    #[test]
    fn original_selector_refusal_stops_cache_dispatch_and_keeps_later_header_unread() {
        for mode in [0, 1, 2] {
            let mut interpreter = native("tcl8.6");
            assert_eq!(interpreter.eval_str(b"oo::define OriginalResidualClass method m {arg} {return CALLED}; OriginalResidualClass create residualObject"), Code::Ok);
            let target = interpreter.oo_resolve_object(b"residualObject");
            let selector = original(b"m", mode);
            let later = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
            let head = word(b"residualObject");
            let tape = [head.as_ptr(), selector.as_ptr(), later.as_ptr()];
            interpreter.set_result_bytes(b"PRIOR");
            let prior = interpreter.get_obj_result();
            let code = interpreter.oo_invoke_with_original(
                target,
                b"m",
                &tape[2..],
                true,
                Some(b"residualObject"),
                Some(&tape),
            );
            assert_eq!(UPDATES.with(Cell::get), 1);
            assert!(!obj::has_string_rep(later.as_ptr()));
            if mode == 0 {
                assert_eq!(code, Code::Ok);
                assert_eq!(interpreter.result_bytes(), b"CALLED");
            } else {
                assert_eq!(code, Code::Error);
                assert_eq!(interpreter.get_obj_result(), prior);
                assert!(interpreter.native_execution_refusal().is_some());
                assert!(core::ptr::eq(
                    obj::obj_type_ptr(selector.as_ptr()),
                    &ORIGINAL
                ));
                assert!(obj::native_method_name::chain(selector.as_ptr()).is_none());
            }
            assert_eq!(unsafe { (*later.as_ptr()).ref_count }, 1);
            assert_eq!(interpreter.entered_native_operation_count(), 0);
        }
    }

    #[test]
    fn slot_selected_list_preserves_parse_refusal_and_stops_before_cache_after_callback() {
        for mode in [0, 1, 2] {
            let mut interpreter = native("tcl8.6");
            let (target, caller) = definition(&mut interpreter);
            let slot = interpreter.oo_resolve_object(b"::oo::define::filter");
            interpreter
                .oo
                .borrow_mut()
                .classes
                .get_mut(&target)
                .unwrap()
                .filters = vec![b"SENTINEL".to_vec()];
            let value = if mode == 0 {
                word(b"{")
            } else {
                original(b"first second", mode)
            };
            interpreter.set_result_bytes(b"PRIOR");
            let prior = interpreter.get_obj_result();
            assert_eq!(
                slot_inst_set(&mut interpreter, slot, &[value.as_ptr()]),
                Code::Error
            );
            assert_eq!(
                interpreter.oo.borrow().classes[&target].filters,
                [b"SENTINEL".to_vec()]
            );
            if mode == 0 {
                assert!(interpreter.native_execution_refusal().is_none());
                assert!(interpreter
                    .result_bytes()
                    .windows(b"unmatched".len())
                    .any(|part| part == b"unmatched"));
            } else {
                assert_eq!(UPDATES.with(Cell::get), 1);
                assert!(interpreter.native_execution_refusal().is_some());
                assert_eq!(interpreter.get_obj_result(), prior);
                assert!(core::ptr::eq(obj::obj_type_ptr(value.as_ptr()), &ORIGINAL));
                assert!(obj::has_string_rep(value.as_ptr()));
            }
            assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
            interpreter.oo_leave_definition(caller);
            assert_eq!(interpreter.entered_native_operation_count(), 0);
        }
        let mut interpreter = native("tcl8.6");
        let (target, caller) = definition(&mut interpreter);
        let slot = interpreter.oo_resolve_object(b"::oo::define::filter");
        let first = word(b"a\0b");
        let value = obj::Owned::fresh(interpreter.new_list_object(&[first.as_ptr()]));
        assert_eq!(
            slot_inst_set(&mut interpreter, slot, &[value.as_ptr()]),
            Code::Ok
        );
        assert_eq!(
            interpreter.oo.borrow().classes[&target].filters,
            [b"a\0b".to_vec()]
        );
        assert!(!obj::has_string_rep(value.as_ptr()));
        assert_eq!(unsafe { (*first.as_ptr()).ref_count }, 2);
        interpreter.oo_leave_definition(caller);
    }

    #[test]
    fn property_script_refusal_stops_later_name_and_accessor_publication() {
        for mode in [1, 2] {
            let mut interpreter = native("tcl9.1");
            let (target, caller) = definition(&mut interpreter);
            let head = word(b"property");
            let name = word(b"p");
            let option = word(b"-get");
            let body = original(b"return REACHED", mode);
            let later = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
            interpreter.set_result_bytes(b"PRIOR");
            let prior = interpreter.get_obj_result();
            assert_eq!(
                prop_define(
                    &mut interpreter,
                    &[
                        head.as_ptr(),
                        name.as_ptr(),
                        option.as_ptr(),
                        body.as_ptr(),
                        later.as_ptr()
                    ],
                    false
                ),
                Code::Error
            );
            assert_eq!(UPDATES.with(Cell::get), 1);
            assert!(!obj::has_string_rep(later.as_ptr()));
            assert!(!interpreter.oo.borrow().classes[&target]
                .methods
                .contains_key(b"<ReadProp-p>".as_slice()));
            assert_eq!(interpreter.get_obj_result(), prior);
            assert!(interpreter.native_execution_refusal().is_some());
            assert_eq!(unsafe { (*body.as_ptr()).ref_count }, 1);
            interpreter.oo_leave_definition(caller);
            assert_eq!(interpreter.entered_native_operation_count(), 0);
        }
    }

    #[test]
    fn deleted_definition_target_cannot_supply_constructor_continuation() {
        // A real nested destroy changes target lifecycle while operation context remains current.
        let mut interpreter = native("tcl8.6");
        let (target, caller) = definition(&mut interpreter);
        let head = word(b"constructor");
        let formals = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
        let body = original(b"return REACHED", 3);
        assert_eq!(
            def_constructor(
                &mut interpreter,
                &[head.as_ptr(), formals.as_ptr(), body.as_ptr()]
            ),
            Code::Error
        );
        assert_eq!(UPDATES.with(Cell::get), 1);
        assert!(!obj::has_string_rep(formals.as_ptr()));
        assert!(!interpreter.oo.borrow().objects.contains_key(&target));
        assert!(!interpreter.oo.borrow().classes.contains_key(&target));
        assert!(interpreter.native_execution_refusal().is_none());
        assert_eq!(interpreter.error_code(), b"TCL OO MONKEY_BUSINESS");
        assert_eq!(unsafe { (*formals.as_ptr()).ref_count }, 1);
        interpreter.oo_leave_definition(caller);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

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
