//! The typed front-end rejects out-of-subset constructs with the right codes.

use bpf_tcl_ir::{BpfDiag, compile_module};

#[test]
fn rejects_plain_set() {
    let e = compile_module("when SOCKET_FILTER { set x 1\n accept }\n").unwrap_err();
    assert_eq!(e.code, BpfDiag::OutOfSubset);
}

#[test]
fn rejects_unknown_event() {
    let e = compile_module("when TURBOENCABULATOR { accept }\n").unwrap_err();
    assert_eq!(e.code, BpfDiag::BadEvent);
}

#[test]
fn rejects_unbounded_while() {
    let e = compile_module(
        "when SOCKET_FILTER { seti32 n {0}\n while {1} { seti32 n {1} }\n accept }\n",
    )
    .unwrap_err();
    assert_eq!(e.code, BpfDiag::UnboundedLoop);
}

#[test]
fn rejects_pointer_arithmetic() {
    let e =
        compile_module("when SOCKET_FILTER { setbuf pkt ctx\n setint x {$pkt + 1}\n accept }\n")
            .unwrap_err();
    assert_eq!(e.code, BpfDiag::TypeMismatch);
}

#[test]
fn rejects_undefined_var() {
    let e = compile_module("when SOCKET_FILTER { setint x {$nope + 1}\n accept }\n").unwrap_err();
    assert_eq!(e.code, BpfDiag::UndefinedVar);
}

#[test]
fn rejects_unknown_command() {
    let e = compile_module("when SOCKET_FILTER { frobnicate 1 2\n accept }\n").unwrap_err();
    assert_eq!(e.code, BpfDiag::UnknownCommand);
}
