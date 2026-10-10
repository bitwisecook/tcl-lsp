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

//! Shared original imported-command query traversal and closed public observations.

use std::cell::RefCell;
use std::rc::Rc;

use crate::interp::{Code, Interp};
use tcl_platform::{
    Capabilities, Clock, Env, Filesystem, Host, NativeIntegerFormatter, NumericEnvironment,
    Process, Sockets, StdIo, SystemEncoding,
};

struct ImportOutputHost {
    actual: Rc<dyn Host>,
    stdout: RefCell<Vec<u8>>,
    stderr: RefCell<Vec<u8>>,
}

impl StdIo for ImportOutputHost {
    fn write_stdout(&self, bytes: &[u8]) {
        self.stdout.borrow_mut().extend_from_slice(bytes);
    }

    fn write_stderr(&self, bytes: &[u8]) {
        self.stderr.borrow_mut().extend_from_slice(bytes);
    }
}

impl Host for ImportOutputHost {
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

const SOURCE: &[u8] = include_bytes!(
    "../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/probe.tcl"
);

fn query_rows(stdout: &str) -> Vec<&str> {
    stdout
        .lines()
        .filter(|row| {
            row.starts_with("CASE ")
                && !row.starts_with("CASE version.")
                && !row.starts_with("CASE helper.")
        })
        .collect()
}

#[test]
fn original_deep_import_queries_match_all_546_c_and_jim_public_rows() {
    // naming.namespace.original-deep-import-chain
    // docs/design/analysis/name-resolution-proofs/namespace-original-deep-import-chain.md
    // The complete original ASCII/LF probe is unchanged. Only 91 public
    // code/result rows per provider are compared; CLI version/path and helper
    // declaration observations are independent bootstrap metadata.
    // Setup failure is retained: C8.4 and Jim unsupported ensemble queries do
    // not establish an ensemble chain. Jim import aliases retain their own
    // textual rename/replacement semantics and do not gain C imported tokens.
    // No native header, compiled instruction or generic handler is observed.
    let providers = [
        (
            "tcl8.4",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/providers/8.4.20/execute.stdout"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/providers/8.5.19/execute.stdout"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/providers/8.6.18/execute.stdout"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/providers/9.0.4/execute.stdout"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/providers/9.1.0/execute.stdout"
            ),
        ),
        (
            "jim",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/providers/jim/execute.stdout"
            ),
        ),
    ];
    let mut comparisons = 0;
    for (engine, observed) in providers {
        crate::counters::reset();
        {
            let host = Rc::new(ImportOutputHost {
                actual: super::default_host(),
                stdout: RefCell::new(Vec::new()),
                stderr: RefCell::new(Vec::new()),
            });
            let mut interp = Interp::with_native_core(
                host.clone(),
                tcl_registry::model::ingress::resolve_environment(engine).unit_profile(),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            if engine == "jim" {
                // The actual CLI's namespace-ensemble helper is explicit;
                // its own NamespaceInfo dependency remains separately owned.
                // No complete distribution roster or C ensemble grant follows.
                crate::cmd_proc::install_stock_scripted_library(
                    &mut interp,
                    tcl_registry::native_scripted_distribution::NativeScriptedLibrary::NamespaceEnsemble,
                );
                crate::cmd_binary::install(&mut interp);
            }
            assert_eq!(
                interp.eval_str(SOURCE),
                Code::Ok,
                "{engine}: {:?}",
                interp.result_bytes()
            );
            assert!(
                !interp.host_refusal_pending(),
                "{engine}: {:?}",
                interp.native_access_refusal()
            );
            assert!(host.stderr.borrow().is_empty(), "{engine}: stderr");
            let stdout = host.stdout.borrow();
            let stdout = std::str::from_utf8(&stdout).unwrap();
            let actual = query_rows(stdout);
            let expected = query_rows(observed);
            assert_eq!(expected.len(), 91, "{engine}: exact original query count");
            assert_eq!(actual, expected, "{engine}: original public query rows");
            comparisons += actual.len();
        }
        assert_eq!(crate::counters::finalize(), 0, "{engine}: retained objects");
        assert_eq!(
            crate::counters::double_free_count(),
            0,
            "{engine}: destruction"
        );
    }
    assert_eq!(comparisons, 546);
}

fn placeholder(_: &mut Interp, _: &[*mut crate::obj::TclObj]) -> Code {
    Code::Ok
}

#[test]
fn malformed_actual_import_generation_cycle_has_no_partial_query_origin() {
    // naming.namespace.original-deep-import-chain
    // docs/design/analysis/name-resolution-proofs/namespace-original-deep-import-chain.md
    // Software-only invariant control: direct owned table mutation deliberately
    // bypasses the public import loop rejection to exercise query termination.
    // The original public probe observes no native cycle/token/header purpose.
    let mut interp = Interp::with_native_core(
        super::default_host(),
        tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    interp.register_builtin(b"cycle_a", placeholder);
    interp.register_builtin(b"cycle_b", placeholder);
    let old_a = interp.resolve_cmd_token(b"cycle_a").unwrap();
    let old_b = interp.resolve_cmd_token(b"cycle_b").unwrap();
    interp.bind_command_replacement(
        crate::namespace::GLOBAL,
        b"cycle_a",
        super::Command::Imported {
            source: b"::cycle_b".to_vec(),
            source_generation: old_b,
            ensemble: None,
            identity: Rc::new(super::ImportToken),
        },
    );
    interp.bind_command_replacement(
        crate::namespace::GLOBAL,
        b"cycle_b",
        super::Command::Imported {
            source: b"::cycle_a".to_vec(),
            source_generation: old_a,
            ensemble: None,
            identity: Rc::new(super::ImportToken),
        },
    );
    let id = interp
        .find_command_id(crate::namespace::GLOBAL, b"cycle_a")
        .unwrap();
    assert!(interp.proc_def(b"cycle_a").is_none());
    assert!(interp.ensemble_config_at(b"cycle_a").is_none());
    assert!(interp.imported_source_id(id).is_none());
    assert!(!interp.host_refusal_pending());
}
