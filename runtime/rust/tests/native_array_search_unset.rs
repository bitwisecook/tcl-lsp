// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg(have_tommath)]

use std::rc::Rc;
use tcl_host_native::NativeHost;
use tcl_runtime::interp::{Code, Interp};

fn runtime(environment: &str) -> Interp {
    Interp::with_native_core(
        Rc::new(NativeHost::new()),
        tcl_registry::model::resolve_environment(environment).unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .expect("matching native core")
}

#[test]
fn compiled_array_pair_counts_match_original_oracles() {
    let source = include_bytes!("../../../experiments/name-handling/results/runtime-array-search-var132-current/conformance.tcl");
    for (environment, expected) in [
        (
            "tcl8.6",
            include_bytes!("../../../experiments/name-handling/results/runtime-array-search-var132-current/c-8.6.18.stdout.txt").as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!("../../../experiments/name-handling/results/runtime-array-search-var132-current/c-9.0.4.stdout.txt").as_slice(),
        ),
    ] {
        let mut interp = runtime(environment);
        assert_eq!(
            interp.eval_str(source),
            Code::Ok,
            "{environment}: {:?}",
            interp.result_bytes()
        );
        assert_eq!(interp.result_bytes(), expected, "{environment}");
    }
}

#[test]
fn original_var_13_2_retires_the_selected_search() {
    let mut interp = runtime("tcl8.6");
    let source = include_bytes!("../../../experiments/name-handling/results/runtime-array-search-var132-current/original-body.tcl");
    assert_eq!(interp.eval_str(source), Code::Error);
    assert_eq!(interp.result_bytes(), b"couldn't find search \"s-1-a\"");
    assert_eq!(interp.eval_str(b"set ::errorCode"), Code::Ok);
    assert_eq!(interp.result_bytes(), b"TCL LOOKUP ARRAYSEARCH s-1-a");
}
