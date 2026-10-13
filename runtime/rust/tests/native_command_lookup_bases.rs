// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg(have_tommath)]

use std::rc::Rc;
use tcl_dialect::TclVersion;
use tcl_host_native::NativeHost;
use tcl_runtime::interp::{Code, Interp};
use tcl_syntax::naming::conformance::{vector_call, vector_script, vector_setup, vectors};
use tcl_test_support::{oracle_row_progress, required_tclshs, run_script};

#[test]
fn current_path_root_resolution_matches_real_oracle_vectors() {
    let oracles = required_tclshs(&TclVersion::ALL).expect("all five original C Tcl releases");
    for oracle in oracles {
        let environment = format!("tcl{}", oracle.version.version_string());
        for vector in vectors().into_iter().filter(|vector| {
            matches!(
                vector.call.as_str(),
                "p" | "::p" | "helper" | "inner::p" | "c::d::p" | "sub::q"
            )
        }) {
            let case = format!("vector-{}", vector.line);
            oracle_row_progress("native-command-lookup-bases", &environment, &case, None);
            let expected = vector.wants.get(oracle.version);
            let observed = run_script(&oracle.path, vector_script(&vector).as_bytes())
                .expect("execute the original C oracle")
                .strict_text();
            let mut interp = Interp::with_native_core(
                Rc::new(NativeHost::new()),
                tcl_registry::model::resolve_environment(&environment).unit_profile(),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .expect("matching native core");
            let setup = interp.eval_str(vector_setup(&vector).as_bytes());
            if expected == "!ERROR" {
                assert!(observed.is_err(), "{environment}: vector {}", vector.line);
                assert_eq!(setup, Code::Error, "{environment}: vector {}", vector.line);
                oracle_row_progress("native-command-lookup-bases", &environment, &case, Some(1));
                continue;
            }
            assert_eq!(
                observed.as_deref(),
                Ok(expected),
                "{environment}: vector {}: oracle differs from retained expectation",
                vector.line
            );
            assert_eq!(
                setup,
                Code::Ok,
                "{environment}: vector {} setup: {:?}",
                vector.line,
                interp.result_bytes()
            );
            let code = interp.eval_str(vector_call(&vector).as_bytes());
            assert!(
                interp.native_access_refusal().is_none(),
                "{environment}: vector {}: {:?}",
                vector.line,
                interp.native_access_refusal()
            );
            if expected == "-" {
                assert_eq!(code, Code::Error, "{environment}: vector {}", vector.line);
            } else {
                assert_eq!(code, Code::Ok, "{environment}: vector {}", vector.line);
                assert_eq!(
                    interp.result_bytes(),
                    expected.as_bytes(),
                    "{environment}: vector {}",
                    vector.line
                );
            }
            oracle_row_progress("native-command-lookup-bases", &environment, &case, Some(1));
        }
    }
}
