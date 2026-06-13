// ============================================================================
// SPIKE -- throwaway proof-of-concept. NOT the final design.
// Do not derive the production runtime, ABI, or tcl.h shape from this file;
// see runtime/rust-spike/README.md and docs/design/runtime/c-extension-abi.md
// for the real plan. Memory is leaked, there is one global command table, and
// only the API slice the sample extension needs is implemented.
// ============================================================================
//
//! Dynamic-linking C-extension spike: the **runtime** module.
//!
//! Compiled to a `wasm32-unknown-unknown` cdylib that exports `memory`, the
//! `__indirect_function_table`, the Tcl C API (imported by the side module),
//! and a few `spike_*` helpers the host loader drives. The companion
//! `loader.py` instantiates this module, then loads the PIC side module
//! (`pkga.c` built `-fPIC -shared`) at host-allocated memory/table bases, so
//! the side module's commands land in *this* module's shared table and memory.
//!
//! The Tcl C API implementation is identical in spirit to the static-link
//! spike; the point of this crate is to prove the *dynamic* wiring works:
//! cross-module shared memory, shared function table, and `call_indirect` from
//! the runtime into a separately-loaded extension.

#![no_std]
#![allow(non_snake_case, non_camel_case_types, static_mut_refs, clippy::missing_safety_doc)]

use core::ffi::{c_char, c_int, c_void};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

// ---- Tcl C ABI types (must match ../../include/tcl.h) ----
pub type Tcl_Size = isize;
pub type Tcl_WideInt = i64;

#[repr(C)]
pub struct Tcl_Obj {
    pub refCount: Tcl_Size,
    pub bytes: *mut c_char,
    pub length: Tcl_Size,
    pub typePtr: *const c_void,
    pub internalRep: u64,
}

pub type Tcl_ObjCmdProc =
    extern "C" fn(*mut c_void, *mut Interp, c_int, *const *mut Tcl_Obj) -> c_int;
pub type Tcl_CmdDeleteProc = extern "C" fn(*mut c_void);

#[repr(C)]
pub struct Interp {
    result: *mut Tcl_Obj,
}

// ---- bump arena over a static buffer (leaks; spike only) ----
const ARENA_SIZE: usize = 8 * 1024 * 1024;
static mut HEAP: [u8; ARENA_SIZE] = [0; ARENA_SIZE];
static mut OFF: usize = 0;

fn bump(n: usize, align: usize) -> *mut u8 {
    unsafe {
        let base = HEAP.as_mut_ptr() as usize;
        let aligned = (base + OFF + align - 1) & !(align - 1);
        OFF = aligned - base + n;
        aligned as *mut u8
    }
}

unsafe fn strlen(p: *const c_char) -> usize {
    let mut n = 0;
    while *p.add(n) != 0 {
        n += 1;
    }
    n
}

unsafe fn slice_eq(a: *const u8, b: *const u8, n: usize) -> bool {
    for i in 0..n {
        if *a.add(i) != *b.add(i) {
            return false;
        }
    }
    true
}

fn new_obj(bytes: *const u8, len: usize) -> *mut Tcl_Obj {
    let buf = bump(len + 1, 1);
    unsafe {
        core::ptr::copy_nonoverlapping(bytes, buf, len);
        *buf.add(len) = 0;
    }
    let o = bump(core::mem::size_of::<Tcl_Obj>(), 8) as *mut Tcl_Obj;
    unsafe {
        (*o).refCount = 0;
        (*o).bytes = buf as *mut c_char;
        (*o).length = len as Tcl_Size;
        (*o).typePtr = core::ptr::null();
        (*o).internalRep = 0;
    }
    o
}

fn utf8_len(lead: u8) -> usize {
    if lead < 0x80 {
        1
    } else if lead >> 5 == 0b110 {
        2
    } else if lead >> 4 == 0b1110 {
        3
    } else if lead >> 3 == 0b11110 {
        4
    } else {
        1
    }
}

// ---- one global command table (spike: single interp) ----
#[derive(Clone, Copy)]
struct Command {
    name: *const u8,
    nlen: usize,
    proc_: Tcl_ObjCmdProc,
    cd: *mut c_void,
}
static mut CMDS: [Option<Command>; 128] = [None; 128];
static mut NCMD: usize = 0;

unsafe fn find_cmd(name: *const u8, nlen: usize) -> Option<Command> {
    for i in 0..NCMD {
        if let Some(c) = CMDS[i] {
            if c.nlen == nlen && slice_eq(c.name, name, nlen) {
                return Some(c);
            }
        }
    }
    None
}

// ===========================================================================
// The Tcl C API -- exported with the C ABI, imported by the side module.
// ===========================================================================

#[no_mangle]
pub extern "C" fn Tcl_InitStubs(_i: *mut Interp, _v: *const c_char, _e: c_int) -> *const c_char {
    b"9.0\0".as_ptr() as *const c_char
}

#[no_mangle]
pub extern "C" fn Tcl_PkgProvideEx(
    _i: *mut Interp,
    _name: *const c_char,
    _ver: *const c_char,
    _cd: *const c_void,
) -> c_int {
    0
}

#[no_mangle]
pub extern "C" fn Tcl_CreateObjCommand(
    _i: *mut Interp,
    name: *const c_char,
    proc_: Option<Tcl_ObjCmdProc>,
    cd: *mut c_void,
    _del: Option<Tcl_CmdDeleteProc>,
) -> *mut c_void {
    unsafe {
        let nlen = strlen(name);
        CMDS[NCMD] = Some(Command {
            name: name as *const u8,
            nlen,
            proc_: proc_.unwrap(),
            cd,
        });
        NCMD += 1;
        NCMD as *mut c_void
    }
}

