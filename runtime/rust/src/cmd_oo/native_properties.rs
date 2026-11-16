// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Real C9.1 property metadata owners and native List cache headers.
use super::{
    native_context, obj, property_loopword_error, wrong_args, BTreeMap, BTreeSet, Code, Interp,
    OoId, OoState, Rc, TclObj,
};
use crate::interp::native_arguments::OriginalArguments;
use crate::interp::native_operation_currency::NativeOperationScope;
use tcl_cmd_core::CmdError;
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
/// A report retains its actual operation through the caller's result transfer.
struct PropertyReportOwner {
    scope: NativeOperationScope,
    target: native_context::OriginalOoTarget,
    object: OoId,
    class: bool,
    writable: bool,
    all: bool,
    epoch: u64,
    mutation: (Rc<()>, u64, u64),
    members: BTreeMap<Vec<u8>, *mut TclObj>,
    deriving: std::cell::Cell<bool>,
}
impl PropertyReportOwner {
    fn enter(
        interpreter: &Interp,
        object: OoId,
        class: bool,
        writable: bool,
        all: bool,
    ) -> Result<Self, CmdError> {
        let scope = NativeOperationScope::enter(interpreter)?;
        let target = native_context::OriginalOoTarget::capture(interpreter, object, class)?;
        let state = interpreter.oo.borrow();
        let members = property_members(&state, object, class, writable, all)?;
        Ok(Self {
            scope,
            target,
            object,
            class,
            writable,
            all,
            epoch: state.property_foundation_epoch,
            mutation: state.native_methods.mutation_stamp(object),
            members,
            deriving: std::cell::Cell::new(true),
        })
    }
    fn check(&self, interpreter: &Interp) -> Result<(), CmdError> {
        self.scope.currency().ensure_current_or_refuse()?;
        self.target.check(interpreter)?;
        if !self.deriving.get() {
            return Ok(());
        }
        let state = interpreter.oo.borrow();
        let mutation = state.native_methods.mutation_stamp(self.object);
        if state.property_foundation_epoch != self.epoch
            || !Rc::ptr_eq(&self.mutation.0, &mutation.0)
            || self.mutation.1 != mutation.1
            || self.mutation.2 != mutation.2
            || property_members(&state, self.object, self.class, self.writable, self.all)?
                != self.members
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "changed original property report world",
            )
            .into());
        }
        Ok(())
    }
    fn freeze(&self) {
        self.deriving.set(false);
    }

    fn leases(&self) -> Result<Vec<obj::NativeObjectLifetime>, CmdError> {
        self.members
            .values()
            .map(|&original| {
                obj::check_native_liveness(original)?;
                Ok(obj::NativeObjectLifetime::retain(original))
            })
            .collect()
    }
}

pub(super) struct PropertyHeaderLease {
    pub(super) header: *mut TclObj,
    _lifetime: obj::NativeObjectLifetime,
    _owned: Option<obj::Owned>,
    report: Option<Rc<PropertyReportOwner>>,
}
impl PropertyHeaderLease {
    fn new(header: *mut TclObj) -> Self {
        Self {
            header,
            _lifetime: obj::NativeObjectLifetime::retain(header),
            _owned: None,
            report: None,
        }
    }
    pub(super) fn check(&self, interpreter: &Interp) -> Result<(), CmdError> {
        obj::check_native_liveness(self.header)?;
        match &self.report {
            Some(report) => report.check(interpreter),
            None => interpreter
                .check_entered_native_operation()
                .map_err(CmdError::from_execution_refusal),
        }
    }
}

fn property_members(
    state: &OoState,
    target: OoId,
    class: bool,
    writable: bool,
    all: bool,
) -> Result<BTreeMap<Vec<u8>, *mut TclObj>, ValueError> {
    let mut members = BTreeMap::new();
    if !all {
        if let Some(originals) = state.property_members.get(&(target, class, writable)) {
            members.extend(
                originals
                    .iter()
                    .map(|(name, original)| (name.clone(), original.as_ptr())),
            );
        }
    } else if class {
        state.collect_native_class_properties(
            target,
            writable,
            &mut BTreeSet::new(),
            &mut members,
        )?;
    } else {
        let object = state
            .objects
            .get(&target)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native property object",
            ))?;
        state.collect_native_declared_properties(
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
        let mut seen = BTreeSet::new();
        for mixin in &object.mixins {
            state.collect_native_class_properties(*mixin, writable, &mut seen, &mut members)?;
        }
        state.collect_native_class_properties(object.class, writable, &mut seen, &mut members)?;
    }
    Ok(members)
}

