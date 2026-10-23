// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Counted data transport for a callback-free visible command inventory.
//! Decoded coordinates and generation numbers supply no installed-command receipt.

use tcl_core_types::{ByteCommandSlot, ByteNamespacePath};
use tcl_dialect::TclVersion;

/// One actual visible table row captured by the engine in its current instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineCommandInventoryRow {
    /// Actual arena coordinate in the producing instance.
    pub namespace: u64,
    /// Actual command generation at the captured table slot.
    pub generation: u64,
    /// Retained namespace components and exact simple key.
    pub slot: ByteCommandSlot,
    /// Optional operand checked against actual root lookup by the producer.
    pub root_operand: Option<Vec<u8>>,
}

/// A C command inventory and its independently selected lookup recipe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineCommandInventory {
    /// Independently selected C command recipe.
    pub version: TclVersion,
    /// Every directly bound row permitted by the current surface.
    pub rows: Vec<EngineCommandInventoryRow>,
}

impl EngineCommandInventory {
    /// Checked transport size before any output allocation.
    #[must_use]
    pub fn encoded_len(&self) -> Option<usize> {
        u32::try_from(self.rows.len()).ok()?;
        let mut length = 10usize;
        for row in &self.rows {
            let parts = row.slot.namespace.as_segments();
            u32::try_from(parts.len()).ok()?;
            length = length.checked_add(25)?;
            for bytes in parts
                .iter()
                .map(AsRef::<[u8]>::as_ref)
                .chain(std::iter::once(row.slot.simple.as_ref()))
            {
                u32::try_from(bytes.len()).ok()?;
                length = length.checked_add(bytes.len())?;
            }
            length = length.checked_add(parts.len().checked_mul(4)?)?;
            if let Some(bytes) = &row.root_operand {
                u32::try_from(bytes.len()).ok()?;
                length = length.checked_add(4)?.checked_add(bytes.len())?;
            }
        }
        Some(length)
    }

    /// Encode exact namespace components and simple names, including opaque bytes.
    #[must_use]
    pub fn encode(&self) -> Option<Vec<u8>> {
        fn bytes(out: &mut Vec<u8>, value: &[u8]) -> Option<()> {
            out.extend_from_slice(&u32::try_from(value.len()).ok()?.to_le_bytes());
            out.extend_from_slice(value);
            Some(())
        }
        let mut out = Vec::with_capacity(self.encoded_len()?);
        out.extend_from_slice(b"TCIN\x01");
        out.push(match self.version {
            TclVersion::V8_4 => 0,
            TclVersion::V8_5 => 1,
            TclVersion::V8_6 => 2,
            TclVersion::V9_0 => 3,
            TclVersion::V9_1 => 4,
        });
        out.extend_from_slice(&u32::try_from(self.rows.len()).ok()?.to_le_bytes());
        for row in &self.rows {
            out.extend_from_slice(&row.namespace.to_le_bytes());
            out.extend_from_slice(&row.generation.to_le_bytes());
            let components = row.slot.namespace.as_segments();
            out.extend_from_slice(&u32::try_from(components.len()).ok()?.to_le_bytes());
            for component in components {
                bytes(&mut out, component.as_ref())?;
            }
            bytes(&mut out, row.slot.simple.as_ref())?;
            out.push(u8::from(row.root_operand.is_some()));
            if let Some(operand) = &row.root_operand {
                bytes(&mut out, operand)?;
            }
        }
        Some(out)
    }

    /// Decode one complete transport; reject impossible counts and trailing bytes.
    #[must_use]
    pub fn decode(input: &[u8]) -> Option<Self> {
        struct Reader<'a>(&'a [u8]);
        impl<'a> Reader<'a> {
            fn take(&mut self, count: usize) -> Option<&'a [u8]> {
                let (first, rest) = self.0.split_at_checked(count)?;
                self.0 = rest;
                Some(first)
            }
            fn u32(&mut self) -> Option<usize> {
                usize::try_from(u32::from_le_bytes(self.take(4)?.try_into().ok()?)).ok()
            }
            fn u64(&mut self) -> Option<u64> {
                Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
            }
            fn bytes(&mut self) -> Option<&'a [u8]> {
                let length = self.u32()?;
                self.take(length)
            }
        }
        let mut reader = Reader(input);
        if reader.take(5)? != b"TCIN\x01" {
            return None;
        }
        let version = match reader.take(1)?[0] {
            0 => TclVersion::V8_4,
            1 => TclVersion::V8_5,
            2 => TclVersion::V8_6,
            3 => TclVersion::V9_0,
            4 => TclVersion::V9_1,
            _ => return None,
        };
        let count = reader.u32()?;
        if count > reader.0.len() / 25 {
            return None;
        }
        let mut rows = Vec::with_capacity(count);
        for _ in 0..count {
            let namespace = reader.u64()?;
            let generation = reader.u64()?;
            let count = reader.u32()?;
            if count > reader.0.len() / 4 {
                return None;
            }
            let mut components = Vec::with_capacity(count);
            for _ in 0..count {
                components.push(reader.bytes()?.to_vec());
            }
            let simple = reader.bytes()?.to_vec();
            let root_operand = match reader.take(1)?[0] {
                0 => None,
                1 => Some(reader.bytes()?.to_vec()),
                _ => return None,
            };
            rows.push(EngineCommandInventoryRow {
                namespace,
                generation,
                root_operand,
                slot: ByteCommandSlot::new(
                    ByteNamespacePath::from_segments(components),
                    simple.into(),
                ),
            });
        }
        reader.0.is_empty().then_some(Self { version, rows })
    }
}
