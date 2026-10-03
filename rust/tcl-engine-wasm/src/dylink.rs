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

//! What a side module's `dylink.0` section says it needs from the module that
//! loads it: the bytes of linear memory its data takes and the table slots its
//! functions take, each with its alignment (the WebAssembly tool conventions'
//! dynamic linking, `MEM_INFO`). Read by hand from the bytes, so the crate
//! carries no parser of its own beside the engine's.

/// A side module's memory and table needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Layout {
    /// Bytes of linear memory its data segments take.
    pub(crate) memory_size: u32,
    /// Their alignment, as a power of two.
    pub(crate) memory_align: u32,
    /// Table slots its element segments take.
    pub(crate) table_size: u32,
    /// Their alignment, as a power of two.
    pub(crate) table_align: u32,
}

/// The `dylink.0` subsection that carries the memory and table needs.
const MEM_INFO: u8 = 1;
/// The subsection naming the libraries the module needs loaded first.
const NEEDED: u8 = 2;

/// Read `module`'s `dylink.0` section, which the tool conventions put first.
///
/// # Errors
///
/// What is wrong with it, in words: not a module, no `dylink.0` section (an
/// ordinary module, not a side module), a section cut short, or one that names
/// libraries to load first, which this host does not do.
pub(crate) fn layout(module: &[u8]) -> Result<Layout, String> {
    if module.len() < 8 || &module[..4] != b"\0asm" {
        return Err("not a WebAssembly module".to_owned());
    }
    let not_side = || "no dylink.0 section: not a side module".to_owned();
    let mut at = 8;
    let id = *module.get(at).ok_or_else(not_side)?;
    at += 1;
    let size = leb(module, &mut at)? as usize;
    let end = at
        .checked_add(size)
        .filter(|&end| end <= module.len())
        .ok_or_else(|| "the first section is cut short".to_owned())?;
    let name_length = leb(module, &mut at)? as usize;
    let name = module
        .get(at..at + name_length)
        .ok_or_else(|| "the first section's name is cut short".to_owned())?;
    if id != 0 || name != b"dylink.0" {
        return Err(not_side());
    }
    at += name_length;
    let mut layout = Layout {
        memory_size: 0,
        memory_align: 0,
        table_size: 0,
        table_align: 0,
    };
    while at < end {
        let kind = module[at];
        at += 1;
        let length = leb(module, &mut at)? as usize;
        let next = at
            .checked_add(length)
            .filter(|&next| next <= end)
            .ok_or_else(|| "a dylink.0 subsection is cut short".to_owned())?;
        match kind {
            MEM_INFO => {
                layout.memory_size = leb(module, &mut at)?;
                layout.memory_align = leb(module, &mut at)?;
                layout.table_size = leb(module, &mut at)?;
                layout.table_align = leb(module, &mut at)?;
            }
            NEEDED => {
                let count = leb(module, &mut at)?;
                if count > 0 {
                    return Err(format!(
                        "the side module needs {count} other librar{} loaded first",
                        if count == 1 { "y" } else { "ies" }
                    ));
                }
            }
            _ => {}
        }
        at = next;
    }
    Ok(layout)
}

/// An unsigned LEB128 `u32` at `*at`, advancing past it.
fn leb(bytes: &[u8], at: &mut usize) -> Result<u32, String> {
    let mut value: u64 = 0;
    for shift in (0..35).step_by(7) {
        let byte = *bytes
            .get(*at)
            .ok_or_else(|| "a number is cut short".to_owned())?;
        *at += 1;
        value |= u64::from(byte & 0x7F) << shift;
        if byte & 0x80 == 0 {
            return u32::try_from(value).map_err(|_| "a number is too large".to_owned());
        }
    }
    Err("a number is too long".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A side module's first section, then anything.
    fn module(subsections: &[u8]) -> Vec<u8> {
        let mut payload = vec![8];
        payload.extend_from_slice(b"dylink.0");
        payload.extend_from_slice(subsections);
        let mut bytes = b"\0asm\x01\0\0\0".to_vec();
        bytes.push(0);
        bytes.push(u8::try_from(payload.len()).expect("small"));
        bytes.extend_from_slice(&payload);
        bytes
    }

    #[test]
    fn the_memory_and_table_needs_are_read_from_mem_info() {
        // MEM_INFO: memory 0x1234 bytes aligned 2^4, table 6 slots aligned 2^0.
        let bytes = module(&[MEM_INFO, 5, 0xB4, 0x24, 4, 6, 0]);
        assert_eq!(
            layout(&bytes),
            Ok(Layout {
                memory_size: 0x1234,
                memory_align: 4,
                table_size: 6,
                table_align: 0,
            })
        );
    }

    #[test]
    fn what_is_not_a_side_module_is_refused() {
        assert!(layout(b"\0asm").is_err(), "cut short");
        assert_eq!(
            layout(b"\0asm\x01\0\0\0"),
            Err("no dylink.0 section: not a side module".to_owned()),
            "no sections at all"
        );
        assert!(layout(b"not wasm at all").is_err());
        let ordinary = b"\0asm\x01\0\0\0\x01\x01\0".to_vec();
        assert_eq!(
            layout(&ordinary),
            Err("no dylink.0 section: not a side module".to_owned())
        );
        let needing = module(&[NEEDED, 6, 1, 4, b'l', b'i', b'b', b'x']);
        assert!(
            layout(&needing).is_err_and(|why| why.contains("1 other library")),
            "a side module that needs another is refused"
        );
        let cut = module(&[MEM_INFO, 9, 1]);
        assert!(layout(&cut).is_err(), "a subsection past the section's end");
    }
}
