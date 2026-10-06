// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original C9.1 property metadata and borrowed native List table ownership.
use super::{
    BTreeMap, BTreeSet, Code, Completion, Method, OoId, OoState, Rc, TclOoPropertyKind, Value, Vm,
    apply_property_bytes, display_oo_bytes, native_context, native_method_key, ok,
    oo_invoke_value_with_head, private_storage_name_bytes, slot_ref_class, slot_ref_obj,
};
use tcl_registry::native_property_lookup::{
    NativePropertyGraphDependents, NativePropertyInvalidation,
};
use tcl_syntax::value::ValueError;

#[derive(Default)]
pub(super) struct NativePropertyCache {
    epoch: u64,
    readable: Option<Value>,
    writable: Option<Value>,
}
impl OoState {
    pub(super) fn retire_native_properties(&mut self, target: OoId) {
        self.native_methods.retire(target);
        self.property_members.retain(|(id, _, _), _| *id != target);
        self.property_caches.retain(|(id, _), _| *id != target);
    }
    fn apply_property_invalidation(&mut self, target: OoId, action: NativePropertyInvalidation) {
        match action {
            NativePropertyInvalidation::None => {}
            NativePropertyInvalidation::Foundation => {
                self.property_foundation_epoch = self
                    .property_foundation_epoch
                    .checked_add(1)
                    .expect("Foundation epoch exhausted");
            }
            NativePropertyInvalidation::Instance => {
                self.property_caches.remove(&(target, false));
            }
            NativePropertyInvalidation::ClassRepresentative => {
                self.property_caches.remove(&(target, true));
            }
        }
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
    pub(super) fn native_property_structure_changed(&mut self, target: OoId, class: bool) {
        self.native_method_structure_changed(target, class);
        let Some(recipe) = self
            .actual_native_invocation_dialect()
            .native_property_lookup_protocol()
        else {
            return;
        };
        let subclasses = self
            .oo
            .classes
            .iter()
            .any(|(id, c)| *id != target && c.supers.contains(&target));
        let instances = self
            .oo
            .objects
            .values()
            .any(|o| o.class == target || o.mixins.contains(&target));
        let mixins = self.oo.classes.values().any(|c| c.mixins.contains(&target));
        let representative = self
            .oo
            .objects
            .get(&target)
            .is_some_and(|o| !o.mixins.is_empty());
        let action = recipe.structure_changed(
            class,
            NativePropertyGraphDependents {
                subclasses,
                instances,
                mixin_dependents: mixins,
            },
            representative,
        );
        self.oo.apply_property_invalidation(target, action);
    }
    pub(super) fn native_property_method_created(&mut self, target: OoId, class: bool) {
        self.native_method_created(target, class);
        if let Some(recipe) = self
            .actual_native_invocation_dialect()
            .native_property_lookup_protocol()
        {
            self.oo
                .apply_property_invalidation(target, recipe.method_created(class));
        }
    }
    pub(super) fn install_original_property_members(
        &mut self,
        target: OoId,
        class: bool,
        writable: bool,
        members: Vec<(String, Value)>,
    ) {
        let mut originals = BTreeMap::new();
        for (name, original) in members {
            originals.entry(name).or_insert(original);
        }
        self.oo
            .property_members
            .insert((target, class, writable), originals);
        if let Some(cache) = self.oo.property_caches.get_mut(&(target, class)) {
            if writable {
                cache.writable = None;
            } else {
                cache.readable = None;
            }
        }
    }
    pub(super) fn native_all_property_header(
        &mut self,
        target: OoId,
        class: bool,
        writable: bool,
    ) -> Result<crate::value::NativeObjectLifetimeLease, ValueError> {
        let recipe = self
            .actual_native_invocation_dialect()
            .native_property_lookup_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native property List issuer",
            ))?;
        let epoch = self.oo.property_foundation_epoch;
        if let Some(cache) = self.oo.property_caches.get(&(target, class))
            && cache.epoch == epoch
            && let Some(header) = if writable {
                &cache.writable
            } else {
                &cache.readable
            }
        {
            return Ok(header.native_lifetime_lease());
        }
        let members =
            {
                let mut members = BTreeMap::new();
                let mut seen = BTreeSet::new();
                if class {
                    self.oo.collect_native_class_properties(
                        target,
                        writable,
                        &mut seen,
                        &mut members,
                    )?;
                } else {
                    let object = self.oo.objects.get(&target).ok_or(
                        ValueError::CommandProtocolUnavailable("native property object"),
                    )?;
                    self.oo.collect_native_declared_properties(
                        target,
                        false,
                        writable,
                        slot_ref_obj(object, writable),
                        &mut members,
                    )?;
                    for mixin in &object.mixins {
                        self.oo.collect_native_class_properties(
                            *mixin,
                            writable,
                            &mut seen,
                            &mut members,
                        )?;
                    }
                    self.oo.collect_native_class_properties(
                        object.class,
                        writable,
                        &mut seen,
                        &mut members,
                    )?;
                }
                let mut members: Vec<_> = members.into_iter().collect();
                let mut failure = None;
                members.sort_by(|(_, a), (_, b)| {
                    match compare_original_members(a, b, recipe.strings()) {
                        Ok(order) => order,
                        Err(error) => {
                            failure = Some(error);
                            std::cmp::Ordering::Equal
                        }
                    }
                });
                if let Some(error) = failure {
                    return Err(error);
                }
                members
                    .into_iter()
                    .map(|(_, original)| original.clone())
                    .collect()
            };
        let header = Value::native_list_constructor(members, recipe.strings());
        // SortPropList reaches GetElements even for an empty native NULL header.
        drop(header.native_object_list_elements(recipe.strings())?);
        let lease = header.native_lifetime_lease();
        let cache = self.oo.property_caches.entry((target, class)).or_default();
        if cache.epoch != epoch {
            cache.readable = None;
            cache.writable = None;
            cache.epoch = epoch;
        }
        if writable {
            cache.writable = Some(header);
        } else {
            cache.readable = Some(header);
        }
        Ok(lease)
    }
}

