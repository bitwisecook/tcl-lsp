//! Rust-runtime C-extension spike.
//!
//! Proves the load-bearing claim behind a Rust rewrite of the WASM runtime:
//! an **unmodified** real C Tcl extension can be compiled to WebAssembly
//! against an authored `tcl.h` and linked against a **Rust** runtime, with no
//! per-extension shim.
//!
//! What this file is: a deliberately minimal Rust implementation of the slice
//! of the Tcl C API that `ext/pkga.c` uses (`Tcl_CreateObjCommand`,
//! `Tcl_GetStringFromObj`, `Tcl_NewIntObj`, `Tcl_SetObjResult`,
//! `Tcl_WrongNumArgs`, `Tcl_PkgProvide`, `Tcl_InitStubs`, `Tcl_NumUtfChars`,
//! `Tcl_UtfNcmp`).  It is NOT the real runtime -- `Tcl_Obj`s are leaked, there
//! is no shimmering or refcount discipline -- it exists only to close the
//! call loop end to end.
//!
//! The flow exercised by `main`:
//!   1. Rust creates a `Tcl_Interp`.
//!   2. Rust calls the C extension's `Pkga_Init(interp)` (compiled by clang).
//!   3. `Pkga_Init` calls back into Rust (`Tcl_PkgProvide`,
//!      `Tcl_CreateObjCommand`) to register `pkga_eq` / `pkga_quote`.
//!   4. Rust dispatches those commands through its OWN command table -- exactly
//!      the path compiled user code would take -- which `call_indirect`s into
//!      the extension's C `Tcl_ObjCmdProc` through a shared function table.
//!   5. The command builds a result `Tcl_Obj` via the Rust obj API; Rust reads
//!      it back.
//!
//! Everything lives in one wasm module (static link): C extension objects +
//! Rust runtime objects + a driver standing in for compiled user code.

#![allow(non_snake_case, non_camel_case_types)]

use core::ffi::{c_char, c_int, c_void};
use std::ffi::CStr;

// ---------------------------------------------------------------------------
// Tcl C ABI types -- must match include/tcl.h field for field.
// ---------------------------------------------------------------------------

pub type Tcl_Size = isize; // ptrdiff_t on wasm32 == i32
pub type Tcl_WideInt = i64;

#[repr(C)]
pub struct Tcl_Obj {
    pub refCount: Tcl_Size,
    pub bytes: *mut c_char,
    pub length: Tcl_Size,
    pub typePtr: *const c_void,
    pub internalRep: u64, // 8-byte / align-8 stand-in for the C union
}

/// Matches `typedef int (Tcl_ObjCmdProc)(void*, Tcl_Interp*, int, Tcl_Obj* const*)`.
pub type Tcl_ObjCmdProc =
    extern "C" fn(*mut c_void, *mut Tcl_Interp, c_int, *const *mut Tcl_Obj) -> c_int;
pub type Tcl_CmdDeleteProc = extern "C" fn(*mut c_void);

/// Opaque to C; the real structure on the Rust side. C only ever holds a
/// `Tcl_Interp *`.
pub struct Tcl_Interp {
    result: *mut Tcl_Obj,
    commands: Vec<Command>,
    packages: Vec<(String, String)>,
}

struct Command {
    name: String,
    proc_: Tcl_ObjCmdProc,
    client_data: *mut c_void,
}

// ---------------------------------------------------------------------------
// Small helpers (runtime-internal, not part of the C ABI).
// ---------------------------------------------------------------------------

/// Allocate a `Tcl_Obj` whose string rep is `s` (NUL-terminated). Leaked: a
/// real runtime would refcount and free; the spike does not.
fn new_obj_from_bytes(s: &[u8]) -> *mut Tcl_Obj {
    let mut buf = Vec::with_capacity(s.len() + 1);
    buf.extend_from_slice(s);
    buf.push(0);
    let len = s.len() as Tcl_Size;
    let bytes = buf.as_mut_ptr() as *mut c_char;
    core::mem::forget(buf);
    Box::into_raw(Box::new(Tcl_Obj {
        refCount: 0,
        bytes,
        length: len,
        typePtr: core::ptr::null(),
        internalRep: 0,
    }))
}

