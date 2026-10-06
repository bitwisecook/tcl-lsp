// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original pinned Jim name objects, active command leases and weak VarVal caches.

use super::{Code, Command, Interp};
use crate::{
    frame::JimVariableRead,
    namespace::NsId,
    obj::{self, Owned, TclObj},
};
use tcl_syntax::value::{ValueError, ValueOps};

pub(super) struct JimActiveCommandScope {
    interp: Interp,
    token: u64,
}
impl Drop for JimActiveCommandScope {
    fn drop(&mut self) {
        let mut active = self.interp.jim_active_command_workers.borrow_mut();
        let (token, _) = active.pop().expect("owned synchronous Jim invocation");
        assert_eq!(token, self.token);
        drop(active);
        self.interp.namespaces.borrow_mut().leave_jim_command(token);
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
        let dialect = self.native_invocation_dialect();
        let recipe =
            dialect
                .native_jim_lookup_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "Jim original command lookup",
                ))?;
        let _context = self.native_jim_object_context()?;
        let hit = obj::with_jim_command_cache(original, |cache| {
            if cache.interpreter != self.native_command_interpreter {
                return Err(ValueError::CommandProtocolUnavailable(
                    "foreign Jim command cache",
                ));
            }
            let world = self.namespaces.borrow();
            if cache.epoch != world.jim_procedure_epoch() {
                return Ok(None);
            }
            let same_namespace = world
                .jim_namespace_bytes(current)
                .is_some_and(|bytes| obj::bytes_of(cache.namespace.as_ptr()) == bytes);
            if !same_namespace {
                return Ok(None);
            }
            let command = world.jim_node_command(cache.token).or_else(|| {
                self.jim_active_command_workers
                    .borrow()
                    .iter()
                    .rev()
                    .find(|(token, _)| *token == cache.token)
                    .map(|(_, command)| command.clone())
            });
            if recipe.command_is_current(
                cache.epoch,
                world.jim_procedure_epoch(),
                same_namespace,
                command.is_some(),
            ) {
                Ok(command.map(|command| (command, Some(cache.token))))
            } else {
                Ok(None)
            }
        })
        .transpose()?
        .flatten();
        if let Some((command, Some(token))) = hit {
            return Ok(self
                .namespaces
                .borrow()
                .jim_previous_selection(token, command)
                .map(|(command, token)| (command, Some(token))));
        }
        let bytes = self.native_string_bytes(&original)?;
        let selected = self.resolve_dispatchable_with_generation(current, &bytes);
        if let Some((_, Some(token))) = &selected {
            let world = self.namespaces.borrow();
            let namespace = world.jim_namespace_object(current).ok_or(
                ValueError::CommandProtocolUnavailable("Jim current namespace original holder"),
            )?;
            let epoch = world.jim_procedure_epoch();
            drop(world);
            obj::install_jim_command_cache(
                original,
                obj::JimCommandCache {
                    interpreter: self.native_command_interpreter,
                    epoch,
                    token: *token,
                    namespace,
                },
                dialect,
            )?;
        }
        // Native miss preserves the old command/Source/other primary unchanged.
        Ok(selected.and_then(|(command, token)| match token {
            Some(token) => self
                .namespaces
                .borrow()
                .jim_previous_selection(token, command)
                .map(|(command, token)| (command, Some(token))),
            None => Some((command, None)),
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
        let frames = self.frames.borrow();
        let world = self.namespaces.borrow();
        if let Some(cell) =
            crate::vars::original_jim_variable_cell(&frames, &world, self.current_ns.get(), bytes)
        {
            let global = bytes.starts_with(b"::");
            obj::install_jim_variable_cache(
                original,
                obj::JimVariableCache {
                    interpreter: self.native_command_interpreter,
                    frame: frames.native_jim_frame_id(global),
                    global,
                    cell,
                },
                self.native_invocation_dialect(),
            )?;
        }
        Ok(())
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
        if self
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
            let value = match cache.cell.read() {
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
            if let Some(JimVariableRead::Link(link)) = cache.cell.read() {
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
        if self.native_c_variable_name_protocol().is_some() {
            return self.store_original_c_variable(original, value);
        }
        self.assign_original_named_variable(original, value)?;
        self.set_result(value);
        Ok(())
    }

    /// Assign through the original getter without publishing an interpreter result.
    pub(crate) fn assign_original_named_variable(
        &mut self,
        original: *mut TclObj,
        value: *mut TclObj,
    ) -> Result<(), Code> {
        if self.native_c_variable_name_protocol().is_some() {
            return self.assign_original_c_variable(original, value);
        }
        if self
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .is_none()
        {
            let bytes = self
                .native_string_bytes(&original)
                .map_err(|error| self.report_cmd_error(error.into()))?;
            let (base, element) = self
                .variable_name_parts(&bytes)
                .map_err(|error| crate::builtins::var_error(self, &bytes, error))?;
            let assigned = match element {
                Some(element) => self.var_set_elem(&base, &element, value),
                None => self.var_set(&base, value),
            };
            return assigned.map_err(|error| crate::builtins::var_error(self, &bytes, error));
        }
        self.associate_native_jim_arguments(&[original, value])
            .map_err(|error| self.report_cmd_error(error.into()))?;
        let cache = self
            .current_jim_variable_cache(original)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        if let Some(cache) = cache {
            match cache.cell.store(value) {
                Ok(None) => {
                    return Ok(());
                }
                Ok(Some(link)) => {
                    if let Some(result) = self.store_original_jim_link(&link, value) {
                        return result;
                    }
                    if crate::vars::store_jim_cached_link(
                        &mut self.frames.borrow_mut(),
                        &mut self.namespaces.borrow_mut(),
                        &link,
                        value,
                    )
                    .is_ok()
                    {
                        return Ok(());
                    }
                }
                Err(_) => {}
            }
        }
        let bytes = self
            .native_string_bytes(&original)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        if self.is_native_jim_dictionary_name(original, &bytes) {
            return self.assign_native_jim_dictionary_sugar(original, value);
        }
        self.install_original_jim_variable(original, &bytes)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        if let Some(cache) = self
            .current_jim_variable_cache(original)
            .map_err(|error| self.report_cmd_error(error.into()))?
        {
            if let Some(JimVariableRead::Link(link)) = cache.cell.read() {
                if let Some(result) = self.store_original_jim_link(&link, value) {
                    return result;
                }
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
        assigned.map_err(|error| crate::builtins::var_error(self, &bytes, error))?;
        if !existed && element.is_none() {
            let key = if bytes.starts_with(b"::") {
                // JimCreateVariable owns a separate counted stripped global key.
                let tail = bytes
                    .iter()
                    .position(|byte| *byte != b':')
                    .unwrap_or(bytes.len());
                Owned::fresh(obj::new_string_bytes(&bytes[tail..]))
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
        self.install_original_jim_variable(original, &bytes)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        Ok(())
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
