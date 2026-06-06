//! Small host/misc commands needed to bootstrap the real library (M2).
//!
//! `encoding` is near-trivial because UTF-8 is the internal string rep (the
//! cross-cutting contract): `convertto`/`convertfrom` pass through, `system` is
//! `utf-8`, and `dirs` is a no-op store (we don't load encoding files). C ref
//! `tclEncoding.c`. Non-UTF-8 codecs are a deferred edge translation.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::interp::{obj_bytes, Code, Interp};
use crate::obj::TclObj;

/// Register the misc bootstrap commands.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"encoding", encoding_cmd);
    // The `clock` C subsystem is L3; init.tcl's startup calls this configure
    // hook unconditionally — accept it as a no-op until `clock` lands.
    interp.register_builtin(b"::tcl::unsupported::clock::configure", noop);
}

/// A no-op command (returns the empty string) — a placeholder for a C subsystem
/// hook the bootstrap invokes but doesn't depend on the result of.
fn noop(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    interp.set_result_bytes(b"");
    Code::Ok
}

fn wrong_args(interp: &mut Interp, usage: &[u8]) -> Code {
    let mut m = b"wrong # args: should be \"".to_vec();
    m.extend_from_slice(usage);
    m.push(b'"');
    interp.set_error(&m)
}

fn encoding_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return wrong_args(interp, b"encoding subcommand ?arg ...?");
    }
    match obj_bytes(argv[1]).as_slice() {
        // `encoding dirs ?list?` — we don't search encoding files; accept + ignore.
        b"dirs" => {
            interp.set_result_bytes(b"");
            Code::Ok
        }
        b"system" => {
            interp.set_result_bytes(b"utf-8");
            Code::Ok
        }
        b"names" => {
            interp.set_result_bytes(b"utf-8 unicode ascii iso8859-1");
            Code::Ok
        }
        // `convertto`/`convertfrom ?encoding? data` — pass through (UTF-8 internal).
        b"convertto" | b"convertfrom" => {
            let Some(&data) = argv.last() else {
                return wrong_args(interp, b"encoding convertto ?encoding? data");
            };
            interp.set_result(data);
            Code::Ok
        }
        other => {
            let mut m = b"unknown or ambiguous subcommand \"".to_vec();
            m.extend_from_slice(other);
            m.extend_from_slice(b"\": must be convertfrom, convertto, dirs, names, or system");
            interp.set_error(&m)
        }
    }
}
