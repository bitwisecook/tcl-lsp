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

//! Tcl 9.0.4-pinned coverage for `{*}` expansion at command-word position.

use std::path::{Path, PathBuf};

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::TclVersion;
use tcl_test_support::{locate_source_tree, run_script_from_source_tree};
use tcl_vm::{CompileService, Vm};

const EXPECTED: &str = "statement 2 \
head-ordinary-expanded {head ordinary tail1 tail2} empty {} multi {a b} \
substitutions seed value {value position} malformed 1 \
{unmatched open brace in list} {TCL VALUE LIST BRACE}";

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture() -> String {
    std::fs::read_to_string(repository_root().join("tests/fixtures/tcl9/expanded-command-head.tcl"))
        .expect("read expanded-command-head fixture")
}

fn compile_for(release: &str, source: &str) -> tcl_bytecode::ModuleAsm {
    let profile = tcl_registry::model::ingress::resolve_environment(release).analyser_profile();
    BytecodeCompileService::for_profile(profile)
        .compile(source)
        .unwrap_or_else(|error| panic!("compile for {release}: {}", error.0))
}

fn run_compiled(release: &str, source: &str) -> String {
    let profile = tcl_registry::model::ingress::resolve_environment(release).analyser_profile();
    let asm = compile_for(release, source);
    let mut vm = Vm::new();
    vm.set_dialect_profile(profile);
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
    let completion = vm.run_module(&asm);
    assert!(
        completion.code.is_ok(),
        "compiled fixture failed: {}",
        completion.result.to_str()
    );
    completion.result.to_str().to_string()
}

#[test]
fn command_head_expansion_matrix_matches_exact_tcl_9_0_4() {
    let source = fixture();
    let tree = locate_source_tree(&repository_root(), TclVersion::V9_0, None)
        .expect("locate Tcl 9.0.4 source tree")
        .expect("Tcl 9.0.4 source tree is installed");
    assert_eq!(tree.patchlevel, "9.0.4", "exact Tcl oracle pin");
    let oracle_source = format!("{source}\nputs -nonewline [set ::out]\n");
    let oracle = run_script_from_source_tree(&tree, TclVersion::V9_0, oracle_source.as_bytes())
        .expect("run Tcl 9.0.4 oracle")
        .strict_text()
        .expect("Tcl 9.0.4 oracle succeeds");
    assert_eq!(oracle, EXPECTED, "unexpected Tcl 9.0.4 oracle result");
    assert_eq!(run_compiled("tcl9.0", &source), oracle);
}

#[test]
fn expansion_opcodes_follow_the_resolved_document_grammar() {
    let source = "set head {list ok}\n{*}$head";
    let release = "tcl8.4";
    let profile = tcl_registry::model::ingress::resolve_environment(release).analyser_profile();
    let failure = match BytecodeCompileService::for_profile(profile).compile(source) {
        Ok(_) => panic!("{release} must not recognise Tcl 8.5 expansion syntax"),
        Err(error) => error.0,
    };
    assert_eq!(
        failure, "extra characters after close-brace",
        "{release} must retain its resolved pre-8.5 word grammar",
    );

    // F5's Tcl 8.4-derived grammar has its own word-boundary rules, so this
    // shape remains compilable there, but `{*}` is still ordinary text rather
    // than an expansion marker.
    let f5 = compile_for("f5-irules", source);
    let f5_ops = f5
        .top_level
        .instructions
        .iter()
        .map(|instruction| instruction.op)
        .collect::<Vec<_>>();
    assert!(!f5_ops.contains(&tcl_bytecode::Op::EXPAND_STKTOP));
    assert!(!f5_ops.contains(&tcl_bytecode::Op::INVOKE_EXPANDED));

    for release in ["tcl8.5", "tcl8.6", "tcl9.0"] {
        let asm = compile_for(release, source);
        let ops = asm
            .top_level
            .instructions
            .iter()
            .map(|instruction| instruction.op)
            .collect::<Vec<_>>();
        assert!(
            ops.contains(&tcl_bytecode::Op::EXPAND_STKTOP),
            "{release} expansion grammar: {ops:?}",
        );
        assert!(
            ops.contains(&tcl_bytecode::Op::INVOKE_EXPANDED),
            "{release} invocation grammar: {ops:?}",
        );
    }
}
