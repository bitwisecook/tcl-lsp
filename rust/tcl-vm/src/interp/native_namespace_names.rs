// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original namespace-name conversion through actual retained namespace tokens.

use super::Vm;
use crate::Value;
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

impl Vm {
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
                let value = Value::from_native_string_cache(
                    tcl_syntax::native_object::NativeObjectCacheSnapshot::JimString {
                        num_chars: None,
                    },
                    dialect,
                    Some((
                        Namespaces::name_bytes(self, namespace).into(),
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
                .ns_parents
                .get(namespace.0 as usize)
                .copied()
                .flatten()
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

    pub(crate) fn native_namespace_object_lookup(
        &self,
        original: &Value,
    ) -> Result<Option<NsId>, ValueError> {
        let current = self.current_ns_id();
        let dialect = self.native_invocation_dialect();
        let Some(protocol) = dialect.native_namespace_name_protocol() else {
            if original.native_namespace_name_cache().is_some() {
                return Err(ValueError::CommandProtocolUnavailable(
                    "native namespace-name object issuer",
                ));
            }
            let bytes = self
                .native_name_operand_bytes(original)
                .map_err(|_| ValueError::CommandProtocolUnavailable("namespace object lookup"))?;
            return Namespaces::find_namespace_bytes_checked(self, current, &bytes);
        };
        let recipe = protocol.recipe();
        let mut bytes = if recipe.absolute_references_global() {
            Some(
                original
                    .native_string_bytes(tcl_syntax::native_string::NativeStringProtocol::C(
                        recipe.version(),
                    ))
                    .map_err(|error| {
                        ValueError::NativeStringAccess(
                            tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error),
                        )
                    })?,
            )
        } else {
            None
        };
        let reference_ns = if bytes.as_ref().is_some_and(|bytes| bytes.starts_with(b"::")) {
            ROOT_NS
        } else {
            current
        };
        let reference = self.native_namespace_name_token(reference_ns)?;
        let interpreter = reference.interpreter();
        if let Some(cache) = original.native_namespace_name_cache() {
            if cache.version() != recipe.version() {
                return Err(ValueError::CommandProtocolUnavailable(
                    "native namespace-name descriptor origin",
                ));
            }
            if cache.is_current(recipe, interpreter, &reference)
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
                if resident.is_none() {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "retired namespace-name resident storage",
                    ));
                }
            }
        }
        if bytes.is_none() {
            bytes = Some(
                original
                    .native_string_bytes(tcl_syntax::native_string::NativeStringProtocol::C(
                        recipe.version(),
                    ))
                    .map_err(|error| {
                        ValueError::NativeStringAccess(
                            tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error),
                        )
                    })?,
            );
        }
        let bytes = bytes.expect("original namespace string was materialised");
        let selected = Namespaces::find_namespace_bytes_checked(self, reference_ns, &bytes)?;
        if let Some(namespace) = selected {
            let target = self.native_namespace_name_token(namespace)?;
            let reference = (!bytes.starts_with(b"::") || recipe.absolute_references_global())
                .then_some(reference);
            original.install_native_namespace_name_cache(
                Cache::resolved(recipe.version(), target, reference),
                dialect,
            )?;
        } else if recipe.missing_installs_unresolved() {
            original
                .install_native_namespace_name_cache(Cache::unresolved(interpreter), dialect)?;
        }
        Ok(selected)
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
        vm.native_compilation_entry_for_namespace_token(Some(namespace), true)
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
                let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
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
            let unrelated = actual_vm(release);
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
