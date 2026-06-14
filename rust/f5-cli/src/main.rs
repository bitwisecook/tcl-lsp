//! Native `f5-query` CLI binary entry point.
//!
//! Thin-main-over-lib (same pattern as `tcl-lsp-server` / `tcl-cli`): all
//! parsing and dispatch lives in [`f5_cli`].

#![forbid(unsafe_code)]

fn main() -> std::process::ExitCode {
    f5_cli::run(std::env::args_os())
}
