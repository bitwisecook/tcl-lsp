// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original C9.1 property metadata and borrowed native List table ownership.
use super::{
    BTreeMap, BTreeSet, Code, Completion, Method, MethodParameters, OoId, OoState, Rc,
    TclOoPropertyKind, Value, Vm, apply_property_bytes, display_oo_bytes, native_context,
    native_method_key_for_target, ok, oo_invoke_value_with_head, private_storage_name_bytes,
    slot_ref_class, slot_ref_obj,
};
use tcl_cmd_core::CmdError;
use tcl_registry::native_property_lookup::{
    NativePropertyGraphDependents, NativePropertyInvalidation,
};
use tcl_syntax::value::ValueError;
pub(super) mod mutation;
mod original_stages;
use original_stages::{OriginalPropertyOperation, PropertyDerivation};

#[derive(Default)]
pub(super) struct NativePropertyCache {
    epoch: u64,
    readable: Option<Value>,
    writable: Option<Value>,
}
struct RetiredNativeProperties {
    methods: Option<super::native_method_cache::RetiredMethodTable>,
    members: [[Option<BTreeMap<String, Value>>; 2]; 2],
    caches: [Option<NativePropertyCache>; 2],
    recipes: [Option<tcl_registry::native_property_lookup::NativePropertyLookupProtocol>; 2],
}
impl RetiredNativeProperties {
    fn finish(
        mut self,
        vm: &mut Vm,
        owner: &crate::interp::oo_cleanup::OriginalOoCleanup,
    ) -> Result<(), CmdError> {
        use tcl_registry::native_property_lookup::NativePropertyStorageRetirementStep as Stage;
        let mut first = owner.check_cleanup(vm).err();
        for class in 0..2 {
            if let Some(recipe) = self.recipes[class] {
                for stage in recipe.storage_retirement() {
                    match stage {
                        Stage::ReadableCache => drop(
                            self.caches[class]
                                .as_mut()
                                .and_then(|cache| cache.readable.take()),
                        ),
                        Stage::WritableCache => drop(
                            self.caches[class]
                                .as_mut()
                                .and_then(|cache| cache.writable.take()),
                        ),
                        Stage::ReadableMembers => drop(self.members[class][0].take()),
                        Stage::WritableMembers => drop(self.members[class][1].take()),
                    }
                    if let Err(error) = owner.check_cleanup(vm) {
                        first.get_or_insert(error);
                    }
                }
            } else {
                // Unissued compatibility storage has only ordinary Rust ownership.
                drop(self.members[class][0].take());
                drop(self.members[class][1].take());
                drop(self.caches[class].take());
                if let Err(error) = owner.check_cleanup(vm) {
                    first.get_or_insert(error);
                }
            }
        }
        drop(self.methods.take());
        if let Err(error) = owner.check_cleanup(vm) {
            first.get_or_insert(error);
        }
        first.map_or(Ok(()), Err)
    }
}
impl OoState {
    fn take_native_properties(&mut self, target: OoId) -> RetiredNativeProperties {
        let methods = self.native_methods.retire(target);
        let members = [false, true].map(|class| {
            [false, true].map(|writable| self.property_members.remove(&(target, class, writable)))
        });
        let caches = [false, true].map(|class| self.property_caches.remove(&(target, class)));
        let recipes = [false, true].map(|class| self.property_origins.remove(&(target, class)));
        RetiredNativeProperties {
            methods,
            members,
            caches,
            recipes,
        }
    }
    fn apply_property_invalidation(
        &mut self,
        target: OoId,
        action: NativePropertyInvalidation,
    ) -> Result<Option<NativePropertyCache>, ValueError> {
        Ok(match action {
            NativePropertyInvalidation::None => None,
            NativePropertyInvalidation::Foundation => {
                self.property_foundation_epoch = self
                    .property_foundation_epoch
                    .checked_add(1)
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "original OO property Foundation epoch",
                    ))?;
                None
            }
            NativePropertyInvalidation::Instance => self.property_caches.remove(&(target, false)),
            NativePropertyInvalidation::ClassRepresentative => {
                self.property_caches.remove(&(target, true))
            }
        })
    }
    fn collect_native_class_properties<'a>(
        &'a self,
        target: OoId,
        writable: bool,
        seen: &mut BTreeSet<OoId>,
        out: &mut BTreeMap<Vec<u8>, &'a Value>,
    ) -> Result<(), ValueError> {
        if !seen.insert(target) {
            return Ok(());
        }
        let Some(class) = self.classes.get(&target) else {
            return Ok(());
        };
        self.collect_native_declared_properties(
            target,
            true,
            writable,
            slot_ref_class(class, writable),
            out,
        )?;
        if Some(target) == self.object_root {
            return Ok(());
        }
        for mixin in &class.mixins {
            self.collect_native_class_properties(*mixin, writable, seen, out)?;
        }
        if class.supers.is_empty() {
            if let Some(root) = self.object_root {
                self.collect_native_class_properties(root, writable, seen, out)?;
            }
        } else {
            for sup in &class.supers {
                self.collect_native_class_properties(*sup, writable, seen, out)?;
            }
        }
        Ok(())
    }
    fn collect_native_declared_properties<'a>(
        &'a self,
        target: OoId,
        class: bool,
        writable: bool,
        names: &BTreeSet<String>,
        out: &mut BTreeMap<Vec<u8>, &'a Value>,
    ) -> Result<(), ValueError> {
        for name in names {
            if out.contains_key(self.method_name_bytes(name)) {
                continue;
            }
            let original = self
                .property_members
                .get(&(target, class, writable))
                .and_then(|members| members.get(name))
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original property metadata member",
                ))?;
            out.insert(self.method_name_bytes(name).to_vec(), original);
        }
        Ok(())
    }
}
impl Vm {
    pub(super) fn retire_original_property_structure(
        &mut self,
        target: OoId,
        owner: &crate::interp::oo_cleanup::OriginalOoCleanup,
    ) -> Result<(), CmdError> {
        let (methods, property_cache, exhausted) = {
            let state = owner.oo_mut(self)?;
            let class = state.classes.contains_key(&target);
            let subclasses = state
                .classes
                .iter()
                .any(|(id, entry)| *id != target && entry.supers.contains(&target));
            let instances = state
                .objects
                .values()
                .any(|entry| entry.class == target || entry.mixins.contains(&target));
            let mixins = state
                .classes
                .values()
                .any(|entry| entry.mixins.contains(&target));
            let representative = state
                .objects
                .get(&target)
                .is_some_and(|entry| !entry.mixins.is_empty());
            let methods = if owner.method_cache_selected {
                state.native_methods.structure_changed_checked(
                    target,
                    class,
                    subclasses || instances || mixins,
                    representative,
                )
            } else {
                Ok(None)
            };
            let recipe = state
                .property_origins
                .get(&(target, class))
                .copied()
                .or(owner.property_recipe);
            let action = recipe.map_or(NativePropertyInvalidation::None, |recipe| {
                recipe.structure_changed(
                    class,
                    NativePropertyGraphDependents {
                        subclasses,
                        instances,
                        mixin_dependents: mixins,
                    },
                    representative,
                )
            });
            let mut exhausted = false;
            let property_cache = match action {
                NativePropertyInvalidation::None => None,
                NativePropertyInvalidation::Foundation => {
                    if let Some(next) = state.property_foundation_epoch.checked_add(1) {
                        state.property_foundation_epoch = next;
                    } else {
                        exhausted = true;
                    }
                    None
                }
                NativePropertyInvalidation::Instance => {
                    state.property_caches.remove(&(target, false))
                }
                NativePropertyInvalidation::ClassRepresentative => {
                    state.property_caches.remove(&(target, true))
                }
            };
            (methods, property_cache, exhausted)
        };
        let mut first = owner.check_cleanup(self).err();
        let methods = match methods {
            Ok(methods) => methods,
            Err(error) => {
                let error = CmdError::from(error);
                crate::interp::oo_cleanup::retain_error(self, &error);
                first.get_or_insert(error);
                None
            }
        };
        if exhausted {
            let error: CmdError =
                ValueError::CommandProtocolUnavailable("original OO property Foundation epoch")
                    .into();
            crate::interp::oo_cleanup::retain_error(self, &error);
            first.get_or_insert(error);
        }
        drop(methods);
        if let Err(error) = owner.check_cleanup(self) {
            first.get_or_insert(error);
        }
        drop(property_cache);
        if let Err(error) = owner.check_cleanup(self) {
            first.get_or_insert(error);
        }
        first.map_or(Ok(()), Err)
    }
    pub(super) fn retire_native_properties(
        &mut self,
        target: OoId,
        owner: &crate::interp::oo_cleanup::OriginalOoCleanup,
    ) -> Result<(), CmdError> {
        let retired = owner.oo_mut(self)?.take_native_properties(target);
        retired.finish(self, owner)
    }
    pub(super) fn native_property_structure_changed(
        &mut self,
        target: OoId,
        class: bool,
    ) -> Result<(), CmdError> {
        mutation::invalidate(self, target, class, false)
    }
    pub(super) fn native_property_method_created(
        &mut self,
        target: OoId,
        class: bool,
    ) -> Result<(), CmdError> {
        mutation::invalidate(self, target, class, true)
    }
    fn install_original_property_members(
        &mut self,
        target: OoId,
        class: bool,
        writable: bool,
        operation: &OriginalPropertyOperation,
        headers: &[crate::value::NativeObjectLifetimeLease],
    ) -> Result<BTreeSet<String>, CmdError> {
        mutation::install_members(self, target, class, writable, operation, headers)
    }
    pub(super) fn native_all_property_header(
        &mut self,
        target: OoId,
        class: bool,
        writable: bool,
    ) -> Result<crate::value::NativeObjectLifetimeLease, CmdError> {
        let recipe = self
            .actual_native_invocation_dialect()
            .native_property_lookup_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native property List issuer",
            ))?;
        let operation = OriginalPropertyOperation::enter(self, target, class, recipe.strings())?;
        let reached = (|| {
            self.oo
                .property_origins
                .entry((target, class))
                .or_insert(recipe);
            let epoch = self.oo.property_foundation_epoch;
            if let Some(cache) = self.oo.property_caches.get(&(target, class))
                && cache.epoch == epoch
                && let Some(header) = if writable {
                    &cache.writable
                } else {
                    &cache.readable
                }
            {
                let lease = header.native_lifetime_lease();
                operation.check(self)?;
                lease.value().check_native_header()?;
                return Ok(lease);
            }
            let derivation = PropertyDerivation::capture(self, target, class, writable, false)?;
            let mut members: Vec<_> =
                original_stages::report_members(self, target, class, writable, false)?
                    .into_iter()
                    .map(|(_, original)| original.value().clone())
                    .collect();
            // These actual member roles become the report List's roles; leases add none.
            let mut failure = None;
            members.sort_by(|a, b| {
                if failure.is_some() {
                    return std::cmp::Ordering::Equal;
                }
                match original_stages::compare(self, &operation, a, b, |vm| {
                    derivation.check(vm, &operation)
                }) {
                    Ok(order) => order,
                    Err(error) => {
                        original_stages::retain_error(self, &error);
                        failure = Some(error);
                        std::cmp::Ordering::Equal
                    }
                }
            });
            if let Some(error) = failure {
                return Err(error);
            }
            derivation.check(self, &operation)?;
            let header = Value::native_list_constructor(members, recipe.strings());
            drop(operation.list_checked(self, &header, |vm| derivation.check(vm, &operation))?);
            let lease = header.native_lifetime_lease();
            let (old_writable, old_readable) = {
                let cache = self.oo.property_caches.entry((target, class)).or_default();
                if cache.epoch != epoch {
                    (cache.writable.take(), cache.readable.take())
                } else if writable {
                    (cache.writable.take(), None)
                } else {
                    (None, cache.readable.take())
                }
            };
            // GetAllObjectProperties reaches writable, then readable cache retirement.
            drop(old_writable);
            derivation.check(self, &operation)?;
            drop(old_readable);
            derivation.check(self, &operation)?;
            let cache = self.oo.property_caches.entry((target, class)).or_default();
            let vacant = if writable {
                cache.writable.is_none()
            } else {
                cache.readable.is_none()
            };
            operation.require(self, vacant, "original property report cache adoption")?;
            let cache = self.oo.property_caches.entry((target, class)).or_default();
            cache.epoch = epoch;
            if writable {
                cache.writable = Some(header);
            } else {
                cache.readable = Some(header);
            }
            Ok(lease)
        })();
        operation.complete(self, reached)
    }

    pub(super) fn native_declared_property_header(
        &mut self,
        target: OoId,
        class: bool,
        writable: bool,
    ) -> Result<Value, CmdError> {
        let protocol = self
            .actual_native_invocation_dialect()
            .native_property_lookup_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native property metadata issuer",
            ))?
            .strings();
        let operation = OriginalPropertyOperation::enter(self, target, class, protocol)?;
        let reached = (|| {
            let derivation = PropertyDerivation::capture(self, target, class, writable, true)?;
            let mut members: Vec<_> =
                original_stages::report_members(self, target, class, writable, true)?
                    .into_iter()
                    .map(|(_, original)| original.value().clone())
                    .collect();
            let mut failure = None;
            members.sort_by(|a, b| {
                if failure.is_some() {
                    return std::cmp::Ordering::Equal;
                }
                match original_stages::compare(self, &operation, a, b, |vm| {
                    derivation.check(vm, &operation)
                }) {
                    Ok(order) => order,
                    Err(error) => {
                        original_stages::retain_error(self, &error);
                        failure = Some(error);
                        std::cmp::Ordering::Equal
                    }
                }
            });
            if let Some(error) = failure {
                return Err(error);
            }
            derivation.check(self, &operation)?;
            let header = Value::native_list_constructor(members, protocol);
            drop(operation.list_checked(self, &header, |vm| derivation.check(vm, &operation))?);
            Ok(header)
        })();
        operation.complete(self, reached)
    }
}