fn sorted_property_members(
    interpreter: &Interp,
    owner: &PropertyReportOwner,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
) -> Result<Vec<*mut TclObj>, CmdError> {
    let _leases = owner.leases()?;
    let mut members: Vec<_> = owner.members.values().copied().collect();
    let mut first = None;
    members.sort_by(|a, b| {
        if first.is_some() {
            return std::cmp::Ordering::Equal;
        }
        match compare_original_members(*a, *b, protocol, &|| owner.check(interpreter)) {
            Ok(order) => order,
            Err(error) => {
                first = Some(error);
                std::cmp::Ordering::Equal
            }
        }
    });
    owner.check(interpreter)?;
    match first {
        Some(first) => Err(first),
        None => Ok(members),
    }
}

impl OoState {
    fn apply_property_invalidation(
        &mut self,
        target: OoId,
        action: NativePropertyInvalidation,
    ) -> Option<NativePropertyCache> {
        match action {
            NativePropertyInvalidation::None => None,
            NativePropertyInvalidation::Foundation => {
                self.property_foundation_epoch = self
                    .property_foundation_epoch
                    .checked_add(1)
                    .expect("Foundation epoch exhausted");
                None
            }
            NativePropertyInvalidation::Instance => self.property_caches.remove(&(target, false)),
            NativePropertyInvalidation::ClassRepresentative => {
                self.property_caches.remove(&(target, true))
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
    pub(super) fn native_property_structure_changed(
        &mut self,
        target: OoId,
        class: bool,
    ) -> Result<(), CmdError> {
        let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(self)?;
        let owner = native_context::OriginalOoTarget::capture(self, target, class)?;
        self.native_method_structure_changed(target, class);
        scope.currency().ensure_current_or_refuse()?;
        owner.check(self)?;
        let Some(recipe) = self
            .native_invocation_dialect()
            .native_property_lookup_protocol()
        else {
            return Ok(());
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
        let retired = self
            .oo
            .borrow_mut()
            .apply_property_invalidation(target, action);
        drop(retired);
        scope.currency().ensure_current_or_refuse()?;
        owner.check(self)
    }

    pub(super) fn native_property_method_created(
        &mut self,
        target: OoId,
        class: bool,
    ) -> Result<(), CmdError> {
        let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(self)?;
        let owner = native_context::OriginalOoTarget::capture(self, target, class)?;
        self.native_method_created(target, class);
        scope.currency().ensure_current_or_refuse()?;
        owner.check(self)?;
        if let Some(recipe) = self
            .native_invocation_dialect()
            .native_property_lookup_protocol()
        {
            let retired = self
                .oo
                .borrow_mut()
                .apply_property_invalidation(target, recipe.method_created(class));
            drop(retired);
            scope.currency().ensure_current_or_refuse()?;
            owner.check(self)?;
        }
        Ok(())
    }

    /// Mandatory original property-role cleanup, including reached refusal.
    /// A torn-down target receives no fresh live-target or invocation authority.
    pub(super) fn retire_native_properties(&mut self, target: OoId) -> Result<(), CmdError> {
        use tcl_registry::native_property_lookup::NativePropertyStorageRetirementStep as Stage;
        let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(self);
        if let Err(first) = &scope {
            self.refuse_native_execution(first.clone());
        }
        let check = || match &scope {
            Ok(scope) => scope.currency().ensure_current_or_refuse(),
            Err(first) => Err(first.clone()),
        };
        let (method_cache, mut members, mut caches, recipes) = {
            let mut oo = self.oo.borrow_mut();
            let method_cache = oo.native_methods.retire(target);
            let members = [false, true].map(|class| {
                [false, true].map(|writable| oo.property_members.remove(&(target, class, writable)))
            });
            let caches = [false, true].map(|class| oo.property_caches.remove(&(target, class)));
            let recipes = [false, true].map(|class| oo.property_origins.remove(&(target, class)));
            (method_cache, members, caches, recipes)
        };
        let mut first = check().err();
        for class in 0..2 {
            if let Some(recipe) = recipes[class] {
                for stage in recipe.storage_retirement() {
                    match stage {
                        Stage::ReadableCache => drop(
                            caches[class]
                                .as_mut()
                                .and_then(|cache| cache.readable.take()),
                        ),
                        Stage::WritableCache => drop(
                            caches[class]
                                .as_mut()
                                .and_then(|cache| cache.writable.take()),
                        ),
                        Stage::ReadableMembers => drop(members[class][0].take()),
                        Stage::WritableMembers => drop(members[class][1].take()),
                    }
                    let reached = check().err();
                    if first.is_none() {
                        first = reached;
                    }
                }
            } else {
                // Unissued compatibility storage receives only ordinary Rust
                // ownership cleanup, without a selected Native release recipe.
                drop(members[class][0].take());
                drop(members[class][1].take());
                drop(caches[class].take());
                let reached = check().err();
                if first.is_none() {
                    first = reached;
                }
            }
        }
        drop(method_cache);
        let reached = check().err();
        if first.is_none() {
            first = reached;
        }
        match first {
            Some(first) => Err(CmdError::from_execution_refusal(first)),
            None => Ok(()),
        }
    }

    /// Release the selected cached List before the incoming slot references.
    fn retire_original_property_cache(
        &mut self,
        target: OoId,
        class: bool,
        writable: bool,
        originals: &OriginalArguments,
        owner: &native_context::OriginalOoTarget,
    ) -> Result<(), CmdError> {
        originals.check()?;
        owner.check(self)?;
        let retired = self
            .oo
            .borrow_mut()
            .property_caches
            .get_mut(&(target, class))
            .and_then(|cache| {
                if writable {
                    cache.writable.take()
                } else {
                    cache.readable.take()
                }
            });
        drop(retired);
        originals.check()?;
        owner.check(self)
    }

    /// Extract the old property-list roles, then retire them outside OO borrows.
    fn retire_original_property_members(
        &mut self,
        target: OoId,
        class: bool,
        writable: bool,
        originals: &OriginalArguments,
        owner: &native_context::OriginalOoTarget,
    ) -> Result<(), CmdError> {
        originals.check()?;
        owner.check(self)?;
        let retired = self
            .oo
            .borrow_mut()
            .property_members
            .remove(&(target, class, writable));
        drop(retired);
        originals.check()?;
        owner.check(self)
    }

    pub(super) fn install_original_property_members(
        &mut self,
        target: OoId,
        class: bool,
        writable: bool,
        members: BTreeMap<Vec<u8>, obj::Owned>,
        originals: &OriginalArguments,
        owner: &native_context::OriginalOoTarget,
    ) -> Result<(), CmdError> {
        originals.check()?;
        owner.check(self)?;
        let recipe = self
            .native_invocation_dialect()
            .native_property_lookup_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "property metadata origin",
            ))?;
        let retired = {
            let mut oo = self.oo.borrow_mut();
            oo.property_origins.entry((target, class)).or_insert(recipe);
            oo.property_members
                .insert((target, class, writable), members)
        };
        // A reentrant definition can have published its own new members.
        drop(retired);
        originals.check()?;
        owner.check(self)
    }
    pub(super) fn native_all_property_header(
        &mut self,
        target: OoId,
        class: bool,
        writable: bool,
    ) -> Result<PropertyHeaderLease, CmdError> {
        let report = Rc::new(PropertyReportOwner::enter(
            self, target, class, writable, true,
        )?);
        let outcome = (|| -> Result<PropertyHeaderLease, CmdError> {
            let recipe = self
                .native_invocation_dialect()
                .native_property_lookup_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "native property List issuer",
                ))?;
            let cached = {
                let state = self.oo.borrow();
                state
                    .property_caches
                    .get(&(target, class))
                    .filter(|cache| cache.epoch == report.epoch)
                    .and_then(|cache| {
                        if writable {
                            cache.writable.as_ref()
                        } else {
                            cache.readable.as_ref()
                        }
                    })
                    .map(|header| PropertyHeaderLease::new(header.as_ptr()))
            };
            if let Some(mut cached) = cached {
                report.check(self)?;
                report.freeze();
                cached.report = Some(Rc::clone(&report));
                return Ok(cached);
            }
            let member_leases = report.leases()?;
            let members = sorted_property_members(self, &report, recipe.strings())?;
            let header =
                obj::Owned::fresh(crate::list::new_list_obj_native(&members, recipe.strings()));
            let prepared = crate::list::list_elements_native_with_currency(
                header.as_ptr(),
                recipe.strings(),
                Some(report.scope.currency()),
            );
            report.check(self)?;
            drop(prepared?);
            drop(member_leases);
            report.check(self)?;
            let stale = {
                let mut state = self.oo.borrow_mut();
                if state
                    .property_caches
                    .get(&(target, class))
                    .is_some_and(|cache| cache.epoch != report.epoch)
                {
                    state.property_caches.remove(&(target, class))
                } else {
                    None
                }
            };
            drop(stale);
            report.check(self)?;
            // A reentrant reporting getter can have installed a current cache.
            let displaced = {
                let mut state = self.oo.borrow_mut();
                state
                    .property_caches
                    .get_mut(&(target, class))
                    .and_then(|cache| {
                        if writable {
                            cache.writable.take()
                        } else {
                            cache.readable.take()
                        }
                    })
            };
            drop(displaced);
            report.check(self)?;
            let mut lease = PropertyHeaderLease::new(header.as_ptr());
            {
                let mut state = self.oo.borrow_mut();
                state
                    .property_origins
                    .entry((target, class))
                    .or_insert(recipe);
                let cache = state.property_caches.entry((target, class)).or_default();
                cache.epoch = report.epoch;
                if writable {
                    cache.writable = Some(header);
                } else {
                    cache.readable = Some(header);
                }
            }
            report.check(self)?;
            report.freeze();
            lease.report = Some(Rc::clone(&report));
            Ok(lease)
        })();
        report.check(self)?;
        outcome
    }

