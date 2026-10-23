// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original compiled List headers and actual variable-frame/temporary selection.
use super::*;
use crate::interp::native_arguments::OriginalArguments;
use tcl_cmd_core::CmdError;

pub(super) use crate::interp::native_arguments::OriginalArgumentFrame as BodyListFrame;

/// An allocation-only original header projection. The frame birth and optional
/// anonymous local are observations, not new native or variable owning roles.
pub(super) struct BodyListArguments {
    arguments: OriginalArguments,
    original: *mut TclObj,
    frame: BodyListFrame,
    temporary: Option<usize>,
}
impl BodyListArguments {
    pub(super) fn capture(
        interpreter: &Interp,
        original: *mut TclObj,
        temporary: Option<usize>,
    ) -> Result<Self, CmdError> {
        let arguments = OriginalArguments::enter(interpreter, &[original])?;
        let captured = Self {
            arguments,
            original,
            frame: BodyListFrame::capture(interpreter),
            temporary,
        };
        captured.check(interpreter)?;
        Ok(captured)
    }
    pub(super) fn check(&self, actual: &Interp) -> Result<(), CmdError> {
        self.arguments
            .check_original_for(actual, 0, self.original)?;
        self.check_selection(actual)?;
        Ok(())
    }
    pub(super) fn check_selection(&self, actual: &Interp) -> Result<(), CmdError> {
        self.frame.check(actual, &self.arguments)?;
        let same = self.temporary.is_none_or(|slot| {
            actual
                .frames
                .borrow()
                .native_compiled_temporary(slot)
                .ok()
                .flatten()
                == Some(self.original)
        });
        if !same {
            return Err(self
                .arguments
                .retain_refusal(unavailable("original compiled List frame or temporary").into()));
        }
        Ok(())
    }
    pub(super) fn refuse(&self, error: ValueError) -> CmdError {
        self.arguments.retain_refusal(error.into())
    }
    pub(super) fn members(
        &self,
        interpreter: &Interp,
        protocol: NativeStringProtocol,
    ) -> Result<Vec<*mut TclObj>, CmdError> {
        self.check(interpreter)?;
        crate::list::list_elements_native_using(
            self.original,
            protocol,
            &|| self.check(interpreter),
            &|error| self.refuse(error),
        )
    }
    pub(super) fn length(&self, interpreter: &mut Interp) -> Result<usize, CmdError> {
        crate::value_ops::list_length_using(
            interpreter,
            &self.original,
            &mut |actual, _original| self.check(actual),
            |actual, original| {
                let protocol = actual.native_invocation_dialect().native_string_materialization(
                    Some(tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation),
                ).ok_or_else(|| self.refuse(unavailable("native object list conversion")))?.protocol();
                if protocol.is_jim084() {
                    let context = actual
                        .native_jim_object_context()
                        .map_err(|error| self.refuse(error))?;
                    self.check(actual)?;
                    crate::native_source::bind_context(*original, &context)
                        .map_err(|error| self.refuse(error))?;
                    self.check(actual)?;
                }
                crate::list::list_elements_native_using(
                    *original,
                    protocol,
                    &|| self.check(actual),
                    &|error| self.refuse(error),
                )
            },
        )
    }
}

