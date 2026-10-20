// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original-object command lookup and retained compiler literal actions.

use super::native_original_lookup::OriginalLookup;
use super::{
    Command, CommandId, CommandSidecarKey, CommandSlot, FunctionAsm, NamespaceExecutionContext,
    NamespacePath, Namespaces, NsId, ROOT_NS, Rc, Value, Vm,
};
use tcl_cmd_core::CmdError;
use tcl_runtime_api::native_command_name::{
    NativeCommandNameCache, NativeCommandNameLookupState, NativeCommandNamePriming,
    NativeCommandNameReference, NativeCommandNameTarget, NativeLiteralContext,
};
use tcl_syntax::value::ValueError;

pub(super) struct JimNamespaceOriginBackend<'a> {
    pub(super) vm: &'a mut Vm,
    pub(super) entered: &'a OriginalLookup,
    pub(super) protocol: tcl_syntax::native_jim_lookup::NativeJimLookupProtocol,
}

impl tcl_registry::native_jim_lookup::NativeJimNamespaceOriginBackend
    for JimNamespaceOriginBackend<'_>
{
    type Original = Value;
    type Error = CmdError;

    fn canonical(&mut self, original: &Value) -> Result<Value, CmdError> {
        self.entered.check(self.vm)?;
        let namespace =
            self.entered
                .jim_namespace_original()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "Jim origin namespace object",
                ))?;
        self.vm
            .original_jim_canonical_namespace_object(namespace, original, &|vm| {
                self.entered.check(vm)
            })
    }

    fn alias_prefix(&mut self, canonical: &Value) -> Result<Option<Value>, CmdError> {
        use tcl_runtime_api::Aliases;
        let selected = self.vm.lookup_original_command_at(ROOT_NS, canonical);
        self.entered.check(self.vm)?;
        let selected = selected?;
        let is_alias = match selected.as_ref().map(|(_, command)| command) {
            Some(Command::CallerAlias(_)) => true,
            Some(Command::Alias { .. }) => {
                return Err(ValueError::CommandProtocolUnavailable(
                    "Jim origin original alias flag owner",
                )
                .into());
            }
            _ => false,
        };
        drop(selected);
        self.entered.check(self.vm)?;
        if !is_alias {
            return Ok(None);
        }
        let reached = self.vm.alias_prefix_original_value_at(ROOT_NS, canonical);
        self.entered.check(self.vm)?;
        let failure = match reached? {
            tcl_runtime_api::AliasPrefixLookup::Prefix(original) => return Ok(Some(original)),
            tcl_runtime_api::AliasPrefixLookup::MissingCommand => {
                tcl_runtime_api::NativeJimAliasLookupFailure::MissingCommand
            }
            tcl_runtime_api::AliasPrefixLookup::NotAlias => {
                tcl_runtime_api::NativeJimAliasLookupFailure::NotAlias
            }
        };
        let bytes = self.vm.native_name_operand_bytes_checked(canonical)?;
        self.entered.check(self.vm)?;
        Err(CmdError::new_bytes(failure.message(&bytes))
            .with_native_string_result(tcl_syntax::native_string::NativeStringProtocol::Jim084))
    }

    fn command_exists(&mut self, original: &Value) -> Result<bool, CmdError> {
        let reached = self.vm.native_command_from_original_at(ROOT_NS, original);
        self.entered.check(self.vm)?;
        reached.map(|command| command.is_some())
    }

    fn report(&mut self, original: &Value) -> Result<Vec<u8>, CmdError> {
        let reached = self.vm.native_name_operand_bytes_checked(original);
        self.entered.check(self.vm)?;
        reached.map(|bytes| self.protocol.namespace_origin_result(&bytes))
    }

    fn same_original(&self, left: &Value, right: &Value) -> bool {
        left.is_same_object(right)
    }

    fn alias_cycle_unavailable(&mut self) -> CmdError {
        self.vm
            .retain_name_query_refusal("unbounded Jim namespace origin alias cycle")
    }
}

