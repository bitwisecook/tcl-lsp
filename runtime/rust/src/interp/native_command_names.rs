// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original command-object lookup and metadata-only cache validation.

use super::{Command, Interp};
use crate::{
    namespace::NsId,
    obj::{self, TclObj},
};
use tcl_runtime_api::native_command_name::{NativeCommandNameCache, NativeCommandNameLookupState};
use tcl_syntax::value::{ValueError, ValueOps};

type OriginalJimCommandKey = (std::rc::Rc<obj::Owned>, Option<std::rc::Rc<[u8]>>);
type OriginalCommandCheck<'a> = &'a dyn Fn(&Interp) -> Result<(), ValueError>;

#[derive(Clone)]
pub(super) struct OriginalByteCommandRegistration {
    original: Vec<u8>,
    namespace: NsId,
    tail: Vec<u8>,
}

impl Interp {
    pub(super) fn prune_original_byte_command_registrations(&self) {
        #[cfg(test)]
        let _diagnostic = super::constructor_diagnostics::WorkMeasurement::new(
            super::constructor_diagnostics::Work::Prune,
        );
        let world = self.namespaces();
        let revision = world.original_publication_revision();
        if revision.as_ref().is_some_and(|current| {
            self.original_byte_command_registration_revision
                .borrow()
                .as_ref()
                .is_some_and(|previous| previous.matches(current))
        }) {
            return;
        }
        // Keep the same retain walk; only this token needs a fresh slot lookup.
        let changed_generation = revision.as_ref().and_then(|current| {
            self.original_byte_command_registration_revision
                .borrow()
                .as_ref()
                .and_then(|previous| current.single_unlink_since(previous))
        });
        self.original_byte_command_registrations
            .borrow_mut()
            .retain(|&generation, registration| {
                changed_generation.is_some_and(|changed| changed != generation)
                    || world.original_publication_is_current(
                        registration.namespace,
                        &registration.tail,
                        generation,
                    )
            });
        *self
            .original_byte_command_registration_revision
            .borrow_mut() = revision;
    }

    pub(super) fn remember_original_byte_command_registration(
        &self,
        original: &[u8],
        namespace: NsId,
        tail: &[u8],
        generation: u64,
    ) {
        self.prune_original_byte_command_registrations();
        if self
            .namespaces()
            .original_publication_is_current(namespace, tail, generation)
        {
            self.original_byte_command_registrations
                .borrow_mut()
                .insert(
                    generation,
                    OriginalByteCommandRegistration {
                        original: original.to_vec(),
                        namespace,
                        tail: tail.to_vec(),
                    },
                );
        }
    }

