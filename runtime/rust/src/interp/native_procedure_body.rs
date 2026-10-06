// SPDX-License-Identifier: AGPL-3.0-or-later
//! The actual procedure body owner, chosen before argument-list parsing or temporary retention.

use super::Interp;
use crate::obj::{self, Owned, TclObj};
use tcl_registry::native_procedure_body::NativeProcedureBodyAction;
use tcl_syntax::value::{ValueError, ValueOps};

impl Interp {
    pub(super) fn acquire_native_procedure_execution(
        &mut self,
        procedure: Option<&std::rc::Rc<super::ProcDef>>,
    ) -> Result<(), ValueError> {
        if !self
            .native_invocation_dialect()
            .native_string_protocol()
            .is_some_and(|protocol| protocol.tcl_version().is_some())
        {
            return Ok(());
        }
        let Some(procedure) = procedure else {
            return Ok(());
        };
        let role =
            tcl_runtime_api::native_procedure_roles::NativeProcedureReference::acquire(procedure)
                .map_err(|_| {
                ValueError::CommandProtocolUnavailable("native procedure declaration lifetime")
            })?;
        self.frames
            .borrow_mut()
            .retain_native_procedure_execution(role);
        Ok(())
    }

    pub(super) fn release_native_procedure_execution(&mut self) {
        let role = self.frames.borrow_mut().take_native_procedure_execution();
        drop(role);
    }

    /// Select the actual ordinary procedure header at its post-publication
    /// string-access boundary. Only a possible bare `args` header reaches the
    /// original body getter; other bodies keep their original representation.
    pub(super) fn original_procedure_compiler_header(
        &mut self,
        parameters: Option<*mut TclObj>,
        original_body: *mut TclObj,
    ) -> Result<tcl_dialect::NativeProcedureHeaderCompilation, ValueError> {
        use tcl_dialect::NativeProcedureHeaderCompilation as Header;
        let dialect = self.native_invocation_dialect();
        if dialect.family() == Some(tcl_dialect::model::Family::Jim) {
            return Ok(Header::Absent);
        }
        let Some(parameters) = parameters else {
            return Ok(Header::Unknown);
        };
        let parameters = ValueOps::native_string_bytes(self, &parameters)?;
        let incomplete = tcl_registry::native_procedure::procedure_header_compilation_bytes(
            dialect,
            Some(&parameters),
            None,
            Some(false),
        );
        if incomplete != Header::Unknown {
            return Ok(incomplete);
        }
        let body = ValueOps::native_string_bytes(self, &original_body)?;
        Ok(
            tcl_registry::native_procedure::procedure_header_compilation_bytes(
                dialect,
                Some(&parameters),
                Some(&body),
                Some(false),
            ),
        )
    }

    pub(crate) fn native_callable_interpreter(
        &self,
    ) -> tcl_runtime_api::native_compilation::NativeInterpreterIdentity {
        self.native_command_interpreter
    }

    /// Create genuine commandless Proc clientData from its already selected body.
    pub(crate) fn create_native_callable_from_chosen(
        &self,
        params: Vec<super::Param>,
        body: Owned,
        namespace: crate::namespace::NsId,
        source: Option<std::rc::Rc<[u8]>>,
        body_line_base: u32,
    ) -> super::NativeCallableProcedure {
        super::NativeCallableProcedure::new(
            params,
            body,
            super::NativeProcedureDefinition {
                location: super::ProcLocation {
                    namespace,
                    qualified_name: Vec::new(),
                    jim_namespace: None,
                },
                jim_parameters: None,
                source,
                body_line_base,
                native: None,
                statics: None,
            },
        )
    }

