// SPDX-License-Identifier: AGPL-3.0-or-later
//! Real C9.1 property metadata owners and native List cache headers.
use super::*;
use tcl_registry::native_property_lookup::{
    NativePropertyGraphDependents, NativePropertyInvalidation,
};
use tcl_syntax::value::ValueError;

#[derive(Default)]
pub(super) struct NativePropertyCache {
    epoch: u64,
    readable: Option<obj::Owned>,
    writable: Option<obj::Owned>,
}
pub(super) struct PropertyHeaderLease {
    pub(super) header: *mut TclObj,
    _lifetime: obj::NativeObjectLifetime,
}
impl PropertyHeaderLease {
    fn new(header: *mut TclObj) -> Self {
        Self {
            header,
            _lifetime: obj::NativeObjectLifetime::retain(header),
        }
    }
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
                    .expect("Foundation epoch exhausted")
            }
            NativePropertyInvalidation::Instance => {
                self.property_caches.remove(&(target, false));
            }
            NativePropertyInvalidation::ClassRepresentative => {
                self.property_caches.remove(&(target, true));
            }
        }
    }
    fn collect_native_declared_properties(
        &self,
        target: OoId,
        class: bool,
        writable: bool,
        names: &[Vec<u8>],
        out: &mut BTreeMap<Vec<u8>, *mut TclObj>,
    ) -> Result<(), ValueError> {
        for name in names {
            if out.contains_key(name) {
                continue;
            }
            let original = self
                .property_members
                .get(&(target, class, writable))
                .and_then(|members| members.get(name))
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original property metadata member",
                ))?;
            out.insert(name.clone(), original.as_ptr());
        }
        Ok(())
    }
    fn collect_native_class_properties(
        &self,
        target: OoId,
        writable: bool,
        seen: &mut BTreeSet<OoId>,
        out: &mut BTreeMap<Vec<u8>, *mut TclObj>,
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
            if writable {
                &class.writable_properties
            } else {
                &class.readable_properties
            },
            out,
        )?;
        if Some(target) == self.object_root {
            return Ok(());
        }
        for mixin in &class.mixins {
            self.collect_native_class_properties(*mixin, writable, seen, out)?;
        }
        for sup in &class.supers {
            self.collect_native_class_properties(*sup, writable, seen, out)?;
        }
        Ok(())
    }
}
impl Interp {
    pub(super) fn native_property_structure_changed(&mut self, target: OoId, class: bool) {
        self.native_method_structure_changed(target, class);
        let Some(recipe) = self
            .native_invocation_dialect()
            .native_property_lookup_protocol()
        else {
            return;
        };
        let action = {
            let oo = self.oo.borrow();
            recipe.structure_changed(
                class,
                NativePropertyGraphDependents {
                    subclasses: oo
                        .classes
                        .iter()
                        .any(|(id, c)| *id != target && c.supers.contains(&target)),
                    instances: oo
                        .objects
                        .values()
                        .any(|o| o.class == target || o.mixins.contains(&target)),
                    mixin_dependents: oo.classes.values().any(|c| c.mixins.contains(&target)),
                },
                oo.objects
                    .get(&target)
                    .is_some_and(|o| !o.mixins.is_empty()),
            )
        };
        self.oo
            .borrow_mut()
            .apply_property_invalidation(target, action);
    }
    pub(super) fn native_property_method_created(&mut self, target: OoId, class: bool) {
        self.native_method_created(target, class);
        if let Some(recipe) = self
            .native_invocation_dialect()
            .native_property_lookup_protocol()
        {
            self.oo
                .borrow_mut()
                .apply_property_invalidation(target, recipe.method_created(class));
        }
    }
    pub(super) fn install_original_property_members(
        &mut self,
        target: OoId,
        class: bool,
        writable: bool,
        members: Vec<(Vec<u8>, obj::Owned)>,
    ) {
        let mut originals = BTreeMap::new();
        for (name, original) in members {
            originals.entry(name).or_insert(original);
        }
        let mut oo = self.oo.borrow_mut();
        oo.property_members
            .insert((target, class, writable), originals);
        if let Some(cache) = oo.property_caches.get_mut(&(target, class)) {
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
    ) -> Result<PropertyHeaderLease, ValueError> {
        let recipe = self
            .native_invocation_dialect()
            .native_property_lookup_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native property List issuer",
            ))?;
        let epoch = self.oo.borrow().property_foundation_epoch;
        {
            let oo = self.oo.borrow();
            if let Some(cache) = oo.property_caches.get(&(target, class)) {
                if cache.epoch == epoch {
                    if let Some(header) = if writable {
                        &cache.writable
                    } else {
                        &cache.readable
                    } {
                        return Ok(PropertyHeaderLease::new(header.as_ptr()));
                    }
                }
            }
        }
        let header = {
            let oo = self.oo.borrow();
            let mut members = BTreeMap::new();
            let mut seen = BTreeSet::new();
            if class {
                oo.collect_native_class_properties(target, writable, &mut seen, &mut members)?;
            } else {
                let object =
                    oo.objects
                        .get(&target)
                        .ok_or(ValueError::CommandProtocolUnavailable(
                            "native property object",
                        ))?;
                oo.collect_native_declared_properties(
                    target,
                    false,
                    writable,
                    if writable {
                        &object.writable_properties
                    } else {
                        &object.readable_properties
                    },
                    &mut members,
                )?;
                for mixin in &object.mixins {
                    oo.collect_native_class_properties(*mixin, writable, &mut seen, &mut members)?;
                }
                oo.collect_native_class_properties(
                    object.class,
                    writable,
                    &mut seen,
                    &mut members,
                )?;
            }
            let mut members: Vec<_> = members.into_iter().collect();
            let mut failure = None;
            members.sort_by(|(_, a), (_, b)| {
                match compare_original_members(*a, *b, recipe.strings()) {
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
            let members: Vec<_> = members.into_iter().map(|(_, original)| original).collect();
            obj::Owned::fresh(crate::list::new_list_obj_native(&members, recipe.strings()))
        };
        drop(crate::list::list_elements_native_checked(
            header.as_ptr(),
            recipe.strings(),
        )?);
        let lease = PropertyHeaderLease::new(header.as_ptr());
        let mut oo = self.oo.borrow_mut();
        let cache = oo.property_caches.entry((target, class)).or_default();
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

impl Interp {
    pub(super) fn native_declared_property_header(
        &mut self,
        target: OoId,
        class: bool,
        writable: bool,
    ) -> Result<obj::Owned, ValueError> {
        let protocol = self
            .native_invocation_dialect()
            .native_property_lookup_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native property metadata issuer",
            ))?
            .strings();
        let oo = self.oo.borrow();
        let mut members: Vec<_> = oo
            .property_members
            .get(&(target, class, writable))
            .map(|members| members.values().map(obj::Owned::as_ptr).collect())
            .unwrap_or_default();
        let mut failure = None;
        members.sort_by(|a, b| match compare_original_members(*a, *b, protocol) {
            Ok(order) => order,
            Err(error) => {
                failure = Some(error);
                std::cmp::Ordering::Equal
            }
        });
        if let Some(error) = failure {
            return Err(error);
        }
        let header = obj::Owned::fresh(crate::list::new_list_obj_native(&members, protocol));
        drop(crate::list::list_elements_native_checked(
            header.as_ptr(),
            protocol,
        )?);
        Ok(header)
    }
}

pub(super) fn lookup_property(
    interp: &mut Interp,
    target: OoId,
    original: *mut TclObj,
    writable: bool,
    table: &mut Option<obj::Owned>,
    retain_table: bool,
) -> Result<PropertyHeaderLease, Code> {
    let recipe = interp
        .native_invocation_dialect()
        .native_property_lookup_protocol()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "native property lookup issuer",
        ))
        .map_err(|e| interp.report_cmd_error(e.into()))?;
    recipe
        .validate_index_origin(obj::native_index::cache(original).map(|(_, origin)| origin))
        .map_err(|e| interp.report_cmd_error(e.into()))?;
    let header = interp
        .native_all_property_header(target, false, writable)
        .map_err(|e| interp.report_cmd_error(e.into()))?;
    if retain_table && table.is_none() {
        *table = Some(
            crate::list::native_list_copy(header.header, recipe.strings())
                .map_err(|e| interp.report_cmd_error(e.into()))?,
        );
    }
    let names = crate::list::list_elements_native_checked(
        table.as_ref().map_or(header.header, obj::Owned::as_ptr),
        recipe.strings(),
    )
    .map_err(|e| interp.report_cmd_error(e.into()))?;
    let bytes = interp
        .native_object_string_bytes(original)
        .map_err(|e| interp.report_cmd_error(e.into()))?;
    let words: Vec<_> = names
        .iter()
        .map(|&name| interp.native_object_string_bytes(name))
        .collect::<Result<_, _>>()
        .map_err(|e| interp.report_cmd_error(e.into()))?;
    let borrowed: Vec<_> = words.iter().map(|word| word.as_ref()).collect();
    match recipe.lookup(&bytes, &borrowed) {
        Ok(index) => {
            let current =
                crate::list::list_elements_native_checked(header.header, recipe.strings())
                    .map_err(|e| interp.report_cmd_error(e.into()))?;
            let selected = current.get(index).ok_or_else(|| {
                interp.report_cmd_error(
                    ValueError::CommandProtocolUnavailable(
                        "property callback changed native table extent",
                    )
                    .into(),
                )
            })?;
            Ok(PropertyHeaderLease::new(*selected))
        }
        Err(message) => {
            let other = interp
                .native_all_property_header(target, false, !writable)
                .map_err(|e| interp.report_cmd_error(e.into()))?;
            let other = crate::list::list_elements_native_checked(other.header, recipe.strings())
                .map_err(|e| interp.report_cmd_error(e.into()))?;
            let other_words: Vec<_> = other
                .iter()
                .map(|&name| interp.native_object_string_bytes(name))
                .collect::<Result<_, _>>()
                .map_err(|e| interp.report_cmd_error(e.into()))?;
            let other_borrowed: Vec<_> = other_words.iter().map(|word| word.as_ref()).collect();
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
            let protocol = interp
                .native_invocation_dialect()
                .native_index_lookup_protocol()
                .expect("selected C91 property Index recipe");
            Err(interp.report_cmd_error(
                tcl_cmd_core::CmdError::with_error_code_bytes(
                    error,
                    protocol.error_code(b"property", &bytes),
                )
                .with_native_string_result(recipe.strings()),
            ))
        }
    }
}