/// Frozen temporary-table Strings retain the original copied List and target.
/// A later pair still projects its old table index into the current report.
pub(super) struct RetainedPropertyTable {
    header: Value,
    words: Vec<Rc<[u8]>>,
    currency: crate::interp::native_operation_currency::NativeOperationScope,
    target: super::original_name_operands::OriginalOoTarget,
}
impl RetainedPropertyTable {
    fn check(&self, vm: &mut Vm, operation: &OriginalPropertyOperation) -> Result<(), CmdError> {
        self.currency.check(vm)?;
        operation.require(
            vm,
            self.target.is_current(vm),
            "original property temporary table target",
        )?;
        self.header.check_native_header()?;
        operation.check(vm)
    }
}

pub(super) fn lookup_property(
    vm: &mut Vm,
    target: OoId,
    original: &Value,
    writable: bool,
    table: &mut Option<RetainedPropertyTable>,
    retain_table: bool,
) -> Result<crate::value::NativeObjectLifetimeLease, Completion<Value>> {
    let recipe = vm
        .actual_native_invocation_dialect()
        .native_property_lookup_protocol()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "native property lookup issuer",
        ))
        .map_err(|e| crate::command::completion_from_cmd_error(vm, e.into()))?;
    let operation = OriginalPropertyOperation::enter(vm, target, false, recipe.strings())
        .map_err(|e| crate::command::completion_from_cmd_error(vm, e))?;
    let reached = lookup_property_entered(
        vm,
        target,
        original,
        writable,
        table,
        retain_table,
        &operation,
    );
    operation.complete_result(vm, reached)
}

