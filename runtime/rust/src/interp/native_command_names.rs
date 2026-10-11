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
    /// Execute JimQualifyName on the actual supplied object. The namespace
    /// length test precedes the name getter, and the native temporary reference
    /// starts only after qualification has finished.
    pub(crate) fn original_jim_command_key(
        &mut self,
        origin: NsId,
        original: *mut TclObj,
        purpose: tcl_syntax::naming::NativeNamePurpose,
    ) -> Result<Option<(std::rc::Rc<obj::Owned>, Option<std::rc::Rc<[u8]>>)>, ValueError> {
        use tcl_syntax::naming::{
            NativeJimNamespaceConstruction, NativeNameContext, NativeNamePurpose,
        };
        let recipe = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original command key issuer",
            ))?
            .recipe();
        if !recipe.is_jim084() {
            return Ok(None);
        }
        if !matches!(
            purpose,
            NativeNamePurpose::CommandPublication
                | NativeNamePurpose::RenameSource
                | NativeNamePurpose::RenameDestination
        ) {
            return Err(ValueError::CommandProtocolUnavailable(
                "original qualified Jim key purpose",
            ));
        }
        let scope = super::native_operation_currency::NativeOperationScope::enter(self).map_err(
            |first| {
                self.refuse_native_execution(first);
                ValueError::CommandProtocolUnavailable("original Jim key entry")
            },
        )?;
        obj::check_native_liveness(original)?;
        let original_allocation = obj::NativeObjectLifetime::retain(original);
        let namespace = self
            .namespaces()
            .jim_namespace_original_owner(origin)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original Jim key namespace owner",
            ))?;
        let namespace_bytes = self.native_object_string_bytes(namespace.as_ptr());
        scope.currency().ensure_current_or_refuse().map_err(|_| {
            ValueError::CommandProtocolUnavailable("original Jim key namespace currency")
        })?;
        let namespace_bytes = namespace_bytes?;
        // A root frame reaches no String getter on the incoming name here.
        let bytes = if namespace_bytes.is_empty() {
            std::rc::Rc::<[u8]>::from([])
        } else {
            let bytes = self.native_object_string_bytes(original);
            scope.currency().ensure_current_or_refuse().map_err(|_| {
                ValueError::CommandProtocolUnavailable("original Jim key getter currency")
            })?;
            bytes?
        };
        let path = self.namespaces().native_context_path(origin).ok_or(
            ValueError::CommandProtocolUnavailable("original Jim key namespace path"),
        )?;
        let context = NativeNameContext::with_jim_namespace(&path, &namespace_bytes);
        let construction = recipe
            .jim_command_key_construction(context, &bytes, purpose)
            .map_err(|_| {
                ValueError::CommandProtocolUnavailable("original Jim key construction purpose")
            })?;
        let result = match construction {
            NativeJimNamespaceConstruction::RetainOriginal => {
                obj::check_native_liveness(original)?;
                std::rc::Rc::new(obj::Owned::retain(original))
            }
            NativeJimNamespaceConstruction::DuplicateNamespaceAndAppend => {
                // JimQualifyName increments only after both CString appends.
                // This lease protects allocation without changing sharing.
                let duplicate = obj::duplicate(namespace.as_ptr());
                let allocation = obj::NativeObjectLifetime::retain(duplicate);
                let prepared = (|| {
                    let dialect = self.native_invocation_dialect();
                    let append = dialect.native_object_append_protocol(None).ok_or(
                        ValueError::CommandProtocolUnavailable("Jim original key append issuer"),
                    )?;
                    let operations = crate::value_ops::RuntimeAppendObjects {
                        dialect,
                        binary_recipe: dialect.byte_array_string_recipe(None),
                    };
                    let receiver = crate::value_ops::RuntimeAppendValue::borrowed(duplicate);
                    let mut check = || {
                        scope.currency().ensure_current_or_refuse().map_err(|_| {
                            ValueError::CommandProtocolUnavailable(
                                "original Jim key append currency",
                            )
                        })?;
                        obj::check_native_liveness(duplicate)
                    };
                    check()?;
                    for part in [b"::".as_slice(), tcl_core_types::c_string_extent(&bytes)] {
                        tcl_cmd_core::native_append::append_unshared_jim_cstring(
                            &operations,
                            append.recipe(),
                            &receiver,
                            part,
                            &mut check,
                        )?;
                    }
                    check()?;
                    crate::native_source::bind_context(
                        duplicate,
                        &self.native_jim_object_context()?,
                    )?;
                    check()?;
                    Ok::<_, ValueError>(())
                })();
                if let Err(error) = prepared {
                    // The failed constructor owns a live rc-zero header.
                    // Retire it without introducing a native +1 role.
                    if obj::allocation_is_live(duplicate) {
                        unsafe { obj::free_obj(duplicate) };
                    }
                    drop(allocation);
                    scope.currency().ensure_current_or_refuse().map_err(|_| {
                        ValueError::CommandProtocolUnavailable("original Jim key cleanup currency")
                    })?;
                    return Err(error);
                }
                let created = obj::Owned::fresh(duplicate);
                drop(allocation);
                std::rc::Rc::new(created)
            }
            NativeJimNamespaceConstruction::FreshString => {
                return Err(ValueError::CommandProtocolUnavailable(
                    "qualified Jim key cannot use RegisterCmd",
                ));
            }
        };
        scope.currency().ensure_current_or_refuse().map_err(|_| {
            ValueError::CommandProtocolUnavailable("original Jim key producer currency")
        })?;
        drop(original_allocation);
        Ok(Some((
            result,
            (!namespace_bytes.is_empty()).then_some(bytes),
        )))
    }

    /// Genuine byte-name APIs construct their own incoming object. The child
    /// RegisterCmd API uses a fresh CString key; authored aliases pass their
    /// already retained original object directly to the table owner.
    pub(crate) fn generated_jim_command_key(
        &mut self,
        origin: NsId,
        bytes: &[u8],
        purpose: tcl_syntax::naming::NativeNamePurpose,
    ) -> Result<Option<std::rc::Rc<obj::Owned>>, ValueError> {
        use tcl_syntax::naming::{
            NativeJimNamespaceConstruction, NativeNameContext, NativeNamePurpose,
        };
        let recipe = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "generated command key issuer",
            ))?
            .recipe();
        if !recipe.is_jim084() {
            return Ok(None);
        }
        let created = obj::Owned::fresh(obj::new_string_bytes(tcl_core_types::c_string_extent(
            bytes,
        )));
        crate::native_source::bind_context(created.as_ptr(), &self.native_jim_object_context()?)?;
        match purpose {
            // Qualification always consumes the actual namespace holder. A
            // byte API's fresh operand cannot manufacture a root-frame purpose.
            NativeNamePurpose::CommandPublication
            | NativeNamePurpose::RenameSource
            | NativeNamePurpose::RenameDestination => self
                .original_jim_command_key(origin, created.as_ptr(), purpose)
                .map(|prepared| prepared.map(|(key, _)| key)),
            _ => {
                match recipe.jim_command_key_construction(NativeNameContext::root(), bytes, purpose)
                {
                    Ok(
                        NativeJimNamespaceConstruction::FreshString
                        | NativeJimNamespaceConstruction::RetainOriginal,
                    ) => Ok(Some(std::rc::Rc::new(created))),
                    _ => Err(ValueError::CommandProtocolUnavailable(
                        "generated Jim key constructor purpose",
                    )),
                }
            }
        }
    }

    /// Retain the original object's actual selected command token before any
    /// name or imported-origin reporting. No reporting query redoes lookup.
    fn native_selected_original_command_token(
        &mut self,
        original: *mut TclObj,
    ) -> Result<Option<tcl_runtime_api::CommandId>, ValueError> {
        let Some((command, token)) = self.resolve_original_command(original)? else {
            return Ok(None);
        };
        drop(command);
        let token = token.ok_or(ValueError::CommandProtocolUnavailable(
            "original selected command token",
        ))?;
        let (name, _) = self.raw_command_location_by_generation(token).ok_or(
            ValueError::CommandProtocolUnavailable("original selected command placement"),
        )?;
        Ok(Some(tcl_runtime_api::CommandId(
            self.intern_cmd(&name, token),
        )))
    }

    /// Report the selected original command's current name, preserving an
    /// import's own token rather than following its origin.
    pub(crate) fn native_resolved_command_name(
        &mut self,
        original: *mut TclObj,
    ) -> Result<Option<Vec<u8>>, ValueError> {
        self.native_selected_original_command_token(original)?
            .map(|command| {
                tcl_cmd_core::namespace::command_name_from_command_checked(self, command)
            })
            .transpose()
    }

    /// Report the selected original command's imported origin without a second
    /// lookup or reconstruction of the original operand.
    pub(crate) fn native_namespace_origin(
        &mut self,
        original: *mut TclObj,
    ) -> Result<Option<Vec<u8>>, ValueError> {
        self.native_selected_original_command_token(original)?
            .map(|command| tcl_cmd_core::namespace::origin_from_command_checked(self, command))
            .transpose()
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

    /// Validate the already selected node and reference; never look its name up again.
    pub(super) fn original_command_cache_current(
        &self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
        original: *mut TclObj,
        cache: &NativeCommandNameCache,
        origin: NsId,
    ) -> Result<(), ValueError> {
        currency.ensure_current_or_refuse().map_err(|_| {
            ValueError::CommandProtocolUnavailable("original command cache currency")
        })?;
        obj::check_native_liveness(original)?;
        let protocol = self
            .native_invocation_dialect()
            .native_command_name_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original command cache purpose",
            ))?;
        if !protocol.cache_is_current(
            cache,
            &self.native_command_name_lookup_state_at(cache, origin)?,
        ) {
            return Err(ValueError::CommandProtocolUnavailable(
                "captured original command binding",
            ));
        }
        Ok(())
    }

    /// Resolve the original object, checking native cache state before lookup.
    /// Failed lookup retains the C9 primary or applies C8's null descriptor.
    pub(crate) fn resolve_original_command(
        &mut self,
        original: *mut TclObj,
    ) -> Result<Option<(Command, Option<u64>)>, ValueError> {
        self.resolve_original_command_at(self.current_ns.get(), original)
    }

    /// Resolve an original native operand to the actual selected command generation.
    pub(crate) fn resolve_original_command_generation(
        &mut self,
        original: *mut TclObj,
    ) -> Result<Option<u64>, ValueError> {
        let Some((command, generation)) = self.resolve_original_command(original)? else {
            return Ok(None);
        };
        drop(command);
        generation
            .map(Some)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original native command generation",
            ))
    }

    /// Resolve an original object in a separately retained lookup namespace,
    /// without changing the caller's variable frame or namespace.
    pub(crate) fn resolve_original_command_at(
        &mut self,
        current: NsId,
        original: *mut TclObj,
    ) -> Result<Option<(Command, Option<u64>)>, ValueError> {
        let scope = super::native_operation_currency::NativeOperationScope::enter(self).map_err(
            |first| {
                self.refuse_native_execution(first);
                ValueError::CommandProtocolUnavailable("original command lookup entry")
            },
        )?;
        obj::check_native_liveness(original)?;
        let _allocation = obj::NativeObjectLifetime::retain(original);
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
            let check =
                || self.original_command_cache_current(scope.currency(), original, &cache, origin);
            let reached = obj::install_native_command_name_cache_with_stage_check(
                original,
                cache.clone(),
                dialect,
                Some(&check),
            );
            check()?;
            reached?;
            // Cache preparation observes a token; actual worker ownership starts
            // only after the reached old representation has retired.
            let node = self.namespaces.borrow().native_command_at_node(cache.token);
            if let Some((command, fqn)) = node {
                return Ok(self
                    .command_visible_for_surface_at(&command, &fqn, Some(cache.token))
                    .then_some((command, Some(cache.token))));
            }
        } else {
            let reference = self.namespaces.borrow().native_command_reference(origin);
            let check = || {
                scope.currency().ensure_current_or_refuse().map_err(|_| {
                    ValueError::CommandProtocolUnavailable("original unresolved command currency")
                })?;
                obj::check_native_liveness(original)?;
                if self.namespaces.borrow().native_command_reference(origin) != reference {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "original unresolved command reference",
                    ));
                }
                Ok(())
            };
            let reached = obj::install_unresolved_native_command_name_cache_with_stage_check(
                original,
                dialect,
                Some(&check),
            );
            check()?;
            reached?;
        }
        Ok(None)
    }
}