pub(super) fn register_property(
    interp: &mut Interp,
    target: OoId,
    class: bool,
    name: &[u8],
    readable: bool,
    writable: bool,
) -> Result<(), ValueError> {
    if interp
        .native_invocation_dialect()
        .native_property_lookup_protocol()
        .is_none()
    {
        return Ok(());
    }
    let original = obj::Owned::fresh(obj::new_string_bytes(name));
    let materialization = interp
        .native_invocation_dialect()
        .native_string_materialization(None)
        .ok_or(ValueError::CommandProtocolUnavailable(
            "property formatted name issuer",
        ))?;
    obj::retain_native_string_representation(original.as_ptr(), materialization)?;
    let mut changed = false;
    for (direction, adding) in [(false, readable), (true, writable)] {
        let mut oo = interp.oo.borrow_mut();
        let slot = oo
            .property_members
            .entry((target, class, direction))
            .or_default();
        let present = slot.contains_key(name);
        if !present && adding {
            slot.insert(name.to_vec(), original.clone());
            changed = true;
        }
        if present && !adding {
            slot.remove(name);
            changed = true;
        }
        if present != adding {
            if let Some(cache) = oo.property_caches.get_mut(&(target, class)) {
                if direction {
                    cache.writable = None;
                } else {
                    cache.readable = None;
                }
            }
        }
        let list = if class {
            oo.classes.get_mut(&target).map(|c| {
                if direction {
                    &mut c.writable_properties
                } else {
                    &mut c.readable_properties
                }
            })
        } else {
            oo.objects.get_mut(&target).map(|o| {
                if direction {
                    &mut o.writable_properties
                } else {
                    &mut o.readable_properties
                }
            })
        };
        if let Some(list) = list {
            if adding && !present {
                list.push(name.to_vec());
            } else if !adding {
                list.retain(|entry| entry != name);
            }
        }
    }
    if changed && class {
        interp.native_property_structure_changed(target, true);
    }
    Ok(())
}

