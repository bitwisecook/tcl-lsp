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

//! Userspace eBPF run harness, driving emitted programs through the in-crate
//! [`crate::vm`] interpreter (Linux kernel semantics, cpu v4 included) over a
//! synthetic packet. The metadata buffer carries the packet start/end
//! pointers at offsets 0 and 8, matching the `rbpf` target's codegen prologue.
//!
//! Maps are emulated in a per-run store reached by the helper closure. A real
//! kernel uses map fds; this is test/run scaffolding. Map keys and values are
//! passed by value (v1 maps are integer→integer).
//!
//! The emulation honours the declared map schema: an **array** map is
//! preallocated and zero-filled (every in-range key is present from the
//! start, out-of-range keys are ignored), a **hash** map is sparse and
//! enforces `max_entries` (an update introducing a new key when the map is
//! full is dropped, matching `bpf_map_update_elem`'s `E2BIG`). `map_has`
//! reports presence so a caller can tell a missing key from a stored zero.

use std::collections::HashMap;

use bpf_tcl_codegen::ebpf::EbpfObject;
use bpf_tcl_ir::{MapDef, MapKind};

use crate::vm;

/// One emulated map: its declared schema plus its live contents.
struct EmuMap {
    kind: MapKind,
    max_entries: u32,
    store: HashMap<u64, u64>,
}

impl EmuMap {
    fn new(def: &MapDef) -> Self {
        let mut store = HashMap::new();
        // An array map is preallocated and zero-filled.
        if def.kind == MapKind::Array {
            for k in 0..u64::from(def.max_entries) {
                store.insert(k, 0);
            }
        }
        Self {
            kind: def.kind,
            max_entries: def.max_entries,
            store,
        }
    }

    fn key_in_range(&self, key: u64) -> bool {
        self.kind != MapKind::Array || key < u64::from(self.max_entries)
    }

    /// `bpf_map_update`-style, subject to capacity and (for arrays) key range.
    fn set(&mut self, key: u64, val: u64) {
        if !self.key_in_range(key) {
            return;
        }
        // A hash map at capacity rejects a brand-new key (E2BIG).
        if !self.store.contains_key(&key)
            && self.kind == MapKind::Hash
            && self.store.len() >= self.max_entries as usize
        {
            return;
        }
        self.store.insert(key, val);
    }
}

/// Helper ids 1 (`map_get`, 0 if absent), 2 (`map_set`) and 3 (`map_has`).
fn call_helper(maps: &mut [EmuMap], id: i32, [map, key, val, _, _]: [u64; 5]) -> Option<u64> {
    let map = usize::try_from(map).ok().and_then(|i| maps.get_mut(i));
    Some(match id {
        1 => map.and_then(|t| t.store.get(&key).copied()).unwrap_or(0),
        2 => {
            if let Some(t) = map {
                t.set(key, val);
            }
            0
        }
        3 => u64::from(map.is_some_and(|t| t.store.contains_key(&key))),
        _ => return None,
    })
}

/// Run an emitted socket-filter program once over `packet`, returning the
/// verdict (`r0`: bytes to accept; `0` = drop).
///
/// # Errors
/// Returns the interpreter's decode/run error rendered as a string.
pub fn run_socket_filter(obj: &EbpfObject, packet: &mut [u8]) -> Result<u64, String> {
    run_with_maps(obj, packet, 1).map(|(verdict, _)| verdict)
}

/// Run an emitted program `times` times over `packet` (sharing one map store),
/// returning the last verdict and the final contents of every emulated map.
/// Useful for asserting stateful map behaviour (e.g. a counter).
///
/// # Errors
/// Returns the interpreter's decode/run error rendered as a string.
pub fn run_socket_filter_repeated(
    obj: &EbpfObject,
    packet: &mut [u8],
    times: usize,
) -> Result<(u64, Vec<HashMap<u64, u64>>), String> {
    run_with_maps(obj, packet, times)
}

fn run_with_maps(
    obj: &EbpfObject,
    packet: &mut [u8],
    times: usize,
) -> Result<(u64, Vec<HashMap<u64, u64>>), String> {
    let mut maps: Vec<EmuMap> = obj.maps.iter().map(EmuMap::new).collect();
    let mut last = 0u64;
    for _ in 0..times {
        last = vm::execute(&obj.raw, packet, &mut |id, args| {
            call_helper(&mut maps, id, args)
        })?;
    }
    Ok((last, maps.into_iter().map(|t| t.store).collect()))
}
