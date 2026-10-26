// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Execute the original retained Jim Script token objects.

use super::{CmdFrame, Code, Interp};
use crate::obj::Owned;
use tcl_syntax::{
    jim_script_objects::{missing_message, JimScriptObjectKind, JimScriptObjects, JimScriptWord},
    native_string::NativeStringProtocol,
    value::ValueError,
};

/// Source operands from one actual Script command and its entered procedure.
/// This owns allocation safety and constructor/frame evidence only.
pub(super) struct OriginalSourceInvocation {
    frame: super::native_arguments::OriginalArgumentFrame,
    procedure: std::rc::Weak<super::ProcDef>,
    purpose: tcl_registry::native_jim_lookup::NativeJimNamespaceHelperKind,
    pub(super) root: std::rc::Rc<crate::native_source::NativeJimSourceOrigin>,
    argv: Vec<crate::obj::NativeObjectLifetime>,
    pub(super) operands: Vec<Option<OriginalSourceOperand>>,
    relay: Option<std::rc::Rc<OriginalSourceInvocation>>,
    inventory_return: std::cell::RefCell<
        Option<std::rc::Rc<super::native_jim_lookup::OriginalJimCommandInventoryReturn>>,
    >,
}

pub(super) struct OriginalSourceOperand {
    pub(super) source: std::rc::Rc<crate::native_source::NativeJimSourceOrigin>,
    pub(super) returned: bool,
    inventory: Option<std::rc::Rc<super::native_jim_lookup::OriginalJimCommandInventoryReturn>>,
}

#[derive(Clone)]
pub(crate) struct OriginalSourceConstruction {
    invocation: std::rc::Rc<OriginalSourceInvocation>,
}

impl OriginalSourceConstruction {
    pub(crate) fn check(&self, actual: &Interp) -> Result<(), ValueError> {
        self.invocation.check_retained(actual)
    }

    pub(super) fn invocation(&self) -> &std::rc::Rc<OriginalSourceInvocation> {
        &self.invocation
    }
}

/// The exact object returned by one reached Command token. This does not
/// establish command-table membership or permit an Import traversal.
pub(crate) struct OriginalSourceCommandReturn {
    invocation: std::rc::Rc<OriginalSourceInvocation>,
    index: usize,
}

impl OriginalSourceCommandReturn {
    pub(crate) fn check(
        &self,
        actual: &Interp,
        original: *mut crate::obj::TclObj,
    ) -> Result<(), ValueError> {
        let argv = self
            .invocation
            .argv
            .iter()
            .map(crate::obj::NativeObjectLifetime::checked_ptr)
            .collect::<Result<Vec<_>, _>>()?;
        self.invocation.check(actual, &argv)?;
        if argv.get(self.index) != Some(&original) {
            return Err(ValueError::CommandProtocolUnavailable(
                "original source Command return header",
            ));
        }
        Ok(())
    }

    pub(crate) fn check_inventory(
        &self,
        actual: &Interp,
        original: *mut crate::obj::TclObj,
    ) -> Result<(), ValueError> {
        self.check(actual, original)?;
        let inventory = self.invocation.operands[self.index]
            .as_ref()
            .and_then(|operand| operand.inventory.as_ref())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original Import inventory return producer",
            ))?;
        inventory.check(actual, original)
    }
}

impl OriginalSourceInvocation {
    pub(super) fn check_suffix(
        &self,
        actual: &Interp,
        inputs: &[*mut crate::obj::TclObj],
    ) -> Result<(), ValueError> {
        self.check_retained(actual)?;
        let start = self.argv.len().checked_sub(inputs.len()).ok_or(
            ValueError::CommandProtocolUnavailable("original Source construction operand extent"),
        )?;
        for (held, &input) in self.argv[start..].iter().zip(inputs) {
            if held.checked_ptr()? != input {
                return Err(ValueError::CommandProtocolUnavailable(
                    "original Source construction original operands",
                ));
            }
        }
        Ok(())
    }

