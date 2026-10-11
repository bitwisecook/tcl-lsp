// SPDX-License-Identifier: AGPL-3.0-or-later
//! Jim's local detachment and incoming/old/assignment stages under one receipt.
use super::*;
use crate::interp::native_arguments::OriginalArguments;
use crate::obj::Owned;
use crate::vars::{CapturedJimAliasTarget, PreparedJimAliasLocal};
use std::rc::Weak;
use tcl_cmd_core::CmdError;
use tcl_syntax::{native_jim_lookup::NativeJimAliasFailure, value::ValueError};

struct JimAliasFrame {
    level: usize,
    birth: Weak<()>,
}
impl JimAliasFrame {
    fn capture(interpreter: &Interp, level: usize) -> Result<Self, CmdError> {
        let birth = interpreter
            .frames
            .borrow()
            .original_activation_owner_at(level)
            .ok_or_else(|| {
                CmdError::from_execution_refusal(
                    interpreter.clone().refuse_completion_value_access(
                        ValueError::CommandProtocolUnavailable("original Jim alias activation"),
                    ),
                )
            })?;
        Ok(Self { level, birth })
    }
    fn check(&self, interpreter: &Interp, arguments: &OriginalArguments) -> Result<(), CmdError> {
        arguments.check_for(interpreter)?;
        if !interpreter
            .frames
            .borrow()
            .original_activation_owner_at(self.level)
            .is_some_and(|current| {
                self.birth.strong_count() != 0 && Weak::ptr_eq(&current, &self.birth)
            })
        {
            return Err(arguments.retain_refusal(
                ValueError::CommandProtocolUnavailable("original Jim alias activation").into(),
            ));
        }
        Ok(())
    }
}
impl Interp {
    pub(crate) fn link_original_jim_upvar_objects(
        &mut self,
        target: *mut TclObj,
        level: usize,
        local: *mut TclObj,
    ) -> Code {
        let arguments = match OriginalArguments::enter(self, &[local, target]) {
            Ok(arguments) => arguments,
            Err(error) => return self.report_cmd_error(error),
        };
        let reached = self.link_original_jim_alias_entered(&arguments, level);
        let checked = arguments.check_for(self);
        match checked.and(reached) {
            Ok(()) => Code::Ok,
            Err(error) => self.report_cmd_error(error),
        }
    }
    fn link_original_jim_alias_entered(
        &mut self,
        arguments: &OriginalArguments,
        requested_level: usize,
    ) -> Result<(), CmdError> {
        let protocol = self
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .ok_or_else(|| {
                arguments.retain_refusal(
                    ValueError::CommandProtocolUnavailable("actual Jim original alias worker")
                        .into(),
                )
            })?;
        self.name_policy_protocol().ok_or_else(|| {
            arguments.retain_refusal(
                ValueError::CommandProtocolUnavailable("actual Jim original alias names").into(),
            )
        })?;
        let active = JimAliasFrame::capture(self, self.current_level())?;
        let requested = JimAliasFrame::capture(self, requested_level)?;
        let global = JimAliasFrame::capture(self, 0)?;
        let local_header = arguments.original(0)?;
        let local = match self
            .current_jim_variable_cache(local_header)
            .map_err(|error| arguments.retain_refusal(error.into()))?
        {
            Some(cache) => PreparedJimAliasLocal::from_cache(
                self,
                &cache.cell,
                cache.global,
                arguments.currency(),
            )?,
            None => {
                // SetVariableFromAny's actual initial String and lookup stage.
                let bytes = arguments.bytes(0)?;
                active.check(self, arguments)?;
                if self.is_native_jim_dictionary_name(local_header, &bytes) {
                    return Err(tcl_cmd_core::var::native_jim_alias_error(
                        protocol,
                        NativeJimAliasFailure::LocalElement,
                        &bytes,
                    ));
                }
                self.install_original_jim_variable(local_header, &bytes)
                    .map_err(|error| arguments.retain_refusal(error.into()))?;
                active.check(self, arguments)?;
                match self
                    .current_jim_variable_cache(local_header)
                    .map_err(|error| arguments.retain_refusal(error.into()))?
                {
                    Some(cache) => PreparedJimAliasLocal::from_cache(
                        self,
                        &cache.cell,
                        cache.global,
                        arguments.currency(),
                    )?,
                    None => PreparedJimAliasLocal::from_bytes(self, &bytes, arguments.currency())?,
                }
            }
        };
        let Some(local) = local else {
            let bytes = arguments.bytes(0)?;
            active.check(self, arguments)?;
            return Err(tcl_cmd_core::var::native_jim_alias_error(
                protocol,
                NativeJimAliasFailure::Exists,
                &bytes,
            ));
        };
        active.check(self, arguments)?;
        local.detach(self, arguments.currency())?;
        // The link marker has already cleared; its stored pointer remains in
        // this same scalar cell through the later required local getter.
        let local_bytes = arguments.bytes(0)?;
        local.check(self, arguments.currency())?;
        active.check(self, arguments)?;
        let local_level = if local.global() { 0 } else { active.level };
        let local_birth = if local.global() {
            global.birth.clone()
        } else {
            active.birth.clone()
        };
        let target_header = arguments.original(1)?;
        let target_bytes = arguments.bytes(1)?;
        local.check(self, arguments.currency())?;
        active.check(self, arguments)?;
        let (target_frame, target) = match protocol.link_target_input(&target_bytes) {
            NativeJimLinkTargetInput::Original => (requested, Owned::retain(target_header)),
            NativeJimLinkTargetInput::StrippedGlobal(tail) => {
                (global, Owned::fresh(obj::new_string_bytes(tail)))
            }
        };
        let reached = (|| {
            target_frame.check(self, arguments)?;
            local.check(self, arguments.currency())?;
            if local_level < target_frame.level {
                return Err(tcl_cmd_core::var::native_jim_alias_error(
                    protocol,
                    NativeJimAliasFailure::Inverted,
                    &local_bytes,
                ));
            }
            if Weak::ptr_eq(&local_birth, &target_frame.birth) {
                let cycle_local = match protocol.link_target_input(&local_bytes) {
                    NativeJimLinkTargetInput::Original => local_bytes.as_ref(),
                    NativeJimLinkTargetInput::StrippedGlobal(tail) => tail,
                };
                self.check_original_jim_alias_cycle(
                    arguments,
                    &local,
                    &target_frame,
                    target.as_ptr(),
                    cycle_local,
                )?;
            }
            let mut destination = CapturedJimAliasTarget::capture(
                self,
                &target_bytes,
                target_frame.level,
                arguments.currency(),
            )?;
            let missing = local.was_missing();
            if missing {
                // Jim_SetVariable's second unresolved-name lookup occurs
                // before its genuine incoming cell hold is acquired.
                let _ = arguments.bytes(0)?;
                local.check(self, arguments.currency())?;
            }
            // Jim_SetVariable acquires the genuine incoming cell +1 first.
            // The target temporary above remains a separate source role.
            let incoming = Owned::retain(target.as_ptr());
            let binding = (|| {
                if missing {
                    // JimCreateVariable reads the original name after the
                    // incoming VarVal reference, before table publication.
                    let _ = arguments.bytes(0)?;
                    local.check(self, arguments.currency())?;
                }
                let retired = local.begin_assignment(self, arguments.currency())?;
                retired.finish(|| {
                    target_frame.check(self, arguments)?;
                    active.check(self, arguments)?;
                    local.check(self, arguments.currency())?;
                    destination.check(self, arguments.currency())
                })?;
                target_frame.check(self, arguments)?;
                active.check(self, arguments)?;
                local.assign(self, incoming, arguments.currency())?;
                let frame = self
                    .frames
                    .borrow()
                    .jim_link_storage(target_frame.level)
                    .ok_or_else(|| {
                        arguments.retain_refusal(
                            ValueError::CommandProtocolUnavailable(
                                "original Jim link frame storage",
                            )
                            .into(),
                        )
                    })?;
                destination.check(self, arguments.currency())?;
                if missing {
                    if local.global() {
                        let offset = local_bytes
                            .iter()
                            .position(|byte| *byte != b':')
                            .unwrap_or(local_bytes.len());
                        // Jim's stripped fresh key starts at rc0; the hash
                        // publication acquires its one genuine key reference.
                        let key = obj::new_string_bytes(&local_bytes[offset..]);
                        let allocation = obj::NativeObjectLifetime::retain(key);
                        let reached = local.retain_key(self, key, arguments.currency());
                        if reached.is_err()
                            && obj::check_native_liveness(key).is_ok()
                            && unsafe { (*key).ref_count == 0 }
                        {
                            unsafe { obj::free_obj(key) };
                        }
                        drop(allocation);
                        arguments.check_for(self)?;
                        reached?;
                    } else {
                        // The original name gains only the real hash role.
                        local.retain_key(self, local_header, arguments.currency())?;
                    }
                }
                let check = || {
                    local.check(self, arguments.currency()).map_err(|_| {
                        ValueError::CommandProtocolUnavailable(
                            "original Jim link cache publication",
                        )
                    })
                };
                self.install_original_jim_variable_at(
                    local_header,
                    local.cache_cell(),
                    local.global(),
                    &check,
                )
                .map_err(|error| arguments.retain_refusal(error.into()))?;
                local.check(self, arguments.currency())?;
                target_frame.check(self, arguments)?;
                destination.check(self, arguments.currency())?;
                local.mark_link(self, destination.take_link(), frame, arguments.currency())
            })();
            // On refusal the non-owning visible old pointer is cleared only
            // when it still belongs to this retirement ticket. A callback's
            // replacement remains untouched. The actual old owner has retired.
            local.clear_retiring();
            arguments.check_for(self)?;
            binding
        })();
        if let Err(error) = &reached {
            arguments.retain_refusal(error.clone());
        }
        drop(target);
        arguments.check_for(self)?;
        reached
    }
    fn check_original_jim_alias_cycle(
        &mut self,
        arguments: &OriginalArguments,
        local: &PreparedJimAliasLocal,
        target_frame: &JimAliasFrame,
        mut original: *mut TclObj,
        local_bytes: &[u8],
    ) -> Result<(), CmdError> {
        let protocol = self
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .expect("selected Jim alias worker");
        let mut previous: Option<Rc<OriginalJimLinkTarget>> = None;
        let mut selected: Option<crate::dict::NativeDictionaryStageCheck> = None;
        let mut visited = std::collections::BTreeSet::new();
        loop {
            target_frame.check(self, arguments)?;
            local.check(self, arguments.currency())?;
            if !visited.insert(original as usize) {
                return Err(arguments.retain_refusal(
                    ValueError::CommandProtocolUnavailable("cyclic retained Jim alias chain")
                        .into(),
                ));
            }
            if let Some(selected) = &selected {
                selected().map_err(|error| arguments.retain_refusal(error.into()))?;
            }
            if let Some(previous) = &previous {
                previous
                    .original_name()
                    .map_err(|error| arguments.retain_refusal(error.into()))?;
            }
            obj::check_native_liveness(original)
                .map_err(|error| arguments.retain_refusal(error.into()))?;
            // Jim_Length and Jim_String reach the original String producer
            // only when its representation is absent. Their pointer/extent
            // reads otherwise use this same already resident representation.
            if !obj::has_string_rep(original) {
                let reached = self
                    .native_object_string_bytes(original)
                    .map_err(|error| arguments.retain_refusal(error.into()));
                arguments.check_for(self)?;
                local.check(self, arguments.currency())?;
                target_frame.check(self, arguments)?;
                if let Some(selected) = &selected {
                    selected().map_err(|error| arguments.retain_refusal(error.into()))?;
                }
                reached?;
            }
            let length = unsafe { (*original).length };
            if usize::try_from(length).ok() == Some(local_bytes.len()) {
                // SAFETY: the original live resident counted String was
                // established above under the selected genuine Jim producer.
                let bytes = unsafe {
                    std::slice::from_raw_parts((*original).bytes.cast::<u8>(), local_bytes.len())
                };
                if bytes == local_bytes {
                    return Err(tcl_cmd_core::var::native_jim_alias_error(
                        protocol,
                        NativeJimAliasFailure::SelfLink,
                        local_bytes,
                    ));
                }
            }
            // SetVariableFromAny selects the same frame already active here.
            // A cache miss reaches its original getter and shared cache worker.
            let mut cache = self
                .current_jim_variable_cache(original)
                .map_err(|error| arguments.retain_refusal(error.into()))?;
            if cache.is_none() {
                let reached = self
                    .native_object_string_bytes(original)
                    .map_err(|error| arguments.retain_refusal(error.into()));
                arguments.check_for(self)?;
                local.check(self, arguments.currency())?;
                target_frame.check(self, arguments)?;
                if let Some(selected) = &selected {
                    selected().map_err(|error| arguments.retain_refusal(error.into()))?;
                }
                let bytes = reached?;
                if self.is_native_jim_dictionary_name(original, &bytes) {
                    return Ok(());
                }
                self.install_original_jim_variable(original, &bytes)
                    .map_err(|error| arguments.retain_refusal(error.into()))?;
                arguments.check_for(self)?;
                local.check(self, arguments.currency())?;
                target_frame.check(self, arguments)?;
                if let Some(selected) = &selected {
                    selected().map_err(|error| arguments.retain_refusal(error.into()))?;
                }
                cache = self
                    .current_jim_variable_cache(original)
                    .map_err(|error| arguments.retain_refusal(error.into()))?;
            }
            let Some(cache) = cache else { return Ok(()) };
            let Some(crate::frame::JimVariableRead::Link(link)) = cache
                .cell
                .read_checked()
                .map_err(|error| arguments.retain_refusal(error.into()))?
            else {
                return Ok(());
            };
            let Some(target) = link.original_jim_target else {
                return Err(arguments.retain_refusal(
                    ValueError::CommandProtocolUnavailable("original Jim cycle link role").into(),
                ));
            };
            let Some(level) = self.frames.borrow().jim_link_level(&target.frame) else {
                return Err(arguments.retain_refusal(
                    ValueError::CommandProtocolUnavailable("original Jim cycle link frame").into(),
                ));
            };
            if level != target_frame.level {
                return Ok(());
            }
            let stage = cache
                .cell
                .original_stage_check()
                .map_err(|error| arguments.retain_refusal(error.into()))?;
            original = target
                .original_name()
                .map_err(|error| arguments.retain_refusal(error.into()))?;
            previous = Some(target);
            selected = Some(stage);
        }
    }
}
