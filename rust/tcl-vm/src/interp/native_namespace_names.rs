// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original namespace-name conversion through actual retained namespace tokens.

use super::{Vm, native_name_queries::QueryNamespace, native_original_lookup::OriginalLookup};
use crate::Value;
use tcl_cmd_core::CmdError;
use tcl_core_types::{NsId, ROOT_NS};
use tcl_runtime_api::native_namespace_name::{
    NativeNamespaceNameCache as Cache, NativeNamespaceNameToken as Token,
};
use tcl_runtime_api::{Namespaces, native_compilation::NativeInterpreterIdentity};
use tcl_syntax::{
    native_namespace_name::{
        NativeNamespaceCurrentPrimary as Primary, NativeNamespaceObjectProducer as Producer,
    },
    value::ValueError,
};

mod jim_parent;

/// A reached C8.5 standalone ensemble namespace string-result stage.
/// Only this owner can issue it for the actual selected interpreter result.
pub(crate) struct OriginalEnsembleNamespaceResult<'a> {
    scope: &'a super::native_name_consumers::OriginalNameConsumerScope,
    result: &'a Value,
    namespace: Token,
    selected: NsId,
    lifecycle: tcl_syntax::native_namespace_name::NativeNamespaceLifecycle,
}

impl OriginalEnsembleNamespaceResult<'_> {
    fn check(&self, vm: &mut Vm) -> Result<(), CmdError> {
        self.scope.check(vm)?;
        let selected = vm.native_namespace_name_token(self.selected)?;
        let current = self.namespace.same_token(&selected)
            && self.namespace.lifecycle() == self.lifecycle
            && vm.with_native_interp_result(|actual| actual.is_same_object(self.result))?
            && !self.result.native_object_is_shared()
            && self.result.native_object_reference_count() == 1;
        self.scope
            .require(vm, current, "original ensemble namespace result owner")
    }

    pub(crate) fn bytes_for(&self, result: &Value) -> Result<&[u8], ValueError> {
        result.check_native_header()?;
        if !result.is_same_object(self.result)
            || result.native_object_is_shared()
            || result.native_object_reference_count() != 1
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "original ensemble namespace result header",
            ));
        }
        Ok(self.namespace.full_name().as_bytes())
    }
}

impl Vm {
    /// Install explicit storage geometry for fixed tests; this supplies no native
    /// operand spelling, lookup receipt, or production visibility extension.
    #[cfg(test)]
    pub(crate) fn install_constructed_namespace_test_fixture(
        &mut self,
        path: tcl_core_types::ByteNamespacePath,
    ) -> NsId {
        self.declare_namespace_path_with_origin(path.clone(), true);
        let selected = self.definition_namespace_token_at_path(&path, true);
        drop(path);
        selected
    }

