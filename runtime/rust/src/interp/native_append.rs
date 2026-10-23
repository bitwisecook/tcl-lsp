// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native append keeps one physical receiver across operand publication boundaries.

use super::{Code, Interp, TraceAccess};
use crate::frame::VariableReceiver;
use crate::obj::TclObj;
use crate::value_ops::{RuntimeAppendObjects, RuntimeAppendValue};
use tcl_cmd_core::native_append::{
    append_continuation, append_object, append_operands, NativeAppendVariable, PreparedAppendValue,
};
use tcl_syntax::native_object_append::NativeObjectAppendProtocol;

trait VariableAppendObjects:
    tcl_cmd_core::native_append::NativeAppendObjects<Value = RuntimeAppendValue>
{
    fn finish_append<T>(
        &self,
        reached: Result<T, tcl_syntax::value::ValueError>,
    ) -> Result<T, tcl_cmd_core::CmdError>;
}
impl VariableAppendObjects for RuntimeAppendObjects {
    fn finish_append<T>(
        &self,
        reached: Result<T, tcl_syntax::value::ValueError>,
    ) -> Result<T, tcl_cmd_core::CmdError> {
        reached.map_err(Into::into)
    }
}
impl VariableAppendObjects for crate::value_ops::OriginalRuntimeAppendObjects {
    fn finish_append<T>(
        &self,
        reached: Result<T, tcl_syntax::value::ValueError>,
    ) -> Result<T, tcl_cmd_core::CmdError> {
        self.finish(reached)
    }
}

/// The actual caller's frame and selected receiver observation. These callbacks
/// transport existing original checks and update its expected publication only.
pub(super) struct OriginalAppendVariable {
    pub(super) current: std::rc::Rc<dyn Fn() -> Result<(), tcl_cmd_core::CmdError>>,
    pub(super) entered: std::rc::Rc<dyn Fn() -> Result<(), tcl_cmd_core::CmdError>>,
    pub(super) select: std::rc::Rc<dyn Fn(Result<Option<*mut TclObj>, crate::frame::VarError>)>,
    pub(super) refuse: std::rc::Rc<dyn Fn(tcl_syntax::value::ValueError) -> tcl_cmd_core::CmdError>,
}

/// Earliest physical append operation and existing actual frame observation.
struct OriginalAppendCommand {
    arguments: super::native_arguments::OriginalArguments,
    frame: super::native_arguments::OriginalArgumentFrame,
    actual: Interp,
    namespace: crate::namespace::NsId,
}
impl OriginalAppendCommand {
    fn capture(
        actual: &Interp,
        argv: &[*mut TclObj],
    ) -> Result<std::rc::Rc<Self>, tcl_cmd_core::CmdError> {
        let original = std::rc::Rc::new(Self {
            arguments: super::native_arguments::OriginalArguments::enter(actual, argv)?,
            frame: super::native_arguments::OriginalArgumentFrame::capture(actual),
            actual: actual.clone(),
            namespace: actual.current_ns.get(),
        });
        original.check()?;
        Ok(original)
    }
    fn check(&self) -> Result<(), tcl_cmd_core::CmdError> {
        self.frame.check(&self.actual, &self.arguments)?;
        if self.actual.current_ns.get() != self.namespace
            || !self
                .actual
                .namespaces
                .borrow()
                .namespace_is_live(self.namespace)
        {
            return Err(
                self.refuse(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original append namespace",
                )),
            );
        }
        Ok(())
    }
    fn retain(&self, error: tcl_cmd_core::CmdError) -> tcl_cmd_core::CmdError {
        self.arguments.retain_refusal(error)
    }
    fn refuse(&self, error: tcl_syntax::value::ValueError) -> tcl_cmd_core::CmdError {
        self.retain(error.into())
    }
    fn bytes(
        &self,
        value: *mut TclObj,
        extra: &dyn Fn() -> Result<(), tcl_cmd_core::CmdError>,
    ) -> Result<std::rc::Rc<[u8]>, tcl_cmd_core::CmdError> {
        self.check()?;
        extra()?;
        let header = super::native_arguments::OriginalArguments::enter(&self.actual, &[value])?;
        self.actual.native_object_string_bytes_using(
            value,
            &|| {
                self.check()?;
                extra()?;
                header.check_original_for(&self.actual, 0, value)
            },
            &|error| self.refuse(error),
        )
    }
}
struct OriginalAppendReceiver {
    receiver: VariableReceiver,
    expected: std::cell::Cell<Result<Option<*mut TclObj>, crate::frame::VarError>>,
    _allocation: Option<crate::obj::NativeObjectLifetime>,
}
impl OriginalAppendReceiver {
    fn new(receiver: VariableReceiver) -> std::rc::Rc<Self> {
        let expected = receiver.read_initial();
        std::rc::Rc::new(Self {
            _allocation: expected
                .ok()
                .flatten()
                .map(crate::obj::NativeObjectLifetime::retain),
            receiver,
            expected: std::cell::Cell::new(expected),
        })
    }
    fn entered(&self, original: &OriginalAppendCommand) -> Result<(), tcl_cmd_core::CmdError> {
        original.check()?;
        self.receiver.check_current().map_err(|_| {
            original.refuse(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original append receiver",
            ))
        })?;
        if self.receiver.read_initial() == Err(crate::frame::VarError::NameProtocolUnavailable) {
            return Err(original.refuse(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original append scalar role",
                ),
            ));
        }
        Ok(())
    }
    fn check(&self, original: &OriginalAppendCommand) -> Result<(), tcl_cmd_core::CmdError> {
        self.entered(original)?;
        let actual = self.receiver.read_initial();
        if actual != self.expected.get() {
            return Err(original.refuse(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original append receiver header",
                ),
            ));
        }
        if let Ok(Some(value)) = actual {
            crate::obj::check_native_liveness(value).map_err(|error| original.refuse(error))?;
        }
        Ok(())
    }
}

// The neutral owner keeps its actual owned receiver drop order. An original
// caller borrows the independently captured receiver without another rmw role.
enum AppendReceiverReference<'a> {
    Owned(VariableReceiver),
    Original(&'a VariableReceiver),
}
impl std::ops::Deref for AppendReceiverReference<'_> {
    type Target = VariableReceiver;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Owned(receiver) => receiver,
            Self::Original(receiver) => receiver,
        }
    }
}

