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

//! Userspace eBPF run harness, driving emitted programs through `rbpf`'s
//! `EbpfVmFixedMbuff` over a synthetic packet (the real eBPF `data`/`data_end`
//! model: the metadata buffer carries the packet start/end pointers at offsets
//! 0 and 8, matching the codegen prologue).
//!
//! Maps are emulated in a thread-local store. rbpf helpers are bare `fn`s with
//! no captured state, so the store is shared through a `thread_local!` — each
//! test thread gets its own, so parallel tests don't interfere. A real kernel
//! uses map fds; this is test/run scaffolding. Map keys and values are passed
//! by value (v1 maps are integer→integer).
//!
//! The emulation honours the declared map schema: an **array** map is
//! preallocated and zero-filled (every in-range key is present from the
//! start, out-of-range keys are ignored), a **hash** map is sparse and
//! enforces `max_entries` (an update introducing a new key when the map is
//! full is dropped, matching `bpf_map_update_elem`'s `E2BIG`). `map_has`
//! reports presence so a caller can tell a missing key from a stored zero.

use std::cell::RefCell;
use std::collections::HashMap;

use bpf_tcl_codegen::ebpf::EbpfObject;
use bpf_tcl_ir::{MapDef, MapKind};
use rbpf::EbpfVmFixedMbuff;

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
}

thread_local! {
    /// One emulated map per declared map, indexed by map index.
    static MAPS: RefCell<Vec<EmuMap>> = const { RefCell::new(Vec::new()) };
}

/// Helper id 1: `bpf_map_lookup`-style. `r0 = MAPS[map][key]` or 0 if absent.
fn map_get_helper(map: u64, key: u64, _: u64, _: u64, _: u64) -> u64 {
    let Ok(idx) = usize::try_from(map) else {
        return 0;
    };
    MAPS.with_borrow(|m| {
        m.get(idx)
            .and_then(|t| t.store.get(&key))
            .copied()
            .unwrap_or(0)
    })
}

/// Helper id 2: `bpf_map_update`-style. `MAPS[map][key] = val`, subject to the
/// map's capacity and (for arrays) key range.
fn map_set_helper(map: u64, key: u64, val: u64, _: u64, _: u64) -> u64 {
    let Ok(idx) = usize::try_from(map) else {
        return 0;
    };
    MAPS.with_borrow_mut(|m| {
        if let Some(t) = m.get_mut(idx) {
            if !t.key_in_range(key) {
                return;
            }
            let new_key = !t.store.contains_key(&key);
            // A hash map at capacity rejects a brand-new key (E2BIG).
            if new_key && t.kind == MapKind::Hash && t.store.len() >= t.max_entries as usize {
                return;
            }
            t.store.insert(key, val);
        }
    });
    0
}

/// Helper id 3: `bpf_map_lookup`-presence. `r0 = 1` when `key` is present,
/// else `0` — the way to distinguish a missing key from a stored zero.
fn map_has_helper(map: u64, key: u64, _: u64, _: u64, _: u64) -> u64 {
    let Ok(idx) = usize::try_from(map) else {
        return 0;
    };
    MAPS.with_borrow(|m| u64::from(m.get(idx).is_some_and(|t| t.store.contains_key(&key))))
}

/// Run an emitted socket-filter program once over `packet`, returning the
/// verdict (`r0`: bytes to accept; `0` = drop).
///
/// # Errors
/// Returns rbpf's load/verify/run error rendered as a string.
pub fn run_socket_filter(obj: &EbpfObject, packet: &mut [u8]) -> Result<u64, String> {
    run_with_maps(obj, packet, 1).map(|(verdict, _)| verdict)
}

/// Run an emitted program `times` times over `packet` (sharing one map store),
/// returning the last verdict and the final contents of every emulated map.
/// Useful for asserting stateful map behaviour (e.g. a counter).
///
/// # Errors
/// Returns rbpf's load/verify/run error rendered as a string.
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
    MAPS.with_borrow_mut(|m| {
        m.clear();
        m.extend(obj.maps.iter().map(EmuMap::new));
    });

    // Copy the program so the VM's lifetime stays local and unifies with the
    // `packet` borrow rbpf's API requires.
    let prog = obj.raw.clone();
    let mut last = 0u64;
    // rbpf ties the packet borrow to the VM's lifetime, so build a fresh VM per
    // run; the thread-local map store persists across runs.
    for _ in 0..times {
        let mut vm = EbpfVmFixedMbuff::new(Some(&prog), 0, 8).map_err(|e| e.to_string())?;
        vm.register_helper(1, map_get_helper)
            .map_err(|e| e.to_string())?;
        vm.register_helper(2, map_set_helper)
            .map_err(|e| e.to_string())?;
        vm.register_helper(3, map_has_helper)
            .map_err(|e| e.to_string())?;
        last = vm.execute_program(packet).map_err(|e| e.to_string())?;
    }

    let maps = MAPS.with_borrow(|m| m.iter().map(|t| t.store.clone()).collect());
    Ok((last, maps))
}
