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

//! Transient call frames and the whole statement shapes the emitter builds on one.

use super::*;

/// Call-frame storage is dynamically distinct, so a nested generated call
/// cannot overwrite the argv or completion storage held by its caller.
#[test]
fn call_frames_are_reentrant_and_layout_checked() {
    unsafe {
        let before = tcl_codegen_call_frame_outstanding();
        let outer = tcl_codegen_call_frame_alloc(32, 8);
        assert!(!outer.is_null());
        assert_eq!(tcl_codegen_call_frame_outstanding(), before + 1);
        outer.write_bytes(0xA5, 32);

        let inner = tcl_codegen_call_frame_alloc(48, 16);
        assert!(!inner.is_null());
        assert_ne!(outer, inner);
        assert_eq!(tcl_codegen_call_frame_outstanding(), before + 2);
        inner.write_bytes(0x5A, 48);
        assert_eq!(outer.read(), 0xA5);

        assert_eq!(tcl_codegen_call_frame_free(inner), 0);
        assert_eq!(tcl_codegen_call_frame_free(outer.wrapping_add(1)), -1);
        assert_eq!(tcl_codegen_call_frame_free(outer), 0);
        assert_eq!(tcl_codegen_call_frame_outstanding(), before);
        assert_eq!(tcl_codegen_call_frame_free(outer), -1);
        assert!(tcl_codegen_call_frame_alloc(0, 4).is_null());
        assert!(tcl_codegen_call_frame_alloc(8, 3).is_null());
    }
}

/// Model the frame an emitted leaf statement allocates: `slots` owned
/// object slots followed by `completions` completion records.
///
/// This mirrors `codegen::wasm::leaf_invoke`'s layout, so the round-trip
/// tests below exercise the same allocation, adoption, and release order
/// the emitted module performs. The emitter's byte offsets are the wasm32
/// ones fixed in `tcl-runtime-api`; a native test frame uses the host's own
/// pointer width for the same shape.
struct CompiledFrame {
    base: *mut u8,
    slots: usize,
}

impl CompiledFrame {
    const STRIDE: usize = core::mem::size_of::<*mut TclObj>();
    const ALIGN: usize =
        if core::mem::align_of::<TclCompletionAbi>() > core::mem::align_of::<*mut TclObj>() {
            core::mem::align_of::<TclCompletionAbi>()
        } else {
            core::mem::align_of::<*mut TclObj>()
        };

    pub(super) fn new(slots: usize, completions: usize) -> Self {
        let objects = slots * Self::STRIDE;
        let completion_base = objects.next_multiple_of(Self::ALIGN);
        let bytes = completion_base + completions * core::mem::size_of::<TclCompletionAbi>();
        let base = tcl_codegen_call_frame_alloc(
            i32::try_from(bytes).expect("test frame fits i32"),
            i32::try_from(Self::ALIGN).expect("test frame alignment fits i32"),
        );
        assert!(!base.is_null());
        let frame = Self { base, slots };
        // The emitted prologue nulls every object slot so the single
        // cleanup path can release them null-safely.
        for slot in 0..slots {
            frame.store(slot, ptr::null_mut());
        }
        frame
    }

    fn slot_ptr(&self, slot: usize) -> *mut *mut TclObj {
        assert!(slot < self.slots);
        // SAFETY: `slot` is inside the allocation this frame owns.
        unsafe { self.base.add(slot * Self::STRIDE).cast::<*mut TclObj>() }
    }

    fn store(&self, slot: usize, value: *mut TclObj) {
        // SAFETY: the slot is inside this frame's allocation.
        unsafe { self.slot_ptr(slot).write(value) };
    }

    fn load(&self, slot: usize) -> *mut TclObj {
        // SAFETY: every slot was written by `new` before any read.
        unsafe { self.slot_ptr(slot).read() }
    }

    pub(super) fn completion(&self, index: usize) -> *mut TclCompletionAbi {
        let base = (self.slots * Self::STRIDE).next_multiple_of(Self::ALIGN);
        // SAFETY: completion storage follows the object slots.
        unsafe {
            self.base
                .add(base + index * core::mem::size_of::<TclCompletionAbi>())
                .cast::<TclCompletionAbi>()
        }
    }

    /// The emitted epilogue: release every object slot, then free the frame.
    pub(super) fn release(self) {
        for slot in 0..self.slots {
            // SAFETY: each slot is null or one owned reference.
            unsafe { tcl_obj_release(self.load(slot)) };
        }
        // SAFETY: the frame came from the matching allocator.
        assert_eq!(unsafe { tcl_codegen_call_frame_free(self.base) }, 0);
    }
}