    pub(super) fn native_declared_property_header(
        &mut self,
        target: OoId,
        class: bool,
        writable: bool,
    ) -> Result<PropertyHeaderLease, CmdError> {
        let report = Rc::new(PropertyReportOwner::enter(
            self, target, class, writable, false,
        )?);
        let outcome = (|| -> Result<PropertyHeaderLease, CmdError> {
            let recipe = self
                .native_invocation_dialect()
                .native_property_lookup_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "native property metadata issuer",
                ))?;
            let member_leases = report.leases()?;
            let members = sorted_property_members(self, &report, recipe.strings())?;
            let header =
                obj::Owned::fresh(crate::list::new_list_obj_native(&members, recipe.strings()));
            let prepared = crate::list::list_elements_native_with_currency(
                header.as_ptr(),
                recipe.strings(),
                Some(report.scope.currency()),
            );
            report.check(self)?;
            drop(prepared?);
            drop(member_leases);
            report.check(self)?;
            let mut lease = PropertyHeaderLease::new(header.as_ptr());
            lease._owned = Some(header);
            report.freeze();
            lease.report = Some(Rc::clone(&report));
            Ok(lease)
        })();
        report.check(self)?;
        outcome
    }
}

pub(super) struct RetainedPropertyTable {
    header: obj::Owned,
    report: Rc<PropertyReportOwner>,
    words: Vec<Rc<[u8]>>,
}
impl RetainedPropertyTable {
    fn check(&self, interpreter: &Interp) -> Result<(), CmdError> {
        self.report.check(interpreter)?;
        obj::check_native_liveness(self.header.as_ptr())?;
        Ok(())
    }
}