    pub(super) fn receive_namespace_info_inventory(
        &self,
        actual: &Interp,
        inventory: std::rc::Rc<super::native_jim_lookup::OriginalJimCommandInventoryReturn>,
    ) -> Result<(), ValueError> {
        if self.relay.is_none() {
            return Err(ValueError::CommandProtocolUnavailable(
                "original NamespaceInfo completion receiver",
            ));
        }
        self.check_retained(actual)?;
        let invocations = actual.jim_invocation_borrows.borrow();
        if !invocations
            .last()
            .and_then(|invocation| invocation.source.as_ref())
            .is_some_and(|source| std::ptr::eq(source.as_ref(), self))
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "original NamespaceInfo selected completion receiver",
            ));
        }
        inventory.check(actual, actual.result_obj())?;
        *self.inventory_return.borrow_mut() = Some(inventory);
        Ok(())
    }
    pub(super) fn namespace_info_relay(
        actual: &Interp,
        original: &[*mut crate::obj::TclObj],
        source: &std::rc::Rc<Self>,
    ) -> Result<std::rc::Rc<Self>, ValueError> {
        source.check_retained(actual)?;
        if source.relay.is_some() {
            return Err(ValueError::CommandProtocolUnavailable(
                "original NamespaceInfo Source relay issuer",
            ));
        }
        let relay = std::rc::Rc::new(Self {
            frame: super::native_arguments::OriginalArgumentFrame::capture(actual),
            procedure: source.procedure.clone(),
            purpose: source.purpose,
            root: std::rc::Rc::clone(&source.root),
            argv: original
                .iter()
                .map(|&original| crate::obj::NativeObjectLifetime::retain(original))
                .collect(),
            operands: (0..original.len()).map(|_| None).collect(),
            relay: Some(std::rc::Rc::clone(source)),
            inventory_return: std::cell::RefCell::new(None),
        });
        relay.check(actual, original)?;
        Ok(relay)
    }

    pub(super) fn forward_namespace_info_inventory(
        &self,
        actual: &Interp,
        original: *mut crate::obj::TclObj,
    ) -> Result<(), ValueError> {
        self.check_retained(actual)?;
        let caller = self
            .relay
            .as_ref()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original NamespaceInfo inventory relay",
            ))?;
        let inventory = self.inventory_return.borrow_mut().take().ok_or(
            ValueError::CommandProtocolUnavailable(
                "original NamespaceInfo inventory completion producer",
            ),
        )?;
        inventory.check(actual, original)?;
        *caller.inventory_return.borrow_mut() = Some(inventory);
        Ok(())
    }

    pub(super) fn check_retained(&self, actual: &Interp) -> Result<(), ValueError> {
        let argv = self
            .argv
            .iter()
            .map(crate::obj::NativeObjectLifetime::checked_ptr)
            .collect::<Result<Vec<_>, _>>()?;
        self.check(actual, &argv)
    }

    fn capture(
        actual: &Interp,
        backing: &crate::native_script::NativeJimScript,
        argv: &[*mut crate::obj::TclObj],
        operands: Vec<Option<OriginalSourceOperand>>,
    ) -> Result<Option<std::rc::Rc<Self>>, ValueError> {
        let Some(root) = backing.source_origin() else {
            return Ok(None);
        };
        let procedure = actual.frames.borrow().current_original_jim_procedure()?;
        let Some(procedure) = procedure else {
            return Ok(None);
        };
        let Some((purpose, installed)) = procedure.original_jim_namespace_helper_origin(actual)?
        else {
            return Ok(None);
        };
        if !installed.same_root(root)
            || operands
                .iter()
                .flatten()
                .any(|operand| !operand.source.same_root(root))
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "original Import mixed source invocation",
            ));
        }
        Ok(Some(std::rc::Rc::new(Self {
            frame: super::native_arguments::OriginalArgumentFrame::capture(actual),
            procedure: std::rc::Rc::downgrade(&procedure),
            purpose,
            root: std::rc::Rc::clone(root),
            argv: argv
                .iter()
                .map(|&original| crate::obj::NativeObjectLifetime::retain(original))
                .collect(),
            operands,
            relay: None,
            inventory_return: std::cell::RefCell::new(None),
        })))
    }

    pub(super) fn check(
        &self,
        actual: &Interp,
        argv: &[*mut crate::obj::TclObj],
    ) -> Result<(), ValueError> {
        self.frame.check_frame(actual)?;
        if let Some(caller) = &self.relay {
            caller.check_retained(actual)?;
        }
        let procedure = actual.frames.borrow().current_original_jim_procedure()?;
        let Some((owner, current)) = self.procedure.upgrade().zip(procedure) else {
            return Err(ValueError::CommandProtocolUnavailable(
                "original Import source invocation frame",
            ));
        };
        if !std::rc::Rc::ptr_eq(&owner, &current)
            || !owner
                .original_jim_namespace_helper_origin(actual)?
                .is_some_and(|(purpose, root)| {
                    purpose == self.purpose && root.same_root(&self.root)
                })
            || argv.len() != self.argv.len()
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "original Import source invocation frame",
            ));
        }
        for (operand, &original) in self.argv.iter().zip(argv) {
            if operand.checked_ptr()? != original {
                return Err(ValueError::CommandProtocolUnavailable(
                    "original Import source operand header",
                ));
            }
        }
        Ok(())
    }
}

