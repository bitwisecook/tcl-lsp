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

use std::cell::RefCell;
use std::collections::HashMap;

use bpf_tcl_codegen::ebpf::EbpfObject;
use rbpf::EbpfVmFixedMbuff;

thread_local! {
    /// One emulated map (key → value) per declared map, indexed by map index.
    static MAPS: RefCell<Vec<HashMap<u64, u64>>> = const { RefCell::new(Vec::new()) };
}

/// Helper id 1: `bpf_map_lookup`-style. `r0 = MAPS[map][key]` or 0 if absent.
fn map_get_helper(map: u64, key: u64, _: u64, _: u64, _: u64) -> u64 {
    let Ok(idx) = usize::try_from(map) else {
        return 0;
    };
    MAPS.with_borrow(|m| m.get(idx).and_then(|t| t.get(&key)).copied().unwrap_or(0))
}

/// Helper id 2: `bpf_map_update`-style. `MAPS[map][key] = val`.
fn map_set_helper(map: u64, key: u64, val: u64, _: u64, _: u64) -> u64 {
    let Ok(idx) = usize::try_from(map) else {
        return 0;
    };
    MAPS.with_borrow_mut(|m| {
        if let Some(t) = m.get_mut(idx) {
            t.insert(key, val);
        }
    });
    0
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
/// returning the last verdict and the final state of every emulated map. Useful
/// for asserting stateful map behaviour (e.g. a counter).
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
        m.resize(obj.maps.len(), HashMap::new());
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
        last = vm.execute_program(packet).map_err(|e| e.to_string())?;
    }

    let maps = MAPS.with_borrow(Clone::clone);
    Ok((last, maps))
}