pub(super) fn lookup_property(
    interp: &mut Interp,
    target: OoId,
    original: *mut TclObj,
    writable: bool,
    table: &mut Option<RetainedPropertyTable>,
    retain_table: bool,
) -> Result<PropertyHeaderLease, CmdError> {
    let originals = OriginalArguments::enter(interp, &[original])?;
    let recipe = interp
        .native_invocation_dialect()
        .native_property_lookup_protocol()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "native property lookup issuer",
        ))?;
    recipe.validate_index_origin(obj::native_index::cache(original).map(|(_, origin)| origin))?;
    if let Some(table) = table.as_ref() {
        table.check(interp)?;
    }
    let header = interp.native_all_property_header(target, false, writable)?;
    let header_arguments = OriginalArguments::enter(interp, &[header.header])?;
    let observer = interp.clone();
    let check = || -> Result<(), CmdError> {
        originals.check_for(&observer)?;
        header_arguments.check_original_for(&observer, 0, header.header)?;
        header.check(&observer)
    };
    let words = if let Some(table) = table.as_ref() {
        table.check(interp)?;
        table.words.clone()
    } else {
        let names = crate::list::list_elements_native_with_currency(
            header.header,
            recipe.strings(),
            Some(originals.currency()),
        );
        check()?;
        let names = names?;
        let member_arguments = OriginalArguments::enter(interp, &names)?;
        let mut words = Vec::new();
        for index in 0..names.len() {
            let word = member_arguments.bytes(index);
            check()?;
            words.push(word?);
        }
        if retain_table {
            let copied = crate::list::native_list_copy_using(
                header.header,
                recipe.strings(),
                &check,
                &|error| originals.retain_refusal(error.into()),
            );
            check()?;
            *table = Some(RetainedPropertyTable {
                header: copied?,
                report: Rc::clone(header.report.as_ref().ok_or(
                    ValueError::CommandProtocolUnavailable("original property table report owner"),
                )?),
                words: words.clone(),
            });
        }
        words
    };
    let bytes = originals.bytes(0)?;
    check()?;
    let borrowed: Vec<_> = words.iter().map(|word| word.as_ref()).collect();
    let verdict = recipe.lookup(&bytes, &borrowed);
    check()?;
    match verdict {
        Ok(index) => {
            let current = crate::list::list_elements_native_with_currency(
                header.header,
                recipe.strings(),
                Some(originals.currency()),
            );
            check()?;
            let current = current?;
            let selected = *current
                .get(index)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "property callback changed native table extent",
                ))?;
            let mut selected = PropertyHeaderLease::new(selected);
            selected.report = header.report.clone();
            Ok(selected)
        }
        Err(message) => {
            let other = interp.native_all_property_header(target, false, !writable)?;
            check()?;
            let other_members = crate::list::list_elements_native_with_currency(
                other.header,
                recipe.strings(),
                Some(originals.currency()),
            );
            check()?;
            other.check(interp)?;
            let other_members = other_members?;
            let other_arguments = OriginalArguments::enter(interp, &other_members)?;
            let mut other_words = Vec::new();
            for index in 0..other_members.len() {
                let word = other_arguments.bytes(index);
                check()?;
                other.check(interp)?;
                other_words.push(word?);
            }
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
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "selected C91 property Index recipe",
                ))?;
            check()?;
            Err(
                CmdError::with_error_code_bytes(error, protocol.error_code(b"property", &bytes))
                    .with_native_string_result(recipe.strings()),
            )
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
) -> Result<(), CmdError> {
    let Some(recipe) = interp
        .native_invocation_dialect()
        .native_property_lookup_protocol()
    else {
        return Ok(());
    };
    let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(interp)?;
    let owner = native_context::OriginalOoTarget::capture(interp, target, class)?;
    let original = obj::Owned::fresh(obj::new_string_bytes(name));
    let materialization = interp
        .native_invocation_dialect()
        .native_string_materialization(None)
        .ok_or(ValueError::CommandProtocolUnavailable(
            "property formatted name issuer",
        ))?;
    let prepared = obj::retain_native_string_representation(original.as_ptr(), materialization);
    scope.currency().ensure_current_or_refuse()?;
    owner.check(interp)?;
    prepared?;
    let mut changed = false;
    for (direction, adding) in [(false, readable), (true, writable)] {
        scope.currency().ensure_current_or_refuse()?;
        owner.check(interp)?;
        let (present, retired) = {
            let mut oo = interp.oo.borrow_mut();
            oo.property_origins.entry((target, class)).or_insert(recipe);
            let slot = oo
                .property_members
                .entry((target, class, direction))
                .or_default();
            let present = slot.contains_key(name);
            let retired = if !present && adding {
                slot.insert(name.to_vec(), original.clone())
            } else if present && !adding {
                slot.remove(name)
            } else {
                None
            };
            (present, retired)
        };
        changed |= present != adding;
        drop(retired);
        scope.currency().ensure_current_or_refuse()?;
        owner.check(interp)?;
        if present != adding {
            let retired = interp
                .oo
                .borrow_mut()
                .property_caches
                .get_mut(&(target, class))
                .and_then(|cache| {
                    if direction {
                        cache.writable.take()
                    } else {
                        cache.readable.take()
                    }
                });
            drop(retired);
            scope.currency().ensure_current_or_refuse()?;
            owner.check(interp)?;
        }
        let mut oo = interp.oo.borrow_mut();
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
        interp.native_property_structure_changed(target, true)?;
    }
    scope.currency().ensure_current_or_refuse()?;
    owner.check(interp)
}

