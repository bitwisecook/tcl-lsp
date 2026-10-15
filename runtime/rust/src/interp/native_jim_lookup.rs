// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original pinned Jim name objects, active command leases and weak VarVal caches.

use super::{native_arguments::OriginalArguments, Code, Command, Interp};
use crate::{
    frame::JimVariableRead,
    namespace::NsId,
    obj::{self, Owned, TclObj},
};
use tcl_syntax::value::{ValueError, ValueOps};

enum OriginalJimCachedStore {
    Stored,
    Link(crate::frame::Link),
    Unavailable,
}

/// The genuine current namespace holder and frame, before any reached getter.
/// A weak owner observation and allocation lease preserve actual native retirement.
pub(super) struct OriginalJimLookupNamespace {
    interpreter: Interp,
    namespace: NsId,
    holder_birth: std::rc::Weak<Owned>,
    original: obj::NativeObjectLifetime,
    frame: super::native_arguments::OriginalArgumentFrame,
}
impl OriginalJimLookupNamespace {
    pub(super) fn capture(interpreter: &Interp, namespace: NsId) -> Result<Self, ValueError> {
        let holder = interpreter
            .namespaces()
            .jim_namespace_original_owner(namespace)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original Jim lookup namespace holder",
            ))?;
        obj::check_native_liveness(holder.as_ptr())?;
        Ok(Self {
            interpreter: interpreter.clone(),
            namespace,
            holder_birth: std::rc::Rc::downgrade(&holder),
            original: obj::NativeObjectLifetime::retain(holder.as_ptr()),
            frame: super::native_arguments::OriginalArgumentFrame::capture(interpreter),
        })
    }
    pub(super) fn original_ptr(&self) -> *mut TclObj {
        self.original.as_ptr()
    }
    pub(super) fn check(&self, actual: &Interp) -> Result<(), ValueError> {
        let reached = (|| {
            if !std::rc::Rc::ptr_eq(&self.interpreter.0, &actual.0) {
                return Err(ValueError::CommandProtocolUnavailable(
                    "foreign Jim lookup namespace",
                ));
            }
            actual.check_entered_native_operation().map_err(|_| {
                ValueError::CommandProtocolUnavailable("entered original Jim lookup namespace")
            })?;
            self.frame.check_frame(actual)?;
            self.original.checked_ptr()?;
            if !actual
                .namespaces()
                .jim_namespace_original_owner(self.namespace)
                .is_some_and(|holder| {
                    std::rc::Weak::ptr_eq(&std::rc::Rc::downgrade(&holder), &self.holder_birth)
                })
            {
                return Err(ValueError::CommandProtocolUnavailable(
                    "retired original Jim lookup namespace",
                ));
            }
            Ok(())
        })();
        if let Err(error) = &reached {
            if let Some(refusal) = error.native_access_refusal() {
                actual.clone().refuse_native_execution(refusal.into());
            }
        }
        reached
    }
}

