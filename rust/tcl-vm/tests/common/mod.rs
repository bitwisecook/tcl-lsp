// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Authentic physical core and independent source compiler for resolution fixtures.

use std::{cell::RefCell, path::Path, rc::Rc};
use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_vm::{Vm, host_native::NativeHost};

#[derive(Clone, Default)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl std::io::Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub fn vm_output(source: &str, environment: &str) -> String {
    let profile = tcl_registry::model::resolve_environment(environment).unit_profile();
    let capture = Capture::default();
    let mut vm = Vm::with_native_core(
        Box::new(capture.clone()),
        Rc::new(NativeHost::new()),
        profile,
        tcl_registry::special_vars::NativeBootstrapInputs {
            package_path: Vec::new(),
            default_library: None,
        },
    )
    .expect("matching actual native core before registrations");
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
    let completion = vm
        .try_eval_source(source)
        .expect("original source provider");
    assert!(
        completion.code.is_ok(),
        "{environment}: {:?}: {}",
        completion.code,
        completion.result.to_str()
    );
    String::from_utf8(capture.0.borrow().clone())
        .expect("fixed fixture output is UTF-8")
        .trim()
        .to_owned()
}

pub fn oracle_output(binary: &Path, source: &str) -> String {
    tcl_test_support::run_script(binary, source.as_bytes())
        .expect("run validated original oracle source")
        .strict_text()
        .expect("native fixture succeeds with UTF-8 output")
}
