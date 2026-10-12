// SPDX-License-Identifier: AGPL-3.0-or-later
use std::rc::Rc;
use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_vm::{Vm, host_native::NativeHost};

fn main() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile();
    let mut vm = Vm::with_native_core(
        Box::new(std::io::stdout()),
        Rc::new(NativeHost::new()),
        profile,
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .expect("actual native C8.6 constructor");
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
    match vm.try_init_library() {
        Ok(completion) if completion.code.is_ok() => {}
        Ok(completion) => {
            eprintln!("INIT_GUEST {:?}", completion);
            std::process::exit(1);
        }
        Err(error) => {
            eprintln!("INIT_HOST {error:?}");
            std::process::exit(2);
        }
    }
    match vm.try_eval_source("puts REACHED\n") {
        Ok(completion) if completion.code.is_ok() => {}
        Ok(completion) => {
            eprintln!("STARTUP_GUEST {:?}", completion);
            std::process::exit(3);
        }
        Err(error) => {
            eprintln!("STARTUP_HOST {error:?}");
            std::process::exit(4);
        }
    }
}
