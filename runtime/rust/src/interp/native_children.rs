// SPDX-License-Identifier: AGPL-3.0-or-later
//! Child command identity remains independent of its parent table spelling.

use super::{Cell, Code, Interp, InterpState, Rc};
use crate::obj::{self, TclObj};
use std::rc::Weak;

/// A retained child-command allocation; it keeps no interpreter lifetime lease.
#[derive(Clone)]
pub struct ChildInterpreterCommand {
    name: Vec<u8>,
    interpreter: Weak<InterpState>,
    deleted: Rc<Cell<bool>>,
}

impl ChildInterpreterCommand {
    pub(super) fn new(name: Vec<u8>, interpreter: Interp) -> Self {
        Self {
            name,
            interpreter: Rc::downgrade(&interpreter.0),
            deleted: Rc::new(Cell::new(false)),
        }
    }

    pub(crate) fn same_allocation(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.deleted, &other.deleted)
    }

    pub(crate) fn retire(&self) {
        self.deleted.set(true);
        if let Some(state) = self.interpreter.upgrade() {
            let interpreter = Interp(state);
            interpreter.pending_delete.set(true);
            interpreter.invalidate_interpreter_policy();
        }
    }
}

impl Interp {
    /// A retained activation keeps storage alive, without permitting further eval.
    pub(super) fn evaluation_is_live(&mut self) -> bool {
        if !self.pending_delete.get() {
            return true;
        }
        if let Some((message, code)) = self
            .native_invocation_dialect()
            .native_eval_object_protocol()
            .and_then(tcl_registry::native_eval_object::NativeEvalObjectProtocol::deleted_interpreter_error)
        {
            self.error_with_code(message, code);
        } else {
            self.error(b"attempt to call eval in deleted interpreter");
        }
        false
    }

    pub(super) fn dispatch_original_child(
        &mut self,
        command: &ChildInterpreterCommand,
        argv: &[*mut TclObj],
    ) -> Code {
        let current = self
            .children
            .borrow()
            .get(&command.name)
            .is_some_and(|child| Weak::ptr_eq(&Rc::downgrade(&child.0), &command.interpreter));
        if command.deleted.get() || !current {
            return self.error(b"interpreter has been deleted");
        }
        self.dispatch_child(&command.name, argv)
    }

    pub(super) fn retire_pending_children(&mut self) {
        let changed = {
            let mut children = self.children.borrow_mut();
            let before = children.len();
            children.retain(|_, child| !child.pending_delete.get());
            children.len() != before
        };
        if changed {
            self.invalidate_interpreter_policy();
        }
    }

    fn transfer_child_completion(&mut self, result: obj::Owned, options: obj::Owned) -> Code {
        use tcl_cmd_core::return_options::{self, ReturnOptionsPurpose};
        let (mut ops, protocol) = match crate::return_options::NativeReturnOps::selected(self) {
            Ok(selected) => selected,
            Err(error) => return self.report_cmd_error(error),
        };
        let purpose = ReturnOptionsPurpose::InternalDictionary;
        let prepared = return_options::prepare_option_pairs(&mut ops, protocol, &options, purpose)
            .and_then(|pairs| {
                return_options::prepare_return_pairs(
                    &mut ops,
                    protocol,
                    pairs,
                    Some(result),
                    purpose,
                )
            });
        match prepared {
            Ok(prepared) => crate::return_options::publish(self, &mut ops, prepared),
            Err(error) => self.report_cmd_error(error),
        }
    }