#[no_mangle]
pub extern "C" fn Tcl_GetStringFromObj(o: *mut Tcl_Obj, len: *mut Tcl_Size) -> *mut c_char {
    unsafe {
        if !len.is_null() {
            *len = (*o).length;
        }
        (*o).bytes
    }
}

#[no_mangle]
pub extern "C" fn Tcl_GetString(o: *mut Tcl_Obj) -> *mut c_char {
    unsafe { (*o).bytes }
}

#[no_mangle]
pub extern "C" fn Tcl_NumUtfChars(src: *const c_char, length: Tcl_Size) -> Tcl_Size {
    let len = if length < 0 {
        unsafe { strlen(src) }
    } else {
        length as usize
    };
    let mut count = 0;
    let mut i = 0;
    unsafe {
        while i < len {
            i += utf8_len(*(src.add(i) as *const u8));
            count += 1;
        }
    }
    count as Tcl_Size
}

#[no_mangle]
pub extern "C" fn Tcl_UtfNcmp(s1: *const c_char, s2: *const c_char, n: usize) -> c_int {
    let (mut a, mut b) = (s1 as *const u8, s2 as *const u8);
    let mut count = 0usize;
    unsafe {
        while count < n {
            let (ca, cb) = (*a, *b);
            if ca != cb {
                return ca as c_int - cb as c_int;
            }
            if ca == 0 {
                return 0;
            }
            let adv = utf8_len(ca);
            for i in 1..adv {
                let (x, y) = (*a.add(i), *b.add(i));
                if x != y {
                    return x as c_int - y as c_int;
                }
            }
            a = a.add(adv);
            b = b.add(adv);
            count += 1;
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn Tcl_SetObjResult(i: *mut Interp, r: *mut Tcl_Obj) {
    unsafe {
        (*r).refCount += 1;
        (*i).result = r;
    }
}

#[no_mangle]
pub extern "C" fn Tcl_NewWideIntObj(v: Tcl_WideInt) -> *mut Tcl_Obj {
    let neg = v < 0;
    let mut u = if neg {
        (!(v as u64)).wrapping_add(1)
    } else {
        v as u64
    };
    let mut buf = [0u8; 24];
    let mut i = buf.len();
    if u == 0 {
        i -= 1;
        buf[i] = b'0';
    }
    while u > 0 {
        i -= 1;
        buf[i] = b'0' + (u % 10) as u8;
        u /= 10;
    }
    if neg {
        i -= 1;
        buf[i] = b'-';
    }
    new_obj(buf[i..].as_ptr(), buf.len() - i)
}

#[no_mangle]
pub extern "C" fn Tcl_NewStringObj(bytes: *const c_char, length: Tcl_Size) -> *mut Tcl_Obj {
    let len = if length < 0 {
        unsafe { strlen(bytes) }
    } else {
        length as usize
    };
    new_obj(bytes as *const u8, len)
}

#[no_mangle]
pub extern "C" fn Tcl_WrongNumArgs(
    i: *mut Interp,
    _objc: Tcl_Size,
    _objv: *const *mut Tcl_Obj,
    message: *const c_char,
) {
    // Spike: surface the usage message as the result (error path is untested).
    let len = unsafe { strlen(message) };
    unsafe { (*i).result = new_obj(message as *const u8, len) };
}

// ===========================================================================
// Host-facing helpers -- the loader (loader.py) calls these.
// ===========================================================================

/// Bump-allocate `n` bytes of shared memory; returns the linear-memory address.
#[no_mangle]
pub extern "C" fn spike_alloc(n: i32) -> i32 {
    bump(n as usize, 16) as i32
}

#[no_mangle]
pub extern "C" fn spike_new_interp() -> i32 {
    let p = bump(core::mem::size_of::<Interp>(), 8) as *mut Interp;
    unsafe { (*p).result = core::ptr::null_mut() };
    p as i32
}

/// Build a string `Tcl_Obj` from `len` bytes already written at `ptr`.
#[no_mangle]
pub extern "C" fn spike_make_string_obj(ptr: i32, len: i32) -> i32 {
    new_obj(ptr as usize as *const u8, len as usize) as i32
}

/// Dispatch a command through the runtime's command table. `argv_ptr` points to
/// an array of `argc` i32 `Tcl_Obj` addresses; `argv[0]`'s string is the command
/// name. This is the path compiled user code would take -- it `call_indirect`s
/// into the (separately loaded) side module's `Tcl_ObjCmdProc`.
#[no_mangle]
pub extern "C" fn spike_dispatch(interp: i32, argc: i32, argv_ptr: i32) -> i32 {
    let ip = interp as usize as *mut Interp;
    let argv = argv_ptr as usize as *const *mut Tcl_Obj;
    unsafe {
        let a0 = *argv;
        let name = (*a0).bytes as *const u8;
        let nlen = (*a0).length as usize;
        match find_cmd(name, nlen) {
            None => -1,
            Some(c) => {
                (*ip).result = core::ptr::null_mut();
                (c.proc_)(c.cd, ip, argc, argv)
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn spike_result_ptr(interp: i32) -> i32 {
    let ip = interp as usize as *const Interp;
    unsafe {
        let r = (*ip).result;
        if r.is_null() {
            0
        } else {
            (*r).bytes as i32
        }
    }
}

#[no_mangle]
pub extern "C" fn spike_result_len(interp: i32) -> i32 {
    let ip = interp as usize as *const Interp;
    unsafe {
        let r = (*ip).result;
        if r.is_null() {
            0
        } else {
            (*r).length as i32
        }
    }
}
