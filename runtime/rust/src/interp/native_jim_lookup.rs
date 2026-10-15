// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original pinned Jim name objects, active command leases and weak VarVal caches.

use super::{
    native_arguments::OriginalArguments, native_jim_tailcall::JimCommandReference, Code, Command,
    Interp,
};
use crate::{
    frame::JimVariableRead,
    namespace::NsId,
    obj::{self, Owned, TclObj},
};
use tcl_syntax::value::{ValueError, ValueOps};

#[cfg(test)]
#[path = "native_jim_lookup/inventory_key_tests.rs"]
mod inventory_key_tests;

#[derive(Clone, Copy)]
pub(super) enum OriginalJimVariableReport {
    Missing,
    Silent,
}

enum OriginalJimCachedStore {
    Stored,
    Link(crate::frame::Link),
    Unavailable,
}

/// The actual List and original keys produced by one completed inventory.
/// A retained generation records production; this grants no later callable.
pub(super) struct OriginalJimCommandInventoryReturn {
    original: obj::NativeObjectLifetime,
    backing: crate::list::NativeListBacking,
    keys: Vec<crate::namespace::OriginalJimCommandInventoryKey>,
    namespace: OriginalJimLookupNamespace,
}

impl OriginalJimCommandInventoryReturn {
    pub(super) fn capture(
        actual: &Interp,
        original: *mut TclObj,
        keys: Vec<crate::namespace::OriginalJimCommandInventoryKey>,
    ) -> Result<Self, ValueError> {
        obj::check_native_liveness(original)?;
        let backing = crate::list::native_list_backing(original).ok_or(
            ValueError::CommandProtocolUnavailable("original Jim inventory result List"),
        )?;
        let result = Self {
            original: obj::NativeObjectLifetime::retain(original),
            backing,
            keys,
            namespace: OriginalJimLookupNamespace::capture(actual, crate::namespace::GLOBAL)?,
        };
        result.check(actual, original)?;
        Ok(result)
    }

    pub(super) fn check(&self, actual: &Interp, original: *mut TclObj) -> Result<(), ValueError> {
        self.namespace.check(actual)?;
        if self.original.checked_ptr()? != original {
            return Err(ValueError::CommandProtocolUnavailable(
                "original Jim inventory result header",
            ));
        }
        self.backing.check_original_header(original)?;
        let members = self.backing.elements()?;
        if members.len() != self.keys.len() {
            return Err(ValueError::CommandProtocolUnavailable(
                "original Jim inventory result members",
            ));
        }
        for (&member, key) in members.iter().zip(&self.keys) {
            if key.original()? != member {
                return Err(ValueError::CommandProtocolUnavailable(
                    "original Jim inventory original member",
                ));
            }
        }
        Ok(())
    }
}

