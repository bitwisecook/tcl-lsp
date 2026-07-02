//! Native port of `tests/lsp_e2e/test_server_version.py`.
//!
//! The smallest real round-trip: boot the live server and read the version it
//! reports in the `initialize` response's `serverInfo`. Guards against the
//! banner regressing to a `dev` fallback, and pins the reported version to the
//! workspace crate version (`CARGO_PKG_VERSION`) the compiled banner comes from.

mod common;

use common::Lsp;

#[test]
fn initialize_reports_version() {
    let lsp = Lsp::tcl();
    let info = lsp
        .server_info()
        .expect("initialize result had no serverInfo — cannot read the version banner");
    let reported = info
        .get("version")
        .and_then(|v| v.as_str())
        .expect("server reported no version banner");
    assert!(
        !reported.is_empty() && reported != "vdev" && reported != "dev",
        "server fell back to a dev version banner: {reported:?}"
    );
    // The native binary reports the workspace crate version verbatim.
    let expected = env!("CARGO_PKG_VERSION");
    assert_eq!(
        reported, expected,
        "native server reported version {reported:?}, expected the workspace crate \
         version {expected:?} (CARGO_PKG_VERSION)"
    );
}

#[test]
fn server_info_name() {
    let lsp = Lsp::tcl();
    let info = lsp.server_info().expect("serverInfo present");
    assert_eq!(
        info.get("name").and_then(|v| v.as_str()),
        Some("tcl-lsp"),
        "server identifies as tcl-lsp"
    );
}
