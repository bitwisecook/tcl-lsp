// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original C variable-name lookups and authentic compiled-local owners.

use super::{HashSet, ResolvedVar, Value, VarBinding, VarTableOwner, Vm};
use crate::value::NativeObjectLifetimeLease;
use std::rc::Rc;
use tcl_runtime_api::Completion;
use tcl_syntax::native_variable_name::{
    NativeLocalVariableName, NativeLocalVariableOwner, NativeParsedVariableElement,
    NativeParsedVariableName, NativeVariableNameLookupPurpose,
};

type CachedOriginalVariableParts = Option<(NativeObjectLifetimeLease, Rc<[u8]>)>;

type PreparedOriginalVariable = (
    crate::value::NativeObjectLifetimeLease,
    Rc<[u8]>,
    Option<Rc<[u8]>>,
    ResolvedVar,
);

impl Vm {
    pub(crate) fn native_c_variable_name_protocol(
        &self,
    ) -> Option<tcl_syntax::native_variable_name::NativeVariableNameProtocol> {
        let selected = self.name_policy_protocol()?;
        let protocol = self
            .actual_native_invocation_dialect()
            .native_variable_name_protocol()?;
        (selected.authority() == tcl_syntax::naming::NamePolicyAuthority::Native
            && selected.recipe() == tcl_syntax::naming::NativeNameProtocol::C(protocol.version()))
        .then_some(protocol)
    }

    pub(crate) fn install_native_variable_name_owners(
        &mut self,
        procedure: &Rc<crate::command::ProcDef>,
        body: &crate::compiled::CompiledUnit,
    ) -> Result<(), Completion<Value>> {
        let Some(protocol) = self.native_c_variable_name_protocol() else {
            return Ok(());
        };
        procedure.retain_native_compiled_names(body.asm.lvt.native_slot_names());
        if protocol.local_cache_owns_procedure() {
            self.frames
                .last_mut()
                .expect("actual procedure activation")
                .native_c_procedure = Some(Rc::clone(procedure));
            return Ok(());
        }
        if body.native_local_names.borrow().is_none() {
            let names = crate::literal_pool::create_local_names(
                &self.native_literal_world,
                &body.asm.lvt.native_slot_names(),
                tcl_syntax::native_string::NativeStringProtocol::C(protocol.version()),
            )
            .map_err(|error| self.refuse_host_command(error.to_string()))?;
            *body.native_local_names.borrow_mut() = Some(names);
        }
        let names = body.native_local_names.borrow().clone();
        self.frames
            .last_mut()
            .expect("actual procedure activation")
            .native_local_names = names;
        Ok(())
    }

    fn native_original_local_binding(&self, original: &Value) -> Option<VarBinding> {
        let frame = self.frames.last()?;
        original
            .with_native_local_variable(|cache| {
                let current = self.native_c_variable_name_protocol()?;
                if current != cache.protocol || cache.index >= frame.compiled_locals.len() {
                    return None;
                }
                let matches = match &cache.owner {
                    NativeLocalVariableOwner::Procedure(procedure) => frame
                        .native_c_procedure
                        .as_ref()
                        .is_some_and(|current| Rc::ptr_eq(procedure.declaration(), current)),
                    NativeLocalVariableOwner::Name(canonical) => frame
                        .native_local_names
                        .as_ref()?
                        .names
                        .get(cache.index)?
                        .as_ref()
                        .is_some_and(|current| {
                            current.is_same_object(canonical.as_ref().unwrap_or(original))
                        }),
                };
                matches.then(|| VarBinding {
                    owner: VarTableOwner::CompiledLocal {
                        level: self.current_level(),
                        slot: cache.index,
                    },
                    name: frame.compiled_locals[cache.index].0.clone(),
                })
            })
            .flatten()
    }

    fn install_selected_original_variable_cache(
        &mut self,
        original: &Value,
        binding: &VarBinding,
    ) -> Result<(), Completion<Value>> {
        if self.native_original_local_binding(original).as_ref() == Some(binding) {
            return Ok(());
        }
        let dialect = self.actual_native_invocation_dialect();
        let protocol = self
            .native_c_variable_name_protocol()
            .expect("selected actual C lookup");
        let cache = if let VarTableOwner::CompiledLocal { level, slot } = binding.owner {
            let owner = if protocol.local_cache_owns_procedure() {
                let procedure = self
                    .frames
                    .get(level)
                    .and_then(|frame| frame.native_c_procedure.clone())
                    .ok_or_else(|| {
                        self.refuse_host_command(
                            "actual C8.4 local procedure owner unavailable".into(),
                        )
                    })?;
                NativeLocalVariableOwner::Procedure(
                    crate::command::NativeProcedureReference::acquire(&procedure),
                )
            } else {
                let canonical = self
                    .frames
                    .get(level)
                    .and_then(|frame| frame.native_local_names.as_ref())
                    .and_then(|table| table.names.get(slot))
                    .and_then(Option::as_ref)
                    .map(Value::native_lifetime_lease)
                    .ok_or_else(|| {
                        self.refuse_host_command("actual canonical local name unavailable".into())
                    })?;
                // A lifetime view does not add a native name reference while converting.
                let canonical = canonical.value();
                let owner = if canonical.is_same_object(original) {
                    NativeLocalVariableOwner::Name(None)
                } else {
                    NativeLocalVariableOwner::Name(Some(canonical.clone()))
                };
                original
                    .install_native_local_variable(
                        NativeLocalVariableName {
                            protocol,
                            index: slot,
                            owner,
                        },
                        dialect,
                    )
                    .map_err(|error| self.refuse_host_command(error.to_string()))?;
                if !canonical.is_same_object(original)
                    && protocol.installs_canonical_local_self_cache()
                {
                    canonical.retire_native_variable_primary();
                    canonical
                        .install_native_local_variable(
                            NativeLocalVariableName {
                                protocol,
                                index: slot,
                                owner: NativeLocalVariableOwner::Name(None),
                            },
                            dialect,
                        )
                        .map_err(|error| self.refuse_host_command(error.to_string()))?;
                } else if protocol.version() == tcl_dialect::TclVersion::V8_6
                    && !canonical.is_same_object(original)
                    && !canonical
                        .with_native_local_variable(|cache| {
                            matches!(cache.owner, NativeLocalVariableOwner::Name(None))
                        })
                        .unwrap_or(false)
                {
                    canonical.retire_native_variable_primary();
                }
                return Ok(());
            };
            Some(NativeLocalVariableName {
                protocol,
                index: slot,
                owner,
            })
        } else {
            None
        };
        if let Some(cache) = cache {
            original.install_native_local_variable(cache, dialect)
        } else {
            original.install_native_parsed_variable(
                NativeParsedVariableName {
                    protocol,
                    array: None,
                },
                dialect,
            )
        }
        .map_err(|error| self.refuse_host_command(error.to_string()))
    }

    /// Prepare the original name before any selected receiver observers run.
    fn prepare_native_original_variable(
        &mut self,
        original: &Value,
        purpose: NativeVariableNameLookupPurpose,
    ) -> Result<PreparedOriginalVariable, Completion<Value>> {
        self.prepare_native_original_variable_in(original, purpose, None)
    }

    fn prepare_native_original_variable_in(
        &mut self,
        original: &Value,
        purpose: NativeVariableNameLookupPurpose,
        namespace: Option<tcl_core_types::NsId>,
    ) -> Result<PreparedOriginalVariable, Completion<Value>> {
        self.prepare_native_original_variable_parts(original, purpose, namespace, None)
    }

