//! Userspace eBPF run harness, driving emitted programs through `rbpf`'s
//! `EbpfVmFixedMbuff` over a synthetic packet (the real eBPF `data`/`data_end`
//! model: the metadata buffer carries the packet start/end pointers at offsets
//! 0 and 8, matching the codegen prologue).

use bpf_tcl_codegen::ebpf::EbpfObject;
use rbpf::EbpfVmFixedMbuff;

/// Run an emitted socket-filter program over `packet`, returning the verdict
/// (`r0`: bytes to accept; `0` = drop).
///
/// # Errors
/// Returns rbpf's load/verify/run error rendered as a string.
pub fn run_socket_filter(obj: &EbpfObject, packet: &mut [u8]) -> Result<u64, String> {
    // Copy the program so the VM's lifetime stays local and unifies with the
    // `packet` borrow rbpf's API requires.
    let prog = obj.raw.clone();
    let mut vm = EbpfVmFixedMbuff::new(Some(&prog), 0, 8).map_err(|e| e.to_string())?;
    vm.execute_program(packet).map_err(|e| e.to_string())
}
