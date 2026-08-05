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

//! Typed kernel→userspace event records and their ring-buffer transport
//! (issue #1204's userspace event channel).
//!
//! Observability events (tracepoints, kprobes, …) do not return a packet
//! verdict; they emit **records** to userspace. This module defines a
//! **versioned record ABI**, a **record schema** generated from an event's
//! typed context fields, a bounded **ring-buffer transport** with explicit
//! loss accounting and back-pressure behaviour, and a **structured consumer**
//! that renders records as JSON or text.
//!
//! # Wire ABI
//!
//! Every record on the wire is a fixed [`RecordHeader`] followed by the field
//! payload. All multi-byte header integers are **little-endian** (the ABI's
//! fixed transport order, independent of any field's own declared byte order).
//! The header is versioned: a consumer rejects a record whose magic or version
//! it does not recognise, so the transport can evolve without silently
//! misreading old records.
//!
//! ```text
//! offset  size  field
//! 0       4     magic      = RECORD_MAGIC ("BTR1" little-endian)
//! 4       2     abi_version = RECORD_ABI_VERSION
//! 6       2     event_type  (registry event index)
//! 8       4     payload_len (bytes of field payload following the header)
//! 12      4     seq         (monotonic per-stream sequence number)
//! = 16-byte header, then `payload_len` bytes of packed fields.
//! ```
//!
//! # Back-pressure and record drops
//!
//! The [`RingBuffer`] is bounded by a byte capacity. When a record does not
//! fit, it is **dropped** (never truncated or partially written) and the
//! [`RingBuffer::dropped`] counter is incremented — the same lossy-but-counted
//! behaviour as the kernel `BPF_MAP_TYPE_RINGBUF` reserve failing under
//! back-pressure. A consumer therefore always sees whole records, and can
//! observe exactly how many were lost.

use tcl_registry::bpf_op::{BpfCtxField, BpfEventSpec, BpfFieldOrder};

/// Magic that prefixes every record: ASCII `"BTR1"` read little-endian.
pub const RECORD_MAGIC: u32 = u32::from_le_bytes(*b"BTR1");

/// The record ABI version. Bumped on any incompatible header/payload change.
pub const RECORD_ABI_VERSION: u16 = 1;

/// The fixed record header size in bytes.
pub const HEADER_LEN: usize = 16;

/// A record's fixed, versioned header (little-endian on the wire).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordHeader {
    /// Always [`RECORD_MAGIC`].
    pub magic: u32,
    /// Always [`RECORD_ABI_VERSION`] for records this build produces.
    pub abi_version: u16,
    /// Registry event index (`BPF_EVENTS[event_type]`).
    pub event_type: u16,
    /// Length of the field payload that follows the header.
    pub payload_len: u32,
    /// Monotonic per-stream sequence number (lets a consumer detect gaps).
    pub seq: u32,
}

impl RecordHeader {
    /// Serialize to the fixed 16-byte little-endian header.
    #[must_use]
    pub fn to_bytes(self) -> [u8; HEADER_LEN] {
        let mut out = [0u8; HEADER_LEN];
        out[0..4].copy_from_slice(&self.magic.to_le_bytes());
        out[4..6].copy_from_slice(&self.abi_version.to_le_bytes());
        out[6..8].copy_from_slice(&self.event_type.to_le_bytes());
        out[8..12].copy_from_slice(&self.payload_len.to_le_bytes());
        out[12..16].copy_from_slice(&self.seq.to_le_bytes());
        out
    }

    /// Parse a header, validating magic and version.
    ///
    /// # Errors
    /// [`RecordError::Truncated`] if fewer than [`HEADER_LEN`] bytes,
    /// [`RecordError::BadMagic`] / [`RecordError::UnsupportedVersion`] on a
    /// header this build does not recognise.
    pub fn parse(bytes: &[u8]) -> Result<Self, RecordError> {
        if bytes.len() < HEADER_LEN {
            return Err(RecordError::Truncated);
        }
        let magic = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        if magic != RECORD_MAGIC {
            return Err(RecordError::BadMagic(magic));
        }
        let abi_version = u16::from_le_bytes(bytes[4..6].try_into().unwrap());
        if abi_version != RECORD_ABI_VERSION {
            return Err(RecordError::UnsupportedVersion(abi_version));
        }
        Ok(RecordHeader {
            magic,
            abi_version,
            event_type: u16::from_le_bytes(bytes[6..8].try_into().unwrap()),
            payload_len: u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            seq: u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
        })
    }
}

