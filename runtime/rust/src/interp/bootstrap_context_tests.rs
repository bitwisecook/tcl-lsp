// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software controls for the actual entered Host/bootstrap owner. These do
//! not measure a native provider's constructor timing or root-cell identity.

use super::{native_operation_currency::NativeOperationScope, Code, Interp, WeakInterp};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use tcl_platform::{Env, Host, HostError};
use tcl_registry::special_vars::{NativeBootstrapInputs, NativeBootstrapProtocol};
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::raw_string::NativeValueAccessRefusal;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Never,
    Retirement,
    Encoding,
    Environment,
    Get,
    Vars,
}

struct CallbackState {
    owner: RefCell<Option<WeakInterp>>,
    replacement: RefCell<Option<Rc<dyn Host>>>,
    stage: Cell<Stage>,
    events: RefCell<Vec<String>>,
}

impl CallbackState {
    fn new() -> Self {
        Self {
            owner: RefCell::new(None),
            replacement: RefCell::new(None),
            stage: Cell::new(Stage::Never),
            events: RefCell::new(Vec::new()),
        }
    }

    fn reached(&self, stage: Stage, label: &str) {
        self.events.borrow_mut().push(label.to_owned());
        if self.stage.get() != stage {
            return;
        }
        self.stage.set(Stage::Never);
        // Drop both record borrows before the actual nested public setter.
        let owner = self.owner.borrow().clone().unwrap().upgrade().unwrap();
        let replacement = self.replacement.borrow().clone().unwrap();
        owner.set_host(replacement);
    }

    fn arm(&self, interp: &Interp, replacement: Rc<dyn Host>, stage: Stage) {
        *self.owner.borrow_mut() = Some(interp.downgrade());
        *self.replacement.borrow_mut() = Some(replacement);
        self.events.borrow_mut().clear();
        self.stage.set(stage);
    }
}

struct CallbackHost {
    base: Rc<dyn Host>,
    state: Rc<CallbackState>,
    user: &'static str,
}

impl CallbackHost {
    fn new(user: &'static str) -> Rc<Self> {
        Rc::new(Self {
            base: super::default_host(),
            state: Rc::new(CallbackState::new()),
            user,
        })
    }
}

impl Host for CallbackHost {
    fn capabilities(&self) -> tcl_platform::Capabilities {
        self.base.capabilities()
    }
    fn clock(&self) -> &dyn tcl_platform::Clock {
        self.base.clock()
    }
    fn stdio(&self) -> &dyn tcl_platform::StdIo {
        self.base.stdio()
    }
    fn env(&self) -> &dyn Env {
        self.state.reached(Stage::Environment, "env");
        self
    }
    fn system_encoding(&self) -> tcl_platform::SystemEncoding {
        self.state.reached(Stage::Encoding, "encoding");
        tcl_platform::SystemEncoding::Utf8
    }
}

struct RetiringHost {
    base: Rc<CallbackHost>,
    retirement: Rc<CallbackState>,
}

impl Host for RetiringHost {
    fn capabilities(&self) -> tcl_platform::Capabilities {
        self.base.capabilities()
    }
    fn clock(&self) -> &dyn tcl_platform::Clock {
        self.base.clock()
    }
    fn stdio(&self) -> &dyn tcl_platform::StdIo {
        self.base.stdio()
    }
    fn env(&self) -> &dyn Env {
        self.base.env()
    }
    fn system_encoding(&self) -> tcl_platform::SystemEncoding {
        self.base.system_encoding()
    }
}

impl Drop for RetiringHost {
    fn drop(&mut self) {
        if self.retirement.stage.get() == Stage::Retirement {
            self.retirement
                .reached(Stage::Retirement, "drop:previous-host");
        }
    }
}

impl Env for CallbackHost {
    fn get(&self, key: &str) -> Option<String> {
        self.state.reached(Stage::Get, &format!("get:{key}"));
        match key {
            "USER" => Some(self.user.into()),
            "TCL_LIBRARY" => Some(format!("/{}/library", self.user)),
            _ => None,
        }
    }
    fn set(&self, _: &str, _: &str) {}
    fn unset(&self, _: &str) {}
    fn vars(&self) -> Vec<(String, String)> {
        self.state.reached(Stage::Vars, "vars");
        vec![("BOOTSTRAP_USER".into(), self.user.into())]
    }
    fn cwd(&self) -> Result<String, HostError> {
        Ok("/bootstrap-control".into())
    }
    fn chdir(&self, _: &str) -> Result<(), HostError> {
        Ok(())
    }
}