pub(super) fn set_property_slot(
    interp: &mut Interp,
    target: OoId,
    class: bool,
    writable: bool,
    original: *mut TclObj,
) -> Result<(), ValueError> {
    let recipe = interp
        .native_invocation_dialect()
        .native_property_lookup_protocol()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "property metadata slot issuer",
        ))?;
    let members = crate::list::list_elements_native_checked(original, recipe.strings())?;
    let mut names = Vec::new();
    let mut originals = BTreeMap::new();
    for original in members {
        let name = interp.native_object_string_bytes(original)?.to_vec();
        if let std::collections::btree_map::Entry::Vacant(entry) = originals.entry(name) {
            names.push(entry.key().clone());
            entry.insert(obj::Owned::retain(original));
        }
    }
    {
        let mut oo = interp.oo.borrow_mut();
        let list = if class {
            oo.classes.get_mut(&target).map(|c| {
                if writable {
                    &mut c.writable_properties
                } else {
                    &mut c.readable_properties
                }
            })
        } else {
            oo.objects.get_mut(&target).map(|o| {
                if writable {
                    &mut o.writable_properties
                } else {
                    &mut o.readable_properties
                }
            })
        };
        let list = list.ok_or(ValueError::CommandProtocolUnavailable(
            "native property slot target",
        ))?;
        *list = names;
    }
    interp.install_original_property_members(
        target,
        class,
        writable,
        originals.into_iter().collect(),
    );
    if class {
        interp.native_property_structure_changed(target, true);
    }
    Ok(())
}

