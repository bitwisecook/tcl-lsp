//! The template/macro facet of the profile-based top layer: a
//! `template NAME { params } { body }` is a reusable parameterised handler that
//! `use NAME k=v …` splices inline (binding each param via a synthetic
//! `setint`). End-to-end under rbpf: two `use` sites of a map-bumping template
//! accumulate into a verdict.

use bpf_tcl::run::run_socket_filter_repeated;
use bpf_tcl_codegen::ebpf::emit_program;
use bpf_tcl_ir::{BpfDiag, compile_module};

fn verdict(src: &str) -> u64 {
    let module = compile_module(src).expect("compiles");
    let obj = emit_program(&module.programs[0].program).expect("emits");
    let mut pkt = vec![0u8; 16];
    let (v, _) = run_socket_filter_repeated(&obj, &mut pkt, 1).expect("runs");
    v
}

/// `bump` reads a counter, adds `amount`, writes it back. Two `use` sites add
/// 2 then 3 to `ctr[0]`; the verdict reads back the total (5).
const BUMP_TWICE: &str = "template bump {key amount} {\n\
        map_get cur ctr {$key}\n\
        setint next {$cur + $amount}\n\
        map_set ctr {$key} {$next}\n\
    }\n\
    when SOCKET_FILTER {\n\
        map ctr hash 8 8 16\n\
        use bump key=0 amount=2\n\
        use bump key=0 amount=3\n\
        map_get total ctr {0}\n\
        accept {$total}\n\
    }\n";

#[test]
fn template_use_accumulates() {
    assert_eq!(verdict(BUMP_TWICE), 5);
}

/// A param drives control flow: `gate limit=10` lets a small packet through.
const GATE: &str = "template gate {limit} {\n\
        pktlen len pkt\n\
        if {$len > $limit} { drop }\n\
    }\n\
    when SOCKET_FILTER {\n\
        setbuf pkt ctx\n\
        use gate limit=100\n\
        accept\n\
    }\n";

#[test]
fn template_param_controls_flow() {
    // 16-byte packet, limit 100 → not dropped → accept whole packet (16).
    assert_eq!(verdict(GATE), 16);
}

#[test]
fn template_param_drops_when_over_limit() {
    let src = "template gate {limit} {\n\
            pktlen len pkt\n\
            if {$len > $limit} { drop }\n\
        }\n\
        when SOCKET_FILTER {\n\
            setbuf pkt ctx\n\
            use gate limit=4\n\
            accept\n\
        }\n";
    // 16-byte packet, limit 4 → dropped → verdict 0.
    assert_eq!(verdict(src), 0);
}

#[test]
fn unknown_template_rejected() {
    let err = compile_module("when SOCKET_FILTER { use nope x=1\n accept }\n").unwrap_err();
    assert_eq!(err.code, BpfDiag::BadTemplate);
}

#[test]
fn missing_binding_rejected() {
    let src = "template t {x} { setint y {$x} }\n\
        when SOCKET_FILTER { use t\n accept }\n";
    let err = compile_module(src).unwrap_err();
    assert_eq!(err.code, BpfDiag::BadTemplate);
}

#[test]
fn unknown_param_rejected() {
    let src = "template t {x} { setint y {$x} }\n\
        when SOCKET_FILTER { use t x=1 z=2\n accept }\n";
    let err = compile_module(src).unwrap_err();
    assert_eq!(err.code, BpfDiag::BadTemplate);
}

#[test]
fn non_integer_binding_rejected() {
    let src = "template t {x} { setint y {$x} }\n\
        when SOCKET_FILTER { use t x=hello\n accept }\n";
    let err = compile_module(src).unwrap_err();
    assert_eq!(err.code, BpfDiag::BadTemplate);
}

#[test]
fn duplicate_template_rejected() {
    let src = "template t {x} { setint y {$x} }\n\
        template t {z} { setint w {$z} }\n\
        when SOCKET_FILTER { accept }\n";
    let err = compile_module(src).unwrap_err();
    assert_eq!(err.code, BpfDiag::BadTemplate);
}