/// A record decode/validation error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordError {
    /// Fewer bytes than the header (or declared payload) needs.
    Truncated,
    /// The leading magic did not match [`RECORD_MAGIC`].
    BadMagic(u32),
    /// The header's ABI version is not one this build understands.
    UnsupportedVersion(u16),
    /// The payload length did not match the schema's fixed record size.
    PayloadSizeMismatch {
        /// Bytes the schema expects.
        expected: usize,
        /// Bytes the header declared.
        found: usize,
    },
}

impl core::fmt::Display for RecordError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RecordError::Truncated => write!(f, "record is truncated"),
            RecordError::BadMagic(m) => write!(f, "bad record magic {m:#010x}"),
            RecordError::UnsupportedVersion(v) => {
                write!(f, "unsupported record ABI version {v}")
            }
            RecordError::PayloadSizeMismatch { expected, found } => {
                write!(
                    f,
                    "payload size mismatch: expected {expected}, found {found}"
                )
            }
        }
    }
}

impl std::error::Error for RecordError {}

/// The byte width of a field in a record payload.
#[must_use]
fn field_bytes(width_bits: u16) -> usize {
    (width_bits as usize).div_ceil(8)
}

/// A typed record schema derived from an event's context fields. The payload is
/// each field packed to its declared width and byte order, in field order, with
/// no padding (a stable, self-describing layout).
#[derive(Debug, Clone)]
pub struct RecordSchema {
    /// The registry event index this schema describes.
    pub event_type: u16,
    /// The event's canonical name (for the JSON/text consumer).
    pub event_name: &'static str,
    /// The fields, in payload order.
    pub fields: &'static [BpfCtxField],
}

impl RecordSchema {
    /// Build the schema for a registry event, given its index.
    #[must_use]
    pub fn for_event(event_type: u16, spec: &'static BpfEventSpec) -> Self {
        RecordSchema {
            event_type,
            event_name: spec.name,
            fields: spec.context.fields,
        }
    }

    /// The fixed payload size in bytes.
    #[must_use]
    pub fn payload_len(&self) -> usize {
        self.fields.iter().map(|f| field_bytes(f.width_bits)).sum()
    }

    /// The whole record size (header + payload).
    #[must_use]
    pub fn record_len(&self) -> usize {
        HEADER_LEN + self.payload_len()
    }

    /// Encode one record from field values (one per schema field, in order).
    /// A value wider than its field is truncated to the field width.
    ///
    /// # Panics
    /// If `values.len()` does not equal the number of schema fields.
    #[must_use]
    pub fn encode(&self, seq: u32, values: &[u64]) -> Vec<u8> {
        assert_eq!(
            values.len(),
            self.fields.len(),
            "encode expects one value per field"
        );
        let payload_len = self.payload_len();
        let header = RecordHeader {
            magic: RECORD_MAGIC,
            abi_version: RECORD_ABI_VERSION,
            event_type: self.event_type,
            payload_len: u32::try_from(payload_len).unwrap_or(u32::MAX),
            seq,
        };
        let mut out = Vec::with_capacity(HEADER_LEN + payload_len);
        out.extend_from_slice(&header.to_bytes());
        for (field, &value) in self.fields.iter().zip(values) {
            pack_field(&mut out, field, value);
        }
        out
    }

    /// Decode one record's field values, validating the header and payload size
    /// against this schema.
    ///
    /// # Errors
    /// A header/version/size validation error.
    pub fn decode(&self, bytes: &[u8]) -> Result<DecodedRecord, RecordError> {
        let header = RecordHeader::parse(bytes)?;
        let payload = &bytes[HEADER_LEN..];
        let expected = self.payload_len();
        if header.payload_len as usize != expected || payload.len() < expected {
            return Err(RecordError::PayloadSizeMismatch {
                expected,
                found: header.payload_len as usize,
            });
        }
        let mut values = Vec::with_capacity(self.fields.len());
        let mut off = 0usize;
        for field in self.fields {
            let n = field_bytes(field.width_bits);
            values.push(unpack_field(&payload[off..off + n], field));
            off += n;
        }
        Ok(DecodedRecord { header, values })
    }

    /// Render a decoded record as a single-line JSON object.
    #[must_use]
    pub fn to_json(&self, record: &DecodedRecord) -> String {
        use core::fmt::Write as _;
        let mut s = String::new();
        let _ = write!(
            s,
            "{{\"event\":\"{}\",\"seq\":{},\"fields\":{{",
            self.event_name, record.header.seq
        );
        for (i, (field, value)) in self.fields.iter().zip(&record.values).enumerate() {
            if i > 0 {
                s.push(',');
            }
            let _ = write!(s, "\"{}\":{value}", field.name);
        }
        s.push_str("}}");
        s
    }

