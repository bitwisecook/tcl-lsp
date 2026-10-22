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

//! A `Vm` owned by a thread-local is dropped during thread exit, possibly
//! after the VM's own thread-locals (the native retirement queue, the Jim
//! allocator stack) have been destroyed. That is how the LSP's per-worker
//! pack hook host (`pack_hooks`' `HOST`, holding a `TclVmEngine`) dies when a
//! tokio worker exits. Touching a destroyed thread-local panics, and a panic
//! inside a destructor aborts the whole process, so every drop path must
//! degrade instead.

use std::cell::RefCell;

use tcl_vm::{Value, Vm};

thread_local! {
    // Not `const`-initialised: its destructor registers on first access, so
    // touching it before the VM exists makes it outlive (drop after) every
    // thread-local the VM registers later.
    static HOLDER: RefCell<Option<Vm>> = RefCell::new(None);
}

/// Native values inside a VM, nested deep enough that retiring them goes
/// through the queued (non-recursive) release.
fn vm_holding_native_values() -> Vm {
    let mut vm = Vm::new();
    let mut nested = Value::new_native_string_bytes(b"leaf".as_slice());
    for _ in 0..64 {
        nested = Value::list(vec![nested, Value::new_native_string_bytes(b"x".as_slice())]);
    }
    vm.set_var("held", nested).expect("set held");
    vm.eval_source("set doubled [expr {1.5 * 2}]; set l [list a b c]")
        .expect("compiles");
    vm
}

#[test]
fn a_vm_dropped_by_a_thread_local_at_thread_exit_does_not_abort() {
    let worker = std::thread::spawn(|| {
        // Register the holder's destructor first ...
        HOLDER.with(|_| ());
        // ... then build the VM, whose own values register the VM's
        // thread-locals afterwards: those are destroyed first at exit, and
        // the held VM drops after them.
        let vm = vm_holding_native_values();
        // Retire one more native value now, so the retirement queue is
        // certainly registered after the holder.
        drop(Value::list(vec![Value::new_native_string_bytes(
            b"retired".as_slice(),
        )]));
        HOLDER.with(|holder| *holder.borrow_mut() = Some(vm));
    });
    // A panic in a thread-local destructor aborts the process rather than
    // failing the join, so reaching the assertion at all is the proof.
    worker.join().expect("worker thread exits cleanly");
}

/// The same VM dropped normally still retires everything (nothing leaks
/// while the thread is alive).
#[test]
fn a_vm_dropped_while_the_thread_lives_retires_normally() {
    let held = Value::new_native_string_bytes(b"observer".as_slice());
    let weak = held.downgrade_native_object();
    let mut vm = Vm::new();
    vm.set_var("held", held).expect("set held");
    drop(vm);
    assert!(weak.upgrade().is_none(), "the held value was retired");
}
