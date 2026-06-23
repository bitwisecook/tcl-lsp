//! Minimal PCAPNG reader/writer for `f5 pcap-remap`.
//!
//! PCAPNG is a chain of variably-sized blocks, each with
//! `[block_type:4][block_total_length:4][body...][block_total_length:4]`.
//! For our purposes we recognise SHB (sets section endianness), IDB (gives the
//! per-interface link-layer type) and EPB (the packet); every other block is
//! round-tripped byte-for-byte.

// PCAPNG block lengths are 32-bit on the wire; the usize<->u32 conversions are
// bounded by file size and width-checked by construction.
#![allow(clippy::cast_possible_truncation)]

/// Section Header Block.
pub const BLOCK_TYPE_SHB: u32 = 0x0A0D_0D0A;
/// Interface Description Block.
pub const BLOCK_TYPE_IDB: u32 = 0x0000_0001;
/// Enhanced Packet Block.
pub const BLOCK_TYPE_EPB: u32 = 0x0000_0006;
/// Name Resolution Block.
pub const BLOCK_TYPE_NRB: u32 = 0x0000_0004;
/// Decryption Secrets Block.
pub const BLOCK_TYPE_DSB: u32 = 0x0000_000A;

/// NRB record type: end-of-records sentinel.
pub const NRB_RECORD_END: u16 = 0x0000;
/// NRB record type: IPv4 address → name(s).
pub const NRB_RECORD_IPV4: u16 = 0x0001;
/// NRB record type: IPv6 address → name(s).
pub const NRB_RECORD_IPV6: u16 = 0x0002;

/// DSB secrets type: NSS-format TLS keylog text (`"TLSK"`).
pub const DSB_SECRETS_TYPE_TLS: u32 = 0x544C_534B;

const SHB_BYTE_ORDER_MAGIC: u32 = 0x1A2B_3C4D;

/// Per-section endianness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Endian {
    /// Little-endian section.
    Little,
    /// Big-endian section.
    Big,
}

impl Endian {
    fn u16(self, b: [u8; 2]) -> u16 {
        match self {
            Endian::Little => u16::from_le_bytes(b),
            Endian::Big => u16::from_be_bytes(b),
        }
    }
    fn u32(self, b: [u8; 4]) -> u32 {
        match self {
            Endian::Little => u32::from_le_bytes(b),
            Endian::Big => u32::from_be_bytes(b),
        }
    }
    fn u32_bytes(self, v: u32) -> [u8; 4] {
        match self {
            Endian::Little => v.to_le_bytes(),
            Endian::Big => v.to_be_bytes(),
        }
    }
    fn u16_bytes(self, v: u16) -> [u8; 2] {
        match self {
            Endian::Little => v.to_le_bytes(),
            Endian::Big => v.to_be_bytes(),
        }
    }
}

/// One raw block read from a PCAPNG file.
#[derive(Clone, Debug)]
pub struct PcapngBlock {
    /// Block type tag.
    pub block_type: u32,
    /// Body bytes between the two `block_total_length` fields.
    pub body: Vec<u8>,
    /// Section endianness in force for this block.
    pub endian: Endian,
    /// EPB only: interface id.
    pub interface_id: Option<u32>,
    /// EPB only: captured length.
    pub captured_len: Option<u32>,
    /// EPB only: original length.
    pub original_len: Option<u32>,
    /// EPB only: timestamp high half.
    pub timestamp_high: Option<u32>,
    /// EPB only: timestamp low half.
    pub timestamp_low: Option<u32>,
    /// EPB only: the captured packet bytes (mutated in place by the rewriter).
    pub packet_data: Option<Vec<u8>>,
    /// EPB only: trailing option bytes after the (padded) packet data.
    pub options: Option<Vec<u8>>,
    /// IDB only: the link-layer type.
    pub linktype: Option<u16>,
}

fn pad32(n: usize) -> usize {
    (n + 3) & !3
}