#[cfg(test)]
mod current_cache_tests;

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
        // Native proof naming.namespace.origin-generated-opaque-failure-units:
        // docs/design/analysis/name-resolution-proofs/namespace-origin-generated-opaque-failure-units.md
        let source = include_bytes!("../../tests/data/native_namespace_origin_failures/source.tcl");
        let rows = include_str!("../../tests/data/native_namespace_origin_failures/controls.tsv");
        for (version, row) in TclVersion::ALL.into_iter().zip(rows.lines()) {
            let mut interp = native_interpreter(version);
            // The retained observer measures binary-scan units after the catch,
            // independently of the result's native modified-UTF-8 string bytes.
            let mut observer = b"set observed [".to_vec();
            observer.extend_from_slice(source.trim_ascii());
            observer.extend_from_slice(b"]\nbinary scan $observed H* hex\nset hex");
            assert_eq!(interp.eval_str(&observer), Code::Ok, "{version:?}");
            let expected = row.split_once('\t').unwrap().1;
            assert_eq!(interp.result_bytes(), expected.as_bytes(), "{version:?}");
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

#[cfg(test)]
mod original_key_construction_tests {
    use super::*;
    use crate::interp::native_operation_currency::NativeOperationScope;
    use std::cell::{Cell, RefCell};
    use tcl_syntax::naming::NativeNamePurpose;

    thread_local! {
        static OWNER: RefCell<Option<Interp>> = const { RefCell::new(None) };
        static MODE: Cell<u8> = const { Cell::new(0) };
        static REFS: Cell<i64> = const { Cell::new(-1) };
    }
    extern "C" fn update(original: *mut TclObj) {
        REFS.with(|refs| refs.set(unsafe { (*original).ref_count } as i64));
        let mut interp = OWNER.with(|owner| owner.borrow().as_ref().unwrap().clone());
        let effect = obj::Owned::fresh(obj::new_string_bytes(b"REACHED\0\xff"));
        interp
            .var_set(b"original_key_getter_effect", effect.as_ptr())
            .unwrap();
        match MODE.with(Cell::get) {
            1 => {
                interp.refuse_host_command("original qualification name getter");
            }
            2 => {
                let original = interp.runtime_context();
                let mut changed = original.clone();
                changed.packages = vec![("original-key-construction".to_owned(), "1.0".to_owned())];
                interp.pin_context(&changed).unwrap();
                interp.pin_context(&original).unwrap();
            }
            _ => {}
        }
        unsafe { obj::set_native_updater_string_rep(original, b"child\0ignored", false) };
    }
    static TYPE: obj::TclObjType = obj::TclObjType {
        name: c"originalJimQualifiedName".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(update),
        set_from_any_proc: None,
    };

    #[test]
    fn original_jim_qualification_keeps_namespace_owner_and_stops_after_refused_getter() {
        // naming.alias.original-command-key-replacement-child-capture-and-rename
        // docs/design/analysis/name-resolution-proofs/alias-original-command-key-replacement-child-capture-and-rename.md
        // Qualified source/first-cause controls; Native421b observes root keys.
        for mode in [0, 1, 2] {
            let mut interp = crate::cmd_alias::original_operand_tests::native("jim");
            let _cleanup = crate::cmd_namespace::original_argument_tests::FixtureCleanup(|| {
                OWNER.with(|owner| owner.borrow_mut().take());
            });
            let captured = crate::cmd_namespace::original_argument_tests::jim_fixture_namespace(
                &mut interp,
                b"key_scope",
            );
            let namespace = captured.token;
            let namespace_owner = std::rc::Rc::clone(&captured.owner);
            let namespace_refs = unsafe { (*namespace_owner.as_ptr()).ref_count };
            let original = obj::Owned::fresh(obj::alloc_typed(&TYPE, 0));
            OWNER.with(|owner| *owner.borrow_mut() = Some(interp.clone()));
            MODE.with(|selected| selected.set(mode));
            REFS.with(|refs| refs.set(-1));
            let prior = crate::cmd_alias::original_operand_tests::seed(&mut interp);
            let before = interp.namespaces().native_command_generations();
            let result = interp.original_jim_command_key(
                namespace,
                original.as_ptr(),
                NativeNamePurpose::CommandPublication,
            );
            assert_eq!(
                REFS.with(Cell::get),
                1,
                "qualification reads before any original-name native retain"
            );
            assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
            assert_eq!(
                unsafe { (*namespace_owner.as_ptr()).ref_count },
                namespace_refs
            );
            assert_eq!(interp.namespaces().native_command_generations(), before);
            assert_eq!(interp.get_obj_result(), prior.as_ptr());
            if mode == 0 {
                let (key, source) = result.unwrap().unwrap();
                assert_ne!(key.as_ptr(), original.as_ptr());
                assert_eq!(unsafe { (*key.as_ptr()).ref_count }, 1);
                assert_eq!(source.unwrap().as_ref(), b"child\0ignored");
                let scope = NativeOperationScope::enter(&interp).unwrap();
                assert_eq!(
                    interp
                        .native_object_string_bytes(key.as_ptr())
                        .unwrap()
                        .as_ref(),
                    b"key_scope::child"
                );
                assert!(scope.currency().ensure_current_or_refuse().is_ok());
                let generated = interp
                    .generated_jim_command_key(
                        namespace,
                        b"generated_child\0ignored",
                        NativeNamePurpose::CommandPublication,
                    )
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    obj::bytes_of(generated.as_ptr()),
                    b"key_scope::generated_child"
                );
                assert_eq!(unsafe { (*generated.as_ptr()).ref_count }, 1);
                assert!(scope.currency().ensure_current_or_refuse().is_ok());
            } else {
                assert!(result.is_err());
                assert!(interp.native_execution_refusal().is_some());
                assert_eq!(obj::bytes_of(original.as_ptr()), b"child\0ignored");
            }
            assert_eq!(
                obj::bytes_of(
                    interp
                        .namespaces()
                        .var_table(crate::namespace::GLOBAL)
                        .load_scalar(b"original_key_getter_effect")
                        .unwrap(),
                ),
                b"REACHED\0\xff"
            );
            OWNER.with(|owner| owner.borrow_mut().take());
        }
    }

    #[test]
    fn original_jim_key_append_keeps_the_actual_zero_reference_receiver() {
        // naming.alias.original-command-key-replacement-child-capture-and-rename
        // docs/design/analysis/name-resolution-proofs/alias-original-command-key-replacement-child-capture-and-rename.md
        let interp = crate::cmd_alias::original_operand_tests::native("jim");
        let scope = NativeOperationScope::enter(&interp).unwrap();
        let raw = obj::new_string_bytes(b"key_scope");
        let allocation = obj::NativeObjectLifetime::retain(raw);
        crate::native_source::bind_context(raw, &interp.native_jim_object_context().unwrap())
            .unwrap();
        let dialect = interp.native_invocation_dialect();
        let ops = crate::value_ops::RuntimeAppendObjects {
            dialect,
            binary_recipe: dialect.byte_array_string_recipe(None),
        };
        let receiver = crate::value_ops::RuntimeAppendValue::borrowed(raw);
        let recipe = dialect
            .native_object_append_protocol(None)
            .unwrap()
            .recipe();
        let mut checks = 0;
        let mut check = || {
            checks += 1;
            assert_eq!(unsafe { (*raw).ref_count }, 0);
            scope.currency().ensure_current_or_refuse().map_err(|_| {
                ValueError::CommandProtocolUnavailable("original append test currency")
            })
        };
        for part in [b"::".as_slice(), b"child\0ignored"] {
            tcl_cmd_core::native_append::append_unshared_jim_cstring(
                &ops, recipe, &receiver, part, &mut check,
            )
            .unwrap();
        }
        assert!(checks > 2);
        assert_eq!(obj::bytes_of(raw), b"key_scope::child");
        assert_eq!(unsafe { (*raw).ref_count }, 0);
        // Retire the real constructor receiver directly; a lease is not a
        // protective native reference or a manufactured key/lookup receipt.
        unsafe { obj::free_obj(raw) };
        drop(allocation);
    }

    #[test]
    fn original_jim_missing_rename_source_stops_before_destination_lookup() {
        // naming.alias.original-command-key-replacement-child-capture-and-rename
        // docs/design/analysis/name-resolution-proofs/alias-original-command-key-replacement-child-capture-and-rename.md
        let mut interp = crate::cmd_alias::original_operand_tests::native("jim");
        let original = obj::Owned::fresh(obj::alloc_typed(&TYPE, 0));
        let source = obj::Owned::fresh(obj::new_string_bytes(b"missing_original_key"));
        OWNER.with(|owner| *owner.borrow_mut() = Some(interp.clone()));
        MODE.with(|mode| mode.set(0));
        let scope = NativeOperationScope::enter(&interp).unwrap();
        let (old, new, old_key, new_key) = interp
            .original_jim_rename_operands(source.as_ptr(), original.as_ptr())
            .unwrap();
        assert_eq!(obj::bytes_of(original.as_ptr()), b"child\0ignored");
        // The destination length getter was reached once before qualification.
        // A missing exact source cannot publish or borrow a fallback binding.
        let before = interp.namespaces().native_command_generations();
        assert_eq!(
            interp.rename_command_with_original_jim_key(&old, &new, new_key),
            crate::interp::RenameOutcome::NoSuchCommand
        );
        assert_eq!(interp.namespaces().native_command_generations(), before);
        assert_eq!(old_key.as_ptr(), source.as_ptr());
        assert_eq!(unsafe { (*old_key.as_ptr()).ref_count }, 2);
        assert!(scope.currency().ensure_current_or_refuse().is_ok());
        OWNER.with(|owner| owner.borrow_mut().take());
        // The source hash lookup is exact in the current Jim namespace. A
        // global command of the same spelling cannot close a missing source.
        assert_eq!(
            interp.eval_str(b"namespace eval key_scope {rename list missing_original_key}"),
            crate::interp::Code::Error,
        );
        assert!(interp
            .namespaces()
            .command_in(crate::namespace::GLOBAL, b"list")
            .is_some());
        assert!(interp
            .namespaces()
            .command_in(crate::namespace::GLOBAL, b"key_scope::missing_original_key")
            .is_none());
        assert!(interp.native_execution_refusal().is_none());
    }
}
