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

use std::path::Path;
use std::rc::Rc;

use tcl_dialect::TclVersion;
use tcl_test_support::{locate_source_tree, run_script_from_source_tree};
use tcl_vm::{Value, Vm, host_native::NativeHost};

fn presence(environment: &str) -> Vec<u8> {
    let mut vm = Vm::with_native_core(
        Box::new(std::io::sink()),
        Rc::new(NativeHost::new()),
        tcl_registry::model::resolve_environment(environment).unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .expect("selected native core");
    let registered = vm
        .registered_command_token_bytes(b"::tcl::InitAutoPath")
        .is_some();
    if environment == "jim" {
        assert!(!registered, "Jim native core command table");
        return b"\n".to_vec();
    }
    let completion = vm
        .try_invoke_command(
            "info",
            &[
                Value::new_native_string_bytes(b"commands".as_slice()),
                Value::new_native_string_bytes(b"::tcl::InitAutoPath".as_slice()),
            ],
        )
        .expect("native command query");
    assert!(completion.code.is_ok(), "{environment}: {completion:?}");
    let mut result = tcl_syntax::value::ValueOps::native_string_bytes(&mut vm, &completion.result)
        .expect("native query result getter")
        .to_vec();
    assert_eq!(
        registered,
        !result.is_empty(),
        "native core table and info query"
    );
    result.push(b'\n');
    result
}

#[test]
fn init_auto_path_presence_matches_each_native_core() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for version in TclVersion::ALL {
        let actual = presence(version.dialect_profile_name());
        let expected = if version == TclVersion::V9_1 {
            b"::tcl::InitAutoPath\n".as_slice()
        } else {
            b"\n".as_slice()
        };
        assert_eq!(actual, expected, "{version:?}");
        let Some(tree) = locate_source_tree(&repository, version, None).expect("source discovery")
        else {
            continue;
        };
        let oracle = run_script_from_source_tree(
            &tree,
            version,
            b"puts [info commands ::tcl::InitAutoPath]\n",
        )
        .expect("C Tcl command-presence oracle");
        assert!(
            oracle.success() && oracle.stderr.is_empty(),
            "{version:?}: {oracle:?}"
        );
        assert_eq!(actual, oracle.stdout, "{version:?}");
    }
    assert_eq!(presence("jim"), b"\n");
    if let Some(jim) = tcl_test_support::locate_jimsh().expect("Jim oracle discovery") {
        let oracle =
            tcl_test_support::run_script(&jim.path, b"puts [info commands ::tcl::InitAutoPath]\n")
                .expect("Jim command-presence oracle");
        assert!(oracle.success() && oracle.stderr.is_empty(), "{oracle:?}");
        assert_eq!(presence("jim"), oracle.stdout, "Jim");
    }
}
