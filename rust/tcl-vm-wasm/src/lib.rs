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

//! The Tcl bytecode VM as a self-contained `wasm32-unknown-unknown` cdylib.
//!
//! The VM and compiler are statically linked, so the module has **no host
//! imports and needs no WASI** (the VM's numeric tower is pure-Rust
//! `num-bigint`, not C libtommath). A host writes a Tcl script into linear
//! memory and calls [`tcl_eval`]; the VM compiles it in-module and runs it,
//! coroutines included, on its explicit activation stack. This supersedes the
//! canonical Tcl-to-WASM compiler path in `tcl-compiler`; it is an embeddable
//! execution engine, not a selectable `tcl compwasm` code-generation backend.
//!
//! ## ABI
//! Strings cross the boundary as `(ptr, len)` byte spans in the module's own
//! linear memory:
//! - `tcl_alloc(len) -> ptr` — reserve `len` bytes for the host to write a
//!   script into (host frees it with `tcl_dealloc`).
//! - `tcl_eval(ptr, len) -> u64` — run the script at `ptr[..len]`; returns a
//!   packed `(out_ptr << 32) | out_len` for the captured `puts` output (a
//!   non-OK completion appends its message). The host reads `out_ptr[..out_len]`
//!   then frees it with `tcl_dealloc`.
//! - `tcl_dealloc(ptr, len)` — free a buffer returned by `tcl_alloc`/`tcl_eval`.

use std::cell::RefCell;
use std::rc::Rc;

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_vm::Vm;

/// The injected compile service: `lower → CFG → bytecode`, the same pipeline the
/// native embedder (`tcl-vm-cli`) uses. It lowers with `lower_to_ir_for_bytecode`
/// (not the analysis `lower_to_ir`) so the VM-faithful barriers fire — e.g. a
/// branching `lmap` or a directly-nested `foreach` routes to its runtime builtin
/// rather than an inline shape the bytecode path can't compile correctly.
/// A `Write` sink capturing the interpreter's `puts` output into a shared buffer.
#[derive(Clone)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl std::io::Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Compile and run `script` on a fresh VM, returning the captured `puts` output.
/// A non-OK completion (error / uncaught return) appends its message so the host
/// observes failures rather than a silent empty result.
fn run(script: &str) -> Vec<u8> {
    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut vm = Vm::with_output(Box::new(Capture(Rc::clone(&buf))));
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    // On a non-OK completion, append the message so the host sees the failure.
    let trailer = match vm.eval_source(script) {
        Ok(comp) if comp.code.is_ok() => None,
        Ok(comp) => Some(comp.result.to_str().to_string()),
        Err(e) => Some(e.message),
    };
    let mut out = buf.borrow().clone();
    if let Some(msg) = trailer {
        out.extend_from_slice(msg.as_bytes());
    }
    out
}

/// Hand a heap buffer to the host as a packed `(ptr << 32) | len` (wasm32
/// pointers are 32-bit, so both halves fit in a `u64`). The host reads the bytes
/// and frees them with [`tcl_dealloc`].
fn into_packed(bytes: Vec<u8>) -> u64 {
    let boxed = bytes.into_boxed_slice();
    let len = boxed.len() as u64;
    let ptr = Box::into_raw(boxed).cast::<u8>() as u64;
    (ptr << 32) | len
}

/// Reserve `len` bytes in the module's linear memory for the host to write into.
/// Freed with [`tcl_dealloc`]. Returns a null pointer for a zero-length request.
#[unsafe(no_mangle)]
pub extern "C" fn tcl_alloc(len: usize) -> *mut u8 {
    if len == 0 {
        return std::ptr::null_mut();
    }
    let boxed = vec![0u8; len].into_boxed_slice();
    Box::into_raw(boxed).cast::<u8>()
}

/// Free a buffer previously returned by [`tcl_alloc`] or the output half of
/// [`tcl_eval`].
///
/// # Safety
/// `ptr`/`len` must be a buffer this module handed out and not yet freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tcl_dealloc(ptr: *mut u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }
    // SAFETY: the caller guarantees `ptr`/`len` came from `tcl_alloc`/`tcl_eval`.
    let slice = unsafe { std::slice::from_raw_parts_mut(ptr, len) };
    drop(unsafe { Box::from_raw(slice as *mut [u8]) });
}

/// Run the Tcl script at `ptr[..len]` and return its captured `puts` output as a
/// packed `(out_ptr << 32) | out_len`. The host reads `out_ptr[..out_len]` then
/// frees it with [`tcl_dealloc`].
///
/// # Safety
/// `ptr`/`len` must describe a valid readable span in this module's memory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tcl_eval(ptr: *const u8, len: usize) -> u64 {
    let script = if ptr.is_null() || len == 0 {
        String::new()
    } else {
        // SAFETY: the caller guarantees `ptr`/`len` is a readable span.
        let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
        String::from_utf8_lossy(bytes).into_owned()
    };
    into_packed(run(&script))
}