/// Earliest compiled frame and operation, retained through every reached
/// header, body and owned-result cleanup. Header views add allocation safety
/// only and never replace this original entered receipt.
pub(super) struct BodyObjectOperation {
    arguments: OriginalArguments,
    frame: BodyListFrame,
}
impl BodyObjectOperation {
    pub(super) fn capture(actual: &Interp) -> Result<Rc<Self>, CmdError> {
        let owner = Rc::new(Self {
            arguments: OriginalArguments::enter(actual, &[])?,
            frame: BodyListFrame::capture(actual),
        });
        owner.check(actual)?;
        Ok(owner)
    }
    pub(super) fn check(&self, actual: &Interp) -> Result<(), CmdError> {
        self.frame.check(actual, &self.arguments)
    }
    pub(super) fn retain(&self, error: CmdError) -> CmdError {
        self.arguments.retain_refusal(error)
    }
    pub(super) fn refuse(&self, error: ValueError) -> CmdError {
        self.retain(error.into())
    }
    pub(super) fn bytes(
        &self,
        actual: &Interp,
        original: *mut TclObj,
    ) -> Result<Rc<[u8]>, CmdError> {
        self.bytes_using(actual, original, &|| Ok(()))
    }
    pub(super) fn bytes_using(
        &self,
        actual: &Interp,
        original: *mut TclObj,
        extra: &dyn Fn() -> Result<(), CmdError>,
    ) -> Result<Rc<[u8]>, CmdError> {
        self.check(actual)?;
        extra()?;
        let header = OriginalArguments::enter(actual, &[original])?;
        actual.native_object_string_bytes_using(
            original,
            &|| {
                self.check(actual)?;
                extra()?;
                header.check_original_for(actual, 0, original)
            },
            &|error| self.refuse(error),
        )
    }
    /// Selected scalar primitive under the same earliest compiled operation.
    /// The compatibility ValueError is inspected only after the exact retained
    /// operation/frame/header cause, including a reached Guest failure.
    pub(super) fn scalar(
        &self,
        actual: &Interp,
        original: *mut TclObj,
        kind: tcl_syntax::scalar_getter::NativeScalarGetterKind,
    ) -> Result<tcl_syntax::scalar_getter::NativeScalarGetterValue, CmdError> {
        self.check(actual)?;
        let header = OriginalArguments::enter(actual, &[original])?;
        let current = || {
            self.check(actual)?;
            header.check_original_for(actual, 0, original)
        };
        current()?;
        let dialect = actual.native_invocation_dialect();
        dialect
            .native_scalar_getter_protocol()
            .ok_or_else(|| self.refuse(ValueError::ScalarNumericInputUnavailable))?;
        let host = actual.host();
        let environment = host.numeric_environment();
        current()?;
        let stage = || current().map_err(|_| ValueError::ScalarNumericInputUnavailable);
        let reached = crate::typed_value::native_scalar_getter_with_stage_check(
            original,
            dialect,
            kind,
            environment,
            Some(self.arguments.currency()),
            Some(&stage),
        )
        .map_err(|error| self.refuse(error));
        current()?;
        reached
    }
    pub(super) fn members(
        &self,
        actual: &Interp,
        original: *mut TclObj,
        protocol: NativeStringProtocol,
    ) -> Result<Vec<*mut TclObj>, CmdError> {
        self.check(actual)?;
        let header = OriginalArguments::enter(actual, &[original])?;
        crate::list::list_elements_native_using(
            original,
            protocol,
            &|| {
                self.check(actual)?;
                header.check_original_for(actual, 0, original)
            },
            &|error| self.refuse(error),
        )
    }
    pub(super) fn list(
        &self,
        actual: &Interp,
        original: *mut TclObj,
        protocol: NativeStringProtocol,
    ) -> Result<BodyListMembers, CmdError> {
        let members = self.members(actual, original, protocol)?;
        let header = OriginalArguments::enter(actual, &[original])?;
        let children = OriginalArguments::enter(actual, &members)?;
        let primary =
            obj::NativePrimaryReceipt::capture(original).map_err(|error| self.refuse(error))?;
        let backing = crate::list::native_list_backing(original)
            .ok_or_else(|| self.refuse(unavailable("original compiled List backing")))?;
        Ok(BodyListMembers {
            original,
            header,
            children,
            primary,
            backing,
            members,
        })
    }
    pub(super) fn pairs(
        &self,
        actual: &Interp,
        original: *mut TclObj,
        protocol: NativeStringProtocol,
    ) -> Result<BodyDictionaryPairs, CmdError> {
        self.check(actual)?;
        let header = OriginalArguments::enter(actual, &[original])?;
        let pairs = crate::dict::native_dict_pairs_using(
            original,
            protocol,
            &|| {
                self.check(actual)?;
                header.check_original_for(actual, 0, original)
            },
            &|error| self.refuse(error),
        )?;
        let backing = crate::dict::original_dictionary_backing(original)
            .map_err(|error| self.refuse(error))?;
        let pointers = pairs
            .iter()
            .flat_map(|(key, value)| [*key, *value])
            .collect::<Vec<_>>();
        let children = OriginalArguments::enter(actual, &pointers)?;
        Ok(BodyDictionaryPairs {
            original,
            header,
            children,
            backing,
            pairs,
        })
    }
    pub(super) fn get(
        &self,
        actual: &mut Interp,
        original: *mut TclObj,
        keys: &[*mut TclObj],
        protocol: NativeStringProtocol,
    ) -> Result<*mut TclObj, CmdError> {
        self.dictionary_lookup(
            actual,
            original,
            keys,
            protocol,
            tcl_registry::native_dictionary_compilation::NativeDictionaryLookupKind::Get,
            None,
        )
    }
    pub(super) fn dictionary_lookup(
        &self,
        actual: &mut Interp,
        original: *mut TclObj,
        keys: &[*mut TclObj],
        protocol: NativeStringProtocol,
        kind: tcl_registry::native_dictionary_compilation::NativeDictionaryLookupKind,
        default: Option<*mut TclObj>,
    ) -> Result<*mut TclObj, CmdError> {
        use tcl_registry::native_dictionary_compilation::NativeDictionaryLookupKind as Kind;
        let retained = std::cell::RefCell::new(Vec::<Rc<BodyDictionaryPairs>>::new());
        let check = |actual: &Interp| {
            self.check(actual)?;
            for view in retained.borrow().iter() {
                view.check(actual, self)?;
            }
            Ok::<(), CmdError>(())
        };
        let string = |actual: &mut Interp, key: &*mut TclObj| {
            check(actual)?;
            let bytes = self.bytes_using(actual, *key, &|| check(actual))?;
            check(actual)?;
            Ok(bytes)
        };
        let pairs = |actual: &mut Interp, value: &*mut TclObj| {
            check(actual)?;
            let view = Rc::new(self.pairs(actual, *value, protocol)?);
            retained.borrow_mut().push(view.clone());
            Ok(view)
        };
        let lookup = |actual: &mut Interp, view: &Rc<BodyDictionaryPairs>, key: &[u8]| {
            check(actual)?;
            let reached = view.lookup(actual, self, key);
            check(actual)?;
            reached
        };
        let reached = match kind {
            Kind::Get => tcl_cmd_core::dict::get_using(
                actual,
                &original,
                keys,
                string,
                pairs,
                lookup,
                |actual, key| match check(actual) {
                    Err(first) => first,
                    Ok(()) => tcl_cmd_core::dict::missing_key_error(
                        actual,
                        tcl_syntax::naming::NativeDictionaryMissingKeyOperation::Get,
                        key,
                    ),
                },
            ),
            Kind::Exists => {
                tcl_cmd_core::dict::exists_using(actual, &original, keys, string, pairs, lookup)
                    .map(|exists| obj::new_boolean_obj(i32::from(exists)))
            }
            Kind::GetDefault => {
                let default = default.ok_or_else(|| {
                    self.refuse(unavailable("original compiled dictionary default operand"))
                })?;
                tcl_cmd_core::dict::getdef_using(
                    actual, &original, keys, &default, string, pairs, lookup,
                )
            }
        };
        check(actual)?;
        reached
    }
    pub(super) fn objects(
        self: &Rc<Self>,
        actual: &Interp,
    ) -> Result<crate::value_ops::RuntimeDictionaryObjects, CmdError> {
        self.objects_using(actual, Rc::new(|| Ok(())))
    }
    pub(super) fn objects_using(
        self: &Rc<Self>,
        actual: &Interp,
        extra: Rc<dyn Fn() -> Result<(), CmdError>>,
    ) -> Result<crate::value_ops::RuntimeDictionaryObjects, CmdError> {
        self.check(actual)?;
        extra()?;
        let current = self.clone();
        let current_interp = actual.clone();
        let strings = self.clone();
        let strings_interp = actual.clone();
        let refuse = self.clone();
        let retain = self.clone();
        let current_extra = extra.clone();
        Ok(
            crate::value_ops::RuntimeDictionaryObjects::selected(actual)?.with_original_checks(
                move || {
                    current.check(&current_interp)?;
                    current_extra()
                },
                move |value, check| {
                    strings.bytes_using(&strings_interp, value, &|| {
                        extra()?;
                        check()
                    })
                },
                move |error| refuse.refuse(error),
                move |error| retain.arguments.retain_refusal(error),
            ),
        )
    }
}
pub(super) struct BodyListMembers {
    original: *mut TclObj,
    header: OriginalArguments,
    children: OriginalArguments,
    primary: obj::NativePrimaryReceipt,
    backing: crate::list::NativeListBacking,
    pub(super) members: Vec<*mut TclObj>,
}
impl BodyListMembers {
    pub(super) fn check(
        &self,
        actual: &Interp,
        operation: &BodyObjectOperation,
    ) -> Result<(), CmdError> {
        operation.check(actual)?;
        self.header.check_original_for(actual, 0, self.original)?;
        self.children.check_for(actual)?;
        self.primary
            .check_for(self.original, "original compiled List backing")
            .map_err(|error| operation.refuse(error))?;
        let current = self
            .backing
            .elements()
            .map_err(|error| operation.refuse(error))?;
        if current.as_ref() != self.members.as_slice() {
            return Err(operation.refuse(unavailable("original compiled List member vector")));
        }
        Ok(())
    }
}

pub(super) struct BodyDictionaryPairs {
    original: *mut TclObj,
    header: OriginalArguments,
    children: OriginalArguments,
    backing: Weak<()>,
    pub(super) pairs: Vec<(*mut TclObj, *mut TclObj)>,
}
impl BodyDictionaryPairs {
    pub(super) fn check(
        &self,
        actual: &Interp,
        operation: &BodyObjectOperation,
    ) -> Result<(), CmdError> {
        operation.check(actual)?;
        self.header.check_original_for(actual, 0, self.original)?;
        self.children.check_for(actual)?;
        crate::dict::original_dictionary_pairs_current(self.original, &self.backing, &self.pairs)
            .map_err(|error| operation.refuse(error))
    }
    pub(super) fn lookup(
        &self,
        actual: &Interp,
        operation: &BodyObjectOperation,
        bytes: &[u8],
    ) -> Result<Option<*mut TclObj>, CmdError> {
        tcl_cmd_core::dict::lookup_using(&self.pairs, bytes, |key| {
            self.check(actual, operation)?;
            let bytes = operation.bytes_using(actual, *key, &|| self.check(actual, operation))?;
            self.check(actual, operation)?;
            Ok(bytes)
        })
    }
}

#[cfg(test)]
mod tests;
