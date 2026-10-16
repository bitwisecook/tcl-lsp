// SPDX-License-Identifier: AGPL-3.0-or-later
//! Retained original Jim namespace objects and helper activation.

use super::{Code, Interp, TclObj};
use crate::obj::{self, Owned};
use tcl_syntax::naming::{NativeJimNamespaceConstruction, NativeNameContext};
use tcl_syntax::value::{ValueError, ValueOps};

pub(crate) struct OriginalJimNamespaceObject {
    allocation: obj::NativeObjectLifetime,
    fresh: bool,
}

impl OriginalJimNamespaceObject {
    fn new(original: *mut TclObj, fresh: bool) -> Self {
        Self {
            allocation: obj::NativeObjectLifetime::retain(original),
            fresh,
        }
    }

    pub(crate) fn checked_ptr(&self) -> Result<*mut TclObj, ValueError> {
        self.allocation.checked_ptr()
    }
}

impl Drop for OriginalJimNamespaceObject {
    fn drop(&mut self) {
        if self.fresh {
            if let Ok(original) = self.allocation.checked_ptr() {
                // SAFETY: the allocation lease pins this unpublished zero-reference header.
                if unsafe { (*original).ref_count == 0 } {
                    // SAFETY: no native owner remains on this private constructed result.
                    unsafe { obj::free_obj(original) };
                }
            }
        }
    }
}

impl Interp {
    /// Materialize this actual Jim object context only at a reached namespace consumer.
    pub(crate) fn materialize_jim_namespace(
        &mut self,
        namespace: crate::namespace::NsId,
    ) -> Result<std::rc::Rc<[u8]>, ValueError> {
        self.materialize_jim_namespace_using(namespace, &|| Ok(()))
    }

