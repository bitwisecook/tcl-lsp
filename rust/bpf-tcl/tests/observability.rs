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

//! The userspace event channel end to end (issue #1204): a producer encodes
//! typed records against a schema-generated ABI, a bounded ring buffer transports
//! them with loss accounting under back-pressure, and a structured consumer
//! renders each record as JSON — the shape a tracepoint/observability family will
//! use once its codegen lands.

use bpf_tcl_ir::event_spec;
use bpf_tcl_ir::ringbuf::{RECORD_ABI_VERSION, RecordSchema, RingBuffer};

fn connect4_schema() -> RecordSchema {
    let spec = event_spec("CGROUP_CONNECT4").expect("event exists");
    let idx = tcl_registry_index("CGROUP_CONNECT4");
    RecordSchema::for_event(idx, spec)
}

/// Resolve an event's index in the registry table (the record header's
/// `event_type`) without depending on the registry crate directly.
fn tcl_registry_index(name: &str) -> u16 {
    // The schema helper on `RecordSchema` needs the index; derive it from the
    // known event ordering exposed through bpf-tcl-ir's known-event names.
    let idx = bpf_tcl_ir::known_event_names()
        .iter()
        .position(|n| *n == name)
        .expect("event listed");
    u16::try_from(idx).expect("event index fits u16")
}

#[test]
fn producer_to_json_consumer_stream_with_versioned_abi() {
    let schema = connect4_schema();
    // A bounded channel with room for two records.
    let mut rb = RingBuffer::new(schema.record_len() * 2);

    // Producer: three connect events (127.0.0.1:22, :80, :443).
    let events = [
        (0x0100_007f_u64, 22_u64, 2_u64),
        (0x0100_007f, 80, 2),
        (0x0100_007f, 443, 2),
    ];
    for (seq, (ip, port, fam)) in events.iter().enumerate() {
        let rec = schema.encode(u32::try_from(seq).unwrap(), &[*ip, *port, *fam]);
        rb.push(&rec);
    }
    // The third does not fit — it is dropped and counted, never truncated.
    assert_eq!(rb.dropped(), 1);

    // Consumer: drain and render each whole record as JSON.
    let lines: Vec<String> = rb
        .drain()
        .iter()
        .map(|bytes| {
            let rec = schema.decode(bytes).expect("valid record");
            assert_eq!(rec.header.abi_version, RECORD_ABI_VERSION);
            schema.to_json(&rec)
        })
        .collect();

    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("\"event\":\"CGROUP_CONNECT4\""));
    assert!(lines[0].contains("\"user_port\":22"));
    assert!(lines[1].contains("\"user_port\":80"));
    assert_eq!(rb.delivered(), 2);
}
