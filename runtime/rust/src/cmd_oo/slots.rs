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

//! TIP 380 slots: the `::oo::Slot` class and its built-in instances, their accessors over the definition target, and the list operations.

use super::configurable::{read_property_set, write_property_set};
use super::introspection::set_list;
use super::{
    err, native_context, native_properties, variable_mapping, wrong_args, DeclaredVariable,
    DeclaredVariableHeader, DefTarget, Method, NativeMethod,
};
use crate::interp::{Code, Interp};
use crate::obj;
use crate::obj::TclObj;
use tcl_core_types::OoId;
use tcl_registry::native_tcloo_method_definition::NativeTclooMethodPayloadType;
use tcl_syntax::value::ValueOps;

/// Define `::oo::Slot` (TIP 380): the overridable `Get`/`Set`/`Resolve` and the
/// `unknown`-driven defaulting are pure Tcl; the `-set`/`-append`/… operations
/// are native so they add no call frame (the `info level` the ops observe
/// matches C). The default operation is `-append` (subclasses override it).
pub(super) fn install_slot_class(interp: &mut Interp) {
    // `--default-operation` is a forward (`my -append`) and `unknown` is a
    // native method (`slot_m_unknown`): like C's `TclOONewForwardMethod` /
    // `Slot_Unknown`, neither adds a Tcl call frame, so the `info level` the
    // overridable `Get`/`Set`/`Resolve` observe is the same as for a direct
    // operation. `destroy` is hidden too, so calling it on a slot is just
    // another datum routed through the default operation.
    let _ = interp.eval_str(
        b"oo::class create ::oo::Slot {\n\
            method Get {} { return {} }\n\
            method Set list { return }\n\
            method Resolve x { return $x }\n\
            forward --default-operation my -append\n\
            unexport Get Set Resolve --default-operation unknown destroy\n\
          }",
    );
    // Inject the native list operations + unknown handler as class methods.
    let ops: &[(&str, NativeMethod)] = &[
        ("-set", slot_m_set),
        ("-append", slot_m_append),
        ("-prepend", slot_m_prepend),
        ("-remove", slot_m_remove),
        ("-appendifnew", slot_m_appendifnew),
        ("-clear", slot_m_clear),
        ("unknown", slot_m_unknown),
    ];
    let slot_class = match interp.oo_resolve_object_bytes(b"::oo::Slot") {
        Ok(object) => object,
        Err(error) => {
            interp.report_cmd_error(error);
            return;
        }
    };
    if let Some(cl) = interp.oo.borrow_mut().classes.get_mut(&slot_class) {
        for name in ["Get", "Set", "Resolve"] {
            if let Some(owner) = cl.methods.owner(name.as_bytes()) {
                if let Method::Body { report_type, .. } = &mut *owner.borrow_mut() {
                    *report_type = NativeTclooMethodPayloadType::SlotOperation(name);
                }
            }
        }
        for (name, f) in ops {
            drop(cl.methods.insert_generated(
                name.as_bytes().to_vec(),
                Method::builtin(*f, NativeTclooMethodPayloadType::SlotOperation(name)),
            ));
        }
    }
}