    /// The same actual namespace holder with a caller check at every reached getter stage.
    pub(crate) fn materialize_jim_namespace_using<E: From<ValueError>>(
        &mut self,
        namespace: crate::namespace::NsId,
        check: &impl Fn() -> Result<(), E>,
    ) -> Result<std::rc::Rc<[u8]>, E> {
        check()?;
        let scope = super::native_operation_currency::NativeOperationScope::enter(self).map_err(
            |first| {
                self.refuse_native_execution(first);
                ValueError::CommandProtocolUnavailable("Jim namespace operation entry")
            },
        )?;
        let _context = self.native_jim_object_context()?;
        let original = self
            .namespaces()
            .jim_namespace_original_owner(namespace)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "Jim original namespace context",
            ))?;
        let original_birth = std::rc::Rc::downgrade(&original);
        let allocation = obj::NativeObjectLifetime::retain(original.as_ptr());
        drop(original);
        let caller_namespace = self.current_ns();
        let caller_frame = super::native_arguments::OriginalArgumentFrame::capture(self);
        let actual = self.clone();
        let current = || -> Result<(), E> {
            check()?;
            scope
                .currency()
                .ensure_current_or_refuse()
                .map_err(|first| {
                    actual.clone().refuse_native_execution(first);
                    ValueError::CommandProtocolUnavailable("Jim namespace operation currency")
                })?;
            allocation.checked_ptr()?;
            if actual.current_ns() != caller_namespace
                || caller_frame.check_frame(&actual).is_err()
                || !actual
                    .namespaces()
                    .jim_namespace_original_owner(namespace)
                    .is_some_and(|owner| {
                        std::rc::Weak::ptr_eq(&original_birth, &std::rc::Rc::downgrade(&owner))
                    })
            {
                let error =
                    ValueError::CommandProtocolUnavailable("Jim namespace getter frame owner");
                actual.clone().refuse_native_execution(
                    error
                        .native_access_refusal()
                        .expect("native refusal")
                        .into(),
                );
                return Err(error.into());
            }
            Ok(())
        };
        current()?;
        let reached =
            self.native_object_string_bytes_using(allocation.checked_ptr()?, &current, &E::from);
        current()?;
        let bytes = reached?;
        crate::namespace::Namespaces::record_jim_namespace_materialized(
            &mut self.namespaces_mut(),
            namespace,
            allocation.checked_ptr()?,
            std::rc::Rc::clone(&bytes),
        )?;
        current()?;
        Ok(bytes)
    }

    pub(crate) fn jim_current_namespace_object(&self) -> Result<Owned, ValueError> {
        if self.current_ns() == crate::namespace::GLOBAL {
            let _context = self.native_jim_object_context()?;
        }
        self.namespaces()
            .jim_namespace_object(self.current_ns())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "Jim retained namespace object",
            ))
    }

    pub(crate) fn jim_canonical_namespace_object(
        &mut self,
        namespace: &Owned,
        original: *mut TclObj,
    ) -> Result<Owned, ValueError> {
        let result =
            self.jim_canonical_namespace_object_using(namespace.as_ptr(), original, &|_| {
                Ok::<(), ValueError>(())
            })?;
        Ok(Owned::retain(result.checked_ptr()?))
    }

    pub(crate) fn jim_canonical_namespace_object_using<E: From<ValueError>>(
        &mut self,
        namespace: *mut TclObj,
        original: *mut TclObj,
        outer: &impl Fn(&Interp) -> Result<(), E>,
    ) -> Result<OriginalJimNamespaceObject, E> {
        outer(self)?;
        let protocol = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "Jim namespace canonical issuer",
            ))?
            .recipe();
        let scope = super::native_operation_currency::NativeOperationScope::enter(self).map_err(
            |first| {
                self.refuse_native_execution(first);
                ValueError::CommandProtocolUnavailable("Jim canonical namespace entry")
            },
        )?;
        obj::check_native_liveness(original)?;
        obj::check_native_liveness(namespace)?;
        let original_allocation = obj::NativeObjectLifetime::retain(original);
        let namespace_allocation = obj::NativeObjectLifetime::retain(namespace);
        let current = self.current_ns();
        let frame = super::native_arguments::OriginalArgumentFrame::capture(self);
        let check = |actual: &Interp| -> Result<(), E> {
            outer(actual)?;
            scope.currency().ensure_current_or_refuse().map_err(|_| {
                ValueError::CommandProtocolUnavailable("Jim canonical namespace currency")
            })?;
            let checked = (|| -> Result<(), ValueError> {
                frame.check_frame(actual)?;
                if actual.current_ns() != current {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "Jim canonical namespace caller",
                    ));
                }
                original_allocation.checked_ptr()?;
                namespace_allocation.checked_ptr()?;
                Ok(())
            })();
            if let Err(error) = &checked {
                if let Some(first) = error.native_access_refusal() {
                    actual.clone().refuse_native_execution(first.into());
                }
            }
            checked.map_err(E::from)
        };
        let actual = self.clone();
        let original_bytes =
            self.native_object_string_bytes_using(original, &|| check(&actual), &E::from)?;
        check(self)?;
        if let Some(absolute) = protocol
            .jim_namespace_absolute_canonical_input(&original_bytes)
            .map_err(|_| {
                ValueError::CommandProtocolUnavailable("Jim absolute namespace canonical name")
            })?
        {
            return Ok(OriginalJimNamespaceObject::new(
                obj::new_string_bytes(absolute.selected()),
                true,
            ));
        }
        let is_current = self
            .namespaces()
            .jim_namespace_original_owner(current)
            .is_some_and(|holder| holder.as_ptr() == namespace);
        let namespace_bytes = if is_current {
            self.materialize_jim_namespace_using(current, &|| check(&actual))?
        } else {
            self.native_object_string_bytes_using(namespace, &|| check(&actual), &E::from)?
        };
        check(self)?;
        let path = tcl_core_types::ByteNamespacePath::root();
        let context = NativeNameContext::with_jim_namespace(&path, &namespace_bytes);
        let construction = protocol
            .jim_namespace_construction(context, &original_bytes)
            .map_err(|_| {
                ValueError::CommandProtocolUnavailable("Jim namespace canonical construction")
            })?;
        let result = match construction {
            NativeJimNamespaceConstruction::RetainOriginal => {
                OriginalJimNamespaceObject::new(original, false)
            }
            NativeJimNamespaceConstruction::FreshString => {
                let projection = protocol
                    .jim_namespace_canonical_input(context, &original_bytes)
                    .map_err(|_| {
                        ValueError::CommandProtocolUnavailable("Jim namespace canonical name")
                    })?;
                OriginalJimNamespaceObject::new(obj::new_string_bytes(projection.selected()), true)
            }
            NativeJimNamespaceConstruction::DuplicateNamespaceAndAppend => {
                let dialect = self.native_invocation_dialect();
                let append = dialect.native_object_append_protocol(None).ok_or(
                    ValueError::CommandProtocolUnavailable("Jim namespace object append"),
                )?;
                let operations = crate::value_ops::RuntimeAppendObjects {
                    dialect,
                    binary_recipe: dialect.byte_array_string_recipe(None),
                };
                let duplicate = obj::duplicate_with_stage_check(
                    namespace,
                    &mut || check(&actual),
                    &mut |_| {},
                )?;
                let result = OriginalJimNamespaceObject::new(duplicate, true);
                let receiver = crate::value_ops::RuntimeAppendValue::borrowed(duplicate);
                let source = crate::value_ops::RuntimeAppendValue::borrowed(original);
                let mut first = None;
                let mut private_current = || -> Result<(), ValueError> {
                    if let Err(error) = check(&actual) {
                        first.get_or_insert(error);
                        return Err(ValueError::CommandProtocolUnavailable(
                            "Jim original canonical append",
                        ));
                    }
                    result.checked_ptr()?;
                    // SAFETY: the allocation lease pins this live private receiver.
                    if unsafe { (*duplicate).ref_count != 0 } {
                        return Err(ValueError::CommandProtocolUnavailable(
                            "Jim private canonical receiver",
                        ));
                    }
                    Ok(())
                };
                let appended = tcl_cmd_core::native_append::append_unshared_jim_cstring(
                    &operations,
                    append.recipe(),
                    &receiver,
                    b"::",
                    &mut private_current,
                )
                .and_then(|()| {
                    tcl_cmd_core::native_append::append_unshared_jim_object_checked(
                        &operations,
                        append.recipe(),
                        &receiver,
                        &source,
                        &mut private_current,
                    )
                });
                if let Some(error) = first {
                    return Err(error);
                }
                appended?;
                check(self)?;
                result
            }
        };
        check(self)?;
        Ok(result)
    }

    pub(crate) fn original_jim_namespace_text_object(
        &mut self,
        original: *mut TclObj,
        tail: bool,
        ordinal: usize,
        arguments: &super::native_arguments::OriginalArguments,
    ) -> Result<*mut TclObj, tcl_cmd_core::CmdError> {
        let context = self.native_jim_object_context()?;
        let namespace = self.current_ns();
        let original_namespace =
            super::native_jim_lookup::OriginalJimLookupNamespace::capture(self, namespace)?;
        let check = |actual: &Interp| original_namespace.check_current_arguments(actual, arguments);
        check(self)?;
        arguments.check_original_for(self, ordinal, original)?;
        let bytes = arguments.bytes_using(ordinal, &|| check(self));
        check(self)?;
        let bytes = bytes?;
        let protocol = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "Jim namespace text issuer",
            ))?
            .recipe();
        use tcl_syntax::naming::NativeNamespaceTextResult;
        check(self)?;
        let result = match protocol.namespace_text_result(&bytes, tail) {
            NativeNamespaceTextResult::SharedEmpty => context.empty_object().as_ptr(),
            NativeNamespaceTextResult::Original => original,
            NativeNamespaceTextResult::Counted(bytes) => obj::new_string_bytes(bytes),
            NativeNamespaceTextResult::Append(_) => {
                return Err(ValueError::CommandProtocolUnavailable(
                    "C append outside Jim namespace text producer",
                )
                .into());
            }
        };
        Ok(result)
    }

    pub(crate) fn original_jim_namespace_text(
        &mut self,
        original: *mut TclObj,
        tail: bool,
        arguments: &super::native_arguments::OriginalArguments,
    ) -> Code {
        let reached = (|| -> Result<Code, tcl_cmd_core::CmdError> {
            let _context = self.native_jim_object_context()?;
            let namespace = self.current_ns();
            let original_namespace =
                super::native_jim_lookup::OriginalJimLookupNamespace::capture(self, namespace)?;
            let result = self.original_jim_namespace_text_object(original, tail, 2, arguments)?;
            self.set_result(result);
            original_namespace.check_current_arguments(self, arguments)?;
            Ok(Code::Ok)
        })();
        match reached {
            Ok(code) => code,
            Err(error) => self.report_cmd_error(arguments.retain_refusal(error)),
        }
    }

    /// Keep original Jim parent getters, literal cache and result publication stages.
    pub(crate) fn original_jim_namespace_parent(
        &mut self,
        argv: &[*mut TclObj],
        arguments: &super::native_arguments::OriginalArguments,
    ) -> Code {
        let reached = (|| -> Result<Code, tcl_cmd_core::CmdError> {
            let _context = self.native_jim_object_context()?;
            let namespace = self.current_ns();
            let original_namespace =
                super::native_jim_lookup::OriginalJimLookupNamespace::capture(self, namespace)?;
            let check =
                |actual: &Interp| original_namespace.check_current_arguments(actual, arguments);
            check(self)?;
            let original = argv
                .get(2)
                .copied()
                .unwrap_or_else(|| original_namespace.original_ptr());
            let actual = self.clone();
            let current = || check(&actual);
            if argv.len() != 2 {
                arguments.check_original_for(self, 2, original)?;
            }
            let bytes = if argv.len() == 2 {
                self.materialize_jim_namespace_using(namespace, &current)
            } else {
                arguments.bytes_using(2, &current)
            };
            check(self)?;
            let bytes = bytes?;
            if !bytes.is_empty() {
                let literal = self
                    .native_invocation_dialect()
                    .native_jim_namespace_parent_root_comparison_table()
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "Jim parent root comparison declaration",
                    ))?;
                let comparison =
                    self.native_jim_compare_immediate_checked(original, &literal, 0, &current);
                check(self)?;
                if comparison? {
                    return Ok(Code::Ok);
                }
            }
            let protocol = self
                .name_policy_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "Jim parent text issuer",
                ))?
                .recipe();
            let result = protocol.jim_namespace_parent_result(&bytes).map_err(|_| {
                ValueError::CommandProtocolUnavailable("Jim parent text result purpose")
            })?;
            self.publish_original_jim_parent_result(result, &check)?;
            check(self)?;
            Ok(Code::Ok)
        })();
        match reached {
            Ok(code) => code,
            Err(error) => self.report_cmd_error(arguments.retain_refusal(error)),
        }
    }

    fn publish_original_jim_parent_result(
        &mut self,
        result: tcl_syntax::naming::NativeJimNamespaceParentResult<'_>,
        check: &impl Fn(&Interp) -> Result<(), tcl_cmd_core::CmdError>,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        use tcl_syntax::naming::NativeJimNamespaceParentResult;
        match result {
            NativeJimNamespaceParentResult::Unchanged => {}
            NativeJimNamespaceParentResult::CountedQualifier(bytes) => self.set_result_bytes(bytes),
            NativeJimNamespaceParentResult::RootedQualifier(bytes) => {
                self.append_original_jim_parent_qualifier(Some(bytes), check)?;
            }
            NativeJimNamespaceParentResult::RootedSharedEmpty => {
                self.append_original_jim_parent_qualifier(None, check)?;
            }
        }
        check(self)
    }

    fn append_original_jim_parent_qualifier(
        &mut self,
        fresh_bytes: Option<&[u8]>,
        check: &impl Fn(&Interp) -> Result<(), tcl_cmd_core::CmdError>,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        let context = self.native_jim_object_context()?;
        let qualifier =
            fresh_bytes.map_or_else(|| context.empty_object().as_ptr(), obj::new_string_bytes);
        let allocation = obj::NativeObjectLifetime::retain(qualifier);
        let reached = (|| {
            let dialect = self.native_invocation_dialect();
            let append = dialect.native_object_append_protocol(None).ok_or(
                ValueError::CommandProtocolUnavailable("Jim parent qualifier append"),
            )?;
            let operations = crate::value_ops::RuntimeAppendObjects {
                dialect,
                binary_recipe: None,
            };
            self.set_result_bytes(b"::");
            check(self)?;
            let published = self.result_obj();
            let receiver = crate::value_ops::RuntimeAppendValue::borrowed(published);
            let source = crate::value_ops::RuntimeAppendValue::borrowed(allocation.checked_ptr()?);
            let result_lifetime = obj::NativeObjectLifetime::retain(published);
            let actual = self.clone();
            let mut first = None;
            let appended =
                tcl_cmd_core::native_append::append_published_unshared_jim_object_checked(
                    &operations,
                    append.recipe(),
                    &receiver,
                    &source,
                    &mut || {
                        check(&actual).map_err(|error| {
                            first.get_or_insert(error);
                            ValueError::CommandProtocolUnavailable("Jim parent original operation")
                        })?;
                        allocation.checked_ptr()?;
                        result_lifetime.checked_ptr()?;
                        if actual.result_obj() != published
                        || obj::is_shared(published)
                        // SAFETY: the checked result lease pins this live header.
                        || unsafe { (*published).ref_count != 1 }
                        {
                            return Err(ValueError::CommandProtocolUnavailable(
                                "Jim parent published append receiver",
                            ));
                        }
                        Ok(())
                    },
                );
            if let Some(error) = first {
                return Err(error);
            }
            appended?;
            check(self)?;
            // Jim retires the actual qualifier only after appending it.
            let qualifier = allocation.checked_ptr()?;
            // SAFETY: the checked original source is live; the pair balances
            // exactly Jim's post-append temporary reference.
            unsafe {
                obj::incr_ref_count(qualifier);
                obj::decr_ref_count(qualifier);
            }
            Ok(())
        })();
        if reached.is_err() && fresh_bytes.is_some() && obj::allocation_is_live(qualifier)
            // SAFETY: the memory-only lease pins this fresh source header.
            && unsafe { (*qualifier).ref_count == 0 }
        {
            // SAFETY: refused unpublished source has no native owner to release.
            unsafe { obj::free_obj(qualifier) };
        }
        check(self)?;
        reached
    }

    #[cfg(test)]
    pub(crate) fn ns_eval_objects(&mut self, name: *mut TclObj, body: *mut TclObj) -> Code {
        self.ns_eval_objects_with_purpose(
            name,
            body,
            tcl_registry::native_eval_object::EvalObjectPurpose::NamespaceBody,
        )
    }

    #[cfg(test)]
    pub(crate) fn ns_eval_objects_with_purpose(
        &mut self,
        name: *mut TclObj,
        body: *mut TclObj,
        purpose: tcl_registry::native_eval_object::EvalObjectPurpose,
    ) -> Code {
        self.ns_eval_objects_with_arguments(name, body, purpose, None)
    }

    /// Evaluate the original body while the caller owns the original argument
    /// headers throughout the namespace activation.
    pub(crate) fn ns_eval_objects_with_arguments(
        &mut self,
        name: *mut TclObj,
        body: *mut TclObj,
        purpose: tcl_registry::native_eval_object::EvalObjectPurpose,
        original_arguments: Option<&[*mut TclObj]>,
    ) -> Code {
        let body = Owned::retain(body);
        let location = self.arg_loc(body.as_ptr());
        if let Err(first) = self.clear_return_options_checked() {
            return self.refuse_native_execution(first);
        }
        if self
            .name_policy_protocol()
            .is_some_and(|protocol| protocol.recipe().is_jim084())
        {
            let namespace = match self.jim_current_namespace_object() {
                Ok(namespace) => namespace,
                Err(error) => return self.report_cmd_error(error.into()),
            };
            let canonical = match self.jim_canonical_namespace_object(&namespace, name) {
                Ok(canonical) => canonical,
                Err(error) => return self.report_cmd_error(error.into()),
            };
            let bytes = match self.native_string_bytes(&canonical.as_ptr()) {
                Ok(bytes) => bytes,
                Err(error) => return self.report_cmd_error(error.into()),
            };
            let target = self.namespaces_mut().retain_jim_namespace(canonical, bytes);
            self.ns_eval_in_token(
                target,
                &[],
                location,
                true,
                Some((purpose, body.as_ptr())),
                original_arguments,
            )
        } else {
            match self.native_namespace_object_lookup(name) {
                Ok(Some(target)) => {
                    return self.ns_eval_in_token(
                        target,
                        &[],
                        location,
                        true,
                        Some((purpose, body.as_ptr())),
                        original_arguments,
                    );
                }
                Ok(None) => {}
                Err(error) => return self.report_cmd_error(error.into()),
            }
            let name_bytes = match self.native_string_bytes(&name) {
                Ok(bytes) => bytes,
                Err(error) => return self.report_cmd_error(error.into()),
            };
            self.ns_eval_framed(
                &name_bytes,
                &[],
                location,
                true,
                Some((purpose, body.as_ptr())),
                original_arguments,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    thread_local! {
        static CANONICAL_NAMESPACE: std::cell::Cell<*mut TclObj> = const {
            std::cell::Cell::new(std::ptr::null_mut())
        };
        static CANONICAL_NAME_OBSERVATIONS: std::cell::RefCell<Vec<bool>> = const {
            std::cell::RefCell::new(Vec::new())
        };
    }

    extern "C" fn update_canonical_probe_name(value: *mut TclObj) {
        let resident = CANONICAL_NAMESPACE.with(|namespace| obj::has_string_rep(namespace.get()));
        CANONICAL_NAME_OBSERVATIONS.with(|observations| observations.borrow_mut().push(resident));
        let bytes: &[u8] = if obj::internal_rep(value) == 0 {
            b"::target\0tail"
        } else {
            b"child\0tail"
        };
        // SAFETY: the actual updater owns the String slot of this live original object.
        unsafe { obj::set_string_rep(value, bytes) };
    }

    static CANONICAL_PROBE_NAME: obj::TclObjType = obj::TclObjType {
        name: c"canonical-name-probe".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(update_canonical_probe_name),
        set_from_any_proc: None,
    };

    #[test]
    fn original_canonical_name_getter_precedes_relative_namespace_and_skips_absolute_namespace() {
        let evidence = include_str!(
            "../../tests/data/native_jim_namespace_canonical/getter-order-20261015.tsv"
        );
        let row = |kind: &str, label: &str| {
            evidence
                .lines()
                .find(|line| {
                    let mut fields = line.split('|');
                    fields.next() == Some(kind) && fields.next() == Some(label)
                })
                .unwrap()
                .split('|')
                .collect::<Vec<_>>()
        };
        for (label, absolute, root) in [
            ("ABSOLUTE", true, false),
            ("RELATIVE", false, false),
            ("ROOT_RELATIVE", false, true),
        ] {
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect("jimtcl"),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let context = interp.native_jim_object_context().unwrap();
            let namespace = if root {
                Owned::fresh(obj::new_string_bytes(b""))
            } else {
                Owned::fresh(obj::new_wide_int_obj(42))
            };
            crate::native_source::bind_context(namespace.as_ptr(), &context).unwrap();
            let token = interp
                .namespaces_mut()
                .retain_unmaterialized_jim_namespace(Owned::retain(namespace.as_ptr()));
            let caller = interp.current_ns();
            interp.set_current_ns(token);
            let name = Owned::fresh(obj::alloc_typed(
                &CANONICAL_PROBE_NAME,
                u64::from(!absolute),
            ));
            crate::native_source::bind_context(name.as_ptr(), &context).unwrap();
            CANONICAL_NAMESPACE.with(|slot| slot.set(namespace.as_ptr()));
            CANONICAL_NAME_OBSERVATIONS.with(|observations| observations.borrow_mut().clear());
            let references = unsafe { (*namespace.as_ptr()).ref_count };
            let namespace_type = obj::obj_type_ptr(namespace.as_ptr());
            assert_eq!(
                obj::has_string_rep(namespace.as_ptr()),
                row("NAMESPACE_BEFORE", label)[2] == "1"
            );
            assert!(!obj::has_string_rep(name.as_ptr()));
            let result = interp
                .jim_canonical_namespace_object(&namespace, name.as_ptr())
                .unwrap();
            assert_eq!(
                CANONICAL_NAME_OBSERVATIONS.with(|observations| observations.borrow().clone()),
                vec![row("NAMESPACE_AT_NAME_GETTER", label)[2] == "1"]
            );
            assert_eq!(
                obj::has_string_rep(namespace.as_ptr()),
                row("NAMESPACE_AFTER", label)[2] == "1"
            );
            assert_eq!(obj::obj_type_ptr(namespace.as_ptr()), namespace_type);
            assert_eq!(
                obj::obj_type_ptr(name.as_ptr()),
                &CANONICAL_PROBE_NAME as *const obj::TclObjType
            );
            assert_eq!(unsafe { (*namespace.as_ptr()).ref_count }, references);
            assert_eq!(
                result.as_ptr() == name.as_ptr(),
                row("RESULT_IDENTITY", label)[3] == "31"
            );
            assert_eq!(
                interp.namespaces().native_context_path(token).is_some(),
                !absolute
            );
            let expected = row("RESULT", label)[3]
                .as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect::<Vec<_>>();
            assert_eq!(
                interp
                    .native_string_bytes(&result.as_ptr())
                    .unwrap()
                    .as_ref(),
                expected
            );
            assert!(interp.native_execution_refusal().is_none());
            interp.set_current_ns(caller);
            CANONICAL_NAMESPACE.with(|slot| slot.set(std::ptr::null_mut()));
        }
    }

    #[test]
    fn jim_top_namespace_owns_the_original_context_empty_object() {
        let mut interp = super::Interp::new();
        interp.set_dialect_profile(profile());
        let namespace = interp.jim_current_namespace_object().unwrap();
        let context = interp.native_jim_object_context().unwrap();
        assert_eq!(namespace.as_ptr(), context.empty_object().as_ptr());
        let written = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"n\0z"));
        let canonical = interp
            .jim_canonical_namespace_object(&namespace, written.as_ptr())
            .unwrap();
        assert_eq!(canonical.as_ptr(), written.as_ptr());
        assert_eq!(
            interp.jim_current_namespace_object().unwrap().as_ptr(),
            context.empty_object().as_ptr()
        );
    }

    use super::*;
    use tcl_runtime_api::{Namespaces, NsId};
    fn profile() -> &'static tcl_dialect::DialectProfile {
        Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        )))
    }

    #[test]
    fn jim_namespace_error_unwind_does_not_prepare_an_unused_c_namespace_display() {
        // Software frame control: the selected Jim error stack omits the C eval frame.
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("jimtcl"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let context = interp.native_jim_object_context().unwrap();
        let original = Owned::fresh(obj::new_wide_int_obj(42));
        let pointer = original.as_ptr();
        crate::native_source::bind_context(pointer, &context).unwrap();
        let original_type = obj::obj_type_ptr(pointer);
        let namespace = interp
            .namespaces_mut()
            .retain_unmaterialized_jim_namespace(original);
        let _namespace_owner = interp
            .namespaces_mut()
            .take_jim_namespace_owner(namespace)
            .unwrap();
        let body = Owned::fresh(obj::new_string_bytes(b"$missing"));
        let references = unsafe { (*pointer).ref_count };
        let caller = interp.current_ns();
        assert_eq!(
            interp.ns_eval_in_token(
                namespace,
                &[],
                None,
                true,
                Some((
                    tcl_registry::native_eval_object::EvalObjectPurpose::NamespaceBody,
                    body.as_ptr(),
                )),
                None,
            ),
            Code::Error
        );
        assert_eq!(interp.current_ns(), caller);
        assert!(obj::native_object_snapshot(pointer)
            .unwrap()
            .resident
            .is_none());
        assert_eq!(obj::obj_type_ptr(pointer), original_type);
        assert_eq!(unsafe { (*pointer).ref_count }, references);
        assert!(interp.namespaces().native_context_path(namespace).is_none());
        assert!(interp.native_execution_refusal().is_none());
        assert_eq!(
            interp.result_bytes(),
            b"can't read \"missing\": no such variable"
        );
    }

    #[test]
    fn namespace_body_dispatch_retains_original_list_members() {
        let mut interp = Interp::new();
        let namespace = Owned::fresh(obj::new_string_bytes(b"n"));
        let value = Owned::fresh(obj::new_string_bytes(b"v\0\xff"));
        let command = Owned::fresh(obj::new_string_bytes(b"return"));
        let body = Owned::fresh(crate::list::new_list_obj(&[
            command.as_ptr(),
            value.as_ptr(),
        ]));
        assert_eq!(
            interp.ns_eval_objects(namespace.as_ptr(), body.as_ptr()),
            Code::Return
        );
        assert_eq!(interp.result_obj(), value.as_ptr());
        assert_eq!(interp.current_ns(), crate::namespace::GLOBAL);
        assert!(obj::native_object_snapshot(body.as_ptr())
            .unwrap()
            .resident
            .is_none());
    }

    #[test]
    fn root_relative_canonicalisation_and_activation_retain_original_opaque_object() {
        let mut interp = Interp::new();
        interp.set_dialect_profile(profile());
        let original = Owned::fresh(obj::new_string_bytes(b"n\0z"));
        let root = interp.jim_current_namespace_object().unwrap();
        let canonical = interp
            .jim_canonical_namespace_object(&root, original.as_ptr())
            .unwrap();
        assert_eq!(canonical.as_ptr(), original.as_ptr());
        let body = Owned::fresh(obj::new_string_bytes(b"namespace canonical"));
        assert_eq!(
            interp.ns_eval_objects(original.as_ptr(), body.as_ptr()),
            Code::Ok
        );
        assert_eq!(interp.result_obj(), original.as_ptr());
        assert_eq!(interp.current_ns(), crate::namespace::GLOBAL);
        assert_eq!(obj::bytes_of(original.as_ptr()), b"n\0z");
    }

    #[test]
    fn canonical_construction_and_query_use_actual_jim_holder_without_c_tree() {
        let mut interp = Interp::new();
        interp.set_dialect_profile(profile());
        let original = Owned::fresh(obj::new_string_bytes(b"n\0z"));
        let token = interp
            .namespaces_mut()
            .retain_jim_namespace(original.clone(), std::rc::Rc::from(&b"n\0z"[..]));
        let relative = Owned::fresh(obj::new_string_bytes(b"p\xff"));
        let joined = interp
            .jim_canonical_namespace_object(&original, relative.as_ptr())
            .unwrap();
        assert_ne!(joined.as_ptr(), original.as_ptr());
        assert_ne!(joined.as_ptr(), relative.as_ptr());
        assert_eq!(
            interp
                .native_string_bytes(&joined.as_ptr())
                .unwrap()
                .as_ref(),
            b"n\0z::p\xff"
        );
        assert_eq!(
            interp
                .namespace_variable_name_bytes_checked(NsId(token as u32), b"absent")
                .unwrap(),
            Some(b"::n\0z::absent".to_vec())
        );
        let absolute = Owned::fresh(obj::new_string_bytes(b":::p\0q"));
        let selected = interp
            .jim_canonical_namespace_object(&original, absolute.as_ptr())
            .unwrap();
        assert_ne!(selected.as_ptr(), absolute.as_ptr());
        assert_eq!(
            interp
                .native_string_bytes(&selected.as_ptr())
                .unwrap()
                .as_ref(),
            b"p"
        );
        assert_eq!(interp.namespaces().qualified_name(token), b"::n\0z");
    }

    #[test]
    fn jim_root_command_context_requires_its_live_original_top_object() {
        // Implementation contract: naming.runtime.original-root-command-context
        // docs/design/analysis/name-resolution-proofs/original-root-command-context.md
        let mut interp = Interp::new();
        interp.set_dialect_profile(profile());
        let context = interp.native_jim_object_context().unwrap();
        let root = interp.jim_current_namespace_object().unwrap();
        assert_eq!(root.as_ptr(), context.empty_object().as_ptr());
        assert_eq!(
            interp.root_command_context_checked().unwrap(),
            Some(tcl_runtime_api::ROOT_NS)
        );
        assert!(interp
            .find_namespace_bytes_checked(tcl_runtime_api::ROOT_NS, b"::")
            .is_err());
        interp
            .namespaces_mut()
            .adopt_jim_root_namespace(Owned::fresh(obj::new_string_bytes(b"")));
        assert!(
            interp.root_command_context_checked().is_err(),
            "equal bytes cannot donate the original object owner"
        );
        interp.namespaces_mut().adopt_jim_root_namespace(root);
        assert_eq!(
            interp.root_command_context_checked().unwrap(),
            Some(tcl_runtime_api::ROOT_NS)
        );
        context.retire();
        assert!(
            interp.root_command_context_checked().is_err(),
            "retired interpreter cannot issue a context"
        );
    }

    #[test]
    fn jim_flat_command_context_and_helper_enumeration_match_native_controls() {
        // Native source question: naming.namespace.jim-flat-command-context-source-controls
        // docs/design/analysis/name-resolution-proofs/jim-flat-command-context-source-controls.md
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("jimtcl"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let script = br#"proc p {} {return ROOT}; namespace eval n {proc p {} {return INNER}; namespace eval child {proc q {} {return CHILD}}}; namespace eval D {namespace import ::n::p}; list [namespace canonical] [namespace eval n {namespace canonical}] [lsort [namespace eval n {info procs *}]] [namespace eval n {info commands p}] [lsort [namespace eval n {info procs ::n::*}]] [namespace eval n {namespace which -variable absent}] [namespace eval n {set value LOCAL; namespace eval child {set value CHILD}; set value}] [D::p] [namespace origin D::p]"#;
        assert_eq!(
            interp.eval_str(script),
            Code::Ok,
            "result={:?}, native refusal={:?}, admission={:?}",
            interp.result_bytes(),
            interp.native_access_refusal(),
            interp.native_compilation_admission_error()
        );
        assert_eq!(
            interp.result_bytes(),
            b"{} n {child::q p} p {::n::child::q ::n::p} ::n::absent LOCAL INNER ::n::p"
        );
    }
}
