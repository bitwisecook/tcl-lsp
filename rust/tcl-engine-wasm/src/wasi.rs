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

//! The WASI a hosted module sees: a stub for each function it imports, defined
//! for that module alone, that hands it nothing of the machine it runs on. The
//! clock reads zero, randomness is zeros, the environment and the arguments are
//! empty, no directory is open, standard output and error swallow what is
//! written, and `proc_exit` ends the evaluation. So an evaluation's answer is a
//! function of its words: run twice, it answers the same, and nothing about
//! the analysing machine reaches it.

use wasmtime::{Caller, Linker, Memory, Module, Val};

/// The WASI preview 1 import module.
pub(crate) const WASI: &str = "wasi_snapshot_preview1";

/// `errno` values (WASI preview 1).
const SUCCESS: i32 = 0;
const BADF: i32 = 8;
const NOSYS: i32 = 52;
const NOTCAPABLE: i32 = 76;

/// `filetype::character_device`, what standard input, output and error are.
const CHARACTER_DEVICE: u8 = 2;

/// A module ended the evaluation through `proc_exit`.
#[derive(Debug)]
pub(crate) struct ProcExit(pub(crate) i32);

impl std::fmt::Display for ProcExit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the module called proc_exit({})", self.0)
    }
}

impl std::error::Error for ProcExit {}

/// The store data a stub writes through: the memory the module shares.
pub(crate) trait SharedMemory {
    /// The runtime's linear memory, once the runtime is instantiated.
    fn memory(&self) -> Option<Memory>;
}

/// Define a stub on `linker` for every WASI function `module` imports.
///
/// # Errors
///
/// The linker's, when a definition fails.
pub(crate) fn define_stubs<T: SharedMemory + 'static>(
    linker: &mut Linker<T>,
    module: &Module,
) -> wasmtime::Result<()> {
    for import in module.imports() {
        if import.module() != WASI {
            continue;
        }
        let Some(ty) = import.ty().func().cloned() else {
            continue;
        };
        let name = import.name().to_owned();
        linker.func_new(
            WASI,
            import.name(),
            ty,
            move |mut caller, params, results| {
                let errno = stub(&name, &mut caller, params)?;
                if let Some(slot) = results.first_mut() {
                    *slot = Val::I32(errno);
                }
                Ok(())
            },
        )?;
    }
    Ok(())
}

/// What the stub for `name` does, answering its `errno`.
fn stub<T: SharedMemory>(
    name: &str,
    caller: &mut Caller<'_, T>,
    params: &[Val],
) -> wasmtime::Result<i32> {
    let argument = |index: usize| {
        params
            .get(index)
            .and_then(Val::i32)
            .map_or(0, |value| value.cast_unsigned() as usize)
    };
    let Some(memory) = caller.data().memory() else {
        return Ok(NOSYS);
    };
    let errno = match name {
        "proc_exit" => {
            let code = params.first().and_then(Val::i32).unwrap_or(0);
            return Err(wasmtime::Error::new(ProcExit(code)));
        }
        "random_get" => {
            let zeros = vec![0; argument(1)];
            memory.write(&mut *caller, argument(0), &zeros)?;
            SUCCESS
        }
        "clock_time_get" => {
            memory.write(&mut *caller, argument(2), &0_u64.to_le_bytes())?;
            SUCCESS
        }
        "clock_res_get" => {
            memory.write(&mut *caller, argument(1), &1_u64.to_le_bytes())?;
            SUCCESS
        }
        "environ_sizes_get" | "args_sizes_get" => {
            memory.write(&mut *caller, argument(0), &0_u32.to_le_bytes())?;
            memory.write(&mut *caller, argument(1), &0_u32.to_le_bytes())?;
            SUCCESS
        }
        "environ_get" | "args_get" => SUCCESS,
        "fd_write" if matches!(argument(0), 1 | 2) => {
            // Swallow the bytes, and say each was written.
            let mut written: u32 = 0;
            for iov in 0..argument(2) {
                let mut length = [0; 4];
                memory.read(&*caller, argument(1) + iov * 8 + 4, &mut length)?;
                written = written.saturating_add(u32::from_le_bytes(length));
            }
            memory.write(&mut *caller, argument(3), &written.to_le_bytes())?;
            SUCCESS
        }
        "fd_read" if argument(0) == 0 => {
            memory.write(&mut *caller, argument(3), &0_u32.to_le_bytes())?;
            SUCCESS
        }
        "fd_fdstat_get" if argument(0) <= 2 => {
            let mut fdstat = [0; 24];
            fdstat[0] = CHARACTER_DEVICE;
            memory.write(&mut *caller, argument(1), &fdstat)?;
            SUCCESS
        }
        name if name.starts_with("fd_") => BADF,
        name if name.starts_with("path_") => NOTCAPABLE,
        _ => NOSYS,
    };
    Ok(errno)
}