    /// Snapshot the selected physical namespace reporting owner without parsing
    /// display bytes back into a namespace or granting a callable identity.
    pub(crate) fn original_namespace_report_parts(
        &mut self,
        namespace: NsId,
    ) -> Result<tcl_runtime_api::NativeNamespaceReportParts, CmdError> {
        use super::native_name_consumers::OriginalNameConsumerScope;
        use tcl_runtime_api::NativeNamespaceReportParts;

        let scope = OriginalNameConsumerScope::enter_for_namespace(self, Some(namespace))?;
        let reached = (|| {
            let policy =
                self.name_policy_protocol()
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "namespace report naming issuer",
                    ))?;
            if !policy.recipe().is_jim084() {
                let path = self
                    .name_world
                    .borrow()
                    .ns_arena
                    .get(namespace.0 as usize)
                    .cloned()
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "original constructed namespace report path",
                    ))?;
                scope.check(self)?;
                return Ok(NativeNamespaceReportParts::CConstructed(path));
            }
            let original =
                self.with_jim_namespace_at(namespace, |holder| Ok(holder.native_lifetime_lease()))?;
            let context = self.native_jim_object_context()?;
            let check = |vm: &mut Vm| -> Result<(), CmdError> {
                scope.current_for_value(vm, original.value())?;
                let current_context = vm.native_jim_object_context()?;
                scope.require(
                    vm,
                    std::rc::Rc::ptr_eq(&context, &current_context),
                    "original namespace report Jim context",
                )?;
                let same_holder = vm.with_jim_namespace_at(namespace, |holder| {
                    Ok(holder.is_same_object(original.value()))
                })?;
                scope.require(vm, same_holder, "original namespace report Jim holder")
            };
            check(self)?;
            // This existing getter owns allocator/context binding and String
            // callback stages; the report path has no private converter.
            let bytes = self.native_name_operand_bytes_checked_using(original.value(), &check);
            check(self)?;
            let bytes = bytes?;
            let bound_context = original.value().native_jim_context()?;
            scope.require(
                self,
                std::rc::Rc::ptr_eq(&context, &bound_context),
                "original namespace report object context",
            )?;
            Ok(NativeNamespaceReportParts::JimFlat(bytes))
        })();
        scope.finish(self)?;
        reached
    }

    pub(crate) fn original_jim_canonical_namespace_object(
        &mut self,
        namespace: &Value,
        original: &Value,
        check: &impl Fn(&mut Vm) -> Result<(), CmdError>,
    ) -> Result<Value, CmdError> {
        use tcl_syntax::naming::{NativeJimNamespaceConstruction, NativeNameContext};

        check(self)?;
        let namespace = namespace.native_lifetime_lease();
        let original = original.native_lifetime_lease();
        let scope = super::native_operation_currency::NativeOperationScope::enter(self)?;
        let current = |vm: &mut Vm| {
            check(vm)?;
            let operation =
                super::native_numeric_ops::OriginalNumericOps::in_original_scope(vm, &scope)?;
            operation.current_for_value(namespace.value())?;
            operation.current_for_value(original.value())
        };
        current(self)?;
        let dialect = self.actual_native_invocation_dialect();
        let protocol = dialect
            .native_name_protocol()
            .filter(|protocol| protocol.is_jim084())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "Jim canonical namespace issuer",
            ))?;
        let bytes = self.native_name_operand_bytes_checked_using(original.value(), &current);
        current(self)?;
        let bytes = bytes?;
        if let Some(absolute) = protocol
            .jim_namespace_absolute_canonical_input(&bytes)
            .map_err(|_| {
                ValueError::CommandProtocolUnavailable("Jim absolute canonical namespace")
            })?
        {
            let value = Value::from_native_string_bytes(absolute.selected());
            value.bind_native_jim_context(&self.native_jim_object_context()?)?;
            current(self)?;
            return Ok(value);
        }

        let is_current = self
            .with_jim_current_namespace(|current| Ok(current.is_same_object(namespace.value())))?;
        current(self)?;
        if is_current
            && let super::NamespaceExecutionContext::OriginalJim(context) =
                self.current_namespace_execution_context()
        {
            let reached = context.materialize_using(self, &current);
            current(self)?;
            reached?;
        }
        let namespace_bytes =
            self.native_name_operand_bytes_checked_using(namespace.value(), &current);
        current(self)?;
        let namespace_bytes = namespace_bytes?;
        let path = tcl_core_types::ByteNamespacePath::root();
        let context = NativeNameContext::with_jim_namespace(&path, &namespace_bytes);
        let construction = protocol
            .jim_namespace_construction(context, &bytes)
            .map_err(|_| ValueError::CommandProtocolUnavailable("Jim canonical construction"))?;
        let value = match construction {
            NativeJimNamespaceConstruction::RetainOriginal => original.value().clone(),
            NativeJimNamespaceConstruction::FreshString => {
                let projection = protocol
                    .jim_namespace_canonical_input(context, &bytes)
                    .map_err(|_| {
                        ValueError::CommandProtocolUnavailable("Jim canonical namespace name")
                    })?;
                Value::from_native_string_bytes(projection.selected())
            }
            NativeJimNamespaceConstruction::DuplicateNamespaceAndAppend => {
                let duplicate = namespace.value().duplicate_native_object_in(
                    tcl_syntax::native_string::NativeStringProtocol::Jim084,
                );
                current(self)?;
                let append = dialect.native_object_append_protocol(None).ok_or(
                    ValueError::CommandProtocolUnavailable("Jim canonical append issuer"),
                )?;
                let separator = Value::from_native_string_bytes(b"::".as_slice());
                let mut prepared = tcl_cmd_core::native_append::append_object(
                    &crate::value::VmAppendObjects,
                    append.recipe(),
                    Some(&duplicate),
                    &separator,
                )?;
                current(self)?;
                tcl_cmd_core::native_append::append_continuation(
                    &crate::value::VmAppendObjects,
                    &mut prepared,
                    original.value(),
                )?;
                current(self)?;
                prepared.into_value()
            }
        };
        value.bind_native_jim_context(&self.native_jim_object_context()?)?;
        current(self)?;
        Ok(value)
    }

    pub(crate) fn original_ensemble_namespace_result(
        &mut self,
        namespace: NsId,
        stage: tcl_syntax::native_namespace_name::NativeEnsembleNamespaceResultStage,
    ) -> Result<Value, CmdError> {
        use super::native_name_consumers::OriginalNameConsumerScope;
        use tcl_syntax::native_namespace_name::NativeEnsembleNamespaceResult;

        let scope = OriginalNameConsumerScope::enter_for_namespace(self, Some(namespace))?;
        let reached = (|| {
            let protocol = self
                .actual_native_invocation_dialect()
                .native_namespace_name_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "ensemble namespace result issuer",
                ))?;
            let production = protocol
                .recipe()
                .ensemble_configuration_result(stage)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "ensemble namespace result stage",
                ))?;
            if production == NativeEnsembleNamespaceResult::FreshObject {
                return self
                    .native_namespace_result_object(namespace, Producer::EnsembleConfiguration)
                    .map_err(CmdError::from);
            }
            let selected = namespace;
            let namespace = self.native_namespace_name_token(selected)?;
            let lifecycle = namespace.lifecycle();
            scope.require(
                self,
                protocol
                    .recipe()
                    .result_primary(Producer::EnsembleConfiguration, lifecycle)
                    .is_some(),
                "original ensemble namespace result lifetime",
            )?;
            let reuse =
                self.with_native_interp_result(|result| !result.native_object_is_shared())?;
            let retired = if reuse {
                None
            } else {
                self.native_c_result
                    .replace(Some(Value::new_native_string_bytes(b"".as_slice())))
            };
            let result = self.with_native_interp_result(Value::native_lifetime_lease)?;
            let original = OriginalEnsembleNamespaceResult {
                scope: &scope,
                result: result.value(),
                namespace,
                selected,
                lifecycle,
            };
            drop(retired);
            original.check(self)?;
            result
                .value()
                .initialise_original_ensemble_namespace_result(&original, &mut || {
                    original.check(self)
                })?;
            original.check(self)?;
            drop(original);
            Ok(result.into_value())
        })();
        scope.finish(self)?;
        reached
    }

    pub(crate) fn native_namespace_result_object(
        &mut self,
        namespace: NsId,
        producer: Producer,
    ) -> Result<Value, ValueError> {
        let dialect = self.actual_native_invocation_dialect();
        let Some(protocol) = dialect.native_namespace_name_protocol() else {
            if let Some(provider) = self.name_policy_protocol()
                && dialect.authored_namespace_result_recipe(provider).is_some()
            {
                return Value::authored_namespace_result(
                    &Namespaces::name_bytes(self, namespace),
                    producer,
                    provider,
                    dialect,
                );
            }
            if dialect.native_string_protocol()
                == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
                && matches!(producer, Producer::Current | Producer::CodeContext)
            {
                let recipe = dialect.native_jim_lookup_protocol().ok_or(
                    ValueError::CommandProtocolUnavailable("Jim original namespace result recipe"),
                )?;
                if namespace != self.current_ns_id() {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "Jim original namespace result frame",
                    ));
                }
                let original = self.with_jim_current_namespace(|original| Ok(original.clone()))?;
                let bytes = tcl_syntax::value::ValueOps::native_string_bytes(self, &original)?;
                let name = recipe.namespace_current_result(&bytes);
                let value = Value::from_native_string_cache(
                    tcl_syntax::native_object::NativeObjectCacheSnapshot::JimString {
                        num_chars: None,
                    },
                    dialect,
                    Some((
                        name.into(),
                        tcl_syntax::native_string::NativeStringStorageIdentity::Allocated,
                    )),
                )?;
                value.bind_native_jim_context(&self.native_jim_object_context()?)?;
                return Ok(value);
            }
            return Err(ValueError::CommandProtocolUnavailable(
                "native namespace result producer",
            ));
        };
        let token = self.native_namespace_name_token(namespace)?;
        let recipe = protocol.recipe();
        let primary = recipe.result_primary(producer, token.lifecycle()).ok_or(
            ValueError::CommandProtocolUnavailable("native namespace result lifetime"),
        )?;
        if primary == Primary::String {
            return Value::from_native_string_cache(
                tcl_syntax::native_object::NativeObjectCacheSnapshot::String {
                    protocol: tcl_syntax::native_string::NativeStringProtocol::C(recipe.version()),
                    num_chars: None,
                    unicode: None,
                },
                dialect,
                Some((
                    token.full_name().as_bytes().into(),
                    tcl_syntax::native_string::NativeStringStorageIdentity::Allocated,
                )),
            );
        }
        let value = Value::from_native_string_bytes(token.full_name().as_bytes());
        if primary == Primary::NamespaceName {
            value.install_native_namespace_name_cache(
                Cache::resolved(recipe.version(), token, None),
                dialect,
            )?;
        }
        Ok(value)
    }

    pub(crate) fn native_namespace_token_has_owner(&self, namespace: NsId) -> bool {
        let world = self.name_world.borrow();
        let Some(path) = world.ns_arena.get(namespace.0 as usize) else {
            return false;
        };
        if self
            .actual_native_invocation_dialect()
            .native_name_protocol()
            == Some(tcl_syntax::naming::NativeNameProtocol::Jim084)
        {
            // Jim publishes an original namespace object holder, independently
            // of C namespace tree registration. NsId births are never reused.
            return world.is_live() && world.jim_namespace_objects.contains_key(&namespace);
        }
        world.is_live()
            && ((world.namespaces.contains(path) && world.ns_intern.get(path) == Some(&namespace))
                || world.dying_namespaces.contains(&namespace)
                || world.ns_deferral.owners.contains_key(&namespace)
                || world.ns_deferral.retained.contains_key(&namespace))
    }

    pub(crate) fn native_namespace_name_token(&self, namespace: NsId) -> Result<Token, ValueError> {
        let protocol = self
            .actual_native_invocation_dialect()
            .native_namespace_name_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native namespace-name token issuer",
            ))?;
        let _recipe = protocol.recipe();
        {
            let world = self.name_world.borrow();
            if !world.is_live() || world.ns_arena.get(namespace.0 as usize).is_none() {
                return Err(ValueError::CommandProtocolUnavailable(
                    "native namespace incarnation",
                ));
            }
            if let Some(token) = world.namespace_name_tokens.get(&namespace) {
                return Ok(token.clone());
            }
        }
        if !self.native_namespace_token_has_owner(namespace) {
            return Err(ValueError::CommandProtocolUnavailable(
                "reserved namespace token has no native owner",
            ));
        }
        if self.ns_path(namespace).as_segments().iter().any(|segment| {
            segment.as_bytes().contains(&0)
                || segment.as_bytes().windows(2).any(|pair| pair == b"::")
        }) {
            return Err(ValueError::CommandProtocolUnavailable(
                "constructed namespace component has no native C spelling",
            ));
        }
        let fullname = Namespaces::name_bytes(self, namespace);
        let (owner, interpreter) = self.native_object_bridge_identity();
        let mut world = self.name_world.borrow_mut();
        let token = Token::new(
            NativeInterpreterIdentity { owner, interpreter },
            u64::from(namespace.0),
            fullname.into(),
            world
                .physical_namespace_parent(namespace)
                .map(|parent| u64::from(parent.0)),
        );
        if world.dying_namespaces.contains(&namespace) {
            token.mark_dying();
        } else if world.dead_namespaces.contains(&namespace) {
            if world.ns_deferral.retained.contains_key(&namespace) {
                token.mark_dying();
                token.detach_parent();
            } else if !world.ns_deferral.owners.contains_key(&namespace) {
                token.mark_dead();
            }
        }
        world.namespace_name_tokens.insert(namespace, token.clone());
        Ok(token)
    }

    pub(crate) fn begin_native_namespace_name_retirement(&self, namespace: NsId, detached: bool) {
        if let Ok(token) = self.native_namespace_name_token(namespace) {
            token.mark_dying();
            if detached {
                token.detach_parent();
            }
        }
    }

    pub(crate) fn finish_native_namespace_name_retirement(
        &self,
        namespace: NsId,
        global_reset: bool,
    ) {
        if let Some(token) = self
            .name_world
            .borrow()
            .namespace_name_tokens
            .get(&namespace)
        {
            if global_reset {
                let _restored = token.restore_after_global_reset();
            } else {
                token.mark_dead();
            }
        }
    }

    fn current_original_namespace_cache(
        &mut self,
        original: &Value,
        recipe: tcl_syntax::native_namespace_name::NativeNamespaceNameRecipe,
        interpreter: NativeInterpreterIdentity,
        reference: &Token,
        entered: &OriginalLookup,
    ) -> Result<Option<NsId>, CmdError> {
        if let Some(cache) = original.native_namespace_name_cache() {
            if cache.version() != recipe.version() {
                return Err(ValueError::CommandProtocolUnavailable(
                    "native namespace-name descriptor origin",
                )
                .into());
            }
            if cache.is_current(recipe, interpreter, reference)
                && let Some(target) = cache.namespace()
            {
                let namespace = NsId(u32::try_from(target.token()).map_err(|_| {
                    ValueError::CommandProtocolUnavailable("native namespace token width")
                })?);
                if self
                    .name_world
                    .borrow()
                    .namespace_name_tokens
                    .get(&namespace)
                    .is_some_and(|actual| actual.same_token(target))
                {
                    return Ok(Some(namespace));
                }
            }
            if !recipe.missing_installs_unresolved() {
                let resident = original.resident_string_bytes();
                original.retire_native_namespace_name_cache();
                entered.check(self)?;
                if resident.is_none() {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "retired namespace-name resident storage",
                    )
                    .into());
                }
            }
        }
        Ok(None)
    }

    fn install_original_resolved_namespace_cache(
        &mut self,
        original: &Value,
        dialect: tcl_registry::InvocationDialect,
        entered: &OriginalLookup,
        namespace: NsId,
        target: Token,
        reference: Option<Token>,
    ) -> Result<(), CmdError> {
        let recipe = dialect
            .native_namespace_name_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native namespace-name object issuer",
            ))?
            .recipe();
        let selected_target = QueryNamespace::capture(self, namespace)?;
        let target_lifecycle = target.lifecycle();
        let target_observation = target.clone();
        let reached = original.install_native_namespace_name_cache_checked(
            Cache::resolved(recipe.version(), target, reference),
            dialect,
            &mut || {
                entered.check(self)?;
                let current = selected_target.current(self)
                    && target_observation.lifecycle() == target_lifecycle
                    && self
                        .name_world
                        .borrow()
                        .namespace_name_tokens
                        .get(&namespace)
                        .is_some_and(|actual| actual.same_token(&target_observation));
                entered.require(self, current, "original namespace lookup target")
            },
        );
        entered.check(self)?;
        reached?;
        Ok(())
    }

    pub(crate) fn native_namespace_object_lookup(
        &mut self,
        original: &Value,
    ) -> Result<Option<NsId>, CmdError> {
        let current = self.current_ns_id();
        let entered = OriginalLookup::enter(self, current, original)?;
        let reached: Result<Option<NsId>, CmdError> = (|| {
            let dialect = self.native_invocation_dialect();
            let Some(protocol) = dialect.native_namespace_name_protocol() else {
                if original.native_namespace_name_cache().is_some() {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native namespace-name object issuer",
                    )
                    .into());
                }
                let bytes = entered.read(self)?;
                return Namespaces::find_namespace_bytes_checked(self, current, &bytes)
                    .map_err(Into::into);
            };
            let recipe = protocol.recipe();
            let names =
                dialect
                    .native_name_protocol()
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "native namespace-name input protocol",
                    ))?;
            let context_path = self
                .name_world
                .borrow()
                .ns_arena
                .get(current.0 as usize)
                .cloned()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "native namespace-name input context",
                ))?;
            let context = tcl_syntax::naming::NativeNameContext::new(&context_path);
            let mut bytes = if recipe.absolute_references_global() {
                Some(entered.read(self)?)
            } else {
                None
            };
            let reference_ns = if let Some(bytes) = &bytes {
                let selected = names.namespace_address_input(context, bytes).map_err(|_| {
                    ValueError::CommandProtocolUnavailable("native namespace-name input extent")
                })?;
                if selected.qualification() == tcl_syntax::naming::NativeNameQualification::Absolute
                {
                    ROOT_NS
                } else {
                    current
                }
            } else {
                current
            };
            let reference = self.native_namespace_name_token(reference_ns)?;
            let interpreter = reference.interpreter();
            if let Some(namespace) = self.current_original_namespace_cache(
                original,
                recipe,
                interpreter,
                &reference,
                &entered,
            )? {
                return Ok(Some(namespace));
            }
            if bytes.is_none() {
                bytes = Some(entered.read(self)?);
            }
            let bytes = bytes.expect("original namespace string was materialised");
            let input = names
                .namespace_address_input(context, &bytes)
                .map_err(|_| {
                    ValueError::CommandProtocolUnavailable("native namespace-name input extent")
                })?;
            let absolute =
                input.qualification() == tcl_syntax::naming::NativeNameQualification::Absolute;
            let selected = Namespaces::find_namespace_bytes_checked(self, reference_ns, &bytes)?
                .map(|namespace| {
                    self.native_namespace_name_token(namespace)
                        .map(|target| (namespace, target))
                })
                .transpose()?
                .filter(|(_, target)| recipe.permits_target(target.lifecycle()));
            let namespace = selected.as_ref().map(|(namespace, _)| *namespace);
            if let Some((namespace, target)) = selected {
                let reference =
                    (!absolute || recipe.absolute_references_global()).then_some(reference);
                self.install_original_resolved_namespace_cache(
                    original, dialect, &entered, namespace, target, reference,
                )?;
            } else if recipe.missing_installs_unresolved() {
                let reached = original.install_native_namespace_name_cache_checked(
                    Cache::unresolved(interpreter),
                    dialect,
                    &mut || entered.check(self),
                );
                entered.check(self)?;
                reached?;
            }
            Ok(namespace)
        })();
        entered.finish(self, reached)
    }
}

