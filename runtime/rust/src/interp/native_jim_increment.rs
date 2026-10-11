// SPDX-License-Identifier: AGPL-3.0-or-later
//! Jim's original-name JIM_UNSHARED lookup precedes member integer conversion.

use super::{
    native_arguments::OriginalArguments, native_operation_currency::NativeOperationCurrency, Code,
    Interp,
};
#[cfg(test)]
use crate::obj::Owned;
use crate::{
    frame::JimVariableRead,
    native_substitution,
    obj::{self, TclObj},
};
use std::rc::{Rc, Weak};
use tcl_cmd_core::CmdError;
use tcl_syntax::{native_string::NativeStringProtocol, value::ValueError};

pub(crate) enum JimDictionaryPathFailure {
    Command(CmdError),
    MissingKey(Vec<u8>),
    Guest(Code),
}
impl From<CmdError> for JimDictionaryPathFailure {
    fn from(error: CmdError) -> Self {
        Self::Command(error)
    }
}
impl From<ValueError> for JimDictionaryPathFailure {
    fn from(error: ValueError) -> Self {
        Self::Command(error.into())
    }
}
impl JimDictionaryPathFailure {
    pub(crate) fn report(self, interpreter: &mut Interp) -> Code {
        match self {
            Self::Command(error) => interpreter.report_cmd_error(error),
            Self::Guest(code) => code,
            Self::MissingKey(bytes) => {
                let error = tcl_cmd_core::dict::missing_key_error(
                    interpreter,
                    tcl_syntax::naming::NativeDictionaryMissingKeyOperation::Get,
                    &bytes,
                );
                interpreter.report_cmd_error(error)
            }
        }
    }
}

pub(in crate::interp) struct OriginalJimDictionaryRoot {
    interpreter: Interp,
    currency: NativeOperationCurrency,
    location: OriginalJimDictionaryLocation,
    frame: Weak<()>,
    aliases: Vec<crate::dict::NativeDictionaryStageCheck>,
    _original: obj::NativeObjectLifetime,
}
enum OriginalJimDictionaryLocation {
    Cell(crate::frame::CapturedJimDictionaryCell),
    Member {
        parent: Rc<OriginalJimDictionaryRoot>,
        key: obj::NativeObjectLifetime,
        bytes: Vec<u8>,
        expected: std::cell::Cell<*mut TclObj>,
    },
}
impl OriginalJimDictionaryRoot {
    fn storage_current(&self) -> Result<(), ValueError> {
        if !self
            .interpreter
            .frames
            .borrow()
            .owns_original_jim_frame_incarnation(&self.frame)
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "captured original Jim dictionary frame",
            ));
        }
        for check in &self.aliases {
            check()?;
        }
        match &self.location {
            OriginalJimDictionaryLocation::Cell(cell) => cell.check(),
            OriginalJimDictionaryLocation::Member {
                parent,
                bytes,
                expected,
                ..
            } => {
                parent.storage_current()?;
                if crate::dict::original_cached_member(parent.original(), bytes)?
                    != Some(expected.get())
                {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "captured original Jim dictionary member root",
                    ));
                }
                obj::check_native_liveness(expected.get())
            }
        }
    }
    pub(in crate::interp) fn check(&self) -> Result<(), CmdError> {
        self.currency
            .ensure_current_or_refuse()
            .map_err(CmdError::from_execution_refusal)?;
        self.storage_current().map_err(CmdError::from)
    }
    pub(in crate::interp) fn original(&self) -> *mut TclObj {
        match &self.location {
            OriginalJimDictionaryLocation::Cell(cell) => cell.root(),
            OriginalJimDictionaryLocation::Member { expected, .. } => expected.get(),
        }
    }
    pub(in crate::interp) fn stage_check(&self) -> crate::dict::NativeDictionaryStageCheck {
        let interpreter = self.interpreter.clone();
        let frame = self.frame.clone();
        let aliases = self.aliases.clone();
        let storage: crate::dict::NativeDictionaryStageCheck = match &self.location {
            OriginalJimDictionaryLocation::Cell(cell) => {
                let cell = cell.clone();
                Rc::new(move || cell.check())
            }
            OriginalJimDictionaryLocation::Member {
                parent,
                bytes,
                expected,
                ..
            } => {
                let parent = Rc::clone(parent);
                let bytes = bytes.clone();
                let expected = expected.get();
                Rc::new(move || {
                    parent.storage_current()?;
                    if crate::dict::original_cached_member(parent.original(), &bytes)?
                        != Some(expected)
                    {
                        return Err(ValueError::CommandProtocolUnavailable(
                            "captured original Jim dictionary member root",
                        ));
                    }
                    obj::check_native_liveness(expected)
                })
            }
        };
        Rc::new(move || {
            if !interpreter
                .frames
                .borrow()
                .owns_original_jim_frame_incarnation(&frame)
            {
                return Err(ValueError::CommandProtocolUnavailable(
                    "captured original Jim dictionary frame",
                ));
            }
            for check in &aliases {
                check()?;
            }
            storage()
        })
    }
    pub(in crate::interp) fn publish(&self, value: *mut TclObj) -> Result<(), CmdError> {
        self.check()?;
        match &self.location {
            OriginalJimDictionaryLocation::Cell(cell) => {
                let reached = cell.publish(value);
                self.check()?;
                reached.map_err(CmdError::from)
            }
            OriginalJimDictionaryLocation::Member {
                parent,
                key,
                expected,
                ..
            } => {
                let selected = self
                    .interpreter
                    .native_invocation_dialect()
                    .native_dictionary_root_protocol(
                        tcl_registry::native_dictionary::NativeDictionaryRootPurpose::VariableSet,
                    )
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "actual Jim member-root publication",
                    ))?;
                let reached = crate::dict::EnteredNativeDictionary::prepare_original(
                    parent.original(),
                    selected,
                    NativeStringProtocol::Jim084,
                    &self.currency,
                    parent.stage_check(),
                );
                parent.check()?;
                let mut prepared = reached?;
                let member = prepared.capture_member(key.as_ptr());
                parent.check()?;
                let member = member?;
                member.expect_publication(value);
                let stored = prepared.set_member(key.as_ptr(), value);
                parent.check()?;
                member.check()?;
                stored?;
                // Same authorised member and parent; publish before retirement.
                expected.set(value);
                let published = parent.publish(prepared.as_ptr());
                drop(prepared);
                self.check()?;
                published
            }
        }
    }
}

