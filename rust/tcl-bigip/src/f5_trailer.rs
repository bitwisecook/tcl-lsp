//! Parser for the F5 BIG-IP Ethernet trailer (HSB / "noise") added by
//! `tcpdump -i 0.0:nnn[p]` captures.
//!
//! Faithful Rust port of `dialects/f5/bigip/f5_trailer.py`.
//!
//! Two on-the-wire formats coexist on production fleets:
//!
//! - **Legacy** (TMOS 9.4–13.x): a chain of variable-length entries
//!   `[type:1][length:1][version:1][value: length-3]`.
//! - **DPT** (TMOS 14+): an 8-byte header `[magic:4=0xf5deb0f5][length:2]
//!   [version:2]` followed by TLVs `[provider:2][type:2][length:2]
//!   [version:2][value]`.
//!
//! The built-in schemas locate the peer-IP fields within the HIGH entries.

// DPT (new) format
/// `F5_DPT_V1_HDR_MAGIC`.
pub const DPT_HDR_MAGIC: u32 = 0xF5DE_B0F5;
/// magic(4) + length(2) + version(2).
pub const DPT_HDR_LEN: usize = 8;
/// provider(2) + type(2) + length(2) + version(2).
pub const DPT_TLV_HDR_LEN: usize = 8;
/// `F5_DPT_PROVIDER_NOISE`.
pub const DPT_PROVIDER_NOISE: u16 = 1;

// Legacy types (also used as DPT NOISE TLV types)
const LEGACY_TYPE_LOW: u8 = 1;
const LEGACY_TYPE_MED: u8 = 2;
const LEGACY_TYPE_HIGH: u8 = 3;
const LEGACY_MIN_SANE: usize = 7;
const LEGACY_MAX_SANE: usize = 140;

/// IPv6 prefix that signals an IPv4-mapped address (`::ffff:a.b.c.d`).
const IPV4_MAPPED_PREFIX: [u8; 12] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff];

/// The F5 route-domain prefix (`2620:0:c10:f501:...`), full 10-byte form.
const F5_RTDOM_PREFIX_10: [u8; 10] = [0x26, 0x20, 0, 0, 0x0c, 0x10, 0xf5, 0x01, 0, 0];

/// The `kind` of an IP-address field located inside a parsed TLV.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IpKind {
    /// 4-byte IPv4 field.
    V4,
    /// 16-byte IPv6 field.
    V6,
    /// 16-byte field that may carry an IPv4-mapped / route-domain v4.
    V6OrV4Mapped,
}

/// A single IP-address field located inside a parsed TLV.
#[derive(Clone, Copy, Debug)]
pub struct IpFieldRef {
    /// Absolute offset within the trailer bytes.
    pub offset: usize,
    /// The field kind.
    pub kind: IpKind,
}

/// One entry in the trailer chain.
#[derive(Clone, Debug)]
pub struct ParsedTlv {
    /// `type` (legacy) / `type` within the DPT provider.
    pub type_: u16,
    /// Entry version.
    pub version: u16,
    /// Full entry length (header + value).
    pub length: usize,
    /// Absolute offset within the trailer bytes.
    pub offset: usize,
    /// Located IP fields (empty when the schema has none).
    pub ip_fields: Vec<IpFieldRef>,
    /// `false` -> no registered schema; caller decides policy.
    pub schema_known: bool,
}

/// The trailer framing format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrailerFmt {
    /// Legacy (3-byte header) chain.
    Legacy,
    /// DPT (8-byte header + 8-byte TLV headers).
    Dpt,
}

/// Result of [`parse_trailer`].
#[derive(Clone, Debug)]
pub struct TrailerParse {
    /// `None` when the bytes don't look like an F5 trailer at all.
    pub fmt: Option<TrailerFmt>,
    /// One entry per TLV in the chain.
    pub tlvs: Vec<ParsedTlv>,
}

fn be_u16(data: &[u8], off: usize) -> u16 {
    u16::from_be_bytes([data[off], data[off + 1]])
}

fn be_u32(data: &[u8], off: usize) -> u32 {
    u32::from_be_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
}