fn lookup_property_entered(
    vm: &mut Vm,
    target: OoId,
    original: &Value,
    writable: bool,
    table: &mut Option<RetainedPropertyTable>,
    retain_table: bool,
    operation: &OriginalPropertyOperation,
) -> Result<crate::value::NativeObjectLifetimeLease, Completion<Value>> {
    let recipe = vm
        .actual_native_invocation_dialect()
        .native_property_lookup_protocol()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "native property lookup issuer",
        ))
        .map_err(|e| crate::command::completion_from_cmd_error(vm, e.into()))?;
    recipe
        .validate_index_origin(original.native_index_cache().map(|(_, origin)| origin))
        .map_err(|e| crate::command::completion_from_cmd_error(vm, e.into()))?;
    if let Some(table) = table.as_ref() {
        table
            .check(vm, operation)
            .map_err(|e| crate::command::completion_from_cmd_error(vm, e))?;
    }
    let header = vm
        .native_all_property_header(target, false, writable)
        .map_err(|e| crate::command::completion_from_cmd_error(vm, e))?;
    let words = if let Some(table) = table.as_ref() {
        table
            .check(vm, operation)
            .map_err(|e| crate::command::completion_from_cmd_error(vm, e))?;
        table.words.clone()
    } else {
        let names = operation
            .list(vm, header.value())
            .map_err(|e| crate::command::completion_from_cmd_error(vm, e))?;
        let mut words = Vec::with_capacity(names.len());
        for name in names.iter() {
            words.push(
                operation
                    .string(vm, name)
                    .map_err(|e| crate::command::completion_from_cmd_error(vm, e))?,
            );
        }
        if retain_table {
            let copied = operation
                .copy_list(vm, header.value())
                .map_err(|e| crate::command::completion_from_cmd_error(vm, e))?;
            let target = operation
                .retained_target()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original property temporary table target",
                ))
                .map_err(|e| crate::command::completion_from_cmd_error(vm, e.into()))?;
            *table = Some(RetainedPropertyTable {
                header: copied,
                words: words.clone(),
                currency: operation.operation.clone(),
                target,
            });
        }
        words
    };
    let bytes = operation
        .string(vm, original)
        .map_err(|e| crate::command::completion_from_cmd_error(vm, e))?;
    if let Some(table) = table.as_ref() {
        table
            .check(vm, operation)
            .map_err(|e| crate::command::completion_from_cmd_error(vm, e))?;
    }
    let borrowed: Vec<_> = words.iter().map(std::convert::AsRef::as_ref).collect();
    let verdict = recipe.lookup(&bytes, &borrowed);
    operation
        .check(vm)
        .map_err(|e| crate::command::completion_from_cmd_error(vm, e))?;
    match verdict {
        Ok(index) => selected_property_member(vm, header.value(), operation, index),
        Err(message) => {
            property_lookup_failure(vm, target, writable, operation, &bytes, recipe, message)
        }
    }
}