fn native(host: Rc<dyn Host>, engine: &str) -> Interp {
    Interp::with_native_core(
        host,
        crate::environment::profile_for_dialect(engine),
        NativeBootstrapInputs::default(),
    )
    .unwrap()
}

fn protocol(interp: &Interp) -> NativeBootstrapProtocol {
    interp
        .native_invocation_dialect()
        .native_bootstrap_protocol()
        .unwrap()
}

fn stale() -> NativeExecutionError {
    NativeExecutionError::ValueAccessRefusal(NativeValueAccessRefusal::CommandProtocolUnavailable(
        "stale entered native operation",
    ))
}

fn root(interp: &Interp, name: &[u8]) -> Option<*mut crate::obj::TclObj> {
    crate::vars::get_at(
        &interp.frames.borrow(),
        &interp.namespaces.borrow(),
        name,
        0,
    )
}

fn element(interp: &Interp, name: &[u8], key: &[u8]) -> Option<*mut crate::obj::TclObj> {
    crate::vars::get_elem_at(
        &interp.frames.borrow(),
        &interp.namespaces.borrow(),
        name,
        key,
        0,
    )
}

#[test]
fn original_native_constructor_keeps_selected_inventory_and_checked_host_values() {
    // naming.bootstrap.original-root-cells
    // docs/design/analysis/name-resolution-proofs/bootstrap-original-root-cells.md
    // Actual software constructor, not an additional external-provider row.
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let host = CallbackHost::new("original");
        let interp = native(host.clone(), engine);
        assert!(interp.native_execution_refusal().is_none(), "{engine}");
        assert_eq!(
            super::obj_bytes(element(&interp, b"tcl_platform", b"user").unwrap()),
            b"original"
        );
        if engine == "jim" {
            assert!(root(&interp, b"env").is_none());
            assert_eq!(
                interp.get_obj_result(),
                interp
                    .native_jim_object_context()
                    .unwrap()
                    .empty_object()
                    .as_ptr()
            );
        } else {
            assert_eq!(
                super::obj_bytes(element(&interp, b"env", b"BOOTSTRAP_USER").unwrap()),
                b"original"
            );
            assert_eq!(
                super::obj_bytes(root(&interp, b"tcl_version").unwrap()),
                interp.runtime_version().version_string().as_bytes()
            );
            assert!(super::obj_bytes(interp.get_obj_result()).is_empty());
        }
        assert!(host
            .state
            .events
            .borrow()
            .iter()
            .any(|event| event == "vars"));
        assert!(host
            .state
            .events
            .borrow()
            .iter()
            .any(|event| event == "get:TCL_LIBRARY"));
        assert_eq!(interp.entered_native_operation_count(), 0);
    }
}

#[test]
fn original_native_bootstrap_stops_after_each_reached_host_callback_mutates_context() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Replay the selected bootstrap owner under a real retained interpreter;
    // this is not a fresh native birth or external callback-timing assertion.
    for stage in [Stage::Environment, Stage::Get, Stage::Vars] {
        let host = CallbackHost::new("before");
        let replacement = CallbackHost::new("after");
        let mut interp = native(host.clone(), "tcl8.6");
        let selected = protocol(&interp);
        let old = element(&interp, b"env", b"BOOTSTRAP_USER").unwrap();
        let old_platform = element(&interp, b"tcl_platform", b"user").unwrap();
        let errors = interp.native_error_cells.borrow().clone();
        let precision = interp.native_precision_cell.get();
        host.state.arm(&interp, replacement.clone(), stage);
        assert_eq!(
            interp.bootstrap_native_core(selected, &NativeBootstrapInputs::default()),
            Err(Code::Error),
            "{stage:?}"
        );
        assert_eq!(interp.native_execution_refusal(), Some(stale()));
        assert_eq!(element(&interp, b"env", b"BOOTSTRAP_USER"), Some(old));
        assert_eq!(
            element(&interp, b"tcl_platform", b"user"),
            Some(old_platform)
        );
        assert_eq!(*interp.native_error_cells.borrow(), errors);
        assert_eq!(interp.native_precision_cell.get(), precision);
        assert!(
            replacement.state.events.borrow().is_empty(),
            "nested setter must not bootstrap"
        );
        let reached = host.state.events.borrow();
        assert_eq!(
            reached.last().unwrap(),
            match stage {
                Stage::Environment => "env",
                Stage::Get => reached
                    .iter()
                    .find(|event| event.starts_with("get:"))
                    .unwrap(),
                Stage::Vars => "vars",
                _ => unreachable!(),
            }
        );
        assert!(!reached.iter().any(|event| event == "get:TCL_LIBRARY"));
        assert_eq!(interp.entered_native_operation_count(), 0);
    }
}