fn compare_original_members(
    a: *mut TclObj,
    b: *mut TclObj,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
) -> Result<std::cmp::Ordering, ValueError> {
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
    if a == b {
        return Ok(std::cmp::Ordering::Equal);
    }
    let sa = obj::native_object_snapshot(a)?;
    let sb = obj::native_object_snapshot(b)?;
    if matches!(sa.cache, Cache::String { .. }) && matches!(sb.cache, Cache::String { .. }) {
        let representation=tcl_registry::native_string_length::NativeStringLengthRepresentation::PureProperByteArray;
        let ca = obj::native_character_count(a, protocol, representation)?;
        let cb = obj::native_character_count(b, protocol, representation)?;
        if sa.resident.as_ref().is_some_and(|bytes| bytes.len() == ca)
            && sb.resident.as_ref().is_some_and(|bytes| bytes.len() == cb)
        {
            return Ok(sa.resident.cmp(&sb.resident));
        }
        return Ok(
            obj::native_unicode_units(a, protocol)?.cmp(&obj::native_unicode_units(b, protocol)?)
        );
    }
    if let (Cache::ByteArray { bytes: a, .. }, Cache::ByteArray { bytes: b, .. }) =
        (&sa.cache, &sb.cache)
    {
        if sa.resident.is_none() && sb.resident.is_none() {
            return Ok(a.cmp(b));
        }
    }
    let a = crate::dict::native_object_bytes(a, protocol)?;
    let b = crate::dict::native_object_bytes(b, protocol)?;
    let version = protocol
        .tcl_version()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "property member string comparison",
        ))?;
    let utf = tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version);
    Ok(utf.decode_units(&a).cmp(&utf.decode_units(&b)))
}

