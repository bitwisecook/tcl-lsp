//! BPF-Tcl CLI binary.
#![forbid(unsafe_code)]

use std::process::ExitCode;

fn main() -> ExitCode {
    bpf_tcl::run(std::env::args())
}