struct AppendVariable<'a, O> {
    interp: &'a mut Interp,
    name: &'a [u8],
    receiver: AppendReceiverReference<'a>,
    home: std::borrow::Cow<'a, crate::vars::TraceHome>,
    access: TraceAccess,
    protocol: NativeObjectAppendProtocol,
    objects: O,
    original: Option<OriginalAppendVariable>,
    pending: Option<PreparedAppendValue<RuntimeAppendValue>>,
}
impl<O> AppendVariable<'_, O> {
    fn check_original(&mut self, selected: bool) -> Result<(), Code> {
        if let Some(original) = &self.original {
            let reached = if selected {
                (original.current)()
            } else {
                (original.entered)()
            };
            reached.map_err(|first| self.interp.report_cmd_error(first))?;
        }
        Ok(())
    }
}
impl<O: VariableAppendObjects> NativeAppendVariable for AppendVariable<'_, O> {
    type Value = RuntimeAppendValue;
    type Error = Code;

    fn append_operand(&mut self, source: &RuntimeAppendValue) -> Result<(), Code> {
        self.check_original(true)?;
        if let Some(pending) = &mut self.pending {
            let reached = append_continuation(&self.objects, pending, source);
            return self
                .objects
                .finish_append(reached)
                .map_err(|first| self.interp.report_cmd_error(first));
        }
        let original = self
            .receiver
            .read_initial()
            .map_err(|error| crate::builtins::var_error(self.interp, self.name, error))?
            .map(RuntimeAppendValue::borrowed);
        let reached = append_object(&self.objects, self.protocol, original.as_ref(), source);
        let prepared = self
            .objects
            .finish_append(reached)
            .map_err(|first| self.interp.report_cmd_error(first))?;
        self.pending = Some(prepared);
        Ok(())
    }

    fn publish(&mut self) -> Result<RuntimeAppendValue, Code> {
        self.check_original(true)?;
        let pending = self
            .pending
            .take()
            .expect("pending native append value")
            .into_value();
        if let Some(original) = &self.original {
            (original.select)(Ok(Some(pending.as_ptr())));
        }
        if let Err(error) = self.receiver.store(pending.as_ptr()) {
            self.check_original(false)?;
            return Err(crate::builtins::var_error(self.interp, self.name, error));
        }
        // The actual variable role precedes working retirement and write observers.
        drop(pending);
        self.check_original(true)?;
        let failed = self
            .interp
            .fire_var_trace_resolved(&self.home, &self.access, b"write")
            .map_err(|first| self.interp.refuse_native_execution(first))?
            .is_guest_error();
        self.check_original(false)?;
        if failed {
            return Err(crate::builtins::var_error(
                self.interp,
                self.name,
                crate::frame::VarError::TraceError,
            ));
        }
        if self.interp.host_refusal_pending() {
            return Err(Code::Error);
        }
        if let Some(original) = &self.original {
            if self.receiver.check_current().is_err() {
                return Err(self.interp.report_cmd_error((original.refuse)(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original compiled String append receiver",
                    ),
                )));
            }
        }
        let selected = self.receiver.read();
        if let Some(original) = &self.original {
            if selected == Err(crate::frame::VarError::NameProtocolUnavailable) {
                return Err(self.interp.report_cmd_error((original.refuse)(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original compiled String append result scalar role",
                    ),
                )));
            }
            // The existing live receiver may be legally replaced by a write trace.
            (original.select)(selected);
        }
        let result = match selected {
            Ok(Some(value)) => RuntimeAppendValue::borrowed(value),
            Ok(None) | Err(_) => RuntimeAppendValue::fresh_string(b""),
        };
        self.check_original(false)?;
        Ok(result)
    }
}

struct FreshCountedAppendDuplicate {
    allocation: crate::obj::NativeObjectLifetime,
    transferred: bool,
}
impl FreshCountedAppendDuplicate {
    fn new(original: *mut TclObj) -> Self {
        Self {
            allocation: crate::obj::NativeObjectLifetime::retain(original),
            transferred: false,
        }
    }
}
impl Drop for FreshCountedAppendDuplicate {
    fn drop(&mut self) {
        let original = self.allocation.as_ptr();
        if !self.transferred
            && crate::obj::allocation_is_live(original)
            && unsafe { (*original).ref_count == 0 }
        {
            // No native owner was taken on this refused, unpublished allocation.
            unsafe { crate::obj::free_obj(original) };
        }
    }
}

