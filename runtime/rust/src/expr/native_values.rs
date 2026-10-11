// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original collection and authored predicate ingress over shared value owners.

use super::{native_numeric::NumericOperation, ExprCtx, ExprError, Owned};
use crate::obj::{self, TclObj};
use tcl_syntax::expr::BinOp;
use tcl_syntax::value::ValueError;

/// The view owns allocation safety and the original List incarnation, without
/// becoming a native List header or an additional native member reference.
struct Members {
    backing: Option<crate::list::NativeListBacking>,
    elements: Vec<obj::NativeObjectLifetime>,
}
impl Members {
    fn capture(list: *mut TclObj, elements: Vec<*mut TclObj>) -> Result<Self, ExprError> {
        let elements = elements
            .into_iter()
            .map(|element| {
                obj::check_native_liveness(element)
                    .map_err(|error| ExprError::from_cmd_error(error.into()))?;
                Ok(obj::NativeObjectLifetime::retain(element))
            })
            .collect::<Result<Vec<_>, ExprError>>()?;
        Ok(Self {
            backing: crate::list::native_list_backing(list),
            elements,
        })
    }
    fn check(&self) -> Result<(), ExprError> {
        if let Some(backing) = &self.backing {
            backing
                .check_generation()
                .map_err(|error| ExprError::from_cmd_error(error.into()))?;
        }
        Ok(())
    }
}

pub(super) fn in_list(
    context: &mut dyn ExprCtx,
    needle: &Owned,
    list: &Owned,
) -> Result<bool, ExprError> {
    let operation = NumericOperation::enter_values(context)?;
    let result = (|| {
        operation.check()?;
        if let Some(interpreter) = operation.interpreter() {
            let associated =
                interpreter.associate_native_jim_arguments(&[needle.ptr(), list.ptr()]);
            operation.check()?;
            associated.map_err(|error| ExprError::from_cmd_error(error.into()))?;
        }
        let recipe = operation
            .dialect()
            .native_string_materialization(Some(
                tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation,
            ))
            .ok_or_else(|| {
                ExprError::from_cmd_error(
                    ValueError::CommandProtocolUnavailable("original expression List protocol").into(),
                )
            })?;
        let elements = crate::list::list_elements_native_with_currency(
            list.ptr(),
            recipe.protocol(),
            operation.currency(),
        );
        operation.check()?;
        let members = Members::capture(
            list.ptr(),
            elements.map_err(|error| ExprError::from_cmd_error(error.into()))?,
        )?;
        let needle = operation.string(needle.ptr())?;
        members.check()?;
        for member in &members.elements {
            operation.check()?;
            members.check()?;
            obj::check_native_liveness(member.as_ptr())
                .map_err(|error| ExprError::from_cmd_error(error.into()))?;
            let bytes = operation.string(member.as_ptr())?;
            members.check()?;
            obj::check_native_liveness(member.as_ptr())
                .map_err(|error| ExprError::from_cmd_error(error.into()))?;
            if bytes == needle {
                return Ok(true);
            }
        }
        Ok(false)
    })();
    operation.after(result)
}