/// Run one compiled invocation exactly as the emitter lays it out: dispatch
/// the argv run starting at `argv_slot`, stash the code, and adopt the
/// completion's owned result and options into their frame slots.
unsafe fn compiled_invoke(
    frame: &CompiledFrame,
    argv_slot: usize,
    argc: usize,
    completion: usize,
    result_slot: usize,
    options_slot: usize,
) -> i32 {
    let out = frame.completion(completion);
    // SAFETY: the argv run and completion record are inside the frame.
    unsafe {
        tcl_invoke_argv(
            frame.slot_ptr(argv_slot).cast_const(),
            i32::try_from(argc).expect("test argc fits i32"),
            out,
        );
        frame.store(result_slot, (*out).result);
        frame.store(options_slot, (*out).options);
        (*out).code
    }
}

/// The whole compiled leaf-statement shape — allocate one frame, evaluate
/// every word into it, dispatch, adopt the completion, release everything —
/// balances its allocations and leaves no outstanding call frame.
#[test]
fn compiled_leaf_statement_round_trip_is_leak_free() {
    let frames_before = tcl_codegen_call_frame_outstanding();
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        assert_eq!(tcl_eval_code(box_str(b"set value abc")), 0);

        // `string length $value`: three argv slots, then the root result
        // and options, and one completion record.
        let frame = CompiledFrame::new(5, 1);
        frame.store(0, tcl_obj_new_string_owned(b"string".as_ptr(), 6));
        frame.store(1, tcl_obj_new_string_owned(b"length".as_ptr(), 6));
        frame.store(2, tcl_codegen_var_get(b"value".as_ptr(), 5));
        assert!(!frame.load(2).is_null());

        let code = compiled_invoke(&frame, 0, 3, 0, 3, 4);
        assert_eq!(code, 0);
        assert_eq!(obj_bytes(frame.load(3)), b"3");
        frame.release();

        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
    assert_eq!(tcl_codegen_call_frame_outstanding(), frames_before);
}

/// A nested `[…]` word adopts the inner completion's owned result as the
/// outer word's value; both invocations share one frame and one cleanup.
#[test]
fn compiled_nested_word_round_trip_is_leak_free() {
    let frames_before = tcl_codegen_call_frame_outstanding();
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);

        // `string length [string tolower ABC]`:
        //   slots 0..2  outer argv (slot 2 receives the nested result)
        //   slot  3     outer options, slot 4 outer result
        //   slots 5..7  nested argv, slot 8 nested options
        let frame = CompiledFrame::new(9, 2);
        frame.store(0, tcl_obj_new_string_owned(b"string".as_ptr(), 6));
        frame.store(1, tcl_obj_new_string_owned(b"length".as_ptr(), 6));
        frame.store(5, tcl_obj_new_string_owned(b"string".as_ptr(), 6));
        frame.store(6, tcl_obj_new_string_owned(b"tolower".as_ptr(), 7));
        frame.store(7, tcl_obj_new_string_owned(b"ABC".as_ptr(), 3));

        assert_eq!(compiled_invoke(&frame, 5, 3, 1, 2, 8), 0);
        assert_eq!(obj_bytes(frame.load(2)), b"abc");
        assert_eq!(compiled_invoke(&frame, 0, 3, 0, 4, 3), 0);
        assert_eq!(obj_bytes(frame.load(4)), b"3");
        frame.release();

        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
    assert_eq!(tcl_codegen_call_frame_outstanding(), frames_before);
}

/// The abrupt-completion paths are the ones that can leak: the statement
/// leaves part-way through word evaluation, or the invocation itself
/// completes `error` / `break` / `return`. Every one still runs the single
/// cleanup path, so the counters and the frame ledger both balance.
#[test]
fn compiled_abrupt_completions_release_every_partial_word() {
    let frames_before = tcl_codegen_call_frame_outstanding();
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);

        // A missing variable aborts mid-argv: two words are already owned
        // by the frame and the third read failed.
        let frame = CompiledFrame::new(5, 1);
        frame.store(0, tcl_obj_new_string_owned(b"string".as_ptr(), 6));
        frame.store(1, tcl_obj_new_string_owned(b"length".as_ptr(), 6));
        frame.store(2, tcl_codegen_var_get(b"missing".as_ptr(), 7));
        assert!(frame.load(2).is_null(), "the read must report an error");
        frame.release();

        // `error`, `break`, and `return` all complete through the same
        // adopt-then-release path before the emitted dispatch branches.
        for (command, expected) in [(&b"error"[..], 1), (&b"break"[..], 3), (&b"return"[..], 2)] {
            let frame = CompiledFrame::new(3, 1);
            frame.store(
                0,
                tcl_obj_new_string_owned(
                    command.as_ptr(),
                    i32::try_from(command.len()).expect("test word fits i32"),
                ),
            );
            assert_eq!(compiled_invoke(&frame, 0, 1, 0, 1, 2), expected);
            frame.release();
        }

        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
    assert_eq!(tcl_codegen_call_frame_outstanding(), frames_before);
}