pub(super) fn configure_all(interp: &mut Interp, target: OoId) -> Code {
    let header = match interp.native_all_property_header(target, false, false) {
        Ok(header) => header,
        Err(e) => return interp.report_cmd_error(e.into()),
    };
    let header = obj::Owned::retain(header.header);
    let protocol = interp
        .native_invocation_dialect()
        .native_property_lookup_protocol()
        .expect("selected properties")
        .strings();
    let names = match crate::list::list_elements_native_checked(header.as_ptr(), protocol) {
        Ok(names) => names,
        Err(e) => return interp.report_cmd_error(e.into()),
    };
    let mut owned = Vec::new();
    let mut pairs = Vec::new();
    for original in names {
        let code = read_original_property(interp, target, original);
        if code != Code::Ok {
            return code;
        }
        let value = obj::Owned::retain(interp.get_obj_result());
        owned.push(obj::Owned::retain(original));
        pairs.push((original, value.as_ptr()));
        owned.push(value);
    }
    if pairs.is_empty() {
        interp.set_result_bytes(b"");
        return Code::Ok;
    }
    match crate::dict::new_dict_obj_native(&pairs, None, protocol) {
        Ok(value) => {
            interp.set_result(value);
            Code::Ok
        }
        Err(e) => interp.report_cmd_error(e.into()),
    }
}

/// The actual standard accessor's retained declaration object. Method snapshots
/// share this allocation; only cloning a native declaration duplicates its role.
pub(super) struct NativePropertyAccessor {
    pub(super) original: obj::Owned,
    pub(super) writable: bool,
}
impl NativePropertyAccessor {
    pub(super) fn new(original: *mut TclObj, writable: bool) -> Self {
        Self {
            original: obj::Owned::retain(original),
            writable,
        }
    }
}
fn default_storage_name(
    interp: &Interp,
    target: OoId,
    original: *mut TclObj,
    strings: tcl_syntax::native_string::NativeStringProtocol,
) -> Result<Option<Vec<u8>>, ValueError> {
    let activation = interp.frames.borrow().current_activation();
    let scope = {
        let state = interp.oo.borrow();
        state
            .call_stack
            .iter()
            .rev()
            .find(|frame| frame.activation == Some(activation))
            .and_then(|frame| frame.chain.get(frame.index))
            .map(|step| (step.provider, step.is_object))
    };
    let Some((provider, is_object)) = scope else {
        return Ok(None);
    };
    if is_object && provider != target {
        return Ok(None);
    }
    if !is_object {
        let (class, mixins) = match interp.oo.borrow().objects.get(&target) {
            Some(object) => (object.class, object.mixins.clone()),
            None => return Ok(None),
        };
        if !interp.class_precedence(class).contains(&provider)
            && !mixins
                .iter()
                .any(|mixin| interp.class_precedence(*mixin).contains(&provider))
        {
            return Ok(None);
        }
    }
    let state = interp.oo.borrow();
    let variables = if is_object {
        state
            .objects
            .get(&provider)
            .map(|object| &object.private_variables)
    } else {
        state
            .classes
            .get(&provider)
            .map(|class| &class.private_variables)
    };
    if let Some(variables) = variables {
        for variable in variables {
            if compare_original_members(variable.original.as_ptr(), original, strings)?
                == std::cmp::Ordering::Equal
            {
                let creation = state
                    .objects
                    .get(&provider)
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "private property variable provider",
                    ))?
                    .creation_id;
                let mut storage = format!("{creation} : ").into_bytes();
                storage.extend_from_slice(&variable.name);
                return Ok(Some(storage));
            }
        }
    }
    Ok(None)
}