#[test]
fn original_host_setter_preserves_old_globals_when_a_reached_callback_changes_host() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for stage in [Stage::Encoding, Stage::Environment, Stage::Get, Stage::Vars] {
        let old_host = CallbackHost::new("before");
        let callback = CallbackHost::new("attempted");
        let replacement = CallbackHost::new("reached");
        let interp = native(old_host, "tcl8.6");
        let old = element(&interp, b"env", b"BOOTSTRAP_USER").unwrap();
        let old_platform = element(&interp, b"tcl_platform", b"user").unwrap();
        callback.state.arm(&interp, replacement.clone(), stage);
        interp.set_host(callback.clone());
        assert_eq!(
            interp.native_execution_refusal(),
            Some(stale()),
            "{stage:?}"
        );
        assert_eq!(element(&interp, b"env", b"BOOTSTRAP_USER"), Some(old));
        assert_eq!(
            element(&interp, b"tcl_platform", b"user"),
            Some(old_platform)
        );
        assert!(replacement.state.events.borrow().is_empty());
        let replacement_owner: Rc<dyn Host> = replacement.clone();
        assert!(
            Rc::ptr_eq(&interp.host(), &replacement_owner),
            "reached Host replacement remains"
        );
        assert!(!callback
            .state
            .events
            .borrow()
            .iter()
            .any(|event| event == "get:TCL_LIBRARY"));
        assert_eq!(interp.entered_native_operation_count(), 0);
    }
}

#[test]
fn original_bootstrap_and_setter_preserve_prior_host_refusal_without_callbacks() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let host = CallbackHost::new("before");
    let mut interp = native(host.clone(), "tcl8.6");
    let selected = protocol(&interp);
    interp.set_result_bytes(b"RETAINED RESULT");
    let result = interp.get_obj_result();
    let old = element(&interp, b"env", b"BOOTSTRAP_USER").unwrap();
    let first = NativeExecutionError::ValueAccessRefusal(
        NativeValueAccessRefusal::CommandProtocolUnavailable("original prior bootstrap cause"),
    );
    interp.refuse_native_execution(first.clone());
    host.state.events.borrow_mut().clear();
    assert_eq!(
        interp.bootstrap_native_core(selected, &NativeBootstrapInputs::default()),
        Err(Code::Error)
    );
    assert!(host.state.events.borrow().is_empty());
    assert_eq!(interp.get_obj_result(), result);
    assert_eq!(element(&interp, b"env", b"BOOTSTRAP_USER"), Some(old));
    let replacement = CallbackHost::new("after");
    interp.set_host(replacement.clone());
    assert!(replacement.state.events.borrow().is_empty());
    assert_eq!(interp.native_execution_refusal(), Some(first));
    assert_eq!(interp.get_obj_result(), result);
    assert_eq!(interp.entered_native_operation_count(), 0);
}

#[test]
fn original_ordinary_host_bootstrap_updates_snapshot_globals_without_refusal() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let before = CallbackHost::new("before");
    let after = CallbackHost::new("after");
    let interp = native(before, "tcl8.6");
    interp.set_host(after.clone());
    assert!(interp.native_execution_refusal().is_none());
    assert_eq!(
        super::obj_bytes(element(&interp, b"env", b"BOOTSTRAP_USER").unwrap()),
        b"after"
    );
    assert_eq!(
        super::obj_bytes(element(&interp, b"tcl_platform", b"user").unwrap()),
        b"after"
    );
    assert_eq!(
        super::obj_bytes(root(&interp, b"tcl_library").unwrap()),
        b"/after/library"
    );
    assert_eq!(
        after.state.events.borrow().last().map(String::as_str),
        Some("get:TCL_LIBRARY")
    );
    assert_eq!(interp.entered_native_operation_count(), 0);
}