pub(super) struct JimActiveCommandScope {
    interp: Interp,
    token: u64,
}
impl Drop for JimActiveCommandScope {
    fn drop(&mut self) {
        let (token, command) = {
            let mut active = self.interp.jim_active_command_workers.borrow_mut();
            active.pop().expect("owned synchronous Jim invocation")
        };
        assert_eq!(token, self.token);
        // The actual current worker retires before its previous-command edge.
        // Neither its delete hook nor previous cleanup may run under a borrow.
        drop(command);
        self.interp.namespaces.borrow_mut().leave_jim_command(token);
        if let Err(first) = self.interp.retire_pending_native_ensemble_roles() {
            self.interp.refuse_native_execution(first);
        }
    }
}
impl Interp {
    pub(super) fn retain_active_jim_command(
        &self,
        command: &Command,
        token: Option<u64>,
    ) -> Option<JimActiveCommandScope> {
        self.native_invocation_dialect()
            .native_jim_lookup_protocol()?;
        let token = token?;
        self.jim_active_command_workers
            .borrow_mut()
            .push((token, command.clone()));
        self.namespaces.borrow_mut().enter_jim_command(token);
        Some(JimActiveCommandScope {
            interp: self.clone(),
            token,
        })
    }
    pub(super) fn resolve_original_jim_command(
        &mut self,
        current: NsId,
        original: *mut TclObj,
    ) -> Result<Option<(Command, Option<u64>)>, ValueError> {
        let scope = super::native_operation_currency::NativeOperationScope::enter(self).map_err(
            |first| {
                self.refuse_native_execution(first);
                ValueError::CommandProtocolUnavailable("original Jim command entry")
            },
        )?;
        obj::check_native_liveness(original)?;
        let _allocation = obj::NativeObjectLifetime::retain(original);
        let dialect = self.native_invocation_dialect();
        let recipe =
            dialect
                .native_jim_lookup_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "Jim original command lookup",
                ))?;
        let _context = self.native_jim_object_context()?;
        let namespace_scope: OriginalJimLookupNamespace =
            OriginalJimLookupNamespace::capture(self, current)?;
        let lookup_interpreter = self.clone();
        let check_lookup = || {
            scope.currency().ensure_current_or_refuse().map_err(|_| {
                ValueError::CommandProtocolUnavailable("original Jim lookup currency")
            })?;
            obj::check_native_liveness(original)?;
            namespace_scope.check(&lookup_interpreter)
        };
        check_lookup()?;
        // Copy only observations and an allocation lease. No native namespace
        // reference or borrowed cache/world payload crosses a reached getter.
        let cached = obj::with_jim_command_cache(original, |cache| {
            if cache.interpreter != self.native_command_interpreter {
                return Err(ValueError::CommandProtocolUnavailable(
                    "foreign Jim command cache",
                ));
            }
            Ok((
                cache.epoch,
                cache.token,
                cache.namespace.as_ptr(),
                obj::NativeObjectLifetime::retain(cache.namespace.as_ptr()),
            ))
        })
        .transpose()?;
        let hit = if let Some((epoch, token, cached_namespace, _cached_allocation)) = cached {
            if epoch != self.namespaces().jim_procedure_epoch() {
                None
            } else {
                let same_namespace = if cached_namespace == namespace_scope.original_ptr() {
                    // Jim StringEqObj accepts the actual same pointer without
                    // either getter, even when the original is still stringless.
                    true
                } else {
                    let cached_bytes = self.native_object_string_bytes_using(
                        cached_namespace,
                        &check_lookup,
                        &|error| error,
                    );
                    check_lookup()?;
                    let cached_bytes = cached_bytes?;
                    let current_bytes = self.materialize_jim_namespace(current);
                    check_lookup()?;
                    cached_bytes.as_ref() == current_bytes?.as_ref()
                };
                check_lookup()?;
                if !obj::with_jim_command_cache(original, |cache| {
                    cache.interpreter == self.native_command_interpreter
                        && cache.epoch == epoch
                        && cache.token == token
                        && cache.namespace.as_ptr() == cached_namespace
                })
                .unwrap_or(false)
                {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "changed original Jim command cache",
                    ));
                }
                let world = self.namespaces.borrow();
                let command = world.jim_node_command(token).or_else(|| {
                    self.jim_active_command_workers
                        .borrow()
                        .iter()
                        .rev()
                        .find(|(actual, _)| *actual == token)
                        .map(|(_, command)| command.clone())
                });
                if recipe.command_is_current(
                    epoch,
                    world.jim_procedure_epoch(),
                    same_namespace,
                    command.is_some(),
                ) {
                    command.map(|command| (command, Some(token)))
                } else {
                    None
                }
            }
        } else {
            None
        };
        if let Some((command, Some(token))) = hit {
            return Ok(self
                .namespaces
                .borrow()
                .jim_previous_selection(token, command)
                .map(|(command, token)| (command, Some(token))));
        }
        // JimQualifyName reaches its namespace length before the name getter,
        // including an absolute incoming command name. Capture remains getter-free.
        let namespace_bytes = self.materialize_jim_namespace(current);
        check_lookup()?;
        namespace_bytes?;
        let bytes = self.native_object_string_bytes_using(original, &check_lookup, &|error| error);
        check_lookup()?;
        let bytes = bytes?;
        let selected = self.resolve_dispatchable_with_generation(current, &bytes);
        // The selection is metadata during old-primary retirement. Keeping a
        // Command clone here would postpone an idle owned delete callback.
        let token = match selected {
            Some((command, Some(token))) => {
                drop(command);
                Some(token)
            }
            other => return Ok(other),
        };
        if let Some(token) = token {
            self.namespaces
                .borrow_mut()
                .observe_original_jim_command_name(token)?;
            let epoch = self.namespaces().jim_procedure_epoch();
            let namespace = namespace_scope.original_ptr();
            let check = || {
                namespace_scope.check(self)?;
                scope.currency().ensure_current_or_refuse().map_err(|_| {
                    ValueError::CommandProtocolUnavailable("original Jim command currency")
                })?;
                obj::check_native_liveness(original)?;
                obj::check_native_liveness(namespace)?;
                let world = self.namespaces.borrow();
                if world.jim_procedure_epoch() != epoch || world.jim_node_command(token).is_none() {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "captured original Jim command binding",
                    ));
                }
                Ok(())
            };
            // Capture owns only the allocation lease and weak namespace birth.
            // Its cache-native reference is acquired after old primary retirement.
            let reached = obj::install_jim_command_cache_with_stage_check(
                original,
                || {
                    Ok(obj::JimCommandCache {
                        interpreter: self.native_command_interpreter,
                        epoch,
                        token,
                        namespace: Owned::retain(namespace),
                    })
                },
                dialect,
                &check,
            );
            check()?;
            reached?;
        }
        // Project the same selected node only after preparation has settled.
        // Native miss returned above without changing its existing primary.
        Ok(token.and_then(|token| {
            let world = self.namespaces.borrow();
            let command = world.jim_node_command(token)?;
            world
                .jim_previous_selection(token, command)
                .map(|(command, token)| (command, Some(token)))
        }))
    }

    pub(super) fn current_jim_variable_cache(
        &self,
        original: *mut TclObj,
    ) -> Result<Option<obj::JimVariableCache>, ValueError> {
        let recipe = self
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "Jim original variable lookup",
            ))?;
        obj::with_jim_variable_cache(original, |cache| {
            if cache.interpreter != self.native_command_interpreter {
                return Err(ValueError::CommandProtocolUnavailable(
                    "foreign Jim variable cache",
                ));
            }
            let frame = self.frames.borrow().native_jim_frame_id(cache.global);
            Ok(recipe
                .variable_is_current(cache.frame, frame)
                .then(|| cache.clone()))
        })
        .transpose()
        .map(Option::flatten)
    }
    pub(super) fn install_original_jim_variable(
        &self,
        original: *mut TclObj,
        bytes: &[u8],
    ) -> Result<(), ValueError> {
        let captured = {
            let frames = self.frames.borrow();
            let world = self.namespaces.borrow();
            crate::vars::original_jim_variable_cell(&frames, &world, self.current_ns.get(), bytes)
                .map(|cell| {
                    let global = tcl_syntax::naming::NativeNameProtocol::Jim084
                        .variable_root_input(bytes)
                        .qualification()
                        == tcl_syntax::naming::NativeNameQualification::Absolute;
                    (
                        obj::JimVariableCache {
                            interpreter: self.native_command_interpreter,
                            frame: frames.native_jim_frame_id(global),
                            global,
                            cell,
                        },
                        frames.original_jim_frame_incarnation(global),
                    )
                })
        };
        if let Some((cache, frame)) = captured {
            self.install_captured_original_jim_variable(original, cache, frame, None)?;
        }
        Ok(())
    }
    pub(super) fn install_original_jim_variable_at(
        &self,
        original: *mut TclObj,
        cell: crate::frame::WeakJimVariableCell,
        global: bool,
        additional: &dyn Fn() -> Result<(), ValueError>,
    ) -> Result<(), ValueError> {
        let frames = self.frames.borrow();
        let cache = obj::JimVariableCache {
            interpreter: self.native_command_interpreter,
            frame: frames.native_jim_frame_id(global),
            global,
            cell,
        };
        let frame = frames.original_jim_frame_incarnation(global);
        drop(frames);
        self.install_captured_original_jim_variable(original, cache, frame, Some(additional))
    }
    fn install_captured_original_jim_variable(
        &self,
        original: *mut TclObj,
        cache: obj::JimVariableCache,
        frame: std::rc::Weak<()>,
        additional: Option<&dyn Fn() -> Result<(), ValueError>>,
    ) -> Result<(), ValueError> {
        let cell_check = cache.cell.original_stage_check()?;
        let check = || {
            self.check_entered_native_operation().map_err(|_| {
                ValueError::CommandProtocolUnavailable("entered original Jim variable cache")
            })?;
            if let Some(additional) = additional {
                additional()?;
            }
            if !self
                .frames
                .borrow()
                .owns_original_jim_frame_incarnation(&frame)
            {
                return Err(ValueError::CommandProtocolUnavailable(
                    "captured original Jim cache frame",
                ));
            }
            cell_check()
        };
        let reached = obj::install_jim_variable_cache_with_stage_check(
            original,
            cache,
            self.native_invocation_dialect(),
            Some(&check),
        );
        let reached = reached.map_err(|error| {
            self.clone().refuse_completion_value_access(error.clone());
            error
        });
        check()?;
        reached
    }

    fn store_original_jim_cached_scalar(
        &self,
        cache: &obj::JimVariableCache,
        value: *mut TclObj,
        arguments: &OriginalArguments,
    ) -> Result<OriginalJimCachedStore, tcl_cmd_core::CmdError> {
        arguments.check_for(self)?;
        arguments.original(1)?;
        let local = crate::vars::PreparedJimAliasLocal::from_scalar_cache(
            self,
            &cache.cell,
            cache.global,
            arguments.currency(),
        )?;
        if let Some(local) = local {
            // This is the source's one genuine incoming cell reference. No
            // target-name temporary or protective native hold is acquired.
            let mut incoming = Some(Owned::retain(value));
            let reached = (|| {
                let retired = local.begin_assignment(self, arguments.currency())?;
                retired.finish(|| {
                    arguments.original(1)?;
                    local.check(self, arguments.currency())
                })?;
                local.assign(
                    self,
                    incoming
                        .take()
                        .expect("genuine incoming Jim cell reference"),
                    arguments.currency(),
                )
            })();
            let reached = reached.map_err(|error| arguments.retain_refusal(error));
            local.clear_retiring();
            drop(incoming);
            let after_cleanup = arguments
                .original(1)
                .and_then(|_| arguments.check_for(self));
            reached?;
            after_cleanup?;
            return Ok(OriginalJimCachedStore::Stored);
        }
        let reached = cache
            .cell
            .read_checked()
            .map_err(|error| arguments.retain_refusal(error.into()));
        arguments.original(1)?;
        arguments.check_for(self)?;
        Ok(match reached? {
            Some(JimVariableRead::Link(link)) => OriginalJimCachedStore::Link(link),
            _ => OriginalJimCachedStore::Unavailable,
        })
    }
    /// Native original-name read; successful conversion updates this same object.
    /// The returned value remains borrowed from its actual VarVal.
    pub(crate) fn read_original_named_variable(
        &mut self,
        original: *mut TclObj,
    ) -> Result<*mut TclObj, Code> {
        if self.native_c_variable_name_protocol().is_some() {
            return self.read_original_c_variable(original);
        }
        if self.observed_name_policy_selected()
            || self
                .native_invocation_dialect()
                .native_jim_lookup_protocol()
                .is_none()
        {
            let bytes = self
                .native_string_bytes(&original)
                .map_err(|error| self.report_cmd_error(error.into()))?;
            return self.read_named_variable(&bytes);
        }
        let cache = self
            .current_jim_variable_cache(original)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        if let Some(cache) = cache {
            let value = match cache
                .cell
                .read_checked()
                .map_err(|error| self.report_cmd_error(error.into()))?
            {
                Some(JimVariableRead::Scalar(value)) => Some(value),
                Some(JimVariableRead::Link(link)) => {
                    if let Some(result) = self.read_original_jim_link(original, &link) {
                        return result;
                    }
                    crate::vars::read_jim_cached_link(
                        &self.frames.borrow(),
                        &self.namespaces.borrow(),
                        &link,
                    )
                }
                None => None,
            };
            if let Some(value) = value {
                return Ok(value);
            }
        }
        let bytes = self
            .native_string_bytes(&original)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        // SetVariableFromAny installs the original alias even if a later
        // linked-target read misses. Missing original names preserve primary.
        if self.is_native_jim_dictionary_name(original, &bytes) {
            return self
                .read_native_jim_dictionary_sugar(original, false)?
                .ok_or(Code::Error);
        }
        self.install_original_jim_variable(original, &bytes)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        if let Some(cache) = self
            .current_jim_variable_cache(original)
            .map_err(|error| self.report_cmd_error(error.into()))?
        {
            if let Some(JimVariableRead::Link(link)) = cache
                .cell
                .read_checked()
                .map_err(|error| self.report_cmd_error(error.into()))?
            {
                if let Some(result) = self.read_original_jim_link(original, &link) {
                    return result;
                }
            }
        }
        self.read_named_variable(&bytes)
    }
    /// Assign through the original name, then publish the actual supplied value.
    pub(crate) fn store_original_named_variable(
        &mut self,
        original: *mut TclObj,
        value: *mut TclObj,
    ) -> Result<(), Code> {
        let arguments = OriginalArguments::enter(self, &[original, value])
            .map_err(|error| self.report_cmd_error(error))?;
        let result = (|| {
            if self.native_c_variable_name_protocol().is_some() {
                return self.store_original_c_variable(original, value);
            }
            self.assign_original_named_variable(original, value)?;
            arguments
                .original(1)
                .map_err(|error| self.report_cmd_error(error))?;
            self.set_result(value);
            Ok(())
        })();
        arguments
            .check()
            .map_err(|error| self.report_cmd_error(error))?;
        result
    }

    /// Assign through the original getter without publishing an interpreter result.
    pub(crate) fn assign_original_named_variable(
        &mut self,
        original: *mut TclObj,
        value: *mut TclObj,
    ) -> Result<(), Code> {
        let arguments = OriginalArguments::enter(self, &[original, value])
            .map_err(|error| self.report_cmd_error(error))?;
        let result = self.assign_original_named_variable_entered(original, value, &arguments);
        arguments
            .check()
            .map_err(|error| self.report_cmd_error(error))?;
        result
    }

    fn assign_original_named_variable_entered(
        &mut self,
        original: *mut TclObj,
        value: *mut TclObj,
        arguments: &OriginalArguments,
    ) -> Result<(), Code> {
        if self.native_c_variable_name_protocol().is_some() {
            return self.assign_original_c_variable(original, value);
        }
        if self.observed_name_policy_selected()
            || self
                .native_invocation_dialect()
                .native_jim_lookup_protocol()
                .is_none()
        {
            let bytes = arguments
                .bytes(0)
                .map_err(|error| self.report_cmd_error(error))?;
            arguments
                .original(1)
                .map_err(|error| self.report_cmd_error(error))?;
            let assigned = self.var_set_named(&bytes, value);
            arguments
                .check()
                .map_err(|error| self.report_cmd_error(error))?;
            return assigned.map_err(|error| crate::builtins::var_error(self, &bytes, error));
        }
        let associated = self.associate_native_jim_arguments(&[original, value]);
        arguments
            .original(1)
            .map_err(|error| self.report_cmd_error(error))?;
        associated.map_err(|error| self.report_cmd_error(error.into()))?;
        let cache = self
            .current_jim_variable_cache(original)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        if let Some(cache) = cache {
            let stored = self
                .store_original_jim_cached_scalar(&cache, value, arguments)
                .map_err(|error| self.report_cmd_error(error))?;
            arguments
                .original(1)
                .map_err(|error| self.report_cmd_error(error))?;
            match stored {
                OriginalJimCachedStore::Stored => {
                    return Ok(());
                }
                OriginalJimCachedStore::Link(link) => {
                    let linked = self.store_original_jim_link(&link, value);
                    arguments
                        .original(1)
                        .map_err(|error| self.report_cmd_error(error))?;
                    if let Some(result) = linked {
                        return result;
                    }
                    return self.store_captured_compatibility_jim_link(&link, value, arguments);
                }
                OriginalJimCachedStore::Unavailable => {}
            }
        }
        let bytes = arguments
            .bytes(0)
            .map_err(|error| self.report_cmd_error(error))?;
        arguments
            .original(1)
            .map_err(|error| self.report_cmd_error(error))?;
        if self.is_native_jim_dictionary_name(original, &bytes) {
            return self.assign_native_jim_dictionary_sugar(original, value);
        }
        let installed = self.install_original_jim_variable(original, &bytes);
        arguments
            .original(1)
            .map_err(|error| self.report_cmd_error(error))?;
        installed.map_err(|error| self.report_cmd_error(error.into()))?;
        if let Some(cache) = self
            .current_jim_variable_cache(original)
            .map_err(|error| self.report_cmd_error(error.into()))?
        {
            let stored = self
                .store_original_jim_cached_scalar(&cache, value, arguments)
                .map_err(|error| self.report_cmd_error(error))?;
            arguments
                .original(1)
                .map_err(|error| self.report_cmd_error(error))?;
            match stored {
                OriginalJimCachedStore::Stored => return Ok(()),
                OriginalJimCachedStore::Link(link) => {
                    let linked = self.store_original_jim_link(&link, value);
                    arguments
                        .original(1)
                        .map_err(|error| self.report_cmd_error(error))?;
                    if let Some(result) = linked {
                        return result;
                    }
                    return self.store_captured_compatibility_jim_link(&link, value, arguments);
                }
                OriginalJimCachedStore::Unavailable => {}
            }
        }
        let existed = crate::vars::original_jim_variable_cell(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            self.current_ns.get(),
            &bytes,
        )
        .is_some();
        let (base, element) = self
            .variable_name_parts(&bytes)
            .map_err(|error| crate::builtins::var_error(self, &bytes, error))?;
        let assigned = match &element {
            Some(element) => self.var_set_elem(&base, element, value),
            None => self.var_set(&base, value),
        };
        arguments
            .original(1)
            .map_err(|error| self.report_cmd_error(error))?;
        assigned.map_err(|error| crate::builtins::var_error(self, &bytes, error))?;
        if !existed && element.is_none() {
            let key = if tcl_syntax::naming::NativeNameProtocol::Jim084
                .variable_root_input(&bytes)
                .qualification()
                == tcl_syntax::naming::NativeNameQualification::Absolute
            {
                // JimCreateVariable owns a separate counted stripped global key.
                let key = tcl_syntax::naming::jim_global_variable_key_bytes(b"::", &bytes);
                Owned::fresh(obj::new_string_bytes(&key))
            } else {
                Owned::retain(original)
            };
            crate::vars::retain_original_jim_variable_key(
                &self.frames.borrow(),
                &self.namespaces.borrow(),
                self.current_ns.get(),
                &bytes,
                key.as_ptr(),
            );
        }
        let installed = self.install_original_jim_variable(original, &bytes);
        arguments
            .original(1)
            .map_err(|error| self.report_cmd_error(error))?;
        installed.map_err(|error| self.report_cmd_error(error.into()))?;
        Ok(())
    }
    fn store_captured_compatibility_jim_link(
        &mut self,
        link: &crate::frame::Link,
        value: *mut TclObj,
        arguments: &OriginalArguments,
    ) -> Result<(), Code> {
        let captured = crate::vars::capture_jim_cached_link(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            link,
        );
        let mut captured = captured.map_err(|_| {
            self.report_cmd_error(
                ValueError::CommandProtocolUnavailable(
                    "captured compatibility linked-variable root",
                )
                .into(),
            )
        })?;
        let reached = captured.store(value, arguments.currency());
        arguments
            .check()
            .map_err(|error| self.report_cmd_error(error))?;
        match reached {
            Ok(()) => Ok(()),
            Err(crate::frame::JimLinkStoreError::Command(error)) => {
                Err(self.report_cmd_error(error))
            }
            Err(crate::frame::JimLinkStoreError::Variable(error)) => {
                let name = arguments
                    .bytes(0)
                    .map_err(|error| self.report_cmd_error(error))?;
                Err(crate::builtins::var_error(self, &name, error))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::namespace::GLOBAL;

    fn jim() -> Interp {
        let mut interp = Interp::new();
        interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
        interp
    }
    fn name(bytes: &[u8]) -> Owned {
        Owned::fresh(obj::new_string_bytes(bytes))
    }
    fn worker(_interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
        Code::Ok
    }

    #[test]
    fn original_jim_alias_prefix_query_reaches_lookup_with_a_stringless_namespace() {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("jimtcl"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        interp
            .install_alias(b"a", b"return".to_vec(), vec![b"PREFIX".to_vec()])
            .unwrap();
        let command = name(b"a");
        let prefix = match interp
            .original_alias_prefix_value_at(GLOBAL, command.as_ptr())
            .unwrap()
        {
            tcl_runtime_api::AliasPrefixLookup::Prefix(prefix) => prefix,
            _ => panic!("actual alias prefix was unavailable"),
        };
        let namespace = Owned::fresh(obj::new_wide_int_obj(42));
        let context = interp.native_jim_object_context().unwrap();
        crate::native_source::bind_context(namespace.as_ptr(), &context).unwrap();
        let lookup = interp
            .namespaces_mut()
            .retain_unmaterialized_jim_namespace(Owned::retain(namespace.as_ptr()));
        let warm = interp
            .original_alias_prefix_value_at(lookup, command.as_ptr())
            .unwrap();
        assert!(
            matches!(warm, tcl_runtime_api::AliasPrefixLookup::Prefix(actual) if actual == prefix)
        );
        assert_eq!(
            obj::with_jim_command_cache(command.as_ptr(), |cache| cache.namespace.as_ptr()),
            Some(namespace.as_ptr())
        );
        obj::invalidate_string(namespace.as_ptr());
        assert!(interp.namespaces().jim_namespace_bytes(lookup).is_none());
        let references = unsafe { (*namespace.as_ptr()).ref_count };
        let hot = interp
            .original_alias_prefix_value_at(lookup, command.as_ptr())
            .unwrap();
        assert!(
            matches!(hot, tcl_runtime_api::AliasPrefixLookup::Prefix(actual) if actual == prefix)
        );
        assert!(!obj::has_string_rep(namespace.as_ptr()));
        assert_eq!(unsafe { (*namespace.as_ptr()).ref_count }, references);
        assert!(interp.namespaces().native_context_path(lookup).is_none());
        let cold = name(b"a");
        let reached = interp
            .original_alias_prefix_value_at(lookup, cold.as_ptr())
            .unwrap();
        assert!(
            matches!(reached, tcl_runtime_api::AliasPrefixLookup::Prefix(actual) if actual == prefix)
        );
        assert_eq!(
            interp.namespaces().jim_namespace_bytes(lookup),
            Some(b"42".as_slice())
        );
        let missing = name(b"NO_SUCH_ALIAS");
        assert!(matches!(
            interp
                .original_alias_prefix_value_at(lookup, missing.as_ptr())
                .unwrap(),
            tcl_runtime_api::AliasPrefixLookup::MissingCommand
        ));
        let ordinary = name(b"return");
        assert!(matches!(
            interp
                .original_alias_prefix_value_at(lookup, ordinary.as_ptr())
                .unwrap(),
            tcl_runtime_api::AliasPrefixLookup::NotAlias
        ));
        assert!(interp.native_execution_refusal().is_none());
        // A genuinely withdrawn context owner cannot borrow the global alias.
        let original = Owned::fresh(obj::new_wide_int_obj(43));
        let retired = interp
            .namespaces_mut()
            .retain_unmaterialized_jim_namespace(original);
        let owner = interp
            .namespaces_mut()
            .take_jim_namespace_owner(retired)
            .unwrap();
        drop(owner);
        assert!(interp
            .original_alias_prefix_value_at(retired, cold.as_ptr())
            .is_err());
    }

    #[test]
    fn original_jim_namespace_receipt_does_not_keep_a_withdrawn_native_owner_alive() {
        let mut interp = jim();
        let _context = interp.native_jim_object_context().unwrap();
        let original = Owned::fresh(obj::new_wide_int_obj(42));
        let pointer = original.as_ptr();
        let namespace = interp
            .namespaces_mut()
            .retain_unmaterialized_jim_namespace(original);
        let owner = interp
            .namespaces_mut()
            .take_jim_namespace_owner(namespace)
            .unwrap();
        let _entered =
            super::super::native_operation_currency::NativeOperationScope::enter(&interp).unwrap();
        let receipt = OriginalJimLookupNamespace::capture(&interp, namespace).unwrap();
        assert_eq!(unsafe { (*pointer).ref_count }, 1);
        assert!(!obj::has_string_rep(pointer));
        drop(owner);
        // The allocation lease prevents address reuse, not the real final
        // native decrement or retirement of this exact namespace object.
        assert!(obj::check_native_liveness(pointer).is_err());
        assert!(receipt.check(&interp).is_err());
    }

    #[test]
    fn deferred_jim_command_lookup_preserves_same_pointer_cache_and_reached_namespace_getters() {
        let mut interp = jim();
        interp.bind_command_replacement(GLOBAL, b"p", Command::Builtin(worker));
        let namespace = Owned::fresh(obj::new_wide_int_obj(42));
        let native_context = interp.native_jim_object_context().unwrap();
        crate::native_source::bind_context(namespace.as_ptr(), &native_context).unwrap();
        let current = interp
            .namespaces_mut()
            .retain_unmaterialized_jim_namespace(Owned::retain(namespace.as_ptr()));
        assert!(!obj::has_string_rep(namespace.as_ptr()));
        assert!(interp.namespaces().jim_namespace_bytes(current).is_none());
        assert!(interp.namespaces().native_context_path(current).is_none());
        let command = name(b"p");
        let (_, token) = interp
            .resolve_original_command_at(current, command.as_ptr())
            .unwrap()
            .unwrap();
        assert!(token.is_some());
        assert!(obj::has_string_rep(namespace.as_ptr()));
        assert_eq!(
            interp.namespaces().jim_namespace_bytes(current),
            Some(b"42".as_slice())
        );
        assert_eq!(
            obj::with_jim_command_cache(command.as_ptr(), |cache| cache.namespace.as_ptr()),
            Some(namespace.as_ptr())
        );

        // A genuine installed cache keeps the same original namespace object.
        // Drop only its resident string, leaving the actual typed integer alive.
        obj::invalidate_string(namespace.as_ptr());
        let references = unsafe { (*namespace.as_ptr()).ref_count };
        assert!(interp.namespaces().jim_namespace_bytes(current).is_none());
        let (_, hot) = interp
            .resolve_original_command_at(current, command.as_ptr())
            .unwrap()
            .unwrap();
        assert_eq!(hot, token);
        assert!(!obj::has_string_rep(namespace.as_ptr()));
        assert!(interp.namespaces().native_context_path(current).is_none());
        assert_eq!(unsafe { (*namespace.as_ptr()).ref_count }, references);

        // Equal bytes in another actual object require cached/current getters.
        let other = Owned::fresh(obj::new_wide_int_obj(42));
        crate::native_source::bind_context(other.as_ptr(), &native_context).unwrap();
        let other_context = interp
            .namespaces_mut()
            .retain_unmaterialized_jim_namespace(Owned::retain(other.as_ptr()));
        assert_ne!(other.as_ptr(), namespace.as_ptr());
        assert!(!obj::has_string_rep(other.as_ptr()));
        let (_, equal) = interp
            .resolve_original_command_at(other_context, command.as_ptr())
            .unwrap()
            .unwrap();
        assert_eq!(equal, token);
        assert!(obj::has_string_rep(namespace.as_ptr()));
        assert!(obj::has_string_rep(other.as_ptr()));
        assert_eq!(
            interp.namespaces().jim_namespace_bytes(other_context),
            Some(b"42".as_slice())
        );
        assert_eq!(
            obj::with_jim_command_cache(command.as_ptr(), |cache| cache.namespace.as_ptr()),
            Some(namespace.as_ptr())
        );

        // An absolute incoming cold head still reaches the genuine namespace.
        let rooted_namespace = Owned::fresh(obj::new_wide_int_obj(43));
        crate::native_source::bind_context(rooted_namespace.as_ptr(), &native_context).unwrap();
        let rooted_context = interp
            .namespaces_mut()
            .retain_unmaterialized_jim_namespace(Owned::retain(rooted_namespace.as_ptr()));
        let rooted = name(b"::p");
        assert!(interp
            .resolve_original_command_at(rooted_context, rooted.as_ptr())
            .unwrap()
            .is_some());
        assert!(obj::has_string_rep(rooted_namespace.as_ptr()));
        assert_eq!(
            interp.namespaces().jim_namespace_bytes(rooted_context),
            Some(b"43".as_slice())
        );
        assert!(interp.native_execution_refusal().is_none());
    }

    #[test]
    fn original_jim_lookup_primaries_match_native_key_and_cache_ownership() {
        crate::counters::reset();
        {
            let mut interp = jim();
            let value = name(b"VALUE");
            for (spelling, owned_key) in [
                (b"key".as_slice(), true),
                (b"::absolute", false),
                (b"k\0tail", true),
                (b"k\xff", true),
            ] {
                let original = name(spelling);
                let pointer = unsafe { (*original.as_ptr()).bytes };
                assert!(interp
                    .read_original_named_variable(original.as_ptr())
                    .is_err());
                assert!(obj::obj_type_ptr(original.as_ptr()).is_null());
                interp
                    .store_original_named_variable(original.as_ptr(), value.as_ptr())
                    .unwrap();
                assert_eq!(
                    unsafe { (*original.as_ptr()).ref_count },
                    if owned_key { 2 } else { 1 }
                );
                let cache = obj::with_jim_variable_cache(original.as_ptr(), |cache| {
                    (cache.frame, cache.global)
                })
                .unwrap();
                assert_eq!(cache.1, spelling.starts_with(b"::"));
                assert_eq!(
                    interp
                        .read_original_named_variable(original.as_ptr())
                        .unwrap(),
                    value.as_ptr()
                );
                let duplicate = Owned::fresh(obj::duplicate(original.as_ptr()));
                assert_eq!(unsafe { (*duplicate.as_ptr()).ref_count }, 1);
                assert_eq!(
                    obj::with_jim_variable_cache(duplicate.as_ptr(), |cache| (
                        cache.frame,
                        cache.global
                    )),
                    Some(cache)
                );
                assert_eq!(
                    interp
                        .read_original_named_variable(duplicate.as_ptr())
                        .unwrap(),
                    value.as_ptr()
                );
                assert_eq!(unsafe { (*original.as_ptr()).bytes }, pointer);
                assert!(interp.var_unset(spelling));
                assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
                assert!(interp
                    .read_original_named_variable(original.as_ptr())
                    .is_err());
                assert_eq!(
                    obj::with_jim_variable_cache(original.as_ptr(), |cache| cache.frame),
                    Some(cache.0)
                );
            }
        }
        assert_eq!(
            crate::counters::finalize(),
            0,
            "{} objects, {} buffers",
            crate::counters::live_objs(),
            crate::counters::live_bufs()
        );
        assert_eq!(crate::counters::double_free_count(), 0);
    }

    #[test]
    fn jim_original_variable_frame_miss_and_unrelated_unset_match_native_controls() {
        let mut interp = jim();
        let original = name(b"key");
        let value = name(b"VALUE");
        interp
            .store_original_named_variable(original.as_ptr(), value.as_ptr())
            .unwrap();
        let first = obj::with_jim_variable_cache(original.as_ptr(), |cache| cache.frame).unwrap();
        interp.frames.borrow_mut().push(GLOBAL);
        assert!(interp
            .read_original_named_variable(original.as_ptr())
            .is_err());
        assert_eq!(
            obj::with_jim_variable_cache(original.as_ptr(), |cache| cache.frame),
            Some(first)
        );
        interp.frames.borrow_mut().pop();
        assert_eq!(
            interp
                .read_original_named_variable(original.as_ptr())
                .unwrap(),
            value.as_ptr()
        );
        interp.var_set(b"other", value.as_ptr()).unwrap();
        assert!(interp.var_unset(b"other"));
        assert_ne!(interp.frames.borrow().native_jim_frame_id(false), first);
        interp
            .read_original_named_variable(original.as_ptr())
            .unwrap();
        assert_eq!(
            obj::with_jim_variable_cache(original.as_ptr(), |cache| cache.frame),
            Some(interp.frames.borrow().native_jim_frame_id(false))
        );
    }

    #[test]
    fn jim_command_name_observation_preserves_hot_lookup_and_native_key_ownership() {
        let mut interp = jim();
        interp.bind_command_replacement(GLOBAL, b"p", Command::Builtin(worker));
        let key = interp
            .namespaces
            .borrow()
            .original_jim_command_key(GLOBAL, b"p")
            .unwrap();
        let before = unsafe { (*key).ref_count };
        let original = name(b"p");
        let (_, generation) = interp
            .resolve_original_command(original.as_ptr())
            .unwrap()
            .unwrap();
        let generation = generation.unwrap();
        assert_eq!(unsafe { (*key).ref_count }, before);
        assert_eq!(
            interp
                .namespaces
                .borrow()
                .original_jim_command_name_observation(generation)
                .unwrap()
                .as_ptr(),
            key
        );
        interp
            .resolve_original_command(original.as_ptr())
            .unwrap()
            .unwrap();
        assert_eq!(unsafe { (*key).ref_count }, before);
        assert_eq!(
            interp
                .namespaces
                .borrow()
                .original_jim_command_name_observation(generation)
                .unwrap()
                .as_ptr(),
            key
        );
        let native_owner = Owned::retain(key);
        assert_eq!(
            interp.rename_command(b"p", b"q"),
            crate::namespace::RenameOutcome::Renamed
        );
        assert!(obj::check_native_liveness(key).is_ok());
        assert_eq!(
            interp
                .namespaces
                .borrow()
                .original_jim_command_name_observation(generation)
                .unwrap()
                .as_ptr(),
            key
        );
        let renamed = name(b"q");
        let (_, actual) = interp
            .resolve_original_command(renamed.as_ptr())
            .unwrap()
            .unwrap();
        assert_eq!(actual, Some(generation));
        let new_key = interp
            .namespaces
            .borrow()
            .original_jim_command_key(GLOBAL, b"q")
            .unwrap();
        assert_ne!(key, new_key);
        assert_eq!(
            interp
                .namespaces
                .borrow()
                .original_jim_command_name_observation(generation)
                .unwrap()
                .as_ptr(),
            new_key
        );
        assert_eq!(obj::bytes_of(native_owner.as_ptr()), b"p");
    }

    #[test]
    fn jim_command_name_observation_does_not_revive_a_retired_key() {
        let mut interp = jim();
        interp.bind_command_replacement(GLOBAL, b"p", Command::Builtin(worker));
        let original = name(b"p");
        let (_, generation) = interp
            .resolve_original_command(original.as_ptr())
            .unwrap()
            .unwrap();
        let generation = generation.unwrap();
        let observation = interp
            .namespaces
            .borrow()
            .original_jim_command_name_observation(generation)
            .unwrap()
            .clone();
        let exact = obj::NativeObjectLifetime::retain(observation.as_ptr());
        assert_eq!(
            interp.rename_command(b"p", b"q"),
            crate::namespace::RenameOutcome::Renamed
        );
        assert!(exact.checked_ptr().is_err());
        interp.bind_command_replacement(GLOBAL, b"p", Command::Builtin(worker));
        let replacement = interp
            .namespaces
            .borrow()
            .original_jim_command_key(GLOBAL, b"p")
            .unwrap();
        assert_eq!(observation.current_ptr().unwrap(), replacement);
        assert_eq!(obj::bytes_of(replacement), b"p");
        assert!(exact.checked_ptr().is_err());
        let original_name = interp
            .namespaces
            .borrow()
            .original_jim_command_name_observation(generation)
            .unwrap()
            .as_ptr();
        assert_eq!(original_name, observation.as_ptr());
    }

    #[test]
    fn jim_automatic_error_frame_reads_the_last_original_cold_lookup_name() {
        let profile = crate::environment::profile_for_dialect("jim");
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        // Original Jim observation: jim-command-name-error-frames-20261014.
        let source = b"proc P {n} {if {$n} {rename P Q; Q 0; error fail}; return ok}\nset c [catch {P 1} result options]\nlist $c $result [dict get $options -errorinfo]";
        assert_eq!(interp.eval_sourced(source, b"stdin"), Code::Ok);
        assert_eq!(
            interp.result_bytes(),
            b"1 fail {Q stdin 1 {error fail} {} stdin 2 {P 1}}"
        );
        assert!(!interp.host_refusal_pending());
    }

    #[test]
    fn jim_automatic_capture_does_not_read_an_unselected_retired_command_key() {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("jim"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let source = b"proc P {} {rename eval {}; error fail}\nset c [catch {eval {P}} result options]\nlist $c $result [dict get $options -errorinfo]";
        assert_eq!(interp.eval_sourced(source, b"stdin"), Code::Ok);
        assert_eq!(
            interp.result_bytes(),
            b"1 fail {P stdin 1 {error fail} {} stdin 2 P}"
        );
        assert!(!interp.host_refusal_pending());
    }

    #[test]
    fn jim_original_command_replacement_and_rename_preserve_miss_primary() {
        let mut interp = jim();
        interp.bind_command_replacement(GLOBAL, b"p", Command::Builtin(worker));
        let original = name(b"p");
        interp
            .resolve_original_command(original.as_ptr())
            .unwrap()
            .unwrap();
        let first =
            obj::with_jim_command_cache(original.as_ptr(), |cache| (cache.epoch, cache.token))
                .unwrap();
        let namespace =
            obj::with_jim_command_cache(original.as_ptr(), |cache| cache.namespace.as_ptr())
                .unwrap();
        let namespace_refs = unsafe { (*namespace).ref_count };
        let duplicate = Owned::fresh(obj::duplicate(original.as_ptr()));
        assert_eq!(unsafe { (*namespace).ref_count }, namespace_refs + 1);
        assert_eq!(
            obj::with_jim_command_cache(duplicate.as_ptr(), |cache| (cache.epoch, cache.token)),
            Some(first)
        );
        interp.bind_command_replacement(GLOBAL, b"p", Command::Builtin(worker));
        assert_eq!(interp.namespaces.borrow().jim_procedure_epoch(), first.0);
        interp
            .resolve_original_command(original.as_ptr())
            .unwrap()
            .unwrap();
        let replacement =
            obj::with_jim_command_cache(original.as_ptr(), |cache| (cache.epoch, cache.token))
                .unwrap();
        assert_eq!(replacement.0, first.0);
        assert_ne!(replacement.1, first.1);
        assert_eq!(
            interp.rename_command(b"p", b"q"),
            crate::namespace::RenameOutcome::Renamed
        );
        assert_ne!(
            interp.namespaces.borrow().jim_procedure_epoch(),
            replacement.0
        );
        assert!(interp
            .resolve_original_command(original.as_ptr())
            .unwrap()
            .is_none());
        assert_eq!(
            obj::with_jim_command_cache(original.as_ptr(), |cache| (cache.epoch, cache.token)),
            Some(replacement)
        );
    }

    #[test]
    fn jim_cache_namespace_equality_uses_counted_original_objects_not_c_arena_tokens() {
        let mut interp = jim();
        interp.bind_command_replacement(GLOBAL, b"p", Command::Builtin(worker));
        let original = name(b"p");
        interp
            .resolve_original_command(original.as_ptr())
            .unwrap()
            .unwrap();
        let primary = obj::internal_rep(original.as_ptr());
        let namespace = interp
            .namespaces
            .borrow_mut()
            .retain_jim_namespace(name(b""), std::rc::Rc::from(&b""[..]));
        assert_ne!(namespace, GLOBAL);
        assert!(interp
            .resolve_original_command_at(namespace, original.as_ptr())
            .unwrap()
            .is_some());
        assert_eq!(obj::internal_rep(original.as_ptr()), primary);
    }

    fn delete_active(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
        let before = obj::with_jim_command_cache(argv[0], |cache| cache.token).unwrap();
        assert!(interp.namespaces.borrow_mut().delete(GLOBAL, b"active"));
        assert_eq!(
            interp.resolve_original_command(argv[0]).unwrap().unwrap().1,
            Some(before)
        );
        let fresh = name(b"active");
        assert!(interp
            .resolve_original_command(fresh.as_ptr())
            .unwrap()
            .is_none());
        Code::Ok
    }
    #[test]
    fn jim_deleted_active_original_head_hits_only_until_the_real_invocation_retires() {
        let mut interp = jim();
        interp.bind_command_replacement(GLOBAL, b"active", Command::Builtin(delete_active));
        let original = name(b"active");
        let (command, generation) = interp
            .resolve_original_command(original.as_ptr())
            .unwrap()
            .unwrap();
        let epoch = interp.namespaces.borrow().jim_procedure_epoch();
        assert_eq!(
            interp.invoke_bound(command, generation, &[original.as_ptr()]),
            Code::Ok
        );
        assert_eq!(interp.namespaces.borrow().jim_procedure_epoch(), epoch);
        assert!(interp
            .resolve_original_command(original.as_ptr())
            .unwrap()
            .is_none());
        assert_eq!(
            obj::with_jim_command_cache(original.as_ptr(), |cache| cache.epoch),
            Some(epoch)
        );
        assert!(interp.jim_active_command_workers.borrow().is_empty());
    }

    #[test]
    fn foreign_jim_lookup_receipts_refuse_without_mutating_the_original() {
        let mut first = jim();
        let mut second = jim();
        let value = name(b"VALUE");
        let variable = name(b"key");
        first
            .store_original_named_variable(variable.as_ptr(), value.as_ptr())
            .unwrap();
        let primary = obj::internal_rep(variable.as_ptr());
        assert!(second
            .read_original_named_variable(variable.as_ptr())
            .is_err());
        assert_eq!(obj::internal_rep(variable.as_ptr()), primary);
        assert!(second.host_refusal_pending());
        let command = name(b"set");
        first
            .resolve_original_command(command.as_ptr())
            .unwrap()
            .unwrap();
        let primary = obj::internal_rep(command.as_ptr());
        assert!(second.resolve_original_command(command.as_ptr()).is_err());
        assert_eq!(obj::internal_rep(command.as_ptr()), primary);
    }
}

#[cfg(test)]
#[path = "native_jim_lookup/active_retirement_tests.rs"]
mod active_retirement_tests;