pub(super) fn invoke_default(
    interp: &mut Interp,
    target: OoId,
    accessor: &NativePropertyAccessor,
    args: &[*mut TclObj],
    original_argv: Option<&[*mut TclObj]>,
) -> Code {
    if args.len() != usize::from(accessor.writable) {
        let Some(words) = original_argv else {
            return interp.report_cmd_error(
                ValueError::CommandProtocolUnavailable("default property original invocation")
                    .into(),
            );
        };
        let header = words.get(..2).unwrap_or(words);
        let usage = match interp.argument_usage_prefix(header, header.len()) {
            Ok(mut usage) => {
                if accessor.writable {
                    usage.extend_from_slice(b" value");
                }
                usage
            }
            Err(error) => return error,
        };
        return wrong_args(interp, &usage);
    }
    let Some(recipe) = interp
        .native_invocation_dialect()
        .native_property_name_protocol()
    else {
        return interp.report_cmd_error(
            ValueError::CommandProtocolUnavailable("native default property accessor").into(),
        );
    };
    let bytes = match interp.native_object_string_bytes(accessor.original.as_ptr()) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let namespace = interp
        .oo
        .borrow()
        .objects
        .get(&target)
        .map(|object| object.var_ns);
    let Some(namespace) = namespace else {
        return interp.report_cmd_error(
            ValueError::CommandProtocolUnavailable("property object namespace").into(),
        );
    };
    let namespace = interp.ns_qualified_name(namespace);
    let mapped =
        match default_storage_name(interp, target, accessor.original.as_ptr(), recipe.strings()) {
            Ok(mapped) => mapped,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
    let variable_bytes = mapped.as_deref().unwrap_or(&bytes);
    let generated = recipe
        .object_variable_operand(variable_bytes, &namespace)
        .map(|bytes| obj::Owned::fresh(obj::new_string_bytes(&bytes)));
    let lookup = generated
        .as_ref()
        .map_or(accessor.original.as_ptr(), obj::Owned::as_ptr);
    if accessor.writable {
        match interp.store_original_c_property_variable(lookup, accessor.original.as_ptr(), args[0])
        {
            Ok(()) => Code::Ok,
            Err(error) => error,
        }
    } else {
        match interp.read_original_c_property_variable(lookup, accessor.original.as_ptr()) {
            Ok(value) => {
                interp.set_result(value);
                Code::Ok
            }
            Err(error) => error,
        }
    }
}

pub(super) fn configure_native(interp: &mut Interp, target: OoId, args: &[*mut TclObj]) -> Code {
    if args.is_empty() {
        return configure_all(interp, target);
    }
    if args.len() > 1 && args.len() % 2 != 0 {
        return wrong_args(interp, b"configure ?-option value ...?");
    }
    let mut table = None;
    if args.len() == 1 {
        let property = match lookup_property(interp, target, args[0], false, &mut table, false) {
            Ok(property) => property,
            Err(error) => return error,
        };
        return read_original_property(interp, target, property.header);
    }
    for pair in args.chunks_exact(2) {
        let property = match lookup_property(interp, target, pair[0], true, &mut table, true) {
            Ok(property) => property,
            Err(error) => return error,
        };
        let code = invoke_original_accessor(interp, target, property.header, true, Some(pair[1]));
        if code != Code::Ok {
            return code;
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}
fn invoke_original_accessor(
    interp: &mut Interp,
    target: OoId,
    property: *mut TclObj,
    writable: bool,
    value: Option<*mut TclObj>,
) -> Code {
    let recipe = interp
        .native_invocation_dialect()
        .native_property_lookup_protocol()
        .expect("selected property accessor");
    let accessor = match obj::native_property_name::accessor(property, recipe, writable) {
        Ok(accessor) => obj::Owned::retain(accessor),
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let head = interp.oo.borrow().property_my_name.as_ref().cloned();
    let Some(head) = head else {
        return interp.report_cmd_error(
            ValueError::CommandProtocolUnavailable("Foundation original my name").into(),
        );
    };
    let method = match interp.native_object_string_bytes(accessor.as_ptr()) {
        Ok(method) => method,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let mut argv = vec![head.as_ptr(), accessor.as_ptr()];
    if let Some(value) = value {
        argv.push(value);
    }
    let code =
        interp.oo_invoke_with_original(target, &method, value.as_slice(), false, None, Some(&argv));
    let name = match interp.native_object_string_bytes(property) {
        Ok(name) => name,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    property_loopword_error(
        interp,
        code,
        if writable { b"setter" } else { b"getter" },
        &name,
    )
}
fn read_original_property(interp: &mut Interp, target: OoId, property: *mut TclObj) -> Code {
    invoke_original_accessor(interp, target, property, false, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn instance() -> Interp {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("tcl9.1"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(interp.eval_str(b"oo::configurable create C {property yellow -get {return Y}; property zinc -get {return Z}}; C create o"),Code::Ok,"{:?}",interp.result_bytes());
        interp
    }
    #[test]
    fn property_headers_and_temporary_tables_retain_original_members() {
        let mut interp = instance();
        let id = interp.oo_resolve_object(b"o");
        let header = interp.native_all_property_header(id, false, false).unwrap();
        let strings = interp
            .native_invocation_dialect()
            .native_property_lookup_protocol()
            .unwrap()
            .strings();
        let members = crate::list::list_elements_native_checked(header.header, strings).unwrap();
        let first = members[0];
        let refs = unsafe { (*first).ref_count };
        let copy = crate::list::native_list_copy(header.header, strings).unwrap();
        let copied = crate::list::list_elements_native_checked(copy.as_ptr(), strings).unwrap();
        assert_ne!(copy.as_ptr(), header.header);
        assert_eq!(copied[0], first);
        assert_eq!(unsafe { (*first).ref_count }, refs);
        let caller = obj::Owned::fresh(obj::new_string_bytes(b"-y"));
        let mut table = None;
        assert_eq!(
            {
                let selected =
                    lookup_property(&mut interp, id, caller.as_ptr(), false, &mut table, true)
                        .unwrap();
                interp.native_object_string_bytes(selected.header).unwrap()
            }
            .as_ref(),
            b"-yellow"
        );
        assert_eq!(
            obj::native_object_snapshot(caller.as_ptr()).unwrap().cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::None
        );
        assert!(table.is_some());
        assert_eq!(
            interp
                .native_all_property_header(id, false, false)
                .unwrap()
                .header,
            header.header
        );
    }
    #[test]
    fn opaque_property_lookup_uses_original_accessor_members() {
        // Native proof: naming.property.opaque-name-custom-getter-option-cache
        // docs/design/analysis/name-resolution-proofs/property-opaque-name-custom-getter-option-cache.md
        let mut interp = instance();
        let target = interp.oo_resolve_object(b"o");
        for name in [b"x\xff".as_slice(), b"x\0tail".as_slice()] {
            let words = [
                b"oo::define".as_slice(),
                b"C",
                b"property",
                name,
                b"-get",
                b"return RAW",
            ];
            let owners: Vec<_> = words
                .into_iter()
                .map(|word| obj::Owned::fresh(obj::new_string_bytes(word)))
                .collect();
            let argv: Vec<_> = owners.iter().map(obj::Owned::as_ptr).collect();
            assert_eq!(interp.dispatch(&argv), Code::Ok);
            let mut dashed = b"-".to_vec();
            dashed.extend_from_slice(name);
            let caller = obj::Owned::fresh(obj::new_string_bytes(&dashed));
            assert_eq!(
                configure_native(&mut interp, target, &[caller.as_ptr()]),
                Code::Ok
            );
            assert_eq!(interp.result_bytes(), b"RAW");
            assert_eq!(
                obj::native_object_snapshot(caller.as_ptr()).unwrap().cache,
                tcl_syntax::native_object::NativeObjectCacheSnapshot::None
            );
            let mut table = None;
            let selected = lookup_property(
                &mut interp,
                target,
                caller.as_ptr(),
                false,
                &mut table,
                false,
            )
            .unwrap();
            assert!(obj::native_property_name::is_cached(selected.header));
        }
        let rows = include_str!("../../tests/data/native_property_opaque/native.tsv");
        assert_eq!(rows, "opaque-0\tRAW\tnone\nopaque-1\tRAW\tnone\n");
    }
    #[test]
    fn default_property_methods_retain_original_clientdata_and_leave_its_primary() {
        // Native proof: naming.property.original-accessor-clientdata
        // docs/design/analysis/name-resolution-proofs/property-original-accessor-clientdata.md
        let mut interp = instance();
        let class = interp.oo_resolve_object(b"C");
        let target = interp.oo_resolve_object(b"o");
        let recipe = interp
            .native_invocation_dialect()
            .native_property_lookup_protocol()
            .unwrap();
        for name in [b"p".as_slice(), b"x\xff".as_slice(), b"x\0tail".as_slice()] {
            let declaration = obj::Owned::fresh(obj::new_string_bytes(name));
            let prefix: Vec<_> = [b"oo::define".as_slice(), b"C", b"property"]
                .into_iter()
                .map(|word| obj::Owned::fresh(obj::new_string_bytes(word)))
                .collect();
            let mut argv: Vec<_> = prefix.iter().map(obj::Owned::as_ptr).collect();
            argv.push(declaration.as_ptr());
            assert_eq!(
                interp.dispatch(&argv),
                Code::Ok,
                "{:?}",
                interp.result_bytes()
            );
            let mut canonical = b"-".to_vec();
            canonical.extend_from_slice(tcl_core_types::c_string_extent(name));
            let (reader, writer) = recipe.member_accessor_names(&canonical);
            for key in [reader, writer] {
                let method = interp.oo.borrow().classes[&class]
                    .methods
                    .get(key.as_slice())
                    .unwrap();
                let Method::Property(property) = method else {
                    panic!("native property method");
                };
                assert_eq!(property.original.as_ptr(), declaration.as_ptr());
            }
            assert_eq!(unsafe { (*declaration.as_ptr()).ref_count }, 3);
            let mut query = b"-".to_vec();
            query.extend_from_slice(name);
            let query = obj::Owned::fresh(obj::new_string_bytes(&query));
            let supplied = obj::Owned::fresh(obj::new_string_bytes(b"RAW"));
            assert_eq!(
                configure_native(&mut interp, target, &[query.as_ptr(), supplied.as_ptr()]),
                Code::Ok,
                "{:?}",
                interp.result_bytes()
            );
            assert!(interp.result_bytes().is_empty());
            assert_eq!(
                configure_native(&mut interp, target, &[query.as_ptr()]),
                Code::Ok,
                "{:?}",
                interp.result_bytes()
            );
            assert_eq!(interp.result_obj(), supplied.as_ptr());
            assert_eq!(unsafe { (*declaration.as_ptr()).ref_count }, 3);
            assert_eq!(
                obj::native_object_snapshot(declaration.as_ptr())
                    .unwrap()
                    .cache,
                tcl_syntax::native_object::NativeObjectCacheSnapshot::None
            );
            assert_eq!(
                obj::native_object_snapshot(query.as_ptr()).unwrap().cache,
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
        // Native proof: naming.property.original-foundation-epoch-and-cache
        // docs/design/analysis/name-resolution-proofs/property-original-foundation-epoch-and-cache.md
        let mut interp = instance();
        let id = interp.oo_resolve_object(b"o");
        let class = interp.oo_resolve_object(b"C");
        let old = interp.native_all_property_header(id, false, false).unwrap();
        let epoch = interp.oo.borrow().property_foundation_epoch;
        interp.native_property_method_created(id, false);
        assert_eq!(interp.oo.borrow().property_foundation_epoch, epoch);
        assert_eq!(
            interp
                .native_all_property_header(id, false, false)
                .unwrap()
                .header,
            old.header
        );
        assert_eq!(
            interp.eval_str(
                b"catch {oo::define C {::oo::define::method live {} {return};error LATE}}"
            ),
            Code::Ok
        );
        assert_eq!(interp.oo.borrow().property_foundation_epoch, epoch + 1);
        assert_eq!(
            interp.oo.borrow().property_caches[&(id, false)]
                .readable
                .as_ref()
                .unwrap()
                .as_ptr(),
            old.header
        );
        assert_ne!(
            interp
                .native_all_property_header(id, false, false)
                .unwrap()
                .header,
            old.header
        );
        let epoch = interp.oo.borrow().property_foundation_epoch;
        interp.native_property_structure_changed(class, true);
        assert_eq!(interp.oo.borrow().property_foundation_epoch, epoch + 1);
    }
}
