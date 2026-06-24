//! XDP program type: `when XDP { … pass/drop/tx }` runs under rbpf and returns
//! XDP verdicts. (rbpf runs an XDP-shaped program identically to a socket
//! filter — the `data`/`data_end` context plus an `r0` verdict.)

use bpf_tcl::run::run_socket_filter;
use bpf_tcl_codegen::ebpf::emit_program;
use bpf_tcl_ir::{BpfDiag, compile_module};

fn run(src: &str, packet: &mut [u8]) -> u64 {
    let module = compile_module(src).expect("compiles");
    let obj = emit_program(&module.programs[0].program).expect("emits");
    run_socket_filter(&obj, packet).expect("runs")
}

#[test]
fn xdp_pass_returns_2() {
    let mut pkt = vec![0u8; 8];
    assert_eq!(run("when XDP { setbuf pkt ctx\n pass }\n", &mut pkt), 2);
}

#[test]
fn xdp_drop_returns_1() {
    let mut pkt = vec![0u8; 8];
    assert_eq!(run("when XDP { drop }\n", &mut pkt), 1);
}

#[test]
fn xdp_tx_returns_3() {
    let mut pkt = vec![0u8; 8];
    assert_eq!(run("when XDP { setbuf pkt ctx\n tx }\n", &mut pkt), 3);
}

const XDP_PORT_FILTER: &str = "when XDP {\n\
    setbuf pkt ctx\n\
    pktlen len pkt\n\
    if {$len < 38} { pass }\n\
    load16 dport pkt 36\n\
    if {$dport == 22} { drop }\n\
    pass\n}\n";

#[test]
fn xdp_drops_port_22() {
    let mut pkt = vec![0u8; 40];
    pkt[36] = 0x16; // native-endian 22
    pkt[37] = 0x00;
    assert_eq!(run(XDP_PORT_FILTER, &mut pkt), 1); // XDP_DROP
}

#[test]
fn xdp_passes_port_80() {
    let mut pkt = vec![0u8; 40];
    pkt[36] = 0x50; // 80
    pkt[37] = 0x00;
    assert_eq!(run(XDP_PORT_FILTER, &mut pkt), 2); // XDP_PASS
}

#[test]
fn accept_rejected_in_xdp() {
    let err = compile_module("when XDP { accept }\n").unwrap_err();
    assert_eq!(err.code, BpfDiag::OutOfSubset);
}

#[test]
fn pass_rejected_in_socket_filter() {
    let err = compile_module("when SOCKET_FILTER { pass }\n").unwrap_err();
    assert_eq!(err.code, BpfDiag::OutOfSubset);
}
