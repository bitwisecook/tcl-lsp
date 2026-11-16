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

//! `info object call` and `info class call`: selecting the call-chain records and rendering them.

use super::super::{native_context, native_method_cache, wrong_args, CallStep};
use super::object_info::original_info_target;
use crate::interp::{Code, Interp};
use crate::obj;
use crate::obj::TclObj;
use std::rc::Rc;
use tcl_core_types::OoId;
use tcl_registry::native_tcloo_method_definition::{
    NativeTclooCallReportKind, NativeTclooMethodPayloadType, NativeTclooMethodReportRole,
};

/// `info object|class call fqn methodName` — the method-resolution chain for
/// `methodName`, each step a `{callType methodName declarer methodType}` list.
pub(super) fn info_call(interp: &mut Interp, id: OoId, argv: &[*mut TclObj], class: bool) -> Code {
    native_context::with_original_arguments(interp, argv, |interp, originals| {
        if argv.len() != 5 {
            let u: &[u8] = if class {
                b"info class call className methodName"
            } else {
                b"info object call objName methodName"
            };
            return wrong_args(interp, u);
        }
        let target_owner = match original_info_target(interp, originals, id, class) {
            Ok(target) => target,
            Err(code) => return code,
        };
        let method = oo_original_bytes!(interp, originals, 4);
        oo_checked!(interp, target_owner.check(interp));
        let report = oo_checked!(interp, build_call_chain(interp, id, &method, class));
        interp.set_result(report.as_ptr());
        drop(report);
        oo_original_check!(interp, originals);
        Code::Ok
    })
}

/// Select all exact report records before constructing the original List.
fn build_call_chain(
    interp: &Interp,
    id: OoId,
    method: &[u8],
    class: bool,
) -> Result<obj::Owned, tcl_cmd_core::CmdError> {
    let providers: Vec<(OoId, bool)> = if class {
        interp
            .class_precedence(id)
            .into_iter()
            .map(|provider| (provider, false))
            .collect()
    } else {
        interp.method_chain_faceted(id)
    };
    let mut filters = Vec::new();
    if !class {
        if let Some(object) = interp.oo.borrow().objects.get(&id) {
            filters.extend(object.filters.iter().cloned());
        }
    }
    for (provider, _) in &providers {
        if let Some(owner) = interp.oo.borrow().classes.get(provider) {
            for name in &owner.filters {
                if !filters.contains(name) {
                    filters.push(name.clone());
                }
            }
        }
    }
    let mut steps = Vec::new();
    for name in filters {
        for &(provider, is_object) in &providers {
            if interp.oo_has_method(provider, &name, is_object) {
                steps.push(CallStep {
                    method_owner: None,
                    provider,
                    method: name.clone(),
                    is_object,
                    is_filter: true,
                    synthetic_type: None,
                    declaring_facet: None,
                });
            }
        }
    }
    let exported = providers.iter().any(|&(provider, is_object)| {
        let state = interp.oo.borrow();
        if is_object {
            state
                .objects
                .get(&provider)
                .is_some_and(|owner| owner.exported.contains(method))
        } else {
            state
                .classes
                .get(&provider)
                .is_some_and(|owner| owner.exported.contains(method))
        }
    });
    let mut target_steps = Vec::new();
    for &(provider, is_object) in &providers {
        if interp.oo_has_method(provider, method, is_object) {
            if interp.method_is_private(provider, method, is_object)
                || (!exported && interp.method_unexported(provider, method, is_object))
            {
                continue;
            }
            target_steps.push(CallStep {
                method_owner: None,
                provider,
                method: method.to_vec(),
                is_object,
                is_filter: false,
                synthetic_type: None,
                declaring_facet: None,
            });
        } else if !is_object {
            if let Some(descriptor) = original_core_report_type(interp, provider, method) {
                let NativeTclooMethodPayloadType::Core(core) = descriptor else {
                    unreachable!("stock core fallback descriptor");
                };
                if !exported
                    && (!core.public() || interp.method_unexported(provider, method, false))
                {
                    continue;
                }
                target_steps.push(CallStep {
                    method_owner: None,
                    provider,
                    method: method.to_vec(),
                    is_object: false,
                    is_filter: false,
                    synthetic_type: Some(descriptor),
                    declaring_facet: None,
                });
            }
        }
    }
    let unknown = target_steps.is_empty();
    if unknown {
        for &(provider, is_object) in &providers {
            if interp.oo_has_method(provider, b"unknown", is_object)
                && !interp.method_is_private(provider, b"unknown", is_object)
            {
                target_steps.push(CallStep {
                    method_owner: None,
                    provider,
                    method: b"unknown".to_vec(),
                    is_object,
                    is_filter: false,
                    synthetic_type: None,
                    declaring_facet: None,
                });
            }
        }
    }
    steps.extend(target_steps);
    let chain = native_method_cache::ReachedMethodChain::from(native_method_cache::retain_owners(
        &interp.oo.borrow(),
        steps,
        NativeTclooMethodReportRole::Named,
    ))
    .with_unknown(unknown);
    render_call_chain(interp, &chain)
}

/// Actual stock fallback producer; an overriding declaration is selected first.
fn original_core_report_type(
    interp: &Interp,
    provider: OoId,
    method: &[u8],
) -> Option<NativeTclooMethodPayloadType> {
    let state = interp.oo.borrow();
    if Some(provider) == state.object_root {
        NativeTclooMethodPayloadType::object_core_fallback(method)
    } else if Some(provider) == state.class_root {
        NativeTclooMethodPayloadType::class_core_fallback(method)
    } else {
        None
    }
}