pub(super) fn set_property_slot(
    interp: &mut Interp,
    target: OoId,
    class: bool,
    writable: bool,
    original: *mut TclObj,
) -> Result<(), CmdError> {
    let originals = OriginalArguments::enter(interp, &[original])?;
    let owner = native_context::OriginalOoTarget::capture(interp, target, class)?;
    let recipe = interp
        .native_invocation_dialect()
        .native_property_lookup_protocol()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "property metadata slot issuer",
        ))?;
    let result: Result<(), CmdError> = (|| {
        let members = crate::list::list_elements_native_with_currency(
            original,
            recipe.strings(),
            Some(originals.currency()),
        );
        originals.check()?;
        owner.check(interp)?;
        let members = members?;
        let member_arguments = OriginalArguments::enter(interp, &members)?;
        interp.retire_original_property_cache(target, class, writable, &originals, &owner)?;
        // SetPropertyList takes every incoming property-list reference before
        // retiring old members or reaching any object-hash String getter.
        let incoming = (0..members.len())
            .map(|index| member_arguments.original(index).map(obj::Owned::retain))
            .collect::<Result<Vec<_>, _>>()?;
        interp.retire_original_property_members(target, class, writable, &originals, &owner)?;
        let mut names = Vec::new();
        let mut retained = BTreeMap::new();
        // The temporary Tcl_Obj hash table has an independent native key role.
        // Its first-win keys survive later hash getters and retire after adoption.
        let mut temporary_keys = BTreeMap::new();
        for (index, incoming) in incoming.into_iter().enumerate() {
            let name = member_arguments.bytes(index)?.to_vec();
            owner.check(interp)?;
            if let std::collections::btree_map::Entry::Vacant(entry) = retained.entry(name) {
                names.push(entry.key().clone());
                temporary_keys.insert(entry.key().clone(), obj::Owned::retain(incoming.as_ptr()));
                entry.insert(incoming);
            } else {
                drop(incoming);
                originals.check()?;
                owner.check(interp)?;
            }
        }
        originals.check()?;
        owner.check(interp)?;
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
            target, class, writable, retained, &originals, &owner,
        )?;
        drop(temporary_keys);
        originals.check()?;
        owner.check(interp)?;
        if class {
            interp.native_property_structure_changed(target, true)?;
        }
        Ok(())
    })();
    // Retained members and reached cleanup retire before this final check.
    originals.check()?;
    owner.check(interp)?;
    result
}