fn property_lookup_failure(
    vm: &mut Vm,
    target: OoId,
    writable: bool,
    operation: &OriginalPropertyOperation,
    bytes: &[u8],
    recipe: tcl_registry::native_property_lookup::NativePropertyLookupProtocol,
    mut error: Vec<u8>,
) -> Result<crate::value::NativeObjectLifetimeLease, Completion<Value>> {
    let other = vm
        .native_all_property_header(target, false, !writable)
        .map_err(|e| crate::command::completion_from_cmd_error(vm, e))?;
    let other = operation
        .list(vm, other.value())
        .map_err(|e| crate::command::completion_from_cmd_error(vm, e))?;
    let mut other_words = Vec::new();
    for name in other.iter() {
        other_words.push(
            operation
                .string(vm, name)
                .map_err(|e| crate::command::completion_from_cmd_error(vm, e))?,
        );
    }
    let other_borrowed: Vec<_> = other_words
        .iter()
        .map(std::convert::AsRef::as_ref)
        .collect();
    if let Ok(index) = recipe.lookup(bytes, &other_borrowed) {
        error = b"property \"".to_vec();
        error.extend_from_slice(tcl_core_types::c_string_extent(&other_words[index]));
        error.extend_from_slice(if writable {
            b"\" is read only"
        } else {
            b"\" is write only"
        });
    }
    let protocol = vm
        .actual_native_invocation_dialect()
        .native_index_lookup_protocol()
        .expect("selected C91 property Index recipe");
    Err(crate::command::completion_from_cmd_error(
        vm,
        tcl_cmd_core::CmdError::with_error_code_bytes(
            error,
            protocol.error_code(b"property", bytes),
        )
        .with_native_string_result(recipe.strings()),
    ))
}

fn selected_property_member(
    vm: &mut Vm,
    header: &Value,
    operation: &OriginalPropertyOperation,
    index: usize,
) -> Result<crate::value::NativeObjectLifetimeLease, Completion<Value>> {
    let current = operation
        .list(vm, header)
        .map_err(|e| crate::command::completion_from_cmd_error(vm, e))?;
    let selected = current.get(index).ok_or_else(|| {
        crate::command::completion_from_cmd_error(
            vm,
            ValueError::CommandProtocolUnavailable("property callback changed native table extent")
                .into(),
        )
    })?;
    selected
        .check_native_header()
        .map_err(|e| crate::command::completion_from_cmd_error(vm, e.into()))?;
    operation
        .check(vm)
        .map_err(|e| crate::command::completion_from_cmd_error(vm, e))?;
    Ok(selected.native_lifetime_lease())
}

pub(super) fn register_property(
    vm: &mut Vm,
    target: OoId,
    class: bool,
    name: &[u8],
    readable: bool,
    writable: bool,
) -> Result<(), CmdError> {
    mutation::register(vm, target, class, name, readable, writable)
}

pub(super) fn set_property_slot(
    vm: &mut Vm,
    target: OoId,
    class: bool,
    writable: bool,
    operation: tcl_registry::definer::SlotOp,
    names: &[Value],
) -> Result<BTreeSet<String>, CmdError> {
    mutation::slot(vm, target, class, writable, operation, names)
}