    /// Acquire the chosen ordinary body once. Shared C bodies are counted string-only copies.
    pub(crate) fn choose_original_procedure_body(
        &mut self,
        original: *mut TclObj,
    ) -> Result<Owned, ValueError> {
        let creation = self
            .native_invocation_dialect()
            .native_procedure_body_creation_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native procedure body creation",
            ))?;
        let shared = obj::is_shared(original);
        match creation.action(shared) {
            NativeProcedureBodyAction::RetainOriginal => Ok(Owned::retain(original)),
            NativeProcedureBodyAction::CopyCountedString => {
                let bytes = ValueOps::native_string_bytes(self, &original)?;
                Ok(Owned::fresh(obj::new_string_bytes(&bytes)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_runtime_api::native_procedure_roles::NativeProcedureRoleOwner;

    fn observe_active_procedure_role(
        interp: &mut Interp,
        _arguments: &[*mut TclObj],
    ) -> super::super::Code {
        let procedure = std::rc::Rc::clone(
            interp
                .frames
                .borrow()
                .current_c_procedure()
                .expect("actual C procedure frame"),
        );
        let expected = if interp.command_exists(b"p") { 2 } else { 1 };
        assert_eq!(
            procedure.native_procedure_role_ledger().references(),
            expected
        );
        assert!(!procedure.native_procedure_role_ledger().is_retired());
        assert!(procedure.body.checked_ptr().is_ok());
        interp.set_result_bytes(b"");
        super::super::Code::Ok
    }

    #[test]
    fn original_c_frame_retains_deleted_declaration_until_raw_body_finishes() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = Interp::with_native_core(
                super::super::default_host(),
                crate::environment::profile_for_dialect(profile),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            interp.register_builtin(
                b"observe_active_procedure_role",
                observe_active_procedure_role,
            );
            let default = Owned::fresh(obj::new_string_bytes(b"DEFAULT"));
            let body = Owned::fresh(obj::new_string_bytes(
                b"observe_active_procedure_role; rename p {}; observe_active_procedure_role; return DONE",
            ));
            interp.define_proc(
                b"p",
                vec![super::super::Param {
                    name: b"x".to_vec(),
                    default: Some(default.clone()),
                }],
                body.as_ptr(),
            );
            drop(body);
            let Some(super::super::Command::Proc(binding)) = interp
                .namespaces
                .borrow()
                .resolve(crate::namespace::GLOBAL, b"p")
            else {
                panic!("native procedure publication: {profile}");
            };
            let query = binding.declaration();
            let original_body = query.body.clone();
            assert_eq!(query.native_procedure_role_ledger().references(), 1);
            let name = Owned::fresh(obj::new_string_bytes(b"p"));
            assert_eq!(
                interp.call_proc(&query, &[name.as_ptr()]),
                super::super::Code::Ok,
                "{profile}"
            );
            assert_eq!(interp.result_bytes(), b"DONE", "{profile}");
            assert_eq!(query.native_procedure_role_ledger().references(), 0);
            assert!(query.native_procedure_role_ledger().is_retired());
            assert!(original_body.checked_ptr().is_err());
            // The surviving declaration and body views retain allocations, not
            // the original default object's native procedure reference.
            assert_eq!(unsafe { (*default.as_ptr()).ref_count }, 1, "{profile}");
            assert!(interp.native_access_refusal().is_none(), "{profile}");
        }
    }

    #[test]
    fn rejected_native_formal_binding_acquires_no_execution_reference() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = Interp::with_native_core(
                super::super::default_host(),
                crate::environment::profile_for_dialect(profile),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let body = Owned::fresh(obj::new_string_bytes(b"return DONE"));
            interp.define_proc(
                b"p",
                vec![super::super::Param {
                    name: b"x".to_vec(),
                    default: None,
                }],
                body.as_ptr(),
            );
            drop(body);
            let Some(super::super::Command::Proc(binding)) = interp
                .namespaces
                .borrow()
                .resolve(crate::namespace::GLOBAL, b"p")
            else {
                panic!("native procedure publication: {profile}");
            };
            let query = binding.declaration();
            let name = Owned::fresh(obj::new_string_bytes(b"p"));
            assert_eq!(
                interp.call_proc(&query, &[name.as_ptr()]),
                super::super::Code::Error,
                "{profile}"
            );
            assert_eq!(query.native_procedure_role_ledger().references(), 1);
            assert!(!query.native_procedure_role_ledger().is_retired());
            assert!(interp.frames.borrow().current_c_procedure().is_none());
            assert_eq!(
                interp.rename_command(b"p", b""),
                crate::namespace::RenameOutcome::Deleted
            );
            assert!(query.native_procedure_role_ledger().is_retired());
            assert!(query.body.checked_ptr().is_err());
        }
    }

    fn assert_retired_original_getters(interp: &mut Interp, original: &obj::ProcedureObject) {
        let pointer = original.as_ptr();
        assert!(!obj::allocation_is_live(pointer));
        let before = unsafe {
            (
                (*pointer).bytes,
                (*pointer).type_ptr,
                (*pointer).internal_rep,
                (*pointer).length,
                (*pointer).ref_count,
            )
        };
        assert!(original.checked_ptr().is_err());
        assert!(!obj::native_string_available(pointer));
        assert!(interp.native_object_string_bytes(pointer).is_err());
        assert!(ValueOps::native_string_bytes(interp, &pointer).is_err());
        assert!(ValueOps::native_char_len(interp, &pointer).is_err());
        assert!(ValueOps::native_unicode_units(interp, &pointer).is_err());
        assert!(ValueOps::native_object_snapshot(interp, &pointer).is_err());
        assert!(ValueOps::as_int(interp, &pointer).is_err());
        assert!(ValueOps::as_double(interp, &pointer).is_err());
        assert!(ValueOps::as_bool(interp, &pointer).is_err());
        assert!(ValueOps::list_len(interp, &pointer).is_err());
        assert!(ValueOps::list_index(interp, &pointer, 0).is_err());
        assert!(ValueOps::list_elements(interp, &pointer).is_err());
        assert!(ValueOps::dict_pairs(interp, &pointer).is_err());
        assert!(crate::typed_value::scalar_number(
            pointer,
            interp.native_invocation_dialect(),
            true,
        )
        .is_err());
        assert!(
            crate::typed_value::boolean_in(pointer, interp.native_invocation_dialect()).is_err()
        );
        assert!(ValueOps::discard_native_internal_representation(interp, &pointer).is_err());
        assert!(crate::typed_value::native_scalar_probe(
            pointer,
            interp.native_invocation_dialect(),
            tcl_syntax::scalar_getter::NativeScalarGetterKind::Wide,
        )
        .is_err());
        assert!(crate::typed_value::native_number_probe(
            pointer,
            interp.native_invocation_dialect(),
            tcl_syntax::scalar_getter::NativeNumberGetterKind::Number,
        )
        .is_err());
        let protocol = interp
            .native_invocation_dialect()
            .native_string_protocol()
            .unwrap();
        assert!(crate::dict::native_object_bytes(pointer, protocol).is_err());
        assert_eq!(
            unsafe {
                (
                    (*pointer).bytes,
                    (*pointer).type_ptr,
                    (*pointer).internal_rep,
                    (*pointer).length,
                    (*pointer).ref_count,
                )
            },
            before,
            "a refused getter must preserve the retired header"
        );
        assert!(!obj::allocation_is_live(pointer));
    }

    #[test]
    fn native_liveness_getters_accept_fresh_unowned_headers() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = Interp::with_native_core(
                super::super::default_host(),
                crate::environment::profile_for_dialect(profile),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let original = obj::new_string_bytes(b"17");
            assert_eq!(unsafe { (*original).ref_count }, 0);
            assert_eq!(
                ValueOps::native_string_bytes(&mut interp, &original)
                    .unwrap()
                    .as_ref(),
                b"17"
            );
            assert_eq!(ValueOps::as_int(&mut interp, &original).unwrap(), 17);
            assert!(obj::allocation_is_live(original));
            assert_eq!(unsafe { (*original).ref_count }, 0);
            super::super::drop_fresh(original);
        }
    }

    #[test]
    fn retired_native_procedure_views_refuse_getters_without_regenerating_headers() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = Interp::with_native_core(
                super::super::default_host(),
                crate::environment::profile_for_dialect(profile),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let body = Owned::fresh(obj::new_string_bytes(b"return DONE"));
            interp.define_proc(
                b"p",
                vec![super::super::Param {
                    name: b"x".to_vec(),
                    default: Some(Owned::fresh(obj::new_string_bytes(b"17"))),
                }],
                body.as_ptr(),
            );
            drop(body);
            let query = interp.proc_def(b"p").unwrap();
            let body_view = query.body.clone();
            let default_view = query.params[0].default.as_ref().unwrap().clone();
            assert_eq!(
                interp.rename_command(b"p", b""),
                crate::namespace::RenameOutcome::Deleted,
                "{profile}"
            );
            assert!(query.check_native_liveness().is_err());
            assert_retired_original_getters(&mut interp, &body_view);
            assert_retired_original_getters(&mut interp, &default_view);
            let head = Owned::fresh(obj::new_string_bytes(b"p"));
            assert_eq!(
                interp.call_proc(&query, &[head.as_ptr()]),
                super::super::Code::Error,
                "{profile}"
            );
            assert!(interp.native_access_refusal().is_some(), "{profile}");
            assert!(unsafe { (*body_view.as_ptr()).bytes }.is_null());
            assert!(unsafe { (*default_view.as_ptr()).bytes }.is_null());
        }
    }

    #[test]
    fn c_interpreter_shutdown_retires_visible_and_hidden_bindings_despite_query_views() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = Interp::with_native_core(
                super::super::default_host(),
                crate::environment::profile_for_dialect(profile),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let mut transports = Vec::new();
            for name in [b"visible".as_slice(), b"hidden"] {
                let body = Owned::fresh(obj::new_string_bytes(b"return DONE"));
                interp.define_proc(
                    name,
                    vec![super::super::Param {
                        name: b"x".to_vec(),
                        default: Some(Owned::fresh(obj::new_string_bytes(b"DEFAULT"))),
                    }],
                    body.as_ptr(),
                );
                drop(body);
                let command = interp
                    .namespaces
                    .borrow()
                    .resolve(crate::namespace::GLOBAL, name)
                    .unwrap();
                let super::super::Command::Proc(binding) = command else {
                    panic!("original procedure binding: {profile}");
                };
                let query = binding.declaration();
                let body = query.body.clone();
                let default = query.params[0].default.as_ref().unwrap().clone();
                assert_eq!(query.native_procedure_role_ledger().references(), 1);
                transports.push((binding, query, body, default));
            }
            assert_eq!(
                interp.hide_command(b"hidden", b"hidden"),
                super::super::CommandVisibilityOutcome::Moved,
                "{profile}"
            );
            drop(interp);
            for (binding, query, body, default) in transports {
                assert_eq!(
                    query.native_procedure_role_ledger().references(),
                    0,
                    "{profile}"
                );
                assert!(
                    query.native_procedure_role_ledger().is_retired(),
                    "{profile}"
                );
                assert!(binding.declaration().check_native_liveness().is_err());
                assert!(body.checked_ptr().is_err());
                assert!(default.checked_ptr().is_err());
                assert!(unsafe { (*body.as_ptr()).bytes }.is_null());
                assert!(unsafe { (*default.as_ptr()).bytes }.is_null());
            }
        }
    }
    #[test]
    fn chosen_body_copies_shared_cache_and_keeps_unshared_original() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = Interp::with_native_core(
                super::super::default_host(),
                crate::environment::profile_for_dialect(profile),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let recipe = interp
                .native_invocation_dialect()
                .byte_array_string_recipe(None)
                .unwrap();
            let unshared = Owned::fresh(crate::bytearray::new_byte_array(b"return\0\xff", recipe));
            let retained = interp
                .choose_original_procedure_body(unshared.as_ptr())
                .unwrap();
            assert_eq!(retained.as_ptr(), unshared.as_ptr());
            // No string getter was reached merely to retain an unshared body.
            assert!(unsafe { (*unshared.as_ptr()).bytes }.is_null());
            drop(retained);
            let alias = Owned::retain(unshared.as_ptr());
            let copy = interp
                .choose_original_procedure_body(unshared.as_ptr())
                .unwrap();
            assert_ne!(copy.as_ptr(), unshared.as_ptr());
            assert!(unsafe { (*copy.as_ptr()).type_ptr }.is_null());
            assert_eq!(
                ValueOps::native_string_bytes(&mut interp, &copy.as_ptr())
                    .unwrap()
                    .as_ref(),
                b"return\xc0\x80\xc3\xbf"
            );
            assert!(!unsafe { (*unshared.as_ptr()).type_ptr }.is_null());
            drop(alias);
        }
    }
}