#[test]
fn original_native_bootstrap_reports_storage_failure_and_stops_later_installation() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let host = CallbackHost::new("before");
    let mut interp = native(host, "tcl8.4");
    let selected = protocol(&interp);
    assert!(interp.quiet_var_unset_at(b"env", 0));
    let value = crate::obj::Owned::fresh(super::new_string(b"SCALAR ENV"));
    interp.var_set_at(b"env", value.as_ptr(), 0).unwrap();
    let precision = interp.native_precision_cell.get();
    assert_eq!(
        interp.bootstrap_native_core(selected, &NativeBootstrapInputs::default()),
        Err(Code::Error)
    );
    assert!(interp.native_execution_refusal().is_none());
    assert!(!super::obj_bytes(interp.get_obj_result()).is_empty());
    assert_eq!(root(&interp, b"env"), Some(value.as_ptr()));
    assert_eq!(interp.native_precision_cell.get(), precision);
    assert_eq!(interp.entered_native_operation_count(), 0);
}

#[test]
fn original_hidden_error_shell_installer_refuses_stale_entry_without_retirement() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut interp = native(CallbackHost::new("before"), "tcl8.6");
    let cells = interp.native_error_cells.borrow().clone();
    let scope = NativeOperationScope::enter(&interp).unwrap();
    interp.set_host(CallbackHost::new("after"));
    assert_eq!(
        interp.install_native_error_variable_traces_checked(),
        Err(Code::Error)
    );
    assert_eq!(*interp.native_error_cells.borrow(), cells);
    assert_eq!(interp.native_execution_refusal(), Some(stale()));
    assert!(scope.currency().ensure_current().is_err());
    drop(scope);
    assert_eq!(interp.entered_native_operation_count(), 0);
}

#[test]
fn original_previous_host_retirement_cannot_refresh_the_outer_setter_entry() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let before = CallbackHost::new("before");
    let retirement = Rc::new(CallbackState::new());
    let retiring = Rc::new(RetiringHost {
        base: before,
        retirement: retirement.clone(),
    });
    let requested = CallbackHost::new("requested");
    let reached = CallbackHost::new("reached");
    let interp = native(retiring.clone(), "tcl8.6");
    let old = element(&interp, b"env", b"BOOTSTRAP_USER").unwrap();
    let platform = element(&interp, b"tcl_platform", b"user").unwrap();
    retirement.arm(&interp, reached.clone(), Stage::Retirement);
    drop(retiring); // The interpreter now holds the sole actual Host Rc.
    interp.set_host(requested.clone());
    assert_eq!(&*retirement.events.borrow(), &["drop:previous-host"]);
    assert!(
        requested.state.events.borrow().is_empty(),
        "encoding must not read the superseded requested Host"
    );
    assert!(
        reached.state.events.borrow().is_empty(),
        "nested setter must not bootstrap"
    );
    let reached_owner: Rc<dyn Host> = reached;
    assert!(Rc::ptr_eq(&interp.host(), &reached_owner));
    assert_eq!(interp.native_execution_refusal(), Some(stale()));
    assert_eq!(element(&interp, b"env", b"BOOTSTRAP_USER"), Some(old));
    assert_eq!(element(&interp, b"tcl_platform", b"user"), Some(platform));
    assert_eq!(interp.entered_native_operation_count(), 0);
}

struct BootstrapObserver {
    replacement: Rc<dyn Host>,
    calls: Rc<Cell<usize>>,
    changes_host: bool,
}

impl tcl_runtime_api::native_variable_trace::NativeVariableObserver<Interp> for BootstrapObserver {
    type Error = tcl_cmd_core::CmdError;
    fn observe(
        &self,
        interp: &mut Interp,
        access: tcl_runtime_api::native_variable_trace::NativeVariableTraceAccess<'_>,
    ) -> Result<(), Self::Error> {
        assert_eq!(access.name1, b"env");
        assert_eq!(access.name2, b"BOOTSTRAP_USER");
        assert_eq!(
            access.operation,
            tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation::Write
        );
        self.calls.set(self.calls.get() + 1);
        if self.changes_host {
            interp.set_host(self.replacement.clone());
        }
        Ok(())
    }
}

