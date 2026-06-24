//! The attach/deployment facet of the profile-based top layer: a top-level
//! `attach KIND TARGET` declaration records a program's deployment target. It
//! resolves to a program type and must match at least one `when` block; the
//! result is metadata on the program declaration (and the emitted object
//! self-describes its program type).

use bpf_tcl_codegen::ebpf::emit_program;
use bpf_tcl_ir::ir::ProgType;
use bpf_tcl_ir::{BpfDiag, compile_module};

#[test]
fn attach_is_carried_onto_the_matching_program() {
    let src = "attach xdp eth0\n\
        when XDP { setbuf p ctx\n pass }\n";
    let module = compile_module(src).expect("compiles");
    let decl = &module.programs[0];
    let attach = decl.attach.as_ref().expect("attach carried");
    assert_eq!(attach.kind, "xdp");
    assert_eq!(attach.target, "eth0");
    // The emitted object self-describes its program type.
    let obj = emit_program(&decl.program).expect("emits");
    assert_eq!(obj.prog_type, ProgType::Xdp);
}

#[test]
fn attach_socket_filter_matches_socket_program() {
    let src = "attach socket_filter lo\n\
        when SOCKET_FILTER { accept }\n";
    let module = compile_module(src).expect("compiles");
    let attach = module.programs[0].attach.as_ref().expect("attach carried");
    assert_eq!(attach.kind, "socket_filter");
    assert_eq!(attach.target, "lo");
}

#[test]
fn no_attach_leaves_metadata_empty() {
    let module = compile_module("when XDP { pass }\n").expect("compiles");
    assert!(module.programs[0].attach.is_none());
}

#[test]
fn attach_mismatched_type_rejected() {
    // `attach xdp` but the only program is a socket filter — matches nothing.
    let src = "attach xdp eth0\n\
        when SOCKET_FILTER { accept }\n";
    let err = compile_module(src).unwrap_err();
    assert_eq!(err.code, BpfDiag::BadAttach);
}

#[test]
fn unknown_attach_kind_rejected() {
    let err = compile_module("attach kprobe foo\nwhen XDP { pass }\n").unwrap_err();
    assert_eq!(err.code, BpfDiag::BadAttach);
}

#[test]
fn two_attach_decls_rejected() {
    let src = "attach xdp eth0\n\
        attach xdp eth1\n\
        when XDP { pass }\n";
    let err = compile_module(src).unwrap_err();
    assert_eq!(err.code, BpfDiag::BadAttach);
}

#[test]
fn attach_bad_arity_rejected() {
    let err = compile_module("attach xdp\nwhen XDP { pass }\n").unwrap_err();
    assert_eq!(err.code, BpfDiag::BadAttach);
}

#[test]
fn attach_does_not_disturb_the_program_body() {
    // The `attach` decl is stripped from executable code; the filter still runs.
    let src = "attach xdp eth0\n\
        when XDP {\n\
            setbuf p ctx\n\
            pktlen len p\n\
            if {$len < 4} { drop }\n\
            pass\n\
        }\n";
    let module = compile_module(src).expect("compiles");
    let obj = emit_program(&module.programs[0].program).expect("emits");
    let mut pkt = vec![0u8; 8];
    let v = bpf_tcl::run::run_socket_filter(&obj, &mut pkt).expect("runs");
    assert_eq!(v, 2); // XDP_PASS — body unaffected by the attach metadata
}