impl Vm {
    pub(super) fn native_declared_property_header(
        &mut self,
        target: OoId,
        class: bool,
        writable: bool,
    ) -> Result<Value, ValueError> {
        let protocol = self
            .actual_native_invocation_dialect()
            .native_property_lookup_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native property metadata issuer",
            ))?
            .strings();
        let mut members: Vec<_> = self
            .oo
            .property_members
            .get(&(target, class, writable))
            .map(|members| members.values().collect())
            .unwrap_or_default();
        let mut failure = None;
        members.sort_by(|a, b| match compare_original_members(a, b, protocol) {
            Ok(order) => order,
            Err(error) => {
                failure = Some(error);
                std::cmp::Ordering::Equal
            }
        });
        if let Some(error) = failure {
            return Err(error);
        }
        let header =
            Value::native_list_constructor(members.into_iter().cloned().collect(), protocol);
        drop(header.native_object_list_elements(protocol)?);
        Ok(header)
    }
}

pub(super) fn lookup_property(
    vm: &mut Vm,
    target: OoId,
    original: &Value,
    writable: bool,
    table: &mut Option<Value>,
    retain_table: bool,
) -> Result<crate::value::NativeObjectLifetimeLease, Completion<Value>> {
    let operation = || ValueError::CommandProtocolUnavailable("native property lookup issuer");
    let recipe = vm
        .actual_native_invocation_dialect()
        .native_property_lookup_protocol()
        .ok_or_else(operation)
        .map_err(|e| crate::command::completion_from_cmd_error(vm, e.into()))?;
    recipe
        .validate_index_origin(original.native_index_cache().map(|(_, origin)| origin))
        .map_err(|e| crate::command::completion_from_cmd_error(vm, e.into()))?;
    let header = vm
        .native_all_property_header(target, false, writable)
        .map_err(|e| crate::command::completion_from_cmd_error(vm, e.into()))?;
    if retain_table && table.is_none() {
        *table = Some(
            header
                .value()
                .native_list_copy(recipe.strings())
                .map_err(|e| crate::command::completion_from_cmd_error(vm, e.into()))?,
        );
    }
    let names = table
        .as_ref()
        .unwrap_or(header.value())
        .native_object_list_elements(recipe.strings())
        .map_err(|e| crate::command::completion_from_cmd_error(vm, e.into()))?;
    let bytes = original
        .native_string_bytes(recipe.strings())
        .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable)
        .map_err(|e| crate::command::completion_from_cmd_error(vm, ValueError::from(e).into()))?;
    let words: Vec<_> = names
        .iter()
        .map(|name| {
            name.native_string_bytes(recipe.strings())
                .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable)
        })
        .collect::<Result<_, _>>()
        .map_err(|e| crate::command::completion_from_cmd_error(vm, ValueError::from(e).into()))?;
    let borrowed: Vec<_> = words.iter().map(std::convert::AsRef::as_ref).collect();
    match recipe.lookup(&bytes, &borrowed) {
        Ok(index) => selected_property_member(vm, header.value(), recipe.strings(), index),
        Err(message) => {
            let other = vm
                .native_all_property_header(target, false, !writable)
                .map_err(|e| crate::command::completion_from_cmd_error(vm, e.into()))?;
            let other = other
                .value()
                .native_object_list_elements(recipe.strings())
                .map_err(|e| crate::command::completion_from_cmd_error(vm, e.into()))?;
            let other_words: Vec<_> = other
                .iter()
                .map(|name| {
                    name.native_string_bytes(recipe.strings())
                        .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable)
                })
                .collect::<Result<_, _>>()
                .map_err(|e| {
                    crate::command::completion_from_cmd_error(vm, ValueError::from(e).into())
                })?;
            let other_borrowed: Vec<_> = other_words
                .iter()
                .map(std::convert::AsRef::as_ref)
                .collect();
            let mut error = message;
            if let Ok(index) = recipe.lookup(&bytes, &other_borrowed) {
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
                    protocol.error_code(b"property", &bytes),
                )
                .with_native_string_result(recipe.strings()),
            ))
        }
    }
}