/// Legacy schema lookup: returns the IP-field layout for `(type, version,
/// length)`, or `None` when no schema is registered. The `Some(empty)` case
/// means "known, no IP fields" (LOW/MED).
fn legacy_schema(type_: u8, version: u8, length: usize) -> Option<Vec<(usize, IpKind)>> {
    match (type_, version, length) {
        // Legacy HIGH v0 (length 42; F5_HIV0_LEN).
        (LEGACY_TYPE_HIGH, 0, 42) => {
            Some(vec![(6, IpKind::V6OrV4Mapped), (22, IpKind::V6OrV4Mapped)])
        }
        // LOW / MED entries carry no remote-peer IP addresses.
        // F5_LOWV94_LEN / F5_LOWV10_LEN / F5_MEDV94_LEN / F5_MEDV10_LEN /
        // F5_MEDV11_LEN.
        (LEGACY_TYPE_LOW, 0, 35 | 22) | (LEGACY_TYPE_MED, 0, 8 | 21 | 29) => Some(Vec::new()),
        _ => None,
    }
}

/// DPT schema lookup keyed by `(provider, type, version)`.
fn dpt_schema(provider: u16, type_: u16, version: u16) -> Option<Vec<(usize, IpKind)>> {
    match (provider, type_, version) {
        // DPT NOISE HIGH v1.
        (DPT_PROVIDER_NOISE, 3, 1) => {
            Some(vec![(11, IpKind::V6OrV4Mapped), (27, IpKind::V6OrV4Mapped)])
        }
        // These all carry no IP fields:
        //   DPT NOISE LOW / MED;
        //   provider 4 = TLS keylog (sub-types 0..3, versions 0/1);
        //   provider 5 = observed in real-world captures.
        (DPT_PROVIDER_NOISE, 1, 1..=4)
        | (DPT_PROVIDER_NOISE, 2, 1 | 4)
        | (4, 0..=3, 0 | 1)
        | (5, 1, 0 | 1) => Some(Vec::new()),
        _ => None,
    }
}

/// The set of legacy `type` values that have any registered schema.
fn legacy_type_known(type_: u8) -> bool {
    matches!(type_, LEGACY_TYPE_LOW | LEGACY_TYPE_MED | LEGACY_TYPE_HIGH)
}

/// Parse `data` as an F5 Ethernet trailer.
///
/// Returns a [`TrailerParse`] with `fmt == None` when the bytes don't look
/// like an F5 trailer at all (the caller should then leave them alone).
#[must_use]
pub fn parse_trailer(data: &[u8]) -> TrailerParse {
    if data.len() < 3 {
        return TrailerParse {
            fmt: None,
            tlvs: Vec::new(),
        };
    }

    // DPT format: starts with the 4-byte magic.
    if data.len() >= DPT_HDR_LEN && be_u32(data, 0) == DPT_HDR_MAGIC {
        return parse_dpt(data);
    }

    // Legacy format: first byte is a known legacy type, total entry length is
    // sane.
    if legacy_type_known(data[0]) {
        let total = data[1] as usize + 2;
        if (LEGACY_MIN_SANE..=LEGACY_MAX_SANE).contains(&total) {
            return parse_legacy(data);
        }
    }

    TrailerParse {
        fmt: None,
        tlvs: Vec::new(),
    }
}

fn parse_legacy(data: &[u8]) -> TrailerParse {
    let mut tlvs = Vec::new();
    let mut pos = 0usize;
    while pos + 3 <= data.len() {
        let type_ = data[pos];
        let wire_length = data[pos + 1] as usize;
        let version = data[pos + 2];
        let total_length = wire_length + 2;
        if !legacy_type_known(type_) {
            break;
        }
        if !(LEGACY_MIN_SANE..=LEGACY_MAX_SANE).contains(&total_length) {
            break;
        }
        if pos + total_length > data.len() {
            break;
        }
        let schema = legacy_schema(type_, version, total_length);
        let ip_fields = schema
            .as_ref()
            .map(|s| {
                s.iter()
                    .map(|&(rel_off, kind)| IpFieldRef {
                        offset: pos + rel_off,
                        kind,
                    })
                    .collect()
            })
            .unwrap_or_default();
        tlvs.push(ParsedTlv {
            type_: u16::from(type_),
            version: u16::from(version),
            length: total_length,
            offset: pos,
            ip_fields,
            schema_known: schema.is_some(),
        });
        pos += total_length;
    }
    TrailerParse {
        fmt: Some(TrailerFmt::Legacy),
        tlvs,
    }
}