/// The eleven built-in slot objects (`::oo::Slot` instances) that C's
/// `TclOODefineSlots` installs in every interp: the list-valued definition
/// slots and the TIP 558 property slots. `oo-1.21` / `oo-34.*` introspect them
/// (`info class instances ::oo::Slot`, per-slot method lists). Each gets
/// per-instance `Get`/`Set` (native, operating on the active definition target)
/// plus, for the class-reference slots, a `Resolve` (resolves a class name) and
/// a `--default-operation` forwarding to `-set`.
pub(super) fn install_slot_instances(interp: &mut Interp) {
    // (slot object name, is a class-reference slot: mixin/superclass)
    const SLOTS: &[(&[u8], &str, bool)] = &[
        (b"::oo::define::filter", "define::filter", false),
        (b"::oo::define::mixin", "define::mixin", true),
        (b"::oo::define::superclass", "define::superclass", true),
        (b"::oo::define::variable", "define::variable", false),
        (b"::oo::objdefine::filter", "objdefine::filter", false),
        (b"::oo::objdefine::mixin", "objdefine::mixin", true),
        (b"::oo::objdefine::variable", "objdefine::variable", false),
        (
            b"::oo::configuresupport::readableproperties",
            "configuresupport::readableproperties",
            false,
        ),
        (
            b"::oo::configuresupport::writableproperties",
            "configuresupport::writableproperties",
            false,
        ),
        (
            b"::oo::configuresupport::objreadableproperties",
            "configuresupport::objreadableproperties",
            false,
        ),
        (
            b"::oo::configuresupport::objwritableproperties",
            "configuresupport::objwritableproperties",
            false,
        ),
    ];
    for (name, descriptor, class_ref) in SLOTS {
        let mut script = b"::oo::Slot create ".to_vec();
        script.extend_from_slice(name);
        let _ = interp.eval_str(&script);
        let slot = match interp.oo_resolve_object_bytes(name) {
            Ok(object) => object,
            Err(error) => {
                interp.report_cmd_error(error);
                return;
            }
        };
        if let Some(o) = interp.oo.borrow_mut().objects.get_mut(&slot) {
            drop(o.methods.insert_generated(
                b"Get".to_vec(),
                Method::builtin(
                    slot_inst_get,
                    NativeTclooMethodPayloadType::SlotAccessor {
                        slot: descriptor,
                        accessor: "Getter",
                    },
                ),
            ));
            drop(o.methods.insert_generated(
                b"Set".to_vec(),
                Method::builtin(
                    slot_inst_set,
                    NativeTclooMethodPayloadType::SlotAccessor {
                        slot: descriptor,
                        accessor: "Setter",
                    },
                ),
            ));
            o.unexported.insert(b"Get".to_vec());
            o.unexported.insert(b"Set".to_vec());
            if *class_ref {
                drop(o.methods.insert_generated(
                    b"Resolve".to_vec(),
                    Method::builtin(
                        slot_inst_resolve,
                        NativeTclooMethodPayloadType::SlotAccessor {
                            slot: descriptor,
                            accessor: "Resolver",
                        },
                    ),
                ));
                o.unexported.insert(b"Resolve".to_vec());
                // Class-reference slots default to `-set` (not the base `-append`).
                drop(o.methods.insert_generated(
                    b"--default-operation".to_vec(),
                    Method::Forward {
                        prefix: obj::Owned::fresh(interp.new_list_object(&[
                            obj::new_string_bytes(b"my"),
                            obj::new_string_bytes(b"-set"),
                        ])),
                    },
                ));
                o.unexported.insert(b"--default-operation".to_vec());
            }
        }
    }
}

/// Classify a built-in slot object by name into the definition-target field it
/// reads/writes. `None` for the property slots (handled via `read/write_
/// property_set`) or an unknown name.
fn slot_property_kind(slot: &[u8]) -> Option<(bool, bool)> {
    // (want_object, writable)
    match slot {
        b"::oo::configuresupport::readableproperties" => Some((false, false)),
        b"::oo::configuresupport::writableproperties" => Some((false, true)),
        b"::oo::configuresupport::objreadableproperties" => Some((true, false)),
        b"::oo::configuresupport::objwritableproperties" => Some((true, true)),
        _ => None,
    }
}

/// Read the current contents of a built-in slot from the active definition
/// target (C's per-slot `*_Get`).
fn slot_field_read(interp: &Interp, slot: &[u8], target: &DefTarget) -> Vec<Vec<u8>> {
    if let Some((want_object, writable)) = slot_property_kind(slot) {
        return read_property_set(interp, target, want_object, writable);
    }
    let oo = interp.oo.borrow();
    match (slot, target) {
        (b"::oo::define::filter", DefTarget::Class(c)) => {
            oo.classes.get(c).map(|x| x.filters.clone())
        }
        (b"::oo::define::mixin", DefTarget::Class(c)) => oo
            .classes
            .get(c)
            .map(|x| x.mixins.iter().map(|id| oo.name(*id).to_vec()).collect()),
        (b"::oo::define::superclass", DefTarget::Class(c)) => oo
            .classes
            .get(c)
            .map(|x| x.supers.iter().map(|id| oo.name(*id).to_vec()).collect()),
        (b"::oo::define::variable", DefTarget::Class(c)) => oo
            .classes
            .get(c)
            .map(|x| x.variables.iter().map(|entry| entry.name.clone()).collect()),
        (b"::oo::objdefine::filter", DefTarget::Object(o)) => {
            oo.objects.get(o).map(|x| x.filters.clone())
        }
        (b"::oo::objdefine::mixin", DefTarget::Object(o)) => oo
            .objects
            .get(o)
            .map(|x| x.mixins.iter().map(|id| oo.name(*id).to_vec()).collect()),
        (b"::oo::objdefine::variable", DefTarget::Object(o)) => oo
            .objects
            .get(o)
            .map(|x| x.variables.iter().map(|entry| entry.name.clone()).collect()),
        _ => None,
    }
    .unwrap_or_default()
}