unsafe fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        return String::new();
    }
    CStr::from_ptr(p).to_string_lossy().into_owned()
}

unsafe fn strlen(p: *const c_char) -> usize {
    let mut n = 0;
    while *p.add(n) != 0 {
        n += 1;
    }
    n
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

fn obj_to_string(o: *mut Tcl_Obj) -> String {
    if o.is_null() {
        return String::new();
    }
    unsafe {
        let ob = &*o;
        let len = ob.length.max(0) as usize;
        let sl = core::slice::from_raw_parts(ob.bytes as *const u8, len);
        String::from_utf8_lossy(sl).into_owned()
    }
}

// ---------------------------------------------------------------------------
// The Tcl C API, implemented in Rust and exported with the C ABI.
// These #[no_mangle] symbols are what the clang-compiled extension imports.
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn Tcl_InitStubs(
    _interp: *mut Tcl_Interp,
    _version: *const c_char,
    _exact: c_int,
) -> *const c_char {
    // Our ABI is direct linking: there is no stubs table to negotiate. Always
    // succeed, reporting our runtime's Tcl-API level.
    b"9.0\0".as_ptr() as *const c_char
}

#[no_mangle]
pub extern "C" fn Tcl_PkgProvideEx(
    interp: *mut Tcl_Interp,
    name: *const c_char,
    version: *const c_char,
    _client_data: *const c_void,
) -> c_int {
    let it = unsafe { &mut *interp };
    it.packages
        .push((unsafe { cstr(name) }, unsafe { cstr(version) }));
    0 // TCL_OK
}

#[no_mangle]
pub extern "C" fn Tcl_CreateObjCommand(
    interp: *mut Tcl_Interp,
    cmd_name: *const c_char,
    proc_: Option<Tcl_ObjCmdProc>,
    client_data: *mut c_void,
    _delete_proc: Option<Tcl_CmdDeleteProc>,
) -> *mut c_void {
    let it = unsafe { &mut *interp };
    it.commands.push(Command {
        name: unsafe { cstr(cmd_name) },
        proc_: proc_.expect("Tcl_CreateObjCommand: null proc"),
        client_data,
    });
    // Return a non-null command token (the real API returns a Tcl_Command).
    it.commands.len() as *mut c_void
}

#[no_mangle]
pub extern "C" fn Tcl_GetStringFromObj(
    obj: *mut Tcl_Obj,
    length_ptr: *mut Tcl_Size,
) -> *mut c_char {
    let ob = unsafe { &*obj };
    if !length_ptr.is_null() {
        unsafe { *length_ptr = ob.length };
    }
    ob.bytes
}

#[no_mangle]
pub extern "C" fn Tcl_GetString(obj: *mut Tcl_Obj) -> *mut c_char {
    unsafe { (*obj).bytes }
}

#[no_mangle]
pub extern "C" fn Tcl_NumUtfChars(src: *const c_char, length: Tcl_Size) -> Tcl_Size {
    let len = if length < 0 {
        unsafe { strlen(src) }
    } else {
        length as usize
    };
    let sl = unsafe { core::slice::from_raw_parts(src as *const u8, len) };
    // One char per non-continuation byte.
    sl.iter().filter(|&&b| (b & 0xC0) != 0x80).count() as Tcl_Size
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
pub extern "C" fn Tcl_SetObjResult(interp: *mut Tcl_Interp, result_obj: *mut Tcl_Obj) {
    unsafe {
        (*result_obj).refCount += 1;
        (*interp).result = result_obj;
    }
}

#[no_mangle]
pub extern "C" fn Tcl_NewWideIntObj(value: Tcl_WideInt) -> *mut Tcl_Obj {
    new_obj_from_bytes(value.to_string().as_bytes())
}

#[no_mangle]
pub extern "C" fn Tcl_NewStringObj(bytes: *const c_char, length: Tcl_Size) -> *mut Tcl_Obj {
    let len = if length < 0 {
        unsafe { strlen(bytes) }
    } else {
        length as usize
    };
    let sl = unsafe { core::slice::from_raw_parts(bytes as *const u8, len) };
    new_obj_from_bytes(sl)
}

#[no_mangle]
pub extern "C" fn Tcl_WrongNumArgs(
    interp: *mut Tcl_Interp,
    _objc: Tcl_Size,
    _objv: *const *mut Tcl_Obj,
    message: *const c_char,
) {
    let msg = format!("wrong # args: should be \"... {}\"", unsafe { cstr(message) });
    unsafe { (*interp).result = new_obj_from_bytes(msg.as_bytes()) };
}

// ---------------------------------------------------------------------------
// The C extension's entry point, provided by the clang-compiled pkga.o.
// ---------------------------------------------------------------------------

// `Tcl_Interp` is opaque to C -- only the pointer crosses the boundary -- so the
// improper_ctypes warning about its Rust-side layout does not apply here.
#[allow(improper_ctypes)]
extern "C" {
    fn Pkga_Init(interp: *mut Tcl_Interp) -> c_int;
}

/// Dispatch a command through the runtime's own command table -- the path
/// compiled user code would take. Returns the command's string result.
fn eval(interp: *mut Tcl_Interp, words: &[&str]) -> (c_int, String) {
    let objv: Vec<*mut Tcl_Obj> = words.iter().map(|w| new_obj_from_bytes(w.as_bytes())).collect();

    // Look the command up and copy out the call target, so we hold no borrow
    // of *interp across the FFI call (the proc mutates the interp via its raw
    // pointer when it sets the result).
    let (proc_, cd) = {
        let it = unsafe { &*interp };
        let cmd = it
            .commands
            .iter()
            .find(|c| c.name == words[0])
            .unwrap_or_else(|| panic!("command not registered: {}", words[0]));
        (cmd.proc_, cmd.client_data)
    };

    unsafe { (*interp).result = core::ptr::null_mut() };
    let rc = proc_(cd, interp, objv.len() as c_int, objv.as_ptr());
    let res = unsafe { (*interp).result };
    (rc, obj_to_string(res))
}

fn main() {
    let mut interp = Box::new(Tcl_Interp {
        result: core::ptr::null_mut(),
        commands: Vec::new(),
        packages: Vec::new(),
    });
    let ip: *mut Tcl_Interp = &mut *interp;

    println!("== Rust-runtime C-extension spike ==");
    println!("extension: ext/pkga.c (Tcl 9.0.3 unix/dltest, byte-identical, unmodified)");
    println!();

    // Step 1-3: run the unmodified C extension's init.
    let init_rc = unsafe { Pkga_Init(ip) };
    println!("Pkga_Init(interp) -> {init_rc} (TCL_OK = {})", init_rc == 0);
    println!("  packages provided : {:?}", interp.packages);
    println!(
        "  commands registered: {:?}",
        interp.commands.iter().map(|c| c.name.as_str()).collect::<Vec<_>>()
    );
    println!();

    // Step 4-5: dispatch the extension's commands through the runtime.
    let mut ok = init_rc == 0;
    let cases: &[(&[&str], &str)] = &[
        (&["pkga_eq", "hello", "hello"], "1"),
        (&["pkga_eq", "hello", "world"], "0"),
        (&["pkga_eq", "café", "café"], "1"), // multi-byte UTF-8 path
        (&["pkga_quote", "round-trip-value"], "round-trip-value"),
    ];
    for (words, want) in cases {
        let (rc, got) = eval(ip, words);
        let pass = rc == 0 && got == *want;
        ok &= pass;
        println!(
            "  {:<34} -> {:<18} [{}]",
            words.join(" "),
            format!("{got:?}"),
            if pass { "PASS" } else { "FAIL" }
        );
    }

    println!();
    println!("{}", if ok { "SPIKE PASS" } else { "SPIKE FAIL" });
    std::process::exit(if ok { 0 } else { 1 });
}