impl Interp {
    /// Counted C setter append onto the receiver selected by the original name.
    /// Guest setter failure remains separate from the original typed Host cause.
    pub(super) fn append_native_counted_c_variable_bytes_checked(
        &mut self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
        captured: super::native_variable_names::OriginalCVariableCapture,
        bytes: &[u8],
    ) -> Result<super::VariableTraceGuestOutcome, tcl_runtime_api::NativeExecutionError> {
        currency.ensure_current_or_refuse()?;
        let mut pending = None;
        let reached = (|| {
            self.check_counted_c_append_receiver(currency, &captured)?;
            if !self.uses_c84_global_error_info() {
                return Err(self.refuse_completion_value_access(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "C84 original counted setter",
                    ),
                ));
            }
            let dialect = self.native_invocation_dialect();
            let protocol = dialect.native_object_append_protocol(None).ok_or_else(|| {
                self.refuse_completion_value_access(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "native counted variable append",
                    ),
                )
            })?;
            // Native proof: naming.error.c84-publication-order

            // C84 TclPtrSetVar (tclVar.c 1652--1677): the actual message owns
            // its caller reference, a missing cell adopts it, and COW publishes
            // a genuine rc0 duplicate before retiring the old cell reference.
            // Host refusal during the pending stage is a software bound only.
            let source = RuntimeAppendValue::fresh_string(bytes);
            if source.as_ptr().is_null() {
                return Err(self.refuse_completion_value_access(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original counted message allocation",
                    ),
                ));
            }
            let initial = match captured.receiver.read() {
                Ok(initial) => initial,
                Err(crate::frame::VarError::NameProtocolUnavailable) => {
                    return Err(self.refuse_completion_value_access(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original counted append pending scalar read",
                        ),
                    ));
                }
                Err(_) => return Ok(super::VariableTraceGuestOutcome::GuestError),
            };
            let objects = RuntimeAppendObjects {
                dialect,
                binary_recipe: dialect.byte_array_string_recipe(None),
            };
            let published = if let Some(original) = initial {
                let lifetime = crate::obj::NativeObjectLifetime::retain(original);
                let check_original = || {
                    self.check_counted_c_append_receiver(currency, &captured)?;
                    crate::obj::check_native_liveness(lifetime.as_ptr())
                        .map_err(|error| self.clone().refuse_completion_value_access(error))?;
                    if captured.receiver.read().ok() != Some(Some(original)) {
                        return Err(self.clone().refuse_completion_value_access(
                            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                "original counted append contents replaced",
                            ),
                        ));
                    }
                    Ok(())
                };
                check_original()?;
                let selected = if crate::obj::is_shared(original) {
                    let duplicate = crate::obj::duplicate(original);
                    if duplicate.is_null() {
                        return Err(self.clone().refuse_completion_value_access(
                            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                "original counted append duplicate allocation",
                            ),
                        ));
                    }
                    let mut fresh = FreshCountedAppendDuplicate::new(duplicate);
                    check_original()?;
                    crate::list::duplicate_native_backing(
                        original,
                        duplicate,
                        protocol.recipe().string_protocol(),
                    );
                    check_original()?;
                    // Transfer only the unowned allocation; this acquires no +1.
                    fresh.transferred = true;
                    let publication = captured
                        .receiver
                        .publish_c84_pending_scalar(original, duplicate);
                    drop(fresh);
                    let (prepared, retired) = publication.map_err(|_| {
                        self.clone().refuse_completion_value_access(
                            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                "original counted append pending publication",
                            ),
                        )
                    })?;
                    pending = Some(prepared);
                    let prepared = pending.as_ref().expect("original pending scalar");
                    retired.finish(|| {
                        currency.ensure_current_or_refuse()?;
                        captured
                            .receiver
                            .check_c84_pending_scalar(prepared)
                            .map_err(|_| {
                                self.clone().refuse_completion_value_access(
                                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                        "original counted append pending cell withdrawn",
                                    ),
                                )
                            })
                    })?;
                    currency.ensure_current_or_refuse()?;
                    captured
                        .receiver
                        .settle_c84_pending_scalar(prepared)
                        .map_err(|_| {
                            self.clone().refuse_completion_value_access(
                                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                    "original counted append pending role cannot settle",
                                ),
                            )
                        })?
                } else {
                    original
                };
                let published = crate::obj::NativeObjectLifetime::retain(selected);
                let borrowed = RuntimeAppendValue::borrowed(selected);
                let check = || {
                    self.check_counted_c_append_receiver(currency, &captured)?;
                    crate::obj::check_native_liveness(published.as_ptr())
                        .map_err(|error| self.clone().refuse_completion_value_access(error))?;
                    if captured.receiver.read().ok() != Some(Some(selected)) {
                        return Err(self.clone().refuse_completion_value_access(
                            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                "original counted append publication replaced",
                            ),
                        ));
                    }
                    Ok(())
                };
                let appended = tcl_cmd_core::native_append::append_published_counted_source_checked(
                    &objects,
                    protocol.recipe(),
                    &borrowed,
                    &source,
                    &mut || {
                        check().map_err(|_| {
                            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                "entered original counted variable append",
                            )
                        })
                    },
                );
                check()?;
                appended.map_err(|error| self.clone().refuse_completion_value_access(error))?;
                drop(borrowed);
                drop(lifetime);
                published
            } else {
                let published = crate::obj::NativeObjectLifetime::retain(source.as_ptr());
                let stored = captured.receiver.store(source.as_ptr());
                self.check_counted_c_append_receiver(currency, &captured)?;
                if stored.is_err() {
                    return Ok(super::VariableTraceGuestOutcome::GuestError);
                }
                published
            };
            // The actual message's caller +1 retires before the write observer.
            drop(source);
            self.check_counted_c_append_receiver(currency, &captured)?;
            crate::obj::check_native_liveness(published.as_ptr())
                .map_err(|error| self.refuse_completion_value_access(error))?;
            if captured.receiver.read().ok() != Some(Some(published.as_ptr())) {
                return Err(self.refuse_completion_value_access(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original counted append final cell replaced",
                    ),
                ));
            }
            let access = self.trace_access(
                &captured.root,
                &captured.root,
                captured.element.as_deref(),
                &captured.home,
                false,
            );
            let traced =
                self.fire_var_trace_resolved_with_errors(&captured.home, &access, b"write", false);
            self.check_counted_c_append_receiver(currency, &captured)?;
            drop(published);
            currency.ensure_current_or_refuse()?;
            traced
        })();
        // Cleanup uses the captured physical cell and exact ticket even after
        // refusal; it cannot adopt or remove a callback-created replacement.
        if let Some(prepared) = pending {
            let retired = captured.receiver.discard_c84_pending_scalar(&prepared);
            let cleaned = retired.finish(|| currency.ensure_current_or_refuse());
            drop(prepared);
            if let Err(first) = cleaned {
                self.refuse_native_execution(first);
            }
        }
        drop(captured);
        currency.ensure_current_or_refuse()?;
        reached
    }

    fn check_counted_c_append_receiver(
        &self,
        currency: &super::native_operation_currency::NativeOperationCurrency,
        captured: &super::native_variable_names::OriginalCVariableCapture,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        currency.ensure_current_or_refuse()?;
        if captured.receiver.check_current().is_err() {
            return Err(self.clone().refuse_completion_value_access(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original counted append receiver retired",
                ),
            ));
        }
        Ok(())
    }

    /// Physical Native argv handoff; the explicit Logical provider uses the
    /// neutral worker separately and cannot acquire Native authority here.
    pub(crate) fn append_original_native_operands(
        &mut self,
        argv: &[*mut TclObj],
        protocol: NativeObjectAppendProtocol,
    ) -> Code {
        let original = match OriginalAppendCommand::capture(self, argv) {
            Ok(original) => original,
            Err(first) => return self.report_cmd_error(first),
        };
        let reached = (|| -> Result<Code, Code> {
            original
                .arguments
                .check_original_for(self, 1, argv[1])
                .map_err(|first| self.report_cmd_error(first))?;
            let name = original
                .bytes(argv[1], &|| Ok(()))
                .map_err(|first| self.report_cmd_error(first))?;
            original
                .check()
                .map_err(|first| self.report_cmd_error(first))?;
            let (base, element) = self
                .variable_name_parts(&name)
                .map_err(|error| crate::builtins::var_error(self, &name, error))?;
            original
                .check()
                .map_err(|first| self.report_cmd_error(first))?;
            let sources = &argv[2..];
            if sources.is_empty() {
                let traced = self.fire_read_trace(&base, element.as_deref());
                original
                    .check()
                    .map_err(|first| self.report_cmd_error(first))?;
                if let Some(code) = traced {
                    return Ok(code);
                }
                let value = match &element {
                    Some(key) => self.var_get_elem(&base, key),
                    None => self.var_get(&base),
                };
                original
                    .check()
                    .map_err(|first| self.report_cmd_error(first))?;
                let code = match value {
                    Some(value) => {
                        self.set_result(value);
                        Code::Ok
                    }
                    None => {
                        let message = self.read_miss_msg(&base, element.as_deref());
                        self.set_error(&message)
                    }
                };
                original
                    .check()
                    .map_err(|first| self.report_cmd_error(first))?;
                return Ok(code);
            }
            let constant = self.const_write_check(&name);
            original
                .check()
                .map_err(|first| self.report_cmd_error(first))?;
            if let Some(code) = constant {
                return Ok(code);
            }
            let receiver = crate::vars::capture_variable_receiver(
                &mut self.frames.borrow_mut(),
                &mut self.namespaces.borrow_mut(),
                original.namespace,
                &base,
                element.as_deref(),
            );
            original
                .check()
                .map_err(|first| self.report_cmd_error(first))?;
            let receiver =
                receiver.map_err(|error| crate::builtins::var_error(self, &name, error))?;
            let selected = OriginalAppendReceiver::new(receiver);
            let home = self.trace_identity(&base);
            let access = self.trace_access(&name, &base, element.as_deref(), &home, false);
            let current_original = original.clone();
            let current_selected = selected.clone();
            let current: std::rc::Rc<dyn Fn() -> Result<(), tcl_cmd_core::CmdError>> =
                std::rc::Rc::new(move || current_selected.check(&current_original));
            let dialect = self.native_invocation_dialect();
            let object_current = current.clone();
            let string_original = original.clone();
            let refuse_original = original.clone();
            let retain_original = original.clone();
            let objects = RuntimeAppendObjects {
                dialect,
                binary_recipe: dialect.byte_array_string_recipe(None),
            }
            .with_original_checks(
                move || object_current(),
                move |value, extra| string_original.bytes(value, extra),
                move |error| refuse_original.refuse(error),
                move |error| retain_original.retain(error),
            );
            let entered_original = original.clone();
            let entered_selected = selected.clone();
            let select = selected.clone();
            let refuse_original = original.clone();
            let owner = OriginalAppendVariable {
                current: current.clone(),
                entered: std::rc::Rc::new(move || entered_selected.entered(&entered_original)),
                select: std::rc::Rc::new(move |value| select.expected.set(value)),
                refuse: std::rc::Rc::new(move |error| refuse_original.refuse(error)),
            };
            let code = self.append_retained_operands_using(
                &name,
                AppendReceiverReference::Original(&selected.receiver),
                std::borrow::Cow::Owned(home),
                access,
                protocol,
                sources,
                objects,
                Some(owner),
            );
            drop(current);
            drop(selected);
            original
                .check()
                .map_err(|first| self.report_cmd_error(first))?;
            Ok(code)
        })();
        match original.check() {
            Ok(()) => reached.unwrap_or_else(|code| code),
            Err(first) => self.report_cmd_error(first),
        }
    }

    pub(crate) fn append_native_operands(
        &mut self,
        name: &[u8],
        base: &[u8],
        element: Option<&[u8]>,
        sources: &[*mut TclObj],
    ) -> Code {
        let dialect = self.native_invocation_dialect();
        let issued =
            match dialect.native_object_append_protocol(Some(
                tcl_registry::native_object_append::LogicalAppendProvider::Tcl84CoreSimulation,
            )) {
                Some(issued) => issued,
                None => return self.refuse_native_access(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "native object append",
                    ),
                ),
            };
        if issued.logical_provider().is_none() {
            return self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "explicit logical append provider",
                ),
            );
        }
        let receiver_result = crate::vars::capture_variable_receiver(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            base,
            element,
        );
        let receiver = match receiver_result {
            Ok(receiver) => receiver,
            Err(error) => return crate::builtins::var_error(self, name, error),
        };
        let home = self.trace_identity(base);
        let access = self.trace_access(name, base, element, &home, false);
        self.append_retained_operands(name, receiver, home, access, issued.recipe(), sources)
    }

    /// Same actual captured receiver and Core append worker under the caller's
    /// original frame/operand/currentness owner, without another name lookup.
    pub(super) fn append_native_captured_using(
        &mut self,
        captured: &super::native_variable_names::OriginalCVariableCapture,
        sources: &[*mut TclObj],
        objects: crate::value_ops::OriginalRuntimeAppendObjects,
        original: OriginalAppendVariable,
    ) -> Code {
        let Some(protocol) = self
            .native_invocation_dialect()
            .native_object_append_protocol(None)
        else {
            return self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "native compiled append",
                ),
            );
        };
        let access = self.trace_access(
            &captured.root,
            &captured.root,
            captured.element.as_deref(),
            &captured.home,
            false,
        );
        self.append_retained_operands_using(
            &captured.root,
            AppendReceiverReference::Original(&captured.receiver),
            std::borrow::Cow::Borrowed(&captured.home),
            access,
            protocol.recipe(),
            sources,
            objects,
            Some(original),
        )
    }

    fn append_retained_operands(
        &mut self,
        name: &[u8],
        receiver: VariableReceiver,
        home: crate::vars::TraceHome,
        access: TraceAccess,
        protocol: NativeObjectAppendProtocol,
        sources: &[*mut TclObj],
    ) -> Code {
        if receiver.is_constant() {
            return crate::builtins::var_error(self, name, crate::frame::VarError::IsConstant);
        }
        if receiver.is_array() && !receiver.is_element() {
            return crate::builtins::var_error(self, name, crate::frame::VarError::IsArray);
        }
        let dialect = self.native_invocation_dialect();
        let objects = RuntimeAppendObjects {
            dialect,
            binary_recipe: dialect.byte_array_string_recipe(Some(tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation)),
        };
        self.append_retained_operands_using(
            name,
            AppendReceiverReference::Owned(receiver),
            std::borrow::Cow::Owned(home),
            access,
            protocol,
            sources,
            objects,
            None,
        )
    }

    fn append_retained_operands_using<'a, O: VariableAppendObjects>(
        &'a mut self,
        name: &'a [u8],
        receiver: AppendReceiverReference<'a>,
        home: std::borrow::Cow<'a, crate::vars::TraceHome>,
        access: TraceAccess,
        protocol: NativeObjectAppendProtocol,
        sources: &[*mut TclObj],
        objects: O,
        original: Option<OriginalAppendVariable>,
    ) -> Code {
        if let Some(original) = &original {
            if let Err(first) = (original.current)() {
                return self.report_cmd_error(first);
            }
        }
        if receiver.is_constant() {
            return crate::builtins::var_error(self, name, crate::frame::VarError::IsConstant);
        }
        if receiver.is_array() && !receiver.is_element() {
            return crate::builtins::var_error(self, name, crate::frame::VarError::IsArray);
        }
        let sources = sources
            .iter()
            .copied()
            .map(RuntimeAppendValue::borrowed)
            .collect::<Vec<_>>();
        let mut operation = AppendVariable {
            interp: self,
            name,
            receiver,
            home,
            access,
            protocol,
            objects,
            original,
            pending: None,
        };
        let reached = append_operands(&mut operation, protocol, &sources);
        let reached = match reached {
            Ok(value) => {
                operation.interp.set_result(value.as_ptr());
                let current = operation.check_original(true);
                drop(value);
                match current {
                    Ok(()) => Code::Ok,
                    Err(first) => first,
                }
            }
            Err(first) => first,
        };
        // Mandatory real working cleanup occurs before the same original
        // postcheck, including failures before actual variable publication.
        if operation.original.is_some() {
            drop(operation.pending.take());
        }
        // The neutral field drop sequence remains owned receiver/home before
        // its unpublished working value. Original cleanup checks its borrower.
        match operation.check_original(true) {
            Ok(()) => reached,
            Err(first) => first,
        }
    }
}

