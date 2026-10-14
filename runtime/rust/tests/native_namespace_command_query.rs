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
    fn numeric_environment(&self) -> Option<&dyn tcl_platform::NumericEnvironment> {
        self.native.numeric_environment()
    }
    fn native_list_sorting_environment(
        &self,
    ) -> Option<&dyn tcl_platform::NativeListSortingEnvironment> {
        self.native.native_list_sorting_environment()
    }
    fn native_integer_formatter(&self) -> Option<&dyn tcl_platform::NativeIntegerFormatter> {
        self.native.native_integer_formatter()
    }
    fn system_encoding(&self) -> tcl_platform::SystemEncoding {
        self.native.system_encoding()
    }
    fn encoding_catalogue(&self) -> Option<&dyn tcl_platform::EncodingCatalogue> {
        self.native.encoding_catalogue()
    }
    fn sockets(&self) -> Option<&dyn tcl_platform::Sockets> {
        self.native.sockets()
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
        assert_eq!(interp.eval_str(source), Code::Ok, "{environment}: result={:?}; refusal={:?}; stdout={:?}", interp.result_bytes(), interp.native_execution_refusal(), host.stdout.borrow());
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

fn compare_reports(environment: &str, source: &[u8], expected: &[u8], rows: usize) {
    compare_reports_with_setup(environment, source, expected, rows, |_| {});
}

fn compare_reports_with_setup(
    environment: &str,
    source: &[u8],
    expected: &[u8],
    rows: usize,
    setup: impl FnOnce(&mut Interp),
) {
    let host = Rc::new(CaptureHost {
        native: NativeHost::new(),
        stdout: RefCell::new(Vec::new()),
        stderr: RefCell::new(Vec::new()),
    });
    let mut interp = Interp::with_native_core(
        host.clone(),
        tcl_registry::model::resolve_environment(environment).unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    setup(&mut interp);
    assert_eq!(
        interp.eval_str(source),
        Code::Ok,
        "{environment}: result={:?}; refusal={:?}; stdout={:?}",
        interp.result_bytes(),
        interp.native_execution_refusal(),
        host.stdout.borrow()
    );
    assert!(!interp.host_refusal_pending(), "{environment}");
    assert!(
        host.stderr.borrow().is_empty(),
        "{environment}: {:?}",
        host.stderr.borrow()
    );
    let actual = host.stdout.borrow();
    let split = |bytes: &[u8]| bytes.iter().position(|byte| *byte == b'\n').unwrap();
    let actual_split = split(&actual);
    let expected_split = split(expected);
    if environment == "jim" {
        assert_eq!(&actual[..actual_split], b"IDENTITY 0.84");
        assert_eq!(&expected[..expected_split], b"IDENTITY 0.84-9-g5bac7c9");
    } else {
        assert_eq!(
            &actual[..actual_split],
            &expected[..expected_split],
            "{environment}: selected C release"
        );
    }
    let actual = &actual[actual_split + 1..];
    let expected = &expected[expected_split + 1..];
    assert_eq!(
        actual
            .split(|byte| *byte == b'\n')
            .filter(|row| !row.is_empty())
            .count(),
        rows,
        "{environment}"
    );
    assert_eq!(
        expected
            .split(|byte| *byte == b'\n')
            .filter(|row| !row.is_empty())
            .count(),
        rows,
        "{environment}"
    );
    assert_eq!(
        actual, expected,
        "{environment}: original query/setup/context rows"
    );
}

#[test]
fn namespace_origin_original_c84() {
    compare_reports(
        "tcl8.4",
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/input.tcl"),
        include_bytes!(
            "../../../experiments/name-handling/namespace-origin-query/8.4.20/stdout.bin"
        ),
        37,
    );
}

#[test]
fn namespace_helper_root_original_c84() {
    compare_reports("tcl8.4",
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/helper-root-input.tcl"),
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/helper-root/8.4.20/stdout.bin"), 5);
}

#[test]
fn namespace_origin_original_c85() {
    compare_reports(
        "tcl8.5",
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/input.tcl"),
        include_bytes!(
            "../../../experiments/name-handling/namespace-origin-query/8.5.19/stdout.bin"
        ),
        37,
    );
}

#[test]
fn namespace_helper_root_original_c85() {
    compare_reports("tcl8.5",
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/helper-root-input.tcl"),
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/helper-root/8.5.19/stdout.bin"), 5);
}

#[test]
fn namespace_origin_original_c86() {
    compare_reports(
        "tcl8.6",
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/input.tcl"),
        include_bytes!(
            "../../../experiments/name-handling/namespace-origin-query/8.6.18/stdout.bin"
        ),
        37,
    );
}

#[test]
fn namespace_helper_root_original_c86() {
    compare_reports("tcl8.6",
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/helper-root-input.tcl"),
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/helper-root/8.6.18/stdout.bin"), 5);
}

#[test]
fn namespace_origin_original_c90() {
    compare_reports(
        "tcl9.0",
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/input.tcl"),
        include_bytes!(
            "../../../experiments/name-handling/namespace-origin-query/9.0.4/stdout.bin"
        ),
        37,
    );
}

#[test]
fn namespace_helper_root_original_c90() {
    compare_reports("tcl9.0",
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/helper-root-input.tcl"),
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/helper-root/9.0.4/stdout.bin"), 5);
}

#[test]
fn namespace_origin_original_c91() {
    compare_reports(
        "tcl9.1",
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/input.tcl"),
        include_bytes!(
            "../../../experiments/name-handling/namespace-origin-query/9.1.0/stdout.bin"
        ),
        37,
    );
}

#[test]
fn namespace_helper_root_original_c91() {
    compare_reports("tcl9.1",
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/helper-root-input.tcl"),
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/helper-root/9.1.0/stdout.bin"), 5);
}

#[test]
fn namespace_origin_original_jim() {
    compare_reports(
        "jim",
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/input.tcl"),
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/jim/stdout.bin"),
        37,
    );
}

#[test]
fn namespace_helper_root_original_jim() {
    compare_reports(
        "jim",
        include_bytes!(
            "../../../experiments/name-handling/namespace-origin-query/helper-root-input.tcl"
        ),
        include_bytes!(
            "../../../experiments/name-handling/namespace-origin-query/helper-root/jim/stdout.bin"
        ),
        5,
    );
}

#[test]
fn namespace_which_string_observer_c84() {
    compare_reports("tcl8.4",
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/which.tcl"),
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/which/8.4.20/stdout.bin"), 33);
}

#[test]
fn namespace_origin_string_observer_c84() {
    compare_reports("tcl8.4",
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/origin.tcl"),
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/origin/8.4.20/stdout.bin"), 37);
}

#[test]
fn namespace_helper_root_string_observer_c84() {
    compare_reports("tcl8.4",
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/helper-root.tcl"),
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/helper-root/8.4.20/stdout.bin"), 5);
}

#[test]
fn namespace_which_string_observer_c85() {
    compare_reports("tcl8.5",
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/which.tcl"),
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/which/8.5.19/stdout.bin"), 33);
}

#[test]
fn namespace_origin_string_observer_c85() {
    compare_reports("tcl8.5",
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/origin.tcl"),
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/origin/8.5.19/stdout.bin"), 37);
}

#[test]
fn namespace_helper_root_string_observer_c85() {
    compare_reports("tcl8.5",
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/helper-root.tcl"),
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/helper-root/8.5.19/stdout.bin"), 5);
}

#[test]
fn namespace_which_string_observer_c86() {
    compare_reports("tcl8.6",
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/which.tcl"),
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/which/8.6.18/stdout.bin"), 33);
}

#[test]
fn namespace_origin_string_observer_c86() {
    compare_reports("tcl8.6",
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/origin.tcl"),
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/origin/8.6.18/stdout.bin"), 37);
}

#[test]
fn namespace_helper_root_string_observer_c86() {
    compare_reports("tcl8.6",
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/helper-root.tcl"),
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/helper-root/8.6.18/stdout.bin"), 5);
}

#[test]
fn namespace_which_string_observer_c90() {
    compare_reports("tcl9.0",
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/which.tcl"),
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/which/9.0.4/stdout.bin"), 33);
}

#[test]
fn namespace_origin_string_observer_c90() {
    compare_reports("tcl9.0",
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/origin.tcl"),
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/origin/9.0.4/stdout.bin"), 37);
}

#[test]
fn namespace_helper_root_string_observer_c90() {
    compare_reports("tcl9.0",
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/helper-root.tcl"),
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/helper-root/9.0.4/stdout.bin"), 5);
}

#[test]
fn namespace_which_string_observer_c91() {
    compare_reports("tcl9.1",
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/which.tcl"),
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/which/9.1.0/stdout.bin"), 33);
}

#[test]
fn namespace_origin_string_observer_c91() {
    compare_reports("tcl9.1",
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/origin.tcl"),
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/origin/9.1.0/stdout.bin"), 37);
}

#[test]
fn namespace_helper_root_string_observer_c91() {
    compare_reports("tcl9.1",
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/helper-root.tcl"),
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/helper-root/9.1.0/stdout.bin"), 5);
}

#[test]
fn namespace_which_string_observer_jim() {
    compare_reports("jim",
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/which.tcl"),
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/which/jim/stdout.bin"), 33);
}

#[test]
fn namespace_origin_string_observer_jim() {
    compare_reports("jim",
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/origin.tcl"),
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/origin/jim/stdout.bin"), 37);
}

#[test]
fn namespace_helper_root_string_observer_jim() {
    compare_reports("jim",
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/helper-root.tcl"),
        include_bytes!("../../../experiments/name-handling/runtime-command-query-string-observer/helper-root/jim/stdout.bin"), 5);
}

#[test]
fn alias_registration_guest_nul_original_c84() {
    compare_reports("tcl8.4",
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/alias-registration-nul/input.tcl"),
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/alias-registration-nul/8.4.20/stdout.bin"), 18);
}

#[test]
fn alias_registration_guest_nul_original_c85() {
    compare_reports("tcl8.5",
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/alias-registration-nul/input.tcl"),
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/alias-registration-nul/8.5.19/stdout.bin"), 18);
}

#[test]
fn alias_registration_guest_nul_original_c86() {
    compare_reports("tcl8.6",
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/alias-registration-nul/input.tcl"),
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/alias-registration-nul/8.6.18/stdout.bin"), 18);
}

#[test]
fn alias_registration_guest_nul_original_c90() {
    compare_reports("tcl9.0",
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/alias-registration-nul/input.tcl"),
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/alias-registration-nul/9.0.4/stdout.bin"), 18);
}

#[test]
fn alias_registration_guest_nul_original_c91() {
    compare_reports("tcl9.1",
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/alias-registration-nul/input.tcl"),
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/alias-registration-nul/9.1.0/stdout.bin"), 18);
}

#[test]
fn alias_registration_guest_nul_original_jim() {
    // The original jimsh provisions binary; Jim_CreateInterp alone does not.
    compare_reports_with_setup(
        "jim",
        include_bytes!(
            "../../../experiments/name-handling/namespace-origin-query/alias-registration-nul/input.tcl"
        ),
        include_bytes!(
            "../../../experiments/name-handling/namespace-origin-query/alias-registration-nul/jim/stdout.bin"
        ),
        18,
        tcl_runtime::cmd_binary::install,
    );
}

#[test]
fn retained_import_original_c84() {
    compare_reports(
        "tcl8.4",
        include_bytes!(
            "../../../experiments/name-handling/runtime-retained-import-source/input.tcl"
        ),
        include_bytes!(
            "../../../experiments/name-handling/runtime-retained-import-source/8.4.20/stdout.bin"
        ),
        1,
    );
}

#[test]
fn retained_import_original_c85() {
    compare_reports(
        "tcl8.5",
        include_bytes!(
            "../../../experiments/name-handling/runtime-retained-import-source/input.tcl"
        ),
        include_bytes!(
            "../../../experiments/name-handling/runtime-retained-import-source/8.5.19/stdout.bin"
        ),
        1,
    );
}

#[test]
fn retained_import_original_c86() {
    compare_reports(
        "tcl8.6",
        include_bytes!(
            "../../../experiments/name-handling/runtime-retained-import-source/input.tcl"
        ),
        include_bytes!(
            "../../../experiments/name-handling/runtime-retained-import-source/8.6.18/stdout.bin"
        ),
        1,
    );
}

#[test]
fn retained_import_original_c90() {
    compare_reports(
        "tcl9.0",
        include_bytes!(
            "../../../experiments/name-handling/runtime-retained-import-source/input.tcl"
        ),
        include_bytes!(
            "../../../experiments/name-handling/runtime-retained-import-source/9.0.4/stdout.bin"
        ),
        1,
    );
}

#[test]
fn retained_import_original_c91() {
    compare_reports(
        "tcl9.1",
        include_bytes!(
            "../../../experiments/name-handling/runtime-retained-import-source/input.tcl"
        ),
        include_bytes!(
            "../../../experiments/name-handling/runtime-retained-import-source/9.1.0/stdout.bin"
        ),
        1,
    );
}

#[test]
fn retained_import_original_jim() {
    compare_reports(
        "jim",
        include_bytes!(
            "../../../experiments/name-handling/runtime-retained-import-source/input.tcl"
        ),
        include_bytes!(
            "../../../experiments/name-handling/runtime-retained-import-source/jim/stdout.bin"
        ),
        1,
    );
}

#[test]
fn alias_registration_guest_nul_format_original_jim() {
    compare_reports("jim",
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/jim-alias-format-nul/input.tcl"),
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/jim-alias-format-nul/stdout.bin"), 11);
}

#[test]
fn alias_registration_records_original_c84() {
    compare_reports("tcl8.4",
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/alias-registration-records/input.tcl"),
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/alias-registration-records/8.4.20/stdout.bin"), 26);
}

#[test]
fn alias_registration_records_original_c85() {
    compare_reports("tcl8.5",
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/alias-registration-records/input.tcl"),
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/alias-registration-records/8.5.19/stdout.bin"), 26);
}

#[test]
fn alias_registration_records_original_c86() {
    compare_reports("tcl8.6",
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/alias-registration-records/input.tcl"),
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/alias-registration-records/8.6.18/stdout.bin"), 26);
}

#[test]
fn alias_registration_records_original_c90() {
    compare_reports("tcl9.0",
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/alias-registration-records/input.tcl"),
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/alias-registration-records/9.0.4/stdout.bin"), 26);
}

#[test]
fn alias_registration_records_original_c91() {
    compare_reports("tcl9.1",
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/alias-registration-records/input.tcl"),
        include_bytes!("../../../experiments/name-handling/namespace-origin-query/alias-registration-records/9.1.0/stdout.bin"), 26);
}