fn property_stage<T>(
    check: &impl Fn() -> Result<(), CmdError>,
    result: Result<T, ValueError>,
) -> Result<T, CmdError> {
    check()?;
    result.map_err(Into::into)
}

fn compare_original_members(
    a: *mut TclObj,
    b: *mut TclObj,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
    check: &impl Fn() -> Result<(), CmdError>,
) -> Result<std::cmp::Ordering, CmdError> {
    check()?;
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
    if a == b {
        return Ok(std::cmp::Ordering::Equal);
    }
    let sa = property_stage(check, obj::native_object_snapshot(a))?;
    let sb = property_stage(check, obj::native_object_snapshot(b))?;
    if matches!(sa.cache, Cache::String { .. }) && matches!(sb.cache, Cache::String { .. }) {
        let representation=tcl_registry::native_string_length::NativeStringLengthRepresentation::PureProperByteArray;
        let ca = property_stage(
            check,
            obj::native_character_count(a, protocol, representation),
        )?;
        let cb = property_stage(
            check,
            obj::native_character_count(b, protocol, representation),
        )?;
        if sa.resident.as_ref().is_some_and(|bytes| bytes.len() == ca)
            && sb.resident.as_ref().is_some_and(|bytes| bytes.len() == cb)
        {
            return Ok(sa.resident.cmp(&sb.resident));
        }
        return Ok(
            property_stage(check, obj::native_unicode_units(a, protocol))?.cmp(&property_stage(
                check,
                obj::native_unicode_units(b, protocol),
            )?),
        );
    }
    if let (Cache::ByteArray { bytes: a, .. }, Cache::ByteArray { bytes: b, .. }) =
        (&sa.cache, &sb.cache)
    {
        if sa.resident.is_none() && sb.resident.is_none() {
            return Ok(a.cmp(b));
        }
    }
    let a = property_stage(check, crate::dict::native_object_bytes(a, protocol))?;
    let b = property_stage(check, crate::dict::native_object_bytes(b, protocol))?;
    let version = protocol
        .tcl_version()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "property member string comparison",
        ))?;
    let utf = tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version);
    Ok(utf.decode_units(&a).cmp(&utf.decode_units(&b)))
}