#[cfg(test)]
mod original_c84_counted_append_tests {
    use super::*;
    use crate::interp::{
        default_host, native_operation_currency::NativeOperationScope, WeakInterp, GLOBAL,
    };
    use crate::obj;
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };
    use tcl_runtime_api::NativeExecutionError;

    struct Observer {
        interpreter: WeakInterp,
        action: u8,
        getter: bool,
        arm: Rc<Cell<bool>>,
        events: Rc<RefCell<Vec<(bool, obj::TclSize)>>>,
        publication: Rc<Cell<usize>>,
        in_progress: Rc<Cell<bool>>,
    }
    fn observe(state: &Observer, value: *mut TclObj, getter: bool) {
        if !state.arm.get() {
            return;
        }
        state
            .events
            .borrow_mut()
            .push((getter, unsafe { (*value).ref_count }));
        let Some(mut interpreter) = state.interpreter.upgrade() else {
            return;
        };
        // Read actual storage/flags without issuing another name or String getter.
        state.publication.set(
            crate::vars::get_at(
                &interpreter.frames.borrow(),
                &interpreter.namespaces.borrow(),
                b"::errorInfo",
                0,
            )
            .map_or(0, |value| value as usize),
        );
        state
            .in_progress
            .set(interpreter.exc.borrow_mut().native.legacy_copy);
        if getter != state.getter {
            return;
        }
        match state.action {
            1 => {
                interpreter.refuse_host_command("original C84 counted append callback");
            }
            2 => {
                let old = interpreter.runtime_context();
                let mut changed = old.clone();
                changed.packages = vec![("original-c84-counted-append".into(), "1.0".into())];
                interpreter.pin_context(&changed).unwrap();
                interpreter.pin_context(&old).unwrap();
            }
            3 => {
                let next = obj::Owned::fresh(obj::new_string_bytes(b"REACHED REPLACEMENT"));
                interpreter
                    .store_captured_byte_variable(GLOBAL, None, b"::errorInfo", None, next.as_ptr())
                    .unwrap();
            }
            _ => {}
        }
    }
    extern "C" fn update(value: *mut TclObj) {
        let state = unsafe { &*(obj::internal_rep(value) as usize as *const Observer) };
        unsafe { obj::set_native_updater_string_rep(value, b"SEED", false) };
        observe(state, value, true);
    }
    extern "C" fn retire(value: *mut TclObj) {
        let state = unsafe { Box::from_raw(obj::internal_rep(value) as usize as *mut Observer) };
        observe(&state, value, false);
    }
    static ORIGINAL: obj::TclObjType = obj::TclObjType {
        name: c"originalC84CountedAppend".as_ptr(),
        free_int_rep_proc: Some(retire),
        dup_int_rep_proc: None,
        update_string_proc: Some(update),
        set_from_any_proc: None,
    };
    fn native() -> Interp {
        let environment =
            tcl_registry::model::ingress::resolve_known_environment("tcl8.4").unwrap();
        Interp::with_native_core(
            default_host(),
            environment.unit_profile(),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }
    fn original(
        interpreter: &Interp,
        action: u8,
        getter: bool,
        arm: &Rc<Cell<bool>>,
        events: &Rc<RefCell<Vec<(bool, obj::TclSize)>>>,
        publication: &Rc<Cell<usize>>,
        in_progress: &Rc<Cell<bool>>,
    ) -> obj::Owned {
        let state = Box::new(Observer {
            interpreter: interpreter.downgrade(),
            action,
            getter,
            arm: arm.clone(),
            events: events.clone(),
            publication: publication.clone(),
            in_progress: in_progress.clone(),
        });
        obj::Owned::fresh(obj::alloc_typed(
            &ORIGINAL,
            Box::into_raw(state) as usize as u64,
        ))
    }
    fn resident(interpreter: &Interp, name: &[u8]) -> Option<*mut TclObj> {
        crate::vars::get_at(
            &interpreter.frames.borrow(),
            &interpreter.namespaces.borrow(),
            name,
            0,
        )
    }
    fn assert_refusal(interpreter: &Interp, scope: &NativeOperationScope, action: u8) {
        let first = interpreter.native_execution_refusal().unwrap();
        assert_eq!(
            matches!(first, NativeExecutionError::HostCommandRefusal(_)),
            action == 1
        );
        assert_eq!(
            scope.currency().ensure_current_or_refuse().err(),
            Some(first)
        );
    }

    #[test]
    fn original_c84_seed_getter_refuses_before_any_global_setter() {
        // naming.embedding.original-host-publication-and-fact-transport

        // Allocation-only original result observation and software first-cause transport.
        for action in 1..3 {
            crate::counters::reset();
            {
                let mut interpreter = native();
                let arm = Rc::new(Cell::new(false));
                let events = Rc::new(RefCell::new(Vec::new()));
                let publication = Rc::new(Cell::new(0));
                let in_progress = Rc::new(Cell::new(false));
                let value = original(
                    &interpreter,
                    action,
                    true,
                    &arm,
                    &events,
                    &publication,
                    &in_progress,
                );
                interpreter.set_result(value.as_ptr());
                drop(value);
                let prior = resident(&interpreter, b"::errorInfo");
                let prior_code = resident(&interpreter, b"::errorCode");
                interpreter.exc.borrow_mut().info = Some(b"SEED\0\xff COUNTED".to_vec());
                let scope = NativeOperationScope::enter(&interpreter).unwrap();
                arm.set(true);
                assert!(interpreter
                    .update_native_error_info_checked(scope.currency())
                    .is_err());
                assert_eq!(events.borrow().as_slice(), &[(true, 1)]);
                assert_eq!(resident(&interpreter, b"::errorInfo"), prior);
                assert_eq!(resident(&interpreter, b"::errorCode"), prior_code);
                assert!(!interpreter.exc.borrow().native.legacy_copy);
                assert_refusal(&interpreter, &scope, action);
                arm.set(false);
                drop(scope);
            }
            assert_eq!(crate::counters::finalize(), 0);
        }
    }

    #[test]
    fn original_c84_counted_worker_checks_getter_and_primary_retirement_before_write() {
        // naming.error.c84-publication-order

        // naming.embedding.original-host-publication-and-fact-transport

        // Runtime updater/free observations see the genuine cell-only +1;
        // software Host/ABA behaviour is separate from external C84 observations.
        for getter in [true, false] {
            for action in 0..3 {
                crate::counters::reset();
                {
                    let mut interpreter = native();
                    let arm = Rc::new(Cell::new(false));
                    let events = Rc::new(RefCell::new(Vec::new()));
                    let publication = Rc::new(Cell::new(0));
                    let in_progress = Rc::new(Cell::new(false));
                    let value = original(
                        &interpreter,
                        action,
                        getter,
                        &arm,
                        &events,
                        &publication,
                        &in_progress,
                    );
                    let pointer = value.as_ptr();
                    interpreter
                        .store_captured_byte_variable(GLOBAL, None, b"::errorInfo", None, pointer)
                        .unwrap();
                    drop(value);
                    interpreter.set_result_bytes(b"PRIOR RESULT");
                    let prior_result = interpreter.result_obj();
                    {
                        let mut exc = interpreter.exc.borrow_mut();
                        exc.info = Some(b"SEED\0\xff COUNTED".to_vec());
                        exc.native.info_len = 4;
                        exc.native.legacy_copy = true;
                    }
                    let scope = NativeOperationScope::enter(&interpreter).unwrap();
                    arm.set(true);
                    let reached = interpreter.update_native_error_info_checked(scope.currency());
                    assert_eq!(events.borrow().first().copied(), Some((true, 1)));
                    assert_eq!(publication.get(), pointer as usize);
                    assert_eq!(resident(&interpreter, b"::errorInfo"), Some(pointer));
                    assert_eq!(interpreter.result_obj(), prior_result);
                    assert!(interpreter.exc.borrow().native.legacy_copy);
                    assert_eq!(
                        interpreter.exc.borrow().native.info_len,
                        b"SEED\0\xff COUNTED".len()
                    );
                    if action == 0 {
                        reached.unwrap();
                        assert_eq!(obj::bytes_of(pointer), b"SEED\0\xff COUNTED");
                        assert_eq!(events.borrow().as_slice(), &[(true, 1), (false, 1)]);
                        assert!(interpreter.native_execution_refusal().is_none());
                        assert_eq!(unsafe { (*pointer).ref_count }, 1);
                    } else {
                        assert!(reached.is_err());
                        assert_eq!(events.borrow().len(), if getter { 1 } else { 2 });
                        assert_refusal(&interpreter, &scope, action);
                    }
                    arm.set(false);
                    drop(scope);
                }
                assert_eq!(crate::counters::finalize(), 0);
                assert_eq!(crate::counters::double_free_count(), 0);
            }
        }
    }

    #[test]
    fn original_c84_changed_same_cell_cannot_combine_old_append_bytes_with_new_value() {
        // naming.embedding.original-host-publication-and-fact-transport

        // A genuine legal cell replacement keeps context current; the prepared original header still refuses.
        for append in [true, false] {
            crate::counters::reset();
            {
                let mut interpreter = native();
                let arm = Rc::new(Cell::new(false));
                let events = Rc::new(RefCell::new(Vec::new()));
                let publication = Rc::new(Cell::new(0));
                let in_progress = Rc::new(Cell::new(false));
                let value = original(
                    &interpreter,
                    3,
                    true,
                    &arm,
                    &events,
                    &publication,
                    &in_progress,
                );
                let old = value.as_ptr();
                interpreter
                    .store_captured_byte_variable(GLOBAL, None, b"::errorInfo", None, old)
                    .unwrap();
                drop(value);
                interpreter.set_result_bytes(b"PRIOR RESULT");
                let prior = interpreter.result_obj();
                {
                    let mut exc = interpreter.exc.borrow_mut();
                    exc.info = Some(
                        if append {
                            b"SEED APPEND".as_slice()
                        } else {
                            b"SEED"
                        }
                        .to_vec(),
                    );
                    exc.native.info_len = 4;
                    exc.native.legacy_copy = true;
                }
                let scope = NativeOperationScope::enter(&interpreter).unwrap();
                arm.set(true);
                assert!(interpreter
                    .update_native_error_info_checked(scope.currency())
                    .is_err());
                let next = resident(&interpreter, b"::errorInfo").unwrap();
                assert_ne!(next, old);
                assert_eq!(obj::bytes_of(next), b"REACHED REPLACEMENT");
                assert_eq!(interpreter.result_obj(), prior);
                assert!(matches!(
                    interpreter.native_execution_refusal(),
                    Some(NativeExecutionError::ValueAccessRefusal(_))
                ));
                assert_eq!(
                    scope.currency().ensure_current_or_refuse().err(),
                    interpreter.native_execution_refusal()
                );
                arm.set(false);
                drop(scope);
            }
            assert_eq!(crate::counters::finalize(), 0);
        }
    }

    #[test]
    fn original_c84_error_caller_preserves_reached_seed_and_blocks_later_code_and_result() {
        // naming.error.c84-publication-order

        // naming.embedding.original-host-publication-and-fact-transport

        // Actual installed error worker under its original entry; no modern order or free-callback getter grant.
        for action in 0..3 {
            crate::counters::reset();
            {
                let mut interpreter = native();
                let words: Vec<_> = [
                    b"error".as_slice(),
                    b"FINAL MESSAGE",
                    b" INFO",
                    b"EXPLICIT CODE",
                ]
                .into_iter()
                .map(|bytes| obj::Owned::fresh(obj::new_string_bytes(bytes)))
                .collect();
                let (command, _) = interpreter
                    .resolve_original_command_at(GLOBAL, words[0].as_ptr())
                    .unwrap()
                    .unwrap();
                let crate::interp::Command::Builtin(worker) = command else {
                    panic!("actual installed error worker");
                };
                let arm = Rc::new(Cell::new(false));
                let events = Rc::new(RefCell::new(Vec::new()));
                let publication = Rc::new(Cell::new(0));
                let in_progress = Rc::new(Cell::new(false));
                let value = original(
                    &interpreter,
                    action,
                    false,
                    &arm,
                    &events,
                    &publication,
                    &in_progress,
                );
                interpreter
                    .store_captured_byte_variable(
                        GLOBAL,
                        None,
                        b"::errorInfo",
                        None,
                        value.as_ptr(),
                    )
                    .unwrap();
                drop(value);
                let prior_code = obj::Owned::fresh(obj::new_string_bytes(b"PRIOR CODE"));
                interpreter
                    .store_captured_byte_variable(
                        GLOBAL,
                        None,
                        b"::errorCode",
                        None,
                        prior_code.as_ptr(),
                    )
                    .unwrap();
                interpreter.set_result_bytes(b"SEED");
                let seed = interpreter.result_obj();
                let scope = NativeOperationScope::enter(&interpreter).unwrap();
                arm.set(true);
                let argv: Vec<_> = words.iter().map(obj::Owned::as_ptr).collect();
                assert_eq!(worker(&mut interpreter, &argv), Code::Error);
                assert_eq!(events.borrow().as_slice(), &[(false, 0)]);
                assert_eq!(publication.get(), seed as usize);
                assert!(in_progress.get());
                assert!(interpreter.exc.borrow().native.legacy_copy);
                if action == 0 {
                    assert_eq!(
                        obj::bytes_of(resident(&interpreter, b"::errorInfo").unwrap()),
                        b"SEED INFO"
                    );
                    assert_eq!(
                        resident(&interpreter, b"::errorCode"),
                        Some(words[3].as_ptr())
                    );
                    assert_eq!(interpreter.result_obj(), words[1].as_ptr());
                    assert!(interpreter.exc.borrow().native.global_code_set);
                    assert!(interpreter.native_execution_refusal().is_none());
                } else {
                    assert_eq!(resident(&interpreter, b"::errorInfo"), Some(seed));
                    assert_eq!(obj::bytes_of(seed), b"SEED");
                    assert_eq!(
                        resident(&interpreter, b"::errorCode"),
                        Some(prior_code.as_ptr())
                    );
                    assert_eq!(interpreter.result_obj(), seed);
                    assert!(!interpreter.exc.borrow().native.global_code_set);
                    assert_eq!(interpreter.exc.borrow().native.info_len, 4);
                    assert_refusal(&interpreter, &scope, action);
                }
                arm.set(false);
                drop(scope);
            }
            assert_eq!(crate::counters::finalize(), 0);
        }
    }

    thread_local! {
        static WRITE_MODE: Cell<u8> = const { Cell::new(0) };
        static WRITE_ROWS: RefCell<Vec<(obj::TclSize, Vec<u8>)>> = const { RefCell::new(Vec::new()) };
        static LATER_GETTERS: Cell<usize> = const { Cell::new(0) };
    }
    struct WriteOwner;
    impl Drop for WriteOwner {
        fn drop(&mut self) {
            WRITE_MODE.with(|mode| mode.set(0));
            WRITE_ROWS.with(|rows| rows.borrow_mut().clear());
            LATER_GETTERS.with(|calls| calls.set(0));
        }
    }
    extern "C" fn later_update(value: *mut TclObj) {
        LATER_GETTERS.with(|calls| calls.set(calls.get() + 1));
        unsafe { obj::set_native_updater_string_rep(value, b"LATER GETTER", false) };
    }
    static LATER: obj::TclObjType = obj::TclObjType {
        name: c"originalC84LaterGetter".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(later_update),
        set_from_any_proc: None,
    };
    fn write(interpreter: &mut Interp, _: &[*mut TclObj]) -> Code {
        let value = resident(interpreter, b"::errorInfo").unwrap();
        assert!(obj::has_string_rep(value));
        WRITE_ROWS.with(|rows| {
            rows.borrow_mut()
                .push((unsafe { (*value).ref_count }, obj::bytes_of(value)))
        });
        let mode = WRITE_MODE.with(Cell::get);
        if matches!(mode, 1 | 2) {
            let next = obj::Owned::fresh(obj::alloc_typed(&LATER, 0));
            interpreter
                .store_captured_byte_variable(GLOBAL, None, b"::errorInfo", None, next.as_ptr())
                .unwrap();
        }
        match mode {
            1 => interpreter.refuse_host_command("original C84 write callback"),
            2 => {
                let old = interpreter.runtime_context();
                let mut changed = old.clone();
                changed.packages = vec![("original-c84-write".into(), "1.0".into())];
                interpreter.pin_context(&changed).unwrap();
                interpreter.pin_context(&old).unwrap();
                Code::Ok
            }
            3 => {
                interpreter.set_result_bytes(b"GENUINE GUEST CALLBACK ERROR");
                Code::Error
            }
            _ => Code::Ok,
        }
    }
    #[test]
    fn original_c84_write_outcome_settles_flags_before_refusing_later_getters() {
        // naming.error.nonempty-info-publication-order

        // naming.embedding.original-host-publication-and-fact-transport

        // Genuine cached counted write and callback effects; Guest-ignore/Host-stop is software transport.
        for mode in 0..4 {
            crate::counters::reset();
            {
                let owner = WriteOwner;
                let mut interpreter = native();
                interpreter.register_builtin(b"original_c84_write", write);
                assert_eq!(
                    interpreter.eval_str(b"trace variable ::errorInfo w original_c84_write"),
                    Code::Ok
                );
                let seed = obj::Owned::fresh(obj::new_string_bytes(b"SEED"));
                interpreter
                    .store_captured_byte_variable(GLOBAL, None, b"::errorInfo", None, seed.as_ptr())
                    .unwrap();
                drop(seed);
                interpreter.set_result_bytes(b"PRIOR RESULT");
                let prior = interpreter.result_obj();
                {
                    let mut exc = interpreter.exc.borrow_mut();
                    exc.info = Some(b"SEED\0\xff COUNTED".to_vec());
                    exc.native.info_len = 4;
                    exc.native.legacy_copy = true;
                    exc.already_logged = true;
                }
                WRITE_ROWS.with(|rows| rows.borrow_mut().clear());
                LATER_GETTERS.with(|calls| calls.set(0));
                WRITE_MODE.with(|value| value.set(mode));
                let scope = NativeOperationScope::enter(&interpreter).unwrap();
                let reached = interpreter.update_native_error_info_checked(scope.currency());
                WRITE_ROWS.with(|rows| {
                    assert_eq!(
                        rows.borrow().as_slice(),
                        &[(1, b"SEED\0\xff COUNTED".to_vec())]
                    )
                });
                assert_eq!(LATER_GETTERS.with(Cell::get), 0);
                assert_eq!(interpreter.result_obj(), prior);
                assert!(interpreter.exc.borrow().native.legacy_copy);
                assert!(interpreter.exc.borrow().already_logged);
                assert_eq!(
                    interpreter.exc.borrow().native.info_len,
                    b"SEED\0\xff COUNTED".len()
                );
                if matches!(mode, 1 | 2) {
                    assert!(reached.is_err());
                    let value = resident(&interpreter, b"::errorInfo").unwrap();
                    assert_eq!(obj::obj_type_ptr(value), &LATER as *const obj::TclObjType);
                    assert!(!obj::has_string_rep(value));
                    assert_refusal(&interpreter, &scope, mode);
                } else {
                    reached.unwrap();
                    assert_eq!(
                        obj::bytes_of(resident(&interpreter, b"::errorInfo").unwrap()),
                        b"SEED\0\xff COUNTED"
                    );
                    assert!(interpreter.native_execution_refusal().is_none());
                }
                assert!(interpreter.active_var_trace_scopes.borrow().is_empty());
                drop(scope);
                drop(owner);
            }
            assert_eq!(crate::counters::finalize(), 0);
        }
    }
}

