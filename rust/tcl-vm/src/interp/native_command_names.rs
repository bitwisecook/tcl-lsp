// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original-object command lookup and retained compiler literal actions.

use super::{
    Command, CommandId, CommandSidecarKey, CommandSlot, FunctionAsm, NamespacePath, Namespaces,
    NsId, ROOT_NS, Rc, Value, Vm,
};
use tcl_runtime_api::native_command_name::{
    NativeCommandNameCache, NativeCommandNameLookupState, NativeCommandNamePriming,
    NativeCommandNameReference, NativeCommandNameTarget, NativeLiteralContext,
};
use tcl_syntax::value::ValueError;

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
        &self,
        original: &Value,
    ) -> Result<Option<CommandId>, ValueError> {
        self.native_command_from_original_at(self.current_ns_id(), original)
    }

    fn install_selected_original_command_name(
        &self,
        original: &Value,
        key: &str,
        reference: Option<NativeCommandNameReference>,
        protocol: tcl_registry::native_command_literal::NativeCommandNameProtocol,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<Option<CommandId>, ValueError> {
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
        original.install_native_command_name_cache(cache, dialect)?;
        Ok(Some(CommandId(self.intern_cmd(token))))
    }

    fn native_command_from_original_at(
        &self,
        current: NsId,
        original: &Value,
    ) -> Result<Option<CommandId>, ValueError> {
        let dialect = self.native_scalar_carrier_dialect();
        let policy = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original command-name naming policy",
            ))?;
        if policy.authority() == tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation {
            let bytes = self.native_name_operand_bytes(original).map_err(|_| {
                ValueError::CommandProtocolUnavailable("original command-name string")
            })?;
            return self.find_command_bytes_checked(current, &bytes);
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
            ));
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
        let bytes = self
            .native_name_operand_bytes(original)
            .map_err(|_| ValueError::CommandProtocolUnavailable("original command-name string"))?;
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
        let lookup_context = if absolute && protocol.version() == tcl_dialect::TclVersion::V8_4 {
            ROOT_NS
        } else {
            current
        };
        if let Some(cache) = original.native_command_name_cache() {
            if !protocol.accepts_cache_origin(cache.version) {
                return Err(ValueError::CommandProtocolUnavailable(
                    "original command-name cache origin",
                ));
            }
            let state = self.native_command_name_lookup_state(&cache, lookup_context);
            if protocol.cache_is_current(&cache, &state) {
                return Ok(Some(CommandId(self.intern_cmd(cache.token))));
            }
        }
        let selected = self
            .resolve_command_bytes_checked(lookup_context, &bytes, true)
            .map_err(|_| ValueError::CommandProtocolUnavailable("original command-name lookup"))?;
        let Some(key) = selected else {
            if protocol.installs_unresolved_on_miss() {
                original.install_unresolved_native_command_name_cache(dialect)?;
            }
            return Ok(None);
        };
        self.install_selected_original_command_name(original, &key, reference, protocol, dialect)
    }

    /// Resolve the original operand and produce the selected native full name.
    /// Imported origins use the actual retained command token, not its spelling.
    pub(crate) fn native_namespace_command_name(
        &self,
        original: &Value,
        follow_imports: bool,
    ) -> Result<Option<Vec<u8>>, ValueError> {
        let Some(command) = self.native_command_from_original(original)? else {
            return Ok(None);
        };
        if follow_imports {
            return tcl_cmd_core::namespace::origin_from_command_checked(self, command).map(Some);
        }
        tcl_cmd_core::namespace::command_name_from_command_checked(self, command).map(Some)
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
        let dialect = self.actual_native_invocation_dialect();
        let Some(strings) = dialect
            .native_command_name_protocol()
            .and_then(|_| dialect.native_string_materialization(None))
        else {
            return self.refuse_host_command("native origin diagnostic String issuer".into());
        };
        let name = match self.native_name_operand_bytes(original) {
            Ok(name) => name,
            Err(error) => return self.refuse_host_command(error.to_string()),
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
    }

    /// Original command getter at a retained dispatch namespace. A valid cache
    /// supplies its live token; callers never look up the reporting bytes again.
    pub(crate) fn resolve_original_command_key_at(
        &self,
        context: NsId,
        original: &Value,
    ) -> Result<Option<String>, ValueError> {
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
            _ => Err(ValueError::CommandProtocolUnavailable(
                "resolved visible command token",
            )),
        }
    }

    pub(crate) fn lookup_original_command_at(
        &self,
        context: NsId,
        original: &Value,
    ) -> Result<Option<(String, Command)>, ValueError> {
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
            .ok_or(ValueError::CommandProtocolUnavailable(
                "resolved command implementation",
            ))
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
        let entry = self.native_compilation_entry_for_namespace_token(Some(namespace), false);
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
        for first_pass in table.discarded_native_passes() {
            let discarded =
                self.create_native_literal_table_pool(asm, first_pass, source_namespace)?;
            drop(discarded);
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
        } else {
            self.native_cache_stamp_for_source_namespace(source_namespace)
                .map_or(self.current_ns_id(), |stamp| stamp.namespace)
        };
        let world = Rc::clone(&self.native_literal_world);
        let before = self.native_cache_stamp(namespace);
        let pool = NativeLiteralPool::create_with_actions(
            &world,
            table,
            self.native_scalar_carrier_dialect()
                .native_string_protocol(),
            namespace,
            source_namespace,
            context,
            |effect| match effect {
                crate::literal_pool::NativeLiteralEffect::PrimeCommandName(receipt, original) => {
                    self.prime_native_command_literal(receipt, original)
                }
                crate::literal_pool::NativeLiteralEffect::PublishSyntax { message, options } => {
                    self.publish_original_compiler_syntax(message, options)
                }
            },
        )?;
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
            self.native_literal_world
                .borrow()
                .invalidate_command_literal(protocol, slot.namespace, slot.simple.as_bytes());
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
        let mut vm = Vm::new();
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
            let mut vm = crate::native_fixture::core(profile);
            command(&mut vm, b"head");
            for spelling in [b"head".as_slice(), b"::head"] {
                let original = Value::new_native_string_bytes(spelling);
                let selected = vm.native_command_from_original(&original).unwrap().unwrap();
                let cache = original.native_command_name_cache().unwrap();
                original.invalidate_native_string_for_test();
                let lookup = vm.native_command_from_original(&original);
                if name == "tcl8.4" {
                    assert!(matches!(
                        lookup,
                        Err(ValueError::CommandProtocolUnavailable(
                            "original command-name string"
                        ))
                    ));
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
                let entry = vm.native_compilation_entry_for_namespace_token(Some(ROOT_NS), false);
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
        assert!(matches!(
            vm.native_command_from_original(&original),
            Err(ValueError::CommandProtocolUnavailable(
                "original command-name cache origin"
            ))
        ));
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