    /// Render a decoded record as a compact text line.
    #[must_use]
    pub fn to_text(&self, record: &DecodedRecord) -> String {
        use core::fmt::Write as _;
        let mut s = format!("{} seq={}", self.event_name, record.header.seq);
        for (field, value) in self.fields.iter().zip(&record.values) {
            let _ = write!(s, " {}={value}", field.name);
        }
        s
    }
}

/// Pack one field value into the buffer at its declared width and byte order.
fn pack_field(out: &mut Vec<u8>, field: &BpfCtxField, value: u64) {
    let n = field_bytes(field.width_bits);
    let le = value.to_le_bytes();
    match field.order {
        // Host order: little-endian on this build's little-endian reference.
        BpfFieldOrder::Host => out.extend_from_slice(&le[..n]),
        // Network order: big-endian — take the low `n` bytes, most-significant
        // first.
        BpfFieldOrder::Network => {
            for i in (0..n).rev() {
                out.push(le[i]);
            }
        }
    }
}

/// Unpack one field value from `n` bytes at its declared byte order.
fn unpack_field(bytes: &[u8], field: &BpfCtxField) -> u64 {
    let mut v = 0u64;
    match field.order {
        BpfFieldOrder::Host => {
            for (i, &b) in bytes.iter().enumerate() {
                v |= u64::from(b) << (8 * i);
            }
        }
        BpfFieldOrder::Network => {
            for &b in bytes {
                v = (v << 8) | u64::from(b);
            }
        }
    }
    v
}

/// A record decoded against its schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedRecord {
    /// The validated header.
    pub header: RecordHeader,
    /// Field values, in schema order.
    pub values: Vec<u64>,
}

/// A bounded, lossy-but-counted userspace ring buffer.
///
/// Records are appended whole. When one does not fit the remaining capacity it
/// is dropped and [`RingBuffer::dropped`] is incremented — modelling
/// `BPF_MAP_TYPE_RINGBUF` reserve failures under back-pressure. Draining with
/// [`RingBuffer::drain`] frees the whole capacity again.
#[derive(Debug, Clone)]
pub struct RingBuffer {
    capacity: usize,
    buf: Vec<u8>,
    /// Record boundaries: byte length of each queued record, in order.
    lengths: Vec<usize>,
    dropped: u64,
    delivered: u64,
}

