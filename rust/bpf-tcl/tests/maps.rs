//! Maps end-to-end: a program that counts packets in a map, run under rbpf,
//! accumulates across invocations.

use bpf_tcl::run::run_socket_filter_repeated;
use bpf_tcl_codegen::ebpf::emit_program;
use bpf_tcl_ir::{BpfDiag, compile_module};

const COUNTER: &str = "when SOCKET_FILTER {\n\
    map hits hash 8 8 16\n\
    setint key 0\n\
    map_get n hits {$key}\n\
    setint n1 {$n + 1}\n\
    map_set hits {$key} {$n1}\n\
    accept\n}\n";

#[test]
fn map_counter_accumulates() {
    let module = compile_module(COUNTER).expect("counter should compile");
    let program = &module.programs[0].program;
    assert_eq!(program.maps.len(), 1, "one map declared");
    assert_eq!(program.maps[0].name, "hits");

    let obj = emit_program(program).expect("counter should emit");
    assert_eq!(obj.maps.len(), 1);

    let mut pkt = vec![0u8; 32];
    let (verdict, maps) =
        run_socket_filter_repeated(&obj, &mut pkt, 3).expect("counter should run");

    // Each run accepts the whole packet and increments hits[0].
    assert_eq!(verdict, 32);
    assert_eq!(
        maps[0].get(&0).copied(),
        Some(3),
        "hits[0] == 3 after 3 runs"
    );
}

#[test]
fn map_set_then_get_roundtrips() {
    // Store a value under a key, then read it back into the verdict.
    let src = "when SOCKET_FILTER {\n\
        map m hash 8 8 8\n\
        map_set m {7} {99}\n\
        map_get v m {7}\n\
        accept {$v}\n}\n";
    let module = compile_module(src).expect("compile");
    let obj = emit_program(&module.programs[0].program).expect("emit");
    let mut pkt = vec![0u8; 8];
    let (verdict, _) = run_socket_filter_repeated(&obj, &mut pkt, 1).expect("run");
    assert_eq!(verdict, 99);
}

#[test]
fn unknown_map_is_rejected() {
    let err = compile_module("when SOCKET_FILTER { map_get v nope {0}\n accept }\n").unwrap_err();
    assert_eq!(err.code, BpfDiag::UndefinedVar);
}
