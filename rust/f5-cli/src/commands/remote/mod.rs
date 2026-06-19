//! Remote-access helpers for the `f5` CLI — Rust port of `tooling/f5/f5_remote`.
//!
//! Hosts the credential resolver (`auth`), the iControl REST transport (`rest`),
//! the single-object pull/push request shaping (`object_io`), and the SSH
//! deferral (`ssh`) shared by the `fetch` / `push` / `pull` verbs.

pub mod auth;
pub mod json_compat;
pub mod object_io;
pub mod rest;
pub mod ssh;

/// Render a file-I/O error the way Python's `OSError.__str__` does:
/// `[Errno N] <strerror>: '<path>'`. Used by `f5 push` so its
/// missing-file / permission errors are byte-parity with the Python verb.
#[must_use]
pub fn os_error_string(err: &std::io::Error, path: &str) -> String {
    if let Some(errno) = err.raw_os_error() {
        let strerror = strerror(errno);
        format!("[Errno {errno}] {strerror}: '{path}'")
    } else {
        format!("{err}: '{path}'")
    }
}

/// The libc `strerror` text for the handful of errnos a file read surfaces.
/// Matches the GNU/Linux messages Python echoes (the CI / dev target).
fn strerror(errno: i32) -> &'static str {
    match errno {
        2 => "No such file or directory",
        13 => "Permission denied",
        21 => "Is a directory",
        20 => "Not a directory",
        _ => "I/O error",
    }
}
