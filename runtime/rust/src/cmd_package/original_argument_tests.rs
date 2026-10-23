// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual package entry and callback controls; no external package equivalence.

use super::*;
use crate::interp::WeakInterp;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tcl_platform::{
    Capabilities, Clock, Env, Filesystem, Host, HostError, Metadata, NativeIntegerFormatter,
    NumericEnvironment, Process, Sockets, StdIo, SystemEncoding,
};
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::raw_string::NativeValueAccessRefusal;

fn native(profile: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        tcl_registry::model::resolve_environment(profile).unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
fn words(bytes: &[&[u8]]) -> Vec<obj::Owned> {
    bytes
        .iter()
        .map(|word| obj::Owned::fresh(crate::interp::new_string(word)))
        .collect()
}
fn call(interpreter: &mut Interp, bytes: &[&[u8]]) -> Code {
    let words = words(bytes);
    let argv = words.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
    super::package_cmd(interpreter, &argv)
}
fn stale() -> NativeExecutionError {
    NativeValueAccessRefusal::CommandProtocolUnavailable("stale entered native operation").into()
}
fn change_and_restore(interpreter: &mut Interp) {
    let original = interpreter.runtime_context();
    let mut changed = original.clone();
    changed.packages = vec![("original-package-callback".into(), "1.0".into())];
    interpreter.pin_context(&changed).unwrap();
    interpreter.pin_context(&original).unwrap();
}
thread_local! {
    static UPDATER_OWNER: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
    static UPDATE_HOST: Cell<bool> = const { Cell::new(false) };
    static UPDATE_CALLS: Cell<usize> = const { Cell::new(0) };
    static LOADER_MODE: Cell<u8> = const { Cell::new(0) };
    static LOADER_FIRST: RefCell<Option<NativeExecutionError>> = const { RefCell::new(None) };
}
extern "C" fn updater(original: *mut TclObj) {
    UPDATE_CALLS.with(|calls| calls.set(calls.get() + 1));
    let mut interpreter = UPDATER_OWNER
        .with(|owner| owner.borrow().as_ref().unwrap().upgrade())
        .unwrap();
    if UPDATE_HOST.with(Cell::get) {
        interpreter.refuse_host_command("original package selector updater first refusal");
    } else {
        change_and_restore(&mut interpreter);
    }
    unsafe { obj::set_native_updater_string_rep(original, b"provide", false) };
}
static ORIGINAL: obj::TclObjType = obj::TclObjType {
    name: c"originalPackageOperand".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(updater),
    set_from_any_proc: None,
};

#[test]
fn original_package_selector_refusal_precedes_later_arguments_and_database_effects() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Original checked String stages and software database effects only.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        for host in [false, true] {
            let mut interpreter = native(profile);
            UPDATER_OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
            UPDATE_HOST.with(|selected| selected.set(host));
            UPDATE_CALLS.with(|calls| calls.set(0));
            let head = obj::Owned::fresh(crate::interp::new_string(b"package"));
            let selector = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
            let later = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
            let version = obj::Owned::fresh(crate::interp::new_string(b"1.0"));
            let before = interpreter.packages.borrow().provided.clone();
            interpreter.set_result_bytes(b"PRIOR");
            let prior = interpreter.get_obj_result();
            assert_eq!(
                super::package_cmd(
                    &mut interpreter,
                    &[
                        head.as_ptr(),
                        selector.as_ptr(),
                        later.as_ptr(),
                        version.as_ptr()
                    ]
                ),
                Code::Error
            );
            assert_eq!(UPDATE_CALLS.with(Cell::get), 1);
            assert_eq!(obj::bytes_of(selector.as_ptr()), b"provide");
            assert!(!obj::has_string_rep(later.as_ptr()));
            assert_eq!(unsafe { (*selector.as_ptr()).ref_count }, 1);
            assert_eq!(unsafe { (*later.as_ptr()).ref_count }, 1);
            assert_eq!(interpreter.packages.borrow().provided, before);
            assert_eq!(interpreter.get_obj_result(), prior);
            if host {
                assert!(matches!(
                    interpreter.native_execution_refusal(),
                    Some(NativeExecutionError::HostCommandRefusal(_))
                ));
            } else {
                assert_eq!(interpreter.native_execution_refusal(), Some(stale()));
            }
            assert_eq!(interpreter.entered_native_operation_count(), 0);
            UPDATER_OWNER.with(|owner| owner.borrow_mut().take());
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FsMode {
    Ordinary,
    AccessorRestore,
    ReadErrorRestore,
    ReadSuccessRestore,
    ReadSuccessHost,
}
struct FileHost {
    actual: Rc<dyn Host>,
    interpreter: RefCell<Option<WeakInterp>>,
    mode: Cell<FsMode>,
    reads: Cell<usize>,
}
impl FileHost {
    fn filesystem(&self) -> &dyn Filesystem {
        self.actual.filesystem().expect("actual native filesystem")
    }
    fn change(&self, host: bool) {
        let mut interpreter = self
            .interpreter
            .borrow()
            .as_ref()
            .unwrap()
            .upgrade()
            .unwrap();
        if host {
            interpreter.refuse_host_command("original package filesystem first refusal");
        } else {
            change_and_restore(&mut interpreter);
        }
    }
}
impl Host for FileHost {
    fn capabilities(&self) -> Capabilities {
        self.actual.capabilities()
    }
    fn clock(&self) -> &dyn Clock {
        self.actual.clock()
    }
    fn stdio(&self) -> &dyn StdIo {
        self.actual.stdio()
    }
    fn env(&self) -> &dyn Env {
        self.actual.env()
    }
    fn numeric_environment(&self) -> Option<&dyn NumericEnvironment> {
        self.actual.numeric_environment()
    }
    fn native_integer_formatter(&self) -> Option<&dyn NativeIntegerFormatter> {
        self.actual.native_integer_formatter()
    }
    fn system_encoding(&self) -> SystemEncoding {
        self.actual.system_encoding()
    }
    fn sockets(&self) -> Option<&dyn Sockets> {
        self.actual.sockets()
    }
    fn process(&self) -> Option<&dyn Process> {
        self.actual.process()
    }
    fn filesystem(&self) -> Option<&dyn Filesystem> {
        if self.mode.get() == FsMode::AccessorRestore {
            self.change(false);
        }
        Some(self)
    }
}
impl Filesystem for FileHost {
    fn exists(&self, path: &str) -> bool {
        self.filesystem().exists(path)
    }
    fn metadata(&self, path: &str) -> Result<Metadata, HostError> {
        self.filesystem().metadata(path)
    }
    fn read(&self, path: &str) -> Result<Vec<u8>, HostError> {
        self.read_bytes(path.as_bytes())
    }
    fn read_bytes(&self, path: &[u8]) -> Result<Vec<u8>, HostError> {
        let result = self.filesystem().read_bytes(path);
        self.reads.set(self.reads.get() + 1);
        match self.mode.get() {
            FsMode::ReadErrorRestore if result.is_err() => self.change(false),
            FsMode::ReadSuccessRestore if result.is_ok() => self.change(false),
            FsMode::ReadSuccessHost if result.is_ok() => self.change(true),
            _ => {}
        }
        result
    }
    fn write(&self, path: &str, data: &[u8]) -> Result<(), HostError> {
        self.filesystem().write(path, data)
    }
    fn read_dir(&self, path: &str) -> Result<Vec<String>, HostError> {
        self.filesystem().read_dir(path)
    }
    fn create_dir_all(&self, path: &str) -> Result<(), HostError> {
        self.filesystem().create_dir_all(path)
    }
    fn remove(&self, path: &str, recursive: bool) -> Result<(), HostError> {
        self.filesystem().remove(path, recursive)
    }
}
static DIRECTORY_ID: AtomicUsize = AtomicUsize::new(0);
struct Directory(std::path::PathBuf);
impl Drop for Directory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn original_jim_package_filesystem_results_are_checked_before_continue_load_or_publication() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Real delegated filesystem reads and Jim loader effects; no native provider parity.
    for mode in [
        FsMode::Ordinary,
        FsMode::AccessorRestore,
        FsMode::ReadErrorRestore,
        FsMode::ReadSuccessRestore,
        FsMode::ReadSuccessHost,
    ] {
        let directory = Directory(std::env::temp_dir().join(format!(
            "original-package-{}-{}",
            std::process::id(),
            DIRECTORY_ID.fetch_add(1, Ordering::Relaxed)
        )));
        std::fs::create_dir_all(&directory.0).unwrap();
        std::fs::write(
            directory.0.join("original_package_probe.tcl"),
            b"package provide original_package_probe; set ::original_package_loader_effect REACHED",
        )
        .unwrap();
        let mut interpreter = native("jim");
        let host = Rc::new(FileHost {
            actual: interpreter.host(),
            interpreter: RefCell::new(None),
            mode: Cell::new(FsMode::Ordinary),
            reads: Cell::new(0),
        });
        interpreter.set_host(host.clone());
        *host.interpreter.borrow_mut() = Some(interpreter.downgrade());
        let path = obj::Owned::fresh(crate::interp::new_string(
            directory.0.to_str().unwrap().as_bytes(),
        ));
        let paths = obj::Owned::fresh(interpreter.new_list_object(&[path.as_ptr()]));
        interpreter.var_set(b"::auto_path", paths.as_ptr()).unwrap();
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        host.mode.set(mode);
        let code = call(
            &mut interpreter,
            &[b"package", b"require", b"original_package_probe"],
        );
        host.mode.set(FsMode::Ordinary);
        if mode == FsMode::Ordinary {
            assert_eq!(code, Code::Ok);
            assert_eq!(interpreter.result_bytes(), b"1.0");
            assert_eq!(
                obj::bytes_of(
                    interpreter
                        .var_get(b"::original_package_loader_effect")
                        .unwrap()
                ),
                b"REACHED"
            );
            assert_eq!(host.reads.get(), 2);
            assert!(interpreter.native_execution_refusal().is_none());
        } else {
            assert_eq!(code, Code::Error);
            assert_eq!(interpreter.get_obj_result(), prior);
            assert!(!interpreter
                .packages
                .borrow()
                .provided
                .contains_key(b"original_package_probe".as_slice()));
            assert!(!interpreter.var_exists(b"::original_package_loader_effect"));
            assert_eq!(
                host.reads.get(),
                match mode {
                    FsMode::AccessorRestore => 0,
                    FsMode::ReadErrorRestore => 1,
                    _ => 2,
                }
            );
            if mode == FsMode::ReadSuccessHost {
                assert!(matches!(
                    interpreter.native_execution_refusal(),
                    Some(NativeExecutionError::HostCommandRefusal(_))
                ));
            } else {
                assert_eq!(interpreter.native_execution_refusal(), Some(stale()));
            }
        }
        assert!(interpreter.packages.borrow().loading.is_empty());
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

fn loader(interpreter: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    interpreter
        .var_set(
            b"::original_package_loader_effect",
            crate::interp::new_string(b"REACHED"),
        )
        .unwrap();
    interpreter.set_result_bytes(b"loader reached");
    match LOADER_MODE.with(Cell::get) {
        0 => {
            interpreter.refuse_host_command("original package loader first refusal");
            LOADER_FIRST.with(|first| *first.borrow_mut() = interpreter.native_execution_refusal());
            Code::Other(7)
        }
        1 => {
            change_and_restore(interpreter);
            Code::Other(7)
        }
        2 => interpreter.error(b"original guest loader"),
        3 => interpreter.eval_str(b"package provide original_loader_probe 1.0"),
        _ => unreachable!(),
    }
}
#[test]
fn original_package_loader_cleanup_precedes_first_cause_or_guest_settlement() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Actual loader callback and database/file-scope cleanup, independent Native observations.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in 0..4 {
            let mut interpreter = native(profile);
            interpreter.register_builtin(b"original_package_loader", loader);
            assert_eq!(
                interpreter.eval_str(
                    b"package ifneeded original_loader_probe 1.0 {original_package_loader}"
                ),
                Code::Ok
            );
            LOADER_MODE.with(|selected| selected.set(mode));
            LOADER_FIRST.with(|first| first.borrow_mut().take());
            let code = call(
                &mut interpreter,
                &[b"package", b"require", b"original_loader_probe"],
            );
            assert!(interpreter.packages.borrow().loading.is_empty());
            assert!(interpreter.packages.borrow().file_names.is_empty());
            if mode < 2 {
                assert!(
                    interpreter
                        .var_get(b"::original_package_loader_effect")
                        .is_none(),
                    "the original first cause prohibits another native name operation"
                );
            }
            let reached = crate::vars::get(
                &interpreter.frames.borrow(),
                &interpreter.namespaces(),
                interpreter.current_ns(),
                b"::original_package_loader_effect",
            )
            .expect("actual loader effect remains in retained storage");
            assert_eq!(obj::bytes_of(reached), b"REACHED");
            match mode {
                0 => {
                    assert_eq!(code, Code::Error);
                    assert_eq!(interpreter.result_bytes(), b"loader reached");
                    assert_eq!(
                        interpreter.native_execution_refusal(),
                        LOADER_FIRST.with(|first| first.borrow().clone())
                    );
                }
                1 => {
                    assert_eq!(code, Code::Error);
                    assert_eq!(interpreter.result_bytes(), b"loader reached");
                    assert_eq!(interpreter.native_execution_refusal(), Some(stale()));
                }
                2 => {
                    assert_eq!(code, Code::Error);
                    assert_eq!(interpreter.result_bytes(), b"original guest loader");
                    assert!(interpreter.native_execution_refusal().is_none());
                }
                3 => {
                    assert_eq!(code, Code::Ok);
                    assert_eq!(interpreter.result_bytes(), b"1.0");
                    assert!(interpreter.native_execution_refusal().is_none());
                }
                _ => unreachable!(),
            }
            assert_eq!(interpreter.entered_native_operation_count(), 0);
        }
    }
}

#[test]
fn original_package_pending_first_host_precedes_argument_or_database_access() {
    // naming.embedding.original-host-publication-and-fact-transport

    let mut interpreter = native("tcl8.6");
    let selector = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
    let name = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
    let head = obj::Owned::fresh(crate::interp::new_string(b"package"));
    UPDATE_CALLS.with(|calls| calls.set(0));
    let before = interpreter.packages.borrow().provided.clone();
    interpreter.set_result_bytes(b"PRIOR");
    let prior = interpreter.get_obj_result();
    interpreter.refuse_host_command("first pending package refusal");
    let first = interpreter.native_execution_refusal().unwrap();
    assert_eq!(
        super::package_cmd(
            &mut interpreter,
            &[head.as_ptr(), selector.as_ptr(), name.as_ptr()]
        ),
        Code::Error
    );
    assert_eq!(UPDATE_CALLS.with(Cell::get), 0);
    assert!(!obj::has_string_rep(selector.as_ptr()));
    assert!(!obj::has_string_rep(name.as_ptr()));
    assert_eq!(interpreter.packages.borrow().provided, before);
    assert_eq!(interpreter.get_obj_result(), prior);
    assert_eq!(interpreter.native_execution_refusal(), Some(first));
    assert_eq!(interpreter.entered_native_operation_count(), 0);
}