impl Interp {
    pub(super) fn original_namespace_info_receiver(
        &self,
        argv: Option<&[*mut crate::obj::TclObj]>,
    ) -> Result<Option<std::rc::Rc<OriginalSourceInvocation>>, ValueError> {
        let invocations = self.jim_invocation_borrows.borrow();
        let Some(source) = invocations
            .last()
            .and_then(|invocation| invocation.source.as_ref())
            .filter(|source| source.relay.is_some())
        else {
            return Ok(None);
        };
        let argv = argv.ok_or(ValueError::CommandProtocolUnavailable(
            "original NamespaceInfo entered argv",
        ))?;
        source.check(self, argv)?;
        Ok(Some(std::rc::Rc::clone(source)))
    }
    pub(crate) fn original_source_construction(
        &self,
    ) -> Result<Option<OriginalSourceConstruction>, ValueError> {
        Ok(self
            .current_original_source_invocation()?
            .map(|invocation| OriginalSourceConstruction { invocation }))
    }
    pub(super) fn current_original_source_invocation(
        &self,
    ) -> Result<Option<std::rc::Rc<OriginalSourceInvocation>>, ValueError> {
        let invocations = self.jim_invocation_borrows.borrow();
        let Some(invocation) = invocations.last() else {
            return Ok(None);
        };
        let Some(source) = invocation.source.as_ref() else {
            return Ok(None);
        };
        if source.relay.is_some() {
            return Ok(None);
        }
        source.check(self, &invocation.argv)?;
        Ok(Some(std::rc::Rc::clone(source)))
    }

    pub(super) fn record_original_source_inventory_return(
        &self,
        original: *mut crate::obj::TclObj,
        keys: Vec<crate::namespace::OriginalJimCommandInventoryKey>,
    ) -> Result<(), ValueError> {
        let source = {
            let borrows = self.jim_invocation_borrows.borrow();
            let Some(invocation) = borrows.last() else {
                return Ok(());
            };
            let Some(source) = invocation.source.as_ref() else {
                return Ok(());
            };
            if source.relay.is_some() {
                return Ok(());
            }
            source.check(self, &invocation.argv)?;
            std::rc::Rc::clone(source)
        };
        let inventory = super::native_jim_lookup::OriginalJimCommandInventoryReturn::capture(
            self, original, keys,
        )?;
        let inventory = std::rc::Rc::new(inventory);
        if let Some(owner) = self
            .frames
            .borrow()
            .current_original_namespace_info_frame()?
        {
            owner.record_data(self, std::rc::Rc::clone(&inventory))?;
        }
        *source.inventory_return.borrow_mut() = Some(inventory);
        Ok(())
    }

    pub(super) fn record_original_source_inventory_data(
        &self,
        inventory: std::rc::Rc<super::native_jim_lookup::OriginalJimCommandInventoryReturn>,
    ) -> Result<(), ValueError> {
        let source = self.current_original_source_invocation()?.ok_or(
            ValueError::CommandProtocolUnavailable(
                "original inventory projection Source completion",
            ),
        )?;
        inventory.check(self, inventory.original()?)?;
        *source.inventory_return.borrow_mut() = Some(inventory);
        Ok(())
    }

