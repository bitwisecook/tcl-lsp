// SPDX-License-Identifier: AGPL-3.0-or-later
//! Caught callback results follow the surviving cell, independently of formals.

use super::*;
use std::cell::RefCell;
use std::io::{self, Write};
use std::rc::Rc;

#[derive(Clone)]
struct Output(Rc<RefCell<Vec<u8>>>);

impl Write for Output {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn caught_callback_result_follows_replaced_formal_alias_native_controls() {
    // Native proof: naming.variable.catch-result-alias-continuity
    // docs/design/analysis/name-resolution-proofs/catch-result-alias-continuity.md
    const SOURCE: &[u8] = include_bytes!("../../tests/data/native_catch_formal_alias/cases.tcl");
    for (engine, observed) in [
        (
            "tcl8.4",
            include_str!("../../tests/data/native_catch_formal_alias/8.4.20/stdout"),
        ),
        (
            "tcl8.5",
            include_str!("../../tests/data/native_catch_formal_alias/8.5.19/stdout"),
        ),
        (
            "tcl8.6",
            include_str!("../../tests/data/native_catch_formal_alias/8.6.18/stdout"),
        ),
        (
            "tcl9.0",
            include_str!("../../tests/data/native_catch_formal_alias/9.0.4/stdout"),
        ),
        (
            "tcl9.1",
            include_str!("../../tests/data/native_catch_formal_alias/9.1.0/stdout"),
        ),
        (
            "jim",
            include_str!("../../tests/data/native_catch_formal_alias/jim/stdout"),
        ),
    ] {
        let profile = crate::environment::profile_for_dialect(engine);
        let output = Output(Rc::new(RefCell::new(Vec::new())));
        let mut vm = Vm::with_native_core(
            Box::new(output.clone()),
            Rc::new(crate::host_native::NativeHost::new()),
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs {
                package_path: Vec::new(),
                default_library: None,
            },
        )
        .unwrap();
        vm.set_compiler(Box::new(
            tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
        ));
        if vm
            .native_invocation_dialect()
            .binary_scripted_ingress()
            .is_some()
        {
            // The captured Jim distribution loads pack and its actual binary
            // wrapper; Jim_RegisterCoreCommands alone does not install it.
            crate::cmd_binary::register(&mut vm);
        }
        let result = vm.try_eval_source_bytes(SOURCE).unwrap();
        assert_eq!(result.code, Code::Ok, "{engine}: {result:?}");
        let actual = String::from_utf8(output.0.borrow().clone()).unwrap();
        let measured_rows = |text: &str| {
            text.lines()
                .filter(|line| !line.starts_with("VERSION|"))
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };
        assert_eq!(measured_rows(&actual), measured_rows(observed), "{engine}");
    }
}
