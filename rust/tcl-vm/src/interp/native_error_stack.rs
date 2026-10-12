// SPDX-License-Identifier: AGPL-3.0-or-later
//! The original private TIP 348 List header and its non-owning lifecycle metadata.

#[cfg(test)]
use crate::NativeListItems;
use crate::Value;
use tcl_runtime_api::error_stack::{
    ErrorStack, ErrorStackFrame, ErrorStackValueError, ShiftedErrorStackFrame,
};
use tcl_syntax::native_string::NativeStringProtocol;
use tcl_syntax::value::ValueError;

#[derive(Clone, Debug, Default)]
pub(super) struct NativeErrorStack {
    header: Option<Value>,
    // Saved exception states share this interpreter role; they do not retain
    // another native innerContext header or restore a previous context.
    inner_context: Option<std::rc::Rc<std::cell::RefCell<Value>>>,
    protocol: Option<NativeStringProtocol>,
    metadata: ErrorStack<()>,
}
impl NativeErrorStack {
    #[cfg(test)]
    pub(super) fn original_inner_context(&self) -> Option<std::cell::Ref<'_, Value>> {
        self.inner_context.as_ref().map(|context| context.borrow())
    }
    pub(super) fn configure(
        &mut self,
        recipe: Option<tcl_registry::native_error_objects::NativeErrorObjectsProtocol>,
    ) {
        let protocol = recipe
            .filter(|recipe| recipe.has_error_stack())
            .map(tcl_registry::native_error_objects::NativeErrorObjectsProtocol::strings);
        if self.protocol != protocol {
            self.protocol = protocol;
            self.header =
                protocol.map(|protocol| Value::native_list_constructor(Vec::new(), protocol));
            self.inner_context = protocol.map(|protocol| {
                std::rc::Rc::new(std::cell::RefCell::new(Value::native_list_constructor(
                    Vec::new(),
                    protocol,
                )))
            });
            self.metadata = ErrorStack::default();
        }
    }
    #[cfg(test)]
    pub(super) fn is_reset(&self) -> bool {
        self.metadata.is_reset()
    }
    pub(super) fn mark_reset(&mut self) {
        self.metadata.mark_reset();
    }
    fn replace(&mut self, values: &[Value]) {
        if let (Some(header), Some(protocol)) = (&self.header, self.protocol) {
            self.header = Some(
                Value::native_list_replace_elements(header, values, protocol)
                    .expect("private native List header"),
            );
        }
    }
    pub(super) fn adopt(&mut self, values: &[Value]) -> Result<(), ErrorStackValueError> {
        if !values.len().is_multiple_of(2) {
            return Err(ErrorStackValueError::OddSized);
        }
        self.replace(values);
        self.metadata.adopt(vec![(); values.len()])
    }
    pub(super) fn begin_inner(&mut self, tag: Value, context: Value) -> Result<bool, ValueError> {
        if !self.metadata.is_reset() {
            return Ok(false);
        }
        let protocol = self.protocol.ok_or(ValueError::CommandProtocolUnavailable(
            "native error-context owner",
        ))?;
        let context = context.capture_native_error_context(protocol)?;
        self.replace(&[tag, context]);
        Ok(self.metadata.begin_inner((), ()))
    }
    /// `TclGetInnerContext` reuses its own List header and retains original stack operands.
    pub(super) fn begin_instruction(
        &mut self,
        name: tcl_syntax::native_instruction_name::NativeInstructionName,
        operands: &[Value],
    ) -> Result<(), ValueError> {
        if !self.metadata.is_reset() {
            return Ok(());
        }
        let protocol = self.protocol.ok_or(ValueError::CommandProtocolUnavailable(
            "native instruction context owner",
        ))?;
        let mut values = Vec::with_capacity(operands.len() + 1);
        values.push(Value::new_native_instruction_name(name));
        values.extend(operands.iter().cloned());
        let context = Value::native_list_replace_elements(
            &self
                .inner_context
                .as_ref()
                .expect("original interpreter innerContext")
                .borrow(),
            &values,
            protocol,
        )?;
        let retired = self
            .inner_context
            .as_ref()
            .expect("original innerContext role")
            .replace(context.clone());
        drop(retired);
        self.begin_inner(Value::string("INNER"), context)?;
        Ok(())
    }
    pub(super) fn restart_inner(&mut self, tag: Value, context: Value) -> Result<bool, ValueError> {
        self.metadata.mark_reset();
        self.begin_inner(tag, context)
    }
    fn push_pair(&mut self, tag: Value, value: Value) -> bool {
        if self.metadata.is_reset() {
            return false;
        }
        if let (Some(header), Some(protocol)) = (&self.header, self.protocol) {
            self.header = Some(
                Value::native_list_append_elements(Some(header), &[tag, value], protocol)
                    .expect("private native List header"),
            );
        }
        self.metadata.push_pair((), ())
    }
    pub(super) fn log_frame(
        &mut self,
        frame: ErrorStackFrame<Value>,
        mut tag: impl FnMut(&str) -> Value,
    ) -> bool {
        match frame {
            ErrorStackFrame::Unreported => false,
            ErrorStackFrame::Redirect(value) => self.push_pair(tag("UP"), value),
            ErrorStackFrame::Call(value) => self.push_pair(tag("CALL"), value),
        }
    }
    pub(super) fn enter_shifted_context(&mut self, count: usize, frame: ShiftedErrorStackFrame) {
        self.metadata.enter_shifted_context(count, frame);
    }
    pub(super) fn leave_shifted_context(&mut self) {
        self.metadata.leave_shifted_context();
    }
    pub(super) fn shifted_context_frame(&self, count: usize) -> Option<ShiftedErrorStackFrame> {
        self.metadata.shifted_context_frame(count)
    }
    #[cfg(test)]
    pub(super) fn entries(&self) -> NativeListItems {
        self.header
            .as_ref()
            .and_then(Value::cached_list_representation)
            .map_or_else(
                || NativeListItems::new(Vec::new(), false).lifetime_view(),
                |(items, _)| items,
            )
    }
    pub(super) fn value(&self) -> Value {
        self.header
            .as_ref()
            .expect("selected private error-stack header")
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_stack_retains_header_and_children_through_save_cow_and_restore() {
        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let recipe = tcl_registry::InvocationDialect::for_version(version)
                .native_error_objects_protocol()
                .unwrap();
            let mut stack = NativeErrorStack::default();
            stack.configure(Some(recipe));
            let identity = stack.header.as_ref().unwrap().native_object_identity();
            assert_eq!(
                stack
                    .header
                    .as_ref()
                    .unwrap()
                    .native_object_reference_count(),
                1
            );
            assert!(
                stack
                    .header
                    .as_ref()
                    .unwrap()
                    .cached_list_representation()
                    .is_none()
            );
            let context = Value::new_native_string_bytes(b"original\0\xff".as_slice());
            stack
                .begin_inner(Value::string("INNER"), context.clone())
                .unwrap();
            assert_eq!(
                stack.header.as_ref().unwrap().native_object_identity(),
                identity
            );
            assert_eq!(context.native_object_reference_count(), 2);
            let saved = stack.clone();
            assert_eq!(
                stack
                    .header
                    .as_ref()
                    .unwrap()
                    .native_object_reference_count(),
                2
            );
            assert_eq!(context.native_object_reference_count(), 2);
            stack.mark_reset();
            stack
                .begin_inner(Value::string("INNER"), Value::string("callback"))
                .unwrap();
            assert_ne!(
                stack.header.as_ref().unwrap().native_object_identity(),
                identity
            );
            assert_eq!(
                saved
                    .header
                    .as_ref()
                    .unwrap()
                    .native_object_reference_count(),
                1
            );
            assert_eq!(context.native_object_reference_count(), 2);
            stack = saved;
            assert_eq!(
                stack.header.as_ref().unwrap().native_object_identity(),
                identity
            );
            assert!(stack.entries()[1].is_same_object(&context));
            drop(stack);
            assert_eq!(context.native_object_reference_count(), 1);
        }
    }
    #[test]
    fn reached_error_capture_retains_original_argv_members_after_invocation_release() {
        // naming.error.original-invocation-context-capture
        // docs/design/analysis/name-resolution-proofs/error-original-invocation-context-capture.md
        // Software ownership contract: no native private-pointer observation or
        // execution/frame permission follows from these VM reference counts.
        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let recipe = tcl_registry::InvocationDialect::for_version(version)
                .native_error_objects_protocol()
                .unwrap();
            let member = Value::native_list_constructor(
                vec![Value::string("Y"), Value::string("Z")],
                recipe.strings(),
            );
            let observed = member.native_lifetime_lease();
            let argv = NativeListItems::new(vec![member], false);
            let diagnostic = Value::invocation_list_view(&argv);
            assert_eq!(observed.value().native_object_reference_count(), 1);
            assert!(observed.value().resident_string_bytes().is_none());
            let mut stack = NativeErrorStack::default();
            stack.configure(Some(recipe));
            assert!(
                stack
                    .begin_inner(Value::string("INNER"), diagnostic)
                    .unwrap()
            );
            assert_eq!(observed.value().native_object_reference_count(), 2);
            assert!(observed.value().resident_string_bytes().is_none());
            drop(argv);
            assert!(observed.value().native_object_is_live());
            assert_eq!(observed.value().native_object_reference_count(), 1);
            let stack_value = stack.value();
            let entries = stack_value
                .native_object_list_elements(recipe.strings())
                .unwrap();
            let context = entries[1]
                .native_object_list_elements(recipe.strings())
                .unwrap();
            assert!(context[0].is_same_object(observed.value()));
            assert_eq!(
                stack_value
                    .native_string_bytes(recipe.strings())
                    .unwrap()
                    .as_ref(),
                b"INNER {{Y Z}}"
            );
            drop(context);
            drop(entries);
            drop(stack_value);
            drop(stack);
            assert!(!observed.value().native_object_is_live());
        }
    }

    #[test]
    fn reached_error_capture_declines_retired_argv_and_missing_protocol() {
        // naming.error.original-invocation-context-capture
        // docs/design/analysis/name-resolution-proofs/error-original-invocation-context-capture.md
        // No source text, fresh strings or retired-header revival repairs a view.
        let recipe = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6)
            .native_error_objects_protocol()
            .unwrap();
        let member = Value::native_list_constructor(vec![Value::string("old")], recipe.strings());
        let observed = member.native_lifetime_lease();
        let argv = NativeListItems::new(vec![member], false);
        let diagnostic = Value::invocation_list_view(&argv);
        drop(argv);
        assert!(!observed.value().native_object_is_live());
        let mut stack = NativeErrorStack::default();
        stack.configure(Some(recipe));
        assert!(
            stack
                .begin_inner(Value::string("INNER"), diagnostic)
                .is_err()
        );
        assert!(stack.is_reset());
        assert_eq!(
            stack
                .value()
                .native_string_bytes(recipe.strings())
                .unwrap()
                .as_ref(),
            b""
        );
        let mut missing = NativeErrorStack::default();
        assert!(
            missing
                .begin_inner(Value::string("INNER"), Value::string("actual"))
                .is_err()
        );
        assert!(missing.is_reset());
    }

    #[test]
    fn reached_capture_adapters_preserve_typed_retired_foreign_and_first_causes() {
        // naming.error.original-invocation-context-capture
        // docs/design/analysis/name-resolution-proofs/error-original-invocation-context-capture.md
        // Software transport: no Native execution or source-object observation.
        use tcl_runtime_api::NativeExecutionError;
        use tcl_syntax::raw_string::NativeValueAccessRefusal;
        let version = tcl_dialect::TclVersion::V8_6;
        let strings = NativeStringProtocol::C(version);
        let member = Value::native_list_constructor(vec![Value::string("old")], strings);
        let observed = member.native_lifetime_lease();
        let argv = NativeListItems::new(vec![member], false);
        let retired = Value::invocation_list_view(&argv);
        drop(argv);
        let foreign = Value::native_list_constructor(
            vec![Value::string("foreign")],
            NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0),
        );
        let profile = tcl_dialect::DialectProfile::find(version.dialect_name()).unwrap();
        let captures: [fn(&mut super::super::Vm, Value); 3] = [
            super::super::Vm::begin_error_stack_context,
            super::super::Vm::error_stack_log_value,
            super::super::Vm::error_stack_restart_with_inner,
        ];
        for capture in captures {
            for context in [&retired, &foreign] {
                let expected = context
                    .capture_native_error_context(strings)
                    .expect_err("unavailable original context")
                    .native_access_refusal()
                    .expect("typed operational cause");
                let mut vm = crate::native_fixture::core(profile);
                capture(&mut vm, context.clone());
                assert_eq!(
                    vm.execution_refusal,
                    Some(NativeExecutionError::ValueAccessRefusal(expected)),
                );
                assert!(vm.native_errors.error_stack.is_reset());
                let mut prior = crate::native_fixture::core(profile);
                let first = NativeExecutionError::ValueAccessRefusal(
                    NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "first capture obligation",
                    ),
                );
                prior.execution_refusal = Some(first.clone());
                capture(&mut prior, context.clone());
                assert_eq!(prior.execution_refusal, Some(first));
            }
        }
        assert!(!observed.value().native_object_is_live());
    }

    #[test]
    fn reached_instruction_capture_preserves_missing_owner_refusal() {
        // naming.error.original-invocation-context-capture
        // docs/design/analysis/name-resolution-proofs/error-original-invocation-context-capture.md
        use tcl_registry::native_return_options::NativeReturnOptionsApplication;
        use tcl_runtime_api::NativeExecutionError;
        use tcl_syntax::raw_string::NativeValueAccessRefusal;
        let mut vm =
            crate::native_fixture::core(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let purpose = NativeReturnOptionsApplication::Immediate;
        let name = vm
            .actual_native_invocation_dialect()
            .native_return_options_application(purpose)
            .unwrap()
            .inner_context_name()
            .unwrap();
        vm.native_errors.error_stack.configure(None);
        vm.capture_original_return_instruction_context(purpose, name, &[Value::string("actual")]);
        assert_eq!(
            vm.execution_refusal,
            Some(NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "native instruction context owner"
                ),
            )),
        );
    }

    #[test]
    fn explicit_stack_copies_original_members_into_the_private_header() {
        let recipe = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0)
            .native_error_objects_protocol()
            .unwrap();
        let mut stack = NativeErrorStack::default();
        stack.configure(Some(recipe));
        let identity = stack.header.as_ref().unwrap().native_object_identity();
        let member =
            Value::native_list_constructor(vec![Value::string("original")], recipe.strings());
        stack
            .adopt(&[Value::string("INNER"), member.clone()])
            .unwrap();
        assert_eq!(
            stack.header.as_ref().unwrap().native_object_identity(),
            identity
        );
        assert!(stack.entries()[1].is_same_object(&member));
        assert!(stack.entries()[1].resident_string_bytes().is_none());
        let getter = stack.value();
        assert!(getter.is_same_object(stack.header.as_ref().unwrap()));
        stack
            .adopt(&[Value::string("INNER"), Value::string("second")])
            .unwrap();
        assert!(!getter.is_same_object(stack.header.as_ref().unwrap()));
        assert!(getter.cached_list_representation().unwrap().0[1].is_same_object(&member));
    }
    #[test]
    fn return_options_overlay_uses_same_private_header_even_before_lazy_reset() {
        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let mut vm = super::super::Vm::new();
            vm.set_runtime_version(version);
            let original = vm.error_stack_value();
            let carried = Value::list(vec![Value::string("INNER"), Value::string("carried")]);
            let snapshot = vm.error_stack_for_completion(Some(carried.clone()));
            assert!(snapshot.is_same_object(&original));
            assert!(!snapshot.is_same_object(&carried));
            drop(snapshot);
            drop(original);
            vm.begin_error_stack_context(Value::string("private"));
            vm.native_errors.error_stack.mark_reset();
            let original = vm.error_stack_value();
            let snapshot = vm.error_stack_for_completion(Some(carried.clone()));
            assert!(snapshot.is_same_object(&original));
            assert_eq!(
                snapshot.cached_list_representation().unwrap().0[1]
                    .string_bytes()
                    .as_ref(),
                b"private"
            );
            assert!(vm.native_errors.error_stack.is_reset());
        }
    }
}