#[test]
fn original_native_bootstrap_keeps_reached_write_and_stops_after_its_host_observer() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual software observer registration and entered publication, without
    // a new external C trace-order or constructor timing claim.
    use tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation;
    let mut interp = native(CallbackHost::new("snapshot"), "tcl8.6");
    let selected = protocol(&interp);
    let replacement = CallbackHost::new("after");
    let initial = crate::obj::Owned::fresh(super::new_string(b"INITIAL"));
    interp
        .var_set_elem_at(b"env", b"BOOTSTRAP_USER", initial.as_ptr(), 0)
        .unwrap();
    let name = crate::obj::Owned::fresh(super::new_string(b"env"));
    let first = Rc::new(Cell::new(0));
    let later = Rc::new(Cell::new(0));
    for (calls, changes_host) in [(later.clone(), false), (first.clone(), true)] {
        interp
            .add_native_variable_observer(
                name.as_ptr(),
                &[NativeVariableTraceOperation::Write],
                Rc::new(BootstrapObserver {
                    replacement: replacement.clone(),
                    calls,
                    changes_host,
                }),
            )
            .unwrap();
    }
    let platform = element(&interp, b"tcl_platform", b"user").unwrap();
    let errors = interp.native_error_cells.borrow().clone();
    let precision = interp.native_precision_cell.get();
    interp.set_result_bytes(b"RETAINED RESULT");
    let result = interp.get_obj_result();
    assert_eq!(
        interp.bootstrap_native_core(selected, &NativeBootstrapInputs::default()),
        Err(Code::Error)
    );
    assert_eq!(first.get(), 1);
    assert_eq!(later.get(), 0);
    assert_eq!(interp.native_execution_refusal(), Some(stale()));
    assert_eq!(
        super::obj_bytes(element(&interp, b"env", b"BOOTSTRAP_USER").unwrap()),
        b"snapshot"
    );
    assert_eq!(element(&interp, b"tcl_platform", b"user"), Some(platform));
    assert_eq!(*interp.native_error_cells.borrow(), errors);
    assert_eq!(interp.native_precision_cell.get(), precision);
    assert_eq!(interp.get_obj_result(), result);
    assert!(replacement.state.events.borrow().is_empty());
    assert_eq!(interp.entered_native_operation_count(), 0);
}

struct ResultRetirement {
    owner: WeakInterp,
    calls: Rc<Cell<usize>>,
}

extern "C" fn retire_result_context(original: *mut crate::obj::TclObj) {
    // SAFETY: this descriptor alone owns the original result's boxed record.
    // The interpreter retains the object until the reached reporting setter
    // retires its sole reference; the record holds only its genuine WeakInterp.
    let state = unsafe {
        Box::from_raw(crate::obj::internal_rep(original) as usize as *mut ResultRetirement)
    };
    state.calls.set(state.calls.get() + 1);
    let interp = state.owner.upgrade().unwrap();
    // The central context-retirement producer is exercised independently of
    // nested setter checks, so only the reporting owner's post-stage check
    // can retain this newly reached refusal.
    interp.invalidate_native_operation_context();
}

static BOOTSTRAP_RESULT_TYPE: crate::obj::TclObjType = crate::obj::TclObjType {
    name: c"originalBootstrapResult".as_ptr(),
    free_int_rep_proc: Some(retire_result_context),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};

#[test]
fn original_bootstrap_reporting_retirement_preserves_the_reached_first_cause() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // A genuine owned software result and its actual final destructor; no
    // native getter, foreign header, or external disposal-order claim.
    let mut interp = native(CallbackHost::new("before"), "tcl8.4");
    let selected = protocol(&interp);
    assert!(interp.quiet_var_unset_at(b"env", 0));
    let scalar = crate::obj::Owned::fresh(super::new_string(b"SCALAR ENV"));
    interp.var_set_at(b"env", scalar.as_ptr(), 0).unwrap();
    let calls = Rc::new(Cell::new(0));
    let record = Box::new(ResultRetirement {
        owner: interp.downgrade(),
        calls: calls.clone(),
    });
    let original = crate::obj::Owned::fresh(crate::obj::new_obj());
    crate::obj::change_type(
        original.as_ptr(),
        &BOOTSTRAP_RESULT_TYPE,
        Box::into_raw(record) as usize as u64,
    );
    interp.set_result(original.as_ptr());
    drop(original); // Only the actual interpreter result reference remains.
    assert_eq!(calls.get(), 0);
    assert_eq!(
        interp.bootstrap_native_core(selected, &NativeBootstrapInputs::default()),
        Err(Code::Error)
    );
    assert_eq!(calls.get(), 1);
    assert_eq!(interp.native_execution_refusal(), Some(stale()));
    assert_eq!(root(&interp, b"env"), Some(scalar.as_ptr()));
    assert_eq!(interp.entered_native_operation_count(), 0);
}