pub(super) fn configure_all(interp: &mut Interp, target: OoId) -> Code {
    native_context::with_original_arguments(interp, &[], |interp, _originals| {
        configure_all_original(interp, target)
    })
}
fn configure_all_original(interp: &mut Interp, target: OoId) -> Code {
    let outcome: Result<Code, CmdError> = (|| {
        let header = interp.native_all_property_header(target, false, false)?;
        let working = obj::Owned::retain(header.header);
        let recipe = interp
            .native_invocation_dialect()
            .native_property_lookup_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "selected property configure",
            ))?;
        let names = crate::list::list_elements_native_with_currency(
            header.header,
            recipe.strings(),
            header.report.as_ref().map(|report| report.scope.currency()),
        );
        header.check(interp)?;
        let names = names?;
        let originals = OriginalArguments::enter(interp, &names)?;
        let mut owned = Vec::new();
        let mut pairs = Vec::new();
        for original in names {
            header.check(interp)?;
            let code = read_original_property(interp, target, original);
            originals.check()?;
            header.check(interp)?;
            if code != Code::Ok {
                return Ok(code);
            }
            let value = obj::Owned::retain(interp.get_obj_result());
            owned.push(obj::Owned::retain(original));
            pairs.push((original, value.as_ptr()));
            owned.push(value);
        }
        if pairs.is_empty() {
            interp.set_result_bytes(b"");
        } else {
            let value = crate::dict::new_dict_obj_native(&pairs, None, recipe.strings());
            originals.check()?;
            header.check(interp)?;
            interp.set_result(value?);
        }
        drop(owned);
        drop(working);
        originals.check()?;
        header.check(interp)?;
        Ok(Code::Ok)
    })();
    match outcome {
        Ok(code) => code,
        Err(error) => interp.report_cmd_error(error),
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
) -> Result<Option<Vec<u8>>, CmdError> {
    let originals = OriginalArguments::enter(interp, &[original])?;
    let target_owner = native_context::OriginalOoTarget::capture(interp, target, false)?;
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
    let provider_owner = native_context::OriginalOoTarget::capture(interp, provider, !is_object)?;
    let (creation, mutation, variables) = {
        let state = interp.oo.borrow();
        let source = if is_object {
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
        let variables = source
            .into_iter()
            .flatten()
            .map(|variable| {
                variable.check_originals()?;
                Ok(super::variable_mapping::observation(variable))
            })
            .collect::<Result<Vec<_>, ValueError>>()?;
        let creation = state
            .objects
            .get(&provider)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "private property variable provider",
            ))?
            .creation_id;
        (
            creation,
            state.native_methods.mutation_stamp(provider),
            variables,
        )
    };
    let check = || -> Result<(), CmdError> {
        originals.check()?;
        target_owner.check(interp)?;
        provider_owner.check(interp)?;
        let state = interp.oo.borrow();
        let current = state.native_methods.mutation_stamp(provider);
        let current_variables = if is_object {
            state
                .objects
                .get(&provider)
                .map(|owner| &owner.private_variables)
        } else {
            state
                .classes
                .get(&provider)
                .map(|owner| &owner.private_variables)
        };
        let same_inventory = current_variables.is_some_and(|current| {
            current
                .iter()
                .map(|variable| {
                    (
                        variable.original.as_ptr(),
                        variable.name.as_slice(),
                        variable
                            .full_name
                            .as_ref()
                            .map(|full| (full.original.as_ptr(), full.name.as_slice())),
                    )
                })
                .eq(variables.iter().map(|variable| {
                    (
                        variable.original.as_ptr(),
                        variable.name.as_slice(),
                        variable
                            .full_name
                            .as_ref()
                            .map(|full| (full.original.as_ptr(), full.name.as_slice())),
                    )
                }))
        });
        if !same_inventory
            || !Rc::ptr_eq(&mutation.0, &current.0)
            || mutation.1 != current.1
            || mutation.2 != current.2
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "changed original private property provider",
            )
            .into());
        }
        for variable in &variables {
            variable.check_originals()?;
        }
        Ok(())
    };
    for variable in &variables {
        if compare_original_members(variable.original.as_ptr(), original, strings, &check)?
            == std::cmp::Ordering::Equal
        {
            let storage = variable
                .full_name
                .as_ref()
                .map(|full| full.name.clone())
                .unwrap_or_else(|| {
                    let mut storage = format!("{creation} : ").into_bytes();
                    storage.extend_from_slice(&variable.name);
                    storage
                });
            check()?;
            return Ok(Some(storage));
        }
    }
    check()?;
    Ok(None)
}