pub(in super::super) fn original_core_report_step(
    interp: &Interp,
    provider: OoId,
    method: &[u8],
) -> Option<CallStep> {
    let descriptor = original_core_report_type(interp, provider, method)?;
    let declaring_facet = interp
        .oo
        .borrow()
        .classes
        .get(&provider)
        .map(|class| Rc::downgrade(&class.facet_birth));
    Some(CallStep {
        method_owner: None,
        provider,
        method: method.to_vec(),
        is_object: false,
        is_filter: false,
        synthetic_type: Some(descriptor),
        declaring_facet,
    })
}

pub(in super::super) fn retain_original_terminal_report(
    interp: &Interp,
    providers: &[(OoId, bool)],
    method: &[u8],
    chain: native_method_cache::ReachedMethodChain,
) -> native_method_cache::ReachedMethodChain {
    let terminal = providers.iter().find_map(|&(provider, is_object)| {
        if is_object
            || chain
                .iter()
                .any(|step| step.provider == provider && !step.is_object && !step.is_filter)
        {
            None
        } else {
            original_core_report_step(interp, provider, method)
        }
    });
    if let Some(terminal) = terminal {
        chain.with_terminal_report(terminal)
    } else {
        chain
    }
}

fn call_report_refusal(
    interp: &Interp,
    first: impl Into<tcl_cmd_core::CmdError>,
) -> tcl_cmd_core::CmdError {
    let first = first.into();
    let Some(execution) = first.native_execution_refusal().cloned() else {
        return first;
    };
    let mut original = interp.clone();
    original.refuse_native_execution(execution.clone());
    tcl_cmd_core::CmdError::from_execution_refusal(
        original.native_execution_refusal().unwrap_or(execution),
    )
}

/// Report retained records and current payloads without reaching name String getters.
pub(in super::super) fn render_call_chain(
    interp: &Interp,
    chain: &native_method_cache::ReachedMethodChain,
) -> Result<obj::Owned, tcl_cmd_core::CmdError> {
    let originals = crate::interp::native_arguments::OriginalArguments::enter(interp, &[])?;
    let mut entries = Vec::new();
    for step in chain.iter().chain(chain.terminal_report()) {
        originals.check()?;
        let (descriptor, private) = if let Some(record) = &step.method_owner {
            (record.borrow().report_type(), record.is_private())
        } else if let Some(descriptor) = step.synthetic_type {
            (descriptor, false)
        } else {
            return Err(call_report_refusal(
                interp,
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original call-chain method record",
                ),
            ));
        };
        let call_type =
            NativeTclooCallReportKind::from_flags(step.is_filter, chain.is_unknown(), private);
        let mut fresh = Vec::new();
        let call_type = if call_type == NativeTclooCallReportKind::Unknown {
            interp.oo.borrow().method_reports.as_ref().map(|reports| reports.unknown.as_ptr())
                .ok_or_else(|| call_report_refusal(interp, tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("original Foundation unknown report header")))?
        } else {
            fresh.push(obj::Owned::fresh(obj::new_string_bytes(call_type.name())));
            fresh.last().unwrap().as_ptr()
        };
        let name = if chain.report_role() != NativeTclooMethodReportRole::Named {
            interp.oo.borrow().method_reports.as_ref().and_then(|reports| reports.report(chain.report_role()))
                .ok_or_else(|| call_report_refusal(interp, tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("original Foundation method report header")))?
        } else if let Some(record) = &step.method_owner {
            record.original_name().ok_or_else(|| {
                call_report_refusal(
                    interp,
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "original named method report header",
                    ),
                )
            })?
        } else {
            fresh.push(obj::Owned::fresh(obj::new_string_bytes(&step.method)));
            fresh.last().unwrap().as_ptr()
        };
        let declarer = if step.is_object {
            fresh.push(obj::Owned::fresh(obj::new_string_bytes(b"object")));
            fresh.last().unwrap().as_ptr()
        } else {
            let same = {
                let state = interp.oo.borrow();
                step.declaring_facet.as_ref().is_some_and(|original| {
                    state.classes.get(&step.provider).is_some_and(|class| {
                        std::rc::Weak::ptr_eq(original, &Rc::downgrade(&class.facet_birth))
                            && original.strong_count() != 0
                    })
                }) && state.objects.contains_key(&step.provider)
            };
            if !same {
                return Err(call_report_refusal(
                    interp,
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "original method report declaring facet",
                    ),
                ));
            }
            interp.oo_original_name(step.provider)
        };
        obj::check_native_liveness(name).map_err(|first| call_report_refusal(interp, first))?;
        obj::check_native_liveness(call_type)
            .map_err(|first| call_report_refusal(interp, first))?;
        obj::check_native_liveness(declarer).map_err(|first| call_report_refusal(interp, first))?;
        fresh.push(obj::Owned::fresh(obj::new_string_bytes(&descriptor.name())));
        let entry = obj::Owned::fresh(interp.new_list_object(&[
            call_type,
            name,
            declarer,
            fresh.last().unwrap().as_ptr(),
        ]));
        drop(fresh);
        originals.check()?;
        entries.push(entry);
    }
    let pointers: Vec<_> = entries.iter().map(obj::Owned::as_ptr).collect();
    let report = obj::Owned::fresh(interp.new_list_object(&pointers));
    drop(entries);
    originals.check()?;
    Ok(report)
}
