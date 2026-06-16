//! SSH/scp transport stub for `f5 fetch`.
//!
//! The Python verb (`tooling/f5/f5_remote/ssh.py`) shells out to the system
//! `ssh` / `scp` binaries. The Rust port **defers** SSH: a faithful in-process
//! client would pull in `russh` (which needs `unsafe` / C dependencies the
//! workspace forbids), and shelling out is out of scope for this phase. Any
//! `--transport ssh` request, or an `auto` fallback that reaches SSH, returns
//! this clean deferral error instead.

/// The deferral message surfaced for any SSH-transport request.
pub const SSH_DEFERRAL: &str = "SSH transport is not yet ported to the Rust f5 CLI \
     (use --transport rest, or run the Python f5 CLI for SSH/scp)";