pub(super) fn configure_all(vm: &mut Vm, target: OoId) -> Completion<Value> {
    let Some(recipe) = vm
        .actual_native_invocation_dialect()
        .native_property_lookup_protocol()
    else {
        return crate::command::completion_from_cmd_error(
            vm,
            ValueError::CommandProtocolUnavailable("native property configure issuer").into(),
        );
    };
    let operation = match OriginalPropertyOperation::enter(vm, target, false, recipe.strings()) {
        Ok(operation) => operation,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    let reached = configure_all_entered(vm, target, &operation);
    operation.complete_invocation(vm, reached)
}

fn configure_all_entered(
    vm: &mut Vm,
    target: OoId,
    operation: &OriginalPropertyOperation,
) -> Completion<Value> {
    let header = match vm.native_all_property_header(target, false, false) {
        Ok(header) => header,
        Err(e) => return crate::command::completion_from_cmd_error(vm, e),
    };
    // The configure reader adds one actual List-header role across callbacks.
    let header = header.value().clone();
    let names = match operation.list(vm, &header) {
        Ok(names) => names,
        Err(e) => return crate::command::completion_from_cmd_error(vm, e),
    };
    let empty = match operation.dictionary(vm, Vec::new()) {
        Ok(empty) => empty,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    let mut output = match operation.prepare_dictionary(vm, &empty) {
        Ok(output) => output,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    drop(empty);
    if let Err(error) = operation.check(vm) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    for (index, original) in names.iter().enumerate() {
        let value = match read_original_property(vm, target, original) {
            Ok(value) => value,
            Err(error) => return error,
        };
        if let Err(error) = operation.put_dictionary(vm, &mut output, original.clone(), value) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        if index + 1 < names.len() {
            if let Err(error) =
                vm.adopt_native_interp_result(Value::new_native_string_bytes(b"".as_slice()))
            {
                return crate::command::completion_from_cmd_error(vm, error.into());
            }
            if let Err(error) = operation.check(vm) {
                return crate::command::completion_from_cmd_error(vm, error);
            }
        }
    }
    if names.is_empty() {
        return ok(Value::empty());
    }
    ok(output.into_value())
}

/// Actual GetterType/SetterType clientData. Native method records share this
/// allocation during invocation; a new cloned declaration duplicates its role.
pub(super) struct NativePropertyAccessor {
    pub(super) original: Value,
    pub(super) writable: bool,
}
pub(super) fn default_method(original: &Value, writable: bool) -> Method {
    Method {
        parameters: MethodParameters {
            values: Vec::new(),
            has_args: false,
        },
        compiled_body: None,
        body_src: Value::empty(),
        forward: None,
        property: Some(Rc::new(NativePropertyAccessor {
            original: original.clone(),
            writable,
        })),
        intrinsic: None,
        visibility_only: false,
        exported: false,
        private: false,
    }
}

fn default_storage_name(
    vm: &mut Vm,
    target: OoId,
    original: &Value,
    operation: &OriginalPropertyOperation,
) -> Result<Option<Vec<u8>>, CmdError> {
    let Some(frame) = native_context::current_method(vm) else {
        return Ok(None);
    };
    let step = frame
        .chain
        .get(frame.index)
        .ok_or(ValueError::CommandProtocolUnavailable(
            "original property declaring step",
        ))?;
    let provider = step.provider;
    let is_object = step.is_object;
    if is_object && provider != target {
        return Ok(None);
    }
    if !is_object {
        let reachable = vm.oo.is_a(target, provider)
            || vm.oo.objects.get(&target).is_some_and(|object| {
                object.mixins.iter().any(|mixin| {
                    vm.oo
                        .class_linear_of(*mixin)
                        .iter()
                        .any(|entry| entry.provider == provider)
                })
            });
        if !reachable {
            return Ok(None);
        }
    }
    let variables = original_stages::PrivateVariableRoster::capture(vm, provider, is_object)?;
    variables.lookup(vm, operation, original)
}

pub(super) fn invoke_default(
    vm: &mut Vm,
    target: OoId,
    accessor: &NativePropertyAccessor,
    args: &[Value],
    original_argv: &[Value],
) -> Completion<Value> {
    let Some(recipe) = vm
        .actual_native_invocation_dialect()
        .native_property_name_protocol()
    else {
        return crate::command::completion_from_cmd_error(
            vm,
            ValueError::CommandProtocolUnavailable("native default property accessor").into(),
        );
    };
    let mut operation = match OriginalPropertyOperation::enter(vm, target, false, recipe.strings())
    {
        Ok(operation) => operation,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    if let Err(error) = operation.select_declaring(vm) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    let reached = invoke_default_entered(vm, target, accessor, args, original_argv, &operation);
    operation.complete_invocation(vm, reached)
}

fn invoke_default_entered(
    vm: &mut Vm,
    target: OoId,
    accessor: &NativePropertyAccessor,
    args: &[Value],
    original_argv: &[Value],
    operation: &OriginalPropertyOperation,
) -> Completion<Value> {
    if args.len() != usize::from(accessor.writable) {
        let mut usage = match vm
            .native_argument_usage_header(original_argv.get(..2).unwrap_or(original_argv))
        {
            Ok(usage) => usage,
            Err(error) => return error,
        };
        if accessor.writable {
            usage.extend_from_slice(b" value");
        }
        return crate::command::native_wrong_args_bytes(vm, &usage);
    }
    let Some(recipe) = vm
        .actual_native_invocation_dialect()
        .native_property_name_protocol()
    else {
        return crate::command::completion_from_cmd_error(
            vm,
            ValueError::CommandProtocolUnavailable("native default property accessor").into(),
        );
    };
    let bytes = match operation.string(vm, &accessor.original) {
        Ok(bytes) => bytes,
        Err(error) => {
            return crate::command::completion_from_cmd_error(vm, error);
        }
    };
    let Some(object) = vm.oo.objects.get(&target) else {
        return crate::command::completion_from_cmd_error(
            vm,
            ValueError::CommandProtocolUnavailable("property object namespace").into(),
        );
    };
    let namespace = object.ns.clone();
    let mapped = match default_storage_name(vm, target, &accessor.original, operation) {
        Ok(mapped) => mapped,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    let variable_bytes = mapped.as_deref().unwrap_or(&bytes);
    let operand = match recipe.object_variable_operand(variable_bytes, namespace.as_bytes()) {
        Some(bytes) => Value::new_native_string_bytes(bytes),
        None => accessor.original.native_lifetime_lease().into_value(),
    };
    if accessor.writable {
        match vm.store_original_c_property_variable(&operand, &accessor.original, args[0].clone()) {
            Ok(()) => vm
                .with_native_interp_result(|value| ok(value.native_lifetime_lease().into_value()))
                .unwrap_or_else(|error| {
                    crate::command::completion_from_cmd_error(vm, error.into())
                }),
            Err(error) => error,
        }
    } else {
        match vm.read_original_c_property_variable(&operand, &accessor.original) {
            Ok(value) => ok(value),
            Err(error) => error,
        }
    }
}

pub(super) fn configure_native(vm: &mut Vm, target: OoId, args: &[Value]) -> Completion<Value> {
    let Some(recipe) = vm
        .actual_native_invocation_dialect()
        .native_property_lookup_protocol()
    else {
        return crate::command::completion_from_cmd_error(
            vm,
            ValueError::CommandProtocolUnavailable("native property configure issuer").into(),
        );
    };
    let operation = match OriginalPropertyOperation::enter(vm, target, false, recipe.strings()) {
        Ok(operation) => operation,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    let reached = configure_native_entered(vm, target, args);
    operation.complete_invocation(vm, reached)
}
fn configure_native_entered(vm: &mut Vm, target: OoId, args: &[Value]) -> Completion<Value> {
    if args.is_empty() {
        return configure_all(vm, target);
    }
    if args.len() > 1 && !args.len().is_multiple_of(2) {
        return crate::command::native_wrong_args(vm, "configure ?-option value ...?");
    }
    let mut table = None;
    if args.len() == 1 {
        let property = match lookup_property(vm, target, &args[0], false, &mut table, false) {
            Ok(property) => property,
            Err(error) => return error,
        };
        return match read_original_property(vm, target, property.value()) {
            Ok(value) => ok(value),
            Err(error) => error,
        };
    }
    for pair in args.as_chunks::<2>().0 {
        let property = match lookup_property(vm, target, &pair[0], true, &mut table, args.len() > 2)
        {
            Ok(property) => property,
            Err(error) => return error,
        };
        if let Err(error) = write_original_property(vm, target, property.value(), &pair[1]) {
            return error;
        }
    }
    ok(Value::empty())
}
fn invoke_original_accessor(
    vm: &mut Vm,
    target: OoId,
    property: &Value,
    writable: bool,
    value: Option<&Value>,
) -> Result<Value, Completion<Value>> {
    let recipe = vm
        .actual_native_invocation_dialect()
        .native_property_lookup_protocol()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "selected property accessor",
        ))
        .map_err(|e| crate::command::completion_from_cmd_error(vm, e.into()))?;
    let operation = OriginalPropertyOperation::enter(vm, target, false, recipe.strings())
        .map_err(|e| crate::command::completion_from_cmd_error(vm, e))?;
    let reached =
        invoke_original_accessor_entered(vm, target, property, writable, value, &operation);
    operation.complete_result(vm, reached)
}

fn invoke_original_accessor_entered(
    vm: &mut Vm,
    target: OoId,
    property: &Value,
    writable: bool,
    value: Option<&Value>,
    operation: &OriginalPropertyOperation,
) -> Result<Value, Completion<Value>> {
    let recipe = vm
        .actual_native_invocation_dialect()
        .native_property_lookup_protocol()
        .expect("selected property accessor");
    let accessor = property
        .native_property_accessor_checked(
            recipe,
            writable,
            vm,
            &mut |vm| operation.string(vm, property),
            &mut |vm| operation.check(vm),
        )
        .map_err(|e| crate::command::completion_from_cmd_error(vm, e))?;
    // The reached call owns my, accessor and any setter value until invocation ends.
    let result = {
        let Some(head) = vm.oo.property_my_name.clone() else {
            return Err(crate::command::completion_from_cmd_error(
                vm,
                ValueError::CommandProtocolUnavailable("Foundation original my name").into(),
            ));
        };
        let accessor = accessor.value().clone();
        let args = value.map_or_else(Vec::new, |value| vec![value.clone()]);
        let method = native_method_key_for_target(vm, &accessor, Some((false, target)))?;
        let invoked = Value::from_native_string_bytes(display_oo_bytes(vm, target));
        oo_invoke_value_with_head(
            vm,
            target,
            &method,
            &accessor,
            &args,
            false,
            (&invoked, Some(&head)),
        )
    };
    operation
        .check(vm)
        .map_err(|e| crate::command::completion_from_cmd_error(vm, e))?;
    match result.code {
        Code::Ok => Ok(result.result),
        Code::Break | Code::Continue => {
            let bytes = operation
                .string(vm, property)
                .map_err(|e| crate::command::completion_from_cmd_error(vm, e))?;
            let mut error = b"property ".to_vec();
            error.extend_from_slice(if writable { b"setter" } else { b"getter" });
            error.extend_from_slice(b" for ");
            error.extend_from_slice(tcl_core_types::c_string_extent(&bytes));
            error.extend_from_slice(if result.code == Code::Break {
                b" did a break"
            } else {
                b" did a continue"
            });
            Err(crate::command::completion_from_cmd_error(
                vm,
                tcl_cmd_core::CmdError::with_error_code_bytes(error, b"NONE".to_vec())
                    .with_native_string_result(recipe.strings()),
            ))
        }
        _ => Err(result),
    }
}
fn read_original_property(
    vm: &mut Vm,
    target: OoId,
    property: &Value,
) -> Result<Value, Completion<Value>> {
    invoke_original_accessor(vm, target, property, false, None)
}
fn write_original_property(
    vm: &mut Vm,
    target: OoId,
    property: &Value,
    value: &Value,
) -> Result<(), Completion<Value>> {
    invoke_original_accessor(vm, target, property, true, Some(value)).map(|_| ())
}

fn read_property_definition_options(
    vm: &mut Vm,
    args: &[Value],
    cursor: &mut usize,
    original_names: &mut super::original_name_operands::OriginalOoNameScope,
    recipe: tcl_registry::native_property_lookup::NativePropertyNameProtocol,
    options: &tcl_registry::native_index_lookup::NativeStaticIndexTable,
    kinds: &tcl_registry::native_index_lookup::NativeStaticIndexTable,
) -> Result<(TclOoPropertyKind, Option<Value>, Option<Value>), Completion<Value>> {
    let mut kind = TclOoPropertyKind::ReadWrite;
    let mut getter = None;
    let mut setter = None;
    while *cursor < args.len() {
        let bytes = match original_names.read(vm, &args[*cursor]) {
            Ok(bytes) => bytes,
            Err(error) => {
                return Err(crate::command::completion_from_cmd_error(vm, error));
            }
        };
        if bytes.first() != Some(&b'-') {
            break;
        }
        let reached = vm.native_index_operand_checked(&args[*cursor], options, false, "option");
        if let Err(error) = original_names.check_target(vm) {
            return Err(crate::command::completion_from_cmd_error(vm, error));
        }
        let option = match reached {
            Ok(option) => option,
            Err(error) => return Err(crate::command::completion_from_cmd_error(vm, error)),
        };
        *cursor += 1;
        let Some(value) = args.get(*cursor) else {
            let message = if option == 1 {
                b"missing kind value to go with -kind option".as_slice()
            } else if option == 0 {
                b"missing body to go with -get option".as_slice()
            } else {
                b"missing body to go with -set option".as_slice()
            };
            return Err(crate::command::completion_from_cmd_error(
                vm,
                tcl_cmd_core::CmdError::with_error_code_bytes(
                    message.to_vec(),
                    b"TCL WRONGARGS".to_vec(),
                )
                .with_native_string_result(recipe.strings()),
            ));
        };
        match option {
            0 => getter = Some(value.clone()),
            2 => setter = Some(value.clone()),
            1 => {
                let reached = vm.native_index_operand_checked(value, kinds, false, "kind");
                if let Err(error) = original_names.check_target(vm) {
                    return Err(crate::command::completion_from_cmd_error(vm, error));
                }
                kind = match reached {
                    Ok(0) => TclOoPropertyKind::Readable,
                    Ok(1) => TclOoPropertyKind::ReadWrite,
                    Ok(2) => TclOoPropertyKind::Writable,
                    Ok(_) => unreachable!("closed native kind roster"),
                    Err(error) => {
                        return Err(crate::command::completion_from_cmd_error(vm, error));
                    }
                };
            }
            _ => unreachable!("closed native property option roster"),
        }
        *cursor += 1;
    }
    Ok((kind, getter, setter))
}

pub(super) fn define_original_properties(
    vm: &mut Vm,
    class: bool,
    target: OoId,
    args: &[Value],
) -> Completion<Value> {
    super::original_name_operands::run(vm, Some((class, target)), |vm, original_names| {
        let recipe = vm
            .actual_native_invocation_dialect()
            .native_property_name_protocol()
            .expect("selected original property definition");
        let options = recipe.definition_options();
        let kinds = recipe.definition_kinds();
        let mut cursor = 0;
        while cursor < args.len() {
            let original = &args[cursor];
            let name = match original_names.read(vm, original) {
                Ok(name) => name,
                Err(error) => {
                    return crate::command::completion_from_cmd_error(vm, error);
                }
            };
            cursor += 1;
            let (kind, getter, setter) = match read_property_definition_options(
                vm,
                args,
                &mut cursor,
                original_names,
                recipe,
                &options,
                &kinds,
            ) {
                Ok(options) => options,
                Err(error) => return error,
            };
            if let Err(message) = recipe.validate_declaration(&name) {
                return crate::command::completion_from_cmd_error(
                    vm,
                    tcl_cmd_core::CmdError::with_error_code_bytes(
                        message,
                        b"TCL OO PROPERTY_FORMAT".to_vec(),
                    )
                    .with_native_string_result(recipe.strings()),
                );
            }
            if let Err(error) = original_names.check_target(vm) {
                return crate::command::completion_from_cmd_error(vm, error);
            }
            if let Err(error) = apply_property_bytes(
                vm,
                class,
                target,
                (&name, Some(original)),
                kind,
                getter,
                setter,
            ) {
                return error;
            }
        }
        ok(Value::empty())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd_oo::resolve_object_value;
    fn instance() -> Vm {
        let profile = crate::environment::profile_for_dialect("tcl9.1");
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
        let result=vm.eval_source("oo::configurable create C {property yellow -get {return Y}; property zinc -get {return Z}}; C create o").unwrap();
        assert_eq!(result.code, Code::Ok, "{}", result.result.to_str());
        vm
    }
    #[test]
    fn property_headers_and_temporary_tables_retain_original_members() {
        let mut vm = instance();
        let id = resolve_object_value(&mut vm, &Value::string("o"))
            .unwrap()
            .unwrap();
        let header = vm.native_all_property_header(id, false, false).unwrap();
        let original_id = header.value().native_object_identity();
        let strings = vm
            .actual_native_invocation_dialect()
            .native_property_lookup_protocol()
            .unwrap()
            .strings();
        let names = header.value().native_object_list_elements(strings).unwrap();
        let first_id = names[0].native_object_identity();
        let refs = names[0].native_object_reference_count();
        let copy = header.value().native_list_copy(strings).unwrap();
        let copied = copy.native_object_list_elements(strings).unwrap();
        assert_ne!(copy.native_object_identity(), original_id);
        assert_eq!(copied[0].native_object_identity(), first_id);
        assert_eq!(copied[0].native_object_reference_count(), refs);
        let mut table = None;
        let caller = Value::new_native_string_bytes(b"-y".as_slice());
        assert_eq!(
            lookup_property(&mut vm, id, &caller, false, &mut table, true)
                .unwrap()
                .value()
                .string_bytes()
                .as_ref(),
            b"-yellow"
        );
        assert_eq!(
            caller.native_object_snapshot().cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::None
        );
        assert!(table.is_some());
        let retained = vm.native_all_property_header(id, false, false).unwrap();
        assert_eq!(retained.value().native_object_identity(), original_id);
    }
    #[test]
    fn opaque_property_lookup_uses_original_accessor_members() {
        let mut vm = instance();
        let class = resolve_object_value(&mut vm, &Value::string("C"))
            .unwrap()
            .unwrap();
        let target = resolve_object_value(&mut vm, &Value::string("o"))
            .unwrap()
            .unwrap();
        for name in [b"x\xff".as_slice(), b"x\0tail".as_slice()] {
            let definition = [
                Value::new_native_string_bytes(name),
                Value::string("-get"),
                Value::string("return RAW"),
            ];
            let result = define_original_properties(&mut vm, true, class, &definition);
            assert_eq!(result.code, Code::Ok);
            let mut dashed = b"-".to_vec();
            dashed.extend_from_slice(name);
            let caller = Value::new_native_string_bytes(dashed);
            let result = configure_native(&mut vm, target, std::slice::from_ref(&caller));
            assert_eq!(result.code, Code::Ok);
            assert_eq!(result.result.string_bytes().as_ref(), b"RAW");
            assert_eq!(
                caller.native_object_snapshot().cache,
                tcl_syntax::native_object::NativeObjectCacheSnapshot::None
            );
            let mut table = None;
            let selected =
                lookup_property(&mut vm, target, &caller, false, &mut table, false).unwrap();
            assert!(selected.value().native_property_name_is_cached());
        }
        let rows = include_str!("../../tests/data/native_property_opaque/native.tsv");
        assert_eq!(rows, "opaque-0\tRAW\tnone\nopaque-1\tRAW\tnone\n");
    }
    #[test]
    fn default_property_methods_retain_original_clientdata_and_leave_its_primary() {
        let mut vm = instance();
        let class = resolve_object_value(&mut vm, &Value::string("C"))
            .unwrap()
            .unwrap();
        let target = resolve_object_value(&mut vm, &Value::string("o"))
            .unwrap()
            .unwrap();
        let recipe = vm
            .actual_native_invocation_dialect()
            .native_property_lookup_protocol()
            .unwrap();
        for name in [b"p".as_slice(), b"x\xff".as_slice(), b"x\0tail".as_slice()] {
            let declaration = Value::new_native_string_bytes(name);
            let result = define_original_properties(
                &mut vm,
                true,
                class,
                std::slice::from_ref(&declaration),
            );
            assert_eq!(result.code, Code::Ok, "{}", result.result.to_str());
            drop(result);
            let mut canonical = b"-".to_vec();
            canonical.extend_from_slice(tcl_core_types::c_string_extent(name));
            let (reader, writer) = recipe.member_accessor_names(&canonical);
            for key in [reader, writer] {
                let key = vm
                    .oo
                    .intern_method_key(tcl_core_types::NameBytes::from(key));
                let method = vm.oo.classes[&class].methods.get(&key).unwrap();
                assert_eq!(
                    method
                        .property
                        .as_ref()
                        .unwrap()
                        .original
                        .native_object_identity(),
                    declaration.native_object_identity()
                );
            }
            assert_eq!(declaration.native_object_reference_count(), 3);
            let mut query = b"-".to_vec();
            query.extend_from_slice(name);
            let query = Value::new_native_string_bytes(query);
            let supplied = Value::new_native_string_bytes(b"RAW".as_slice());
            let arguments = [
                query.native_lifetime_lease().into_value(),
                supplied.native_lifetime_lease().into_value(),
            ];
            let written = configure_native(&mut vm, target, &arguments);
            assert_eq!(written.code, Code::Ok, "{}", written.result.to_str());
            assert!(written.result.string_bytes().is_empty());
            drop(written);
            let read = configure_native(&mut vm, target, std::slice::from_ref(&query));
            assert_eq!(read.code, Code::Ok, "{}", read.result.to_str());
            assert_eq!(
                read.result.native_object_identity(),
                supplied.native_object_identity()
            );
            assert_eq!(declaration.native_object_reference_count(), 3);
            assert_eq!(
                declaration.native_object_snapshot().cache,
                tcl_syntax::native_object::NativeObjectCacheSnapshot::None
            );
            assert_eq!(
                query.native_object_snapshot().cache,
                tcl_syntax::native_object::NativeObjectCacheSnapshot::None
            );
        }
        let rows = include_str!("../../tests/data/native_property_clientdata/native.tsv");
        assert_eq!(rows.lines().count(), 9);
        for index in 0..3 {
            assert!(rows.contains(&format!("decl-{index}\t1\t1\t3\tnone\n")));
            assert!(rows.contains(&format!("write-{index}\t3\tnone\t1\n")));
            assert!(rows.contains(&format!("read-{index}\t3\tnone\t1\tnone\n")));
        }
    }
    #[test]
    fn property_epochs_follow_mutations_before_definition_failure() {
        let mut vm = instance();
        let id = resolve_object_value(&mut vm, &Value::string("o"))
            .unwrap()
            .unwrap();
        let class = resolve_object_value(&mut vm, &Value::string("C"))
            .unwrap()
            .unwrap();
        let header = vm.native_all_property_header(id, false, false).unwrap();
        let old = header.value().native_object_identity();
        let epoch = vm.oo.property_foundation_epoch;
        vm.native_property_method_created(id, false).unwrap();
        assert_eq!(vm.oo.property_foundation_epoch, epoch);
        assert_eq!(
            vm.native_all_property_header(id, false, false)
                .unwrap()
                .value()
                .native_object_identity(),
            old
        );
        let result = vm
            .eval_source("catch {oo::define C {::oo::define::method live {} {return};error LATE}}")
            .unwrap();
        assert_eq!(result.code, Code::Ok);
        assert_eq!(vm.oo.property_foundation_epoch, epoch + 1);
        assert_eq!(
            vm.oo.property_caches[&(id, false)]
                .readable
                .as_ref()
                .unwrap()
                .native_object_identity(),
            old
        );
        assert_ne!(
            vm.native_all_property_header(id, false, false)
                .unwrap()
                .value()
                .native_object_identity(),
            old
        );
        let epoch = vm.oo.property_foundation_epoch;
        vm.native_property_structure_changed(class, true).unwrap();
        assert_eq!(vm.oo.property_foundation_epoch, epoch + 1);
    }
}

#[cfg(test)]
mod stage_tests;
