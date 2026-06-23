//! SSH/scp transport stub for `f5 fetch`.
//!
//! SSH transport is not supported: an in-process client would pull in `russh`
//! (which needs `unsafe` / C dependencies the workspace forbids), and shelling
//! out to the system `ssh` / `scp` binaries is out of scope. Any
//! `--transport ssh` request, or an `auto` fallback that reaches SSH, returns
//! this clean deferral error instead.

/// The deferral message surfaced for any SSH-transport request.
pub const SSH_DEFERRAL: &str = "SSH transport is not yet ported to the Rust f5 CLI \
     (use --transport rest, or run the Python f5 CLI for SSH/scp)";