pub(super) fn f5_predicate(
    context: &mut dyn ExprCtx,
    operator: BinOp,
    left: Owned,
    right: Owned,
) -> Result<Owned, ExprError> {
    let operation = NumericOperation::enter_values(context)?;
    let result = (|| {
        let predicate = context
            .f5_string_predicate_provider()
            .and_then(|provider| provider.predicate(operator))
            .ok_or_else(|| ExprError::msg(b"operator"))?;
        let checked = |value: &Owned| {
            tcl_syntax::raw_string::RawString::from_bytes(operation.string(value.ptr())?)
                .unicode()
                .map_err(|_| {
                    ExprError::from_cmd_error(
                        ValueError::CommandProtocolUnavailable(
                            "authored F5 predicate checked Unicode",
                        )
                        .into(),
                    )
                })
        };
        let left = checked(&left)?;
        let right = checked(&right)?;
        super::native_integer_result(
            operation.dialect(),
            i64::from(predicate.evaluate(&left, &right)),
            &[],
        )
    })();
    // Both transferred native references retire on every path, before final
    // currency settlement can publish the result.
    drop(left);
    drop(right);
    operation.after(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interp::Interp;
    use std::cell::Cell;
    use tcl_runtime_api::NativeExecutionError;
    use tcl_syntax::raw_string::NativeValueAccessRefusal;

    thread_local! {
        static CALLS: Cell<usize> = const { Cell::new(0) };
        static RETIRE_LIST: Cell<*mut TclObj> = const { Cell::new(core::ptr::null_mut()) };
    }
    extern "C" fn updater(value: *mut TclObj) {
        CALLS.with(|calls| calls.set(calls.get() + 1));
        let mut interpreter = unsafe { crate::codegen_abi::current_interp().as_ref() }
            .unwrap()
            .clone();
        interpreter.refuse_host_command("first original expression value updater");
        unsafe { obj::set_native_updater_string_rep(value, b"member", false) };
    }
    extern "C" fn retire_list_updater(value: *mut TclObj) {
        CALLS.with(|calls| calls.set(calls.get() + 1));
        RETIRE_LIST.with(|list| obj::change_type(list.get(), core::ptr::null(), 0));
        unsafe { obj::set_native_updater_string_rep(value, b"member", false) };
    }
    static REFUSING: obj::TclObjType = obj::TclObjType {
        name: c"originalExpressionValueRefusing".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(updater),
        set_from_any_proc: None,
    };
    static RETIRING: obj::TclObjType = obj::TclObjType {
        name: c"originalExpressionListRetiring".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(retire_list_updater),
        set_from_any_proc: None,
    };
    struct Context(Interp);
    impl ExprCtx for Context {
        fn boolean_interpreter(&mut self) -> Option<&mut Interp> {
            Some(&mut self.0)
        }
        fn numeric_host(&self) -> Option<std::rc::Rc<dyn tcl_platform::Host>> {
            Some(self.0.host())
        }
        fn invocation_dialect(&self) -> tcl_registry::InvocationDialect {
            self.0.native_invocation_dialect()
        }
        fn f5_string_predicate_provider(
            &self,
        ) -> Option<tcl_syntax::expr::operators::AuthoredF5StringPredicateProvider> {
            tcl_registry::native_expression_program::authored_f5_string_predicate_provider(
                self.0.expression_evaluation_policy().as_ref(),
            )
        }
        fn read_var(&mut self, _: &str) -> Result<Owned, ExprError> {
            Err(ExprError::msg(b"unexpected variable"))
        }
        fn eval_command(&mut self, _: &str) -> Result<Owned, ExprError> {
            Err(ExprError::msg(b"unexpected command"))
        }
        fn call_function(&mut self, _: &str, _: &[Owned]) -> Result<Owned, ExprError> {
            Err(ExprError::msg(b"unexpected function"))
        }
    }
    fn native() -> Context {
        Context(
            Interp::with_native_core(
                crate::interp::default_host(),
                tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile(),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap(),
        )
    }
    fn protocol(context: &Context) -> tcl_syntax::native_string::NativeStringProtocol {
        context
            .0
            .native_invocation_dialect()
            .native_string_protocol()
            .unwrap()
    }
    #[test]
    fn original_list_membership_stops_after_reached_list_needle_or_member_first_host() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Software updater/first-cause control, not native IN opcode behaviour.
        for stage in 0..3 {
            let mut context = native();
            context.0.set_result_bytes(b"PRIOR\0\xff");
            let prior = context.0.get_obj_result();
            let original = Owned::fresh(obj::alloc_typed(&REFUSING, 0));
            let later = Owned::fresh(obj::new_string_bytes(b"later"));
            let later_snapshot = obj::native_object_snapshot(later.ptr()).unwrap();
            let needle = if stage == 1 {
                original.clone()
            } else {
                Owned::fresh(obj::new_string_bytes(b"member"))
            };
            let list = if stage == 0 {
                original.clone()
            } else {
                Owned::fresh(crate::list::new_list_obj_native(
                    &[
                        if stage == 2 {
                            original.ptr()
                        } else {
                            later.ptr()
                        },
                        later.ptr(),
                    ],
                    protocol(&context),
                ))
            };
            let references = unsafe { (*original.ptr()).ref_count };
            CALLS.with(|calls| calls.set(0));
            crate::codegen_abi::tcl_runtime_set_current_interp(&mut context.0);
            let error = in_list(&mut context, &needle, &list).unwrap_err();
            let first = context.0.native_execution_refusal().unwrap();
            assert!(
                matches!(&first, NativeExecutionError::HostCommandRefusal(first) if first.reason == "first original expression value updater")
            );
            assert_eq!(error.native_execution_refusal, Some(first));
            assert_eq!(CALLS.with(Cell::get), 1);
            assert_eq!(obj::bytes_of(original.ptr()), b"member");
            assert_eq!(obj::obj_type_ptr(original.ptr()), &REFUSING as *const _);
            assert_eq!(unsafe { (*original.ptr()).ref_count }, references);
            assert_eq!(
                obj::native_object_snapshot(later.ptr()).unwrap(),
                later_snapshot
            );
            assert_eq!(context.0.get_obj_result(), prior);
            assert_eq!(context.0.result_bytes(), b"PRIOR\0\xff");
            assert_eq!(context.0.entered_native_operation_count(), 0);
            crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
        }
    }
    #[test]
    fn original_list_membership_keeps_counted_bytes_and_native_member_reference_roles() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Original software List/String producers; no provider result comparison.
        let mut context = native();
        let member = Owned::fresh(obj::new_string_bytes("é\0literal $ []".as_bytes()));
        let list = Owned::fresh(crate::list::new_list_obj_native(
            &[member.ptr()],
            protocol(&context),
        ));
        let needle = Owned::fresh(obj::new_string_bytes("é\0literal $ []".as_bytes()));
        let member_refs = unsafe { (*member.ptr()).ref_count };
        let headers = crate::list::native_header_reference_count(list.ptr()).unwrap();
        assert!(in_list(&mut context, &needle, &list).unwrap());
        assert_eq!(unsafe { (*member.ptr()).ref_count }, member_refs);
        assert_eq!(
            crate::list::native_header_reference_count(list.ptr()).unwrap(),
            headers
        );
        assert!(!obj::has_string_rep(list.ptr()));
        assert!(!context.0.host_refusal_pending());
        let foreign = Owned::fresh(crate::list::new_list_obj_native(
            &[member.ptr()],
            tcl_syntax::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0),
        ));
        let refs = unsafe { (*member.ptr()).ref_count };
        assert!(in_list(&mut context, &needle, &foreign)
            .unwrap_err()
            .native_execution_refusal
            .is_some());
        assert_eq!(unsafe { (*member.ptr()).ref_count }, refs);
        assert!(!obj::has_string_rep(foreign.ptr()));
    }
    #[test]
    fn original_list_membership_retains_allocation_safety_without_reviving_retired_members() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Real backing retirement during original getter; native child count stays unchanged.
        let mut context = native();
        let member = obj::new_string_bytes(b"member");
        let list = Owned::fresh(crate::list::new_list_obj_native(
            &[member],
            protocol(&context),
        ));
        let needle = Owned::fresh(obj::alloc_typed(&RETIRING, 0));
        CALLS.with(|calls| calls.set(0));
        RETIRE_LIST.with(|slot| slot.set(list.ptr()));
        let error = in_list(&mut context, &needle, &list).unwrap_err();
        assert!(matches!(
            error.native_execution_refusal,
            Some(NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::CommandProtocolUnavailable(_)
            ))
        ));
        assert_eq!(CALLS.with(Cell::get), 1);
        assert_eq!(obj::obj_type_ptr(list.ptr()), core::ptr::null());
        assert!(!context.0.host_refusal_pending());
        assert_eq!(context.0.entered_native_operation_count(), 0);
        RETIRE_LIST.with(|slot| slot.set(core::ptr::null_mut()));
    }
    #[test]
    fn authored_f5_predicates_require_policy_checked_unicode_and_owned_cleanup() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Authored expression policy only; no BIG-IP native predicate or character claim.
        let mut context = native();
        let left = Owned::fresh(obj::new_string_bytes("é\0prefix suffix".as_bytes()));
        let right = Owned::fresh(obj::new_string_bytes("é\0".as_bytes()));
        assert!(f5_predicate(&mut context, BinOp::StartsWith, left, right).is_err());
        context
            .0
            .set_dialect_profile(tcl_dialect::DialectProfile::irules());
        assert!(context.0.set_logical_expression_parse_provider(
            tcl_registry::invocation_words::LogicalExpressionParseProvider::Tcl84CoreSimulation,
            tcl_dialect::DialectProfile::find("tcl8.6").unwrap()
        ));
        let left = Owned::fresh(obj::new_string_bytes("é\0prefix suffix".as_bytes()));
        let right = Owned::fresh(obj::new_string_bytes("é\0".as_bytes()));
        let result = f5_predicate(&mut context, BinOp::StartsWith, left, right).unwrap();
        assert_eq!(obj::bytes_of(result.ptr()), b"1");
        let invalid = Owned::fresh(obj::new_string_bytes(&[0xff]));
        let invalid_view = invalid.clone();
        let error = f5_predicate(
            &mut context,
            BinOp::Contains,
            invalid,
            Owned::fresh(obj::new_string_bytes(b"x")),
        )
        .err()
        .expect("original predicate must refuse");
        assert!(error.native_execution_refusal.is_some());
        assert_eq!(unsafe { (*invalid_view.ptr()).ref_count }, 1);
        assert_eq!(obj::bytes_of(invalid_view.ptr()), &[0xff]);
        assert_eq!(context.0.entered_native_operation_count(), 0);
    }
    #[test]
    fn authored_f5_getter_first_host_stops_before_later_operand_and_result() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Actual software updater under an installed authored policy; no BIG-IP result claim.
        let mut context = native();
        context
            .0
            .set_dialect_profile(tcl_dialect::DialectProfile::irules());
        assert!(context.0.set_logical_expression_parse_provider(
            tcl_registry::invocation_words::LogicalExpressionParseProvider::Tcl84CoreSimulation,
            tcl_dialect::DialectProfile::find("tcl8.6").unwrap()
        ));
        context.0.set_result_bytes(b"PRIOR\0\xff");
        let prior = context.0.get_obj_result();
        let left = Owned::fresh(obj::alloc_typed(&REFUSING, 0));
        let right = Owned::fresh(obj::new_string_bytes(b"member"));
        let left_view = left.clone();
        let right_view = right.clone();
        let before = obj::native_object_snapshot(right.ptr()).unwrap();
        CALLS.with(|calls| calls.set(0));
        crate::codegen_abi::tcl_runtime_set_current_interp(&mut context.0);
        let error = f5_predicate(&mut context, BinOp::Contains, left, right)
            .err()
            .expect("original predicate must refuse");
        let first = context.0.native_execution_refusal().unwrap();
        assert!(
            matches!(&first, NativeExecutionError::HostCommandRefusal(first) if first.reason == "first original expression value updater")
        );
        assert_eq!(error.native_execution_refusal, Some(first));
        assert_eq!(CALLS.with(Cell::get), 1);
        assert_eq!(obj::bytes_of(left_view.ptr()), b"member");
        assert_eq!(obj::obj_type_ptr(left_view.ptr()), &REFUSING as *const _);
        assert_eq!(
            obj::native_object_snapshot(right_view.ptr()).unwrap(),
            before
        );
        assert_eq!(unsafe { (*left_view.ptr()).ref_count }, 1);
        assert_eq!(unsafe { (*right_view.ptr()).ref_count }, 1);
        assert_eq!(context.0.get_obj_result(), prior);
        assert_eq!(context.0.result_bytes(), b"PRIOR\0\xff");
        assert_eq!(context.0.entered_native_operation_count(), 0);
        crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
    }
}