impl tcl_cmd_core::namespace::NamespaceObjectBackend for Vm {
    fn current_namespace_object(&mut self) -> Result<Value, ValueError> {
        self.native_namespace_result_object(self.current_ns_id(), Producer::Current)
    }
    fn produce_namespace_object(
        &mut self,
        namespace: NsId,
        producer: Producer,
    ) -> Result<Value, ValueError> {
        self.native_namespace_result_object(namespace, producer)
    }
}

impl tcl_cmd_core::namespace::NamespaceDeleteBackend for Vm {
    fn delete_selected_namespace(&mut self, namespace: NsId) -> Result<(), ValueError> {
        if !self.native_namespace_token_has_owner(namespace) {
            return Err(ValueError::CommandProtocolUnavailable(
                "selected namespace retirement owner",
            ));
        }
        self.bump_cmd_epoch();
        self.invalidate_compiled_command_semantics();
        self.invalidate_guard_domain(super::GuardDomain::CommandEnvironment);
        self.delete_namespace_token(namespace, namespace == ROOT_NS);
        if self.execution_refusal.is_some() {
            return Err(ValueError::CommandProtocolUnavailable(
                "namespace retirement host boundary",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod original_jim_canonical_ownership_tests {
    use super::*;

    #[test]
    fn absolute_canonical_name_retains_the_explicit_prefix_liveness_requirement() {
        for retired in [false, true] {
            let mut vm = crate::native_fixture::interpreter(
                crate::environment::profile_for_dialect("jimtcl"),
            );
            let prefix = Value::int(42);
            let original = prefix.native_lifetime_lease();
            let _kept = if retired {
                drop(prefix);
                None
            } else {
                Some(prefix)
            };
            let scope =
                super::super::native_name_consumers::OriginalNameConsumerScope::enter(&mut vm)
                    .unwrap();
            let name = Value::from_native_string_bytes(b"::target".as_slice());
            let reached =
                vm.original_jim_canonical_namespace_object(original.value(), &name, &|vm| {
                    scope.check(vm)
                });
            assert_eq!(reached.is_err(), retired);
            assert!(original.value().resident_string_bytes().is_none());
            assert_eq!(vm.execution_refusal.is_some(), retired);
            if !retired {
                assert_eq!(
                    reached.unwrap().resident_string_bytes().unwrap().as_ref(),
                    b"target"
                );
                scope.finish(&mut vm).unwrap();
            }
        }
    }

    #[test]
    fn absolute_canonical_name_rechecks_the_explicit_prefix_after_the_original_getter() {
        for retire in [false, true] {
            let mut vm = crate::native_fixture::interpreter(
                crate::environment::profile_for_dialect("jimtcl"),
            );
            let prefix = Value::int(42);
            let original = prefix.native_lifetime_lease();
            let retained = std::cell::RefCell::new(Some(prefix));
            let name = Value::native_list_constructor(
                vec![Value::from_native_string_bytes(b"::target".as_slice())],
                tcl_syntax::native_string::NativeStringProtocol::Jim084,
            );
            assert!(name.resident_string_bytes().is_none());
            let scope =
                super::super::native_name_consumers::OriginalNameConsumerScope::enter(&mut vm)
                    .unwrap();
            let reached =
                vm.original_jim_canonical_namespace_object(original.value(), &name, &|vm| {
                    if retire && name.resident_string_bytes().is_some() {
                        drop(retained.borrow_mut().take());
                    }
                    scope.check(vm)
                });
            assert_eq!(reached.is_err(), retire);
            assert!(original.value().resident_string_bytes().is_none());
            assert_eq!(vm.execution_refusal.is_some(), retire);
            if !retire {
                assert_eq!(
                    reached.unwrap().resident_string_bytes().unwrap().as_ref(),
                    b"target"
                );
                scope.finish(&mut vm).unwrap();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_core_types::ByteNamespacePath;
    use tcl_syntax::native_namespace_name::NativeNamespaceLifecycle as State;

    fn actual_vm(version: &str) -> Vm {
        let profile = tcl_registry::model::ingress::resolve_environment(version).unit_profile();
        let mut vm = Vm::new();
        assert!(vm.set_native_engine_profile(profile));
        vm.set_dialect_profile(profile);
        vm
    }

    fn declare(vm: &mut Vm, segments: &[&[u8]]) -> NsId {
        let path = ByteNamespacePath::from_segments(segments.iter().copied());
        vm.declare_namespace_path_with_origin(path.clone(), true);
        vm.definition_namespace_token_at_path(&path, true)
    }

    #[test]
    fn selected_c_report_parts_keep_actual_constructed_geometry_and_retirement() {
        use tcl_runtime_api::NativeNamespaceReportParts;
        for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm = binding_vm(release);
            let root = Namespaces::namespace_report_parts_checked(&mut vm, ROOT_NS).unwrap();
            assert_eq!(
                root,
                NativeNamespaceReportParts::CConstructed(ByteNamespacePath::root())
            );
            // Explicit storage geometry is not a claim that native C input can
            // create a literal separator or NUL inside a component.
            let left_path = ByteNamespacePath::from_segments([b"a::b".as_slice(), b"leaf\0\xff"]);
            let right_path =
                ByteNamespacePath::from_segments([b"a".as_slice(), b"b", b"leaf\0\xff"]);
            let left = vm.install_constructed_namespace_test_fixture(left_path.clone());
            let right = vm.install_constructed_namespace_test_fixture(right_path.clone());
            assert_ne!(left, right);
            assert_eq!(
                Namespaces::name_bytes(&vm, left),
                Namespaces::name_bytes(&vm, right)
            );
            let snapshot = Namespaces::namespace_report_parts_checked(&mut vm, left).unwrap();
            assert_eq!(
                snapshot,
                NativeNamespaceReportParts::CConstructed(left_path)
            );
            assert_eq!(
                Namespaces::namespace_report_parts_checked(&mut vm, right).unwrap(),
                NativeNamespaceReportParts::CConstructed(right_path)
            );
            vm.delete_namespace_token(left, false);
            assert!(Namespaces::namespace_report_parts_checked(&mut vm, left).is_err());
            assert!(matches!(
                snapshot,
                NativeNamespaceReportParts::CConstructed(_)
            ));
        }
    }

    #[test]
    fn selected_c_report_parts_keep_the_retained_dying_activation_then_refuse_retired_birth() {
        use tcl_runtime_api::NativeNamespaceReportParts;
        let mut vm = binding_vm("tcl8.6");
        let namespace = vm.activate_namespace_operand(b"::held::child").unwrap();
        let expected =
            NativeNamespaceReportParts::CConstructed(ByteNamespacePath::from_segments([
                b"held".as_slice(),
                b"child",
            ]));
        vm.push_ns_eval_token_frame(namespace, Vec::new());
        assert!(vm.delete_namespace_operand(b"::held::child").unwrap());
        assert_eq!(
            Namespaces::namespace_report_parts_checked(&mut vm, namespace).unwrap(),
            expected
        );
        vm.pop_ns();
        vm.pop_call_frame();
        assert!(Namespaces::namespace_report_parts_checked(&mut vm, namespace).is_err());
    }

    #[test]
    fn selected_jim_report_parts_reach_the_original_target_holder_and_borrow_its_header() {
        use tcl_runtime_api::NativeNamespaceReportParts;
        let mut vm = binding_vm("jimtcl");
        let context = vm.native_jim_object_context().unwrap();
        let root = Namespaces::namespace_report_parts_checked(&mut vm, ROOT_NS).unwrap();
        assert_eq!(
            root,
            NativeNamespaceReportParts::JimFlat(std::rc::Rc::from(b"".as_slice()))
        );
        let target = vm.intern_jim_namespace_object(b"target::leaf\0\xff");
        vm.push_ns_eval_token_frame(target, Vec::new());
        let original = Value::new_native_string_bytes(b"target::leaf\0\xff".as_slice());
        original.bind_native_jim_context(&context).unwrap();
        *vm.frames.last().unwrap().jim_namespace.borrow_mut() =
            Some((Some(target), original.clone()));
        let other = vm.intern_jim_namespace_object(b"other");
        vm.push_ns_eval_token_frame(other, Vec::new());
        let references = original.native_object_reference_count();
        let snapshot = Namespaces::namespace_report_parts_checked(&mut vm, target).unwrap();
        assert_eq!(
            snapshot,
            NativeNamespaceReportParts::JimFlat(std::rc::Rc::from(
                b"target::leaf\0\xff".as_slice()
            ))
        );
        assert_eq!(vm.current_ns_id(), other);
        assert_eq!(original.native_object_reference_count(), references);
        vm.pop_ns();
        vm.pop_call_frame();
        vm.pop_ns();
        vm.pop_call_frame();
        // Flat metadata remains, but an absent original frame is no producer.
        assert!(vm.native_namespace_token_has_owner(target));
        assert!(Namespaces::namespace_report_parts_checked(&mut vm, target).is_err());
        assert_eq!(
            snapshot,
            NativeNamespaceReportParts::JimFlat(std::rc::Rc::from(
                b"target::leaf\0\xff".as_slice()
            ))
        );
    }

    #[test]
    fn selected_jim_report_parts_get_the_original_stringless_holder() {
        use tcl_runtime_api::NativeNamespaceReportParts;
        let mut vm = binding_vm("jimtcl");
        let target = vm.intern_jim_namespace_object(b"42");
        vm.push_ns_eval_token_frame(target, Vec::new());
        let original = Value::from_native_scalar_cache(
            tcl_syntax::scalar_getter::NativeScalarCache::Number(tcl_syntax::number::Number::Int(
                42,
            )),
            None,
            vm.actual_native_invocation_dialect(),
        )
        .unwrap();
        original
            .bind_native_jim_context(&vm.native_jim_object_context().unwrap())
            .unwrap();
        *vm.frames.last().unwrap().jim_namespace.borrow_mut() =
            Some((Some(target), original.clone()));
        assert!(original.resident_string_bytes().is_none());
        let references = original.native_object_reference_count();
        assert_eq!(
            Namespaces::namespace_report_parts_checked(&mut vm, target).unwrap(),
            NativeNamespaceReportParts::JimFlat(std::rc::Rc::from(b"42".as_slice()))
        );
        assert_eq!(original.resident_string_bytes().unwrap().as_ref(), b"42");
        assert_eq!(original.native_object_reference_count(), references);
        vm.pop_ns();
        vm.pop_call_frame();
    }

    #[test]
    fn selected_report_parts_refuse_withdrawn_world_and_foreign_jim_object_context() {
        for release in ["tcl8.6", "jimtcl"] {
            let mut vm = binding_vm(release);
            vm.name_world.borrow_mut().retire();
            assert!(Namespaces::namespace_report_parts_checked(&mut vm, ROOT_NS).is_err());
        }
        let mut vm = binding_vm("jimtcl");
        let foreign = binding_vm("jimtcl");
        let target = vm.intern_jim_namespace_object(b"held");
        vm.push_ns_eval_token_frame(target, Vec::new());
        let original = Value::new_native_string_bytes(b"held".as_slice());
        original
            .bind_native_jim_context(&foreign.native_jim_object_context().unwrap())
            .unwrap();
        *vm.frames.last().unwrap().jim_namespace.borrow_mut() = Some((Some(target), original));
        assert!(Namespaces::namespace_report_parts_checked(&mut vm, target).is_err());
    }

    #[test]
    fn selected_report_parts_preserve_the_first_execution_refusal_without_a_getter() {
        let mut vm = binding_vm("jimtcl");
        let original = vm
            .with_jim_current_namespace(|holder| Ok(holder.native_lifetime_lease()))
            .unwrap();
        let before = original.value().native_object_snapshot();
        assert_eq!(
            vm.refuse_host_command("original namespace report first cause".into())
                .code,
            tcl_core_types::Code::Error,
        );
        let first = vm.execution_refusal.clone().unwrap();
        let error = Namespaces::namespace_report_parts_checked(&mut vm, ROOT_NS).unwrap_err();
        assert_eq!(error.native_execution_refusal(), Some(&first));
        assert_eq!(original.value().native_object_snapshot(), before);
    }

    #[test]
    fn qualified_inventory_keeps_existing_positive_command_and_variable_results() {
        for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm = crate::native_fixture::interpreter(
                crate::environment::profile_for_dialect(release),
            );
            assert_eq!(
                vm.eval_source("namespace eval selected {proc marker {} {}; variable item 1}")
                    .unwrap()
                    .code,
                tcl_runtime_api::Code::Ok
            );
            let commands = vm.invoke_host_original_object_vector(
                &Value::string("info"),
                &[Value::string("commands"), Value::string("::selected::*")],
            );
            assert_eq!(commands.code, tcl_runtime_api::Code::Ok, "{release}");
            assert!(
                commands
                    .result
                    .as_list()
                    .unwrap()
                    .iter()
                    .any(|name| name.string_bytes().as_ref() == b"::selected::marker")
            );
            let variables = vm.invoke_host_original_object_vector(
                &Value::string("info"),
                &[Value::string("vars"), Value::string("::selected::*")],
            );
            assert_eq!(variables.code, tcl_runtime_api::Code::Ok, "{release}");
            assert!(
                variables
                    .result
                    .as_list()
                    .unwrap()
                    .iter()
                    .any(|name| name.string_bytes().as_ref() == b"::selected::item")
            );
            assert!(vm.execution_refusal.is_none());
        }
    }

    fn binding_vm(release: &str) -> Vm {
        Vm::with_native_core(
            Box::new(std::io::sink()),
            std::rc::Rc::new(crate::host_native::NativeHost::new()),
            tcl_registry::model::ingress::resolve_environment(release).unit_profile(),
            tcl_registry::special_vars::NativeBootstrapInputs {
                package_path: Vec::new(),
                default_library: None,
            },
        )
        .unwrap()
    }

    fn binding_context(
        vm: &Vm,
        namespace: NsId,
    ) -> tcl_runtime_api::native_compilation::NativeNamespaceContext {
        vm.native_compilation_entry_for_namespace_token(namespace, true)
            .retained_namespace_context(u64::from(namespace.0))
            .unwrap()
    }

    fn binding_one(_: &mut Vm, _: &[Value]) -> tcl_core_types::Completion<Value> {
        super::super::ok(Value::int(1))
    }

    fn binding_two(_: &mut Vm, _: &[Value]) -> tcl_core_types::Completion<Value> {
        super::super::ok(Value::int(2))
    }

    fn register_binding_marker(vm: &mut Vm, namespace: NsId, second: bool) -> String {
        vm.register_command_in_slot(
            super::super::CommandSlot::new(namespace, tcl_core_types::NameBytes::from("marker")),
            crate::command::Command::Builtin(if second { binding_two } else { binding_one }),
        )
    }

    #[test]
    fn c85_ensemble_namespace_option_keeps_result_header_and_snapshot_stays_fresh() {
        use tcl_syntax::native_namespace_name::NativeEnsembleNamespaceResultStage as Stage;

        for shared in [false, true] {
            let mut vm = actual_vm("tcl8.5");
            let namespace = declare(&mut vm, &[b"raw\xff"]);
            let before = vm.adopt_native_interp_result(Value::int(42)).unwrap();
            let retained = shared.then(|| before.value().clone());
            vm.native_errors.primitive_error_code = Some(Value::string("EPISODE"));
            vm.native_errors.error_logged = crate::interp::ErrorLogged::Yes;
            vm.native_errors.native_error_info_len = 37;
            let result = vm
                .original_ensemble_namespace_result(namespace, Stage::Option)
                .unwrap();
            assert_eq!(result.is_same_object(before.value()), !shared);
            assert_eq!(
                result.resident_string_bytes().unwrap().as_ref(),
                b"::raw\xff"
            );
            assert_eq!(result.native_object_type_name(), "none");
            assert_eq!(result.native_object_reference_count(), 1);
            assert!(
                vm.with_native_interp_result(|actual| actual.is_same_object(&result))
                    .unwrap()
            );
            if let Some(retained) = retained {
                assert_eq!(retained.native_object_type_name(), "int");
                assert!(retained.resident_string_bytes().is_none());
            }
            let first = vm
                .original_ensemble_namespace_result(namespace, Stage::Snapshot)
                .unwrap();
            let second = vm
                .original_ensemble_namespace_result(namespace, Stage::Snapshot)
                .unwrap();
            assert!(!first.is_same_object(&result));
            assert!(!second.is_same_object(&result));
            assert!(!first.is_same_object(&second));
            assert_eq!(first.native_object_type_name(), "none");
            assert_eq!(
                first.resident_string_bytes().unwrap().as_ref(),
                b"::raw\xff"
            );
            assert!(
                vm.with_native_interp_result(|actual| actual.is_same_object(&result))
                    .unwrap()
            );
            assert_eq!(
                vm.native_errors
                    .primitive_error_code
                    .as_ref()
                    .unwrap()
                    .resident_string_bytes()
                    .unwrap()
                    .as_ref(),
                b"EPISODE"
            );
            assert!(vm.native_errors.error_logged.is_logged());
            assert_eq!(vm.native_errors.native_error_info_len, 37);
        }
    }

    #[test]
    fn compiled_native_namespace_context_separates_equal_colon_displays() {
        for release in ["tcl9.0", "tcl9.1"] {
            let mut vm = binding_vm(release);
            let left = declare(&mut vm, &[b"a:", b"b"]);
            let right = declare(&mut vm, &[b"a", b":b"]);
            assert_ne!(left, right);
            assert_eq!(vm.ns_name_bytes(left), vm.ns_name_bytes(right));
            let left_key = register_binding_marker(&mut vm, left, false);
            let right_key = register_binding_marker(&mut vm, right, true);
            assert_ne!(left_key, right_key);
            let left_context = binding_context(&vm, left);
            let right_context = binding_context(&vm, right);
            assert_ne!(left_context.path, right_context.path);
            for (context, expected_key, expected_result) in [
                (left_context.clone(), left_key, "1"),
                (right_context.clone(), right_key, "2"),
            ] {
                let constructed_path = context.path.clone();
                let binding = tcl_runtime_api::CommandBindingIdentity::in_namespace(
                    "misleading presentation",
                    "marker",
                    "marker",
                )
                .with_namespace_context(Some(
                    tcl_runtime_api::CompiledNamespaceContext::Native(context),
                ));
                let (key, command) = vm.lookup_compiled_command_binding(&binding).unwrap();
                assert_eq!(key, expected_key, "{release}");
                let crate::command::Command::Builtin(marker) = command else {
                    panic!("the selected original marker must be retained");
                };
                assert_eq!(
                    marker(&mut vm, &[]).result.to_str().as_ref(),
                    expected_result
                );
                // Authored source allocations retain component paths without
                // manufacturing an actual native namespace token.
                let constructed = binding.clone().with_namespace_context(Some(
                    tcl_runtime_api::CompiledNamespaceContext::ConstructedPath(constructed_path),
                ));
                assert_eq!(
                    vm.lookup_compiled_command_binding(&constructed).unwrap().0,
                    expected_key,
                    "{release}: constructed context keeps exact component boundaries",
                );
            }
            // Neither a matching display nor a different live token permits
            // replacing the component geometry captured by the original entry.
            let mut conflicting = left_context;
            conflicting.path = right_context.path;
            let invalid = tcl_runtime_api::CommandBindingIdentity::in_namespace(
                vm.ns_name_bytes(right).try_utf8().unwrap(),
                "marker",
                "marker",
            )
            .with_namespace_context(Some(
                tcl_runtime_api::CompiledNamespaceContext::Native(conflicting),
            ));
            assert!(vm.lookup_compiled_command_binding(&invalid).is_none());
        }
    }

    #[test]
    fn compiled_native_namespace_context_rejects_deleted_and_recreated_token() {
        use tcl_cmd_core::namespace::NamespaceDeleteBackend;
        for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm = binding_vm(release);
            let old = declare(&mut vm, &[b"N"]);
            let old_key = register_binding_marker(&mut vm, old, false);
            let original =
                tcl_runtime_api::CommandBindingIdentity::in_namespace("N", "marker", "marker")
                    .with_namespace_context(Some(
                        tcl_runtime_api::CompiledNamespaceContext::Native(binding_context(
                            &vm, old,
                        )),
                    ));
            let constructed = original.clone().with_namespace_context(Some(
                tcl_runtime_api::CompiledNamespaceContext::ConstructedPath(
                    vm.namespace_path_for_token(old),
                ),
            ));
            assert_eq!(
                vm.lookup_compiled_command_binding(&original).unwrap().0,
                old_key
            );
            vm.delete_selected_namespace(old).unwrap();
            assert!(vm.lookup_compiled_command_binding(&original).is_none());
            assert!(vm.lookup_compiled_command_binding(&constructed).is_none());
            let new = declare(&mut vm, &[b"N"]);
            assert_ne!(old, new, "{release}: recreation has its own incarnation");
            let new_key = register_binding_marker(&mut vm, new, true);
            assert!(vm.lookup_compiled_command_binding(&original).is_none());
            assert_eq!(
                vm.lookup_compiled_command_binding(&constructed).unwrap().0,
                new_key,
                "{release}: path-only source context resolves the new live owner",
            );
            let recreated = original.clone().with_namespace_context(Some(
                tcl_runtime_api::CompiledNamespaceContext::Native(binding_context(&vm, new)),
            ));
            assert_eq!(
                vm.lookup_compiled_command_binding(&recreated).unwrap().0,
                new_key
            );
        }
    }

    #[test]
    fn replay_namespace_context_preserves_colon_identity_and_retired_owner() {
        use tcl_cmd_core::namespace::NamespaceDeleteBackend;
        use tcl_runtime_api::CompiledNamespaceContext;
        for release in ["tcl9.0", "tcl9.1"] {
            let mut vm = binding_vm(release);
            let left = declare(&mut vm, &[b"a:", b"b"]);
            let right = declare(&mut vm, &[b"a", b":b"]);
            assert_eq!(vm.ns_name_bytes(left), vm.ns_name_bytes(right));
            let original = CompiledNamespaceContext::Native(binding_context(&vm, left));
            let other = CompiledNamespaceContext::Native(binding_context(&vm, right));
            vm.push_ns_eval_token_frame(ROOT_NS, Vec::new());
            for (context, selected) in [(original.clone(), left), (other, right)] {
                let previous = vm.enter_replay_namespace(&context).unwrap().unwrap();
                assert_eq!(vm.current_ns_id(), selected, "{release}");
                vm.leave_replay_namespace(previous);
                assert_eq!(vm.current_ns_id(), ROOT_NS);
            }
            vm.delete_selected_namespace(left).unwrap();
            let replacement = declare(&mut vm, &[b"a:", b"b"]);
            assert_ne!(left, replacement);
            assert!(vm.enter_replay_namespace(&original).is_err());
            assert_eq!(vm.current_ns_id(), ROOT_NS);
            let context = CompiledNamespaceContext::Native(binding_context(&vm, replacement));
            let previous = vm.enter_replay_namespace(&context).unwrap().unwrap();
            assert_eq!(vm.current_ns_id(), replacement);
            vm.leave_replay_namespace(previous);
        }
    }

    #[test]
    fn compiled_native_namespace_context_rejects_foreign_interpreter() {
        for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut origin = binding_vm(release);
            let mut foreign = binding_vm(release);
            let origin_namespace = declare(&mut origin, &[b"N"]);
            let foreign_namespace = declare(&mut foreign, &[b"N"]);
            register_binding_marker(&mut origin, origin_namespace, false);
            let foreign_key = register_binding_marker(&mut foreign, foreign_namespace, true);
            let context = binding_context(&origin, origin_namespace);
            let foreign_context = binding_context(&foreign, foreign_namespace);
            assert_eq!(context.path, foreign_context.path);
            assert_ne!(context.interpreter, foreign_context.interpreter);
            let binding =
                tcl_runtime_api::CommandBindingIdentity::in_namespace("N", "marker", "marker")
                    .with_namespace_context(Some(
                        tcl_runtime_api::CompiledNamespaceContext::Native(context),
                    ));
            assert!(origin.lookup_compiled_command_binding(&binding).is_some());
            assert!(foreign.lookup_compiled_command_binding(&binding).is_none());
            assert_eq!(
                foreign
                    .lookup_compiled_command_binding(&binding.with_namespace_context(Some(
                        tcl_runtime_api::CompiledNamespaceContext::Native(foreign_context)
                    )),)
                    .unwrap()
                    .0,
                foreign_key,
            );
        }
    }

    #[test]
    fn opaque_children_match_20_native_query_and_primary_windows() {
        // Native proof: naming.namespace-native.opaque-child-enumeration
        // docs/design/analysis/name-resolution-proofs/namespace-native-opaque-child-enumeration.md
        // Native proof: naming.namespace-native.opaque-child-pattern-selection
        // docs/design/analysis/name-resolution-proofs/namespace-native-opaque-child-pattern-selection.md
        use tcl_syntax::native_object::NativeObjectCacheSnapshot as Snapshot;
        let observations = include_str!(
            "../../../tcl-syntax/tests/data/native_namespace_name/opaque_children.txt"
        );
        let mut matched = 0;
        for (release, native) in [
            ("tcl8.4", "8.4.20"),
            ("tcl8.5", "8.5.19"),
            ("tcl8.6", "8.6.18"),
            ("tcl9.0", "9.0.4"),
            ("tcl9.1", "9.1.0"),
        ] {
            let mut vm = actual_vm(release);
            let parent = declare(&mut vm, &[b"raw\xff"]);
            declare(&mut vm, &[b"raw\xff", b"child\xfd"]);
            for (phase, pattern) in [
                ("enumerate", None),
                ("exact", Some(&b"::raw\xff::child\xfd"[..])),
                ("star", Some(&b"*"[..])),
                ("negative", Some(&b"*other*"[..])),
            ] {
                let expected: Vec<_> = observations
                    .lines()
                    .find(|row| row.starts_with(&format!("{native}|{phase}|")))
                    .unwrap()
                    .split('|')
                    .collect();
                let tokens =
                    tcl_cmd_core::namespace::children_tokens_checked(&vm, parent, pattern).unwrap();
                let result = tcl_cmd_core::namespace::children_original(&mut vm, &tokens).unwrap();
                let primary = match result.native_object_snapshot().cache {
                    Snapshot::None => "none",
                    Snapshot::List { .. } => "list",
                    other => panic!("unexpected original child result: {other:?}"),
                };
                assert_eq!(primary, expected[3], "{native} {phase}");
                let bytes = result
                    .native_string_bytes(tcl_syntax::native_string::NativeStringProtocol::C(
                        vm.runtime_version(),
                    ))
                    .unwrap();
                let hex: String = bytes.iter().fold(String::new(), |mut output, byte| {
                    use std::fmt::Write;
                    write!(output, "{byte:02x}").expect("writing into a String");
                    output
                });
                assert_eq!(hex, expected[4], "{native} {phase}");
                matched += 1;
            }
        }
        assert_eq!(matched, 20);
    }

    #[test]
    fn reserved_and_impossible_namespace_components_grant_no_native_descriptor() {
        let mut vm = actual_vm("tcl9.0");
        let reserved = vm.intern_ns("reserved");
        assert!(vm.native_namespace_name_token(reserved).is_err());
        assert_eq!(
            Namespaces::find_namespace_bytes_checked(&vm, ROOT_NS, b"reserved").unwrap(),
            None
        );
        let impossible = declare(&mut vm, &[b"raw\0name"]);
        assert!(vm.native_namespace_name_token(impossible).is_err());
        let normal = declare(&mut vm, &[b"reserved"]);
        assert_eq!(reserved, normal);
        assert!(vm.native_namespace_name_token(normal).is_ok());
    }

    #[test]
    fn native_deletion_validates_all_names_then_relooks_dependent_targets() {
        // Native proof: naming.namespace-native.delete-validates-later
        // docs/design/analysis/name-resolution-proofs/namespace-native-delete-validates-later.md
        // Native proof: naming.namespace-native.delete-dependent-child-duplicate
        // docs/design/analysis/name-resolution-proofs/namespace-native-delete-dependent-child-duplicate.md
        for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm = actual_vm(release);
            let parent = declare(&mut vm, &[b"P"]);
            let child = declare(&mut vm, &[b"P", b"q"]);
            let operands = [
                Value::from_native_string_bytes(b"::P".as_slice()),
                Value::from_native_string_bytes(b"::missing".as_slice()),
            ];
            assert_eq!(
                tcl_cmd_core::namespace::delete_original(&mut vm, &operands).unwrap(),
                Some(1)
            );
            assert_eq!(
                Namespaces::find_namespace_bytes_checked(&vm, ROOT_NS, b"::P").unwrap(),
                Some(parent)
            );
            assert_eq!(
                Namespaces::find_namespace_bytes_checked(&vm, ROOT_NS, b"::P::q").unwrap(),
                Some(child)
            );
            let operands = [
                Value::from_native_string_bytes(b"::P".as_slice()),
                Value::from_native_string_bytes(b"::P::q".as_slice()),
                Value::from_native_string_bytes(b"::P".as_slice()),
            ];
            assert_eq!(
                tcl_cmd_core::namespace::delete_original(&mut vm, &operands).unwrap(),
                None
            );
            assert_eq!(
                Namespaces::find_namespace_bytes_checked(&vm, ROOT_NS, b"::P").unwrap(),
                None
            );
            assert_eq!(
                Namespaces::find_namespace_bytes_checked(&vm, ROOT_NS, b"::P::q").unwrap(),
                None
            );
        }
    }

    #[test]
    fn deletion_second_pass_observes_callback_deleted_and_recreated_incarnations() {
        // Native proof: naming.namespace-native.delete-command-callback-removes-later
        // docs/design/analysis/name-resolution-proofs/namespace-native-delete-command-callback-removes-later.md
        use crate::command::NativeCommand;
        use std::{cell::Cell, rc::Rc};
        use tcl_cmd_core::namespace::NamespaceDeleteBackend;
        use tcl_core_types::Completion;

        struct Marker(Rc<Cell<usize>>);
        impl NativeCommand for Marker {
            fn invoke(&self, _: &mut Vm, _: &[Value]) -> Completion<Value> {
                super::super::ok(Value::empty())
            }
            fn retire(&self, _: &mut Vm) -> Completion<Value> {
                self.0.set(self.0.get() + 1);
                super::super::ok(Value::empty())
            }
        }
        struct DeleteSibling {
            recreate: bool,
            calls: Rc<Cell<usize>>,
            newborn: Rc<Cell<usize>>,
        }
        impl NativeCommand for DeleteSibling {
            fn invoke(&self, _: &mut Vm, _: &[Value]) -> Completion<Value> {
                super::super::ok(Value::empty())
            }
            fn retire(&self, vm: &mut Vm) -> Completion<Value> {
                assert!(vm.name_world.try_borrow_mut().is_ok());
                self.calls.set(self.calls.get() + 1);
                let sibling = Namespaces::find_namespace_bytes_checked(vm, ROOT_NS, b"::B")
                    .unwrap()
                    .unwrap();
                vm.delete_selected_namespace(sibling).unwrap();
                if self.recreate {
                    declare(vm, &[b"B"]);
                    vm.register_native_command(
                        "::B::marker",
                        Rc::new(Marker(Rc::clone(&self.newborn))),
                    );
                }
                super::super::ok(Value::empty())
            }
        }
        for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for recreate in [false, true] {
                let mut vm = actual_vm(release);
                declare(&mut vm, &[b"A"]);
                declare(&mut vm, &[b"B"]);
                let calls = Rc::new(Cell::new(0));
                let old = Rc::new(Cell::new(0));
                let newborn = Rc::new(Cell::new(0));
                vm.register_native_command("::B::marker", Rc::new(Marker(Rc::clone(&old))));
                vm.register_native_command(
                    "::A::hook",
                    Rc::new(DeleteSibling {
                        recreate,
                        calls: Rc::clone(&calls),
                        newborn: Rc::clone(&newborn),
                    }),
                );
                let operands = [
                    Value::from_native_string_bytes(b"::A".as_slice()),
                    Value::from_native_string_bytes(b"::B".as_slice()),
                ];
                assert_eq!(
                    tcl_cmd_core::namespace::delete_original(&mut vm, &operands).unwrap(),
                    None
                );
                assert_eq!(calls.get(), 1);
                assert_eq!(old.get(), 1);
                assert_eq!(newborn.get(), usize::from(recreate));
                assert_eq!(
                    Namespaces::find_namespace_bytes_checked(&vm, ROOT_NS, b"::A").unwrap(),
                    None
                );
                assert_eq!(
                    Namespaces::find_namespace_bytes_checked(&vm, ROOT_NS, b"::B").unwrap(),
                    None
                );
            }
        }
    }

    #[test]
    fn original_c9_namespace_and_duplicate_resolve_unaddressable_token() {
        for release in ["tcl9.0", "tcl9.1"] {
            let mut vm = actual_vm(release);
            let namespace = declare(&mut vm, &[b"a:", b"q"]);
            let original = vm
                .native_namespace_result_object(namespace, Producer::Current)
                .unwrap();
            assert_eq!(
                original.resident_string_bytes().unwrap().as_ref(),
                b"::a:::q"
            );
            assert_eq!(
                vm.namespace_object_lookup(&original).unwrap(),
                Some(namespace)
            );
            let dialect = vm.native_scalar_carrier_dialect();
            let duplicate =
                original.duplicate_native_object_in(dialect.native_string_protocol().unwrap());
            assert!(
                original
                    .native_namespace_name_cache()
                    .unwrap()
                    .same_descriptor(&duplicate.native_namespace_name_cache().unwrap())
            );
            assert_eq!(
                vm.namespace_object_lookup(&duplicate).unwrap(),
                Some(namespace)
            );
            let fresh = Value::from_native_string_bytes(b"::a:::q".as_slice());
            assert_eq!(vm.namespace_object_lookup(&fresh).unwrap(), None);
            assert!(fresh.native_namespace_name_cache().is_none());
            let parent = tcl_cmd_core::namespace::parent_original(&mut vm, namespace).unwrap();
            assert_eq!(
                vm.namespace_object_lookup(&parent).unwrap(),
                Namespaces::parent(&vm, namespace)
            );
        }
    }

    #[test]
    fn actual_namespace_cache_original_extent_context_and_retirement() {
        for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm = actual_vm(release);
            let namespace = declare(&mut vm, &[b"a"]);
            let original = Value::from_native_string_bytes(b"::a\0tail".as_slice());
            assert_eq!(
                vm.namespace_object_lookup(&original).unwrap(),
                Some(namespace)
            );
            assert_eq!(
                original.resident_string_bytes().unwrap().as_ref(),
                b"::a\0tail"
            );
            let cache = original.native_namespace_name_cache().unwrap();
            let version = cache.version();
            assert_eq!(
                cache.reference().is_some(),
                version == tcl_dialect::TclVersion::V8_4
            );
            let mut unrelated = actual_vm(release);
            assert_eq!(unrelated.namespace_object_lookup(&original).unwrap(), None);
            assert_eq!(
                vm.namespace_object_lookup(&original).unwrap(),
                Some(namespace)
            );
            let cache = original.native_namespace_name_cache().unwrap();
            vm.begin_native_namespace_name_retirement(namespace, true);
            assert_eq!(cache.namespace().unwrap().lifecycle(), State::Dying);
            assert_eq!(cache.namespace().unwrap().parent(), None);
            if version == tcl_dialect::TclVersion::V8_4 {
                assert_eq!(
                    vm.namespace_object_lookup(&original).unwrap(),
                    Some(namespace)
                );
            } else {
                assert!(!cache.is_current(
                    tcl_syntax::native_namespace_name::NativeNamespaceNameRecipe::for_tcl_version(
                        version
                    ),
                    cache.interpreter(),
                    &vm.native_namespace_name_token(ROOT_NS).unwrap()
                ));
            }
            vm.finish_native_namespace_name_retirement(namespace, false);
            assert_eq!(cache.namespace().unwrap().lifecycle(), State::Dead);
        }
    }

    #[test]
    fn c84_stringless_updater_uses_live_fullname_then_canonical_empty_after_death() {
        let mut vm = actual_vm("tcl8.4");
        let namespace = declare(&mut vm, &[b"a"]);
        let cache = Cache::resolved(
            tcl_dialect::TclVersion::V8_4,
            vm.native_namespace_name_token(namespace).unwrap(),
            None,
        );
        let dialect = vm.native_scalar_carrier_dialect();
        let original =
            Value::from_native_namespace_name_cache(cache.clone(), dialect, None).unwrap();
        let protocol =
            tcl_syntax::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4);
        assert_eq!(
            original.native_string_bytes(protocol).unwrap().as_ref(),
            b"::a"
        );
        vm.finish_native_namespace_name_retirement(namespace, false);
        let original = Value::from_native_namespace_name_cache(cache, dialect, None).unwrap();
        assert!(original.native_string_bytes(protocol).unwrap().is_empty());
        assert_eq!(
            original.native_object_snapshot().storage,
            Some(tcl_syntax::native_string::NativeStringStorageIdentity::CanonicalEmpty)
        );
    }
}