/// Actual compiler target/frame observations, with the genuine inherited
/// currency recorded by SavedExecutionTarget. An authored None remains None.
pub(super) struct OriginalLiteralOperation<'a> {
    vm: std::cell::RefCell<&'a mut Vm>,
    target: super::native_operation_currency::SavedExecutionTarget,
    frame: (usize, std::rc::Weak<crate::frame::ActivationIdentity>),
    world: std::rc::Weak<std::cell::RefCell<crate::literal_pool::NativeLiteralWorld>>,
}
impl<'a> OriginalLiteralOperation<'a> {
    pub(super) fn capture(vm: &'a mut Vm) -> Self {
        let target = super::native_operation_currency::SavedExecutionTarget::capture(vm);
        let frame = (
            vm.frames.len(),
            Rc::downgrade(
                &vm.frames
                    .last()
                    .expect("compiler physical frame")
                    .activation,
            ),
        );
        let world = Rc::downgrade(&vm.native_literal_world);
        Self {
            vm: std::cell::RefCell::new(vm),
            target,
            frame,
            world,
        }
    }
    fn current_cmd(&self) -> Result<(), CmdError> {
        let mut vm = self.vm.borrow_mut();
        self.target
            .check_current(&mut vm)
            .map_err(CmdError::from_execution_refusal)?;
        let current = vm.frames.len() == self.frame.0
            && vm.frames.last().is_some_and(|frame| {
                std::rc::Weak::ptr_eq(&Rc::downgrade(&frame.activation), &self.frame.1)
            })
            && std::rc::Weak::ptr_eq(&Rc::downgrade(&vm.native_literal_world), &self.world);
        if !current {
            let first = vm.retain_tcl_host_failure(crate::error::TclHostFailure::ValueAccess(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original literal compiler frame or world",
                ),
            ));
            return Err(CmdError::from_execution_refusal(first));
        }
        Ok(())
    }
    pub(super) fn check(&self) -> Result<(), crate::literal_pool::NativeLiteralUnavailable> {
        self.current_cmd().map_err(|_| {
            crate::literal_pool::NativeLiteralUnavailable::unavailable(
                "original literal compiler operation",
            )
        })
    }
    pub(super) fn refuse(
        &self,
        error: ValueError,
    ) -> crate::literal_pool::NativeLiteralUnavailable {
        let error: CmdError = error.into();
        self.retain(&error);
        crate::literal_pool::NativeLiteralUnavailable::unavailable(
            "original literal registration refused",
        )
    }
    fn retain(&self, error: &CmdError) {
        if let Some(first) = error.native_execution_refusal() {
            self.vm
                .borrow_mut()
                .retain_tcl_host_failure(crate::error::TclHostFailure::Execution(first.clone()));
        }
    }
    pub(super) fn string(
        &self,
        value: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        registration: &dyn Fn() -> Result<(), crate::literal_pool::NativeLiteralUnavailable>,
    ) -> Result<Vec<u8>, crate::literal_pool::NativeLiteralUnavailable> {
        let reached = value.native_string_bytes_checked_for_value_retaining::<CmdError>(
            protocol,
            None,
            &mut |original| {
                let reached = (|| {
                    if registration().is_err() {
                        self.current_cmd()?;
                        return Err(ValueError::CommandProtocolUnavailable(
                            "original literal registration stage",
                        )
                        .into());
                    }
                    self.current_cmd()?;
                    original
                        .original_native_header_currentness()
                        .map_err(|error| {
                            let first = self.vm.borrow_mut().retain_tcl_host_failure(
                                crate::error::TclHostFailure::ValueAccess(error),
                            );
                            CmdError::from_execution_refusal(first)
                        })
                })();
                if let Err(error) = &reached {
                    self.retain(error);
                }
                reached
            },
            &|error| self.retain(&error.clone().into()),
        );
        if let Err(error) = &reached {
            self.retain(error);
        }
        self.check()?;
        reached.map(|bytes| bytes.to_vec()).map_err(|_| {
            crate::literal_pool::NativeLiteralUnavailable::unavailable(
                "original literal String getter",
            )
        })
    }
    pub(super) fn with_vm<T>(
        &self,
        work: impl FnOnce(&mut Vm) -> Result<T, crate::literal_pool::NativeLiteralUnavailable>,
    ) -> Result<T, crate::literal_pool::NativeLiteralUnavailable> {
        self.check()?;
        let reached = work(&mut self.vm.borrow_mut());
        self.check()?;
        reached
    }
}

impl Vm {
    fn native_command_name_reference(&self, namespace: NsId) -> NativeCommandNameReference {
        NativeCommandNameReference {
            namespace_token: u64::from(namespace.0),
            command_reference_epoch: self
                .name_world
                .borrow()
                .command_reference_epochs
                .get(&namespace)
                .copied()
                .unwrap_or(0),
        }
    }

    fn native_command_name_target(&self, token: u64) -> Option<NativeCommandNameTarget> {
        let identity = self.command_token_at_generation(token)?;
        let namespace = match &identity.key {
            CommandSidecarKey::Visible(key) => self.command_slot(key)?.namespace,
            // A hidden node remains alive, but hiding has advanced its command epoch.
            CommandSidecarKey::Hidden(_) => ROOT_NS,
        };
        let world = self.name_world.borrow();
        Some(NativeCommandNameTarget {
            token,
            implementation_generation: identity.generation,
            command_epoch: world.command_name_epochs.get(&token).copied().unwrap_or(0),
            namespace_token: u64::from(namespace.0),
            namespace_dying: world.dying_namespaces.contains(&namespace),
        })
    }

    /// The actual live observations required by the shared cache-validity kernel.
    pub(crate) fn native_command_name_lookup_state(
        &self,
        cache: &NativeCommandNameCache,
        reference: NsId,
    ) -> NativeCommandNameLookupState {
        NativeCommandNameLookupState {
            interpreter: self.native_interpreter_identity(),
            reference: self.native_command_name_reference(reference),
            target: self.native_command_name_target(cache.token),
        }
    }

    /// Resolve the same original value through the selected native command getter.
    /// Cache misses perform native byte lookup and reached cache conversion; no
    /// reporting spelling or private storage key becomes a command-name operand.
    pub(crate) fn native_command_from_original(
        &mut self,
        original: &Value,
    ) -> Result<Option<CommandId>, CmdError> {
        if self.uses_native_jim_lookup() {
            let context = self.current_namespace_execution_context();
            let selected = self.native_jim_command_from_original_in_context(&context, original)?;
            return Ok(selected.and_then(|_| {
                original.with_jim_command_cache(|cache| CommandId(self.intern_cmd(cache.token)))
            }));
        }
        self.native_command_from_original_at(self.current_ns_id(), original)
    }