#[test]
fn original_native_bootstrap_current_write_observers_keep_stored_value_and_result() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual software publication/observer stages, not external constructor timing.
    use tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation;
    let mut interp = native(CallbackHost::new("current"), "tcl8.6");
    let selected = protocol(&interp);
    let name = crate::obj::Owned::fresh(super::new_string(b"env"));
    let first = Rc::new(Cell::new(0));
    let later = Rc::new(Cell::new(0));
    for calls in [later.clone(), first.clone()] {
        interp
            .add_native_variable_observer(
                name.as_ptr(),
                &[NativeVariableTraceOperation::Write],
                Rc::new(BootstrapObserver {
                    replacement: CallbackHost::new("unused"),
                    calls,
                    changes_host: false,
                }),
            )
            .unwrap();
    }
    interp.set_result_bytes(b"ORIGINAL RESULT");
    let result = interp.get_obj_result();
    assert_eq!(
        interp.bootstrap_native_core(selected, &NativeBootstrapInputs::default()),
        Ok(())
    );
    assert_eq!(first.get(), 1);
    assert_eq!(later.get(), 1);
    assert_eq!(
        super::obj_bytes(element(&interp, b"env", b"BOOTSTRAP_USER").unwrap()),
        b"current"
    );
    assert_eq!(interp.get_obj_result(), result);
    assert!(interp.native_execution_refusal().is_none());
    assert_eq!(interp.entered_native_operation_count(), 0);
}

struct BootstrapGuestObserver(Rc<Cell<usize>>);
impl tcl_runtime_api::native_variable_trace::NativeVariableObserver<Interp>
    for BootstrapGuestObserver
{
    type Error = tcl_cmd_core::CmdError;
    fn observe(
        &self,
        _: &mut Interp,
        access: tcl_runtime_api::native_variable_trace::NativeVariableTraceAccess<'_>,
    ) -> Result<(), Self::Error> {
        assert_eq!(access.name1, b"env");
        assert_eq!(access.name2, b"BOOTSTRAP_USER");
        assert_eq!(
            access.operation,
            tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation::Write
        );
        self.0.set(self.0.get() + 1);
        Err(tcl_cmd_core::CmdError::new("BOOTSTRAP OBSERVER"))
    }
}

#[test]
fn original_native_bootstrap_guest_write_failure_keeps_store_and_stops_later_stages() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    use tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation;
    let mut interp = native(CallbackHost::new("guest"), "tcl8.6");
    let selected = protocol(&interp);
    let name = crate::obj::Owned::fresh(super::new_string(b"env"));
    let later = Rc::new(Cell::new(0));
    let failing = Rc::new(Cell::new(0));
    interp
        .add_native_variable_observer(
            name.as_ptr(),
            &[NativeVariableTraceOperation::Write],
            Rc::new(BootstrapObserver {
                replacement: CallbackHost::new("unused"),
                calls: later.clone(),
                changes_host: false,
            }),
        )
        .unwrap();
    interp
        .add_native_variable_observer(
            name.as_ptr(),
            &[NativeVariableTraceOperation::Write],
            Rc::new(BootstrapGuestObserver(failing.clone())),
        )
        .unwrap();
    let platform = element(&interp, b"tcl_platform", b"user").unwrap();
    let errors = interp.native_error_cells.borrow().clone();
    let precision = interp.native_precision_cell.get();
    assert_eq!(
        interp.bootstrap_native_core(selected, &NativeBootstrapInputs::default()),
        Err(Code::Error)
    );
    assert_eq!(failing.get(), 1);
    assert_eq!(later.get(), 0);
    assert_eq!(
        super::obj_bytes(element(&interp, b"env", b"BOOTSTRAP_USER").unwrap()),
        b"guest"
    );
    assert_eq!(element(&interp, b"tcl_platform", b"user"), Some(platform));
    assert_eq!(*interp.native_error_cells.borrow(), errors);
    assert_eq!(interp.native_precision_cell.get(), precision);
    assert_eq!(
        super::obj_bytes(interp.get_obj_result()),
        b"can't set \"env\": BOOTSTRAP OBSERVER"
    );
    assert!(interp.native_execution_refusal().is_none());
    assert_eq!(interp.entered_native_operation_count(), 0);
}