    /// Explicit part2 keeps the original part1 scalar name, as requested by
    /// the native array opcode's `TCL_PART1_NOT_PARSED` lookup flag.
    fn cache_native_original_array_name(
        &mut self,
        original: &Value,
        protocol: tcl_syntax::native_variable_name::NativeVariableNameProtocol,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<bool, Completion<Value>> {
        let original_bytes = self
            .native_name_operand_bytes(original)
            .map_err(|error| self.refuse_host_command(error.to_string()))?;
        if let Some((root, element)) = protocol.parsed_array_parts(&original_bytes) {
            let root = Value::new_native_string_bytes(root);
            let element = if let Some(bytes) = protocol.new_element_bytes(element) {
                NativeParsedVariableElement::Bytes(bytes)
            } else {
                NativeParsedVariableElement::Object(Value::new_native_string_bytes(element))
            };
            original
                .install_native_parsed_variable(
                    NativeParsedVariableName {
                        protocol,
                        array: Some((root, element)),
                    },
                    dialect,
                )
                .map_err(|error| self.refuse_host_command(error.to_string()))?;
            return Ok(true);
        }
        Ok(false)
    }

    fn select_native_original_root_binding(
        &mut self,
        root: &Value,
        reported: (&[u8], Option<&[u8]>),
        cached: Option<VarBinding>,
        protocol: tcl_syntax::native_variable_name::NativeVariableNameProtocol,
        namespace: Option<tcl_core_types::NsId>,
        purpose: NativeVariableNameLookupPurpose,
    ) -> Result<VarBinding, Completion<Value>> {
        let (root_bytes, element) = reported;
        if cached.is_none()
            && protocol.retires_before_simple_lookup(root.native_primary_has_free_hook())
        {
            root.retire_native_variable_primary();
        }
        let binding = cached.or_else(|| {
            let selected = tcl_syntax::naming::NativeNameProtocol::C(protocol.version())
                .variable_root_input(root_bytes);
            if let Some(namespace) = namespace {
                self.namespace_var_binding_from_token_bytes(namespace, selected.selected())
            } else {
                self.var_binding_from_bytes(selected.selected(), self.current_level())
            }
        });
        let Some(mut binding) = binding else {
            if !purpose.leaves_error_message() {
                return Err(super::err(""));
            }

            return Err(self.variable_access_error_input(
                purpose.diagnostic_verb(),
                tcl_syntax::naming::NativeVariableInputForm::Separate {
                    root: root_bytes,
                    element,
                },
                "parent namespace doesn't exist",
            ));
        };
        if let VarTableOwner::Frame(level) = binding.owner
            && let Some(slot) = self.compiled_local_dynamic_slot(level, binding.name.as_bytes())
        {
            binding.owner = VarTableOwner::CompiledLocal { level, slot };
        }
        Ok(binding)
    }

    fn ensure_native_original_root_entry(
        &mut self,
        root: &Value,
        reported: (&[u8], Option<&[u8]>),
        binding: &VarBinding,
        protocol: tcl_syntax::native_variable_name::NativeVariableNameProtocol,
        purpose: NativeVariableNameLookupPurpose,
    ) -> Result<ResolvedVar, Completion<Value>> {
        let (root_bytes, element) = reported;
        let create = purpose.creates_entries();
        let hash_birth = self.raw_variable_binding(binding).is_none()
            && !matches!(binding.owner, VarTableOwner::CompiledLocal { .. });
        let mut resolved = self
            .resolve_binding_var(binding.clone(), None, &mut HashSet::new())
            .ok_or_else(|| {
                self.refuse_host_command("actual original variable receiver unavailable".into())
            })?;
        if create && resolved.id.is_none() {
            resolved.id = self.ensure_target_var_at_binding(&resolved.binding, None);
            resolved.base_id = resolved.id;
        }
        if create
            && hash_birth
            && resolved.id.is_some()
            && protocol.version() >= tcl_dialect::TclVersion::V8_5
        {
            let key = if binding.name.as_bytes() == root_bytes {
                root.clone()
            } else {
                Value::new_native_string_bytes(binding.name.as_bytes())
            };
            if let Some(table) = self.var_table_mut(binding.owner) {
                table.retain_original_jim_key(&binding.name, key);
            }
        }
        if create
            && hash_birth
            && let Some(id) = resolved.id
        {
            self.var_arena.record_native_entry(id, binding.clone());
        }
        // Only a successful simple-variable lookup converts the original root.
        if resolved.id.is_some() {
            self.install_selected_original_variable_cache(root, binding)?;
        }
        if create
            && !purpose.creates_element_entries()
            && element.is_some()
            && let Some(base) = resolved.id
            && matches!(
                self.var_arena.get(base).map(crate::vars::VarCell::state),
                Some(super::Local::Undefined)
            )
        {
            self.var_arena
                .replace_state(base, super::Local::Array(super::VarTable::new()));
        }
        Ok(resolved)
    }

    fn cached_original_variable_parts(
        &mut self,
        original: &Value,
        protocol: tcl_syntax::native_variable_name::NativeVariableNameProtocol,
        explicit_element: Option<&[u8]>,
    ) -> Result<CachedOriginalVariableParts, Completion<Value>> {
        let parts = explicit_element
            .is_none()
            .then(|| {
                original.with_native_parsed_variable(|cache| {
                    if cache.protocol != protocol {
                        return None;
                    }
                    cache.array.as_ref().map(|(root, element)| {
                        (|| {
                            let element = match element {
                                NativeParsedVariableElement::Bytes(bytes) => {
                                    Rc::<[u8]>::from(tcl_core_types::c_string_extent(bytes))
                                }
                                NativeParsedVariableElement::Object(element) => element
                                    .native_string_bytes(
                                        tcl_syntax::native_string::NativeStringProtocol::C(
                                            protocol.version(),
                                        ),
                                    )?,
                            };
                            Ok::<_, tcl_syntax::native_string::NativeStringUnavailable>((
                                root.native_lifetime_lease(),
                                element,
                            ))
                        })()
                    })
                })
            })
            .flatten()
            .flatten()
            .transpose()
            .map_err(|error| self.refuse_host_command(error.to_string()))?;
        Ok(parts)
    }

    fn prepare_native_original_variable_parts(
        &mut self,
        original: &Value,
        purpose: NativeVariableNameLookupPurpose,
        namespace: Option<tcl_core_types::NsId>,
        explicit_element: Option<&[u8]>,
    ) -> Result<PreparedOriginalVariable, Completion<Value>> {
        self.retire_released_native_alias_entries();
        let dialect = self.actual_native_invocation_dialect();
        let protocol = self
            .native_c_variable_name_protocol()
            .expect("actual C variable issuer");
        if !protocol.cache_precedes_string_getter() {
            self.native_name_operand_bytes(original)
                .map_err(|error| self.refuse_host_command(error.to_string()))?;
        }
        let mut parts =
            self.cached_original_variable_parts(original, protocol, explicit_element)?;
        let parsed_scalar = original
            .with_native_parsed_variable(|cache| {
                cache.protocol == protocol && cache.array.is_none()
            })
            .unwrap_or(false);
        if parts.is_none()
            && explicit_element.is_none()
            && !parsed_scalar
            && original.with_native_local_variable(|_| ()).is_none()
            && self.cache_native_original_array_name(original, protocol, dialect)?
        {
            return self.prepare_native_original_variable_in(original, purpose, namespace);
        }
        let (root, element) = parts.take().map_or_else(
            || {
                (
                    original.native_lifetime_lease(),
                    explicit_element.map(Rc::<[u8]>::from),
                )
            },
            |(root, element)| (root, Some(element)),
        );
        let cached = namespace
            .is_none()
            .then(|| self.native_original_local_binding(root.value()))
            .flatten();
        let root_bytes: Rc<[u8]> = match &cached {
            Some(binding) if protocol.cache_precedes_string_getter() => {
                Rc::from(binding.name.as_bytes())
            }
            _ => self
                .native_name_operand_bytes(root.value())
                .map_err(|error| self.refuse_host_command(error.to_string()))?,
        };
        let binding = self.select_native_original_root_binding(
            root.value(),
            (&root_bytes, element.as_deref()),
            cached,
            protocol,
            namespace,
            purpose,
        )?;
        let resolved = self.ensure_native_original_root_entry(
            root.value(),
            (&root_bytes, element.as_deref()),
            &binding,
            protocol,
            purpose,
        )?;
        let resolved = if let Some(element) = &element {
            let selected = self
                .name_policy_protocol()
                .expect("selected native names")
                .recipe()
                .separate_variable_input(&root_bytes, Some(element));
            self.resolve_binding_var(
                resolved.binding,
                selected
                    .element()
                    .map(|part| tcl_core_types::NameBytes::from(part.selected())),
                &mut HashSet::new(),
            )
            .ok_or_else(|| {
                self.refuse_host_command("actual original array receiver unavailable".into())
            })?
        } else {
            resolved
        };
        Ok((root, root_bytes, element, resolved))
    }

    fn original_c_parsed_element_key(original: &Value) -> Option<Value> {
        original
            .with_native_parsed_variable(|cache| {
                let (_, element) = cache.array.as_ref()?;
                match cache.protocol.parsed_element_table_key(element)? {
                    tcl_syntax::native_variable_name::NativeElementTableKey::FreshString(bytes) => {
                        Some(Value::new_native_string_bytes(bytes.to_vec()))
                    }
                    tcl_syntax::native_variable_name::NativeElementTableKey::Original(original) => {
                        Some(original.clone())
                    }
                }
            })
            .flatten()
    }

    pub(super) fn link_original_c_namespace_variable(
        &mut self,
        original: &Value,
        namespace: tcl_core_types::NsId,
        local: &VarBinding,
        declare: bool,
    ) -> Result<(), Completion<Value>> {
        let (_root, bytes, _element, resolved) = self.prepare_native_original_variable_in(
            original,
            NativeVariableNameLookupPurpose::Link,
            Some(namespace),
        )?;
        let target = self.settle_original_c_link_target(original, &resolved)?;
        if self
            .native_c_variable_name_protocol()
            .is_some_and(|protocol| protocol.version() >= tcl_dialect::TclVersion::V8_5)
            && let Some((array, key)) = self
                .var_arena
                .element_parent(target)
                .map(|(array, key)| (array, key.clone()))
            && let Some(original_key) = Self::original_c_parsed_element_key(original)
        {
            self.var_arena
                .retain_original_array_key(array, &key, original_key);
        }
        if declare {
            self.var_arena.mark_namespace_declared(target);
        }
        let result = self
            .bind_variable_link(local, super::VariableLinkTarget::Cell(target))
            .map_err(|error| {
                crate::command::upvar_link_error_bytes(error, &bytes, local.name.as_bytes())
            });
        self.retire_released_native_alias_entries();
        result
    }

    pub(crate) fn define_original_c_namespace_variable(
        &mut self,
        original: &Value,
        value: Option<&Value>,
    ) -> Result<(), Completion<Value>> {
        // The command obtains its spelling before namespace-only cache lookup.
        let spelling = self
            .native_name_operand_bytes(original)
            .map_err(|error| self.refuse_host_command(error.to_string()))?;
        let namespace = self.current_ns_id();
        let (_root, bytes, element, resolved) = self.prepare_native_original_variable_in(
            original,
            NativeVariableNameLookupPurpose::Define,
            Some(namespace),
        )?;
        if element.is_some() {
            return Err(self.variable_access_error_input(
                "define",
                tcl_syntax::naming::NativeVariableInputForm::Combined(&spelling),
                "name refers to an element in an array",
            ));
        }
        let target = resolved.id.ok_or_else(|| {
            self.refuse_host_command("namespace declaration cell unavailable".into())
        })?;
        self.var_arena.mark_namespace_declared(target);
        if let Some(value) = value {
            let captured = self.capture_selected_update(&bytes, None, &resolved)?;
            self.with_variable_operation(&captured.cell, |vm| {
                vm.store_captured_update_with_original(
                    &bytes,
                    None,
                    &captured,
                    value.clone(),
                    Some(original),
                )
            })?;
        }
        if self.frame_owns_local_variables(self.current_level()) {
            let local = tcl_syntax::naming::variable_local_name_bytes(
                self.name_policy_protocol()
                    .expect("actual C declaration names")
                    .recipe(),
                &spelling,
            );
            let qualified = self
                .native_c_variable_name_protocol()
                .expect("actual C declaration")
                .alias_local_input(&spelling)
                .qualification()
                != tcl_syntax::naming::NativeNameQualification::Unqualified;
            let local = if qualified {
                Value::new_native_string_bytes(local)
            } else {
                original.native_lifetime_lease().into_value()
            };
            // ObjMakeUpvar resolves the original target again after setter callbacks.
            let (_root, bytes, _element, other) = self.prepare_native_original_variable_in(
                original,
                NativeVariableNameLookupPurpose::Link,
                Some(namespace),
            )?;
            let target = other.id.ok_or_else(|| {
                self.refuse_host_command("namespace alias cell unavailable".into())
            })?;
            self.bind_original_c_alias_local(&local, target, false, &bytes)?;
        }
        self.retire_released_native_alias_entries();
        Ok(())
    }

    /// The local side of `ObjMakeUpvar` is a simple scalar lookup. It retains
    /// an actual hash key at birth, without changing the original name primary.
    fn present_original_c_alias_error(&mut self, failure: Completion<Value>) -> Completion<Value> {
        if self.refused_completion().is_some() {
            return failure;
        }
        let protocol = self
            .native_c_variable_name_protocol()
            .expect("actual C alias diagnostic");
        let code = crate::command::opt_get(&failure.options, "-errorcode")
            .expect("alias failure owns its native error tuple");
        crate::command::completion_from_cmd_error(
            self,
            tcl_cmd_core::CmdError::from_byte_details(tcl_cmd_core::CmdErrorDetails {
                message: failure.result.string_bytes().to_vec(),
                string_result: protocol.diagnostic_string_protocol(),
                error_code: tcl_cmd_core::CmdErrorCodeUpdate::Set(code.string_bytes().to_vec()),
                error_info: None,
                error_line: None,
                primitive_getter: None,
            }),
        )
    }

    fn bind_original_c_alias_local(
        &mut self,
        original: &Value,
        target: tcl_core_types::VarId,
        owner_is_proc: bool,
        other: &[u8],
    ) -> Result<(), Completion<Value>> {
        match self.bind_original_c_alias_local_inner(original, target, owner_is_proc, other) {
            Ok(()) => Ok(()),
            Err(failure) => Err(self.present_original_c_alias_error(failure)),
        }
    }

    fn bind_original_c_alias_local_inner(
        &mut self,
        original: &Value,
        target: tcl_core_types::VarId,
        owner_is_proc: bool,
        other: &[u8],
    ) -> Result<(), Completion<Value>> {
        let protocol = self
            .native_c_variable_name_protocol()
            .expect("actual C alias local");
        let bytes = self
            .native_name_operand_bytes(original)
            .map_err(|error| self.refuse_host_command(error.to_string()))?;
        let input = protocol.alias_local_input(&bytes);
        let qualified =
            input.qualification() != tcl_syntax::naming::NativeNameQualification::Unqualified;
        let namespace = qualified || !self.frame_owns_local_variables(self.current_level());
        let failure = |error| {
            crate::command::upvar_link_error_bytes(
                error,
                other,
                tcl_core_types::c_string_extent(&bytes),
            )
        };
        if owner_is_proc && namespace {
            return Err(failure(super::UpvarLinkError::Inverted));
        }
        if protocol.alias_local_is_element(&bytes) {
            return Err(failure(super::UpvarLinkError::LocalElement));
        }
        let binding = if namespace {
            self.namespace_var_binding_from_token_bytes(self.current_ns_id(), input.selected())
                .ok_or_else(|| failure(super::UpvarLinkError::LocalNamespace))?
        } else {
            let level = self.current_level();
            VarBinding {
                owner: self
                    .compiled_local_dynamic_slot(level, input.selected())
                    .map_or(VarTableOwner::Frame(level), |slot| {
                        VarTableOwner::CompiledLocal { level, slot }
                    }),
                name: tcl_core_types::NameBytes::from(input.selected()),
            }
        };
        let birth = self.raw_variable_binding(&binding).is_none();
        if birth {
            self.bind_new_var(&binding, crate::vars::VarState::Undefined)
                .ok_or_else(|| {
                    self.refuse_host_command("original alias local cell unavailable".into())
                })?;
            if protocol.version() >= tcl_dialect::TclVersion::V8_5
                && !matches!(binding.owner, VarTableOwner::CompiledLocal { .. })
            {
                let key = if qualified {
                    Value::new_native_string_bytes(binding.name.as_bytes())
                } else {
                    original.clone()
                };
                self.var_table_mut(binding.owner)
                    .ok_or_else(|| failure(super::UpvarLinkError::LocalNamespace))?
                    .retain_original_native_key(&binding.name, key);
            }
        }
        self.bind_variable_link(&binding, super::VariableLinkTarget::Cell(target))
            .map_err(failure)?;
        self.retire_released_native_alias_entries();
        Ok(())
    }

    /// A resolved undefined target is a genuine link cell. Array access to an
    /// actual scalar is instead the native guest lookup failure, not missing
    /// host receiver authority.
    fn settle_original_c_link_target(
        &mut self,
        original: &Value,
        resolved: &ResolvedVar,
    ) -> Result<tcl_core_types::VarId, Completion<Value>> {
        if let Some(target) = resolved.id {
            return Ok(target);
        }
        if resolved.elem.is_some()
            && resolved
                .base_id
                .and_then(|id| self.var_arena.get(id))
                .is_some_and(|cell| matches!(cell.state(), super::Local::Scalar(_)))
        {
            let bytes = self
                .native_name_operand_bytes(original)
                .map_err(|error| self.refuse_host_command(error.to_string()))?;
            return Err(self.variable_access_error_input(
                "access",
                tcl_syntax::naming::NativeVariableInputForm::Combined(&bytes),
                "variable isn't array",
            ));
        }
        self.ensure_target_var_at_binding(&resolved.binding, resolved.elem.as_deref())
            .ok_or_else(|| {
                self.refuse_host_command("original alias target cell unavailable".into())
            })
    }

    fn original_c_upvar_target(
        &mut self,
        original: &Value,
        target_level: usize,
        namespace: Option<tcl_core_types::NsId>,
    ) -> Result<(tcl_core_types::VarId, bool, Vec<u8>), Completion<Value>> {
        let selected = self
            .select_execution_frame(target_level)
            .map_err(|error| self.refuse_host_command(error.clone()))?;
        let prepared = self.prepare_native_original_variable_in(
            original,
            NativeVariableNameLookupPurpose::Link,
            namespace,
        );
        self.restore_execution_frame(selected);
        let (_root, bytes, _element, resolved) = prepared?;
        let target = self.settle_original_c_link_target(original, &resolved)?;
        let root = self
            .var_arena
            .element_parent(target)
            .map_or(target, |(root, _)| root);
        let owner_is_proc = self
            .var_arena
            .get(root)
            .is_some_and(|cell| cell.namespace_owner().is_none())
            && matches!(resolved.binding.owner,
                VarTableOwner::Frame(level) | VarTableOwner::CompiledLocal { level, .. }
                    if self.frame_owns_local_variables(level));
        if self
            .native_c_variable_name_protocol()
            .is_some_and(|protocol| protocol.version() >= tcl_dialect::TclVersion::V8_5)
            && let Some((array, key)) = self
                .var_arena
                .element_parent(target)
                .map(|(array, key)| (array, key.clone()))
            && let Some(key_object) = Self::original_c_parsed_element_key(original)
        {
            self.var_arena
                .retain_original_array_key(array, &key, key_object);
        }
        Ok((target, owner_is_proc, bytes.to_vec()))
    }

    pub(crate) fn link_original_c_variable_objects(
        &mut self,
        original: &Value,
        target_level: usize,
        namespace: Option<tcl_core_types::NsId>,
        local: &Value,
    ) -> Result<(), Completion<Value>> {
        let (target, owner_is_proc, bytes) =
            self.original_c_upvar_target(original, target_level, namespace)?;
        self.bind_original_c_alias_local(local, target, owner_is_proc, &bytes)
    }

    pub(super) fn link_original_c_compiled_upvar(
        &mut self,
        original: &Value,
        target_level: usize,
        local: &VarBinding,
    ) -> Result<(), Completion<Value>> {
        let (target, _, bytes) = self.original_c_upvar_target(original, target_level, None)?;
        if let Err(error) = self.bind_variable_link(local, super::VariableLinkTarget::Cell(target))
        {
            let failure =
                crate::command::upvar_link_error_bytes(error, &bytes, local.name.as_bytes());
            return Err(self.present_original_c_alias_error(failure));
        }
        self.retire_released_native_alias_entries();
        Ok(())
    }

    pub(super) fn unset_original_c_variable(
        &mut self,
        original: &Value,
        complain: bool,
    ) -> Result<(), Completion<Value>> {
        match self
            .prepare_native_original_variable(original, Self::original_unset_purpose(complain))
        {
            Ok((_root, root_bytes, element, resolved)) => {
                let observes = self
                    .trace_cell_from_resolved(&resolved)
                    .is_some_and(|cell| {
                        self.original_variable_trace_requires_name(
                            &cell,
                            element.is_some(),
                            "unset",
                        )
                    });
                let bytes = if element.is_some() || observes {
                    self.native_name_operand_bytes(original)
                        .map_err(|error| self.refuse_host_command(error.to_string()))?
                } else {
                    root_bytes
                };
                let result =
                    self.unset_selected_original_variable(&bytes, Some(&resolved), complain);
                if result.is_err() {
                    self.native_name_operand_bytes(original)
                        .map_err(|error| self.refuse_host_command(error.to_string()))?;
                }
                result
            }
            Err(error) if self.refused_completion().is_some() || complain => Err(error),
            Err(_) => Ok(()),
        }
    }

    pub(crate) fn unset_original_c_variable_parts(
        &mut self,
        original: &Value,
        element: &Value,
        complain: bool,
    ) -> Result<(), Completion<Value>> {
        let element_bytes = self
            .native_name_operand_bytes(element)
            .map_err(|error| self.refuse_host_command(error.to_string()))?;
        let selected = self.prepare_native_original_variable_parts(
            original,
            Self::original_unset_purpose(complain),
            None,
            Some(&element_bytes),
        );
        match selected {
            Ok((_root, root_bytes, _element, resolved)) => {
                // These bytes present the diagnostic/trace spelling only. The
                // receiver remains the original part1 cache and selected cell.
                let mut spelling = root_bytes.to_vec();
                spelling.push(b'(');
                spelling.extend_from_slice(&element_bytes);
                spelling.push(b')');
                self.unset_selected_original_variable(&spelling, Some(&resolved), complain)
            }
            Err(error) if self.refused_completion().is_some() || complain => Err(error),
            Err(_) => Ok(()),
        }
    }

    fn original_unset_purpose(complain: bool) -> NativeVariableNameLookupPurpose {
        if complain {
            NativeVariableNameLookupPurpose::Unset
        } else {
            NativeVariableNameLookupPurpose::QuietUnset
        }
    }

    pub(super) fn retire_released_native_alias_entries(&mut self) {
        for (binding, id) in self.var_arena.take_released_native_alias_entries() {
            let discard = self.var_arena.get(id).is_some_and(|cell| {
                matches!(cell.state(), crate::vars::VarState::Undefined)
                    && !cell.namespace_declared()
            }) && !self.var_arena.has_link_refs(id)
                && !self.var_arena.has_operation_refs(id)
                && !self.variable_observers.script_traces.contains_key(&id);
            if !discard {
                continue;
            }
            let removed = self.var_table_mut(binding.owner).and_then(|table| {
                (table.get(&binding.name) == Some(&id))
                    .then(|| table.remove(&binding.name))
                    .flatten()
            });
            if let Some(raw) = removed {
                self.var_arena.unbind(raw);
            }
        }
    }

    pub(super) fn original_variable_trace_requires_name(
        &self,
        cell: &super::VarTraceCell,
        element_access: bool,
        operation: &str,
    ) -> bool {
        if cell.id.is_some_and(|id| self.active_traces.contains(&id)) {
            return false;
        }
        let array_active = cell
            .array
            .is_some_and(|id| self.active_traces.contains(&id));
        self.variable_trace_groups(element_access, cell, array_active, None, operation)
            .into_iter()
            .any(|group| {
                let super::TraceGroup::Live(id) = group else {
                    return false;
                };
                self.variable_observers
                    .script_traces
                    .get(&id)
                    .is_some_and(|traces| {
                        traces
                            .iter()
                            .any(|trace| trace.ops.iter().any(|op| op == operation))
                    })
                    || (matches!(operation, "read" | "unset")
                        && self.variable_observers.native_error_cells.contains_key(&id))
                    || (matches!(operation, "read" | "write")
                        && self.variable_observers.precision_cell == Some(id))
            })
    }

    pub(super) fn read_original_c_variable(
        &mut self,
        original: &Value,
    ) -> Result<Value, Completion<Value>> {
        let (_root, bytes, element, resolved) =
            self.prepare_native_original_variable(original, NativeVariableNameLookupPurpose::Read)?;
        self.read_selected_variable_result_bytes_with_original(
            &bytes,
            element.as_deref(),
            resolved,
            Some(original),
        )
    }
    /// Native object access separates the fresh qualified lookup object from
    /// the retained clientData object used for `PtrGetVar` diagnostics and traces.
    pub(crate) fn read_original_c_property_variable(
        &mut self,
        lookup: &Value,
        reported: &Value,
    ) -> Result<Value, Completion<Value>> {
        let (_root, _bytes, element, resolved) =
            self.prepare_native_original_variable(lookup, NativeVariableNameLookupPurpose::Link)?;
        let bytes = self
            .native_name_operand_bytes(reported)
            .map_err(|error| self.refuse_host_command(error.to_string()))?;
        self.read_selected_variable_result_bytes_with_original(
            &bytes,
            element.as_deref(),
            resolved,
            Some(reported),
        )
    }

    /// Native `PropertySetter` uses the selected cell without publishing its value
    /// as the interpreter result; the enclosing configure operation resets it.
    pub(crate) fn store_original_c_property_variable(
        &mut self,
        lookup: &Value,
        reported: &Value,
        value: Value,
    ) -> Result<(), Completion<Value>> {
        let (_root, _bytes, element, resolved) =
            self.prepare_native_original_variable(lookup, NativeVariableNameLookupPurpose::Write)?;
        let bytes = self
            .native_name_operand_bytes(reported)
            .map_err(|error| self.refuse_host_command(error.to_string()))?;
        let captured =
            self.capture_selected_update_with_errors(&bytes, element.as_deref(), &resolved, true)?;
        self.with_variable_operation(&captured.cell, |vm| {
            vm.store_captured_update_with_original_and_errors(
                &bytes,
                element.as_deref(),
                &captured,
                value,
                Some(reported),
                true,
            )
            .map(|_| ())
        })
    }

    pub(crate) fn store_original_regex_variable(
        &mut self,
        original: &Value,
        value: Value,
    ) -> Result<Value, Completion<Value>> {
        let Some(protocol) = self.native_c_variable_name_protocol() else {
            return self.store_original_named_variable(original, value);
        };
        let purpose = protocol.regex_write_purpose();
        match self.store_original_c_variable_for(original, value, purpose) {
            Ok(value) => Ok(value),
            Err(failure)
                if !purpose.leaves_error_message() && self.refused_completion().is_none() =>
            {
                let bytes = self
                    .native_name_operand_bytes(original)
                    .map_err(|error| self.refuse_host_command(error.to_string()))?;
                let current = self
                    .with_native_interp_result(|value| value.native_lifetime_lease().into_value())
                    .map_err(|error| self.refuse_host_command(error.to_string()))?;
                let mut message = self
                    .native_name_operand_bytes(&current)
                    .map_err(|error| self.refuse_host_command(error.to_string()))?
                    .to_vec();
                message.extend_from_slice(b"couldn't set variable \"");
                message.extend_from_slice(tcl_core_types::c_string_extent(&bytes));
                message.push(b'"');
                let mut failure = failure;
                failure.result = Value::from_string_bytes(message);
                if protocol.diagnostic_string_protocol().is_some() {
                    let recipe = self
                        .actual_native_invocation_dialect()
                        .native_string_materialization(None)
                        .ok_or_else(|| {
                            self.refuse_host_command("actual variable-error String producer".into())
                        })?;
                    failure
                        .result
                        .retain_native_string_representation(recipe)
                        .map_err(|error| self.refuse_host_command(error.to_string()))?;
                }
                Err(failure)
            }
            Err(failure) => Err(failure),
        }
    }

    pub(super) fn store_original_c_variable(
        &mut self,
        original: &Value,
        value: Value,
    ) -> Result<Value, Completion<Value>> {
        self.store_original_c_variable_for(original, value, NativeVariableNameLookupPurpose::Write)
    }

    fn store_original_c_variable_for(
        &mut self,
        original: &Value,
        value: Value,
        purpose: NativeVariableNameLookupPurpose,
    ) -> Result<Value, Completion<Value>> {
        let (_root, bytes, element, resolved) =
            self.prepare_native_original_variable(original, purpose)?;
        let element_birth = resolved.elem.is_some() && resolved.id.is_none();
        let captured = self.capture_selected_update_with_errors(
            &bytes,
            element.as_deref(),
            &resolved,
            purpose.leaves_error_message(),
        )?;
        if element_birth
            && let (Some(array), Some(key)) = (captured.cell.array, captured.cell.elem.as_ref())
            && self
                .native_c_variable_name_protocol()
                .is_some_and(|protocol| protocol.version() >= tcl_dialect::TclVersion::V8_5)
        {
            let key_object = Self::original_c_parsed_element_key(original);
            if let Some(key_object) = key_object {
                self.var_arena
                    .retain_original_array_key(array, key, key_object);
            }
        }
        self.with_variable_operation(&captured.cell, |vm| {
            vm.store_captured_update_with_original_and_errors(
                &bytes,
                element.as_deref(),
                &captured,
                value,
                Some(original),
                purpose.leaves_error_message(),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use tcl_core_types::VarId;
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;

    fn actual(version: &str) -> Vm {
        let profile = tcl_registry::model::ingress::resolve_environment(version).unit_profile();
        let mut vm = Vm::new();
        assert!(vm.set_native_engine_profile(profile));
        vm.set_dialect_profile(profile);
        vm
    }

    #[test]
    fn original_unset_parts_keep_part1_cache_and_quiet_missing_lookup() {
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let mut vm = Vm::with_native_core(
                Box::new(std::io::sink()),
                Rc::new(crate::host_native::NativeHost::new()),
                profile,
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            vm.set_array_elem_bytes(b"a(k)", b"index", Value::string("ORIGINAL"))
                .unwrap();
            vm.set_array_elem_bytes(b"a", b"k", Value::string("SIBLING"))
                .unwrap();
            let root = Value::new_native_string_bytes(b"a(k)".as_slice());
            let index = Value::new_native_string_bytes(b"index".as_slice());
            vm.unset_original_c_variable_parts(&root, &index, true)
                .unwrap();
            assert!(
                vm.get_array_elem_bytes_from(vm.current_level(), b"a(k)", b"index")
                    .is_none(),
                "{engine}"
            );
            assert_eq!(
                vm.get_array_elem_bytes_from(vm.current_level(), b"a", b"k")
                    .unwrap()
                    .to_str()
                    .as_ref(),
                "SIBLING"
            );
            assert!(
                !root
                    .with_native_parsed_variable(|cache| cache.array.is_some())
                    .unwrap_or(false)
            );
            let missing = Value::new_native_string_bytes(b"::absent::x".as_slice());
            assert!(
                vm.unset_original_c_variable(&missing, false).is_ok(),
                "{engine}"
            );
            assert!(vm.refused_completion().is_none());
            assert!(
                vm.unset_original_c_variable(&missing, true).is_err(),
                "{engine}"
            );
        }
    }

    #[test]
    fn original_unset_execution_matches_85_native_completion_windows() {
        fn unhex(text: &str) -> Vec<u8> {
            text.as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect()
        }
        let mut compared = 0;
        for row in
            include_str!("../../../tcl-registry/tests/data/native_unset_compilation/windows.tsv")
                .lines()
                .skip(1)
        {
            let columns: Vec<_> = row.split('\t').collect();
            if columns[0] == "jim0.84" {
                continue;
            }
            let profile = tcl_dialect::DialectProfile::find(&format!("tcl{}", columns[0])).unwrap();
            let mut vm = Vm::with_native_core(
                Box::new(std::io::sink()),
                Rc::new(crate::host_native::NativeHost::new()),
                profile,
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            vm.set_compiler(Box::new(
                tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
            ));
            let body = String::from_utf8(unhex(columns[2])).unwrap();
            let source = format!(
                "proc f {{name other names}} {{\n{body}\n}}\nset ::seen BEFORE\nset code [catch {{f x y {{x y}}}} value]\nlist RESULT $code $value $::seen"
            );
            let completion = vm.try_eval_source(&source).unwrap();
            assert_eq!(
                completion.code,
                tcl_runtime_api::Code::Ok,
                "{}:{}",
                columns[0],
                columns[1]
            );
            let expected = String::from_utf8(unhex(columns[5])).unwrap();
            assert_eq!(
                completion.result.to_str().as_ref(),
                expected,
                "{}:{}",
                columns[0],
                columns[1]
            );
            assert!(
                vm.refused_completion().is_none(),
                "{}:{}",
                columns[0],
                columns[1]
            );
            compared += 1;
        }
        assert_eq!(compared, 85);
    }

    #[test]
    fn original_unset_options_match_66_native_results() {
        fn decode(hex: &str) -> Vec<u8> {
            hex.as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect()
        }
        let mut compared = 0;
        for (engine, rows) in [
            (
                "tcl8.4",
                include_str!("../../../tcl-registry/tests/data/native_unset_options/8.4.tsv"),
            ),
            (
                "tcl8.5",
                include_str!("../../../tcl-registry/tests/data/native_unset_options/8.5.tsv"),
            ),
            (
                "tcl8.6",
                include_str!("../../../tcl-registry/tests/data/native_unset_options/8.6.tsv"),
            ),
            (
                "tcl9.0",
                include_str!("../../../tcl-registry/tests/data/native_unset_options/9.0.tsv"),
            ),
            (
                "tcl9.1",
                include_str!("../../../tcl-registry/tests/data/native_unset_options/9.1.tsv"),
            ),
            (
                "jim",
                include_str!("../../../tcl-registry/tests/data/native_unset_options/jim.tsv"),
            ),
        ] {
            for row in rows.lines() {
                let columns: Vec<_> = row.split('\t').collect();
                let source = String::from_utf8(decode(columns[1])).unwrap();
                // Match the native probe's same-interpreter binary observer.
                let observed = format!(
                    "set c [catch {{{source}}} m];binary scan $m H* h;set observed \"$c\\t$h\""
                );
                let expected = format!("{}\t{}", columns[2], columns[3]);
                let profile =
                    tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
                let mut vm = crate::native_fixture::interpreter(profile);
                if engine == "jim" {
                    let empty = vm.try_eval_source("unset").unwrap();
                    assert_eq!(empty.code, tcl_runtime_api::Code::Ok);
                    assert_eq!(empty.result.to_str().as_ref(), "");
                    assert!(vm.refused_completion().is_none());
                    // jimsh's binary observer is a distribution extension;
                    // it is independent of the native core issuer under test.
                    crate::cmd_binary::register(&mut vm);
                }
                let completion = vm.try_eval_source(&observed).unwrap_or_else(|error| {
                    panic!("{engine}/{} original unset observer: {error:?}", columns[0])
                });
                assert_eq!(
                    completion.code,
                    tcl_runtime_api::Code::Ok,
                    "{engine}/{}",
                    columns[0]
                );
                assert_eq!(
                    completion.result.to_str().as_ref(),
                    expected,
                    "{engine}/{}",
                    columns[0]
                );
                assert!(vm.refused_completion().is_none(), "{engine}/{}", columns[0]);
                compared += 1;
            }
        }
        assert_eq!(compared, 66);
    }

    #[test]
    fn original_unset_traces_and_operand_order_match_36_native_results() {
        fn decode(hex: &str) -> Vec<u8> {
            hex.as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect()
        }
        let mut compared = 0;
        for (engine, rows) in [
            (
                "tcl8.4",
                include_str!("../../../../runtime/rust/tests/data/native_unset_execution/8.4.tsv"),
            ),
            (
                "tcl8.5",
                include_str!("../../../../runtime/rust/tests/data/native_unset_execution/8.5.tsv"),
            ),
            (
                "tcl8.6",
                include_str!("../../../../runtime/rust/tests/data/native_unset_execution/8.6.tsv"),
            ),
            (
                "tcl9.0",
                include_str!("../../../../runtime/rust/tests/data/native_unset_execution/9.0.tsv"),
            ),
            (
                "tcl9.1",
                include_str!("../../../../runtime/rust/tests/data/native_unset_execution/9.1.tsv"),
            ),
            (
                "jim",
                include_str!("../../../../runtime/rust/tests/data/native_unset_execution/jim.tsv"),
            ),
        ] {
            for row in rows.lines() {
                let columns: Vec<_> = row.split('\t').collect();
                let profile =
                    tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
                let mut vm = crate::native_fixture::interpreter(profile);
                let completion = vm.try_eval_source_bytes(&decode(columns[1])).unwrap();
                let code = completion.code;
                assert_eq!(
                    code.as_int(),
                    columns[2].parse::<i64>().unwrap(),
                    "{engine}/{}",
                    columns[0]
                );
                assert_eq!(
                    tcl_syntax::value::ValueOps::native_string_bytes(&mut vm, &completion.result)
                        .unwrap()
                        .to_vec(),
                    decode(columns[3]),
                    "{engine}/{}",
                    columns[0]
                );
                assert!(vm.refused_completion().is_none(), "{engine}/{}", columns[0]);
                compared += 1;
            }
        }
        assert_eq!(compared, 36);
    }

    thread_local! {
        static ALIAS_SIMPLE_ROWS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
        static ALIAS_CANONICAL_ROWS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    fn alias_canonical_row(stage: &str, case: usize, original: &Value) {
        let canonical = match original.native_object_snapshot().cache {
            Cache::List { canonical, .. } => i32::from(canonical),
            _ => -1,
        };
        ALIAS_CANONICAL_ROWS.with(|rows| {
            rows.borrow_mut().push(format!("{stage}{case}|{canonical}"));
        });
    }

    fn alias_simple_probe(vm: &mut Vm, _argv: &[Value]) -> Completion<Value> {
        for (case, name) in [b"v".as_slice(), b"d", b"n\0(k)"].into_iter().enumerate() {
            let local = if case == 2 {
                Value::new_native_string_bytes(name)
            } else {
                Value::list(vec![Value::new_native_string_bytes(name)])
            };
            let kind = if case == 2 { "NULL" } else { "list" };
            ALIAS_SIMPLE_ROWS.with(|rows| {
                rows.borrow_mut().push(format!(
                    "before{case}|{kind}|{}",
                    local.native_object_reference_count(),
                ));
            });
            alias_canonical_row("before", case, &local);
            let target = Value::new_native_string_bytes(b"x".as_slice());
            if let Err(error) = vm.link_upvar_original(0, &target, &local) {
                return error;
            }
            alias_canonical_row("after", case, &local);
            ALIAS_SIMPLE_ROWS.with(|rows| {
                rows.borrow_mut().push(format!(
                    "after{case}|0|{kind}|{}",
                    local.native_object_reference_count(),
                ));
            });
        }
        crate::interp::ok(Value::empty())
    }

    thread_local! {
        static GLOBAL_CACHE_ROWS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
    }
    fn global_cache_probe(vm: &mut Vm, argv: &[Value]) -> Completion<Value> {
        let value = match vm.read_original_c_variable(&argv[0]) {
            Ok(value) => value,
            Err(error) => return error,
        };
        let bytes = vm.native_name_operand_bytes(&value).unwrap();
        GLOBAL_CACHE_ROWS.with(|rows| {
            rows.borrow_mut().push(format!(
                "{}|{}",
                argv[0].native_object_type_name(),
                String::from_utf8(bytes.to_vec()).unwrap(),
            ));
        });
        crate::interp::ok(Value::empty())
    }
    #[test]
    fn dynamic_global_uses_original_compiler_token_tail_and_name_cache() {
        for (engine, expected) in [
            (
                "tcl8.4",
                include_str!("../../tests/data/native_global_cache_token/8.4.20.txt"),
            ),
            (
                "tcl8.5",
                include_str!("../../tests/data/native_global_cache_token/8.5.19.txt"),
            ),
            (
                "tcl8.6",
                include_str!("../../tests/data/native_global_cache_token/8.6.18.txt"),
            ),
            (
                "tcl9.0",
                include_str!("../../tests/data/native_global_cache_token/9.0.4.txt"),
            ),
            (
                "tcl9.1",
                include_str!("../../tests/data/native_global_cache_token/9.1.0.txt"),
            ),
        ] {
            GLOBAL_CACHE_ROWS.with(|rows| rows.borrow_mut().clear());
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let mut vm = Vm::with_native_core(
                Box::new(std::io::sink()),
                Rc::new(crate::host_native::NativeHost::new()),
                profile,
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            vm.set_compiler(Box::new(
                tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
            ));
            vm.register("probe", global_cache_probe);
            vm.set_var_bytes(b"name", Value::new_native_string_bytes(b"v".as_slice()))
                .unwrap();
            for (label, source) in [
                (
                    "simple",
                    "set v GLOBAL;proc p {} {set v LOCAL;probe $::name;unset v;global $::name;probe $::name;return $v};p",
                ),
                (
                    "qualified",
                    "namespace eval N {variable v QUALIFIED};set name ::N::v;p",
                ),
            ] {
                let completion = vm.try_eval_source(source).unwrap();
                GLOBAL_CACHE_ROWS.with(|rows| {
                    rows.borrow_mut().push(format!(
                        "{label}|{}|{}",
                        completion.code.as_int(),
                        String::from_utf8(completion.result.string_bytes().to_vec()).unwrap(),
                    ));
                });
                assert!(vm.refused_completion().is_none(), "{engine}");
            }
            let rows = GLOBAL_CACHE_ROWS.with(|rows| rows.borrow().join("\n") + "\n");
            assert_eq!(rows, expected, "{engine}");
        }
    }

    #[test]
    fn alias_local_simple_lookup_matches_all_15_native_primary_and_key_owners() {
        for (engine, expected) in [
            (
                "tcl8.4",
                include_str!("../../tests/data/native_alias_simple/8.4.20.txt"),
            ),
            (
                "tcl8.5",
                include_str!("../../tests/data/native_alias_simple/8.5.19.txt"),
            ),
            (
                "tcl8.6",
                include_str!("../../tests/data/native_alias_simple/8.6.18.txt"),
            ),
            (
                "tcl9.0",
                include_str!("../../tests/data/native_alias_simple/9.0.4.txt"),
            ),
            (
                "tcl9.1",
                include_str!("../../tests/data/native_alias_simple/9.1.0.txt"),
            ),
        ] {
            ALIAS_SIMPLE_ROWS.with(|rows| rows.borrow_mut().clear());
            ALIAS_CANONICAL_ROWS.with(|rows| rows.borrow_mut().clear());
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let mut vm = Vm::with_native_core(
                Box::new(std::io::sink()),
                Rc::new(crate::host_native::NativeHost::new()),
                profile,
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            vm.set_compiler(Box::new(
                tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
            ));
            vm.register("alias_probe", alias_simple_probe);
            let completion = vm
                .try_eval_source("set x X; proc p {} {alias_probe; return $v}; p")
                .unwrap();
            assert!(completion.code.is_ok(), "{engine}: {:?}", completion.result);
            assert_eq!(completion.result.string_bytes().as_ref(), b"X");
            ALIAS_SIMPLE_ROWS.with(|rows| rows.borrow_mut().push("completion|0|X".into()));
            let rows = ALIAS_SIMPLE_ROWS.with(|rows| rows.borrow().join("\n") + "\n");
            assert_eq!(rows, expected, "{engine}");
            let canonical = match engine {
                "tcl8.4" => {
                    include_str!("../../tests/data/native_alias_simple/canonical/8.4.20.txt")
                }
                "tcl8.5" => {
                    include_str!("../../tests/data/native_alias_simple/canonical/8.5.19.txt")
                }
                "tcl8.6" => {
                    include_str!("../../tests/data/native_alias_simple/canonical/8.6.18.txt")
                }
                "tcl9.0" => {
                    include_str!("../../tests/data/native_alias_simple/canonical/9.0.4.txt")
                }
                "tcl9.1" => {
                    include_str!("../../tests/data/native_alias_simple/canonical/9.1.0.txt")
                }
                _ => unreachable!(),
            };
            let rows = ALIAS_CANONICAL_ROWS.with(|rows| rows.borrow().join("\n") + "\n");
            assert_eq!(rows, canonical, "{engine}");
            assert!(vm.refused_completion().is_none());
        }
    }

    #[test]
    fn generic_namespace_declarations_match_all_25_native_execution_results() {
        struct Output(Rc<std::cell::RefCell<Vec<u8>>>);
        impl std::io::Write for Output {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0.borrow_mut().extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let source = include_str!("../../tests/data/native_namespace_handler_order/source.tcl");
        for (engine, expected) in [
            (
                "tcl8.4",
                include_bytes!("../../tests/data/native_namespace_handler_order/tcl8.4.txt")
                    .as_slice(),
            ),
            (
                "tcl8.5",
                include_bytes!("../../tests/data/native_namespace_handler_order/tcl8.5.txt")
                    .as_slice(),
            ),
            (
                "tcl8.6",
                include_bytes!("../../tests/data/native_namespace_handler_order/tcl8.6.txt")
                    .as_slice(),
            ),
            (
                "tcl9.0",
                include_bytes!("../../tests/data/native_namespace_handler_order/tcl9.0.txt")
                    .as_slice(),
            ),
            (
                "tcl9.1",
                include_bytes!("../../tests/data/native_namespace_handler_order/tcl9.1.txt")
                    .as_slice(),
            ),
        ] {
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let bytes = Rc::new(std::cell::RefCell::new(Vec::new()));
            let mut vm = Vm::with_native_core(
                Box::new(Output(Rc::clone(&bytes))),
                Rc::new(crate::host_native::NativeHost::new()),
                profile,
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            vm.set_compiler(Box::new(
                tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
            ));
            let completion = vm.try_eval_source(source).unwrap();
            assert!(completion.code.is_ok(), "{engine}: {:?}", completion.result);
            assert_eq!(bytes.borrow().as_slice(), expected, "{engine}");
            assert!(vm.refused_completion().is_none());
        }
    }

    fn namespace_cache_probe(vm: &mut Vm, argv: &[Value]) -> Completion<Value> {
        let original = &argv[0];
        assert_eq!(
            vm.read_original_c_variable(original)
                .unwrap()
                .string_bytes()
                .as_ref(),
            b"LOCAL"
        );
        assert!(original.with_native_local_variable(|_| ()).is_some());
        let slot = vm
            .compiled_local_dynamic_slot(vm.current_level(), b"alias")
            .unwrap();
        let before = vm.native_original_local_binding(original).unwrap();
        let before_cell = vm.raw_variable_binding(&before).unwrap();
        let namespace = vm.current_ns_id();
        vm.link_compiled_namespace_original(slot, namespace, original, true)
            .unwrap();
        assert!(original.with_native_local_variable(|_| ()).is_none());
        assert!(
            original
                .with_native_parsed_variable(|cache| cache.array.is_none())
                .unwrap()
        );
        assert_eq!(vm.raw_variable_binding(&before), Some(before_cell));
        assert_eq!(
            vm.get_var_from_bytes(vm.current_level(), b"v")
                .unwrap()
                .string_bytes()
                .as_ref(),
            b"LOCAL"
        );
        assert_eq!(
            vm.read_compiled_variable_result(slot, None)
                .unwrap()
                .string_bytes()
                .as_ref(),
            b"GLOBAL"
        );
        super::super::ok(Value::new_native_string_bytes(b"".as_slice()))
    }

    #[test]
    fn namespace_opcodes_bypass_original_local_cache_and_retain_the_namespace_cell() {
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let mut vm = Vm::with_native_core(
                Box::new(Vec::<u8>::new()),
                Rc::new(crate::host_native::NativeHost::new()),
                profile,
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            vm.set_compiler(Box::new(
                tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
            ));
            vm.register("namespace_cache_probe", namespace_cache_probe);
            let original = Value::new_native_string_bytes(b"v".as_slice());
            vm.set_var("name", original.clone()).unwrap();
            let completion = vm.try_eval_source(
                "set v GLOBAL; proc p {} {set v LOCAL; namespace_cache_probe $::name; return $alias}; p",
            ).unwrap();
            assert!(completion.code.is_ok(), "{engine}: {:?}", completion.result);
            assert_eq!(completion.result.string_bytes().as_ref(), b"GLOBAL");
            assert!(
                original
                    .with_native_parsed_variable(|cache| cache.array.is_none())
                    .unwrap()
            );
            assert!(vm.refused_completion().is_none());
        }
    }

    #[test]
    fn namespace_original_links_preserve_array_parts_and_reject_defined_aliases() {
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let mut vm = Vm::with_native_core(
                Box::new(Vec::<u8>::new()),
                Rc::new(crate::host_native::NativeHost::new()),
                profile,
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let original = Value::new_native_string_bytes(b"arr(k)".as_slice());
            let value = Value::new_native_string_bytes(b"VALUE".as_slice());
            vm.store_original_named_variable(&original, value.clone())
                .unwrap();
            vm.push_call_frame(Some("p".into()), Vec::new());
            vm.install_compiled_local_slots(&[tcl_core_types::NameBytes::from(
                b"alias".as_slice(),
            )])
            .unwrap();
            vm.link_compiled_namespace_original(0, tcl_core_types::ROOT_NS, &original, false)
                .unwrap();
            assert!(
                vm.read_compiled_variable_result(0, None)
                    .unwrap()
                    .is_same_object(&value)
            );
            assert!(
                original
                    .with_native_parsed_variable(|cache| cache.array.is_some())
                    .unwrap()
            );
            vm.pop_call_frame();
            vm.push_call_frame(Some("p".into()), Vec::new());
            vm.install_compiled_local_slots(&[tcl_core_types::NameBytes::from(
                b"alias".as_slice(),
            )])
            .unwrap();
            vm.bind_compiled_formal_slot(0, Value::string("LOCAL"))
                .unwrap();
            let failure = vm
                .link_compiled_namespace_original(0, tcl_core_types::ROOT_NS, &original, true)
                .unwrap_err();
            assert_eq!(failure.code, tcl_runtime_api::Code::Error);
            assert_eq!(
                vm.read_compiled_variable_result(0, None)
                    .unwrap()
                    .string_bytes()
                    .as_ref(),
                b"LOCAL"
            );
            assert!(vm.refused_completion().is_none());
            vm.pop_call_frame();
        }
    }

    thread_local! {
        static CACHE_LOOKUP_CASE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
        static CACHE_LOOKUP_CALLBACKS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
        static CACHE_LOOKUP_ROWS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
    }
    struct CacheLookupObserver;
    impl tcl_runtime_api::native_variable_trace::NativeVariableObserver<Vm> for CacheLookupObserver {
        type Error = tcl_cmd_core::CmdError;
        fn observe(
            &self,
            _vm: &mut Vm,
            _access: tcl_runtime_api::native_variable_trace::NativeVariableTraceAccess<'_>,
        ) -> Result<(), Self::Error> {
            CACHE_LOOKUP_CALLBACKS.with(|count| count.set(count.get() + 1));
            Ok(())
        }
    }
    fn cache_lookup_probe(vm: &mut Vm, argv: &[Value]) -> Completion<Value> {
        use tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation as Op;
        let original = &argv[0];
        let case = CACHE_LOOKUP_CASE.with(std::cell::Cell::get);
        let version = vm.native_c_variable_name_protocol().unwrap().version();
        vm.read_original_c_variable(original).unwrap();
        assert!(original.with_native_local_variable(|_| ()).is_some());
        let before = vm.native_original_local_binding(original).unwrap();
        let before_cell = vm.raw_variable_binding(&before).unwrap();
        if matches!(case, 2 | 3) {
            vm.add_native_variable_observer(
                original,
                &[Op::Read, Op::Write],
                Rc::new(CacheLookupObserver),
            )
            .unwrap();
        }
        if case == 4 {
            assert!(vm.unset_var_bytes(b"x"));
        }
        original.invalidate_native_string_for_test();
        let result = if matches!(case, 1 | 3) {
            vm.store_original_c_variable(
                original,
                Value::new_native_string_bytes(b"NEXT".as_slice()),
            )
        } else {
            vm.read_original_c_variable(original)
        };
        assert_eq!(vm.raw_variable_binding(&before), Some(before_cell));
        if version >= tcl_dialect::TclVersion::V8_5 && case >= 2 {
            assert!(result.is_err());
            assert!(
                vm.refused_completion().is_some(),
                "native updater abort must remain a host refusal"
            );
            assert!(original.resident_string_bytes().is_none());
            assert_eq!(CACHE_LOOKUP_CALLBACKS.with(std::cell::Cell::get), 0);
        } else {
            assert_eq!(result.is_err(), case == 4);
            assert_eq!(
                original.resident_string_bytes().is_some(),
                version == tcl_dialect::TclVersion::V8_4
            );
            assert_eq!(
                CACHE_LOOKUP_CALLBACKS.with(std::cell::Cell::get),
                usize::from(matches!(case, 2 | 3))
            );
        }
        let version_label = match version {
            tcl_dialect::TclVersion::V8_4 => "8.4.20",
            tcl_dialect::TclVersion::V8_5 => "8.5.19",
            tcl_dialect::TclVersion::V8_6 => "8.6.18",
            tcl_dialect::TclVersion::V9_0 => "9.0.4",
            tcl_dialect::TclVersion::V9_1 => "9.1.0",
        };
        let row = if vm.refused_completion().is_some() {
            format!("{version_label}|{case}|updater-unavailable")
        } else {
            format!(
                "{version_label}|{case}|{}|{}|localVarName|1",
                usize::from(result.is_err()),
                usize::from(original.resident_string_bytes().is_some())
            )
        };
        CACHE_LOOKUP_ROWS.with(|rows| rows.borrow_mut().push(row));
        if case != 4 {
            assert_eq!(
                vm.get_var_from_bytes(vm.current_level(), b"x")
                    .unwrap()
                    .string_bytes()
                    .as_ref(),
                if matches!(case, 1 | 3) {
                    b"NEXT".as_slice()
                } else {
                    b"VALUE".as_slice()
                }
            );
        }
        result.map_or_else(|error| error, crate::interp::ok)
    }

    #[test]
    fn original_local_cache_getter_order_matches_all_25_native_paths() {
        let expected = include_str!(
            "../../../tcl-syntax/tests/data/native_variable_name/cache_lookup/paths.txt"
        );
        assert_eq!(expected.lines().count(), 25);
        CACHE_LOOKUP_ROWS.with(|rows| rows.borrow_mut().clear());
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for case in 0..5 {
                CACHE_LOOKUP_CASE.with(|mode| mode.set(case));
                CACHE_LOOKUP_CALLBACKS.with(|count| count.set(0));
                let mut vm = actual(environment);
                vm.set_compiler(Box::new(
                    tcl_compiler::compile_service::BytecodeCompileService::for_profile(
                        vm.dialect_profile(),
                    ),
                ));
                vm.register("probe", cache_lookup_probe);
                let result = vm.try_eval_source("proc p {} {set x VALUE;probe x};p");
                if environment != "tcl8.4" && case >= 2 {
                    assert!(
                        result.is_err(),
                        "{environment}/{case}: typed updater refusal"
                    );
                } else {
                    let completion = result.unwrap();
                    assert_eq!(completion.code.is_ok(), case != 4, "{environment}/{case}");
                }
            }
        }
        CACHE_LOOKUP_ROWS.with(|rows| {
            assert_eq!(
                rows.borrow().as_slice(),
                expected.lines().collect::<Vec<_>>()
            );
        });
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().fold(String::new(), |mut output, byte| {
            use std::fmt::Write;
            write!(output, "{byte:02x}").expect("writing into a String");
            output
        })
    }
    fn observe(version: &str, case: &str, phase: &str, original: &Value) -> String {
        let snapshot = original.native_object_snapshot();
        let kind = match snapshot.cache {
            Cache::None => "none",
            Cache::Numeric(_) => "int",
            Cache::List { .. } => "list",
            Cache::ByteArray { .. } => "bytearray",
            Cache::ParsedVariableName { .. } => "parsedVarName",
            ref other => panic!("unexpected actual original cache: {other:?}"),
        };
        format!(
            "{version}|{case}|{phase}|{kind}|{}|{}|{}|{}",
            original.native_object_reference_count(),
            usize::from(original.native_primary_has_free_hook()),
            usize::from(snapshot.resident.is_some()),
            snapshot.resident.as_deref().map_or_else(String::new, hex)
        )
    }

    use tcl_runtime_api::native_variable_trace::{
        NativeVariableObserver, NativeVariableTraceAccess, NativeVariableTraceOperation,
    };
    struct ElementCallbackWindows {
        version: &'static str,
        key_k: crate::value::NativeObjectLifetimeLease,
        key_j: crate::value::NativeObjectLifetimeLease,
        old_k: VarId,
        old_j: VarId,
        object_table: bool,
        rows: Rc<RefCell<Vec<String>>>,
        sequence: Rc<RefCell<usize>>,
    }
    impl ElementCallbackWindows {
        fn record(&self, vm: &Vm, phase: &str) {
            let (dead_k, defined_k, refs_k) = vm
                .var_arena
                .observe_native_element(self.old_k, self.object_table);
            let (dead_j, defined_j, refs_j) = vm
                .var_arena
                .observe_native_element(self.old_j, self.object_table);
            let current_k = vm
                .resolve_var_from_bytes(b"arr(k)", 0)
                .and_then(|resolved| resolved.id)
                .is_some();
            let current_j = vm
                .resolve_var_from_bytes(b"arr(j)", 0)
                .and_then(|resolved| resolved.id)
                .is_some();
            let key_k = self.key_k.value().native_object_reference_count();
            let key_j = self.key_j.value().native_object_reference_count();
            self.rows.borrow_mut().push(format!("{}|window|{phase}|{key_k}|{key_j}|{dead_k}|{dead_j}|{defined_k}|{defined_j}|{refs_k}|{refs_j}|{}|{}", self.version, i32::from(current_k), i32::from(current_j)));
        }
    }
    impl NativeVariableObserver<Vm> for ElementCallbackWindows {
        type Error = tcl_cmd_core::CmdError;
        fn observe(
            &self,
            vm: &mut Vm,
            access: NativeVariableTraceAccess<'_>,
        ) -> Result<(), Self::Error> {
            let phase = if access.name2.is_empty() {
                "root".to_owned()
            } else {
                *self.sequence.borrow_mut() += 1;
                format!(
                    "element{}-{}",
                    *self.sequence.borrow(),
                    std::str::from_utf8(access.name2).unwrap()
                )
            };
            self.record(vm, &phase);
            // Raw C84 flags do not donate a value or an existence result.
            if !access.name2.is_empty() {
                assert!(
                    !vm.exists_var_bytes_from(if access.name2 == b"k" { b"ak" } else { b"aj" }, 0)
                );
            }
            Ok(())
        }
    }

    #[test]
    fn cpp_table_and_var_roles_match_all_25_original_callback_windows() {
        use std::cell::RefCell;
        use tcl_core_types::VarId;
        let rows = Rc::new(RefCell::new(Vec::new()));
        for (environment, version) in [
            ("tcl8.4", "8.4.20"),
            ("tcl8.5", "8.5.19"),
            ("tcl8.6", "8.6.18"),
            ("tcl9.0", "9.0.4"),
            ("tcl9.1", "9.1.0"),
        ] {
            let mut vm = actual(environment);
            let root = Value::new_native_string_bytes(b"arr".as_slice());
            let key_k = Value::new_native_string_bytes(b"k".as_slice());
            let key_j = Value::new_native_string_bytes(b"j".as_slice());
            let object_table = vm
                .native_c_variable_name_protocol()
                .unwrap()
                .element_table_retains_original();
            let (_, _, _, selected) = vm
                .prepare_native_original_variable(&root, NativeVariableNameLookupPurpose::Write)
                .unwrap();
            let array = selected.id.unwrap();
            let mut ids = Vec::new();
            for key in [&key_k, &key_j] {
                let id = vm
                    .ensure_target_var_at_binding(&selected.binding, Some(&key.string_bytes()))
                    .unwrap();
                vm.var_arena.replace_state(
                    id,
                    crate::vars::VarState::Scalar(Value::new_native_string_bytes(
                        b"ONE".as_slice(),
                    )),
                );
                if object_table {
                    assert!(vm.var_arena.retain_original_array_key(
                        array,
                        &tcl_core_types::NameBytes::from(key.string_bytes().as_ref()),
                        key.clone()
                    ));
                }
                ids.push(id);
            }
            let name_k = Value::new_native_string_bytes(b"arr(k)".as_slice());
            let name_j = Value::new_native_string_bytes(b"arr(j)".as_slice());
            vm.link_upvar_original(
                0,
                &name_k,
                &Value::new_native_string_bytes(b"ak".as_slice()),
            )
            .unwrap();
            vm.link_upvar_original(
                0,
                &name_j,
                &Value::new_native_string_bytes(b"aj".as_slice()),
            )
            .unwrap();
            let windows = Rc::new(ElementCallbackWindows {
                version,
                key_k: key_k.native_lifetime_lease(),
                key_j: key_j.native_lifetime_lease(),
                old_k: ids[0],
                old_j: ids[1],
                object_table,
                rows: Rc::clone(&rows),
                sequence: Rc::new(RefCell::new(0)),
            });
            for name in [&root, &name_k, &name_j] {
                vm.add_native_variable_observer(
                    name,
                    &[NativeVariableTraceOperation::Unset],
                    windows.clone(),
                )
                .unwrap();
            }
            windows.record(&vm, "before");
            assert!(vm.unset_var_bytes(b"arr"));
            windows.record(&vm, "after");
        }
        let expected = include_str!(
            "../../../tcl-syntax/tests/data/native_variable_name/element_alias_callbacks.txt"
        )
        .lines()
        .filter(|line| line.contains("|window|"))
        .collect::<Vec<_>>();
        assert_eq!(rows.borrow().len(), 25);
        assert_eq!(*rows.borrow(), expected);
    }

    #[test]
    fn original_upvar_and_unset_retain_actual_target_cache_and_entry() {
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm = actual(environment);
            let target = Value::new_native_string_bytes(b"target".as_slice());
            let local = Value::new_native_string_bytes(b"alias".as_slice());
            vm.link_upvar_original(0, &target, &local).unwrap();
            assert!(
                target
                    .with_native_parsed_variable(|cache| cache.array.is_none())
                    .unwrap()
            );
            let binding = vm.var_binding_from_bytes(b"target", 0).unwrap();
            let selected = vm.raw_variable_binding(&binding).unwrap();
            assert!(vm.var_arena.has_link_refs(selected));
            vm.store_original_named_variable(&local, Value::int(7))
                .unwrap();
            vm.unset_original_named_variable(&target, true).unwrap();
            assert_eq!(vm.raw_variable_binding(&binding), Some(selected));
            assert!(
                target
                    .with_native_parsed_variable(|cache| cache.array.is_none())
                    .unwrap()
            );
            vm.store_original_named_variable(&local, Value::int(8))
                .unwrap();
            assert_eq!(
                vm.read_original_named_variable(&target)
                    .unwrap()
                    .string_bytes()
                    .as_ref(),
                b"8"
            );
        }
    }

    #[test]
    fn search_free_slots_match_all_20_actual_c_windows() {
        use tcl_cmd_core::native_array_search::NativeArraySearchBackend;
        let mut rows = Vec::new();
        for (environment, version) in [
            ("tcl8.4", "8.4.20"),
            ("tcl8.5", "8.5.19"),
            ("tcl8.6", "8.6.18"),
            ("tcl9.0", "9.0.4"),
            ("tcl9.1", "9.1.0"),
        ] {
            let mut vm = actual(environment);
            let protocol = vm.array_search_protocol().unwrap();
            let name = Value::new_native_string_bytes(b"a(k)".as_slice());
            vm.store_original_named_variable(
                &name,
                Value::new_native_string_bytes(b"v".as_slice()),
            )
            .unwrap();
            let array = Value::new_native_string_bytes(b"a".as_slice());
            let handle = tcl_cmd_core::native_array_search::dispatch(
                &mut vm,
                "startsearch",
                std::slice::from_ref(&array),
                None,
            )
            .unwrap()
            .unwrap();
            let result = tcl_cmd_core::native_array_search::dispatch(
                &mut vm,
                "anymore",
                &[array, handle.clone()],
                None,
            )
            .unwrap()
            .unwrap();
            drop(result);
            let observe = |phase: &str, original: &Value| {
                format!(
                    "{version}|{phase}|{}|{}|{}",
                    if original
                        .native_array_search_cache_in(protocol)
                        .unwrap()
                        .is_some()
                    {
                        "array search"
                    } else if matches!(
                        original.native_object_snapshot().cache,
                        Cache::String { .. }
                    ) {
                        "string"
                    } else {
                        "none"
                    },
                    usize::from(original.native_primary_has_free_hook()),
                    usize::from(original.resident_string_bytes().is_some())
                )
            };
            rows.push(observe("converted", &handle));
            let copy = handle.duplicate_native_object_in(
                vm.actual_native_invocation_dialect()
                    .native_string_protocol()
                    .unwrap(),
            );
            rows.push(observe("duplicate", &copy));
            assert!(vm.read_original_named_variable(&handle).is_err());
            rows.push(observe("missing", &handle));
            assert!(vm.read_original_named_variable(&copy).is_err());
            rows.push(observe("duplicate-missing", &copy));
        }
        let expected = include_str!(
            "../../../tcl-syntax/tests/data/native_variable_name/array_search_free_slot.txt"
        )
        .lines()
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
        assert_eq!(rows.len(), 20);
        assert_eq!(rows, expected);
    }

    #[test]
    fn original_parsed_headers_match_all_150_actual_c_windows() {
        let mut rows = Vec::new();
        for (environment, version) in [
            ("tcl8.4", "8.4.20"),
            ("tcl8.5", "8.5.19"),
            ("tcl8.6", "8.6.18"),
            ("tcl9.0", "9.0.4"),
            ("tcl9.1", "9.1.0"),
        ] {
            let mut vm = actual(environment);
            for case in ["string", "int", "list", "bytearray", "array"] {
                let original = match case {
                    "string" => Value::new_native_string_bytes(b"missing".as_slice()),
                    "int" => Value::int(42),
                    "list" => {
                        Value::list(vec![Value::new_native_string_bytes(b"missing".as_slice())])
                    }
                    "bytearray" => Value::byte_array(b"missing".to_vec()),
                    "array" => Value::new_native_string_bytes(b"arr(k\0z)".as_slice()),
                    _ => unreachable!(),
                };
                rows.push(observe(version, case, "before", &original));
                assert!(vm.read_original_named_variable(&original).is_err());
                rows.push(observe(version, case, "missing", &original));
                let value = Value::new_native_string_bytes(b"VALUE".as_slice());
                assert!(
                    vm.store_original_named_variable(&original, value.clone())
                        .is_ok()
                );
                rows.push(observe(version, case, "stored", &original));
                assert!(
                    vm.read_original_named_variable(&original)
                        .unwrap()
                        .is_same_object(&value)
                );
                rows.push(observe(version, case, "read", &original));
                let copy = original.duplicate_native_object_in(
                    vm.actual_native_invocation_dialect()
                        .native_string_protocol()
                        .unwrap(),
                );
                rows.push(observe(version, case, "duplicate", &copy));
                drop(copy);
                for name in [b"missing".as_slice(), b"42".as_slice(), b"arr".as_slice()] {
                    vm.unset_var_bytes(name);
                }
                assert!(vm.read_original_named_variable(&original).is_err());
                rows.push(observe(version, case, "after-unset-missing", &original));
            }
        }
        let expected =
            include_str!("../../../tcl-syntax/tests/data/native_variable_name/parsed_headers.txt")
                .lines()
                .collect::<Vec<_>>();
        assert_eq!(rows.len(), 150);
        assert_eq!(rows, expected);
    }

    #[test]
    fn scalar_alias_entries_and_array_parts_match_all_95_native_windows() {
        let mut rows = Vec::new();
        for (environment, version) in [
            ("tcl8.4", "8.4.20"),
            ("tcl8.5", "8.5.19"),
            ("tcl8.6", "8.6.18"),
            ("tcl9.0", "9.0.4"),
            ("tcl9.1", "9.1.0"),
        ] {
            let mut vm = actual(environment);
            for mode in 0..2 {
                let name = Value::new_native_string_bytes(b"k".as_slice());
                let value = Value::new_native_string_bytes(b"ONE".as_slice());
                vm.store_original_named_variable(&name, value.clone())
                    .unwrap();
                let binding = vm.var_binding_from_bytes(b"k", 0).unwrap();
                let before = *vm.var_table_mut(binding.owner).unwrap().get(b"k").unwrap();
                let observe = |vm: &mut Vm, phase: &str, fresh: Option<&Value>| {
                    let table = vm.var_table_mut(binding.owner).unwrap();
                    let found = table.get(b"k").copied();
                    assert!(matches!(
                        name.native_object_snapshot().cache,
                        Cache::ParsedVariableName { .. }
                    ));
                    format!(
                        "{version}|alias|{mode}|{phase}|parsedVarName|{}|{}|{}|{}|{}",
                        name.native_object_reference_count(),
                        fresh.map_or(-1, |fresh| i64::try_from(
                            fresh.native_object_reference_count()
                        )
                        .unwrap()),
                        usize::from(found.is_some()),
                        usize::from(found == Some(before)),
                        usize::from(table.native_key_is(b"k", &name))
                    )
                };
                rows.push(observe(&mut vm, "created", None));
                vm.push_call_frame(Some("run".into()), Vec::new());
                vm.link_upvar_bytes(0, b"k", b"a").unwrap();
                rows.push(observe(&mut vm, "linked", None));
                assert!(vm.unset_var_bytes(b"::k"));
                rows.push(observe(&mut vm, "unset-target", None));
                let fresh = Value::new_native_string_bytes(b"k".as_slice());
                let written = Value::new_native_string_bytes(if mode == 0 {
                    b"::k".as_slice()
                } else {
                    b"a".as_slice()
                });
                vm.store_original_named_variable(&written, value.clone())
                    .unwrap();
                rows.push(observe(&mut vm, "recreated", Some(&fresh)));
                assert!(vm.unset_var_bytes(b"::k"));
                rows.push(observe(&mut vm, "unset-recreated-target", Some(&fresh)));
                assert!(!vm.unset_var_bytes(b"a"));
                rows.push(observe(&mut vm, "unset-alias", Some(&fresh)));
                vm.pop_call_frame();
                rows.push(observe(&mut vm, "frame-popped", Some(&fresh)));
                assert!(vm.read_original_named_variable(&fresh).is_err());
                rows.push(observe(&mut vm, "fresh-read", Some(&fresh)));
            }
            let name = Value::new_native_string_bytes(b"arr(k)".as_slice());
            vm.store_original_named_variable(
                &name,
                Value::new_native_string_bytes(b"VALUE".as_slice()),
            )
            .unwrap();
            let observe = |phase: &str| {
                name.with_native_parsed_variable(|cache| {
                    let (root, element) = cache.array.as_ref().unwrap();
                    let element_refs = match element {
                        NativeParsedVariableElement::Object(element) => {
                            i64::try_from(element.native_object_reference_count()).unwrap()
                        }
                        NativeParsedVariableElement::Bytes(_) => -1,
                    };
                    format!(
                        "{version}|parts|{phase}|{}|{element_refs}",
                        root.native_object_reference_count()
                    )
                })
                .unwrap()
            };
            rows.push(observe("stored"));
            let copy = name.duplicate_native_object_in(
                vm.actual_native_invocation_dialect()
                    .native_string_protocol()
                    .unwrap(),
            );
            rows.push(observe("duplicated"));
            drop(copy);
            assert!(vm.unset_var_bytes(b"arr"));
            rows.push(observe("unset"));
        }
        let expected = include_str!(
            "../../../tcl-syntax/tests/data/native_variable_name/scalar_alias_entries.txt"
        )
        .lines()
        .collect::<Vec<_>>();
        assert_eq!(rows.len(), 95);
        assert_eq!(rows, expected);
    }
}