/// True iff `four_bytes` is the PCAPNG SHB block-type magic.
#[must_use]
pub fn is_pcapng_magic(four_bytes: &[u8]) -> bool {
    four_bytes.len() >= 4
        && u32::from_le_bytes([four_bytes[0], four_bytes[1], four_bytes[2], four_bytes[3]])
            == BLOCK_TYPE_SHB
}

/// Determine the section endianness from the first SHB (peek, no consume).
fn peek_endian(data: &[u8]) -> Result<Endian, String> {
    if data.len() < 12 {
        return Err("pcapng: file too short for SHB".to_owned());
    }
    let block_type = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if block_type != BLOCK_TYPE_SHB {
        return Err(format!(
            "pcapng: first block must be SHB (0x0a0d0d0a), got 0x{block_type:08x}"
        ));
    }
    let magic = [data[8], data[9], data[10], data[11]];
    if u32::from_le_bytes(magic) == SHB_BYTE_ORDER_MAGIC {
        return Ok(Endian::Little);
    }
    if u32::from_be_bytes(magic) == SHB_BYTE_ORDER_MAGIC {
        return Ok(Endian::Big);
    }
    Err(format!(
        "pcapng: SHB byte-order magic not recognised: {}",
        hex(&magic)
    ))
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut s = String::new();
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Read every block from `data`.
///
/// # Errors
/// Returns an error string on a malformed / truncated block stream.
pub fn read_blocks(data: &[u8]) -> Result<Vec<PcapngBlock>, String> {
    let mut endian = peek_endian(data)?;
    let mut out = Vec::new();
    let mut cursor = 0usize;

    loop {
        if cursor == data.len() {
            return Ok(out);
        }
        if data.len() - cursor < 8 {
            return Err("pcapng: truncated block header".to_owned());
        }
        let block_type = endian.u32([
            data[cursor],
            data[cursor + 1],
            data[cursor + 2],
            data[cursor + 3],
        ]);
        let block_total_len = endian.u32([
            data[cursor + 4],
            data[cursor + 5],
            data[cursor + 6],
            data[cursor + 7],
        ]) as usize;
        if block_total_len < 12 || !block_total_len.is_multiple_of(4) {
            return Err(format!(
                "pcapng: invalid block_total_length {block_total_len} for type 0x{block_type:08x}"
            ));
        }
        if cursor + block_total_len > data.len() {
            return Err("pcapng: truncated block body".to_owned());
        }
        let body = data[cursor + 8..cursor + block_total_len - 4].to_vec();
        let trailing = endian.u32([
            data[cursor + block_total_len - 4],
            data[cursor + block_total_len - 3],
            data[cursor + block_total_len - 2],
            data[cursor + block_total_len - 1],
        ]) as usize;
        if trailing != block_total_len {
            return Err(format!(
                "pcapng: trailing length 0x{trailing:x} != header 0x{block_total_len:x}"
            ));
        }
        cursor += block_total_len;

        if block_type == BLOCK_TYPE_SHB {
            // Re-derive endian from this section's magic.
            let magic = u32::from_le_bytes([body[0], body[1], body[2], body[3]]);
            endian = if magic == SHB_BYTE_ORDER_MAGIC {
                Endian::Little
            } else {
                Endian::Big
            };
            out.push(block(block_type, body, endian));
            continue;
        }

        if block_type == BLOCK_TYPE_IDB {
            let linktype = endian.u16([body[0], body[1]]);
            let mut b = block(block_type, body, endian);
            b.linktype = Some(linktype);
            out.push(b);
            continue;
        }

        if block_type == BLOCK_TYPE_EPB {
            let interface_id = endian.u32([body[0], body[1], body[2], body[3]]);
            let ts_high = endian.u32([body[4], body[5], body[6], body[7]]);
            let ts_low = endian.u32([body[8], body[9], body[10], body[11]]);
            let cap_len = endian.u32([body[12], body[13], body[14], body[15]]);
            let orig_len = endian.u32([body[16], body[17], body[18], body[19]]);
            let packet_off = 20usize;
            let packet_end = packet_off + cap_len as usize;
            let packet_data = body[packet_off..packet_end].to_vec();
            let options = body[pad32(packet_end)..].to_vec();
            let mut b = block(block_type, body, endian);
            b.interface_id = Some(interface_id);
            b.timestamp_high = Some(ts_high);
            b.timestamp_low = Some(ts_low);
            b.captured_len = Some(cap_len);
            b.original_len = Some(orig_len);
            b.packet_data = Some(packet_data);
            b.options = Some(options);
            out.push(b);
            continue;
        }

        out.push(block(block_type, body, endian));
    }
}

fn block(block_type: u32, body: Vec<u8>, endian: Endian) -> PcapngBlock {
    PcapngBlock {
        block_type,
        body,
        endian,
        interface_id: None,
        captured_len: None,
        original_len: None,
        timestamp_high: None,
        timestamp_low: None,
        packet_data: None,
        options: None,
        linktype: None,
    }
}

/// Serialise `block` to `out`.
///
/// # Errors
/// Returns an error string when an EPB's mutated packet is not
/// length-preserving.
pub fn write_block(out: &mut Vec<u8>, block: &PcapngBlock) -> Result<(), String> {
    let endian = block.endian;
    let rebuilt_epb = if block.block_type == BLOCK_TYPE_EPB
        && let Some(packet) = block.packet_data.as_ref()
        && let Some(captured) = block.captured_len
        && let Some(original) = block.original_len
        && let Some(options) = block.options.as_ref()
    {
        let cap_len = packet.len();
        if cap_len as u32 != captured {
            return Err(format!(
                "pcapng: EPB packet length changed ({cap_len} != {captured}); \
                 the rewriter must be length-preserving"
            ));
        }
        let mut b = Vec::new();
        b.extend_from_slice(&endian.u32_bytes(block.interface_id.unwrap_or(0)));
        b.extend_from_slice(&endian.u32_bytes(block.timestamp_high.unwrap_or(0)));
        b.extend_from_slice(&endian.u32_bytes(block.timestamp_low.unwrap_or(0)));
        b.extend_from_slice(&endian.u32_bytes(cap_len as u32));
        b.extend_from_slice(&endian.u32_bytes(original));
        b.extend_from_slice(packet);
        b.resize(b.len() + (pad32(cap_len) - cap_len), 0);
        b.extend_from_slice(options);
        Some(b)
    } else {
        None
    };
    let body: Vec<u8> = rebuilt_epb.unwrap_or_else(|| block.body.clone());

    let block_total_len = (12 + body.len()) as u32;
    out.extend_from_slice(&endian.u32_bytes(block.block_type));
    out.extend_from_slice(&endian.u32_bytes(block_total_len));
    out.extend_from_slice(&body);
    out.extend_from_slice(&endian.u32_bytes(block_total_len));
    Ok(())
}

/// Build a Name Resolution Block (NRB) from per-address name lists.
///
/// `v4_names` maps 4-byte big-endian IPv4 packed addresses to a list of UTF-8
/// names; `v6_names` maps 16-byte big-endian IPv6 packed addresses similarly.
/// The order of the slices is preserved. A terminating `nrb_record_end`
/// (type 0, length 0) record is always emitted, per the PCAPNG spec.
///
/// # Panics
/// Panics if a v4 address is not 4 bytes or a v6 address is not 16 bytes
/// (callers always pass packed addresses).
#[must_use]
pub fn build_nrb_block(
    endian: Endian,
    v4_names: &[([u8; 4], Vec<String>)],
    v6_names: &[([u8; 16], Vec<String>)],
) -> PcapngBlock {
    let mut body: Vec<u8> = Vec::new();

    let mut append_record = |rec_type: u16, addr: &[u8], names: &[String]| {
        let mut names_blob: Vec<u8> = Vec::new();
        for n in names {
            if n.is_empty() {
                continue;
            }
            names_blob.extend_from_slice(n.as_bytes());
            names_blob.push(0);
        }
        if names_blob.is_empty() {
            return;
        }
        let mut value: Vec<u8> = Vec::with_capacity(addr.len() + names_blob.len());
        value.extend_from_slice(addr);
        value.extend_from_slice(&names_blob);
        body.extend_from_slice(&endian.u16_bytes(rec_type));
        body.extend_from_slice(&endian.u16_bytes(value.len() as u16));
        body.extend_from_slice(&value);
        let pad = value.len().wrapping_neg() & 3;
        body.resize(body.len() + pad, 0);
    };

    for (addr, names) in v4_names {
        append_record(NRB_RECORD_IPV4, addr, names);
    }
    for (addr, names) in v6_names {
        append_record(NRB_RECORD_IPV6, addr, names);
    }

    // End-of-records sentinel — required even with no name records.
    body.extend_from_slice(&endian.u16_bytes(NRB_RECORD_END));
    body.extend_from_slice(&endian.u16_bytes(0));

    block(BLOCK_TYPE_NRB, body, endian)
}

/// Build a Decryption Secrets Block (DSB) carrying `secrets_data`.
#[must_use]
pub fn build_dsb_block(endian: Endian, secrets_type: u32, secrets_data: &[u8]) -> PcapngBlock {
    let mut body: Vec<u8> = Vec::new();
    body.extend_from_slice(&endian.u32_bytes(secrets_type));
    body.extend_from_slice(&endian.u32_bytes(secrets_data.len() as u32));
    body.extend_from_slice(secrets_data);
    let pad = secrets_data.len().wrapping_neg() & 3;
    body.resize(body.len() + pad, 0);
    block(BLOCK_TYPE_DSB, body, endian)
}

#[cfg(test)]
mod nrb_dsb_tests {
    use super::*;

    #[test]
    fn nrb_round_trips_with_records_and_sentinel() {
        let v4 = vec![(
            [10u8, 0, 0, 1],
            vec!["vs-a".to_owned(), "irule-b".to_owned()],
        )];
        let v6: Vec<([u8; 16], Vec<String>)> = vec![];
        let block = build_nrb_block(Endian::Little, &v4, &v6);
        assert_eq!(block.block_type, BLOCK_TYPE_NRB);
        // record: type(0001) len(0010) addr(4) "vs-a\0irule-b\0" = 4+5+8=17 -> pad to 20
        // The value length is 17 (not padded).
        let b = &block.body;
        // First record header.
        assert_eq!(u16::from_le_bytes([b[0], b[1]]), NRB_RECORD_IPV4);
        assert_eq!(u16::from_le_bytes([b[2], b[3]]), 17);
        assert_eq!(&b[4..8], &[10, 0, 0, 1]);
        // Padded to a multiple of 4: 4(hdr)+17(value)=21 -> +3 pad = 24, then end sentinel(4).
        assert_eq!(b.len(), 24 + 4);
        // End sentinel.
        let tail = &b[b.len() - 4..];
        assert_eq!(u16::from_le_bytes([tail[0], tail[1]]), NRB_RECORD_END);
        assert_eq!(u16::from_le_bytes([tail[2], tail[3]]), 0);
    }

    #[test]
    fn dsb_carries_padded_secrets() {
        let block = build_dsb_block(Endian::Little, DSB_SECRETS_TYPE_TLS, b"abc");
        assert_eq!(block.block_type, BLOCK_TYPE_DSB);
        let b = &block.body;
        assert_eq!(
            u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            DSB_SECRETS_TYPE_TLS
        );
        assert_eq!(u32::from_le_bytes([b[4], b[5], b[6], b[7]]), 3);
        assert_eq!(&b[8..11], b"abc");
        // 3 bytes secrets -> 1 byte padding.
        assert_eq!(b.len(), 8 + 3 + 1);
        assert_eq!(b[11], 0);
    }

    #[test]
    fn empty_nrb_is_just_the_sentinel() {
        let block = build_nrb_block(Endian::Big, &[], &[]);
        assert_eq!(block.body, vec![0, 0, 0, 0]);
    }
}