fn selected_property_member(
    vm: &mut Vm,
    header: &Value,
    strings: tcl_syntax::native_string::NativeStringProtocol,
    index: usize,
) -> Result<crate::value::NativeObjectLifetimeLease, Completion<Value>> {
    let current = header
        .native_object_list_elements(strings)
        .map_err(|e| crate::command::completion_from_cmd_error(vm, e.into()))?;
    let selected = current.get(index).ok_or_else(|| {
        crate::command::completion_from_cmd_error(
            vm,
            ValueError::CommandProtocolUnavailable("property callback changed native table extent")
                .into(),
        )
    })?;
    Ok(selected.native_lifetime_lease())
}

pub(super) fn register_property(
    vm: &mut Vm,
    target: OoId,
    class: bool,
    name: &[u8],
    readable: bool,
    writable: bool,
) -> Result<(), ValueError> {
    let Some(recipe) = vm
        .actual_native_invocation_dialect()
        .native_property_lookup_protocol()
    else {
        return Ok(());
    };
    let mut bytes = b"-".to_vec();
    bytes.extend_from_slice(tcl_core_types::c_string_extent(name));
    let dashed = vm
        .oo
        .intern_method_key(tcl_core_types::NameBytes::from(bytes.clone()));
    let original = Value::new_native_string_bytes(bytes);
    let materialization = vm
        .actual_native_invocation_dialect()
        .native_string_materialization(None)
        .ok_or(ValueError::CommandProtocolUnavailable(
            "property formatted name issuer",
        ))?;
    original.retain_native_string_representation(materialization)?;
    let mut changed = false;
    for (direction, adding) in [(false, readable), (true, writable)] {
        let slot = vm
            .oo
            .property_members
            .entry((target, class, direction))
            .or_default();
        let present = slot.contains_key(&dashed);
        if !present && adding {
            slot.insert(dashed.clone(), original.clone());
            changed = true;
        }
        if present && !adding {
            slot.remove(&dashed);
            changed = true;
        }
        if present != adding
            && let Some(cache) = vm.oo.property_caches.get_mut(&(target, class))
        {
            if direction {
                cache.writable = None;
            } else {
                cache.readable = None;
            }
        }
    }
    if changed && class {
        vm.native_property_structure_changed(target, true);
    }
    let _ = recipe;
    Ok(())
}