/// Write a built-in slot's contents back to the active definition target (C's
/// per-slot `*_Set`). The class-reference and filter/variable list slots store
/// the list verbatim; the property slots unique it (first occurrence wins).
fn slot_field_write(
    interp: &mut Interp,
    slot: &[u8],
    target: &DefTarget,
    list: Vec<Vec<u8>>,
) -> Result<(), tcl_cmd_core::CmdError> {
    if let Some((want_object, writable)) = slot_property_kind(slot) {
        let mut uniq: Vec<Vec<u8>> = Vec::with_capacity(list.len());
        for v in list {
            if !uniq.contains(&v) {
                uniq.push(v);
            }
        }
        write_property_set(interp, target, want_object, writable, uniq);
        return Ok(());
    }
    let class_refs = matches!(
        slot,
        b"::oo::define::mixin" | b"::oo::define::superclass" | b"::oo::objdefine::mixin"
    )
    .then(|| {
        list.iter()
            .map(|name| interp.oo_resolve_object_bytes(name))
            .collect::<Result<Vec<_>, _>>()
    })
    .transpose()?;
    let mut oo = interp.oo.borrow_mut();
    let object_root = oo.object_root;
    match (slot, target) {
        (b"::oo::define::filter", DefTarget::Class(c)) => {
            if let Some(x) = oo.classes.get_mut(c) {
                x.filters = list;
            }
        }
        (b"::oo::define::mixin", DefTarget::Class(c)) => {
            if let Some(x) = oo.classes.get_mut(c) {
                x.mixins = class_refs.unwrap_or_default();
            }
        }
        (b"::oo::define::superclass", DefTarget::Class(c)) => {
            if let Some(x) = oo.classes.get_mut(c) {
                x.supers = if list.is_empty() {
                    object_root.into_iter().collect()
                } else {
                    class_refs.unwrap_or_default()
                };
            }
        }
        (b"::oo::objdefine::filter", DefTarget::Object(o)) => {
            if let Some(x) = oo.objects.get_mut(o) {
                x.filters = list;
            }
        }
        (b"::oo::objdefine::mixin", DefTarget::Object(o)) => {
            if let Some(x) = oo.objects.get_mut(o) {
                x.mixins = class_refs.unwrap_or_default();
            }
        }
        _ => {}
    }
    if matches!(slot, b"::oo::objdefine::filter" | b"::oo::objdefine::mixin") {
        if let DefTarget::Object(id) = target {
            oo.recompute_native_method_class_cache(*id);
        }
    }
    drop(oo);
    if matches!(
        slot,
        b"::oo::define::filter"
            | b"::oo::define::mixin"
            | b"::oo::define::superclass"
            | b"::oo::objdefine::filter"
            | b"::oo::objdefine::mixin"
    ) {
        match target {
            DefTarget::Class(id) => interp.native_property_structure_changed(*id, true),
            DefTarget::Object(id) => interp.native_property_structure_changed(*id, false),
        }?;
    }
    Ok(())
}

fn is_variable_slot(slot: &[u8]) -> bool {
    matches!(
        slot,
        b"::oo::define::variable" | b"::oo::objdefine::variable"
    )
}

fn declared_variables(interp: &Interp, target: &DefTarget, private: bool) -> Vec<DeclaredVariable> {
    let oo = interp.oo.borrow();
    match target {
        DefTarget::Class(class) => oo.classes.get(class).map(|entry| {
            if private {
                &entry.private_variables
            } else {
                &entry.variables
            }
        }),
        DefTarget::Object(object) => oo.objects.get(object).map(|entry| {
            if private {
                &entry.private_variables
            } else {
                &entry.variables
            }
        }),
    }
    .map(|entries| entries.iter().map(variable_mapping::observation).collect())
    .unwrap_or_default()
}

