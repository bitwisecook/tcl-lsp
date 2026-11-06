// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Actual no-tower software owner controls; no Native execution inference.
use super::*;

fn native() -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("tcl8.6"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}

#[test]
fn original_no_tower_body_keeps_non_expression_commands_and_actual_cached_owner() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Genuine constructor/source/cache software control in the absent backend build.
    let mut interpreter = native();
    assert_eq!(
        interpreter.eval_str(b"proc plain {} {set x BEFORE; append x AFTER; return $x}; plain"),
        Code::Ok,
    );
    assert_eq!(interpreter.result_bytes(), b"BEFOREAFTER");
    let original = interpreter.proc_def(b"plain").unwrap();
    let artifact = cache(original.body.as_ptr()).expect("actual non-expression body cache");
    assert!(artifact.scripts.values().any(|script| {
        script
            .commands
            .iter()
            .any(|command| matches!(command.operation, Operation::Store(..)))
    }));
    assert!(!interpreter.host_refusal_pending());
    assert_eq!(interpreter.entered_native_operation_count(), 0);
}

#[test]
fn original_no_tower_reached_refusal_preserves_actual_result_and_prior_full_cause() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The actual stage owner is exercised; this does not issue an unavailable expression recipe.
    for prior in [false, true] {
        let mut interpreter = native();
        let scope =
            crate::interp::native_operation_currency::NativeOperationScope::enter(&interpreter)
                .unwrap();
        interpreter.set_result_bytes(b"ORIGINAL_RESULT");
        let original = interpreter.result_obj();
        if prior {
            interpreter.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original prior no-tower boundary",
                ),
            );
        }
        let before = interpreter.native_execution_refusal();
        assert_eq!(
            interpreter.refuse_missing_compiled_expression_backend(),
            Code::Error
        );
        let first = before.unwrap_or(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::ExpressionEngineUnavailable,
        ));
        assert_eq!(interpreter.native_execution_refusal(), Some(first.clone()));
        assert_eq!(scope.currency().ensure_current_or_refuse(), Err(first));
        assert_eq!(interpreter.result_obj(), original);
    }
}