pub(super) fn set_property_slot(
    vm: &mut Vm,
    target: OoId,
    class: bool,
    writable: bool,
    operation: &str,
    names: &[Value],
) -> Result<BTreeSet<String>, ValueError> {
    let recipe = vm
        .actual_native_invocation_dialect()
        .native_property_lookup_protocol()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "property metadata slot issuer",
        ))?;
    let mut incoming = Vec::new();
    for original in names {
        let bytes = original
            .native_string_bytes(recipe.strings())
            .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable)?;
        let name = vm
            .oo
            .intern_method_key(tcl_core_types::NameBytes::from(bytes.as_ref()));
        incoming.push((name, original));
    }
    let mut members = if matches!(operation, "-set" | "-clear") {
        BTreeMap::new()
    } else {
        vm.oo
            .property_members
            .get(&(target, class, writable))
            .cloned()
            .unwrap_or_default()
    };
    match operation {
        "-set" | "-append" => {
            for (name, original) in incoming {
                members.entry(name).or_insert_with(|| original.clone());
            }
        }
        "-remove" => {
            for (name, _) in incoming {
                members.remove(&name);
            }
        }
        "-clear" => {}
        _ => {
            return Err(ValueError::CommandProtocolUnavailable(
                "native property slot operation",
            ));
        }
    }
    let names = members.keys().cloned().collect();
    vm.install_original_property_members(target, class, writable, members.into_iter().collect());
    if class {
        vm.native_property_structure_changed(target, true);
    }
    Ok(names)
}

fn compare_original_members(
    a: &Value,
    b: &Value,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
) -> Result<std::cmp::Ordering, ValueError> {
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
    if a.native_object_identity() == b.native_object_identity() {
        return Ok(std::cmp::Ordering::Equal);
    }
    let sa = a.native_object_snapshot();
    let sb = b.native_object_snapshot();
    if matches!(sa.cache, Cache::String { .. }) && matches!(sb.cache, Cache::String { .. }) {
        let representation=tcl_registry::native_string_length::NativeStringLengthRepresentation::PureProperByteArray;
        let ca = a.native_character_count_with_protocol(protocol, representation)?;
        let cb = b.native_character_count_with_protocol(protocol, representation)?;
        if sa.resident.as_ref().is_some_and(|bytes| bytes.len() == ca)
            && sb.resident.as_ref().is_some_and(|bytes| bytes.len() == cb)
        {
            return Ok(sa.resident.cmp(&sb.resident));
        }
        return Ok(a
            .native_unicode_units(protocol)?
            .cmp(&b.native_unicode_units(protocol)?));
    }
    if let (Cache::ByteArray { bytes: a, .. }, Cache::ByteArray { bytes: b, .. }) =
        (&sa.cache, &sb.cache)
        && sa.resident.is_none()
        && sb.resident.is_none()
    {
        return Ok(a.cmp(b));
    }
    let a = a
        .native_string_bytes(protocol)
        .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable)?;
    let b = b
        .native_string_bytes(protocol)
        .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable)?;
    let utf = tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(tcl_dialect::TclVersion::V9_1);
    Ok(utf.decode_units(&a).cmp(&utf.decode_units(&b)))
}