#[cfg(test)]
mod original_c84_pending_scalar_tests {
    use super::*;
    use crate::interp::{
        default_host, native_operation_currency::NativeOperationScope, WeakInterp, GLOBAL,
    };
    use crate::obj;
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };
    use tcl_runtime_api::NativeExecutionError;

    struct Episode {
        interpreter: WeakInterp,
        caller: Rc<RefCell<Option<obj::Owned>>>,
        external: Rc<RefCell<Option<obj::Owned>>>,
        arm: Rc<Cell<bool>>,
        action: u8,
        copy: bool,
        stages: Rc<RefCell<Vec<(&'static str, obj::TclSize, Option<obj::TclSize>)>>>,
    }
    fn raw_publication(interpreter: &Interp) -> Option<*mut TclObj> {
        interpreter
            .namespaces
            .borrow()
            .original_variable_table_at(GLOBAL)
            .and_then(|table| table.published_scalar_header(b"errorInfo"))
    }
    extern "C" fn duplicate(original: *mut TclObj, copy: *mut TclObj) {
        let episode = unsafe { &*(obj::internal_rep(original) as usize as *const Episode) };
        let incoming = Box::new(Episode {
            interpreter: episode.interpreter.clone(),
            caller: episode.caller.clone(),
            external: episode.external.clone(),
            arm: episode.arm.clone(),
            action: episode.action,
            copy: true,
            stages: episode.stages.clone(),
        });
        unsafe {
            (*copy).type_ptr = &ORIGINAL;
            (*copy).internal_rep = Box::into_raw(incoming) as usize as u64;
        }
        if episode.arm.get() {
            episode.stages.borrow_mut().push((
                "duplicate",
                unsafe { (*original).ref_count },
                Some(unsafe { (*copy).ref_count }),
            ));
            // This is an actual independent caller reference, not a work guard.
            // The shared decision was made at +2; its reached retirement leaves
            // the old cell's sole +1 for the real post-publication decrement.
            let caller = episode.caller.borrow_mut().take();
            drop(caller);
        }
    }
    extern "C" fn update(original: *mut TclObj) {
        let episode = unsafe { &*(obj::internal_rep(original) as usize as *const Episode) };
        if episode.arm.get() {
            episode
                .stages
                .borrow_mut()
                .push(("update", unsafe { (*original).ref_count }, None));
        }
        unsafe { obj::set_native_updater_string_rep(original, b"SEED", false) };
    }
    extern "C" fn retire(original: *mut TclObj) {
        let episode =
            unsafe { Box::from_raw(obj::internal_rep(original) as usize as *mut Episode) };
        if !episode.arm.get() {
            return;
        }
        let Some(mut interpreter) = episode.interpreter.upgrade() else {
            return;
        };
        let published = raw_publication(&interpreter);
        let refs = published.map(|published| unsafe { (*published).ref_count });
        episode.stages.borrow_mut().push((
            if episode.copy {
                "copy-free"
            } else {
                "old-free"
            },
            unsafe { (*original).ref_count },
            refs,
        ));
        if episode.copy {
            return;
        }
        match episode.action {
            1 => {
                interpreter.refuse_host_command("original C84 pending cell retirement");
            }
            2 => {
                let prior = interpreter.runtime_context();
                let mut changed = prior.clone();
                changed.packages = vec![("original-c84-pending-cell".into(), "1.0".into())];
                interpreter.pin_context(&changed).unwrap();
                interpreter.pin_context(&prior).unwrap();
            }
            3 | 5 => {
                if episode.action == 5 {
                    // A genuine callback-created external native role survives
                    // withdrawal of the pending allocation's non-owning cell role.
                    *episode.external.borrow_mut() = Some(obj::Owned::retain(published.unwrap()));
                }
                let next = obj::Owned::fresh(obj::new_string_bytes(b"REACHED REPLACEMENT"));
                interpreter
                    .store_captured_byte_variable(GLOBAL, None, b"::errorInfo", None, next.as_ptr())
                    .unwrap();
            }
            4 => {
                let retired = {
                    let mut namespaces = interpreter.namespaces.borrow_mut();
                    let table = namespaces.var_table_mut(GLOBAL);
                    let binding = table.binding_id(b"errorInfo").unwrap();
                    let (reached, retired) =
                        table.publish_namespace_binding_retirement(b"errorInfo", binding);
                    assert!(reached);
                    retired
                };
                // Native rc0 retirement happens outside namespace/cell borrows.
                drop(retired);
            }
            6..=11 => {
                match episode.action {
                    6 => assert_eq!(
                        interpreter.read_captured_byte_variable(
                            GLOBAL,
                            None,
                            b"::errorInfo",
                            None,
                            false
                        ),
                        Err(crate::frame::VarError::NameProtocolUnavailable),
                    ),
                    7 => assert!(!interpreter.var_exists(b"::errorInfo")),
                    8 => assert!(!interpreter.var_is_array(b"::errorInfo")),
                    9 => assert!(interpreter.array_names(b"::errorInfo").is_none()),
                    10 => assert!(interpreter
                        .with_captured_byte_variable(
                            GLOBAL,
                            None,
                            b"::errorInfo",
                            None,
                            false,
                            |place, currency| place.ensure_array(&interpreter, currency),
                        )
                        .is_err()),
                    _ => assert_eq!(
                        interpreter.read_captured_byte_variable(
                            GLOBAL,
                            None,
                            b"observedPending",
                            None,
                            false
                        ),
                        Err(crate::frame::VarError::NameProtocolUnavailable),
                    ),
                }
                // Legacy bool/Option ports are only unwind carriers; each
                // actual entered owner retains the full terminal role refusal.
                assert!(matches!(
                    interpreter.native_execution_refusal(),
                    Some(NativeExecutionError::ValueAccessRefusal(_))
                ));
            }
            _ => {}
        }
    }
    static ORIGINAL: obj::TclObjType = obj::TclObjType {
        name: c"originalC84PendingScalar".as_ptr(),
        free_int_rep_proc: Some(retire),
        dup_int_rep_proc: Some(duplicate),
        update_string_proc: Some(update),
        set_from_any_proc: None,
    };

    #[test]
    fn original_c84_cow_publishes_pending_allocation_and_settles_only_the_same_live_role() {
        // naming.error.c84-publication-order

        // naming.embedding.original-host-publication-and-fact-transport

        // Selected original C84 publication/refcount stages plus software refusal;
        // no external callback/Host/teardown equivalence is inferred.
        for action in 0..12 {
            crate::counters::reset();
            {
                let environment =
                    tcl_registry::model::ingress::resolve_known_environment("tcl8.4").unwrap();
                let mut interpreter = Interp::with_native_core(
                    default_host(),
                    environment.unit_profile(),
                    tcl_registry::special_vars::NativeBootstrapInputs::default(),
                )
                .unwrap();
                let caller = Rc::new(RefCell::new(None));
                let external = Rc::new(RefCell::new(None));
                let arm = Rc::new(Cell::new(false));
                let stages = Rc::new(RefCell::new(Vec::new()));
                let episode = Box::new(Episode {
                    interpreter: interpreter.downgrade(),
                    caller: caller.clone(),
                    external: external.clone(),
                    arm: arm.clone(),
                    action,
                    copy: false,
                    stages: stages.clone(),
                });
                let original = obj::Owned::fresh(obj::alloc_typed(
                    &ORIGINAL,
                    Box::into_raw(episode) as usize as u64,
                ));
                let old = original.as_ptr();
                interpreter
                    .store_captured_byte_variable(GLOBAL, None, b"::errorInfo", None, old)
                    .unwrap();
                *caller.borrow_mut() = Some(original);
                if action == 11 {
                    assert_eq!(
                        interpreter.eval_str(b"upvar #0 ::errorInfo observedPending"),
                        Code::Ok
                    );
                }
                interpreter.set_result_bytes(b"PRIOR RESULT");
                let prior = interpreter.result_obj();
                {
                    let mut exception = interpreter.exc.borrow_mut();
                    exception.info = Some(b"SEED\0\xff COUNTED".to_vec());
                    exception.native.info_len = 4;
                    exception.native.legacy_copy = true;
                }
                let scope = NativeOperationScope::enter(&interpreter).unwrap();
                arm.set(true);
                let reached = interpreter.update_native_error_info_checked(scope.currency());
                assert_eq!(
                    stages.borrow().first().copied(),
                    Some(("duplicate", 2, Some(0)))
                );
                assert!(stages.borrow().contains(&("old-free", 0, Some(0))));
                assert!(caller.borrow().is_none());
                assert_eq!(interpreter.result_obj(), prior);
                if action == 0 {
                    reached.unwrap();
                    let current = raw_publication(&interpreter).unwrap();
                    assert_ne!(current, old);
                    assert_eq!(unsafe { (*current).ref_count }, 1);
                    assert!(stages.borrow().contains(&("update", 1, None)));
                    assert!(stages.borrow().contains(&("copy-free", 1, Some(1))));
                    assert_eq!(obj::bytes_of(current), b"SEED\0\xff COUNTED");
                    assert!(interpreter.native_execution_refusal().is_none());
                } else {
                    assert!(reached.is_err());
                    assert!(!stages
                        .borrow()
                        .iter()
                        .any(|(stage, _, _)| *stage == "update"));
                    let first = interpreter.native_execution_refusal().unwrap();
                    assert_eq!(
                        matches!(first, NativeExecutionError::HostCommandRefusal(_)),
                        action == 1
                    );
                    assert_eq!(
                        scope.currency().ensure_current_or_refuse().err(),
                        Some(first)
                    );
                    if matches!(action, 3 | 5) {
                        let current = raw_publication(&interpreter).unwrap();
                        assert_eq!(obj::bytes_of(current), b"REACHED REPLACEMENT");
                    } else {
                        assert_eq!(raw_publication(&interpreter), None);
                    }
                    if action == 5 {
                        let held = external.borrow();
                        let held = held.as_ref().unwrap();
                        assert!(obj::allocation_is_live(held.as_ptr()));
                        assert_eq!(unsafe { (*held.as_ptr()).ref_count }, 1);
                        assert!(!stages
                            .borrow()
                            .iter()
                            .any(|(stage, _, _)| *stage == "copy-free"));
                    } else {
                        assert!(stages
                            .borrow()
                            .iter()
                            .any(|(stage, refs, _)| *stage == "copy-free" && *refs == 0));
                    }
                }
                arm.set(false);
                let remaining_caller = caller.borrow_mut().take();
                drop(remaining_caller);
                let remaining_external = external.borrow_mut().take();
                drop(remaining_external);
                drop(scope);
            }
            assert_eq!(crate::counters::finalize(), 0);
            assert_eq!(crate::counters::double_free_count(), 0);
        }
    }

    thread_local! {
        static MISSING_WRITES: RefCell<Vec<(obj::TclSize, Vec<u8>)>> = const { RefCell::new(Vec::new()) };
    }
    struct MissingWriteOwner;
    impl Drop for MissingWriteOwner {
        fn drop(&mut self) {
            MISSING_WRITES.with(|rows| rows.borrow_mut().clear());
        }
    }
    fn missing_write(interpreter: &mut Interp, _argv: &[*mut TclObj]) -> Code {
        let current = raw_publication(interpreter).unwrap();
        let snapshot = obj::native_object_snapshot(current).unwrap();
        let bytes = snapshot
            .resident
            .map_or_else(Vec::new, |bytes| bytes.to_vec());
        MISSING_WRITES.with(|rows| {
            rows.borrow_mut()
                .push((unsafe { (*current).ref_count }, bytes))
        });
        Code::Ok
    }

    #[test]
    fn original_c84_missing_cell_adopts_counted_message_before_write_without_working_hold() {
        // naming.error.c84-publication-order

        // Selected C84 missing-cell source role; no later-version COW or Native teardown claim.
        crate::counters::reset();
        {
            let environment =
                tcl_registry::model::ingress::resolve_known_environment("tcl8.4").unwrap();
            let mut interpreter = Interp::with_native_core(
                default_host(),
                environment.unit_profile(),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let scope = NativeOperationScope::enter(&interpreter).unwrap();
            let initial = obj::Owned::fresh(obj::new_string_bytes(b"OLD CELL"));
            interpreter
                .store_captured_byte_variable(GLOBAL, None, b"::errorInfo", None, initial.as_ptr())
                .unwrap();
            drop(initial);
            assert!(interpreter.var_unset(b"::errorInfo"));
            assert_eq!(raw_publication(&interpreter), None);
            let trace_owner = MissingWriteOwner;
            MISSING_WRITES.with(|rows| rows.borrow_mut().clear());
            interpreter.register_builtin(b"original_c84_missing_write", missing_write);
            assert_eq!(
                interpreter.eval_str(b"trace variable ::errorInfo w original_c84_missing_write"),
                Code::Ok
            );
            interpreter.set_result_bytes(b"PRIOR RESULT");
            let prior = interpreter.result_obj();
            let name = obj::Owned::fresh(obj::new_string_bytes(b"::errorInfo"));
            let captured = interpreter
                .capture_original_c_variable_for(
                    name.as_ptr(),
                    tcl_syntax::native_variable_name::NativeVariableNameLookupPurpose::QuietWrite,
                )
                .unwrap();
            let reached = interpreter.append_native_counted_c_variable_bytes_checked(
                scope.currency(),
                captured,
                b"\0\xff COUNTED",
            );
            assert!(matches!(
                reached,
                Ok(crate::interp::VariableTraceGuestOutcome::Complete)
            ));
            let current = raw_publication(&interpreter).unwrap();
            assert_eq!(unsafe { (*current).ref_count }, 1);
            assert_eq!(obj::bytes_of(current), b"\0\xff COUNTED");
            MISSING_WRITES.with(|rows| {
                assert_eq!(rows.borrow().as_slice(), &[(1, b"\0\xff COUNTED".to_vec())])
            });
            assert_eq!(interpreter.result_obj(), prior);
            assert!(interpreter.native_execution_refusal().is_none());
            scope.currency().ensure_current_or_refuse().unwrap();
            drop(name);
            drop(trace_owner);
            drop(scope);
        }
        assert_eq!(crate::counters::finalize(), 0);
        assert_eq!(crate::counters::double_free_count(), 0);
    }
}

#[cfg(test)]
#[path = "native_append/original_command_boundary_tests.rs"]
mod original_command_boundary_tests;
