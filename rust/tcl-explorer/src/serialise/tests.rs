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

use super::*;

#[test]
fn interproc_method_rows_retain_equal_labels_and_scoped_call_ids() {
    use tcl_compiler::interprocedural::{MethodSummary, ProcSummary};
    use tcl_compiler::ir::{MethodKind, ProcedureIdIssuer};

    let first_owner = ProcedureIdIssuer::default();
    let second_owner = ProcedureIdIssuer::default();
    let first = first_owner.allocate();
    let second = second_owner.allocate();
    assert_eq!(first.ordinal(), second.ordinal());
    assert_ne!(first, second);
    let mut interproc = InterproceduralAnalysis::default();
    for id in [first, second] {
        let mut base = ProcSummary::unknown("::C::m");
        base.calls = vec![first, second];
        interproc.methods.insert(
            id,
            MethodSummary {
                base,
                class_name: "::C".to_owned(),
                method_kind: MethodKind::Method,
                reads_instance_vars: std::collections::HashSet::default(),
                writes_instance_vars: std::collections::HashSet::default(),
                calls_my: Vec::new(),
                calls_next: false,
            },
        );
    }
    let encoded = serialise_interproc(
        &interproc,
        &tcl_compiler::unit_scope::CallSiteEvidence::default(),
    );
    let rows = encoded.as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["name"], rows[1]["name"]);
    assert_eq!(rows[0]["bodyId"], rows[1]["bodyId"]);
    assert_ne!(rows[0]["bodyKey"], rows[1]["bodyKey"]);
    for row in rows {
        assert_eq!(
            row["callBodyKeys"],
            json!([first.reporting_key(), second.reporting_key()])
        );
        assert_eq!(row["calls"], json!(["::C::m", "::C::m"]));
        assert_eq!(row["methodKind"], "method");
    }
}

#[test]
fn native_lookup_decline_and_interpreter_world_domains_remain_distinct() {
    let reason = OwnedInvocationResolutionUnresolved::UnprovedNativeImplementationLookup {
        lookup: tcl_registry::native_compilation::NativeCompilerImplementationLookup {
            ensemble: "::binary",
            member: "encode",
            slot: "::tcl::binary::encode",
            command: "binary",
            prepended: &["encode"],
        },
    };
    let encoded = unproved_implementation_lookup(&reason);
    assert_eq!(encoded["ensemble"], "::binary");
    assert_eq!(encoded["member"], "encode");
    assert_eq!(encoded["slot"], "::tcl::binary::encode");
    assert_eq!(encoded["command"], "binary");
    assert_eq!(encoded["prepended"], json!(["encode"]));
    assert_eq!(
        invocation_resolution_label(&InvocationResolution::Unresolved(reason)),
        "unresolved-unproved-native-implementation-lookup"
    );
    assert_eq!(
        world_domain_name(WorldRegionKind::InterpreterResult),
        "interpreter-result"
    );
    assert_eq!(
        world_domain_name(WorldRegionKind::CompletionState),
        "completion-state"
    );
    assert_eq!(
        registry_world_domain_name(tcl_registry::WorldStateDomain::InterpreterResult),
        "interpreter-result"
    );
    assert_eq!(
        registry_world_domain_name(tcl_registry::WorldStateDomain::CompletionState),
        "completion-state"
    );
}

#[test]
fn segments_reports_words_with_shape_flags() {
    let segs = serialise_segments("string length \"hi\"", LexerConfig::default());
    let arr = segs.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    let cmd = &arr[0];
    assert_eq!(cmd["name"], "string");
    assert_eq!(cmd["subcommand"], Value::Null);
    let words = cmd["words"].as_array().unwrap();
    assert_eq!(words.len(), 3);
    assert_eq!(words[0]["text"], "string");
    assert_eq!(words[2]["quoted"], true);
    assert_eq!(words[2]["braced"], false);
}

#[test]
fn segments_marks_braced_words() {
    let segs = serialise_segments("if {$x} { puts hi }", LexerConfig::default());
    let words = segs[0]["words"].as_array().unwrap();
    // `{$x}` is a braced word.
    assert_eq!(words[1]["braced"], true);
    assert_eq!(words[1]["quoted"], false);
}
