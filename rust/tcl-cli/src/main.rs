//! Native `tcl` CLI binary entry point.
//!
//! Mirrors the thin-main-over-lib pattern used by `tcl-lsp-server`: all
//! argument parsing and dispatch logic lives in [`tcl_cli`]; this binary just
//! forwards the process exit code.

#![forbid(unsafe_code)]

fn main() -> std::process::ExitCode {
    tcl_cli::run(std::env::args_os())
}
