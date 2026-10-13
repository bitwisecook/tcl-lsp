// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg(have_tommath)]

use std::{cell::RefCell, rc::Rc};
use tcl_host_native::NativeHost;
use tcl_platform::{Capabilities, Clock, Env, Filesystem, Host, Process, StdIo};
use tcl_runtime::interp::{Code, Interp};

struct CaptureHost {
    native: NativeHost,
    stdout: RefCell<Vec<u8>>,
    stderr: RefCell<Vec<u8>>,
}
impl StdIo for CaptureHost {
    fn write_stdout(&self, bytes: &[u8]) {
        self.stdout.borrow_mut().extend_from_slice(bytes);
    }
    fn write_stderr(&self, bytes: &[u8]) {
        self.stderr.borrow_mut().extend_from_slice(bytes);
    }
}
impl Host for CaptureHost {
    fn capabilities(&self) -> Capabilities {
        self.native.capabilities()
    }
    fn clock(&self) -> &dyn Clock {
        self.native.clock()
    }
    fn stdio(&self) -> &dyn StdIo {
        self
    }
    fn env(&self) -> &dyn Env {
        self.native.env()
    }
    fn filesystem(&self) -> Option<&dyn Filesystem> {
        self.native.filesystem()
    }
    fn process(&self) -> Option<&dyn Process> {
        self.native.process()
    }
}

#[test]
fn namespace_command_queries_match_original_portable_source_reports() {
    let source = include_bytes!(
        "../../../experiments/name-handling/namespace-command-query/input-portable.tcl"
    );
    for (environment, expected) in [
        ("tcl8.4", include_bytes!("../../../experiments/name-handling/namespace-command-query/portable/8.4.20/stdout.bin").as_slice()),
        ("tcl8.5", include_bytes!("../../../experiments/name-handling/namespace-command-query/portable/8.5.19/stdout.bin").as_slice()),
        ("tcl8.6", include_bytes!("../../../experiments/name-handling/namespace-command-query/portable/8.6.18/stdout.bin").as_slice()),
        ("tcl9.0", include_bytes!("../../../experiments/name-handling/namespace-command-query/portable/9.0.4/stdout.bin").as_slice()),
        ("tcl9.1", include_bytes!("../../../experiments/name-handling/namespace-command-query/portable/9.1.0/stdout.bin").as_slice()),
        ("jim", include_bytes!("../../../experiments/name-handling/namespace-command-query/portable/jim/stdout.bin").as_slice()),
    ] {
        let host = Rc::new(CaptureHost { native: NativeHost::new(), stdout: RefCell::new(Vec::new()), stderr: RefCell::new(Vec::new()) });
        let mut interp = Interp::with_native_core(host.clone(),
            tcl_registry::model::resolve_environment(environment).unit_profile(),
            tcl_registry::special_vars::NativeBootstrapInputs::default()).unwrap();
        assert_eq!(interp.eval_str(source), Code::Ok, "{environment}: {:?}", interp.result_bytes());
        assert!(!interp.host_refusal_pending(), "{environment}");
        assert!(host.stderr.borrow().is_empty(), "{environment}: {:?}", host.stderr.borrow());
        let actual = host.stdout.borrow();
        let split = |bytes: &[u8]| bytes.iter().position(|byte| *byte == b'\n').unwrap();
        let actual_split = split(&actual);
        let expected_split = split(expected);
        if environment == "jim" {
            assert_eq!(&actual[..actual_split], b"IDENTITY 0.84");
            assert_eq!(&expected[..expected_split], b"IDENTITY 0.84-9-g5bac7c9");
        } else {
            assert_eq!(&actual[..actual_split], &expected[..expected_split], "{environment}: selected C release");
        }
        let actual = &actual[actual_split + 1..];
        let expected = &expected[expected_split + 1..];
        assert_eq!(actual.split(|byte| *byte == b'\n').filter(|row| !row.is_empty()).count(), 33, "{environment}");
        assert_eq!(expected.split(|byte| *byte == b'\n').filter(|row| !row.is_empty()).count(), 33, "{environment}");
        assert_eq!(actual, expected, "{environment}: original query/setup/context rows");
    }
}