    pub(super) fn publish_original_jim_registration_keys(&mut self) -> Result<(), ValueError> {
        use tcl_registry::native_command_creation::NativeObjectCommandCreationPurpose;
        use tcl_syntax::naming::NativeNamePurpose;
        if !self
            .name_policy_protocol()
            .is_some_and(|policy| policy.recipe().is_jim084())
        {
            return Ok(());
        }
        let protocol = self
            .native_invocation_dialect()
            .native_command_creation_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original Jim registration constructor",
            ))?;
        let scope = super::native_operation_currency::NativeOperationScope::enter(self).map_err(
            |first| {
                self.refuse_native_execution(first);
                ValueError::CommandProtocolUnavailable("original Jim registration entry")
            },
        )?;
        self.prune_original_byte_command_registrations();
        let registrations: Vec<_> = self
            .original_byte_command_registrations
            .borrow()
            .iter()
            .map(|(&generation, registration)| (generation, registration.clone()))
            .collect();
        for (generation, registration) in registrations {
            if registration.namespace != crate::namespace::GLOBAL
                || !self.namespaces().original_publication_is_current(
                    registration.namespace,
                    &registration.tail,
                    generation,
                )
                || self.namespaces().has_original_jim_registration_key(
                    registration.namespace,
                    &registration.tail,
                    generation,
                )
            {
                continue;
            }
            let Some(report) = protocol.public_jim_registration_key(
                NativeObjectCommandCreationPurpose::LegacyPublicObjectCommand,
                &registration.original,
            ) else {
                continue;
            };
            if report.comparison_bytes() != registration.tail {
                continue;
            }
            let key = self
                .generated_jim_command_key(
                    registration.namespace,
                    &registration.original,
                    NativeNamePurpose::CommandCApiPublication,
                )?
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original Jim registration key",
                ))?;
            scope.currency().ensure_current_or_refuse().map_err(|_| {
                ValueError::CommandProtocolUnavailable("original Jim registration currency")
            })?;
            // The hash owns keyDup; the actual constructor temporary retires outside the table borrow.
            let published = self.namespaces_mut().publish_original_jim_registration_key(
                registration.namespace,
                &registration.tail,
                generation,
                report,
                &key,
            );
            drop(key);
            if !published {
                return Err(ValueError::CommandProtocolUnavailable(
                    "original Jim registration slot",
                ));
            }
            scope.currency().ensure_current_or_refuse().map_err(|_| {
                ValueError::CommandProtocolUnavailable("original Jim registration retirement")
            })?;
        }
        Ok(())
    }
    /// Execute JimQualifyName on the actual supplied object. The namespace
    /// length test precedes the name getter, and the native temporary reference
    /// starts only after qualification has finished.
    pub(crate) fn original_jim_command_key(
        &mut self,
        origin: NsId,
        original: *mut TclObj,
        purpose: tcl_syntax::naming::NativeNamePurpose,
    ) -> Result<Option<OriginalJimCommandKey>, ValueError> {
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
        let namespace_scope =
            super::native_jim_lookup::OriginalJimLookupNamespace::capture(self, origin)?;
        let namespace_bytes = self.materialize_jim_namespace(origin);
        namespace_scope.check(self)?;
        scope.currency().ensure_current_or_refuse().map_err(|_| {
            ValueError::CommandProtocolUnavailable("original Jim key namespace currency")
        })?;
        let namespace_bytes = namespace_bytes?;
        // A root frame reaches no String getter on the incoming name here.
        let bytes = if namespace_bytes.is_empty() {
            std::rc::Rc::<[u8]>::from([])
        } else {
            let bytes = self.native_object_string_bytes(original);
            namespace_scope.check(self)?;
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
                let duplicate = obj::duplicate(namespace_scope.original_ptr());
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
                        namespace_scope.check(self)?;
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
                    namespace_scope.check(self)?;
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
        namespace_scope.check(self)?;
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
        let scope =
            super::native_operation_currency::NativeOperationScope::enter(self).map_err(|_| {
                ValueError::CommandProtocolUnavailable("original Jim byte API allocator")
            })?;
        let created = obj::Owned::fresh(obj::new_string_bytes(tcl_core_types::c_string_extent(
            bytes,
        )));
        scope.currency().ensure_current_or_refuse().map_err(|_| {
            ValueError::CommandProtocolUnavailable("original Jim byte API constructor currency")
        })?;
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

    /// Namespace command queries retain their independent Jim helper purpose.
    pub(crate) fn native_namespace_which_command_name(
        &mut self,
        original: *mut TclObj,
    ) -> Result<Option<Vec<u8>>, ValueError> {
        let policy = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "namespace command query protocol",
            ))?;
        if !policy.recipe().is_jim084() {
            return self.native_resolved_command_name(original);
        }
        let bytes = self.native_object_string_bytes(original)?;
        self.native_namespace_which_command_bytes(&bytes)
    }

    pub(crate) fn native_namespace_which_command_bytes(
        &mut self,
        original: &[u8],
    ) -> Result<Option<Vec<u8>>, ValueError> {
        use tcl_syntax::naming::NativeNameContext;
        let protocol = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "namespace command query protocol",
            ))?
            .recipe();
        if !protocol.is_jim084() {
            return tcl_cmd_core::namespace::which_command_bytes_checked(self, original);
        }
        let scope = super::native_operation_currency::NativeOperationScope::enter(self).map_err(
            |first| {
                self.refuse_native_execution(first);
                ValueError::CommandProtocolUnavailable("Jim namespace command query entry")
            },
        )?;
        let current = self.current_ns();
        let namespace_scope =
            super::native_jim_lookup::OriginalJimLookupNamespace::capture(self, current)?;
        let namespace_bytes = self.materialize_jim_namespace(current);
        namespace_scope.check(self)?;
        scope.currency().ensure_current_or_refuse().map_err(|_| {
            ValueError::CommandProtocolUnavailable("Jim namespace command query currency")
        })?;
        let namespace_bytes = namespace_bytes?;
        let path = tcl_core_types::ByteNamespacePath::root();
        let candidates = protocol
            .jim_namespace_command_query_candidates(
                NativeNameContext::with_jim_namespace(&path, &namespace_bytes),
                original,
            )
            .map_err(|_| {
                ValueError::CommandProtocolUnavailable("Jim namespace command query purpose")
            })?;
        let object_context = self.native_jim_object_context()?;
        for candidate in candidates {
            let input = obj::Owned::fresh(obj::new_string_bytes(candidate.lookup.as_bytes()));
            crate::native_source::bind_context(input.as_ptr(), &object_context)?;
            let found = self
                .resolve_original_command_at(crate::namespace::GLOBAL, input.as_ptr())?
                .is_some();
            scope.currency().ensure_current_or_refuse().map_err(|_| {
                ValueError::CommandProtocolUnavailable(
                    "Jim namespace command query result currency",
                )
            })?;
            if found {
                return Ok(Some(candidate.reported.as_bytes().to_vec()));
            }
        }
        Ok(None)
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
    pub(super) fn original_command_lookup_binding_current(
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
        if !protocol.selected_binding_is_current(
            cache,
            &self.native_command_name_lookup_state_at(cache, origin)?,
        ) {
            return Err(ValueError::CommandProtocolUnavailable(
                "captured fresh original command binding",
            ));
        }
        Ok(())
    }

    /// Project an already selected imported token through the existing origin
    /// traversal. No command-name lookup or String production participates.
    pub(crate) fn original_import_origin_command(&self, generation: u64) -> Option<Command> {
        let (name, command) = self.raw_command_location_by_generation(generation)?;
        self.resolve_import_query(
            (name, generation, command),
            super::ImportQueryPurpose::OriginalObject,
        )
        .map(|target| target.command)
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
        self.resolve_original_command_at_using(current, original, None)
    }

    fn original_command_stage_refusal(&self, error: ValueError) -> ValueError {
        let typed = tcl_cmd_core::CmdError::from(error.clone());
        if let Some(first) = typed.native_execution_refusal() {
            self.clone().refuse_native_execution(first.clone());
        }
        error
    }

    fn original_command_lookup_bytes_using(
        &self,
        original: *mut TclObj,
        extra: Option<OriginalCommandCheck<'_>>,
    ) -> Result<std::rc::Rc<[u8]>, ValueError> {
        let Some(extra) = extra else {
            return self.native_object_string_bytes(original);
        };
        let reached = self.native_object_string_bytes_using(
            original,
            &|| extra(self).map_err(|error| self.original_command_stage_refusal(error)),
            &|error| self.original_command_stage_refusal(error),
        );
        extra(self)?;
        reached
    }

    pub(crate) fn resolve_original_command_at_using(
        &mut self,
        current: NsId,
        original: *mut TclObj,
        extra: Option<OriginalCommandCheck<'_>>,
    ) -> Result<Option<(Command, Option<u64>)>, ValueError> {
        let scope = super::native_operation_currency::NativeOperationScope::enter(self).map_err(
            |first| {
                self.refuse_native_execution(first);
                ValueError::CommandProtocolUnavailable("original command lookup entry")
            },
        )?;
        if let Some(check) = extra {
            check(self)?;
        }
        obj::check_native_liveness(original)?;
        let _allocation = obj::NativeObjectLifetime::retain(original);
        let dialect = self.native_invocation_dialect();
        let Some(protocol) = dialect.native_command_name_protocol() else {
            if dialect.native_jim_lookup_protocol().is_some() {
                return self.resolve_original_jim_command_using(current, original, extra);
            }
            let bytes = self.original_command_lookup_bytes_using(original, extra)?;
            return Ok(self.resolve_dispatchable_with_generation(current, &bytes));
        };
        if protocol.version() >= tcl_dialect::TclVersion::V8_5 {
            if let Some(cache) = obj::native_command_name_cache(original) {
                if protocol.cache_is_current(
                    &cache,
                    &self.native_command_name_lookup_state_at(&cache, current)?,
                ) {
                    if let Some((command, _)) =
                        self.namespaces.borrow().native_command_at_node(cache.token)
                    {
                        return Ok(self
                            .command_visible_for_surface_at(&command, Some(cache.token))
                            .then_some((command, Some(cache.token))));
                    }
                }
            }
        }
        let bytes = self.original_command_lookup_bytes_using(original, extra)?;
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
                if let Some((command, _)) = binding {
                    return Ok(self
                        .command_visible_for_surface_at(&command, Some(cache.token))
                        .then_some((command, Some(cache.token))));
                }
            }
        }
        let binding = self.native_command_name_from_binding(origin, &bytes)?;
        if let Some(cache) = binding {
            let check = || {
                if let Some(check) = extra {
                    check(self)?;
                }
                self.original_command_lookup_binding_current(
                    scope.currency(),
                    original,
                    &cache,
                    origin,
                )
                .map_err(|error| {
                    if extra.is_some() {
                        self.original_command_stage_refusal(error)
                    } else {
                        error
                    }
                })
            };
            let reached = obj::install_native_command_name_cache_with_stage_check(
                original,
                cache.clone(),
                dialect,
                Some(&check),
            );
            if extra.is_some() {
                if let Err(error) = &reached {
                    self.original_command_stage_refusal(error.clone());
                }
            }
            check()?;
            reached?;
            // Cache preparation observes a token; actual worker ownership starts
            // only after the reached old representation has retired.
            let node = self.namespaces.borrow().native_command_at_node(cache.token);
            if let Some((command, _)) = node {
                return Ok(self
                    .command_visible_for_surface_at(&command, Some(cache.token))
                    .then_some((command, Some(cache.token))));
            }
        } else {
            let reference = self.namespaces.borrow().native_command_reference(origin);
            let check = || {
                if let Some(check) = extra {
                    check(self)?;
                }
                scope.currency().ensure_current_or_refuse().map_err(|_| {
                    ValueError::CommandProtocolUnavailable("original unresolved command currency")
                })?;
                obj::check_native_liveness(original).map_err(|error| {
                    if extra.is_some() {
                        self.original_command_stage_refusal(error)
                    } else {
                        error
                    }
                })?;
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
            if extra.is_some() {
                if let Err(error) = &reached {
                    self.original_command_stage_refusal(error.clone());
                }
            }
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
        assert!(interp
            .native_namespace_origin_result(b"::selected")
            .is_err());
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
        assert!(interp
            .resolve_original_command(original.as_ptr())
            .unwrap()
            .is_some());
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
        assert!(interp
            .native_command_name_lookup_state(&initial)
            .unwrap()
            .target
            .is_none());
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
        assert!(interp
            .native_invocation_dialect()
            .native_command_name_protocol()
            .unwrap()
            .cache_is_current(&refreshed, &during));
        interp
            .namespaces
            .borrow_mut()
            .retire_rename_source(&publication);
        assert!(!interp
            .native_invocation_dialect()
            .native_command_name_protocol()
            .unwrap()
            .cache_is_current(
                &refreshed,
                &interp.native_command_name_lookup_state(&refreshed).unwrap()
            ));
    }

    #[test]
    fn original_c_empty_exposure_preserves_the_actual_token_and_global_slot() {
        for version in TclVersion::ALL {
            let mut interp = Interp::new();
            interp.set_runtime_version(version);
            interp.bind_command_replacement(
                crate::namespace::GLOBAL,
                b"kept",
                Command::Builtin(worker),
            );
            let generation = interp
                .namespaces
                .borrow()
                .command_generation(crate::namespace::GLOBAL, b"kept")
                .unwrap();
            assert_eq!(
                interp.hide_command(b"kept", b"hidden"),
                super::super::CommandVisibilityOutcome::Moved
            );
            assert_eq!(
                interp.expose_command(b"hidden", b""),
                super::super::CommandVisibilityOutcome::Moved
            );
            assert_eq!(
                interp
                    .namespaces
                    .borrow()
                    .command_generation(crate::namespace::GLOBAL, b""),
                Some(generation)
            );
            assert_eq!(interp.eval_str(b"{}"), crate::interp::Code::Ok);
            assert!(interp.hidden_names().is_empty());
            assert!(interp.host_command_refusal().is_none());
        }
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
            assert!(interp
                .resolve_original_command(original.as_ptr())
                .unwrap()
                .is_none());
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
        assert!(interp
            .resolve_original_command(original.as_ptr())
            .unwrap()
            .is_some());
        let cache = obj::native_command_name_cache(original.as_ptr()).unwrap();
        assert!(interp.namespaces.borrow_mut().delete(GLOBAL, b"p"));
        // Complete the extracted owner cleanup outside the command-table borrow.
        interp.retire_pending_native_ensemble_roles().unwrap();
        assert!(worker.upgrade().is_none());
        assert_eq!(
            obj::native_command_name_cache(original.as_ptr()),
            Some(cache.clone())
        );
        assert!(interp
            .native_command_name_lookup_state(&cache)
            .unwrap()
            .target
            .is_none());
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

#[cfg(test)]
mod repinned_registration_key_tests {
    use super::*;
    use crate::{interp::Code, namespace::GLOBAL};

    fn worker(_: &mut Interp, _: &[*mut TclObj]) -> Code {
        Code::Ok
    }

    #[test]
    fn jim_repin_publishes_only_actual_unchanged_byte_registration_keys() {
        crate::counters::reset();
        {
            let mut interp = Interp::new();
            interp.register_builtin(b"registered", worker);
            let generation = interp
                .namespaces()
                .command_generation(GLOBAL, b"registered")
                .unwrap();
            assert!(interp
                .namespaces()
                .original_jim_command_key(GLOBAL, b"registered")
                .is_none());
            interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
            assert_eq!(
                interp
                    .namespaces()
                    .command_generation(GLOBAL, b"registered"),
                Some(generation)
            );
            let key = interp
                .namespaces()
                .original_jim_command_key(GLOBAL, b"registered")
                .unwrap();
            let refs = unsafe { (*key).ref_count };
            let original = obj::Owned::fresh(obj::new_string_bytes(b"registered"));
            for _ in 0..2 {
                assert_eq!(
                    interp
                        .resolve_original_command(original.as_ptr())
                        .unwrap()
                        .unwrap()
                        .1,
                    Some(generation)
                );
                assert_eq!(unsafe { (*key).ref_count }, refs);
            }
            assert_eq!(
                interp.eval_str(b"catch {error BOOM} result options; dict get $options -errorinfo"),
                Code::Ok
            );
            assert!(!interp.host_refusal_pending());
        }
        assert_eq!(crate::counters::finalize(), 0);
        assert_eq!(crate::counters::double_free_count(), 0);
    }

    #[test]
    fn jim_repin_declines_moved_or_nonroot_original_registration_slots() {
        let mut interp = Interp::new();
        interp.register_builtin(b"registered", worker);
        assert_eq!(interp.eval_str(b"rename registered moved"), Code::Ok);
        interp.register_builtin(b"N::relative", worker);
        let nonroot = interp.namespaces().find_namespace(GLOBAL, b"N").unwrap();
        let nonroot_generation = interp
            .namespaces()
            .command_generation(nonroot, b"relative")
            .unwrap();
        interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
        assert!(interp
            .namespaces()
            .original_jim_command_key(GLOBAL, b"moved")
            .is_none());
        assert_eq!(
            interp.namespaces().command_generation(nonroot, b"relative"),
            Some(nonroot_generation)
        );
        assert!(interp
            .namespaces()
            .original_jim_command_key(nonroot, b"relative")
            .is_none());
        let nonroot_name = obj::Owned::fresh(obj::new_string_bytes(b"N::relative"));
        assert!(interp
            .resolve_original_command(nonroot_name.as_ptr())
            .unwrap()
            .is_none());
        assert!(!interp.host_refusal_pending());
        let moved = obj::Owned::fresh(obj::new_string_bytes(b"moved"));
        assert!(interp.resolve_original_command(moved.as_ptr()).is_err());
        assert_eq!(interp.eval_str(b"moved"), Code::Error);
        assert!(interp.host_refusal_pending());
    }
    #[test]
    fn original_byte_registration_single_unlink_matches_full_validation() {
        let mut interp = Interp::with_native_core(
            super::super::default_host(),
            crate::environment::profile_for_dialect("tcl9.0"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        interp.register_builtin(b"kept", worker);
        interp.register_builtin(b"removed", worker);
        interp.prune_original_byte_command_registrations();
        let removed = interp
            .namespaces()
            .command_generation(GLOBAL, b"removed")
            .unwrap();
        let kept = interp
            .namespaces()
            .command_generation(GLOBAL, b"kept")
            .unwrap();
        let before = interp.namespaces().original_publication_revision().unwrap();
        drop(
            interp
                .namespaces_mut()
                .unlink_original_command_generation(removed),
        );
        assert_eq!(
            interp
                .namespaces()
                .original_publication_revision()
                .unwrap()
                .single_unlink_since(&before),
            Some(removed)
        );
        let expected: std::collections::BTreeSet<_> = interp
            .original_byte_command_registrations
            .borrow()
            .iter()
            .filter_map(|(&generation, registration)| {
                interp
                    .namespaces()
                    .original_publication_is_current(
                        registration.namespace,
                        &registration.tail,
                        generation,
                    )
                    .then_some(generation)
            })
            .collect();
        interp.prune_original_byte_command_registrations();
        let registrations = interp.original_byte_command_registrations.borrow();
        assert_eq!(
            registrations
                .keys()
                .copied()
                .collect::<std::collections::BTreeSet<_>>(),
            expected
        );
        assert!(!registrations.contains_key(&removed));
        assert_eq!(registrations.get(&kept).unwrap().original, b"kept");
        drop(registrations);

        interp.register_builtin(b"left", worker);
        interp.register_builtin(b"right", worker);
        let before = interp.namespaces().original_publication_revision().unwrap();
        for name in [b"left".as_slice(), b"right"] {
            let generation = interp
                .namespaces()
                .command_generation(GLOBAL, name)
                .unwrap();
            drop(
                interp
                    .namespaces_mut()
                    .unlink_original_command_generation(generation),
            );
        }
        assert_eq!(
            interp
                .namespaces()
                .original_publication_revision()
                .unwrap()
                .single_unlink_since(&before),
            None
        );
        interp.prune_original_byte_command_registrations();
        let registrations = interp.original_byte_command_registrations.borrow();
        assert!(registrations
            .values()
            .all(|registration| registration.original != b"left"
                && registration.original != b"right"));
        assert!(registrations.contains_key(&kept));
    }

    #[test]
    fn original_byte_registration_inputs_retire_with_replaced_and_deleted_slots() {
        let mut interp = Interp::new();
        let baseline = interp.original_byte_command_registrations.borrow().len();
        for _ in 0..32 {
            interp.register_builtin(b"churn", worker);
            assert_eq!(
                interp.original_byte_command_registrations.borrow().len(),
                baseline + 1
            );
        }
        assert_eq!(interp.eval_str(b"rename churn {}"), Code::Ok);
        assert_eq!(
            interp.original_byte_command_registrations.borrow().len(),
            baseline
        );
    }
}