    pub(super) fn child_eval_original(&mut self, name: &[u8], arguments: &[*mut TclObj]) -> Code {
        if self.native_invocation_dialect().native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            return self.eval_original_jim_child(name, arguments);
        }
        let script = if arguments.len() == 1 {
            obj::Owned::retain(arguments[0])
        } else {
            match tcl_cmd_core::list::concat_selected(self, arguments) {
                Ok(script) => obj::Owned::retain(script),
                Err(error) => return self.report_cmd_error(error),
            }
        };
        match self.with_child(name, |child| {
            let code = child.eval_body_obj(script.as_ptr());
            if child.host_refusal_pending() {
                return Err(child.clone());
            }
            let completion = crate::state_traits::capture_completion(child, code);
            // SAFETY: capture_completion transfers one owning reference to each handle.
            Ok(unsafe {
                (
                    obj::Owned::from_raw(completion.result),
                    obj::Owned::from_raw(completion.options),
                )
            })
        }) {
            Some(Ok((result, options))) => self.transfer_child_completion(result, options),
            Some(Err(child)) => self.transport_host_refusal_from(&child),
            None => self.error(b"could not find interpreter"),
        }
    }

    /// JimInterpCopyObj crosses a counted String, never a Script/Source/List
    /// header or its interpreter-owned children. Foreign direct binds remain refusals.
    fn copy_original_jim_object_to(
        &self,
        destination: &Interp,
        original: *mut TclObj,
    ) -> Result<obj::Owned, tcl_syntax::value::ValueError> {
        self.copy_original_jim_object_with_purpose_to(
            destination,
            original,
            tcl_registry::special_vars::NativeJimInterpreterCopyPurpose::Object,
        )
    }

    fn copy_original_jim_object_with_purpose_to(
        &self,
        destination: &Interp,
        original: *mut TclObj,
        purpose: tcl_registry::special_vars::NativeJimInterpreterCopyPurpose,
    ) -> Result<obj::Owned, tcl_syntax::value::ValueError> {
        let producer = self.native_jim_object_context()?;
        let receiver = destination.native_jim_object_context()?;
        crate::native_source::bind_context(original, &producer)?;
        let bytes = self.native_object_string_bytes(original)?;
        let copied = obj::Owned::fresh(obj::new_string_bytes(purpose.input(&bytes)));
        crate::native_source::bind_context(copied.as_ptr(), &receiver)?;
        Ok(copied)
    }

    /// Install only implemented members of the selected pinned Jim static
    /// distribution, then copy the exact original global startup operands.
    pub(super) fn initialise_original_jim_child(
        &self,
        child: &mut Interp,
        initialisation: tcl_registry::special_vars::NativeChildInitialisation,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        // naming.interpreter.original-child-bootstrap-context
        // docs/design/analysis/name-resolution-proofs/interpreter-original-child-bootstrap-context.md
        if !initialisation.initialises_jim_static_extensions() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim original child distribution initialisation",
            ));
        }
        // Namespace, package, interp and array installers are already included
        // in this backend's supported core inventory. Binary is distribution-owned
        // in Jim and is installed here, separately from with_native_core.
        crate::cmd_binary::install(child);
        for name in initialisation.copied_parent_variables() {
            let Some(original) = self.var_get_at(name, 0) else {
                continue;
            };
            let value = self.copy_original_jim_object_with_purpose_to(
                child,
                original,
                tcl_registry::special_vars::NativeJimInterpreterCopyPurpose::ChildStartupVariable,
            )?;
            child.var_set_at(name, value.as_ptr(), 0).map_err(|_| {
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "Jim original child global variable publication",
                )
            })?;
        }
        child.seal_native_compiler_tokens();
        Ok(())
    }

    /// interp_cmd_eval performs Concat even for one argument, then string
    /// copies in both directions. Jim returns the raw code without C options.
    fn eval_original_jim_child(&mut self, name: &[u8], arguments: &[*mut TclObj]) -> Code {
        // naming.interpreter.jim-original-object-crossing
        // docs/design/analysis/name-resolution-proofs/jim-original-object-crossing.md
        let script = match tcl_cmd_core::list::concat_selected(self, arguments) {
            Ok(script) => obj::Owned::retain(script),
            Err(error) => return self.report_cmd_error(error),
        };
        let parent = self.clone();
        match self.with_child(name, |child| {
            let copied = match parent.copy_original_jim_object_to(child, script.as_ptr()) {
                Ok(copied) => copied,
                Err(error) => return Ok(Err(error)),
            };
            let code = child.eval_body_obj(copied.as_ptr());
            if child.host_refusal_pending() {
                return Err(child.clone());
            }
            Ok(child
                .copy_original_jim_object_to(&parent, child.result_obj())
                .map(|result| (code, result)))
        }) {
            Some(Ok(Ok((code, result)))) => {
                self.set_result(result.as_ptr());
                code
            }
            Some(Ok(Err(error))) => self.report_cmd_error(error.into()),
            Some(Err(child)) => self.transport_host_refusal_from(&child),
            None => self.error(b"could not find interpreter"),
        }
    }

    /// The child holds the parent's original alias prefix List. Its elements
    /// keep their producing interpreter until the alias command retires.
    pub(super) fn install_original_jim_parent_alias(
        &mut self,
        child: &[u8],
        name: &[u8],
        arguments: &[*mut TclObj],
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        self.associate_native_jim_arguments(arguments)?;
        let context = self.native_jim_object_context()?;
        let original = obj::Owned::fresh(self.new_list_object(arguments));
        crate::native_source::bind_context(original.as_ptr(), &context)?;
        let strings = arguments
            .iter()
            .map(|&word| self.native_object_string_bytes(word))
            .collect::<Result<Vec<_>, _>>()?;
        let Some((target, prefix)) = strings.split_first() else {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim original parent alias prefix",
            ));
        };
        self.install_parent_alias_with_original(
            child,
            name,
            target.to_vec(),
            prefix.iter().map(|word| word.to_vec()).collect(),
            Some(original),
        )
    }

    pub(super) fn dispatch_original_jim_parent_alias(
        &mut self,
        parent: &mut Interp,
        original_prefix: &obj::Owned,
        argv: &[*mut TclObj],
    ) -> Code {
        // JimInterpAliasProc duplicates the original parent-owned List;
        // every appended child argument crosses as a fresh counted String.
        let prepared = (|| {
            let context = parent.native_jim_object_context()?;
            crate::native_source::bind_context(original_prefix.as_ptr(), &context)?;
            let target_script = obj::Owned::fresh(obj::duplicate(original_prefix.as_ptr()));
            let copies = argv[1..]
                .iter()
                .map(|&word| self.copy_original_jim_object_to(parent, word))
                .collect::<Result<Vec<_>, _>>()?;
            let pointers = copies.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
            crate::list::append_prepared_native_elements(
                target_script.as_ptr(),
                &pointers,
                tcl_syntax::native_string::NativeStringProtocol::Jim084,
            )?;
            Ok::<_, tcl_syntax::value::ValueError>(target_script)
        })();
        let target_script = match prepared {
            Ok(script) => script,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        super::CROSS_INTERP_DEPTH.with(|depth| depth.set(depth.get() + 1));
        let code = parent.eval_body_obj(target_script.as_ptr());
        super::CROSS_INTERP_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
        if parent.host_refusal_pending() {
            return self.transport_host_refusal_from(parent);
        }
        match parent.copy_original_jim_object_to(self, parent.result_obj()) {
            Ok(result) => {
                self.set_result(result.as_ptr());
                code
            }
            Err(error) => self.report_cmd_error(error.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interp::{Command, default_host};
    use crate::namespace::GLOBAL;

    fn parent(engine: &str) -> Interp {
        Interp::with_native_core(
            default_host(),
            crate::environment::profile_for_dialect(engine),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }

    fn token(parent: &Interp, namespace: usize, name: &[u8]) -> ChildInterpreterCommand {
        let Some(Command::ChildInterp(command)) =
            parent.namespaces.borrow().command_in(namespace, name)
        else {
            panic!("actual child command allocation");
        };
        command
    }

    #[test]
    fn jim_child_crossing_copies_counted_strings_and_preserves_original_context_refusal() {
        // naming.interpreter.jim-original-object-crossing
        // docs/design/analysis/name-resolution-proofs/jim-original-object-crossing.md
        // This Rust ownership control complements public native rows; it is no
        // claim about private pointer identities observed from jimsh.
        let producer_interp = parent("jim");
        let child = parent("jim");
        let producer = producer_interp.native_jim_object_context().unwrap();
        let receiver = child.native_jim_object_context().unwrap();
        let original = obj::Owned::fresh(obj::new_string_bytes(b"A\0\xc3\xa9\xe2\x82\xac"));
        crate::native_source::bind_context(original.as_ptr(), &producer).unwrap();
        crate::native_source::install_source(
            original.as_ptr(),
            crate::native_source::NativeJimSourceInfo {
                filename: producer.empty_object().clone(),
                line: 7,
            },
            &producer,
        )
        .unwrap();
        let copied = producer_interp
            .copy_original_jim_object_to(&child, original.as_ptr())
            .unwrap();
        assert_ne!(copied.as_ptr(), original.as_ptr());
        assert_eq!(obj::bytes_of(copied.as_ptr()), b"A\0\xc3\xa9\xe2\x82\xac");
        assert!(Rc::ptr_eq(
            &crate::native_source::context(copied.as_ptr()).unwrap(),
            &receiver
        ));
        assert!(Rc::ptr_eq(
            &crate::native_source::context(original.as_ptr()).unwrap(),
            &producer
        ));
        assert_eq!(
            crate::native_source::pin_source_info(original.as_ptr(), &producer)
                .unwrap()
                .line,
            7
        );
        assert_eq!(
            crate::native_source::pin_source_info(copied.as_ptr(), &receiver)
                .unwrap()
                .line,
            1
        );
        assert!(crate::native_source::bind_context(original.as_ptr(), &receiver).is_err());
        assert!(
            child
                .copy_original_jim_object_to(&producer_interp, original.as_ptr())
                .is_err()
        );
        assert!(
            producer_interp
                .copy_original_jim_object_to(&parent("tcl9.0"), original.as_ptr())
                .is_err()
        );
    }
    #[test]
    fn child_constructor_uses_the_explicit_parent_profile_recipe() {
        // Native proof: naming.bootstrap.original-root-cells
        // docs/design/analysis/name-resolution-proofs/bootstrap-original-root-cells.md
        // The constructor fixture establishes the native core's root-cell
        // presence. The logical-child control checks only embedding behavior.
        let mut logical = Interp::new();
        let logical_profile = logical.dialect_profile();
        assert!(
            tcl_registry::InvocationDialect::of_profile(logical_profile)
                .native_bootstrap_protocol()
                .is_none()
        );
        logical.create_child(Some(b"logical".to_vec()));
        logical
            .with_child(b"logical", |child| {
                assert!(std::ptr::eq(child.dialect_profile(), logical_profile));
                assert!(
                    child
                        .namespaces
                        .borrow()
                        .var_table(GLOBAL)
                        .has_native_namespace_cell(b"argv")
                );
            })
            .unwrap();

        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let mut parent = parent(engine);
            let profile = parent.dialect_profile();
            assert!(
                tcl_registry::InvocationDialect::of_profile(profile)
                    .native_bootstrap_protocol()
                    .is_some()
            );
            parent.create_child(Some(b"native".to_vec()));
            parent
                .with_child(b"native", |child| {
                    assert!(std::ptr::eq(child.dialect_profile(), profile), "{engine}");
                    assert_eq!(
                        child.runtime_version(),
                        profile.vm_runtime_version,
                        "{engine}"
                    );
                    let namespaces = child.namespaces.borrow();
                    let variables = namespaces.var_table(GLOBAL);
                    for name in [b"argv".as_slice(), b"argc", b"argv0", b"tcl_library"] {
                        assert!(
                            !variables.has_native_namespace_cell(name),
                            "{engine}: {name:?}"
                        );
                    }
                    assert!(child.find_command_id(GLOBAL, b"set").is_some(), "{engine}");
                    assert!(
                        child.find_command_id(GLOBAL, b"binary").is_some(),
                        "{engine}"
                    );
                })
                .unwrap();
        }
    }

    #[test]
    fn jim_child_bootstrap_copies_global_variable_extents_without_parent_object_sharing() {
        // naming.interpreter.original-child-bootstrap-context
        // docs/design/analysis/name-resolution-proofs/interpreter-original-child-bootstrap-context.md
        // Public native startup-value controls and source anchors govern this
        // purpose; no exhaustive extension roster or native header identity claim.
        let mut parent = parent("jim");
        assert!(parent.find_command_id(GLOBAL, b"binary").is_none());
        assert_eq!(parent.eval_str(br#"set ::argv "A\u0000B"; set ::argc COUNT; set ::argv0 "\u00e9\u20ac"; namespace eval ::jim {}; set ::jim::argv0 JARGV; set ::jim::exe JEXE; set ::jim::lineedit JLINE; namespace eval ::N {proc make {} {set argv LOCAL; return [interp]}}; ::N::make"#), Code::Ok);
        assert!(
            !parent.host_refusal_pending(),
            "{:?}",
            parent.native_access_refusal()
        );
        let name = parent.result_bytes();
        let parent_value = parent.var_get_at(b"argv", 0).unwrap();
        assert_eq!(
            parent
                .native_object_string_bytes(parent_value)
                .unwrap()
                .as_ref(),
            b"A\0B"
        );
        parent
            .with_child(&name, |child| {
                for command in [
                    b"namespace".as_slice(),
                    b"package",
                    b"interp",
                    b"array",
                    b"binary",
                ] {
                    assert!(child.find_command_id(GLOBAL, command).is_some());
                }
                let argv = child.var_get_at(b"argv", 0).unwrap();
                assert_ne!(argv, parent_value);
                assert_eq!(
                    child.native_object_string_bytes(argv).unwrap().as_ref(),
                    b"A"
                );
                assert_eq!(
                    child
                        .native_object_string_bytes(child.var_get_at(b"argc", 0).unwrap())
                        .unwrap()
                        .as_ref(),
                    b"COUNT"
                );
                assert_eq!(
                    child
                        .native_object_string_bytes(child.var_get_at(b"argv0", 0).unwrap())
                        .unwrap()
                        .as_ref(),
                    b"\xc3\xa9\xe2\x82\xac"
                );
                for (name, expected) in [
                    (b"jim::argv0".as_slice(), b"JARGV".as_slice()),
                    (b"jim::exe", b"JEXE"),
                    (b"jim::lineedit", b"JLINE"),
                ] {
                    assert_eq!(
                        child
                            .native_object_string_bytes(child.var_get_at(name, 0).unwrap())
                            .unwrap()
                            .as_ref(),
                        expected
                    );
                }
                assert!(matches!(
                    child.namespaces.borrow().resolve(GLOBAL, b"binary"),
                    Some(Command::Proc(_))
                ));
                assert!(
                    child
                        .find_command_id(GLOBAL, b"::tcl::binary::format")
                        .is_none()
                );
            })
            .unwrap();
        parent.delete_child(&name);
        assert_eq!(
            parent.eval_str(b"unset ::argv; set child [interp]; $child eval {info exists ::argv}"),
            Code::Ok
        );
        assert_eq!(parent.result_bytes(), b"0");
    }

    #[test]
    fn child_commands_keep_namespace_and_allocation_through_rename_and_recreation() {
        // Native proof: naming.interpreter.child-command-allocation-lifetime
        // docs/design/analysis/name-resolution-proofs/child-command-allocation-lifetime.md
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut parent = parent(engine);
            let namespace = parent
                .namespaces
                .borrow_mut()
                .ensure_namespace(GLOBAL, b"::N");
            parent.set_current_ns(namespace);
            parent.create_child(Some(b"kid".to_vec()));
            let original = token(&parent, GLOBAL, b"kid");
            assert!(
                parent
                    .namespaces
                    .borrow()
                    .command_in(namespace, b"kid")
                    .is_none()
            );
            assert_eq!(
                parent.rename_command(b"kid", b"moved"),
                crate::namespace::RenameOutcome::Renamed
            );
            parent.set_current_ns(GLOBAL);
            assert!(parent.delete_child(b"kid"));
            assert!(!parent.child_exists(b"kid"));
            assert!(
                parent
                    .namespaces
                    .borrow()
                    .command_in(namespace, b"moved")
                    .is_none()
            );
            parent.create_child(Some(b"kid".to_vec()));
            let recreated = token(&parent, GLOBAL, b"kid");
            assert!(!original.same_allocation(&recreated));
            let argv = [
                obj::Owned::fresh(obj::new_string_bytes(b"kid")),
                obj::Owned::fresh(obj::new_string_bytes(b"issafe")),
            ];
            let pointers: Vec<_> = argv.iter().map(obj::Owned::as_ptr).collect();
            assert_eq!(
                parent.dispatch_original_child(&original, &pointers),
                Code::Error
            );
            assert_eq!(
                parent.dispatch_original_child(&recreated, &pointers),
                Code::Ok
            );
        }
    }

    #[test]
    fn replacing_or_deleting_child_command_retires_only_its_actual_interpreter() {
        // Native proof: naming.interpreter.child-command-allocation-lifetime
        // docs/design/analysis/name-resolution-proofs/child-command-allocation-lifetime.md
        fn replacement(interp: &mut Interp, _: &[*mut TclObj]) -> Code {
            interp.set_result_bytes(b"replacement");
            Code::Ok
        }
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut parent = parent(engine);
            parent.create_child(Some(b"kid".to_vec()));
            parent.ns_register(b"kid", Command::Builtin(replacement));
            assert!(!parent.child_exists(b"kid"));
            assert!(matches!(
                parent.namespaces.borrow().command_in(GLOBAL, b"kid"),
                Some(Command::Builtin(_))
            ));
            parent.create_child(Some(b"active".to_vec()));
            let mut reentrant = parent.clone();
            parent
                .with_child(b"active", |old| {
                    assert!(reentrant.delete_child(b"active"));
                    reentrant.create_child(Some(b"active".to_vec()));
                    assert!(old.pending_delete.get());
                    assert!(old.eval_active.get() > 0);
                })
                .unwrap();
            assert!(parent.child_exists(b"active"));
            assert!(
                !parent
                    .children
                    .borrow()
                    .get(b"active".as_slice())
                    .unwrap()
                    .pending_delete
                    .get()
            );
        }
    }

    #[test]
    fn child_host_failure_keeps_its_typed_cause_and_parent_result() {
        // naming.interpreter.original-child-host-refusal-transport
        // docs/design/analysis/name-resolution-proofs/interpreter-original-child-host-refusal-transport.md
        // A host-worker control in the authored model, not native Tcl evidence.
        use tcl_syntax::raw_string::NativeValueAccessRefusal;
        fn worker(interp: &mut Interp, _: &[*mut TclObj]) -> Code {
            interp.refuse_native_access(NativeValueAccessRefusal::CommandProtocolUnavailable(
                "explicit child host provider",
            ))
        }
        for prior in [None, Some("earlier parent host provider")] {
            let mut parent = Interp::new();
            parent.create_child(Some(b"child".to_vec()));
            parent
                .with_child(b"child", |child| {
                    child.ns_register(b"host_worker", Command::Builtin(worker));
                })
                .unwrap();
            parent.set_result_bytes(b"PARENT RESULT");
            if let Some(prior) = prior {
                parent.refuse_native_access(NativeValueAccessRefusal::CommandProtocolUnavailable(
                    prior,
                ));
            }
            let original = obj::Owned::fresh(obj::new_string_bytes(b"host_worker"));
            assert_eq!(
                parent.child_eval_original(b"child", &[original.as_ptr()]),
                Code::Error
            );
            assert_eq!(
                parent.native_access_refusal(),
                Some(NativeValueAccessRefusal::CommandProtocolUnavailable(
                    prior.unwrap_or("explicit child host provider")
                ))
            );
            assert!(parent.host_refusal_pending());
            assert_eq!(parent.result_bytes(), b"PARENT RESULT");
            assert_eq!(
                parent
                    .children
                    .borrow()
                    .get(b"child".as_slice())
                    .unwrap()
                    .native_access_refusal(),
                Some(NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "explicit child host provider"
                ))
            );
        }
    }
    fn hex_bytes(value: &str) -> Vec<u8> {
        assert!(value.len().is_multiple_of(2));
        value
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    #[test]
    fn child_source_controls_match_native_creation_and_retirement_windows() {
        // Native proof: naming.interpreter.child-command-root-and-qualified-publication
        // docs/design/analysis/name-resolution-proofs/child-command-root-and-qualified-publication.md
        // Native proof: naming.interpreter.child-command-rename-replacement-deletion
        // docs/design/analysis/name-resolution-proofs/child-command-rename-replacement-deletion.md
        // Native proof: naming.interpreter.deleted-child-evaluation-and-recreation
        // docs/design/analysis/name-resolution-proofs/deleted-child-evaluation-and-recreation.md
        let providers = [
            (
                "tcl8.4",
                include_str!(
                    "../../tests/data/native_child_command_lifetime/v2/capture/8.4.20/stdout.tsv"
                ),
            ),
            (
                "tcl8.5",
                include_str!(
                    "../../tests/data/native_child_command_lifetime/v2/capture/8.5.19/stdout.tsv"
                ),
            ),
            (
                "tcl8.6",
                include_str!(
                    "../../tests/data/native_child_command_lifetime/v2/capture/8.6.18/stdout.tsv"
                ),
            ),
            (
                "tcl9.0",
                include_str!(
                    "../../tests/data/native_child_command_lifetime/v2/capture/9.0.4/stdout.tsv"
                ),
            ),
            (
                "tcl9.1",
                include_str!(
                    "../../tests/data/native_child_command_lifetime/v2/capture/9.1.0/stdout.tsv"
                ),
            ),
        ];
        let controls = [
            ("namespace-publication-intermediate", include_bytes!("../../tests/data/native_child_command_lifetime/v2/protocol/inputs/namespace-publication-intermediate.tcl").as_slice()),
            ("namespace-relative-root-lookup", include_bytes!("../../tests/data/native_child_command_lifetime/v2/protocol/inputs/namespace-relative-root-lookup.tcl").as_slice()),
            ("namespace-root-child-survives-delete", include_bytes!("../../tests/data/native_child_command_lifetime/v2/protocol/inputs/namespace-root-child-survives-delete.tcl").as_slice()),
            ("namespace-qualified-publication", include_bytes!("../../tests/data/native_child_command_lifetime/v2/protocol/inputs/namespace-qualified-publication.tcl").as_slice()),
            ("namespace-relative-qualified-publication", include_bytes!("../../tests/data/native_child_command_lifetime/v2/protocol/inputs/namespace-relative-qualified-publication.tcl").as_slice()),
            ("namespace-renamed-child-deleted-with-namespace", include_bytes!("../../tests/data/native_child_command_lifetime/v2/protocol/inputs/namespace-renamed-child-deleted-with-namespace.tcl").as_slice()),
            ("renamed-child-delete-recreate", include_bytes!("../../tests/data/native_child_command_lifetime/v2/protocol/inputs/renamed-child-delete-recreate.tcl").as_slice()),
            ("active-child-continuation-errors", include_bytes!("../../tests/data/native_child_command_lifetime/v2/protocol/inputs/active-child-continuation-errors.tcl").as_slice()),
            ("active-child-replacement-survives-error", include_bytes!("../../tests/data/native_child_command_lifetime/v2/protocol/inputs/active-child-replacement-survives-error.tcl").as_slice()),
        ];
        for (engine, original) in providers {
            for (case, source) in controls {
                let code = original
                    .lines()
                    .find_map(|line| {
                        let words: Vec<_> = line.split('|').collect();
                        (words.first() == Some(&"CODE") && words[1] == case && words[2] == "direct")
                            .then(|| words[3].parse::<i32>().unwrap())
                    })
                    .unwrap();
                let result = original
                    .lines()
                    .find_map(|line| {
                        let words: Vec<_> = line.split('|').collect();
                        (words.first() == Some(&"VALUE")
                            && words[1] == case
                            && words[2] == "direct"
                            && words[3] == "result")
                            .then(|| hex_bytes(words.last().unwrap()))
                    })
                    .unwrap();
                let mut interp = parent(engine);
                let actual = interp.eval_str(source);
                assert!(
                    !interp.host_refusal_pending(),
                    "{engine}/{case}: genuine selected runtime provider"
                );
                assert_eq!(actual.as_int(), i64::from(code), "{engine}/{case}");
                assert_eq!(interp.result_bytes(), result, "{engine}/{case}");
                if case == "active-child-continuation-errors" && engine != "tcl8.4" {
                    let original_options = original
                        .lines()
                        .find_map(|line| {
                            let words: Vec<_> = line.split('|').collect();
                            (words.first() == Some(&"VALUE")
                                && words[1] == case
                                && words[2] == "direct"
                                && words[3] == "return-options")
                                .then(|| hex_bytes(words.last().unwrap()))
                        })
                        .unwrap();
                    let original_options = String::from_utf8(original_options).unwrap();
                    let fields = tcl_syntax::list::split_list(&original_options).unwrap();
                    let original_code = fields
                        .chunks_exact(2)
                        .find(|pair| pair[0] == "-errorcode")
                        .unwrap();
                    assert_eq!(
                        interp.error_code(),
                        original_code[1].as_bytes(),
                        "{engine}/{case} error code"
                    );
                }
            }
        }
    }
}