fn bootstrap_in_actual_procedure(interp: &mut Interp, _: &[*mut crate::obj::TclObj]) -> Code {
    assert!(interp.frames.borrow().current_level() > 0);
    let selected = protocol(interp);
    match interp.bootstrap_native_core(selected, &NativeBootstrapInputs::default()) {
        Ok(()) => Code::Ok,
        Err(code) => code,
    }
}

#[test]
fn original_native_bootstrap_write_observer_uses_root_cell_inside_actual_procedure() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The actual native procedure supplies a distinct local env cell; bootstrap
    // publication retains frame zero without borrowing the ambient local frame.
    use tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation;
    let mut interp = native(CallbackHost::new("root"), "tcl8.6");
    interp.register_builtin(
        b"bootstrap_in_actual_procedure",
        bootstrap_in_actual_procedure,
    );
    let name = crate::obj::Owned::fresh(super::new_string(b"env"));
    let calls = Rc::new(Cell::new(0));
    interp
        .add_native_variable_observer(
            name.as_ptr(),
            &[NativeVariableTraceOperation::Write],
            Rc::new(BootstrapObserver {
                replacement: CallbackHost::new("unused"),
                calls: calls.clone(),
                changes_host: false,
            }),
        )
        .unwrap();
    assert_eq!(
        interp
            .eval_str(b"proc p {} {set env LOCAL; bootstrap_in_actual_procedure; return $env}; p"),
        Code::Ok
    );
    assert_eq!(calls.get(), 1);
    assert_eq!(super::obj_bytes(interp.get_obj_result()), b"LOCAL");
    assert_eq!(
        super::obj_bytes(element(&interp, b"env", b"BOOTSTRAP_USER").unwrap()),
        b"root"
    );
    assert!(interp.native_execution_refusal().is_none());
    assert_eq!(interp.entered_native_operation_count(), 0);
}

struct BootstrapListObserver(Rc<Cell<usize>>);
impl tcl_runtime_api::native_variable_trace::NativeVariableObserver<Interp>
    for BootstrapListObserver
{
    type Error = tcl_cmd_core::CmdError;
    fn observe(
        &self,
        interp: &mut Interp,
        access: tcl_runtime_api::native_variable_trace::NativeVariableTraceAccess<'_>,
    ) -> Result<(), Self::Error> {
        assert_eq!(access.name1, b"tcl_pkgPath");
        assert_eq!(access.name2, b"");
        assert_eq!(
            access.operation,
            tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation::Write
        );
        let value = root(interp, b"tcl_pkgPath").unwrap();
        let snapshot = crate::obj::native_object_snapshot(value).unwrap();
        assert_eq!(snapshot.resident, None);
        assert!(matches!(
            snapshot.cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::List { length: 2, .. }
        ));
        self.0.set(self.0.get() + 1);
        Ok(())
    }
}

#[test]
fn original_native_bootstrap_list_publication_reaches_observer_without_string_conversion() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Readonly header/backing observations do not measure an external Tcl getter.
    use tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation;
    let mut interp = native(CallbackHost::new("list"), "tcl8.6");
    let selected = protocol(&interp);
    let name = crate::obj::Owned::fresh(super::new_string(b"tcl_pkgPath"));
    let calls = Rc::new(Cell::new(0));
    interp
        .add_native_variable_observer(
            name.as_ptr(),
            &[NativeVariableTraceOperation::Write],
            Rc::new(BootstrapListObserver(calls.clone())),
        )
        .unwrap();
    let inputs = NativeBootstrapInputs {
        package_path: b"/one {/two space}".to_vec(),
        ..NativeBootstrapInputs::default()
    };
    assert_eq!(interp.bootstrap_native_core(selected, &inputs), Ok(()));
    assert_eq!(calls.get(), 1);
    let snapshot =
        crate::obj::native_object_snapshot(root(&interp, b"tcl_pkgPath").unwrap()).unwrap();
    assert_eq!(snapshot.resident, None);
    assert!(matches!(
        snapshot.cache,
        tcl_syntax::native_object::NativeObjectCacheSnapshot::List { length: 2, .. }
    ));
    assert!(interp.native_execution_refusal().is_none());
    assert_eq!(interp.entered_native_operation_count(), 0);
}
