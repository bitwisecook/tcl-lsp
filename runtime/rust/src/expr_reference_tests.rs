// SPDX-License-Identifier: AGPL-3.0-or-later
//! Public expression completions from original constructed-value source controls.

use std::cell::RefCell;
use std::rc::Rc;

use crate::interp::{Code, Interp};
use tcl_platform::{
    Capabilities, Clock, Env, Filesystem, Host, NativeIntegerFormatter, NumericEnvironment,
    Process, Sockets, StdIo, SystemEncoding,
};

struct ExpressionOutputHost {
    actual: Rc<dyn Host>,
    stdout: RefCell<Vec<u8>>,
    stderr: RefCell<Vec<u8>>,
}

impl StdIo for ExpressionOutputHost {
    fn write_stdout(&self, bytes: &[u8]) {
        self.stdout.borrow_mut().extend_from_slice(bytes);
    }

    fn write_stderr(&self, bytes: &[u8]) {
        self.stderr.borrow_mut().extend_from_slice(bytes);
    }
}

impl Host for ExpressionOutputHost {
    fn capabilities(&self) -> Capabilities {
        self.actual.capabilities()
    }
    fn clock(&self) -> &dyn Clock {
        self.actual.clock()
    }
    fn stdio(&self) -> &dyn StdIo {
        self
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
    fn filesystem(&self) -> Option<&dyn Filesystem> {
        self.actual.filesystem()
    }
    fn sockets(&self) -> Option<&dyn Sockets> {
        self.actual.sockets()
    }
    fn process(&self) -> Option<&dyn Process> {
        self.actual.process()
    }
}

#[test]
fn original_expression_source_matches_48_public_reference_boundaries() {
    // Proof: naming.expression.variable-reference-boundaries
    // docs/design/analysis/name-resolution-proofs/expression-variable-reference-boundaries.md
    // The unchanged probe constructs expr/name values with binary format H*.
    // Only its eight public completion/result rows are compared per provider;
    // no parsed AST, resident cache, frame or physical object fact is inferred.
    let source = include_bytes!(
        "../../../rust/tcl-registry/tests/data/native_expression_reference_root341/probe.tcl"
    );
    let providers = [
        ("tcl8.4", include_str!("../../../rust/tcl-registry/tests/data/native_expression_reference_root341/8.4.20/stdout")),
        ("tcl8.5", include_str!("../../../rust/tcl-registry/tests/data/native_expression_reference_root341/8.5.19/stdout")),
        ("tcl8.6", include_str!("../../../rust/tcl-registry/tests/data/native_expression_reference_root341/8.6.18/stdout")),
        ("tcl9.0", include_str!("../../../rust/tcl-registry/tests/data/native_expression_reference_root341/9.0.4/stdout")),
        ("tcl9.1", include_str!("../../../rust/tcl-registry/tests/data/native_expression_reference_root341/9.1.0/stdout")),
        ("jim", include_str!("../../../rust/tcl-registry/tests/data/native_expression_reference_root341/jim/stdout")),
    ];
    let mut compared = 0;
    for (dialect, observed) in providers {
        crate::counters::reset();
        {
            let host = Rc::new(ExpressionOutputHost {
                actual: crate::interp::default_host(),
                stdout: RefCell::new(Vec::new()),
                stderr: RefCell::new(Vec::new()),
            });
            let mut interp = Interp::with_native_core(
                host.clone(),
                crate::environment::profile_for_dialect(dialect),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .expect("actual selected native bootstrap recipe");
            // Supply the byte constructor used by the unchanged CLI source.
            // This fixture does not test its distribution binding or procedure.
            crate::cmd_binary::install(&mut interp);
            assert_eq!(
                interp.eval_str(source),
                Code::Ok,
                "{dialect}: {:?}",
                interp.result_bytes()
            );
            assert!(
                interp.native_access_refusal().is_none(),
                "{dialect}: host refusal"
            );
            assert!(host.stderr.borrow().is_empty(), "{dialect}: stderr");
            let captured = host.stdout.borrow();
            let actual: Vec<_> = std::str::from_utf8(&captured)
                .unwrap()
                .lines()
                .filter(|row| row.starts_with("EXPR "))
                .collect();
            let expected: Vec<_> = observed
                .lines()
                .filter(|row| row.starts_with("EXPR "))
                .collect();
            assert_eq!(expected.len(), 8, "{dialect}: independent native row count");
            assert_eq!(actual, expected, "{dialect}: public expr rows");
            compared += actual.len();
        }
        assert_eq!(
            crate::counters::finalize(),
            0,
            "{dialect}: retained objects"
        );
        assert_eq!(
            crate::counters::double_free_count(),
            0,
            "{dialect}: object destruction"
        );
    }
    assert_eq!(compared, 48);
}
