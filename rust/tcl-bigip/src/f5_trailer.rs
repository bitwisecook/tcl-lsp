//! Parser for the F5 BIG-IP Ethernet trailer (HSB / "noise") added by
//! `tcpdump -i 0.0:nnn[p]` captures.
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

use std::collections::HashMap;

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

impl IpKind {
    /// Parse the `kind` string used in schema TOML (`v4` / `v6` /
    /// `v6_or_v4mapped`).
    fn parse(s: &str) -> Option<IpKind> {
        match s {
            "v4" => Some(IpKind::V4),
            "v6" => Some(IpKind::V6),
            "v6_or_v4mapped" => Some(IpKind::V6OrV4Mapped),
            _ => None,
        }
    }
}

/// Additional trailer schemas loaded from a `--schema` TOML overlay.
///
/// Holds the overlay schema registries that [`load_schema_overlay`] populates:
/// entries here are consulted before the built-in schemas and override a
/// matching built-in key, so an overlay can both add unknown `(type, version)`
/// combinations and fix up offsets found wrong in the field.
#[derive(Clone, Debug, Default)]
pub struct SchemaOverlay {
    legacy: HashMap<(u8, u8, usize), Vec<(usize, IpKind)>>,
    dpt: HashMap<(u16, u16, u16), Vec<(usize, IpKind)>>,
}

impl SchemaOverlay {
    /// Merge *other* into `self`; *other*'s entries override on key collision,
    /// so a later `--schema` file wins.
    pub fn merge(&mut self, other: SchemaOverlay) {
        self.legacy.extend(other.legacy);
        self.dpt.extend(other.dpt);
    }

    /// Resolve the legacy IP-field layout for `(type, version, length)`,
    /// overlay first then built-in.
    fn legacy(&self, type_: u8, version: u8, length: usize) -> Option<Vec<(usize, IpKind)>> {
        if let Some(fields) = self.legacy.get(&(type_, version, length)) {
            return Some(fields.clone());
        }
        legacy_schema(type_, version, length)
    }

    /// Resolve the DPT IP-field layout for `(provider, type, version)`, overlay
    /// first then built-in.
    fn dpt(&self, provider: u16, type_: u16, version: u16) -> Option<Vec<(usize, IpKind)>> {
        if let Some(fields) = self.dpt.get(&(provider, type_, version)) {
            return Some(fields.clone());
        }
        dpt_schema(provider, type_, version)
    }

    /// A legacy `type` is recognised if it has a built-in or overlay schema.
    fn legacy_type_known(&self, type_: u8) -> bool {
        legacy_type_known(type_) || self.legacy.keys().any(|&(t, _, _)| t == type_)
    }
}

/// Parse a `--schema` TOML overlay.
///
/// Accepts the `[[legacy]]` / `[[dpt]]` array-of-tables format, each carrying
/// integer keys and an `ip_fields` array of inline `{ offset = N, kind = "…" }`
/// tables. Returns the parsed overlay; the
/// caller threads it through [`parse_trailer_with`] / the remap engine.
///
/// # Errors
/// Returns a message when the TOML is malformed, a required key is missing, or
/// an `ip_fields` `kind` is not one of `v4` / `v6` / `v6_or_v4mapped`.
pub fn load_schema_overlay(text: &str) -> Result<SchemaOverlay, String> {
    let doc = schema_toml::parse(text)?;
    let mut overlay = SchemaOverlay::default();
    for entry in &doc.legacy {
        let type_ = entry.int_into::<u8>("type")?;
        let version = entry.int_into::<u8>("version")?;
        let length = entry.int_into::<usize>("length")?;
        overlay
            .legacy
            .insert((type_, version, length), entry.ip_fields()?);
    }
    for entry in &doc.dpt {
        let provider = entry.int_into::<u16>("provider")?;
        let type_ = entry.int_into::<u16>("type")?;
        let version = entry.int_into::<u16>("version")?;
        overlay
            .dpt
            .insert((provider, type_, version), entry.ip_fields()?);
    }
    Ok(overlay)
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
    parse_trailer_with(data, &SchemaOverlay::default())
}