    pub(crate) fn original_source_command_return(
        &self,
        index: usize,
        original: *mut crate::obj::TclObj,
    ) -> Result<Option<OriginalSourceCommandReturn>, ValueError> {
        let borrows = self.jim_invocation_borrows.borrow();
        let Some(invocation) = borrows.last() else {
            return Ok(None);
        };
        let Some(source) = invocation.source.as_ref() else {
            return Ok(None);
        };
        if source.relay.is_some() {
            return Ok(None);
        }
        source.check(self, &invocation.argv)?;
        if invocation.argv.get(index) != Some(&original) {
            return Err(ValueError::CommandProtocolUnavailable(
                "original source invocation operand index",
            ));
        }
        Ok(source
            .operands
            .get(index)
            .and_then(Option::as_ref)
            .filter(|operand| operand.returned)
            .map(|_| OriginalSourceCommandReturn {
                invocation: std::rc::Rc::clone(source),
                index,
            }))
    }

    /// Enter the fresh Source object selected by the sourced-byte API. Its
    /// original filename and line become the Script token owners' source info.
    pub(super) fn eval_native_jim_source_unpublished(
        &mut self,
        source: &[u8],
        filename: &[u8],
        frame: CmdFrame,
    ) -> Code {
        let context = match self.native_jim_object_context() {
            Ok(context) => context,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        let filename = Owned::fresh(crate::obj::new_string_bytes(filename));
        let original = Owned::fresh(crate::obj::new_string_bytes(source));
        if let Err(error) = crate::native_source::install_source(
            original.as_ptr(),
            crate::native_source::NativeJimSourceInfo { filename, line: 1 },
            &context,
        ) {
            return self.report_cmd_error(error.into());
        }
        if !self.codegen_activation_enter() {
            return Code::Error;
        }
        let code = self.eval_native_jim_script(original, frame);
        // The sourced API settles return and projects options before publishing
        // the outermost completion or processing background errors.
        self.codegen_activation_leave_unpublished();
        code
    }

    pub(super) fn eval_native_jim_script(&mut self, original: Owned, frame: CmdFrame) -> Code {
        self.eval_native_jim_script_with_inventory(original, frame, &mut None)
    }

    pub(super) fn eval_native_jim_script_with_inventory(
        &mut self,
        original: Owned,
        frame: CmdFrame,
        inventory: &mut Option<
            std::rc::Rc<super::native_jim_lookup::OriginalJimCommandInventoryReturn>,
        >,
    ) -> Code {
        *inventory = None;
        let scope = match super::native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => return self.refuse_native_execution(first),
        };
        let reached = (|| {
            let context = match self.native_jim_object_context() {
                Ok(context) => context,
                Err(error) => return self.report_cmd_error(error.into()),
            };
            let prepared =
                crate::native_script::prepare(original.as_ptr(), &context, self.lexer_config());
            if let Err(first) = scope.currency().ensure_current_or_refuse() {
                return self.refuse_native_execution(first);
            }
            let backing = match prepared {
                Ok(backing) => backing,
                Err(error) => return self.report_cmd_error(error.into()),
            };
            let script = match backing.ordinary() {
                Ok(script) => script,
                Err(error) => return self.report_cmd_error(error.into()),
            };
            if let Some(message) = missing_message(script.objects.missing) {
                let code = self.error(message);
                if let Err(first) = self.capture_original_jim_parse_failure(script) {
                    return self.refuse_native_execution(first);
                }
                return code;
            }
            self.set_result(context.empty_object().as_ptr());
            if script.objects.tokens().is_empty() {
                return Code::Ok;
            }
            let _lease = crate::native_script::activate(original, &backing);
            self.cmd_frames.borrow_mut().push(frame);
            self.reset_jim_error_capture();
            let mut cursor = 0;
            let code = loop {
                let command = match script.objects.command_at(cursor) {
                    Ok(Some(command)) => command,
                    Ok(None) => break Code::Ok,
                    Err(error) => break self.report_cmd_error(error.into()),
                };
                let line_index = cursor;
                cursor = command.next;
                let Some((argc, line)) = crate::native_script::script_line(
                    script.objects.tokens()[line_index].value.as_ptr(),
                ) else {
                    break self.report_cmd_error(
                        ValueError::CommandProtocolUnavailable("Jim Script original LINE primary")
                            .into(),
                    );
                };
                if usize::try_from(argc).ok() != Some(command.words.len())
                    || line
                        != script
                            .baseline
                            .wrapping_add(i32::from_ne_bytes(command.line_delta.to_ne_bytes()))
                {
                    break self.report_cmd_error(
                        ValueError::CommandProtocolUnavailable("Jim Script original LINE payload")
                            .into(),
                    );
                }
                script.linenr.set(line);
                let _evaluation = self.enter_original_jim_evaluation(
                    u32::from_ne_bytes(line.to_ne_bytes()),
                    script.filename.as_ptr(),
                );
                if let Some(frame) = self.cmd_frames.borrow_mut().last_mut() {
                    frame.line = u32::from_ne_bytes(line.to_ne_bytes());
                }
                let code =
                    self.eval_native_jim_words(&backing, &script.objects, command.words, inventory);
                if code == Code::Error {
                    // Capture while the actual Script frame and procedure level are
                    // live. Failed lookup has no selected invocation argv.
                    if let Err(first) = self.capture_jim_error_stack() {
                        break self.refuse_native_execution(first);
                    }
                }
                if code != Code::Ok {
                    break code;
                }
            };
            self.cmd_frames.borrow_mut().pop();
            code
        })();
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            self.refuse_native_execution(first)
        } else {
            reached
        }
    }

    fn capture_original_jim_parse_failure(
        &self,
        script: &tcl_syntax::native_jim_substitution::JimOrdinaryScript<Owned>,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        let scope = super::native_operation_currency::NativeOperationScope::enter(self)?;
        let location = tcl_runtime_api::jim_error_stack::JimScriptLocation {
            file: script.filename.clone(),
            line: u32::from_ne_bytes(script.linenr.get().to_ne_bytes()),
        };
        let reached = self.capture_jim_error_stack_at(Some(&location));
        drop(location);
        scope.currency().ensure_current_or_refuse()?;
        reached
    }

    fn eval_native_jim_words(
        &mut self,
        backing: &std::rc::Rc<crate::native_script::NativeJimScript>,
        objects: &JimScriptObjects<Owned>,
        words: Vec<JimScriptWord>,
        inventory: &mut Option<
            std::rc::Rc<super::native_jim_lookup::OriginalJimCommandInventoryReturn>,
        >,
    ) -> Code {
        *inventory = None;
        let mut argv = Vec::<Owned>::with_capacity(words.len());
        let mut origins = Vec::with_capacity(words.len());
        for word in words {
            let mut returned_inventory = None;
            let source = (word.tokens.len() == 1 && !word.expand)
                .then(|| backing.token_origin(word.tokens.start).cloned())
                .flatten();
            let mut parts = Vec::with_capacity(word.tokens.len());
            for token in &objects.tokens()[word.tokens.clone()] {
                let value = match self.eval_native_jim_token_with_inventory(
                    token.kind,
                    &token.value,
                    &mut returned_inventory,
                ) {
                    Ok(value) => value,
                    Err(Code::Return) if word.tokens.len() > 1 => Owned::retain(self.result_obj()),
                    Err(code @ (Code::Break | Code::Continue)) if word.tokens.len() > 1 => {
                        let action = if code == Code::Break {
                            "break"
                        } else {
                            "continue"
                        };
                        return self
                            .error(format!("invoked \"{action}\" outside of a loop").as_bytes());
                    }
                    Err(code) => return code,
                };
                if word.tokens.len() > 1 {
                    if let Err(error) = crate::dict::native_object_bytes(
                        value.as_ptr(),
                        NativeStringProtocol::Jim084,
                    ) {
                        return self.report_cmd_error(error.into());
                    }
                }
                parts.push(value);
            }
            let value = if parts.len() == 1 {
                parts.pop().expect("single original token")
            } else {
                match self.interpolate_native_jim_values(backing, word.tokens.clone(), parts) {
                    Ok(value) => value,
                    Err(code) => return code,
                }
            };
            if word.expand {
                match crate::list::list_elements_native_checked(
                    value.as_ptr(),
                    NativeStringProtocol::Jim084,
                ) {
                    Ok(items) => {
                        origins.extend((0..items.len()).map(|_| None));
                        argv.extend(items.into_iter().map(Owned::retain));
                    }
                    Err(error) => return self.report_cmd_error(error.into()),
                }
            } else {
                origins.push(source.map(|source| OriginalSourceOperand {
                    returned: source.kind()
                        == Some(JimScriptObjectKind::Source(
                            tcl_lexer::JimScriptTokenKind::Command,
                        )),
                    source,
                    inventory: returned_inventory,
                }));
                argv.push(value);
            }
        }
        if argv.is_empty() {
            Code::Ok
        } else {
            let pointers: Vec<_> = argv.iter().map(Owned::as_ptr).collect();
            let origin = match OriginalSourceInvocation::capture(self, backing, &pointers, origins)
            {
                Ok(origin) => origin,
                Err(error) => return self.report_cmd_error(error.into()),
            };
            let code = self.dispatch_original_source(&pointers, origin.as_ref());
            if code == Code::Ok {
                if let Some(receipt) = origin
                    .as_ref()
                    .and_then(|source| source.inventory_return.borrow_mut().take())
                {
                    if let Err(error) = receipt.check(self, self.result_obj()) {
                        return self.report_cmd_error(error.into());
                    }
                    *inventory = Some(receipt);
                }
            }
            code
        }
    }

    pub(super) fn eval_native_jim_token(
        &mut self,
        kind: JimScriptObjectKind,
        original: &Owned,
    ) -> Result<Owned, Code> {
        self.eval_native_jim_token_with_inventory(kind, original, &mut None)
    }

    fn eval_native_jim_token_with_inventory(
        &mut self,
        kind: JimScriptObjectKind,
        original: &Owned,
        inventory: &mut Option<
            std::rc::Rc<super::native_jim_lookup::OriginalJimCommandInventoryReturn>,
        >,
    ) -> Result<Owned, Code> {
        *inventory = None;
        match kind {
            JimScriptObjectKind::Source(
                tcl_lexer::JimScriptTokenKind::String | tcl_lexer::JimScriptTokenKind::Escaped,
            ) => Ok(original.clone()),
            JimScriptObjectKind::Source(tcl_lexer::JimScriptTokenKind::Variable) => self
                .read_original_named_variable(original.as_ptr())
                .map(Owned::retain),
            JimScriptObjectKind::Source(tcl_lexer::JimScriptTokenKind::Command) => {
                let code = self.eval_body_obj_with_inventory(original.as_ptr(), inventory);
                if code == Code::Ok {
                    Ok(Owned::retain(self.result_obj()))
                } else {
                    Err(code)
                }
            }
            JimScriptObjectKind::Source(tcl_lexer::JimScriptTokenKind::Expression) => {
                #[cfg(have_tommath)]
                let code = crate::builtins::eval_expr_original(self, original.as_ptr());
                #[cfg(not(have_tommath))]
                let code = self.refuse_native_access(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::ExpressionEngineUnavailable,
                );
                if code == Code::Ok {
                    Ok(Owned::retain(self.result_obj()))
                } else {
                    Err(code)
                }
            }
            JimScriptObjectKind::Source(tcl_lexer::JimScriptTokenKind::IndexedVariable) => self
                .expand_native_jim_dictionary_substitution(original.as_ptr())
                .map(Owned::retain),
            _ => Err(self.report_cmd_error(
                ValueError::CommandProtocolUnavailable("native Script real token purpose").into(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{interp::default_host, native_source};

    #[test]
    fn sourced_jim_body_tokens_keep_original_lines_and_unlocated_objects_stay_unlocated() {
        let mut interp = Interp::with_native_core(
            default_host(),
            crate::environment::profile_for_dialect("jim"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .expect("selected original Jim constructor");
        assert_eq!(
            interp.eval_sourced(
                b"# first\n# second\n\nproc retained {} {error BOOM}",
                b"source.jim"
            ),
            Code::Ok,
        );
        let context = interp.native_jim_object_context().unwrap();
        let procedure = interp.proc_def(b"retained").unwrap();
        let original = procedure.body.checked_ptr().unwrap();
        let info = native_source::pin_source_info(original, &context).unwrap();
        assert_eq!(info.line, 4);
        assert_eq!(crate::obj::bytes_of(info.filename.as_ptr()), b"source.jim");
        drop(info);
        drop(procedure);

        let unlocated = Owned::fresh(crate::obj::new_string_bytes(b"error BOOM"));
        let head = Owned::fresh(crate::obj::new_string_bytes(b"proc"));
        let name = Owned::fresh(crate::obj::new_string_bytes(b"unlocated"));
        let parameters = Owned::fresh(crate::obj::new_string_bytes(b""));
        assert_eq!(
            interp.dispatch(&[
                head.as_ptr(),
                name.as_ptr(),
                parameters.as_ptr(),
                unlocated.as_ptr()
            ]),
            Code::Ok
        );
        let procedure = interp.proc_def(b"unlocated").unwrap();
        let info = native_source::pin_source_info(procedure.body.checked_ptr().unwrap(), &context)
            .unwrap();
        assert_eq!(info.line, 1);
        assert_eq!(info.filename.as_ptr(), context.empty_object().as_ptr());
    }

    #[test]
    fn jim_control_body_preserves_original_source_in_error_frames() {
        let mut interp = Interp::with_native_core(
            default_host(),
            crate::environment::profile_for_dialect("jim"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .expect("selected original Jim constructor");
        let context = interp.native_jim_object_context().unwrap();
        let filename = Owned::fresh(crate::obj::new_string_bytes(b"original.jim"));
        let body = Owned::fresh(crate::obj::new_string_bytes(b"error BOOM"));
        native_source::install_source(
            body.as_ptr(),
            native_source::NativeJimSourceInfo {
                filename: filename.clone(),
                line: 7,
            },
            &context,
        )
        .unwrap();
        assert_eq!(interp.eval_control_body(body.as_ptr()), Code::Error);
        assert!(!interp.host_refusal_pending());
        let trace = interp.jim_stacktrace_object().unwrap();
        let fields = crate::list::list_elements_native_checked(
            trace.as_ptr(),
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
        )
        .unwrap();
        assert_eq!(fields.len(), 4);
        assert_eq!(fields[1], filename.as_ptr());
        assert_eq!(crate::obj::bytes_of(fields[2]), b"7");
        assert!(interp.jim_invocation_borrows.borrow().is_empty());
        assert!(interp.jim_evaluation_frames.borrow().is_empty());
    }

    #[test]
    fn jim_original_lookup_error_keeps_live_procedure_and_script_frames() {
        let mut interp = Interp::with_native_core(
            default_host(),
            crate::environment::profile_for_dialect("jim"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .expect("selected original Jim constructor");
        assert_eq!(
            interp.eval_sourced(
                b"proc fail {} {missingCommand BOOM}\ncatch {fail} r o; dict get $o -errorinfo",
                b"stdin",
            ),
            Code::Ok,
        );
        assert_eq!(interp.result_bytes(), b"fail stdin 1 {} {} stdin 2 fail");
        assert!(interp.jim_invocation_borrows.borrow().is_empty());
        assert!(interp.jim_evaluation_frames.borrow().is_empty());
        assert_eq!(interp.jim_procedure_level.get(), 0);
    }

    #[test]
    fn normal_original_script_does_not_materialise_its_borrowed_filename() {
        let mut interp = Interp::with_native_core(
            default_host(),
            crate::environment::profile_for_dialect("jim"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .expect("selected original Jim constructor");
        let context = interp.native_jim_object_context().unwrap();
        let filename = Owned::fresh(crate::obj::new_wide_int_obj(37));
        let original = Owned::fresh(crate::obj::new_string_bytes(b"list OK"));
        native_source::install_source(
            original.as_ptr(),
            native_source::NativeJimSourceInfo {
                filename: filename.clone(),
                line: 7,
            },
            &context,
        )
        .unwrap();
        assert!(!crate::obj::has_string_rep(filename.as_ptr()));
        assert_eq!(interp.eval_body_obj(original.as_ptr()), Code::Ok);
        assert!(!crate::obj::has_string_rep(filename.as_ptr()));
        assert!(interp.jim_invocation_borrows.borrow().is_empty());
        assert!(interp.jim_evaluation_frames.borrow().is_empty());
    }
}