pub(super) fn invoke_default(
    interp: &mut Interp,
    target: OoId,
    accessor: &NativePropertyAccessor,
    args: &[*mut TclObj],
    original_argv: Option<&[*mut TclObj]>,
) -> Code {
    let mut words = Vec::with_capacity(args.len() + 1);
    words.push(accessor.original.as_ptr());
    words.extend_from_slice(args);
    native_context::with_original_arguments(interp, &words, |interp, originals| {
        invoke_default_original(interp, target, accessor, args, original_argv, originals)
    })
}
fn invoke_default_original(
    interp: &mut Interp,
    target: OoId,
    accessor: &NativePropertyAccessor,
    args: &[*mut TclObj],
    original_argv: Option<&[*mut TclObj]>,
    originals: &OriginalArguments,
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
    let bytes = match originals.bytes(0) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error),
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
            Err(error) => return interp.report_cmd_error(error),
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
    native_context::with_original_arguments(interp, args, |interp, _originals| {
        configure_native_original(interp, target, args)
    })
}
fn configure_native_original(interp: &mut Interp, target: OoId, args: &[*mut TclObj]) -> Code {
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
            Err(error) => return interp.report_cmd_error(error),
        };
        return read_original_property(interp, target, property.header);
    }
    for pair in args.chunks_exact(2) {
        let property =
            match lookup_property(interp, target, pair[0], true, &mut table, args.len() > 2) {
                Ok(property) => property,
                Err(error) => return interp.report_cmd_error(error),
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
mod tests;

#[cfg(test)]
mod original_slot_boundary_tests;

#[cfg(test)]
#[path = "native_properties/original_order_tests.rs"]
mod original_order_tests;

#[cfg(test)]
mod original_copy_boundary_tests;
