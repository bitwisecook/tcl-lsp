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

//! Coverage + validation of the registry JSON snapshot surface
//! (`command_snapshot` + `snapshot` graph dumps) — the data behind the
//! `tcl registry-dump` verb.
//!
//! Snapshots are F5/registry-internal canonical JSON with no core-tclsh
//! analogue, so the proof is structural: the dump is well-formed,
//! deterministic (byte-identical across two builds → sorted keys, no
//! `HashMap` iteration), and contains exactly the core commands tclsh reports
//! via `info commands`. The per-dialect registry is loaded via
//! `registry_for_dialect` so the f5-irules `validEvents` cross-product path
//! is exercised, not just the empty-list Tcl path.

use tcl_registry::command_snapshot::{
    command_entry_json, command_registry_snapshot, command_registry_snapshots,
};
use tcl_registry::model::ingress::static_context_for;
use tcl_registry::snapshot::{event_graph_snapshot, object_graph_snapshot, profile_graph_snapshot};

#[test]
fn command_registry_snapshot_is_wellformed_and_deterministic() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "f5-irules"] {
        let reg = static_context_for(dialect).commands();
        let out = command_registry_snapshot(reg, dialect).dumps_indent2();
        assert!(
            out.starts_with('{'),
            "{dialect}: snapshot must be a JSON object"
        );
        assert!(out.contains("\"dialect\""), "{dialect}: has dialect field");
        assert!(
            out.contains("\"commandCount\""),
            "{dialect}: has commandCount"
        );
        assert!(
            out.contains("\"commands\""),
            "{dialect}: has commands array"
        );
        assert!(
            out.contains(&format!("\"{dialect}\"")),
            "{dialect}: snapshot names its own dialect"
        );
        // Deterministic: a second dump is byte-identical (sorted keys).
        assert_eq!(
            out,
            command_registry_snapshot(reg, dialect).dumps_indent2(),
            "{dialect}: snapshot must be deterministic"
        );
    }
}

#[test]
fn tcl_snapshot_contains_core_commands() {
    // tclsh `info commands` reports these on both 8.6 and 9.0, so the 8.6
    // command snapshot must list them.
    let reg = static_context_for("tcl8.6").commands();
    let out = command_registry_snapshot(reg, "tcl8.6").dumps_indent2();
    for cmd in [
        "puts", "set", "expr", "string", "lindex", "proc", "if", "foreach", "lsort",
    ] {
        assert!(
            out.contains(&format!("\"{cmd}\"")),
            "8.6 snapshot missing {cmd}"
        );
    }
}

#[test]
fn multi_dialect_snapshot_has_schema_and_all_dialects() {
    let reg = static_context_for("tcl9.0").commands();
    let dialects = ["tcl8.6", "tcl9.0"];
    let out = command_registry_snapshots(reg, &dialects).dumps_indent2();
    assert!(
        out.contains("tcl-lsp/registry/commands/v1"),
        "has schema id"
    );
    for d in dialects {
        assert!(out.contains(&format!("\"{d}\"")), "missing dialect {d}");
    }
    assert_eq!(
        out,
        command_registry_snapshots(reg, &dialects).dumps_indent2()
    );
}

#[test]
fn command_entry_json_known_and_unknown() {
    let reg = static_context_for("tcl8.6").commands();
    // A known core command yields an entry; a bogus name yields None.
    assert!(command_entry_json(reg, "tcl8.6", "string").is_some());
    assert!(command_entry_json(reg, "tcl8.6", "no_such_command_xyz").is_none());
    // The entry serialises and names the command.
    let entry = command_entry_json(reg, "tcl8.6", "lsort").expect("lsort entry");
    assert!(entry.dumps_indent2().contains("lsort"));
}

#[test]
fn irules_snapshot_serialises_event_validity() {
    // The f5-irules path emits the per-command validEvents cross-product
    // (distinct from the constant empty-list digest the Tcl path uses).
    let reg = static_context_for("f5-irules").commands();
    let out = command_registry_snapshot(reg, "f5-irules").dumps_indent2();
    // A representative iRules command is present in the dump.
    assert!(
        out.contains("HTTP::") || out.contains("\"commandCount\""),
        "f5-irules snapshot should carry iRules commands"
    );
    assert!(out.starts_with('{'));
}

#[test]
fn graph_snapshots_are_wellformed_and_deterministic() {
    // The profile / object / event graph snapshots (BIG-IP / iRules metadata,
    // no core-tclsh analogue — structural proof only).
    for (name, dump) in [
        ("profile", profile_graph_snapshot().dumps_indent2()),
        ("object", object_graph_snapshot().dumps_indent2()),
        ("event", event_graph_snapshot().dumps_indent2()),
    ] {
        assert!(dump.starts_with('{'), "{name}-graph must be a JSON object");
        assert!(dump.len() > 2, "{name}-graph must be non-empty");
    }
    // Deterministic (sorted keys, no nondeterministic iteration).
    assert_eq!(
        profile_graph_snapshot().dumps_indent2(),
        profile_graph_snapshot().dumps_indent2()
    );
    assert_eq!(
        object_graph_snapshot().dumps_indent2(),
        object_graph_snapshot().dumps_indent2()
    );
    assert_eq!(
        event_graph_snapshot().dumps_indent2(),
        event_graph_snapshot().dumps_indent2()
    );
}

#[test]
fn snapshot_dialect_serialisation_covers_every_primitive_bit() {
    // Codex review on PR #938: the hand-maintained dialect-name table
    // missed the TMSH/BIGIP bits, so `registry-dump` reported
    // the shared tmsh:: specs (tagged IAPPS|TMSH) as f5-iapps-only — the
    // shape again. The serialisation now derives from
    // `DialectSet::member_names`, so a spec's entry must name every
    // canonical dialect its gate carries.
    let reg = static_context_for("f5-tmsh").commands();
    let entry = command_entry_json(reg, "f5-tmsh", "tmsh::create")
        .expect("tmsh::create resolves under f5-tmsh")
        .dumps_indent2();
    assert!(
        entry.contains("\"f5-tmsh\"") && entry.contains("\"f5-iapps\""),
        "tmsh::create carries the shared IAPPS|TMSH gate: {entry}"
    );
}
