// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Host diagnostic isolation for an owned execution thread. This policy grants
//! no Native operation, source entry, observer absence or command capability.

use std::cell::Cell;

thread_local! {
    // No destructor or guard restores output before other thread-local owners
    // retire. The private worker is joined rather than returned to a pool.
    static SILENCED: Cell<bool> = const { Cell::new(false) };
}

/// Permanently silence host diagnostics in the current owned worker thread.
/// Guest semantic I/O still passes through its actual Host and stdout writer.
pub fn silence_private_worker() {
    SILENCED.with(|silenced| silenced.set(true));
}

/// Whether host diagnostic format arguments and output may be evaluated.
/// A missing thread-local state fails closed; this const Cell has no destructor.
#[must_use]
pub fn host_diagnostics_enabled() -> bool {
    SILENCED
        .try_with(|silenced| !silenced.get())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_is_thread_owned_and_has_no_restore_boundary() {
        assert!(host_diagnostics_enabled());
        std::thread::spawn(|| {
            assert!(host_diagnostics_enabled());
            silence_private_worker();
            assert!(!host_diagnostics_enabled());
            silence_private_worker();
            assert!(!host_diagnostics_enabled());
        })
        .join()
        .unwrap();
        assert!(host_diagnostics_enabled());
    }
}