impl Interp {
    pub(in crate::interp) fn capture_original_jim_dictionary_root(
        &mut self,
        original: *mut TclObj,
        currency: &NativeOperationCurrency,
    ) -> Result<Option<OriginalJimDictionaryRoot>, CmdError> {
        self.capture_original_jim_dictionary_root_inner(original, currency, &mut Vec::new())
    }
    fn capture_original_jim_dictionary_root_inner(
        &mut self,
        original: *mut TclObj,
        currency: &NativeOperationCurrency,
        aliases: &mut Vec<crate::dict::NativeDictionaryStageCheck>,
    ) -> Result<Option<OriginalJimDictionaryRoot>, CmdError> {
        currency
            .ensure_current_or_refuse()
            .map_err(CmdError::from_execution_refusal)?;
        if self.observed_name_policy_selected() {
            return Err(ValueError::CommandProtocolUnavailable(
                "actual Jim dictionary name authority",
            )
            .into());
        }
        self.native_invocation_dialect()
            .native_jim_lookup_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "actual Jim dictionary root lookup",
            ))?;
        self.associate_native_jim_arguments(&[original])?;
        obj::check_native_liveness(original)?;
        let mut cached = self.current_jim_variable_cache(original)?;
        if cached.is_none() {
            let reached = self.native_object_string_bytes(original);
            currency
                .ensure_current_or_refuse()
                .map_err(CmdError::from_execution_refusal)?;
            obj::check_native_liveness(original)?;
            let bytes = reached?;
            if self.is_native_jim_dictionary_name(original, &bytes) {
                let context = self.native_jim_object_context()?;
                let current = || {
                    currency.ensure_current_or_refuse().map_err(|_| {
                        ValueError::CommandProtocolUnavailable(
                            "entered original Jim member-root tuple",
                        )
                    })
                };
                let converted =
                    native_substitution::ensure_dictionary_substitution_with_stage_check(
                        original,
                        &context,
                        Some(&current),
                    );
                currency
                    .ensure_current_or_refuse()
                    .map_err(CmdError::from_execution_refusal)?;
                converted?;
                let (name, key) =
                    native_substitution::with_dictionary_substitution(original, |name, key| {
                        (name, key)
                    })
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "original Jim member-root tuple",
                    ))?;
                let name_lease = obj::NativeObjectLifetime::retain(name);
                let key_lease = obj::NativeObjectLifetime::retain(key);
                let Some(parent) =
                    self.capture_original_jim_dictionary_root_inner(name, currency, aliases)?
                else {
                    return Ok(None);
                };
                let reached = crate::dict::EnteredNativeDictionary::borrowed_converted(
                    parent.original(),
                    NativeStringProtocol::Jim084,
                    currency,
                    parent.stage_check(),
                );
                parent.check()?;
                let prepared = reached?;
                let member = prepared.capture_member(key);
                parent.check()?;
                let member = member?;
                let Some(value) = member.original() else {
                    return Ok(None);
                };
                // The selected key getter already ran in capture_member. Reuse
                // its retained resident bytes; no second original getter.
                let key_bytes = member.key_bytes().to_vec();
                let frame = parent.frame.clone();
                let owner = OriginalJimDictionaryRoot {
                    interpreter: self.clone(),
                    currency: currency.clone(),
                    frame,
                    aliases: Vec::new(),
                    _original: obj::NativeObjectLifetime::retain(value),
                    location: OriginalJimDictionaryLocation::Member {
                        parent: Rc::new(parent),
                        key: key_lease,
                        bytes: key_bytes,
                        expected: std::cell::Cell::new(value),
                    },
                };
                drop(prepared);
                drop(name_lease);
                owner.check()?;
                return Ok(Some(owner));
            }
            let installed = self.install_original_jim_variable(original, &bytes);
            currency
                .ensure_current_or_refuse()
                .map_err(CmdError::from_execution_refusal)?;
            installed?;
            cached = self.current_jim_variable_cache(original)?;
        }
        let Some(cached) = cached else {
            return Ok(None);
        };
        let frame = self
            .frames
            .borrow()
            .original_jim_frame_incarnation(cached.global);
        match cached.cell.read_checked()? {
            Some(JimVariableRead::Scalar(_)) => {
                let cell = cached.cell.capture_dictionary_cell()?;
                let owner = OriginalJimDictionaryRoot {
                    interpreter: self.clone(),
                    currency: currency.clone(),
                    _original: obj::NativeObjectLifetime::retain(cell.root()),
                    location: OriginalJimDictionaryLocation::Cell(cell),
                    frame,
                    aliases: aliases.clone(),
                };
                owner.check()?;
                Ok(Some(owner))
            }
            Some(JimVariableRead::Link(link)) => {
                let target =
                    link.original_jim_target
                        .ok_or(ValueError::CommandProtocolUnavailable(
                            "original Jim dictionary link target",
                        ))?;
                let target_original = target.original_name()?;
                let cell_check = cached.cell.original_stage_check()?;
                let interpreter = self.clone();
                let alias_check: crate::dict::NativeDictionaryStageCheck = Rc::new(move || {
                    if !interpreter
                        .frames
                        .borrow()
                        .owns_original_jim_frame_incarnation(&frame)
                    {
                        return Err(ValueError::CommandProtocolUnavailable(
                            "captured original Jim alias frame",
                        ));
                    }
                    cell_check()
                });
                alias_check()?;
                let level = self.frames.borrow().jim_link_level(&target.frame).ok_or(
                    ValueError::CommandProtocolUnavailable(
                        "retired original Jim dictionary alias frame",
                    ),
                )?;
                let saved = self.frames.borrow_mut().set_active_level(level);
                let namespace = self
                    .current_ns
                    .replace(self.frames.borrow().frame_ns(level));
                aliases.push(alias_check);
                let reached = self.capture_original_jim_dictionary_root_inner(
                    target_original,
                    currency,
                    aliases,
                );
                aliases.pop();
                self.current_ns.set(namespace);
                self.frames.borrow_mut().set_active_level(saved);
                currency
                    .ensure_current_or_refuse()
                    .map_err(CmdError::from_execution_refusal)?;
                target.original_name()?;
                reached
            }
            None => Ok(None),
        }
    }

    /// Jim's immediate parent path publication, using the existing Dict worker.
    /// Accepted parent mutations remain reached if a later child fails.
    pub(crate) fn mutate_original_jim_dictionary_path(
        &mut self,
        name: *mut TclObj,
        keys: &[*mut TclObj],
        value: Option<*mut TclObj>,
        purpose: tcl_registry::native_dictionary::NativeDictionaryRootPurpose,
        arguments: &OriginalArguments,
    ) -> Result<Option<*mut TclObj>, JimDictionaryPathFailure> {
        let reached = (|| -> Result<Option<*mut TclObj>, JimDictionaryPathFailure> {
            use tcl_registry::native_dictionary::NativeDictionaryMissingRoot;
            arguments.check()?;
            let selected = self
                .native_invocation_dialect()
                .native_dictionary_root_protocol(purpose)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "actual Jim dictionary path worker",
                ))?;
            if keys.is_empty() || !selected.immediate_children {
                return Err(ValueError::CommandProtocolUnavailable(
                    "original Jim dictionary path operands",
                )
                .into());
            }
            let mut captured =
                self.capture_original_jim_dictionary_root(name, arguments.currency())?;
            if captured.is_none() {
                if value.is_none()
                    && selected.missing == NativeDictionaryMissingRoot::RequireExisting
                {
                    return Ok(None);
                }
                let empty =
                    crate::dict::new_dict_obj_native(&[], None, NativeStringProtocol::Jim084)?;
                // Jim_SetDictKeysVector installs a new rc0 empty root immediately.
                let lease = obj::NativeObjectLifetime::retain(empty);
                let assigned = self.assign_original_named_variable(name, empty);
                if obj::allocation_is_live(empty) && unsafe { (*empty).ref_count } == 0 {
                    super::drop_fresh(empty);
                }
                drop(lease);
                arguments.check()?;
                if let Err(code) = assigned {
                    if let Some(first) = self.native_execution_refusal() {
                        return Err(CmdError::from_execution_refusal(first).into());
                    }
                    return Err(JimDictionaryPathFailure::Guest(code));
                }
                captured = self.capture_original_jim_dictionary_root(name, arguments.currency())?;
            }
            let captured = captured.ok_or(ValueError::CommandProtocolUnavailable(
                "published original Jim dictionary cell",
            ))?;
            let current = crate::dict::EnteredNativeDictionary::prepare_original(
                captured.original(),
                selected,
                NativeStringProtocol::Jim084,
                arguments.currency(),
                captured.stage_check(),
            );
            captured.check()?;
            let mut current = current?;
            let root = current.as_ptr();
            let mut parents = Vec::new();
            for (index, &key) in keys.iter().enumerate() {
                arguments.check()?;
                if index + 1 == keys.len() {
                    let reached = if let Some(value) = value {
                        current.set_member(key, value).map(|()| true)
                    } else {
                        current.remove_member(key)
                    };
                    captured.check()?;
                    if !reached? && selected.missing == NativeDictionaryMissingRoot::RequireExisting
                    {
                        return Ok(None);
                    }
                    break;
                }
                current.invalidate()?;
                let reached = current.capture_member(key);
                captured.check()?;
                let member = reached?;
                let child = match member.original() {
                    Some(child) => crate::dict::EnteredNativeDictionary::select_original_child(
                        child,
                        NativeStringProtocol::Jim084,
                        arguments.currency(),
                        member.stage_check(),
                    )?,
                    None if value.is_some() => crate::dict::EnteredNativeDictionary::empty(
                        NativeStringProtocol::Jim084,
                        arguments.currency(),
                        member.stage_check(),
                    )?,
                    None => {
                        return Err(JimDictionaryPathFailure::MissingKey(
                            member.key_bytes().to_vec(),
                        ))
                    }
                };
                // Install the copied/new child immediately, before SetDictFromAny
                // on the next child. Later failure retains this parent mutation.
                if member.original() != Some(child.as_ptr()) {
                    member.expect_publication(child.as_ptr());
                    let installed = current.set_member(key, child.as_ptr());
                    captured.check()?;
                    member.check()?;
                    installed?;
                }
                let converted = child.convert();
                captured.check()?;
                converted?;
                parents.push(current);
                current = child;
            }
            current.invalidate()?;
            for parent in &parents {
                parent.invalidate()?;
            }
            captured.check()?;
            captured.publish(root)?;
            drop(current);
            drop(parents);
            captured.check()?;
            arguments.check()?;
            Ok(Some(root))
        })();
        arguments.check()?;
        reached
    }

    pub(super) fn is_native_jim_dictionary_name(
        &self,
        original: *mut TclObj,
        bytes: &[u8],
    ) -> bool {
        native_substitution::with_dictionary_substitution(original, |_, _| ()).is_some()
            || (bytes.last() == Some(&b')')
                && bytes
                    .split(|byte| *byte == 0)
                    .next()
                    .unwrap_or_default()
                    .contains(&b'('))
    }

    pub(super) fn read_original_named_variable_for_update(
        &mut self,
        original: *mut TclObj,
    ) -> Result<Option<*mut TclObj>, Code> {
        if let Some(cache) = self
            .current_jim_variable_cache(original)
            .map_err(|error| self.report_cmd_error(error.into()))?
        {
            match cache
                .cell
                .read_checked()
                .map_err(|error| self.report_cmd_error(error.into()))?
            {
                Some(JimVariableRead::Scalar(value)) => return Ok(Some(value)),
                Some(JimVariableRead::Link(link)) => {
                    if let Some(result) = self.read_original_jim_link_for_update(&link) {
                        return result;
                    }
                    return Err(self.report_cmd_error(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original Jim update link target",
                        )
                        .into(),
                    ));
                }
                None => {}
            }
        }
        let bytes = self
            .native_object_string_bytes(original)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        if self.is_native_jim_dictionary_name(original, &bytes) {
            return self.read_native_jim_dictionary_sugar(original, true);
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
                if let Some(result) = self.read_original_jim_link_for_update(&link) {
                    return result;
                }
                return Err(self.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original Jim update link target",
                    )
                    .into(),
                ));
            }
        }
        // JIM_NONE lookup of a missing ordinary name does not create a guest
        // error or install a primary cache. No original value owner is added.
        Ok(self.var_get(&bytes))
    }

    pub(super) fn read_native_jim_dictionary_sugar(
        &mut self,
        original: *mut TclObj,
        unshared: bool,
    ) -> Result<Option<*mut TclObj>, Code> {
        let arguments = OriginalArguments::enter(self, &[original])
            .map_err(|error| self.report_cmd_error(error))?;
        let reached = (|| {
            let context = self
                .native_jim_object_context()
                .map_err(|error| self.report_cmd_error(error.into()))?;
            let check = || {
                arguments.check().map_err(|_| {
                    ValueError::CommandProtocolUnavailable("entered original Jim read tuple")
                })
            };
            let converted = native_substitution::ensure_dictionary_substitution_with_stage_check(
                original,
                &context,
                Some(&check),
            );
            arguments
                .check()
                .map_err(|error| self.report_cmd_error(error))?;
            converted.map_err(|error| self.report_cmd_error(error.into()))?;
            let (name, key) =
                native_substitution::with_dictionary_substitution(original, |name, key| {
                    (name, key)
                })
                .ok_or_else(|| {
                    self.report_cmd_error(
                        ValueError::CommandProtocolUnavailable("original Jim read tuple").into(),
                    )
                })?;
            let members = OriginalArguments::enter(self, &[name, key])
                .map_err(|error| self.report_cmd_error(error))?;
            let captured = self.capture_original_jim_dictionary_root(name, arguments.currency());
            arguments
                .check()
                .map_err(|error| self.report_cmd_error(error))?;
            let Some(captured) = captured.map_err(|error| self.report_cmd_error(error))? else {
                // JimDictExpandArrayVariable first performs the same parent
                // JIM_ERRMSG read. Preserve its missing-variable completion.
                let missing = self.read_original_named_variable(name);
                arguments
                    .check()
                    .map_err(|error| self.report_cmd_error(error))?;
                match missing {
                    Err(code) if self.host_refusal_pending() => return Err(code),
                    Err(_) => return Ok(None),
                    Ok(_) => {
                        return Err(self.report_cmd_error(
                            ValueError::CommandProtocolUnavailable(
                                "uncaptured original Jim read root",
                            )
                            .into(),
                        ))
                    }
                }
            };
            let prepared = crate::dict::EnteredNativeDictionary::borrowed_converted(
                captured.original(),
                NativeStringProtocol::Jim084,
                arguments.currency(),
                captured.stage_check(),
            );
            captured
                .check()
                .map_err(|error| self.report_cmd_error(error))?;
            let prepared = match prepared {
                Ok(prepared) => prepared,
                Err(error) if error.native_access_refusal().is_none() => {
                    self.native_jim_dictionary_read_error(name, key, true);
                    arguments
                        .check()
                        .map_err(|error| self.report_cmd_error(error))?;
                    return Ok(None);
                }
                Err(error) => return Err(self.report_cmd_error(error.into())),
            };
            let member = prepared.get_member(key);
            captured
                .check()
                .map_err(|error| self.report_cmd_error(error))?;
            let Some(member) = member.map_err(|error| self.report_cmd_error(error.into()))? else {
                self.native_jim_dictionary_read_error(name, key, false);
                arguments
                    .check()
                    .map_err(|error| self.report_cmd_error(error))?;
                return Ok(None);
            };
            if unshared {
                // JimDictSugarGet's actual JIM_UNSHARED read tests sharing only
                // after SetDictFromAny and member lookup. This is independently
                // selected from path mutation's pre-conversion root schedule.
                let duplicate = crate::dict::EnteredNativeDictionary::select_original_child(
                    captured.original(),
                    NativeStringProtocol::Jim084,
                    arguments.currency(),
                    captured.stage_check(),
                );
                captured
                    .check()
                    .map_err(|error| self.report_cmd_error(error))?;
                let duplicate = duplicate.map_err(|error| self.report_cmd_error(error.into()))?;
                if duplicate.as_ptr() != captured.original() {
                    let published = captured.publish(duplicate.as_ptr());
                    drop(duplicate);
                    captured
                        .check()
                        .map_err(|error| self.report_cmd_error(error))?;
                    published.map_err(|error| self.report_cmd_error(error))?;
                }
            }
            drop(prepared);
            drop(members);
            arguments
                .check()
                .map_err(|error| self.report_cmd_error(error))?;
            obj::check_native_liveness(member)
                .map_err(|error| self.report_cmd_error(error.into()))?;
            Ok(Some(member))
        })();
        arguments
            .check()
            .map_err(|error| self.report_cmd_error(error))?;
        reached
    }

    pub(super) fn assign_native_jim_dictionary_sugar(
        &mut self,
        original: *mut TclObj,
        value: *mut TclObj,
    ) -> Result<(), Code> {
        let arguments = OriginalArguments::enter(self, &[original, value])
            .map_err(|error| self.report_cmd_error(error))?;
        let reached = (|| {
            let context = self
                .native_jim_object_context()
                .map_err(|error| self.report_cmd_error(error.into()))?;
            let check = || {
                arguments.check().map_err(|_| {
                    ValueError::CommandProtocolUnavailable("entered original Jim dictionary tuple")
                })
            };
            let converted = native_substitution::ensure_dictionary_substitution_with_stage_check(
                original,
                &context,
                Some(&check),
            );
            arguments
                .check()
                .map_err(|error| self.report_cmd_error(error))?;
            converted.map_err(|error| self.report_cmd_error(error.into()))?;
            let (name, key) =
                native_substitution::with_dictionary_substitution(original, |name, key| {
                    (name, key)
                })
                .ok_or_else(|| {
                    self.report_cmd_error(
                        ValueError::CommandProtocolUnavailable("original Jim dictionary tuple")
                            .into(),
                    )
                })?;
            let members = OriginalArguments::enter(self, &[name, key])
                .map_err(|error| self.report_cmd_error(error))?;
            let outcome = self.mutate_original_jim_dictionary_path(
                name,
                &[key],
                Some(value),
                tcl_registry::native_dictionary::NativeDictionaryRootPurpose::VariableSet,
                &arguments,
            );
            drop(members);
            arguments
                .check()
                .map_err(|error| self.report_cmd_error(error))?;
            match outcome {
                Ok(Some(_)) => Ok(()),
                Ok(None) => Err(self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable(
                        "original Jim dictionary set publication",
                    )
                    .into(),
                )),
                Err(JimDictionaryPathFailure::Command(error))
                    if error.native_execution_refusal().is_none() =>
                {
                    let bytes = arguments
                        .bytes(0)
                        .map_err(|error| self.report_cmd_error(error))?;
                    Err(crate::builtins::var_error(
                        self,
                        &bytes,
                        crate::frame::VarError::IsScalar,
                    ))
                }
                Err(error) => Err(error.report(self)),
            }
        })();
        arguments
            .check()
            .map_err(|error| self.report_cmd_error(error))?;
        reached
    }

    pub(super) fn unset_native_jim_dictionary_sugar(
        &mut self,
        original: *mut TclObj,
    ) -> Result<bool, Code> {
        let arguments = OriginalArguments::enter(self, &[original])
            .map_err(|error| self.report_cmd_error(error))?;
        let context = self
            .native_jim_object_context()
            .map_err(|error| self.report_cmd_error(error.into()))?;
        let check = || {
            arguments.check().map_err(|_| {
                ValueError::CommandProtocolUnavailable(
                    "entered original Jim dictionary unset tuple",
                )
            })
        };
        let converted = native_substitution::ensure_dictionary_substitution_with_stage_check(
            original,
            &context,
            Some(&check),
        );
        arguments
            .check()
            .map_err(|error| self.report_cmd_error(error))?;
        converted.map_err(|error| self.report_cmd_error(error.into()))?;
        let (name, key) =
            native_substitution::with_dictionary_substitution(original, |name, key| (name, key))
                .ok_or_else(|| {
                    self.report_cmd_error(
                        ValueError::CommandProtocolUnavailable(
                            "original Jim dictionary unset tuple",
                        )
                        .into(),
                    )
                })?;
        let members = OriginalArguments::enter(self, &[name, key])
            .map_err(|error| self.report_cmd_error(error))?;
        let outcome = self.mutate_original_jim_dictionary_path(
            name,
            &[key],
            None,
            tcl_registry::native_dictionary::NativeDictionaryRootPurpose::ExistingVariableUnset,
            &arguments,
        );
        members
            .check()
            .map_err(|error| self.report_cmd_error(error))?;
        match outcome {
            Ok(Some(_)) => {
                drop(members);
                arguments
                    .check()
                    .map_err(|error| self.report_cmd_error(error))?;
                self.set_result_bytes(b"");
                arguments
                    .check()
                    .map_err(|error| self.report_cmd_error(error))?;
                return Ok(true);
            }
            Err(JimDictionaryPathFailure::Command(error))
                if error.native_execution_refusal().is_none() => {}
            Err(JimDictionaryPathFailure::MissingKey(_)) => {}
            Err(error) => return Err(error.report(self)),
            Ok(None) => {}
        }
        // Native retries the SAME retained parent with JIM_NONE after failure.
        let parent_exists = self
            .read_original_named_variable_for_update(name)?
            .is_some();
        use tcl_syntax::value::ValueOps;
        let bytes = self
            .native_string_bytes(&original)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        let recipe = self
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .expect("selected Jim original unset");
        let message = recipe
            .dictionary_unset_error(&bytes, parent_exists)
            .map_err(|_| {
                self.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "Jim original unset diagnostic",
                    )
                    .into(),
                )
            })?;
        let code = self.set_error(&message);
        drop(members);
        arguments
            .check()
            .map_err(|error| self.report_cmd_error(error))?;
        Err(code)
    }

    pub(crate) fn increment_native_jim_original(
        &mut self,
        original: *mut TclObj,
        amount: tcl_cmd_core::native_increment::PreparedLegacyIncrementAmount,
    ) -> Code {
        let current = match self.read_original_named_variable_for_update(original) {
            Ok(current) => current,
            Err(code) => return code,
        };
        let objects = match crate::value_ops::RuntimeLegacyIncrementObjects::selected(self) {
            Ok(objects) => objects,
            Err(error) => return self.report_cmd_error(error),
        };
        let original_value = current.map(crate::value_ops::RuntimeAppendValue::borrowed);
        let sum = match tcl_cmd_core::native_increment::increment_legacy(
            &objects,
            original_value.as_ref(),
            amount,
        ) {
            Ok(sum) => sum,
            Err(error) => return self.report_cmd_error(error),
        };
        if let Err(code) = self.assign_original_named_variable(original, sum.as_ptr()) {
            return code;
        }
        self.set_result(sum.as_ptr());
        Code::Ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write;

    #[test]
    fn original_dictionary_unset_matches_nine_native_jim_controls() {
        fn decode(hex: &str) -> Vec<u8> {
            hex.as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect()
        }
        let mut compared = 0;
        for row in
            include_str!("../../../../rust/tcl-syntax/tests/data/native_jim_unset/windows.tsv")
                .lines()
        {
            let fields = row.split('\t').collect::<Vec<_>>();
            let mut interp = Interp::with_native_core(
                super::super::default_host(),
                crate::environment::profile_for_dialect("jim"),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            assert_eq!(
                interp.eval_str(&decode(fields[1])).as_int(),
                fields[2].parse::<i64>().unwrap(),
                "{}",
                fields[0]
            );
            assert_eq!(interp.result_bytes(), decode(fields[3]), "{}", fields[0]);
            assert!(!interp.host_refusal_pending(), "{}", fields[0]);
            compared += 1;
        }
        assert_eq!(compared, 9);
    }
    fn kind(value: *mut TclObj) -> String {
        let descriptor = obj::obj_type_ptr(value);
        if descriptor.is_null() {
            return "NULL".into();
        }
        // SAFETY: actual descriptor names are static while the header lives.
        unsafe { std::ffi::CStr::from_ptr((*descriptor).name) }
            .to_str()
            .unwrap()
            .into()
    }
    fn refs(value: *mut TclObj) -> obj::TclSize {
        // SAFETY: test only observes objects held by actual variable/alias/result owners.
        unsafe { (*value).ref_count }
    }
    #[test]
    fn original_dictionary_sugar_increment_matches_native_publication_windows() {
        let mut observed = String::new();
        for case in 0..7 {
            let mut interp = Interp::new();
            interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
            let key = Owned::fresh(obj::new_string_bytes(b"k"));
            let member = Owned::fresh(obj::new_string_bytes(if case == 2 || case == 3 {
                b"BAD"
            } else {
                b"4"
            }));
            let member_ptr = member.as_ptr();
            let root = Owned::fresh(
                crate::dict::new_dict_obj_native(
                    &[(key.as_ptr(), member_ptr)],
                    None,
                    NativeStringProtocol::Jim084,
                )
                .unwrap(),
            );
            let root_ptr = root.as_ptr();
            interp.var_set(b"d", root_ptr).unwrap();
            let shared = matches!(case, 1 | 3 | 5 | 6);
            let original_root = if shared {
                Some(root)
            } else {
                drop(root);
                None
            };
            let original_member = if case == 4 {
                Some(member)
            } else {
                drop(member);
                None
            };
            drop(key);
            let argv = [
                Owned::fresh(obj::new_string_bytes(b"incr")),
                Owned::fresh(obj::new_string_bytes(if case == 5 {
                    b"d(missing)"
                } else {
                    b"d(k)"
                })),
                Owned::fresh(if case == 6 {
                    obj::new_string_bytes(b"BAD")
                } else {
                    obj::new_wide_int_obj(1)
                }),
            ];
            writeln!(
                observed,
                "BEFORE\t{case}\t{}\t{}\t{}\t{}\t{}\t{}",
                refs(root_ptr),
                kind(root_ptr),
                usize::from(obj::has_string_rep(root_ptr)),
                refs(member_ptr),
                kind(member_ptr),
                usize::from(obj::has_string_rep(member_ptr))
            )
            .unwrap();
            let code = interp.dispatch(&argv.iter().map(Owned::as_ptr).collect::<Vec<_>>());
            let current_root = interp.var_get(b"d").unwrap();
            let current =
                crate::dict::dict_get(current_root, if case == 5 { b"missing" } else { b"k" })
                    .unwrap()
                    .unwrap();
            writeln!(
                observed,
                "AFTER\t{case}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                code.as_int(),
                usize::from(root_ptr == current_root),
                kind(current_root),
                refs(current_root),
                usize::from(member_ptr == current),
                kind(current),
                refs(current),
                usize::from(obj::has_string_rep(current))
            )
            .unwrap();
            if shared {
                writeln!(
                    observed,
                    "OLD\t{case}\t{}\t{}\t{}",
                    refs(member_ptr),
                    kind(member_ptr),
                    usize::from(obj::has_string_rep(member_ptr))
                )
                .unwrap();
            }
            drop(original_root);
            drop(original_member);
        }
        let expected = include_str!(
            "../../../../rust/tcl-syntax/testdata/native_jim_sugar_increment/observations.tsv"
        );
        assert_eq!(expected.lines().count(), 18);
        assert_eq!(observed, expected);
    }
}

#[cfg(test)]
#[path = "native_jim_increment/selected_dictionary_root_tests.rs"]
mod selected_dictionary_root_tests;