    fn install_selected_original_command_name(
        &mut self,
        original: &Value,
        key: &str,
        reference: Option<NativeCommandNameReference>,
        protocol: tcl_registry::native_command_literal::NativeCommandNameProtocol,
        dialect: tcl_registry::InvocationDialect,
        entered: &OriginalLookup,
    ) -> Result<Option<CommandId>, CmdError> {
        let selected = entered.command(self, key)?;
        let slot = self
            .command_slot(key)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original command-name slot",
            ))?;
        let token =
            self.visible_command_generation(key)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original command-name node",
                ))?;
        let cache = NativeCommandNameCache {
            interpreter: self.native_interpreter_identity(),
            version: protocol.version(),
            slot: tcl_core_types::NativeByteCommandSlot::new(
                self.ns_path(slot.namespace),
                slot.simple,
            ),
            namespace_token: u64::from(slot.namespace.0),
            token,
            implementation_generation: token,
            command_epoch: self
                .name_world
                .borrow()
                .command_name_epochs
                .get(&token)
                .copied()
                .unwrap_or(0),
            reference,
        };
        let referencing_namespace = match reference {
            Some(reference) => NsId(u32::try_from(reference.namespace_token).map_err(|_| {
                ValueError::CommandProtocolUnavailable("original command reference token")
            })?),
            None => ROOT_NS,
        };
        let current = self.native_command_name_lookup_state(&cache, referencing_namespace);
        entered.require(
            self,
            protocol.selected_binding_is_current(&cache, &current),
            "fresh original command binding",
        )?;
        let retained = cache.clone();
        let reached =
            original.install_native_command_name_cache_checked(cache, dialect, &mut || {
                selected.check(self, entered)?;
                let current =
                    self.native_command_name_lookup_state(&retained, referencing_namespace);
                entered.require(
                    self,
                    protocol.selected_binding_is_current(&retained, &current),
                    "fresh original command binding",
                )
            });
        selected.check(self, entered)?;
        reached?;
        Ok(Some(CommandId(self.intern_cmd(token))))
    }

    fn native_command_from_original_at(
        &mut self,
        current: NsId,
        original: &Value,
    ) -> Result<Option<CommandId>, CmdError> {
        let entered = OriginalLookup::enter(self, current, original)?;
        let reached: Result<Option<CommandId>, CmdError> = (|| {
            let dialect = self.native_scalar_carrier_dialect();
            let policy =
                self.name_policy_protocol()
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "original command-name naming policy",
                    ))?;
            if policy.authority() == tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation {
                let bytes = entered.read(self)?;
                return self
                    .find_command_bytes_checked(current, &bytes)
                    .map_err(Into::into);
            }
            if dialect.native_jim_lookup_protocol().is_some() {
                let selected = self.native_jim_command_from_original_at(current, original)?;
                return Ok(selected.and_then(|_| {
                    original.with_jim_command_cache(|cache| CommandId(self.intern_cmd(cache.token)))
                }));
            }
            let protocol = dialect.native_command_name_protocol().ok_or(
                ValueError::CommandProtocolUnavailable("original command-name getter"),
            )?;
            if original
                .native_command_name_cache_origin()
                .is_some_and(|origin| !protocol.accepts_cache_origin(origin))
            {
                return Err(ValueError::CommandProtocolUnavailable(
                    "original command-name cache origin",
                )
                .into());
            }
            // C85+ Tcl_GetCommandFromObj validates the original cached node before
            // GetString. C84 instead derives its referencing context from the
            // original spelling, including the absolute-name global reference.
            if protocol.version() >= tcl_dialect::TclVersion::V8_5
                && let Some(cache) = original.native_command_name_cache()
            {
                let state = self.native_command_name_lookup_state(&cache, current);
                if protocol.cache_is_current(&cache, &state) {
                    return Ok(Some(CommandId(self.intern_cmd(cache.token))));
                }
            }
            let bytes = entered.read(self)?;
            let path = self.ns_path(current);
            let names = self
                .name_policy_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original command-name naming policy",
                ))?
                .recipe();
            let projection = names
                .command_lookup_input(tcl_syntax::naming::NativeNameContext::new(&path), &bytes)
                .map_err(|_| {
                    ValueError::CommandProtocolUnavailable("original command-name projection")
                })?;
            let absolute =
                projection.qualification() == tcl_syntax::naming::NativeNameQualification::Absolute;
            let reference = protocol.lookup_reference(
                absolute,
                self.native_command_name_reference(current),
                self.native_command_name_reference(ROOT_NS),
            );
            let lookup_context = if absolute && protocol.version() == tcl_dialect::TclVersion::V8_4
            {
                ROOT_NS
            } else {
                current
            };
            if let Some(cache) = original.native_command_name_cache() {
                if !protocol.accepts_cache_origin(cache.version) {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "original command-name cache origin",
                    )
                    .into());
                }
                let state = self.native_command_name_lookup_state(&cache, lookup_context);
                if protocol.cache_is_current(&cache, &state) {
                    return Ok(Some(CommandId(self.intern_cmd(cache.token))));
                }
            }
            let selected = self
                .resolve_command_bytes_checked(lookup_context, &bytes, true)
                .map_err(|_| {
                    ValueError::CommandProtocolUnavailable("original command-name lookup")
                })?;
            let Some(key) = selected else {
                if protocol.installs_unresolved_on_miss() {
                    let reached = original
                        .install_unresolved_native_command_name_cache_checked(dialect, &mut || {
                            entered.check(self)
                        });
                    entered.check(self)?;
                    reached?;
                }
                return Ok(None);
            };
            self.install_selected_original_command_name(
                original, &key, reference, protocol, dialect, &entered,
            )
        })();
        entered.finish(self, reached)
    }

    /// Resolve the original operand and produce the selected native full name.
    /// Imported origins use the actual retained command token, not its spelling.
    pub(crate) fn native_namespace_command_name(
        &mut self,
        original: &Value,
        follow_imports: bool,
    ) -> Result<Option<Vec<u8>>, CmdError> {
        if follow_imports && self.uses_native_jim_lookup() {
            return self.native_jim_namespace_origin(original).map(Some);
        }
        if !follow_imports
            && self
                .name_policy_protocol()
                .is_some_and(|policy| policy.recipe().is_jim084())
        {
            let context = self.current_namespace_execution_context();
            let entered = OriginalLookup::enter_in_context(self, &context, original)?;
            let reached = (|| {
                let bytes = entered.read(self)?;
                let namespace = entered.read_jim_namespace(self)?;
                let current = entered.materialize_namespace(self)?;
                let path = self.ns_path(current);
                let candidates = tcl_syntax::naming::NativeNameProtocol::Jim084
                    .jim_namespace_command_query_candidates(
                        tcl_syntax::naming::NativeNameContext::with_jim_namespace(
                            &path, &namespace,
                        ),
                        &bytes,
                    )
                    .map_err(|_| {
                        ValueError::CommandProtocolUnavailable(
                            "Jim namespace command query projection",
                        )
                    })?;
                for candidate in candidates {
                    let lookup =
                        Value::from_native_string_bytes(candidate.lookup.as_bytes().to_vec());
                    let selected = self.native_command_from_original_at(ROOT_NS, &lookup)?;
                    entered.check(self)?;
                    drop(lookup);
                    entered.check(self)?;
                    if selected.is_some() {
                        return Ok(Some(candidate.reported.as_bytes().to_vec()));
                    }
                }
                Ok(None)
            })();
            return entered.finish(self, reached);
        }
        let Some(command) = self.native_command_from_original(original)? else {
            return Ok(None);
        };
        if follow_imports {
            return tcl_cmd_core::namespace::origin_from_command_checked(self, command)
                .map(Some)
                .map_err(Into::into);
        }
        tcl_cmd_core::namespace::command_name_from_command_checked(self, command)
            .map(Some)
            .map_err(Into::into)
    }

    fn native_jim_namespace_origin(&mut self, original: &Value) -> Result<Vec<u8>, CmdError> {
        let context = self.current_namespace_execution_context();
        let entered = OriginalLookup::enter_in_context(self, &context, original)?;
        let reached = (|| {
            let protocol = self
                .actual_native_invocation_dialect()
                .native_jim_lookup_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "Jim namespace origin issuer",
                ))?;
            let mut backend = JimNamespaceOriginBackend {
                vm: self,
                entered: &entered,
                protocol,
            };
            let outcome = tcl_registry::native_jim_lookup::native_jim_namespace_origin(
                &mut backend,
                original,
            )?;
            match outcome {
                tcl_registry::native_jim_lookup::NativeJimNamespaceOriginOutcome::Found(bytes) => {
                    Ok(bytes)
                }
                tcl_registry::native_jim_lookup::NativeJimNamespaceOriginOutcome::Missing(
                    original,
                ) => {
                    let bytes = backend.vm.native_name_operand_bytes_checked(&original)?;
                    backend.entered.check(backend.vm)?;
                    Err(
                        CmdError::new_bytes(protocol.namespace_origin_missing(&bytes))
                            .with_native_string_result(
                                tcl_syntax::native_string::NativeStringProtocol::Jim084,
                            ),
                    )
                }
            }
        })();
        entered.finish(self, reached)
    }

    /// Allocate the selected C full-command-name String result independently
    /// of the original command operand and its retained command-name cache.
    pub(crate) fn native_namespace_origin_result(&self, bytes: &[u8]) -> Result<Value, ValueError> {
        let dialect = self.actual_native_invocation_dialect();
        let strings = dialect
            .native_command_name_protocol()
            .and_then(|_| dialect.native_string_materialization(None))
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native origin String issuer",
            ))?;
        let result = Value::from_native_string_bytes(bytes.to_vec());
        result.retain_native_string_representation(strings)?;
        Ok(result)
    }

    /// Present the reached C origin failure from the SAME original operand.
    pub(crate) fn native_namespace_origin_failure(
        &mut self,
        original: &Value,
    ) -> super::Completion<Value> {
        let operation = match super::native_operation_currency::NativeOperationScope::enter(self) {
            Ok(operation) => operation,
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        let result = (|| {
            let dialect = self.actual_native_invocation_dialect();
            let Some(strings) = dialect
                .native_command_name_protocol()
                .and_then(|_| dialect.native_string_materialization(None))
            else {
                return self.refuse_host_command("native origin diagnostic String issuer".into());
            };
            let name = match self.native_name_operand_bytes_checked(original) {
                Ok(name) => name,
                Err(error) => return crate::command::completion_from_cmd_error(self, error),
            };
            let name = tcl_core_types::c_string_extent(&name);
            let mut message = b"invalid command name \"".to_vec();
            message.extend_from_slice(name);
            message.push(b'"');
            let mut code = b"TCL LOOKUP COMMAND ".to_vec();
            tcl_syntax::list::append_list_element(&mut code, name, false);
            let error = if dialect.tcl_version == Some(tcl_dialect::TclVersion::V8_4) {
                tcl_cmd_core::CmdError::new_bytes(message)
            } else {
                tcl_cmd_core::CmdError::with_error_code_bytes(message, code)
            };
            crate::command::completion_from_cmd_error(
                self,
                error.with_native_string_result(strings.protocol()),
            )
        })();
        match operation.check(self) {
            Ok(()) => result,
            Err(error) => crate::command::completion_from_cmd_error(self, error),
        }
    }

    pub(crate) fn resolve_original_command_key_in_context(
        &mut self,
        context: &NamespaceExecutionContext,
        original: &Value,
    ) -> Result<Option<String>, CmdError> {
        if self.uses_native_jim_lookup() {
            return self
                .native_jim_command_from_original_in_context(context, original)
                .map(|selected| selected.map(|(key, _)| key));
        }
        let NamespaceExecutionContext::Resolved(namespace) = context else {
            return Err(ValueError::CommandProtocolUnavailable(
                "original command namespace coordinates",
            )
            .into());
        };
        self.resolve_original_command_key_at(*namespace, original)
    }

    pub(crate) fn lookup_original_command_in_context(
        &mut self,
        context: &NamespaceExecutionContext,
        original: &Value,
    ) -> Result<Option<(String, Command)>, CmdError> {
        if self.uses_native_jim_lookup() {
            return self.native_jim_command_from_original_in_context(context, original);
        }
        let NamespaceExecutionContext::Resolved(namespace) = context else {
            return Err(ValueError::CommandProtocolUnavailable(
                "original command namespace coordinates",
            )
            .into());
        };
        self.lookup_original_command_at(*namespace, original)
    }

    /// Original command getter at a retained dispatch namespace. A valid cache
    /// supplies its live token; callers never look up the reporting bytes again.
    pub(crate) fn resolve_original_command_key_at(
        &mut self,
        context: NsId,
        original: &Value,
    ) -> Result<Option<String>, CmdError> {
        if self.uses_native_jim_lookup() {
            return self
                .native_jim_command_from_original_at(context, original)
                .map(|selected| selected.map(|(key, _)| key));
        }
        let Some(command) = self.native_command_from_original_at(context, original)? else {
            return Ok(None);
        };
        match self.command_sidecar_key(command.0) {
            Some(CommandSidecarKey::Visible(key)) => Ok(Some(key)),
            _ => {
                Err(ValueError::CommandProtocolUnavailable("resolved visible command token").into())
            }
        }
    }

    pub(crate) fn lookup_original_command_at(
        &mut self,
        context: NsId,
        original: &Value,
    ) -> Result<Option<(String, Command)>, CmdError> {
        if self
            .actual_native_invocation_dialect()
            .native_jim_lookup_protocol()
            .is_some()
            && self.name_policy_protocol().is_some_and(|policy| {
                policy.authority() == tcl_syntax::naming::NamePolicyAuthority::Native
            })
        {
            return self.native_jim_command_from_original_at(context, original);
        }
        let Some(key) = self.resolve_original_command_key_at(context, original)? else {
            return Ok(None);
        };
        self.visible_command_at_key(&key)
            .map(|command| Some((key, command)))
            .ok_or(ValueError::CommandProtocolUnavailable("resolved command implementation").into())
    }

    fn prime_native_command_literal(
        &self,
        receipt: &NativeCommandNamePriming,
        original: &Value,
    ) -> Result<(), crate::literal_pool::NativeLiteralUnavailable> {
        use crate::literal_pool::NativeLiteralUnavailable as Error;
        let namespace = NsId(u32::try_from(receipt.context.namespace_token).map_err(|_| {
            Error::unavailable("native command literal namespace token is unavailable")
        })?);
        let entry = self.native_compilation_entry_for_namespace_token(namespace, false);
        if !receipt.matches_entry(&entry) {
            return Err(Error::unavailable(
                "native command literal binding or compilation entry is stale",
            ));
        }
        let dialect = self.native_scalar_carrier_dialect();
        let protocol = dialect
            .native_command_name_protocol()
            .filter(|protocol| protocol.accepts_cache_origin(receipt.version))
            .ok_or(Error::unavailable(
                "native command literal object origin is unavailable",
            ))?;
        let cache = NativeCommandNameCache {
            interpreter: receipt.context.interpreter,
            version: receipt.version,
            slot: receipt.binding.slot.clone(),
            namespace_token: receipt.binding.namespace_token,
            token: receipt.binding.token,
            implementation_generation: receipt.binding.implementation_generation,
            command_epoch: self
                .name_world
                .borrow()
                .command_name_epochs
                .get(&receipt.binding.token)
                .copied()
                .unwrap_or(0),
            reference: protocol.priming_reference(
                receipt.fully_qualified,
                self.native_command_name_reference(namespace),
            ),
        };
        let current = self.native_command_name_lookup_state(&cache, namespace);
        if !protocol.selected_binding_is_current(&cache, &current) {
            return Err(Error::unavailable(
                "native command literal selected binding is stale",
            ));
        }
        original
            .prime_native_command_name_cache(cache, dialect)
            .map_err(|_| Error::unavailable("native command literal primary cache is unavailable"))
    }

    pub(super) fn create_native_literal_pool(
        &mut self,
        asm: &FunctionAsm,
        source_namespace: &NamespacePath,
    ) -> crate::literal_pool::NativeLiteralPoolReceipt {
        self.create_native_literal_table_pool(asm, &asm.literals, source_namespace)
    }

    fn create_native_literal_table_pool(
        &mut self,
        asm: &FunctionAsm,
        table: &tcl_bytecode::LiteralTable,
        source_namespace: &NamespacePath,
    ) -> crate::literal_pool::NativeLiteralPoolReceipt {
        use crate::literal_pool::{NativeLiteralPool, NativeLiteralUnavailable as Error};
        let target = super::native_operation_currency::SavedExecutionTarget::capture(self);
        let frame = (
            self.frames.len(),
            Rc::downgrade(
                &self
                    .frames
                    .last()
                    .expect("compiler physical frame")
                    .activation,
            ),
        );
        for first_pass in table.discarded_native_passes() {
            let discarded =
                self.create_native_literal_table_pool(asm, first_pass, source_namespace)?;
            drop(discarded);
            target
                .check_current(self)
                .map_err(|_| Error::unavailable("original literal discarded-pass cleanup"))?;
            if self.frames.len() != frame.0
                || !self.frames.last().is_some_and(|current| {
                    std::rc::Weak::ptr_eq(&Rc::downgrade(&current.activation), &frame.1)
                })
            {
                let error =
                    ValueError::CommandProtocolUnavailable("original literal discarded-pass frame");
                let error: CmdError = error.into();
                if let Some(first) = error.native_execution_refusal() {
                    self.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(
                        first.clone(),
                    ));
                }
                return Err(Error::unavailable("original literal discarded-pass frame"));
            }
        }
        if let Some(planned) = table.compiler_replay_environment() {
            let current = self.capture_native_compiler_pass_environment(None);
            if !planned.is_current_for(self.native_interpreter_identity())
                || !planned.is_root()
                || planned.has_enabled_limits()
                || current.as_ref() != Some(&planned.without_procedure())
            {
                return Err(Error::unavailable(
                    "native compiler replay environment changed during first-pass publication",
                ));
            }
        }
        let context: Option<&NativeLiteralContext> = table
            .entries()
            .iter()
            .find_map(|literal| match literal.allocation() {
                tcl_bytecode::NativeLiteralAllocation::RegisteredNativeCommand {
                    context, ..
                } => Some(context),
                _ => None,
            })
            .or_else(|| {
                table
                    .native_actions()
                    .iter()
                    .find_map(|action| match action {
                        tcl_bytecode::NativeLiteralAction::PrimeCommandName { receipt, .. } => {
                            Some(&receipt.context)
                        }
                        _ => None,
                    })
            });
        let namespace = if let Some(context) = context {
            let token = NsId(u32::try_from(context.namespace_token).map_err(|_| {
                Error::unavailable("native literal namespace token width is unavailable")
            })?);
            if context.interpreter != self.native_interpreter_identity()
                || context.entry_epoch != self.trace_deopt_epoch()
                || &context.namespace_path != source_namespace
                || self.name_world.borrow().ns_arena.get(token.0 as usize) != Some(source_namespace)
            {
                return Err(Error::unavailable(
                    "native literal retained namespace incarnation is stale or foreign",
                ));
            }
            token
        } else if let Some(stamp) = self.native_cache_stamp_for_source_namespace(source_namespace) {
            stamp.namespace
        } else {
            self.current_namespace_execution_context()
                .resolved_token()
                .ok_or_else(|| {
                    Error::unavailable("native literal namespace coordinate is unavailable")
                })?
        };
        let world = Rc::clone(&self.native_literal_world);
        let before = self.native_cache_stamp(namespace);
        let original = OriginalLiteralOperation::capture(self);
        original.check()?;
        let check = || original.check();
        let string = |value: &Value, protocol, registration: &dyn Fn() -> Result<(), Error>| {
            original.string(value, protocol, registration)
        };
        let refuse = |error| original.refuse(error);
        let callbacks = crate::literal_pool::OriginalLiteralCallbacks {
            check: &check,
            string: &string,
            refuse: &refuse,
        };
        let protocol = original
            .vm
            .borrow()
            .native_scalar_carrier_dialect()
            .native_string_protocol();
        let reached = NativeLiteralPool::create_with_actions_using(
            &world,
            table,
            protocol,
            namespace,
            source_namespace,
            context,
            |effect| {
                original.with_vm(|vm| match effect {
                    crate::literal_pool::NativeLiteralEffect::PrimeCommandName(receipt, value) => {
                        vm.prime_native_command_literal(receipt, value)
                    }
                    crate::literal_pool::NativeLiteralEffect::PublishSyntax {
                        message,
                        options,
                    } => vm.publish_original_compiler_syntax(message, options),
                })
            },
            Some(&callbacks),
        );
        if let Err(first) = original.check() {
            drop(reached);
            let _ = original.check();
            return Err(first);
        }
        let pool = reached?;
        drop(original);
        // The compiler reset invokes real variable traces. Their command,
        // namespace and policy mutations cannot refresh an older CPP receipt.
        if before != self.native_cache_stamp(namespace)
            || !self.native_compiler_prerequisites_match(asm)
            || !self.function_command_bindings_match(asm)
        {
            return Err(Error::unavailable(
                "native literal compiler context changed during publication",
            ));
        }
        Ok(pool)
    }

    pub(super) fn note_native_command_name_node_changed(&self, token: u64) {
        let mut world = self.name_world.borrow_mut();
        let epoch = world.command_name_epochs.entry(token).or_default();
        *epoch = epoch
            .checked_add(1)
            .expect("native command-name node epoch exhausted");
    }

    pub(super) fn note_native_command_reference_changed(&self, namespace: NsId) {
        let mut world = self.name_world.borrow_mut();
        let epoch = world.command_reference_epochs.entry(namespace).or_default();
        *epoch = epoch
            .checked_add(1)
            .expect("native namespace command-reference epoch exhausted");
    }

    pub(super) fn note_native_command_path_dependents(&self, target: NsId) {
        let dependents: Vec<_> = {
            let world = self.name_world.borrow();
            world
                .ns_paths
                .iter()
                .chain(
                    world
                        .ns_deferral
                        .retained
                        .values()
                        .flat_map(|record| record.paths.iter()),
                )
                .flat_map(|(creator, entries)| {
                    entries
                        .iter()
                        .filter(move |entry| **entry == target)
                        .map(move |_| *creator)
                })
                .collect()
        };
        for creator in dependents {
            self.note_native_command_reference_changed(creator);
        }
    }

    pub(super) fn note_native_namespace_command_lookup(&self, namespace: NsId) {
        if self.native_ensemble_namespace_has_exports(namespace) {
            self.advance_native_ensemble_export_epoch(namespace);
        }
        let has_path = {
            let world = self.name_world.borrow();
            let retained = world
                .ns_deferral
                .owners
                .get(&namespace)
                .and_then(|root| world.ns_deferral.retained.get(root))
                .and_then(|record| record.paths.get(&namespace));
            retained
                .or_else(|| world.ns_paths.get(&namespace))
                .is_some_and(|path| !path.is_empty())
        };
        if has_path {
            self.note_native_command_reference_changed(namespace);
        }
    }

    pub(super) fn invalidate_native_command_literal_in_slot(&self, slot: &CommandSlot) {
        if let Some(protocol) = self
            .native_scalar_carrier_dialect()
            .native_string_protocol()
        {
            crate::literal_pool::NativeLiteralWorld::invalidate_command_literal(
                &self.native_literal_world,
                protocol,
                slot.namespace,
                slot.simple.as_bytes(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interp::ok;
    use tcl_bytecode::LiteralTable;
    use tcl_dialect::TclVersion;
    use tcl_runtime_api::Completion;

    fn command(vm: &mut Vm, name: &[u8]) -> String {
        fn handler(_: &mut Vm, _: &[Value]) -> Completion<Value> {
            ok(Value::empty())
        }
        vm.register_command_in_slot(
            CommandSlot {
                namespace: ROOT_NS,
                simple: name.into(),
            },
            Command::Builtin(handler),
        )
    }

    fn cache_class(value: &Value) -> &'static str {
        if value.native_command_name_cache_origin().is_some() {
            "cmdName"
        } else {
            "none"
        }
    }

    #[test]
    fn origin_string_result_requires_actual_native_core_issuer() {
        let mut vm = Vm::without_native_engine();
        vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
        assert!(vm.native_namespace_origin_result(b"::selected").is_err());
    }

    #[test]
    fn native_namespace_origin_retains_string_result_birth_and_opaque_diagnostics() {
        // Native proof naming.namespace.origin-generated-opaque-failure-units:
        // docs/design/analysis/name-resolution-proofs/namespace-origin-generated-opaque-failure-units.md
        let source = include_bytes!(
            "../../../../runtime/rust/tests/data/native_namespace_origin_failures/source.tcl"
        );
        let rows = include_str!(
            "../../../../runtime/rust/tests/data/native_namespace_origin_failures/controls.tsv"
        );
        for (version, row) in TclVersion::ALL.into_iter().zip(rows.lines()) {
            let mut vm = crate::native_fixture::interpreter(
                tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap(),
            );
            let completion = vm.try_eval_source_bytes(source).unwrap();
            assert_eq!(completion.code, crate::Code::Ok, "{version:?}");
            let expected = row.split_once('\t').unwrap().1;
            let expected: Vec<_> = expected
                .as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect();
            assert_eq!(
                vm.native_name_operand_bytes(&completion.result)
                    .unwrap()
                    .as_ref(),
                expected,
                "{version:?}"
            );
            let result = vm.native_namespace_origin_result(b"::selected").unwrap();
            assert_eq!(result.native_object_type_name(), "string");
        }
    }

    #[test]
    fn current_original_command_cache_precedes_string_getter_after_c84() {
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = tcl_dialect::DialectProfile::find(name).unwrap();
            for spelling in [b"head".as_slice(), b"::head"] {
                let mut vm = crate::native_fixture::core(profile);
                command(&mut vm, b"head");
                let original = Value::new_native_string_bytes(spelling);
                let selected = vm.native_command_from_original(&original).unwrap().unwrap();
                let cache = original.native_command_name_cache().unwrap();
                original.invalidate_native_string_for_test();
                let lookup = vm.native_command_from_original(&original);
                if name == "tcl8.4" {
                    let error = lookup.unwrap_err();
                    let expected = CmdError::from(ValueError::NativeStringAccess(
                        tcl_syntax::raw_string::NativeStringAccessError::Unavailable(
                            tcl_syntax::native_string::NativeStringUnavailable::StringUpdater,
                        ),
                    ));
                    assert_eq!(
                        error.native_execution_refusal(),
                        expected.native_execution_refusal()
                    );
                    assert_eq!(
                        vm.execution_refusal.as_ref(),
                        expected.native_execution_refusal()
                    );
                } else {
                    assert_eq!(lookup.unwrap(), Some(selected), "{name} {spelling:?}");
                }
                assert!(original.resident_string_bytes().is_none());
                assert_eq!(original.native_command_name_cache(), Some(cache));
            }
        }
    }

    #[test]
    fn stale_and_foreign_stringless_command_caches_cannot_select_a_worker() {
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = tcl_dialect::DialectProfile::find(name).unwrap();
            let mut vm = crate::native_fixture::core(profile);
            command(&mut vm, b"head");
            let original = Value::new_native_string_bytes(b"head".as_slice());
            vm.native_command_from_original(&original).unwrap().unwrap();
            let cache = original.native_command_name_cache().unwrap();
            original.invalidate_native_string_for_test();
            let mut other = crate::native_fixture::core(profile);
            command(&mut other, b"head");
            assert!(other.native_command_from_original(&original).is_err());
            assert_eq!(original.native_command_name_cache(), Some(cache.clone()));
            assert!(original.resident_string_bytes().is_none());
            command(&mut vm, b"head");
            assert!(vm.native_command_from_original(&original).is_err());
            assert_eq!(original.native_command_name_cache(), Some(cache));
            assert!(original.resident_string_bytes().is_none());
        }
    }

    const COMMAND_ACTION_FIXTURES: [(TclVersion, &str); 5] = [
        (
            TclVersion::V8_4,
            include_str!("../../tests/data/native_literal_pools/command-actions/8.4.20.txt"),
        ),
        (
            TclVersion::V8_5,
            include_str!("../../tests/data/native_literal_pools/command-actions/8.5.19.txt"),
        ),
        (
            TclVersion::V8_6,
            include_str!("../../tests/data/native_literal_pools/command-actions/8.6.18.txt"),
        ),
        (
            TclVersion::V9_0,
            include_str!("../../tests/data/native_literal_pools/command-actions/9.0.4.txt"),
        ),
        (
            TclVersion::V9_1,
            include_str!("../../tests/data/native_literal_pools/command-actions/9.1.0.txt"),
        ),
    ];

    #[test]
    fn ordered_literal_actions_match_sixty_original_native_observations() {
        // Native proof: naming.literal.command-action.ordered-primary
        // docs/design/analysis/name-resolution-proofs/literal-command-action-ordered-primary.md
        // Native proof: naming.literal.command-action.replacement-primary
        // docs/design/analysis/name-resolution-proofs/literal-command-action-replacement-primary.md

        use crate::literal_pool::NativeLiteralPool;
        let fixtures = COMMAND_ACTION_FIXTURES;
        let mut checked = 0;
        for (version, fixture) in fixtures {
            let rows: Vec<_> = fixture
                .lines()
                .map(|line| line.split('\t').collect::<Vec<_>>())
                .collect();
            assert_eq!(rows.len(), 12);
            for (case, absolute, data_first) in [
                ("data-relative", false, true),
                ("command-relative", false, false),
                ("data-absolute", true, true),
                ("command-absolute", true, false),
            ] {
                let mut vm = Vm::new();
                vm.set_runtime_version(version);
                command(&mut vm, b"head");
                let head: &[u8] = if absolute { b"::head" } else { b"head" };
                let entry = vm.native_compilation_entry_for_namespace_token(ROOT_NS, false);
                let context = NativeLiteralContext {
                    interpreter: entry.interpreter,
                    namespace_token: entry.current_namespace,
                    entry_epoch: entry.epoch,
                    namespace_path: tcl_runtime_api::ByteNamespacePath::root(),
                };
                let receipt = NativeCommandNamePriming {
                    context: context.clone(),
                    original: head.into(),
                    binding: entry
                        .lookup_command_bytes(entry.current_namespace, head)
                        .unwrap()
                        .unwrap()
                        .clone(),
                    version,
                    fully_qualified: absolute,
                    authority: tcl_runtime_api::native_command_name::NativeCommandNamePrimingAuthority::OriginalLookup,
                };
                let mut table = LiteralTable::new();
                let before = data_first.then(|| table.intern_bytes(head));
                let index = table.intern_native_command_bytes(head, &context, absolute);
                assert!(table.prime_native_command_name(index, receipt));
                if version == TclVersion::V8_5 {
                    assert!(table.hide_native_literal(index));
                }
                let probe = before.unwrap_or_else(|| table.intern_bytes(head));
                let pool = NativeLiteralPool::create_with_actions(
                    &vm.native_literal_world,
                    &table,
                    vm.native_scalar_carrier_dialect().native_string_protocol(),
                    ROOT_NS,
                    &tcl_runtime_api::ByteNamespacePath::root(),
                    Some(&context),
                    |effect| match effect {
                        crate::literal_pool::NativeLiteralEffect::PrimeCommandName(
                            receipt,
                            value,
                        ) => vm.prime_native_command_literal(receipt, value),
                        crate::literal_pool::NativeLiteralEffect::PublishSyntax { .. } => {
                            panic!("command-only fixture has no Syntax producer")
                        }
                    },
                )
                .unwrap();
                let original = pool.value(probe).unwrap();
                for step in ["probe", "deleted", "recreated"] {
                    if step == "deleted" {
                        assert!(vm.take_command_unchecked("head").is_some());
                    }
                    if step == "recreated" {
                        command(&mut vm, b"head");
                    }
                    let row = rows
                        .iter()
                        .find(|row| row[0] == case && row[1] == step)
                        .unwrap();
                    assert_eq!(cache_class(&original), row[2], "{version:?} {case} {step}");
                    checked += 1;
                }
            }
        }
        assert_eq!(checked, 60);
    }

    #[test]
    fn original_lookup_refreshes_replaced_nodes_and_keeps_release_specific_misses() {
        for version in TclVersion::ALL {
            let mut vm = Vm::new();
            vm.set_runtime_version(version);
            command(&mut vm, b"head");
            let original = Value::new_native_string_bytes(b"head".as_slice());
            let resident = original.resident_string_bytes().unwrap();
            let first = vm.native_command_from_original(&original).unwrap().unwrap();
            let old = original.native_command_name_cache().unwrap();
            command(&mut vm, b"head");
            let second = vm.native_command_from_original(&original).unwrap().unwrap();
            assert_ne!(first, second);
            assert_ne!(
                old.token,
                original.native_command_name_cache().unwrap().token
            );
            assert!(Rc::ptr_eq(
                &resident,
                &original.resident_string_bytes().unwrap()
            ));
            vm.take_command_unchecked("head").unwrap();
            assert_eq!(vm.native_command_from_original(&original).unwrap(), None);
            assert_eq!(
                original.native_command_name_cache().is_some(),
                version >= TclVersion::V9_0
            );
            assert_eq!(original.native_command_name_cache_origin(), Some(version));
            assert_eq!(
                vm.native_namespace_command_name(&original, true).unwrap(),
                None
            );
        }
    }

    #[test]
    fn foreign_unresolved_cache_refuses_before_reached_conversion() {
        let mut vm = Vm::new();
        vm.set_runtime_version(TclVersion::V9_0);
        command(&mut vm, b"head");
        let original = Value::new_native_string_bytes(b"head".as_slice());
        original
            .install_unresolved_native_command_name_cache(
                tcl_registry::InvocationDialect::for_version(TclVersion::V8_4),
            )
            .unwrap();
        let resident = original.resident_string_bytes().unwrap();
        let error = vm.native_command_from_original(&original).unwrap_err();
        let expected = CmdError::from(ValueError::CommandProtocolUnavailable(
            "original command-name cache origin",
        ));
        assert_eq!(
            error.native_execution_refusal(),
            expected.native_execution_refusal()
        );
        assert_eq!(
            vm.execution_refusal.as_ref(),
            expected.native_execution_refusal()
        );
        assert!(Rc::ptr_eq(
            &resident,
            &original.resident_string_bytes().unwrap()
        ));
        assert_eq!(
            original.native_command_name_cache_origin(),
            Some(TclVersion::V8_4)
        );
        assert!(original.native_command_name_cache().is_none());
    }

    #[test]
    fn cmdname_mirror_preserves_original_allocation_and_rejects_foreign_origin() {
        let mut vm = Vm::new();
        vm.set_runtime_version(TclVersion::V9_0);
        command(&mut vm, b"head");
        let original = Value::new_native_string_bytes(b"head".as_slice());
        vm.native_command_from_original(&original).unwrap().unwrap();
        let duplicate = original.duplicate_native_object_in(
            tcl_syntax::native_string::NativeStringProtocol::C(TclVersion::V9_0),
        );
        let resident = original.resident_string_bytes().unwrap();
        original
            .adopt_native_object_representation_with_string_mutation(
                &duplicate,
                vm.native_scalar_carrier_dialect(),
                tcl_core_types::ResidentStringMutation::Preserve,
            )
            .unwrap();
        assert!(Rc::ptr_eq(
            &resident,
            &original.resident_string_bytes().unwrap()
        ));
        assert_eq!(
            original.native_command_name_cache(),
            duplicate.native_command_name_cache()
        );
        let foreign = tcl_registry::InvocationDialect::for_version(TclVersion::V8_6);
        assert!(
            original
                .adopt_native_object_representation(&duplicate, foreign)
                .is_err()
        );
        assert!(original.native_command_name_cache().is_some());
    }
}

#[cfg(test)]
mod info_commands_literal_tests;