pub(super) fn configure_all(vm: &mut Vm, target: OoId) -> Completion<Value> {
    let header = match vm.native_all_property_header(target, false, false) {
        Ok(header) => header,
        Err(e) => return crate::command::completion_from_cmd_error(vm, e.into()),
    };
    // The configure reader adds one actual List-header role across callbacks.
    let header = header.value().clone();
    let protocol = vm
        .actual_native_invocation_dialect()
        .native_property_lookup_protocol()
        .expect("selected properties")
        .strings();
    let names = match header.native_object_list_elements(protocol) {
        Ok(names) => names,
        Err(e) => return crate::command::completion_from_cmd_error(vm, e.into()),
    };
    let mut pairs = Vec::new();
    for original in names.iter() {
        let value = match read_original_property(vm, target, original) {
            Ok(value) => value,
            Err(error) => return error,
        };
        pairs.push((original.clone(), value));
    }
    if pairs.is_empty() {
        return ok(Value::empty());
    }
    match Value::native_dictionary_constructor(pairs, None, protocol) {
        Ok(value) => ok(value),
        Err(e) => crate::command::completion_from_cmd_error(vm, e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            let (reader, writer) = recipe.accessor_names(&canonical);
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
        vm.native_property_method_created(id, false);
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
        vm.native_property_structure_changed(class, true);
        assert_eq!(vm.oo.property_foundation_epoch, epoch + 1);
    }
}

/// Actual GetterType/SetterType clientData. Native method records share this
/// allocation during invocation; a new cloned declaration duplicates its role.
pub(super) struct NativePropertyAccessor {
    pub(super) original: Value,
    pub(super) writable: bool,
}
pub(super) fn default_method(original: &Value, writable: bool) -> Method {
    Method {
        params: Vec::new(),
        has_args: false,
        compiled_body: None,
        body_src: Value::empty(),
        forward: None,
        property: Some(Rc::new(NativePropertyAccessor {
            original: original.clone(),
            writable,
        })),
        exported: false,
    }
}

fn default_storage_name(
    vm: &Vm,
    target: OoId,
    original: &Value,
    strings: tcl_syntax::native_string::NativeStringProtocol,
) -> Result<Option<Vec<u8>>, ValueError> {
    let Some(frame) = native_context::current_method(vm) else {
        return Ok(None);
    };
    let Some(step) = frame.chain.get(frame.index) else {
        return Ok(None);
    };
    let variables = if step.is_object {
        if step.provider != target {
            return Ok(None);
        }
        vm.oo
            .objects
            .get(&target)
            .map(|object| &object.private_variables)
    } else {
        let reachable = vm.oo.is_a(target, step.provider)
            || vm.oo.objects.get(&target).is_some_and(|object| {
                object.mixins.iter().any(|mixin| {
                    vm.oo
                        .class_linear_of(*mixin)
                        .iter()
                        .any(|entry| entry.provider == step.provider)
                })
            });
        if !reachable {
            return Ok(None);
        }
        vm.oo
            .classes
            .get(&step.provider)
            .map(|class| &class.private_variables)
    };
    if let Some(variables) = variables {
        for variable in variables {
            if compare_original_members(&variable.original, original, strings)?
                == std::cmp::Ordering::Equal
            {
                let creation = vm
                    .oo
                    .objects
                    .get(&step.provider)
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "private property variable provider",
                    ))?
                    .creation_id;
                return Ok(Some(
                    private_storage_name_bytes(creation, variable.name.as_bytes())
                        .as_bytes()
                        .to_vec(),
                ));
            }
        }
    }
    Ok(None)
}