/// The genuine current namespace holder and frame, before any reached getter.
/// A weak owner observation and allocation lease preserve actual native retirement.
pub(crate) struct OriginalJimLookupNamespace {
    interpreter: Interp,
    namespace: NsId,
    holder_birth: std::rc::Weak<Owned>,
    original: obj::NativeObjectLifetime,
    frame: super::native_arguments::OriginalArgumentFrame,
}
impl OriginalJimLookupNamespace {
    pub(crate) fn capture(interpreter: &Interp, namespace: NsId) -> Result<Self, ValueError> {
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
    pub(crate) fn check_current_arguments(
        &self,
        actual: &Interp,
        arguments: &OriginalArguments,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        arguments.check_for(actual)?;
        OriginalJimLookupNamespace::check(self, actual)?;
        if actual.current_ns() != self.namespace {
            return Err(arguments.retain_refusal(
                ValueError::CommandProtocolUnavailable("original Jim lookup current namespace")
                    .into(),
            ));
        }
        Ok(())
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
    reference: JimCommandReference,
}
impl Drop for JimActiveCommandScope {
    fn drop(&mut self) {
        let active = self
            .interp
            .jim_active_command_workers
            .borrow_mut()
            .pop()
            .expect("owned synchronous Jim invocation");
        assert!(JimCommandReference::same_reference(
            &self.reference,
            &active
        ));
        drop(active);
    }
}
impl Interp {
    pub(crate) fn original_jim_command_inventory_object(
        &mut self,
        root: tcl_runtime_api::NsId,
        pattern: Option<&[u8]>,
        include_spaces: bool,
        kind: tcl_runtime_api::NativeJimCommandInventoryKind,
    ) -> Result<*mut TclObj, ValueError> {
        let scope =
            super::native_operation_currency::NativeOperationScope::enter(self).map_err(|_| {
                ValueError::CommandProtocolUnavailable("original Jim inventory operation")
            })?;
        if tcl_runtime_api::Namespaces::root_command_context_checked(self)? != Some(root)
            || root != tcl_runtime_api::ROOT_NS
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "original Jim inventory selected table",
            ));
        }
        let namespace = OriginalJimLookupNamespace::capture(self, crate::namespace::GLOBAL)?;
        let caller_namespace = self.current_ns();
        let revision = self
            .namespaces()
            .guard_lookup_environment_revision()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original Jim inventory table revision",
            ))?;
        let check = || -> Result<(), ValueError> {
            scope.currency().ensure_current_or_refuse().map_err(|_| {
                ValueError::CommandProtocolUnavailable("original Jim inventory operation")
            })?;
            namespace.check(self)?;
            if self.current_ns() != caller_namespace {
                return Err(ValueError::CommandProtocolUnavailable(
                    "original Jim inventory caller namespace",
                ));
            }
            if self.namespaces().guard_lookup_environment_revision() != Some(revision) {
                return Err(ValueError::CommandProtocolUnavailable(
                    "original Jim inventory changed table",
                ));
            }
            Ok(())
        };
        check()?;
        let keys = self
            .namespaces()
            .original_jim_command_inventory_keys(kind, &|command, generation| {
                self.command_visible_for_surface_at(command, Some(generation))
            })?;
        let policy = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original Jim inventory naming issuer",
            ))?;
        let reports = keys
            .iter()
            .map(|key| key.report.clone())
            .collect::<Vec<_>>();
        let reached = policy
            .recipe()
            .jim_core_command_reached_ordinals(pattern, &reports)
            .map_err(|_| {
                ValueError::CommandProtocolUnavailable("original Jim inventory table selection")
            })?;
        let keys = reached
            .into_iter()
            .map(|index| &keys[index])
            .collect::<Vec<_>>();
        let mut candidates = Vec::with_capacity(keys.len());
        for key in &keys {
            let current = || {
                check()?;
                self.namespaces()
                    .check_original_jim_command_inventory_key(key)
            };
            if include_spaces {
                if pattern.is_none() {
                    current()?;
                    candidates.push(key.report.clone());
                } else {
                    let held = Owned::retain(key.original()?);
                    let reached =
                        self.native_object_string_bytes_using(held.as_ptr(), &current, &|error| {
                            error
                        });
                    drop(held);
                    current()?;
                    candidates.push(reached?.to_vec());
                }
                continue;
            }
            let bytes =
                self.native_object_string_bytes_using(key.original()?, &current, &|error| error)?;
            current()?;
            let admitted = policy
                .recipe()
                .jim_core_command_candidate_admitted(&bytes, include_spaces)
                .map_err(|_| {
                    ValueError::CommandProtocolUnavailable("original Jim inventory key admission")
                })?;
            if pattern.is_some() && admitted {
                let held = Owned::retain(key.original()?);
                let reached =
                    self.native_object_string_bytes_using(held.as_ptr(), &current, &|error| error);
                drop(held);
                current()?;
                candidates.push(reached?.to_vec());
                continue;
            }
            candidates.push(bytes.to_vec());
        }
        let selected = policy
            .recipe()
            .jim_core_command_ordinals(pattern, &candidates, include_spaces)
            .map_err(|_| {
                ValueError::CommandProtocolUnavailable("original Jim inventory native match")
            })?;
        check()?;
        let mut members = Vec::with_capacity(selected.len());
        let mut originals = Vec::with_capacity(selected.len());
        for index in selected {
            let key = &keys[index];
            self.namespaces()
                .check_original_jim_command_inventory_key(key)?;
            members.push(key.original()?);
            originals.push((**key).clone());
        }
        let result = self.new_list_object(&members);
        if let Err(error) = check() {
            super::drop_fresh(result);
            return Err(error);
        }
        if let Err(error) = self.record_original_source_inventory_return(result, originals) {
            super::drop_fresh(result);
            return Err(error);
        }
        Ok(result)
    }

    /// Preserve the selected original variable-inventory namespace, frame and
    /// key-object stages rather than reconstructing keys from their reports.
    pub(crate) fn original_jim_variable_inventory(
        &mut self,
        scope: tcl_registry::commands::tcl::NativeJimInfoScope,
        original: &[*mut TclObj],
        arguments: &OriginalArguments,
        selector: usize,
        kind: tcl_registry::commands::tcl::NativeJimVariableInventoryKind,
    ) -> Code {
        let refuse = |error: ValueError| arguments.retain_refusal(error.into());
        let reached = (|| -> Result<Code, tcl_cmd_core::CmdError> {
            let protocol =
                self.original_jim_variable_inventory_preflight(original, arguments, selector)?;
            let namespace = self.current_ns();
            let namespace_scope =
                OriginalJimLookupNamespace::capture(self, namespace).map_err(&refuse)?;
            let check = |actual: &Interp| {
                OriginalJimLookupNamespace::check_current_arguments(
                    &namespace_scope,
                    actual,
                    arguments,
                )
            };
            check(self)?;
            let mut pattern = None;
            if scope.variable_inventory_reads_namespace() {
                let namespace_bytes = self.materialize_jim_namespace(namespace);
                check(self)?;
                let namespace_bytes = namespace_bytes.map_err(&refuse)?;
                if namespace_bytes.is_empty() {
                    let bytes = original
                        .get(1)
                        .map(|value| self.native_string_bytes(value))
                        .transpose();
                    check(self)?;
                    pattern = bytes.map_err(&refuse)?;
                }
                if scope.variable_inventory_uses_helper(&namespace_bytes, pattern.as_deref()) {
                    let head = Owned::fresh(super::new_string(b"namespace info"));
                    let mut invocation = Vec::with_capacity(original.len() + 1);
                    invocation.push(head.as_ptr());
                    invocation.extend_from_slice(original);
                    let code = self.dispatch(&invocation);
                    check(self)?;
                    return Ok(code);
                }
            }
            let top = self.frames.borrow().current_level() == 0;
            if kind == tcl_registry::commands::tcl::NativeJimVariableInventoryKind::Locals && top {
                let context = self.native_jim_object_context().map_err(&refuse)?;
                check(self)?;
                self.set_result(context.empty_object().as_ptr());
                check(self)?;
                return Ok(Code::Ok);
            }
            let matcher = self.original_jim_variable_inventory_matcher()?;
            let result = Owned::fresh(self.new_list_object(&[]));
            check(self)?;
            if pattern.is_none() {
                let bytes = original
                    .get(1)
                    .map(|value| self.native_string_bytes(value))
                    .transpose();
                check(self)?;
                pattern = bytes.map_err(&refuse)?;
            }
            let lookup =
                tcl_syntax::native_jim_lookup::NativeJimLookupProtocol::variable_inventory_lookup(
                    protocol,
                    pattern.as_deref(),
                );
            let inventory = self
                .capture_original_jim_variable_inventory_table(kind, top, lookup)
                .map_err(&refuse)?;
            check(self)?;
            crate::frame::OriginalJimVariableInventory::check(&inventory).map_err(&refuse)?;
            self.append_original_jim_inventory_keys(
                &inventory,
                pattern.as_deref(),
                result.as_ptr(),
                matcher,
                arguments,
                &check,
            )?;
            check(self)?;
            crate::frame::OriginalJimVariableInventory::check(&inventory).map_err(&refuse)?;
            self.set_result(result.as_ptr());
            check(self)?;
            crate::frame::OriginalJimVariableInventory::check(&inventory).map_err(&refuse)?;
            Ok(Code::Ok)
        })();
        if let Err(first) = arguments.check_for(self) {
            return self.report_cmd_error(first);
        }
        match reached {
            Ok(code) => code,
            Err(error) => self.report_cmd_error(error),
        }
    }
    fn original_jim_variable_inventory_preflight(
        &self,
        original: &[*mut TclObj],
        arguments: &OriginalArguments,
        selector: usize,
    ) -> Result<tcl_syntax::native_jim_lookup::NativeJimLookupProtocol, tcl_cmd_core::CmdError>
    {
        let protocol = self
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original Jim variable inventory issuer",
            ))?;
        if original.is_empty() || original.len() > 2 {
            return Err(ValueError::CommandProtocolUnavailable(
                "original Jim accepted variable inventory argv",
            )
            .into());
        }
        for (offset, &value) in original.iter().enumerate() {
            arguments.check_original_for(self, selector + offset, value)?;
        }
        Ok(protocol)
    }
    fn original_jim_variable_inventory_matcher(
        &self,
    ) -> Result<tcl_syntax::native_glob::NativeGlobProtocol, ValueError> {
        self.selected_variable_table_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original Jim variable inventory table protocol",
            ))?;
        Ok(
            tcl_syntax::native_glob::NativeGlobProtocol::from_name_policy(
                self.name_policy_protocol()
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "original Jim inventory name policy",
                    ))?,
            ),
        )
    }
    fn capture_original_jim_variable_inventory_table(
        &self,
        kind: tcl_registry::commands::tcl::NativeJimVariableInventoryKind,
        top: bool,
        lookup: tcl_syntax::native_jim_lookup::NativeJimVariableInventoryLookup<'_>,
    ) -> Result<crate::frame::OriginalJimVariableInventory, ValueError> {
        if kind.uses_global_table() || top {
            crate::frame::VarTable::capture_original_jim_inventory(
                self.namespaces().var_table(crate::namespace::GLOBAL),
                kind.includes_links(),
                lookup,
            )
        } else {
            let frames = self.frames.borrow();
            let level = frames.current_level();
            let table = frames
                .table(level)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original Jim variable inventory active frame",
                ))?;
            crate::frame::VarTable::capture_original_jim_inventory(
                table,
                kind.includes_links(),
                lookup,
            )
        }
    }
    fn append_original_jim_inventory_keys(
        &mut self,
        inventory: &crate::frame::OriginalJimVariableInventory,
        pattern: Option<&[u8]>,
        result: *mut TclObj,
        matcher: tcl_syntax::native_glob::NativeGlobProtocol,
        arguments: &OriginalArguments,
        check: &dyn Fn(&Interp) -> Result<(), tcl_cmd_core::CmdError>,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        let refuse = |error: ValueError| arguments.retain_refusal(error.into());
        for key in crate::frame::OriginalJimVariableInventory::keys(inventory) {
            check(self)?;
            crate::frame::OriginalJimVariableInventory::check(inventory).map_err(&refuse)?;
            let original_key =
                crate::frame::OriginalJimVariableInventoryKey::original(key).map_err(&refuse)?;
            let matched = if let Some(pattern) = pattern {
                let bytes = self.native_string_bytes(&original_key);
                check(self)?;
                crate::frame::OriginalJimVariableInventory::check(inventory).map_err(&refuse)?;
                crate::frame::OriginalJimVariableInventoryKey::original(key).map_err(&refuse)?;
                matcher
                    .match_name_pattern(
                        tcl_syntax::native_glob::NativeNameGlobPurpose::InfoVariablesScan,
                        pattern,
                        &bytes.map_err(&refuse)?,
                    )
                    .map_err(|_| {
                        refuse(ValueError::CommandProtocolUnavailable(
                            "original Jim inventory key pattern",
                        ))
                    })?
            } else {
                true
            };
            if matched {
                let currency = arguments.currency();
                let appended = crate::list::append_native_element_with_currency(
                    result,
                    original_key,
                    tcl_syntax::native_string::NativeStringProtocol::Jim084,
                    currency,
                );
                check(self)?;
                crate::frame::OriginalJimVariableInventory::check(inventory).map_err(&refuse)?;
                crate::frame::OriginalJimVariableInventoryKey::original(key).map_err(&refuse)?;
                appended.map_err(&refuse)?;
            }
        }
        Ok(())
    }
    /// Execute the original inventory preflight before any reached namespace getter.
    /// The actual complete argv and weak namespace/frame owner remain independent.
    pub(crate) fn original_jim_command_inventory(
        &mut self,
        scope: tcl_registry::commands::tcl::NativeJimInfoScope,
        original: &[*mut TclObj],
        arguments: &OriginalArguments,
        selector: usize,
        kind: tcl_registry::commands::tcl::NativeJimCommandInventoryKind,
    ) -> Code {
        let reached = (|| -> Result<Code, tcl_cmd_core::CmdError> {
            if self
                .native_invocation_dialect()
                .native_jim_info_protocol()
                .is_none()
                || original.is_empty()
            {
                return Err(ValueError::CommandProtocolUnavailable(
                    "original Jim inventory issuer",
                )
                .into());
            }
            for (offset, &value) in original.iter().enumerate() {
                arguments.check_original_for(self, selector + offset, value)?;
            }
            let namespace = self.current_ns();
            let namespace_scope = OriginalJimLookupNamespace::capture(self, namespace)?;
            let check = |actual: &Interp| -> Result<(), tcl_cmd_core::CmdError> {
                arguments.check_for(actual)?;
                OriginalJimLookupNamespace::check(&namespace_scope, actual)?;
                if actual.current_ns() != namespace {
                    return Err(arguments.retain_refusal(
                        ValueError::CommandProtocolUnavailable(
                            "original Jim inventory current namespace",
                        )
                        .into(),
                    ));
                }
                Ok(())
            };
            check(self)?;
            let first_operand = original
                .get(1)
                .map(|value| self.native_string_bytes(value))
                .transpose();
            check(self)?;
            let first_operand = first_operand?;
            let selected = if let Some(selected) =
                scope.command_inventory_preflight(original.len() - 1, first_operand.as_deref())
            {
                selected
            } else {
                let bytes = self.materialize_jim_namespace(namespace);
                check(self)?;
                scope.command_inventory(original.len() - 1, first_operand.as_deref(), &bytes?)
            };
            let code = match selected {
                tcl_registry::commands::tcl::NativeJimCommandInventory::NamespaceHelper => {
                    let head = Owned::fresh(super::new_string(b"namespace info"));
                    let mut invocation = Vec::with_capacity(original.len() + 1);
                    invocation.push(head.as_ptr());
                    invocation.extend_from_slice(original);
                    self.dispatch(&invocation)
                }
                tcl_registry::commands::tcl::NativeJimCommandInventory::Flat {
                    pattern,
                    include_spaces,
                } => {
                    if let Some(index) = pattern {
                        // The original hash-table matcher reads the actual pattern
                        // before looking up or iterating any table entries.
                        let reached = self.native_string_bytes(&original[index + 1]);
                        check(self)?;
                        reached?;
                    }
                    let value = tcl_cmd_core::info::jim_core_command_list(
                        self,
                        pattern.map(|index| &original[index + 1]),
                        include_spaces,
                        kind,
                    );
                    if let Err(error) = check(self) {
                        if let Ok(value) = value {
                            super::drop_fresh(value);
                        }
                        return Err(error);
                    }
                    match value {
                        Ok(value) => {
                            self.set_result(value);
                            Code::Ok
                        }
                        Err(error) => self.report_cmd_error(error),
                    }
                }
                tcl_registry::commands::tcl::NativeJimCommandInventory::WrongArguments => {
                    let selector = self.native_string_bytes(&original[0]);
                    check(self)?;
                    let selector = selector?;
                    self.report_cmd_error(
                tcl_cmd_core::CmdError::new_bytes(
                    tcl_registry::commands::tcl::NativeJimCommandInventory::wrong_arguments_message(
                        &selector,
                    ),
                )
                .with_native_string_result(tcl_syntax::native_string::NativeStringProtocol::Jim084),
            )
                }
            };
            check(self)?;
            Ok(code)
        })();
        match reached {
            Ok(code) => code,
            Err(error) => self.report_cmd_error(error),
        }
    }

    pub(super) fn retain_active_jim_command(
        &self,
        command: &Command,
        token: Option<u64>,
        selected: Option<JimCommandReference>,
    ) -> Result<Option<JimActiveCommandScope>, ValueError> {
        if self
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .is_none()
        {
            return Ok(None);
        }
        let Some(token) = token else {
            return Ok(None);
        };
        let reference = match selected {
            Some(reference) => {
                JimCommandReference::check(&reference, self)?;
                if JimCommandReference::token(&reference) != token
                    || !JimCommandReference::command(&reference).is_same_binding(command)
                {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "selected Jim worker mismatch",
                    ));
                }
                reference
            }
            None => self.capture_jim_command_reference(command, token)?,
        };
        self.jim_active_command_workers
            .borrow_mut()
            .push(reference.clone());
        Ok(Some(JimActiveCommandScope {
            interp: self.clone(),
            reference,
        }))
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
            OriginalJimLookupNamespace::check(&namespace_scope, &lookup_interpreter)
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
                let command = match world.jim_node_command(token) {
                    Some(command) => Some(command),
                    None => match self.retained_jim_command_reference(token) {
                        Some(reference) => {
                            JimCommandReference::check(&reference, self)?;
                            Some(JimCommandReference::command(&reference))
                        }
                        None => None,
                    },
                };
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
        self.read_original_named_variable_reported(original, OriginalJimVariableReport::Missing)
    }

    pub(crate) fn exists_original_jim_variable(
        &mut self,
        original: *mut TclObj,
    ) -> Result<bool, Code> {
        if self.observed_name_policy_selected()
            || self
                .native_invocation_dialect()
                .native_jim_lookup_protocol()
                .is_none()
        {
            return Err(self.report_cmd_error(
                ValueError::CommandProtocolUnavailable("original Jim variable existence query")
                    .into(),
            ));
        }
        match self
            .read_original_named_variable_reported(original, OriginalJimVariableReport::Silent)
        {
            Ok(_) => Ok(true),
            Err(code) if self.host_refusal_pending() => Err(code),
            Err(_) => Ok(false),
        }
    }

    pub(super) fn read_original_named_variable_reported(
        &mut self,
        original: *mut TclObj,
        report: OriginalJimVariableReport,
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
                    if let Some(result) = self.read_original_jim_link(original, &link, report) {
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
            match cache
                .cell
                .read_checked()
                .map_err(|error| self.report_cmd_error(error.into()))?
            {
                Some(JimVariableRead::Link(link)) => {
                    if let Some(result) = self.read_original_jim_link(original, &link, report) {
                        return result;
                    }
                }
                Some(JimVariableRead::Scalar(value))
                    if matches!(report, OriginalJimVariableReport::Silent) =>
                {
                    return Ok(value);
                }
                _ => {}
            }
        }
        match report {
            OriginalJimVariableReport::Missing => self.read_named_variable(&bytes),
            OriginalJimVariableReport::Silent => Err(Code::Error),
        }
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
        let interp = jim();
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