fn parse_dpt(data: &[u8]) -> TrailerParse {
    let mut tlvs = Vec::new();
    if data.len() < DPT_HDR_LEN {
        return TrailerParse {
            fmt: Some(TrailerFmt::Dpt),
            tlvs,
        };
    }
    let total_len = be_u16(data, 4) as usize;
    let end = data.len().min(total_len);
    let mut pos = DPT_HDR_LEN;
    while pos + DPT_TLV_HDR_LEN <= end {
        let provider = be_u16(data, pos);
        let type_ = be_u16(data, pos + 2);
        let length = be_u16(data, pos + 4) as usize;
        let version = be_u16(data, pos + 6);
        if length < DPT_TLV_HDR_LEN || pos + length > end {
            break;
        }
        let schema = dpt_schema(provider, type_, version);
        let ip_fields = schema
            .as_ref()
            .map(|s| {
                s.iter()
                    .map(|&(rel_off, kind)| IpFieldRef {
                        offset: pos + rel_off,
                        kind,
                    })
                    .collect()
            })
            .unwrap_or_default();
        tlvs.push(ParsedTlv {
            type_,
            version,
            length,
            offset: pos,
            ip_fields,
            schema_known: schema.is_some(),
        });
        pos += length;
    }
    TrailerParse {
        fmt: Some(TrailerFmt::Dpt),
        tlvs,
    }
}

/// True if a 16-byte address is IPv4-mapped (`::ffff:a.b.c.d`).
fn looks_ipv4_mapped(sixteen: &[u8]) -> bool {
    sixteen.len() == 16 && sixteen[..12] == IPV4_MAPPED_PREFIX
}

/// True if a 16-byte address carries an F5 route-domain wrapper.
fn looks_f5_route_domain(sixteen: &[u8]) -> bool {
    sixteen.len() == 16 && sixteen[..10] == F5_RTDOM_PREFIX_10
}

/// Return `(kind, ipv4_offset_within_16)` for a 16-byte field. `kind` is
/// [`IpKind::V4`] if the bytes are IPv4-encoded (mapped or route-domain),
/// otherwise [`IpKind::V6`].
#[must_use]
pub fn classify_v6_or_v4mapped(sixteen: &[u8]) -> (IpKind, usize) {
    if looks_ipv4_mapped(sixteen) {
        return (IpKind::V4, 12);
    }
    if looks_f5_route_domain(sixteen) {
        return (IpKind::V4, 12);
    }
    (IpKind::V6, 0)
}

/// One line of the schema summary (`schema_summary`), per format.
#[derive(Debug)]
pub struct SchemaSummary {
    /// Legacy schema lines (`type=.. version=.. length=.. fields=..`).
    pub legacy: Vec<String>,
    /// DPT schema lines (`provider=.. type=.. version=.. fields=..`).
    pub dpt: Vec<String>,
}

/// Return a human-readable summary of the built-in registered schemas
/// (sorted, matching Python's `sorted(...)` over the schema dict keys).
#[must_use]
pub fn schema_summary() -> SchemaSummary {
    // Mirror the registration order/keys of f5_trailer.py, then sort.
    let mut legacy_keys: Vec<(u8, u8, usize, usize)> = vec![
        (LEGACY_TYPE_HIGH, 0, 42, 2),
        (LEGACY_TYPE_LOW, 0, 35, 0),
        (LEGACY_TYPE_LOW, 0, 22, 0),
        (LEGACY_TYPE_MED, 0, 8, 0),
        (LEGACY_TYPE_MED, 0, 21, 0),
        (LEGACY_TYPE_MED, 0, 29, 0),
    ];
    legacy_keys.sort_by_key(|k| (k.0, k.1, k.2));

    let mut dpt_keys: Vec<(u16, u16, u16, usize)> = vec![(DPT_PROVIDER_NOISE, 3, 1, 2)];
    for v in [1u16, 2, 3, 4] {
        dpt_keys.push((DPT_PROVIDER_NOISE, 1, v, 0));
    }
    for v in [1u16, 4] {
        dpt_keys.push((DPT_PROVIDER_NOISE, 2, v, 0));
    }
    for sub in [0u16, 1, 2, 3] {
        dpt_keys.push((4, sub, 0, 0));
        dpt_keys.push((4, sub, 1, 0));
    }
    dpt_keys.push((5, 1, 1, 0));
    dpt_keys.push((5, 1, 0, 0));
    dpt_keys.sort_by_key(|k| (k.0, k.1, k.2));

    SchemaSummary {
        legacy: legacy_keys
            .iter()
            .map(|&(t, v, l, f)| format!("type={t} version={v} length={l} fields={f}"))
            .collect(),
        dpt: dpt_keys
            .iter()
            .map(|&(p, t, v, f)| format!("provider={p} type={t} version={v} fields={f}"))
            .collect(),
    }
}
