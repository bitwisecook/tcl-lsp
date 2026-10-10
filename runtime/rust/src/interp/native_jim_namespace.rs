// SPDX-License-Identifier: AGPL-3.0-or-later
//! Retained original Jim namespace objects and helper activation.

use super::{Code, Interp, TclObj};
use crate::obj::{self, Owned};
use tcl_syntax::naming::{NativeJimNamespaceConstruction, NativeNameContext};
use tcl_syntax::value::{ValueError, ValueOps};

impl Interp {
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
        let protocol = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "Jim namespace canonical issuer",
            ))?
            .recipe();
        let namespace_bytes = self.native_string_bytes(&namespace.as_ptr())?;
        let original_bytes = self.native_string_bytes(&original)?;
        let path = tcl_core_types::ByteNamespacePath::root();
        let context = NativeNameContext::with_jim_namespace(&path, &namespace_bytes);
        let construction = protocol
            .jim_namespace_construction(context, &original_bytes)
            .map_err(|_| {
                ValueError::CommandProtocolUnavailable("Jim namespace canonical construction")
            })?;
        match construction {
            NativeJimNamespaceConstruction::RetainOriginal => Ok(Owned::retain(original)),
            NativeJimNamespaceConstruction::FreshString => {
                let projection = protocol
                    .jim_namespace_canonical_input(context, &original_bytes)
                    .map_err(|_| {
                        ValueError::CommandProtocolUnavailable("Jim namespace canonical name")
                    })?;
                Ok(Owned::fresh(obj::new_string_bytes(projection.selected())))
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
                let duplicate = Owned::fresh(obj::duplicate(namespace.as_ptr()));
                let separator = Owned::fresh(obj::new_string_bytes(b"::"));
                let receiver = crate::value_ops::RuntimeAppendValue::borrowed(duplicate.as_ptr());
                let separator = crate::value_ops::RuntimeAppendValue::borrowed(separator.as_ptr());
                let source = crate::value_ops::RuntimeAppendValue::borrowed(original);
                let mut prepared = tcl_cmd_core::native_append::append_object(
                    &operations,
                    append.recipe(),
                    Some(&receiver),
                    &separator,
                )?;
                tcl_cmd_core::native_append::append_continuation(
                    &operations,
                    &mut prepared,
                    &source,
                )?;
                Ok(Owned::retain(prepared.value().as_ptr()))
            }
        }
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
        self.clear_return_options();
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
        let mut interp = Interp::new();
        interp.set_dialect_profile(profile());
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
