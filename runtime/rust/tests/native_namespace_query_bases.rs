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

#![cfg(have_tommath)]

use std::rc::Rc;
use tcl_host_native::NativeHost;
use tcl_runtime::interp::{Code, Interp};

#[test]
fn namespace_queries_match_original_source_reports() {
    let source = include_bytes!(
        "../../../experiments/name-handling/variable-namespace-query-bases/result-input.tcl"
    );
    for (environment, expected) in [
        ("tcl8.4", include_bytes!("../../../experiments/name-handling/variable-namespace-query-bases/84/result.stdout.bin").as_slice()),
        ("tcl8.5", include_bytes!("../../../experiments/name-handling/variable-namespace-query-bases/85/result.stdout.bin").as_slice()),
        ("tcl8.6", include_bytes!("../../../experiments/name-handling/variable-namespace-query-bases/86/result.stdout.bin").as_slice()),
        ("tcl9.0", include_bytes!("../../../experiments/name-handling/variable-namespace-query-bases/90/result.stdout.bin").as_slice()),
        ("tcl9.1", include_bytes!("../../../experiments/name-handling/variable-namespace-query-bases/91/result.stdout.bin").as_slice()),
        ("jim", include_bytes!("../../../experiments/name-handling/variable-namespace-query-bases/jim/result.stdout.bin").as_slice()),
    ] {
        let mut interp = Interp::with_native_core(
            Rc::new(NativeHost::new()),
            tcl_registry::model::resolve_environment(environment).unit_profile(),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        ).expect("selected original core");
        assert_eq!(interp.eval_str(source), Code::Ok, "{environment}: {:?}", interp.result_bytes());
        assert!(interp.native_access_refusal().is_none(), "{environment}: {:?}", interp.native_access_refusal());
        let actual_bytes = interp.result_bytes();
        let actual = std::str::from_utf8(&actual_bytes).expect("ASCII query fixture");
        let expected = std::str::from_utf8(expected).expect("ASCII oracle output");
        let (guest_identity, actual_queries) = actual.split_once('\n').expect("guest identity marker");
        let (oracle_identity, expected_queries) = expected.split_once('\n').expect("oracle identity marker");
        // The external executable's revision identifies the oracle; the query
        // contract does not assert the interpreter's separate version reporter.
        eprintln!("{environment}: guest {guest_identity}; oracle {oracle_identity}");
        if environment == "jim" {
            assert_eq!(guest_identity, "PATCHLEVEL 0.84");
            assert_eq!(oracle_identity, "PATCHLEVEL 0.84-9-g5bac7c9");
        } else {
            assert_eq!(guest_identity, oracle_identity, "{environment}: selected C release");
        }
        assert_eq!(actual_queries.lines().count(), 8, "{environment}");
        assert_eq!(expected_queries.lines().count(), 8, "{environment}");
        assert_eq!(actual_queries, expected_queries.strip_suffix('\n').unwrap(), "{environment}");
    }
}