pub(super) fn invoke_default(
    vm: &mut Vm,
    target: OoId,
    accessor: &NativePropertyAccessor,
    args: &[Value],
    original_argv: &[Value],
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
        .native_property_lookup_protocol()
    else {
        return crate::command::completion_from_cmd_error(
            vm,
            ValueError::CommandProtocolUnavailable("native default property accessor").into(),
        );
    };
    let bytes = match accessor.original.native_string_bytes(recipe.strings()) {
        Ok(bytes) => bytes,
        Err(error) => {
            return crate::command::completion_from_cmd_error(
                vm,
                tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error).into(),
            );
        }
    };
    let Some(object) = vm.oo.objects.get(&target) else {
        return crate::command::completion_from_cmd_error(
            vm,
            ValueError::CommandProtocolUnavailable("property object namespace").into(),
        );
    };
    let namespace = object.ns.clone();
    let mapped = match default_storage_name(vm, target, &accessor.original, recipe.strings()) {
        Ok(mapped) => mapped,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error.into()),
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
        let property = match lookup_property(vm, target, &pair[0], true, &mut table, true) {
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
        .expect("selected property accessor");
    let accessor = property
        .native_property_accessor(recipe, writable)
        .map_err(|e| crate::command::completion_from_cmd_error(vm, e.into()))?;
    // TclOO property thunks own the same my and accessor objects for the call.
    let accessor = accessor.value().clone();
    let Some(head) = vm.oo.property_my_name.clone() else {
        return Err(crate::command::completion_from_cmd_error(
            vm,
            ValueError::CommandProtocolUnavailable("Foundation original my name").into(),
        ));
    };
    let method = native_method_key(vm, &accessor)?;
    let bytes = property
        .native_string_bytes(recipe.strings())
        .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable)
        .map_err(|e| crate::command::completion_from_cmd_error(vm, ValueError::from(e).into()))?;
    let result = {
        let invoked = Value::from_native_string_bytes(display_oo_bytes(vm, target));
        let args = value.map_or_else(Vec::new, |value| {
            vec![value.native_lifetime_lease().into_value()]
        });
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
    match result.code {
        Code::Ok => Ok(result.result),
        Code::Break | Code::Continue => {
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

pub(super) fn define_original_properties(
    vm: &mut Vm,
    class: bool,
    target: OoId,
    args: &[Value],
) -> Completion<Value> {
    let recipe = vm
        .actual_native_invocation_dialect()
        .native_property_lookup_protocol()
        .expect("selected original property definition");
    let options = recipe.definition_options();
    let kinds = recipe.definition_kinds();
    let mut cursor = 0;
    while cursor < args.len() {
        let original = &args[cursor];
        let name = match vm.native_name_operand_bytes(original) {
            Ok(name) => name,
            Err(error) => {
                return crate::command::completion_from_cmd_error(
                    vm,
                    tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error).into(),
                );
            }
        };
        cursor += 1;
        let mut kind = TclOoPropertyKind::ReadWrite;
        let mut getter = None;
        let mut setter = None;
        while cursor < args.len() {
            let bytes = match vm.native_name_operand_bytes(&args[cursor]) {
                Ok(bytes) => bytes,
                Err(error) => {
                    return crate::command::completion_from_cmd_error(
                        vm,
                        tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error).into(),
                    );
                }
            };
            if bytes.first() != Some(&b'-') {
                break;
            }
            let option = match vm.native_index_operand(&args[cursor], &options, false, "option") {
                Ok(option) => option,
                Err(error) => return crate::command::completion_from_cmd_error(vm, error),
            };
            cursor += 1;
            let Some(value) = args.get(cursor) else {
                let message = if option == 1 {
                    b"missing kind value to go with -kind option".as_slice()
                } else if option == 0 {
                    b"missing body to go with -get option".as_slice()
                } else {
                    b"missing body to go with -set option".as_slice()
                };
                return crate::command::completion_from_cmd_error(
                    vm,
                    tcl_cmd_core::CmdError::with_error_code_bytes(
                        message.to_vec(),
                        b"TCL WRONGARGS".to_vec(),
                    )
                    .with_native_string_result(recipe.strings()),
                );
            };
            match option {
                0 => getter = Some(value.clone()),
                2 => setter = Some(value.clone()),
                1 => {
                    kind = match vm.native_index_operand(value, &kinds, false, "kind") {
                        Ok(0) => TclOoPropertyKind::Readable,
                        Ok(1) => TclOoPropertyKind::ReadWrite,
                        Ok(2) => TclOoPropertyKind::Writable,
                        Ok(_) => unreachable!("closed native kind roster"),
                        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
                    };
                }
                _ => unreachable!("closed native property option roster"),
            }
            cursor += 1;
        }
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
}