impl RingBuffer {
    /// Create a ring buffer with `capacity` bytes of record storage.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        RingBuffer {
            capacity,
            buf: Vec::new(),
            lengths: Vec::new(),
            dropped: 0,
            delivered: 0,
        }
    }

    /// Append a whole record. Returns `true` if it was stored, `false` if it was
    /// dropped for lack of capacity (and `dropped` was incremented).
    pub fn push(&mut self, record: &[u8]) -> bool {
        if record.len() > self.capacity || self.buf.len() + record.len() > self.capacity {
            self.dropped = self.dropped.saturating_add(1);
            return false;
        }
        self.buf.extend_from_slice(record);
        self.lengths.push(record.len());
        true
    }

    /// Drain every queued record, returning them in FIFO order and freeing the
    /// buffer. Increments `delivered` by the number returned.
    pub fn drain(&mut self) -> Vec<Vec<u8>> {
        let mut out = Vec::with_capacity(self.lengths.len());
        let mut off = 0usize;
        for &len in &self.lengths {
            out.push(self.buf[off..off + len].to_vec());
            off += len;
        }
        self.delivered = self.delivered.saturating_add(out.len() as u64);
        self.buf.clear();
        self.lengths.clear();
        out
    }

    /// The number of records dropped for lack of capacity (loss accounting).
    #[must_use]
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// The number of records drained to a consumer over this buffer's life.
    #[must_use]
    pub fn delivered(&self) -> u64 {
        self.delivered
    }

    /// The number of records currently queued.
    #[must_use]
    pub fn queued(&self) -> usize {
        self.lengths.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::bpf_op::lookup_bpf_event;

    fn schema(name: &str) -> RecordSchema {
        let spec = lookup_bpf_event(name).expect("event exists");
        let idx = tcl_registry::bpf_op::BPF_EVENTS
            .iter()
            .position(|e| e.name == spec.name)
            .unwrap();
        RecordSchema::for_event(u16::try_from(idx).unwrap(), spec)
    }

    #[test]
    fn header_round_trips_and_validates() {
        let h = RecordHeader {
            magic: RECORD_MAGIC,
            abi_version: RECORD_ABI_VERSION,
            event_type: 3,
            payload_len: 12,
            seq: 42,
        };
        let bytes = h.to_bytes();
        assert_eq!(RecordHeader::parse(&bytes).unwrap(), h);
    }

    #[test]
    fn bad_magic_and_version_are_rejected() {
        let mut bytes = RecordHeader {
            magic: RECORD_MAGIC,
            abi_version: RECORD_ABI_VERSION,
            event_type: 0,
            payload_len: 0,
            seq: 0,
        }
        .to_bytes();
        bytes[0] ^= 0xff;
        assert!(matches!(
            RecordHeader::parse(&bytes),
            Err(RecordError::BadMagic(_))
        ));

        let mut bytes = RecordHeader {
            magic: RECORD_MAGIC,
            abi_version: RECORD_ABI_VERSION + 1,
            event_type: 0,
            payload_len: 0,
            seq: 0,
        }
        .to_bytes();
        // Fix magic (to_bytes already wrote it) — flip only version.
        bytes[4] = 99;
        assert!(matches!(
            RecordHeader::parse(&bytes),
            Err(RecordError::UnsupportedVersion(99))
        ));
    }

    #[test]
    fn record_round_trips_field_values_in_declared_order() {
        // CGROUP_CONNECT4 has user_ip4 (net,32), user_port (net,32), family (host,32).
        let s = schema("CGROUP_CONNECT4");
        let values = [0x0100_007f, 22, 2]; // 127.0.0.1, port 22, AF_INET
        let bytes = s.encode(7, &values);
        assert_eq!(bytes.len(), s.record_len());
        let decoded = s.decode(&bytes).expect("decodes");
        assert_eq!(decoded.header.seq, 7);
        assert_eq!(decoded.values, values.to_vec());
    }

    #[test]
    fn network_order_field_is_big_endian_on_the_wire() {
        let s = schema("CGROUP_CONNECT4");
        // user_port = 22 network order -> big-endian 0x00000016 in payload.
        let bytes = s.encode(0, &[0, 22, 2]);
        let payload = &bytes[HEADER_LEN..];
        // user_ip4 occupies the first 4 bytes; user_port the next 4.
        assert_eq!(&payload[4..8], &[0x00, 0x00, 0x00, 0x16]);
    }

    #[test]
    fn json_and_text_render_named_fields() {
        let s = schema("CGROUP_CONNECT4");
        let bytes = s.encode(1, &[10, 20, 2]);
        let rec = s.decode(&bytes).unwrap();
        let json = s.to_json(&rec);
        assert!(json.contains("\"event\":\"CGROUP_CONNECT4\""));
        assert!(json.contains("\"user_ip4\":10"));
        assert!(json.contains("\"user_port\":20"));
        let text = s.to_text(&rec);
        assert!(text.starts_with("CGROUP_CONNECT4 seq=1"));
        assert!(text.contains("family=2"));
    }

    #[test]
    fn ring_buffer_counts_drops_under_back_pressure() {
        let s = schema("CGROUP_CONNECT4");
        let rec = s.encode(0, &[0, 0, 0]);
        // Room for exactly one record.
        let mut rb = RingBuffer::new(rec.len());
        assert!(rb.push(&rec));
        assert!(!rb.push(&rec)); // full -> dropped
        assert_eq!(rb.dropped(), 1);
        assert_eq!(rb.queued(), 1);
        let drained = rb.drain();
        assert_eq!(drained.len(), 1);
        assert_eq!(rb.delivered(), 1);
        // After draining, capacity is free again.
        assert!(rb.push(&rec));
        assert_eq!(rb.dropped(), 1);
    }

    #[test]
    fn oversized_record_is_dropped_not_truncated() {
        let mut rb = RingBuffer::new(8);
        assert!(!rb.push(&[0u8; 16]));
        assert_eq!(rb.dropped(), 1);
        assert_eq!(rb.queued(), 0);
    }

    #[test]
    fn payload_size_mismatch_is_detected() {
        let s = schema("CGROUP_CONNECT4");
        let mut bytes = s.encode(0, &[0, 0, 0]);
        // Corrupt the declared payload_len.
        bytes[8] = 0xff;
        assert!(matches!(
            s.decode(&bytes),
            Err(RecordError::PayloadSizeMismatch { .. })
        ));
    }
}
