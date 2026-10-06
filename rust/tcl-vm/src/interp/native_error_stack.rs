// SPDX-License-Identifier: AGPL-3.0-or-later
//! The original private TIP 348 List header and its non-owning lifecycle metadata.

#[cfg(test)]
use crate::NativeListItems;
use crate::Value;
use tcl_runtime_api::error_stack::{
    ErrorStack, ErrorStackFrame, ErrorStackValueError, ShiftedErrorStackFrame,
};
use tcl_syntax::native_string::NativeStringProtocol;

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
            .map(|recipe| recipe.strings());
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
    pub(super) fn adopt(&mut self, values: Vec<Value>) -> Result<(), ErrorStackValueError> {
        if !values.len().is_multiple_of(2) {
            return Err(ErrorStackValueError::OddSized);
        }
        self.replace(&values);
        self.metadata.adopt(vec![(); values.len()])
    }
    pub(super) fn begin_inner(&mut self, tag: Value, context: Value) -> bool {
        if !self.metadata.is_reset() {
            return false;
        }
        self.replace(&[tag, context]);
        self.metadata.begin_inner((), ())
    }
    /// `TclGetInnerContext` reuses its own List header and retains original stack operands.
    pub(super) fn begin_instruction(
        &mut self,
        name: tcl_syntax::native_instruction_name::NativeInstructionName,
        operands: &[Value],
    ) {
        if !self.metadata.is_reset() {
            return;
        }
        let protocol = self.protocol.expect("selected native inner context");
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
        )
        .expect("original native innerContext List");
        let retired = self
            .inner_context
            .as_ref()
            .expect("original innerContext role")
            .replace(context.clone());
        drop(retired);
        self.begin_inner(Value::string("INNER"), context);
    }
    pub(super) fn restart_inner(&mut self, tag: Value, context: Value) {
        self.metadata.mark_reset();
        self.begin_inner(tag, context);
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
            .map(|(items, _)| items)
            .unwrap_or_else(|| NativeListItems::new(Vec::new(), false).lifetime_view())
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
            stack.begin_inner(Value::string("INNER"), context.clone());
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
            stack.begin_inner(Value::string("INNER"), Value::string("callback"));
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
            .adopt(vec![Value::string("INNER"), member.clone()])
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
            .adopt(vec![Value::string("INNER"), Value::string("second")])
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
            vm.error_stack.mark_reset();
            let original = vm.error_stack_value();
            let snapshot = vm.error_stack_for_completion(Some(carried.clone()));
            assert!(snapshot.is_same_object(&original));
            assert_eq!(
                snapshot.cached_list_representation().unwrap().0[1]
                    .string_bytes()
                    .as_ref(),
                b"private"
            );
            assert!(vm.error_stack.is_reset());
        }
    }
}