pub(super) fn write_declared_variables(
    interp: &mut Interp,
    target: &DefTarget,
    private: bool,
    values: Vec<DeclaredVariable>,
    originals: &crate::interp::native_arguments::OriginalArguments,
    target_owner: &native_context::OriginalOoTarget,
) -> Result<(), tcl_cmd_core::CmdError> {
    originals.check()?;
    target_owner.check(interp)?;
    let mut retired = values;
    let published = {
        let mut oo = interp.oo.borrow_mut();
        let slot = match target {
            DefTarget::Class(class) => oo.classes.get_mut(class).map(|entry| {
                if private {
                    &mut entry.private_variables
                } else {
                    &mut entry.variables
                }
            }),
            DefTarget::Object(object) => oo.objects.get_mut(object).map(|entry| {
                if private {
                    &mut entry.private_variables
                } else {
                    &mut entry.variables
                }
            }),
        };
        if let Some(slot) = slot {
            std::mem::swap(slot, &mut retired);
            true
        } else {
            false
        }
    };
    drop(retired);
    originals.check()?;
    target_owner.check(interp)?;
    if !published {
        return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "TclOO variable declaration target",
        )
        .into());
    }
    Ok(())
}

pub(super) fn set_declared_variable_list(
    interp: &mut Interp,
    values: &[DeclaredVariable],
) -> Result<(), tcl_cmd_core::CmdError> {
    let operation = crate::interp::native_operation_currency::NativeOperationScope::enter(interp)
        .map_err(tcl_cmd_core::CmdError::from_execution_refusal)?;
    let mut members = Vec::new();
    for entry in values {
        obj::check_native_liveness(entry.original.as_ptr()).map_err(|error| {
            tcl_cmd_core::CmdError::from_execution_refusal(
                interp.refuse_completion_value_access(error),
            )
        })?;
        members.push(entry.original.as_ptr());
    }
    interp.set_result(interp.new_list_object(&members));
    operation
        .currency()
        .ensure_current_or_refuse()
        .map_err(tcl_cmd_core::CmdError::from_execution_refusal)
}

pub(super) fn prepare_declared_variables(
    interp: &mut Interp,
    values: &[*mut TclObj],
) -> Result<Vec<DeclaredVariable>, Code> {
    let mut retained = Vec::with_capacity(values.len());
    for &value in values {
        let name = interp
            .native_string_bytes(&value)
            .map_err(|error| interp.report_cmd_error(error.into()))?;
        retained.push(DeclaredVariable {
            original: DeclaredVariableHeader::Stored(obj::Owned::retain(value)),
            name: name.to_vec(),
            full_name: None,
        });
    }
    Ok(retained)
}

pub(super) fn validate_declared_variables(
    interp: &mut Interp,
    values: &[DeclaredVariable],
) -> Result<(), Code> {
    let policy = interp.name_policy_protocol().ok_or_else(|| {
        interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "TclOO variable declarations",
            )
            .into(),
        )
    })?;
    for entry in values {
        match tcl_syntax::naming::validate_native_oo_variable(policy.recipe(), &entry.name) {
            Ok(None) => {}
            Ok(Some(error)) => {
                let words: Vec<_> = error.error_code.iter().map(Vec::as_slice).collect();
                let code = crate::interp::error_code_list(&words);
                return Err(interp.error_with_code(&error.message, &code));
            }
            Err(_) => {
                return Err(interp.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "TclOO variable declaration validation",
                    )
                    .into(),
                ));
            }
        }
    }
    Ok(())
}

/// A built-in slot's `Get` method: returns its current contents from the active
/// definition target.
pub(super) fn slot_inst_get(interp: &mut Interp, slot: OoId, _args: &[*mut TclObj]) -> Code {
    let Some(target) = interp.active_def_target() else {
        return err(
            interp,
            b"this command may only be called from within the context of an \
              ::oo::define or ::oo::objdefine command",
        );
    };
    let slot_name = interp.oo_name(slot);
    if is_variable_slot(&slot_name) {
        let private = interp.oo.borrow().private_depth > 0;
        let values = declared_variables(interp, &target, private);
        oo_checked!(interp, set_declared_variable_list(interp, &values));
    } else {
        let list = slot_field_read(interp, &slot_name, &target);
        set_list(interp, &list);
    }
    Code::Ok
}

/// A built-in slot's `Set` method: stores the (single list) argument into the
/// active definition target.
pub(super) fn slot_inst_set(interp: &mut Interp, slot: OoId, args: &[*mut TclObj]) -> Code {
    native_context::with_original_arguments(interp, args, |interp, originals| {
        slot_inst_set_original(interp, slot, args, originals)
    })
}

