// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original command-object lookup and metadata-only cache validation.

use super::{Command, Interp};
use crate::{
    namespace::NsId,
    obj::{self, TclObj},
};
use tcl_runtime_api::native_command_name::{NativeCommandNameCache, NativeCommandNameLookupState};
use tcl_syntax::value::{ValueError, ValueOps};

impl Interp {
    /// Resolve the same original command object before reporting its imported origin.
    /// The reporting query consumes the selected token without a second name lookup.
    pub(crate) fn native_namespace_origin(
        &mut self,
        original: *mut TclObj,
    ) -> Result<Option<Vec<u8>>, ValueError> {
        let Some((command, token)) = self.resolve_original_command(original)? else {
            return Ok(None);
        };
        // A lookup worker clone is not an invocation or an origin-reporting owner.
        drop(command);
        let token = token.ok_or(ValueError::CommandProtocolUnavailable(
            "original command origin token",
        ))?;
        let (name, _) = self.raw_command_location_by_generation(token).ok_or(
            ValueError::CommandProtocolUnavailable("original command origin placement"),
        )?;
        let command = tcl_runtime_api::CommandId(self.intern_cmd(&name, token));
        tcl_cmd_core::namespace::origin_from_command_checked(self, command).map(Some)
    }

    /// Produce the C full-command-name String independently of the input cache.
    pub(crate) fn native_namespace_origin_result(
        &self,
        bytes: &[u8],
    ) -> Result<obj::Owned, ValueError> {
        let dialect = self.native_invocation_dialect();
        let strings = dialect
            .native_command_name_protocol()
            .and_then(|_| dialect.native_string_materialization(None))
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native origin String issuer",
            ))?;
        let original = obj::Owned::fresh(obj::new_string_bytes(bytes));
        obj::retain_native_string_representation(original.as_ptr(), strings)?;
        Ok(original)
    }

    /// Present the selected C origin lookup failure with its actual String
    /// result birth, retaining the original command operand for the getter.
    pub(crate) fn native_namespace_origin_failure(&mut self, original: *mut TclObj) -> super::Code {
        let dialect = self.native_invocation_dialect();
        let Some(strings) = dialect
            .native_command_name_protocol()
            .and_then(|_| dialect.native_string_materialization(None))
        else {
            return self.report_cmd_error(
                ValueError::CommandProtocolUnavailable("native origin diagnostic String issuer")
                    .into(),
            );
        };
        let name = match self.native_string_bytes(&original) {
            Ok(name) => name,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        let name = tcl_core_types::c_string_extent(&name);
        let mut message = b"invalid command name \"".to_vec();
        message.extend_from_slice(name);
        message.push(b'"');
        let code = super::error_code_list(&[b"TCL", b"LOOKUP", b"COMMAND", name]);
        let error = if dialect.tcl_version == Some(tcl_dialect::TclVersion::V8_4) {
            tcl_cmd_core::CmdError::new_bytes(message)
        } else {
            tcl_cmd_core::CmdError::with_error_code_bytes(message, code)
        };
        self.report_cmd_error(error.with_native_string_result(strings.protocol()))
    }

    /// Observe the actual current reference context and original node identity.
    /// This grants no worker ownership or name-derived cache authority.
    #[cfg(test)]
    pub(crate) fn native_command_name_lookup_state(
        &self,
        cache: &NativeCommandNameCache,
    ) -> Result<NativeCommandNameLookupState, ValueError> {
        self.native_command_name_lookup_state_at(cache, self.current_ns.get())
    }

    fn native_command_name_lookup_state_at(
        &self,
        cache: &NativeCommandNameCache,
        origin: NsId,
    ) -> Result<NativeCommandNameLookupState, ValueError> {
        self.native_invocation_dialect()
            .native_command_name_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native command-name lookup",
            ))?;
        let world = self.namespaces.borrow();
        Ok(NativeCommandNameLookupState {
            interpreter: self.native_command_interpreter,
            reference: world.native_command_reference(origin),
            target: world.native_command_target(cache.token),
        })
    }

    /// Capture a named registration selected in the actual namespace context.
    pub(crate) fn native_command_name_from_binding(
        &self,
        origin: NsId,
        original: &[u8],
    ) -> Result<Option<NativeCommandNameCache>, ValueError> {
        let protocol = self
            .native_invocation_dialect()
            .native_command_name_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native command-name binding",
            ))?;
        let world = self.namespaces.borrow();
        let path =
            world
                .native_context_path(origin)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "native command-name binding namespace",
                ))?;
        let names = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native command-name binding policy",
            ))?;
        let selected = names
            .recipe()
            .command_lookup_input(tcl_syntax::naming::NativeNameContext::new(&path), original)
            .map_err(|_| {
                ValueError::CommandProtocolUnavailable("native command-name binding projection")
            })?;
        Ok(world.native_command_name_cache(
            self.native_command_interpreter,
            origin,
            selected.selected(),
            protocol,
        ))
    }

    /// Resolve the original object, checking native cache state before lookup.
    /// Failed lookup retains the C9 primary or applies C8's null descriptor.
    pub(crate) fn resolve_original_command(
        &mut self,
        original: *mut TclObj,
    ) -> Result<Option<(Command, Option<u64>)>, ValueError> {
        self.resolve_original_command_at(self.current_ns.get(), original)
    }

    /// Resolve an original object in a separately retained lookup namespace,
    /// without changing the caller's variable frame or namespace.
    pub(crate) fn resolve_original_command_at(
        &mut self,
        current: NsId,
        original: *mut TclObj,
    ) -> Result<Option<(Command, Option<u64>)>, ValueError> {
        let dialect = self.native_invocation_dialect();
        let Some(protocol) = dialect.native_command_name_protocol() else {
            if dialect.native_jim_lookup_protocol().is_some() {
                return self.resolve_original_jim_command(current, original);
            }
            let bytes = self.native_string_bytes(&original)?;
            return Ok(self.resolve_dispatchable_with_generation(current, &bytes));
        };
        if protocol.version() >= tcl_dialect::TclVersion::V8_5 {
            if let Some(cache) = obj::native_command_name_cache(original) {
                if protocol.cache_is_current(
                    &cache,
                    &self.native_command_name_lookup_state_at(&cache, current)?,
                ) {
                    if let Some((command, fqn)) =
                        self.namespaces.borrow().native_command_at_node(cache.token)
                    {
                        return Ok(self
                            .command_visible_for_surface_at(&command, &fqn, Some(cache.token))
                            .then_some((command, Some(cache.token))));
                    }
                }
            }
        }
        let bytes = self.native_string_bytes(&original)?;
        let world = self.namespaces.borrow();
        let path =
            world
                .native_context_path(current)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original command-name namespace",
                ))?;
        let names = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original command-name naming policy",
            ))?;
        let projected = names
            .recipe()
            .command_lookup_input(tcl_syntax::naming::NativeNameContext::new(&path), &bytes)
            .map_err(|_| {
                ValueError::CommandProtocolUnavailable("original command-name projection")
            })?;
        let reference = protocol.lookup_reference(
            projected.qualification() == tcl_syntax::naming::NativeNameQualification::Absolute,
            world.native_command_reference(current),
            world.native_command_reference(crate::namespace::GLOBAL),
        );
        let origin = reference.map_or(Ok(current), |reference| {
            NsId::try_from(reference.namespace_token).map_err(|_| {
                ValueError::CommandProtocolUnavailable("original command-name reference context")
            })
        })?;
        drop(world);
        if let Some(cache) = obj::native_command_name_cache(original) {
            let state = self.native_command_name_lookup_state_at(&cache, origin)?;
            if protocol.cache_is_current(&cache, &state) {
                let binding = self.namespaces.borrow().native_command_at_node(cache.token);
                if let Some((command, fqn)) = binding {
                    return Ok(self
                        .command_visible_for_surface_at(&command, &fqn, Some(cache.token))
                        .then_some((command, Some(cache.token))));
                }
            }
        }
        let binding = self.native_command_name_from_binding(origin, &bytes)?;
        if let Some(cache) = binding {
            let node = self.namespaces.borrow().native_command_at_node(cache.token);
            obj::install_native_command_name_cache(original, cache.clone(), dialect)?;
            if let Some((command, fqn)) = node {
                return Ok(self
                    .command_visible_for_surface_at(&command, &fqn, Some(cache.token))
                    .then_some((command, Some(cache.token))));
            }
        } else {
            obj::install_unresolved_native_command_name_cache(original, dialect)?;
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{interp::Code, namespace::GLOBAL};
    use tcl_dialect::TclVersion;

    fn worker(_interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
        Code::Ok
    }

    fn head(bytes: &[u8]) -> obj::Owned {
        obj::Owned::fresh(obj::new_string_bytes(bytes))
    }

    fn native_interpreter(version: TclVersion) -> Interp {
        Interp::with_native_core(
            super::super::default_host(),
            crate::environment::profile_for_dialect(version.dialect_profile_name()),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }

    #[test]
    fn origin_string_result_requires_actual_native_core_issuer() {
        let mut interp = Interp::new();
        interp.set_dialect_profile(tcl_dialect::DialectProfile::irules());
        assert!(
            interp
                .native_namespace_origin_result(b"::selected")
                .is_err()
        );
    }

    #[test]
    fn native_namespace_origin_retains_string_result_birth_and_opaque_diagnostics() {
        let source = include_bytes!("../../tests/data/native_namespace_origin_failures/source.tcl");
        let rows = include_str!("../../tests/data/native_namespace_origin_failures/controls.tsv");
        for (version, row) in TclVersion::ALL.into_iter().zip(rows.lines()) {
            let mut interp = native_interpreter(version);
            assert_eq!(interp.eval_str(source), Code::Ok, "{version:?}");
            let expected = row.split_once('\t').unwrap().1;
            let expected: Vec<_> = expected
                .as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect();
            assert_eq!(interp.result_bytes(), expected, "{version:?}");
            let result = interp
                .native_namespace_origin_result(b"::selected")
                .unwrap();
            assert!(matches!(
                obj::native_object_snapshot(result.as_ptr()).unwrap().cache,
                tcl_syntax::native_object::NativeObjectCacheSnapshot::String { .. }
            ));
        }
    }

    #[test]
    fn original_namespace_origin_tracks_native_import_renames_and_replacement() {
        for version in TclVersion::ALL {
            let mut interp = native_interpreter(version);
            assert_eq!(
                interp.eval_str(b"namespace eval src {proc p {} {return SOURCE};namespace export p};namespace eval mid {namespace import ::src::p;namespace export p};namespace eval dest {namespace import ::mid::p}"),
                Code::Ok,
            );
            let original = head(b"::dest::p");
            assert_eq!(
                interp.native_namespace_origin(original.as_ptr()).unwrap(),
                Some(b"::src::p".to_vec())
            );
            let cache = obj::native_command_name_cache(original.as_ptr()).unwrap();
            assert_eq!(interp.eval_str(b"rename ::src::p ::src::q"), Code::Ok);
            assert_eq!(
                interp.native_namespace_origin(original.as_ptr()).unwrap(),
                Some(b"::src::q".to_vec())
            );
            assert_eq!(
                obj::native_command_name_cache(original.as_ptr())
                    .unwrap()
                    .token,
                cache.token
            );
            assert_eq!(
                interp.eval_str(b"rename ::dest::p {};proc ::dest::p {} {return NEW}"),
                Code::Ok
            );
            assert_eq!(
                interp.native_namespace_origin(original.as_ptr()).unwrap(),
                Some(b"::dest::p".to_vec())
            );
            assert_ne!(
                obj::native_command_name_cache(original.as_ptr())
                    .unwrap()
                    .token,
                cache.token
            );
            assert_eq!(obj::bytes_of(original.as_ptr()), b"::dest::p");
        }
    }

    #[test]
    fn original_namespace_origin_uses_current_stringless_cache_and_refuses_stale_cache() {
        for version in [
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let mut interp = native_interpreter(version);
            assert_eq!(interp.eval_str(b"proc p {} {return P}"), Code::Ok);
            let original = head(b"p");
            assert_eq!(
                interp.native_namespace_origin(original.as_ptr()).unwrap(),
                Some(b"::p".to_vec())
            );
            let cache = obj::native_command_name_cache(original.as_ptr()).unwrap();
            obj::invalidate_string(original.as_ptr());
            let references = unsafe { (*original.as_ptr()).ref_count };
            assert_eq!(
                interp.native_namespace_origin(original.as_ptr()).unwrap(),
                Some(b"::p".to_vec())
            );
            assert!(!obj::has_string_rep(original.as_ptr()));
            assert_eq!(unsafe { (*original.as_ptr()).ref_count }, references);
            assert_eq!(
                obj::native_command_name_cache(original.as_ptr()),
                Some(cache)
            );
            assert_eq!(
                interp.eval_str(b"rename p {};proc p {} {return REPLACED}"),
                Code::Ok
            );
            assert!(interp.native_namespace_origin(original.as_ptr()).is_err());
            assert!(!obj::has_string_rep(original.as_ptr()));
        }
    }

    #[test]
    fn current_original_command_cache_needs_no_resident_string() {
        for version in [
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let mut interp = Interp::new();
            interp.set_runtime_version(version);
            interp.bind_command_replacement(GLOBAL, b"p", Command::Builtin(worker));
            let original = head(b"p");
            let selected = interp
                .resolve_original_command(original.as_ptr())
                .unwrap()
                .unwrap()
                .1;
            let cache = obj::native_command_name_cache(original.as_ptr()).unwrap();
            obj::invalidate_string(original.as_ptr());
            assert!(!obj::has_string_rep(original.as_ptr()));
            assert_eq!(
                interp
                    .resolve_original_command(original.as_ptr())
                    .unwrap()
                    .unwrap()
                    .1,
                selected
            );
            assert_eq!(
                obj::native_command_name_cache(original.as_ptr()),
                Some(cache)
            );
            assert!(!obj::has_string_rep(original.as_ptr()));
        }
    }

    #[test]
    fn absolute_c84_getter_reuses_global_reference_inside_a_namespace() {
        let mut interp = Interp::new();
        interp.set_runtime_version(TclVersion::V8_4);
        interp.bind_command_replacement(GLOBAL, b"p", Command::Builtin(worker));
        let namespace = interp
            .namespaces
            .borrow_mut()
            .ensure_namespace(GLOBAL, b"n");
        interp.set_current_ns(namespace);
        for (spelling, reference) in [(b"::p".as_slice(), GLOBAL), (b"p".as_slice(), namespace)] {
            let original = head(spelling);
            interp
                .resolve_original_command(original.as_ptr())
                .unwrap()
                .unwrap();
            let cache = obj::native_command_name_cache(original.as_ptr()).unwrap();
            assert_eq!(cache.reference.unwrap().namespace_token, reference as u64);
            let primary = obj::internal_rep(original.as_ptr());
            interp
                .resolve_original_command(original.as_ptr())
                .unwrap()
                .unwrap();
            assert_eq!(obj::internal_rep(original.as_ptr()), primary);
            assert_eq!(
                obj::native_command_name_cache(original.as_ptr()),
                Some(cache)
            );
            assert_eq!(obj::bytes_of(original.as_ptr()), spelling);
        }
    }

    #[test]
    fn same_original_object_rebases_and_local_shadowing_invalidates_global_cache() {
        let mut interp = Interp::new();
        interp.bind_command_replacement(GLOBAL, b"p", Command::Builtin(worker));
        let namespace = interp
            .namespaces
            .borrow_mut()
            .ensure_namespace(GLOBAL, b"n");
        interp.set_current_ns(namespace);
        let original = head(b"p\0original");
        assert!(
            interp
                .resolve_original_command(original.as_ptr())
                .unwrap()
                .is_some()
        );
        let global_cache = obj::native_command_name_cache(original.as_ptr()).unwrap();
        assert_eq!(global_cache.namespace_token, GLOBAL as u64);
        interp.bind_command_replacement(namespace, b"p", Command::Builtin(worker));
        let state = interp
            .native_command_name_lookup_state(&global_cache)
            .unwrap();
        let protocol = interp
            .native_invocation_dialect()
            .native_command_name_protocol()
            .unwrap();
        assert!(!protocol.cache_is_current(&global_cache, &state));
        interp
            .resolve_original_command(original.as_ptr())
            .unwrap()
            .unwrap();
        let local_cache = obj::native_command_name_cache(original.as_ptr()).unwrap();
        assert_eq!(local_cache.namespace_token, namespace as u64);
        assert_ne!(local_cache.token, global_cache.token);
        assert_eq!(obj::bytes_of(original.as_ptr()), b"p\0original");
        interp.set_current_ns(GLOBAL);
        interp
            .resolve_original_command(original.as_ptr())
            .unwrap()
            .unwrap();
        assert_eq!(
            obj::native_command_name_cache(original.as_ptr())
                .unwrap()
                .token,
            global_cache.token
        );
    }

    #[test]
    fn duplicate_objects_and_foreign_interpreters_do_not_share_hit_authority() {
        let mut first = Interp::new();
        let mut second = Interp::new();
        let original = head(b"set");
        first
            .resolve_original_command(original.as_ptr())
            .unwrap()
            .unwrap();
        let cache = obj::native_command_name_cache(original.as_ptr()).unwrap();
        let duplicate = obj::Owned::fresh(obj::duplicate(original.as_ptr()));
        assert_eq!(
            obj::native_command_name_cache(duplicate.as_ptr()),
            Some(cache.clone())
        );
        let protocol = second
            .native_invocation_dialect()
            .native_command_name_protocol()
            .unwrap();
        assert!(!protocol.cache_is_current(
            &cache,
            &second.native_command_name_lookup_state(&cache).unwrap()
        ));
        second
            .resolve_original_command(duplicate.as_ptr())
            .unwrap()
            .unwrap();
        assert_ne!(
            obj::native_command_name_cache(duplicate.as_ptr())
                .unwrap()
                .interpreter,
            cache.interpreter
        );
        assert_eq!(
            obj::native_command_name_cache(original.as_ptr()),
            Some(cache)
        );
    }

    #[test]
    fn hide_expose_and_rename_keep_node_identity_with_native_epoch_windows() {
        let mut interp = Interp::new();
        interp.bind_command_replacement(GLOBAL, b"p", Command::Builtin(worker));
        let original = head(b"p");
        interp
            .resolve_original_command(original.as_ptr())
            .unwrap()
            .unwrap();
        let initial = obj::native_command_name_cache(original.as_ptr()).unwrap();
        assert_eq!(
            interp.hide_command(b"p", b"hidden"),
            super::super::CommandVisibilityOutcome::Moved
        );
        assert!(
            interp
                .native_command_name_lookup_state(&initial)
                .unwrap()
                .target
                .is_none()
        );
        assert_eq!(
            interp.expose_command(b"hidden", b"p"),
            super::super::CommandVisibilityOutcome::Moved
        );
        let exposed = interp
            .native_command_name_lookup_state(&initial)
            .unwrap()
            .target
            .unwrap();
        assert_eq!(exposed.token, initial.token);
        assert_eq!(exposed.command_epoch, initial.command_epoch + 1);
        interp
            .resolve_original_command(original.as_ptr())
            .unwrap()
            .unwrap();
        let refreshed = obj::native_command_name_cache(original.as_ptr()).unwrap();
        let namespace = interp
            .namespaces
            .borrow_mut()
            .ensure_namespace(GLOBAL, b"n");
        let publication = interp
            .namespaces
            .borrow_mut()
            .publish_rename_destination(GLOBAL, b"p", b"::n::p")
            .unwrap();
        let during = interp.native_command_name_lookup_state(&refreshed).unwrap();
        assert_eq!(
            during.target.as_ref().unwrap().namespace_token,
            namespace as u64
        );
        assert!(
            interp
                .native_invocation_dialect()
                .native_command_name_protocol()
                .unwrap()
                .cache_is_current(&refreshed, &during)
        );
        interp
            .namespaces
            .borrow_mut()
            .retire_rename_source(&publication);
        assert!(
            !interp
                .native_invocation_dialect()
                .native_command_name_protocol()
                .unwrap()
                .cache_is_current(
                    &refreshed,
                    &interp.native_command_name_lookup_state(&refreshed).unwrap()
                )
        );
    }

    #[test]
    fn failed_lookup_applies_release_selected_primary_without_erasing_c9_cache() {
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let mut interp = Interp::new();
            interp.set_runtime_version(version);
            interp.bind_command_replacement(GLOBAL, b"p", Command::Builtin(worker));
            let original = head(b"p");
            interp
                .resolve_original_command(original.as_ptr())
                .unwrap()
                .unwrap();
            let initial = obj::native_command_name_cache(original.as_ptr()).unwrap();
            interp.namespaces.borrow_mut().delete(GLOBAL, b"p");
            assert!(
                interp
                    .resolve_original_command(original.as_ptr())
                    .unwrap()
                    .is_none()
            );
            if version < TclVersion::V9_0 {
                assert_eq!(obj::native_command_name_cache(original.as_ptr()), None);
                assert_eq!(
                    obj::native_command_name_unresolved_version(original.as_ptr()),
                    Some(version)
                );
            } else {
                assert_eq!(
                    obj::native_command_name_cache(original.as_ptr()),
                    Some(initial)
                );
            }
            assert_eq!(obj::bytes_of(original.as_ptr()), b"p");
        }
    }

    #[test]
    fn command_name_receipt_does_not_keep_retired_procedure_worker_alive() {
        let mut interp = Interp::new();
        assert_eq!(interp.eval_str(b"proc p {} {return OK}"), Code::Ok);
        let Some(Command::Proc(procedure)) = interp.namespaces.borrow().resolve(GLOBAL, b"p")
        else {
            panic!("original procedure binding is missing");
        };
        let worker = std::rc::Rc::downgrade(&procedure.declaration());
        drop(procedure);
        let original = head(b"p");
        assert!(
            interp
                .resolve_original_command(original.as_ptr())
                .unwrap()
                .is_some()
        );
        let cache = obj::native_command_name_cache(original.as_ptr()).unwrap();
        assert!(interp.namespaces.borrow_mut().delete(GLOBAL, b"p"));
        assert!(worker.upgrade().is_none());
        assert_eq!(
            obj::native_command_name_cache(original.as_ptr()),
            Some(cache.clone())
        );
        assert!(
            interp
                .native_command_name_lookup_state(&cache)
                .unwrap()
                .target
                .is_none()
        );
    }
}
