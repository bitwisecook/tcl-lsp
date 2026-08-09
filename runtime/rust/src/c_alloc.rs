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

//! C allocation symbols for the libtommath archive in a freestanding WASM
//! runtime. The allocation size is stored ahead of the aligned user pointer
//! because C's `free` and `realloc` do not carry Rust's required layout.

#![cfg(all(target_arch = "wasm32", target_os = "unknown", have_tommath))]

use core::ptr;
use std::alloc::{alloc, alloc_zeroed, dealloc, realloc as rust_realloc, Layout};

const ALIGN: usize = 16;
const HEADER: usize = ALIGN;

fn allocation_layout(size: usize) -> Option<Layout> {
    Layout::from_size_align(HEADER.checked_add(size.max(1))?, ALIGN).ok()
}

unsafe fn store_size(base: *mut u8, size: usize) {
    // SAFETY: every allocation reserves HEADER bytes before the user pointer.
    unsafe { base.cast::<usize>().write(size.max(1)) };
}

unsafe fn allocation_base(value: *mut u8) -> *mut u8 {
    // SAFETY: callers pass a pointer returned by this module.
    unsafe { value.sub(HEADER) }
}

#[no_mangle]
/// Allocate a maximally aligned C block.
///
/// # Safety
/// The returned pointer must be released only by this module's `free` or `realloc`.
pub unsafe extern "C" fn malloc(size: usize) -> *mut u8 {
    let Some(layout) = allocation_layout(size) else {
        return ptr::null_mut();
    };
    // SAFETY: layout is non-zero and has a power-of-two alignment.
    let base = unsafe { alloc(layout) };
    if base.is_null() {
        return base;
    }
    // SAFETY: base owns layout's HEADER prefix.
    unsafe { store_size(base, size) };
    // SAFETY: HEADER lies within the allocation.
    unsafe { base.add(HEADER) }
}

#[no_mangle]
/// Allocate and zero a maximally aligned C block.
///
/// # Safety
/// The returned pointer must be released only by this module's `free` or `realloc`.
pub unsafe extern "C" fn calloc(count: usize, size: usize) -> *mut u8 {
    let Some(bytes) = count.checked_mul(size) else {
        return ptr::null_mut();
    };
    let Some(layout) = allocation_layout(bytes) else {
        return ptr::null_mut();
    };
    // SAFETY: layout is non-zero and has a power-of-two alignment.
    let base = unsafe { alloc_zeroed(layout) };
    if base.is_null() {
        return base;
    }
    // SAFETY: base owns layout's HEADER prefix.
    unsafe { store_size(base, bytes) };
    // SAFETY: HEADER lies within the allocation.
    unsafe { base.add(HEADER) }
}

#[no_mangle]
/// Release a block returned by this module.
///
/// # Safety
/// `value` must be null or a live pointer returned by `malloc`, `calloc`, or `realloc`.
pub unsafe extern "C" fn free(value: *mut u8) {
    if value.is_null() {
        return;
    }
    // SAFETY: value was returned by this module.
    let base = unsafe { allocation_base(value) };
    // SAFETY: the first header word stores the original allocation size.
    let size = unsafe { base.cast::<usize>().read() };
    if let Some(layout) = allocation_layout(size) {
        // SAFETY: base was allocated with this exact layout.
        unsafe { dealloc(base, layout) };
    }
}

#[no_mangle]
/// Resize a block returned by this module.
///
/// # Safety
/// `value` must be null or a live pointer returned by this module.
pub unsafe extern "C" fn realloc(value: *mut u8, size: usize) -> *mut u8 {
    if value.is_null() {
        // SAFETY: equivalent to C realloc(NULL, size).
        return unsafe { malloc(size) };
    }
    // SAFETY: value was returned by this module.
    let base = unsafe { allocation_base(value) };
    // SAFETY: the first header word stores the original allocation size.
    let old_size = unsafe { base.cast::<usize>().read() };
    let (Some(old_layout), Some(new_layout)) =
        (allocation_layout(old_size), allocation_layout(size))
    else {
        return ptr::null_mut();
    };
    // SAFETY: base owns old_layout and new_layout supplies the new byte count.
    let new_base = unsafe { rust_realloc(base, old_layout, new_layout.size()) };
    if new_base.is_null() {
        return new_base;
    }
    // SAFETY: new_base owns the resized allocation's HEADER prefix.
    unsafe { store_size(new_base, size) };
    // SAFETY: HEADER lies within the allocation.
    unsafe { new_base.add(HEADER) }
}