pub(super) fn slot_inst_set_original(
    interp: &mut Interp,
    slot: OoId,
    args: &[*mut TclObj],
    originals: &crate::interp::native_arguments::OriginalArguments,
) -> Code {
    let Some(target) = interp.active_def_target() else {
        return err(
            interp,
            b"this command may only be called from within the context of an \
          ::oo::define or ::oo::objdefine command",
        );
    };
    let target_owner = oo_checked!(
        interp,
        native_context::OriginalOoTarget::for_definition(interp, &target)
    );
    let slot_name = interp.oo_name(slot);
    if let Some((object, writable)) = slot_property_kind(&slot_name) {
        if interp
            .native_invocation_dialect()
            .native_property_lookup_protocol()
            .is_some()
        {
            let [original] = args else {
                return wrong_args(interp, b"Set filterList");
            };
            let (id, class) = match target {
                DefTarget::Class(id) => (id, !object),
                DefTarget::Object(id) => (id, false),
            };
            let result =
                native_properties::set_property_slot(interp, id, class, writable, *original);
            oo_original_check!(interp, originals);
            oo_checked!(interp, target_owner.check(interp));
            if let Err(error) = result {
                return interp.report_cmd_error(error);
            }
            interp.set_result_bytes(b"");
            return Code::Ok;
        }
    }
    if is_variable_slot(&slot_name) {
        if args.len() != 1 {
            return wrong_args(interp, b"Set filterList");
        }
        let selected = interp
            .name_policy_protocol()
            .filter(|policy| variable_mapping::selected(*policy));
        let mapping = selected
            .map(|policy| {
                variable_mapping::Entry::capture(interp, &target, policy, originals, &target_owner)
            })
            .transpose();
        let mapping = oo_checked!(interp, mapping);
        let members = native_context::original_list_members(interp, args[0], originals);
        oo_original_check!(interp, originals);
        oo_checked!(interp, target_owner.check(interp));
        let members = match members {
            Ok(members) => members,
            Err(error) => return interp.report_cmd_error(error),
        };
        let values = if let Some(mapping) = &mapping {
            oo_checked!(
                interp,
                mapping.prepare(interp, &target, &members, originals, &target_owner)
            )
        } else {
            match prepare_declared_variables(interp, &members) {
                Ok(values) => values,
                Err(code) => return code,
            }
        };
        oo_original_check!(interp, originals);
        oo_checked!(interp, target_owner.check(interp));
        if let Err(code) = validate_declared_variables(interp, &values) {
            return code;
        }
        let values = if mapping.is_some() {
            values
        } else {
            tcl_syntax::naming::apply_native_oo_variable_slot(
                Vec::new(),
                values,
                tcl_syntax::naming::NativeOoVariableSlotOperation::Set,
                |entry| entry.name.as_slice(),
            )
        };
        let private = interp.oo.borrow().private_depth > 0;
        let reached = if let Some(mapping) = &mapping {
            let reached = variable_mapping::install(
                interp,
                &target,
                private,
                mapping,
                &values,
                originals,
                &target_owner,
            );
            let reached = reached.map_err(|error| originals.retain_refusal(error));
            drop(values);
            oo_original_check!(interp, originals);
            oo_checked!(interp, target_owner.check(interp));
            reached
        } else {
            write_declared_variables(interp, &target, private, values, originals, &target_owner)
        };
        let reached = reached.map_err(|error| originals.retain_refusal(error));
        oo_checked!(interp, reached);
    } else {
        let list = if let Some(original) = args.first() {
            let members = native_context::original_list_members(interp, *original, originals);
            oo_original_check!(interp, originals);
            oo_checked!(interp, target_owner.check(interp));
            let members = oo_checked!(interp, members);
            oo_checked!(
                interp,
                native_context::original_word_bytes(interp, &members)
            )
        } else {
            Vec::new()
        };
        oo_original_check!(interp, originals);
        oo_checked!(interp, target_owner.check(interp));
        if let Err(code) = validate_slot_class_references(interp, &slot_name, &list) {
            return code;
        }
        oo_checked!(interp, slot_field_write(interp, &slot_name, &target, list));
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

fn validate_slot_class_references(
    interp: &mut Interp,
    slot: &[u8],
    names: &[Vec<u8>],
) -> Result<(), Code> {
    let noun = match slot {
        b"::oo::define::superclass" => b"superclass".as_slice(),
        b"::oo::define::mixin" | b"::oo::objdefine::mixin" => b"mixin".as_slice(),
        _ => return Ok(()),
    };
    for name in names {
        let object = interp
            .oo_resolve_object_bytes(name)
            .map_err(|error| interp.report_cmd_error(error))?;
        let state = interp.oo.borrow();
        let exists = state.objects.contains_key(&object);
        let is_class = state.classes.contains_key(&object);
        drop(state);
        if !exists {
            let mut message = name.clone();
            message.extend_from_slice(b" does not refer to an object");
            let code = crate::interp::error_code_list(&[b"TCL", b"LOOKUP", b"OBJECT", name]);
            return Err(interp.error_with_code(&message, &code));
        }
        if !is_class {
            let mut message = b"only a class can be a ".to_vec();
            message.extend_from_slice(noun);
            let code = crate::interp::error_code_list(&[b"TCL", b"LOOKUP", b"CLASS", name]);
            return Err(interp.error_with_code(&message, &code));
        }
    }
    Ok(())
}

/// A class-reference slot's `Resolve` method (C's `Slot_ResolveClass`): resolve
/// the item as a class in the active definition context (current namespace then
/// global), returning it unchanged if it does not name a class.
fn slot_inst_resolve(interp: &mut Interp, _slot: OoId, args: &[*mut TclObj]) -> Code {
    native_context::with_original_arguments(interp, args, |interp, originals| {
        let resolved = if args.is_empty() {
            oo_checked!(interp, interp.oo_resolve_object_bytes(b""))
        } else {
            oo_original_object!(interp, originals, 0)
        };
        if interp.oo.borrow().classes.contains_key(&resolved) {
            interp.set_result(obj::new_string_bytes(&interp.oo_name(resolved)));
        } else {
            let raw = if args.is_empty() {
                Vec::new()
            } else {
                oo_original_bytes!(interp, originals, 0)
            };
            interp.set_result(obj::new_string_bytes(&raw));
        }
        Code::Ok
    })
}

/// C's `Slot_Unknown`: the method-miss handler for slots. `args[0]` is the
/// missed method name (prepended by the dispatcher). With no args, or a first
/// arg that does not start with `-`, dispatch the slot's default operation over
/// all the args; a leading `-flag` that is not a known operation chains (as C's
/// `next` would) to the standard unknown-method error.
fn slot_m_unknown(interp: &mut Interp, obj: OoId, args: &[*mut TclObj]) -> Code {
    native_context::with_original_arguments(interp, args, |interp, originals| {
        let first = if args.is_empty() {
            Vec::new()
        } else {
            oo_original_bytes!(interp, originals, 0)
        };
        if !args.is_empty() && first.first() == Some(&b'-') {
            return interp.oo_unknown_method(obj, &first);
        }
        interp.oo_invoke(obj, b"--default-operation", args, false, None)
    })
}

/// A TIP 380 slot operation (`superclass`/`mixin`/`variable`/`filter` accept
/// these instead of a bare value list).
enum SlotOp {
    Set,
    Append,
    Prepend,
    Remove,
    AppendIfNew,
    Clear,
}

// the `::oo::Slot` class (TIP 380)
//
// The public operations are *native* methods so they add no Tcl call frame:
// the overridable `Get`/`Set`/`Resolve` they invoke therefore run at the same
// `info level` the C implementation reports.

/// Invoke a slot accessor with original objects and retain its original result.
fn slot_call(
    interp: &mut Interp,
    object: OoId,
    method: &[u8],
    arguments: &[*mut TclObj],
) -> Result<obj::Owned, Code> {
    let code = interp.oo_invoke(object, method, arguments, false, None);
    if code == Code::Ok {
        Ok(obj::Owned::retain(interp.result_obj()))
    } else {
        Err(code)
    }
}

/// Build an authored reporting List and reach its checked String once.
pub(super) fn build_list(
    interp: &Interp,
    elems: &[Vec<u8>],
) -> Result<Vec<u8>, tcl_cmd_core::CmdError> {
    let originals = crate::interp::native_arguments::OriginalArguments::enter(interp, &[])?;
    let objs: Vec<*mut TclObj> = elems.iter().map(|e| obj::new_string_bytes(e)).collect();
    let list = obj::Owned::fresh(interp.new_list_object(&objs));
    let result = interp.native_object_string_bytes(list.as_ptr());
    originals.check()?;
    drop(list);
    originals.check()?;
    result.map(|bytes| bytes.to_vec()).map_err(Into::into)
}

/// Run a slot operation: `Resolve` each arg, `Get` the current list (except for
/// `-set`/`-clear`), apply the op, then `Set` the result.
fn slot_run_op(
    interp: &mut Interp,
    object: OoId,
    operation: &SlotOp,
    arguments: &[*mut TclObj],
) -> Code {
    use tcl_syntax::naming::NativeOoVariableSlotOperation as Operation;
    let selected = match operation {
        SlotOp::Clear => Operation::Clear,
        SlotOp::Set => Operation::Set,
        SlotOp::Append => Operation::Append,
        SlotOp::Prepend => Operation::Prepend,
        SlotOp::Remove => Operation::Remove,
        SlotOp::AppendIfNew => Operation::AppendIfNew,
    };
    if selected == Operation::Clear && !arguments.is_empty() {
        return wrong_args(interp, b"-clear");
    }
    if arguments.is_empty() && !matches!(selected, Operation::Clear | Operation::Set) {
        interp.set_result_bytes(b"");
        return Code::Ok;
    }
    let mut incoming = Vec::new();
    if selected != Operation::Clear {
        for &argument in arguments {
            match slot_call(interp, object, b"Resolve", &[argument]) {
                Ok(original) => incoming.push(original),
                Err(code) => return code,
            }
        }
    }
    let mut existing = Vec::new();
    if !matches!(selected, Operation::Clear | Operation::Set) {
        let original = match slot_call(interp, object, b"Get", &[]) {
            Ok(original) => original,
            Err(code) => return code,
        };
        let members = match interp.list_elements(&original.as_ptr()) {
            Ok(members) => members,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        existing.extend(members.into_iter().map(obj::Owned::retain));
    }
    let original_records = if matches!(selected, Operation::Remove | Operation::AppendIfNew) {
        let old_pointers: Vec<_> = existing.iter().map(obj::Owned::as_ptr).collect();
        let new_pointers: Vec<_> = incoming.iter().map(obj::Owned::as_ptr).collect();
        let old = match prepare_declared_variables(interp, &old_pointers) {
            Ok(old) => old,
            Err(code) => return code,
        };
        let new = match prepare_declared_variables(interp, &new_pointers) {
            Ok(new) => new,
            Err(code) => return code,
        };
        tcl_syntax::naming::apply_native_oo_slot_records(old, new, selected, |entry| {
            entry.name.as_slice()
        })
        .into_iter()
        .map(|entry| entry.original.into_stored())
        .collect::<Vec<_>>()
    } else {
        // These operations concatenate original records without accessing keys.
        tcl_syntax::naming::apply_native_oo_slot_records(existing, incoming, selected, |_| &[])
    };
    let members: Vec<_> = original_records.iter().map(obj::Owned::as_ptr).collect();
    let list = obj::Owned::fresh(interp.new_list_object(&members));
    match slot_call(interp, object, b"Set", &[list.as_ptr()]) {
        Ok(_) => {
            interp.set_result_bytes(b"");
            Code::Ok
        }
        Err(code) => code,
    }
}

fn slot_m_set(i: &mut Interp, o: OoId, a: &[*mut TclObj]) -> Code {
    slot_run_op(i, o, &SlotOp::Set, a)
}

fn slot_m_append(i: &mut Interp, o: OoId, a: &[*mut TclObj]) -> Code {
    slot_run_op(i, o, &SlotOp::Append, a)
}

fn slot_m_prepend(i: &mut Interp, o: OoId, a: &[*mut TclObj]) -> Code {
    slot_run_op(i, o, &SlotOp::Prepend, a)
}

fn slot_m_remove(i: &mut Interp, o: OoId, a: &[*mut TclObj]) -> Code {
    slot_run_op(i, o, &SlotOp::Remove, a)
}

fn slot_m_appendifnew(i: &mut Interp, o: OoId, a: &[*mut TclObj]) -> Code {
    slot_run_op(i, o, &SlotOp::AppendIfNew, a)
}

fn slot_m_clear(i: &mut Interp, o: OoId, a: &[*mut TclObj]) -> Code {
    slot_run_op(i, o, &SlotOp::Clear, a)
}