/// Parse `data` as an F5 Ethernet trailer, consulting *overlay* schemas before
/// the built-ins. [`parse_trailer`] is the `overlay = default` case.
#[must_use]
pub fn parse_trailer_with(data: &[u8], overlay: &SchemaOverlay) -> TrailerParse {
    if data.len() < 3 {
        return TrailerParse {
            fmt: None,
            tlvs: Vec::new(),
        };
    }

    // DPT format: starts with the 4-byte magic.
    if data.len() >= DPT_HDR_LEN && be_u32(data, 0) == DPT_HDR_MAGIC {
        return parse_dpt(data, overlay);
    }

    // Legacy format: first byte is a known legacy type, total entry length is
    // sane.
    if overlay.legacy_type_known(data[0]) {
        let total = data[1] as usize + 2;
        if (LEGACY_MIN_SANE..=LEGACY_MAX_SANE).contains(&total) {
            return parse_legacy(data, overlay);
        }
    }

    TrailerParse {
        fmt: None,
        tlvs: Vec::new(),
    }
}

fn parse_legacy(data: &[u8], overlay: &SchemaOverlay) -> TrailerParse {
    let mut tlvs = Vec::new();
    let mut pos = 0usize;
    while pos + 3 <= data.len() {
        let type_ = data[pos];
        let wire_length = data[pos + 1] as usize;
        let version = data[pos + 2];
        let total_length = wire_length + 2;
        if !overlay.legacy_type_known(type_) {
            break;
        }
        if !(LEGACY_MIN_SANE..=LEGACY_MAX_SANE).contains(&total_length) {
            break;
        }
        if pos + total_length > data.len() {
            break;
        }
        let schema = overlay.legacy(type_, version, total_length);
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

fn parse_dpt(data: &[u8], overlay: &SchemaOverlay) -> TrailerParse {
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
        let schema = overlay.dpt(provider, type_, version);
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
    schema_summary_with(&SchemaOverlay::default())
}

/// Like [`schema_summary`] but merging any *overlay* schemas: an overlay entry
/// overrides a matching built-in key's field count and new keys are merged in
/// and re-sorted, matching Python's `schema_summary()` over the post-overlay
/// registries.
#[must_use]
pub fn schema_summary_with(overlay: &SchemaOverlay) -> SchemaSummary {
    use std::collections::BTreeMap;

    // BTreeMap keeps the (sorted) merge ordering Python's `sorted(items())`
    // produces; overlay inserts override or extend the built-in keys.
    let mut legacy: BTreeMap<(u8, u8, usize), usize> = BTreeMap::from([
        ((LEGACY_TYPE_HIGH, 0, 42), 2),
        ((LEGACY_TYPE_LOW, 0, 35), 0),
        ((LEGACY_TYPE_LOW, 0, 22), 0),
        ((LEGACY_TYPE_MED, 0, 8), 0),
        ((LEGACY_TYPE_MED, 0, 21), 0),
        ((LEGACY_TYPE_MED, 0, 29), 0),
    ]);
    for (&key, fields) in &overlay.legacy {
        legacy.insert(key, fields.len());
    }

    let mut dpt: BTreeMap<(u16, u16, u16), usize> = BTreeMap::new();
    dpt.insert((DPT_PROVIDER_NOISE, 3, 1), 2);
    for v in [1u16, 2, 3, 4] {
        dpt.insert((DPT_PROVIDER_NOISE, 1, v), 0);
    }
    for v in [1u16, 4] {
        dpt.insert((DPT_PROVIDER_NOISE, 2, v), 0);
    }
    for sub in [0u16, 1, 2, 3] {
        dpt.insert((4, sub, 0), 0);
        dpt.insert((4, sub, 1), 0);
    }
    dpt.insert((5, 1, 1), 0);
    dpt.insert((5, 1, 0), 0);
    for (&key, fields) in &overlay.dpt {
        dpt.insert(key, fields.len());
    }

    SchemaSummary {
        legacy: legacy
            .iter()
            .map(|(&(t, v, l), &f)| format!("type={t} version={v} length={l} fields={f}"))
            .collect(),
        dpt: dpt
            .iter()
            .map(|(&(p, t, v), &f)| format!("provider={p} type={t} version={v} fields={f}"))
            .collect(),
    }
}

/// Hand-parser for the small `--schema` TOML overlay format. The codebase
/// keeps its config readers dependency-free (see `redact::toml_lite`); this is
/// the schema-overlay analogue, covering integer keys and an `ip_fields` array
/// of inline `{ offset = N, kind = "…" }` tables (which may span lines). A `#`
/// begins a line comment.
mod schema_toml {
    use super::IpKind;
    use std::collections::HashMap;

    #[derive(Default)]
    pub struct Entry {
        ints: HashMap<String, i64>,
        fields: Vec<(usize, String)>,
    }

    impl Entry {
        pub fn int(&self, key: &str) -> Result<i64, String> {
            self.ints
                .get(key)
                .copied()
                .ok_or_else(|| format!("schema: entry missing `{key}`"))
        }

        /// Fetch an integer key and convert it into a narrower type, erroring
        /// cleanly when the value is out of range.
        pub fn int_into<T: TryFrom<i64>>(&self, key: &str) -> Result<T, String> {
            T::try_from(self.int(key)?)
                .map_err(|_| format!("schema: `{key}` value is out of range"))
        }

        pub fn ip_fields(&self) -> Result<Vec<(usize, IpKind)>, String> {
            let mut out = Vec::with_capacity(self.fields.len());
            for (offset, kind) in &self.fields {
                let parsed = IpKind::parse(kind)
                    .ok_or_else(|| format!("schema: unknown ip_fields kind '{kind}'"))?;
                out.push((*offset, parsed));
            }
            Ok(out)
        }
    }

    #[derive(Default)]
    pub struct Doc {
        pub legacy: Vec<Entry>,
        pub dpt: Vec<Entry>,
    }

    fn strip_comment(line: &str) -> &str {
        line.find('#').map_or(line, |i| &line[..i])
    }

    /// Collapse the document into logical lines, joining a value whose `[`
    /// array spans multiple physical lines.
    fn logical_lines(text: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut buf = String::new();
        let mut depth: i32 = 0;
        for raw in text.lines() {
            let line = strip_comment(raw);
            if depth > 0 {
                buf.push(' ');
                buf.push_str(line.trim());
            } else {
                line.trim().clone_into(&mut buf);
            }
            depth += i32::try_from(line.matches('[').count()).unwrap_or(0)
                - i32::try_from(line.matches(']').count()).unwrap_or(0);
            if depth <= 0 {
                if !buf.is_empty() {
                    out.push(std::mem::take(&mut buf));
                }
                buf.clear();
                depth = 0;
            }
        }
        if !buf.trim().is_empty() {
            out.push(buf.trim().to_owned());
        }
        out
    }

    fn parse_ip_fields(val: &str) -> Result<Vec<(usize, String)>, String> {
        let inner = val
            .trim()
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .ok_or_else(|| format!("schema: ip_fields must be an array: {val}"))?;
        let mut out = Vec::new();
        for chunk in inner.split('}') {
            let chunk = chunk.trim().trim_start_matches(',').trim();
            if chunk.is_empty() {
                continue;
            }
            let table = chunk.trim_start_matches('{').trim();
            let mut offset: Option<usize> = None;
            let mut kind: Option<String> = None;
            for kv in table.split(',') {
                let kv = kv.trim();
                if kv.is_empty() {
                    continue;
                }
                let (k, v) = kv
                    .split_once('=')
                    .ok_or_else(|| format!("schema: bad ip_fields entry: {kv}"))?;
                match k.trim() {
                    "offset" => {
                        offset = Some(
                            v.trim()
                                .parse()
                                .map_err(|_| format!("schema: bad offset: {}", v.trim()))?,
                        );
                    }
                    "kind" => kind = Some(v.trim().trim_matches('"').to_owned()),
                    other => return Err(format!("schema: unknown ip_fields key `{other}`")),
                }
            }
            out.push((
                offset.ok_or("schema: ip_fields entry missing `offset`")?,
                kind.ok_or("schema: ip_fields entry missing `kind`")?,
            ));
        }
        Ok(out)
    }

    pub fn parse(text: &str) -> Result<Doc, String> {
        let mut doc = Doc::default();
        // `0 = legacy`, `1 = dpt`, `-1 = none yet`.
        let mut section: i8 = -1;
        let mut current: Option<Entry> = None;

        let flush = |section: i8, current: &mut Option<Entry>, doc: &mut Doc| {
            if let Some(entry) = current.take() {
                match section {
                    0 => doc.legacy.push(entry),
                    1 => doc.dpt.push(entry),
                    _ => {}
                }
            }
        };

        for line in logical_lines(text) {
            if line == "[[legacy]]" {
                flush(section, &mut current, &mut doc);
                section = 0;
                current = Some(Entry::default());
                continue;
            }
            if line == "[[dpt]]" {
                flush(section, &mut current, &mut doc);
                section = 1;
                current = Some(Entry::default());
                continue;
            }
            let (key, val) = line
                .split_once('=')
                .ok_or_else(|| format!("schema: cannot parse line: {line}"))?;
            let (key, val) = (key.trim(), val.trim());
            let entry = current
                .as_mut()
                .ok_or_else(|| format!("schema: `{key}` outside a [[legacy]]/[[dpt]] table"))?;
            if key == "ip_fields" {
                entry.fields = parse_ip_fields(val)?;
            } else {
                let n: i64 = val
                    .parse()
                    .map_err(|_| format!("schema: `{key}` is not an integer: {val}"))?;
                entry.ints.insert(key.to_owned(), n);
            }
        }
        flush(section, &mut current, &mut doc);
        Ok(doc)
    }
}

#[cfg(test)]
mod overlay_tests {
    use super::{load_schema_overlay, schema_summary, schema_summary_with};

    #[test]
    fn overlay_adds_and_overrides_summary() {
        let toml = r#"
            # add an unknown legacy layout, override the built-in HIGH count
            [[legacy]]
            type = 7
            version = 0
            length = 50
            ip_fields = [
                { offset = 8,  kind = "v6_or_v4mapped" },
                { offset = 24, kind = "v4" },
            ]

            [[dpt]]
            provider = 9
            type = 2
            version = 1
            ip_fields = [ { offset = 11, kind = "v6_or_v4mapped" } ]
        "#;
        let overlay = load_schema_overlay(toml).expect("overlay parses");
        let base = schema_summary();
        let merged = schema_summary_with(&overlay);
        // The overlay added one legacy and one dpt entry.
        assert_eq!(merged.legacy.len(), base.legacy.len() + 1);
        assert_eq!(merged.dpt.len(), base.dpt.len() + 1);
        // New entries are merged and sorted in.
        assert!(
            merged
                .legacy
                .iter()
                .any(|l| l == "type=7 version=0 length=50 fields=2")
        );
        assert!(
            merged
                .dpt
                .iter()
                .any(|l| l == "provider=9 type=2 version=1 fields=1")
        );
    }

    #[test]
    fn overlay_rejects_bad_kind_and_missing_keys() {
        assert!(load_schema_overlay("[[legacy]]\ntype = 1\n").is_err());
        let bad_kind =
            "[[dpt]]\nprovider=1\ntype=1\nversion=1\nip_fields=[{offset=0,kind=\"nope\"}]\n";
        assert!(load_schema_overlay(bad_kind).is_err());
    }

    #[test]
    fn empty_overlay_summary_matches_builtin() {
        let empty = load_schema_overlay("").expect("empty parses");
        let a = schema_summary();
        let b = schema_summary_with(&empty);
        assert_eq!(a.legacy, b.legacy);
        assert_eq!(a.dpt, b.dpt);
    }
}
