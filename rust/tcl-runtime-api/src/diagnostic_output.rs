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

// WORK-only detached metadata; it grants no source or Native permission.
#[cfg(debug_assertions)]
mod closed_observation {
    use std::cell::RefCell;
    use std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    };

    thread_local! {
        // This passive owner contains only integers. It never restores SILENCED.
        static MARKER: RefCell<Option<Arc<AtomicU64>>> = const { RefCell::new(None) };
    }

    pub fn attach(marker: Option<Arc<AtomicU64>>) {
        MARKER.with(|slot| *slot.borrow_mut() = marker);
    }

    pub fn note(stage: u8, detail: u16, ordinal: u16) {
        let value = u64::from(stage) | (u64::from(detail) << 8) | (u64::from(ordinal) << 24);
        let _ = MARKER.try_with(|slot| {
            if let Ok(slot) = slot.try_borrow()
                && let Some(marker) = slot.as_ref()
            {
                let _ = marker.compare_exchange(0, value, Ordering::Relaxed, Ordering::Relaxed);
            }
        });
    }
}

/// Attach passive failure metadata to an owned debug worker, without output.
#[cfg(debug_assertions)]
pub fn attach_closed_native_observation(
    marker: Option<std::sync::Arc<std::sync::atomic::AtomicU64>>,
) {
    closed_observation::attach(marker);
}

/// Preserve the first already-refused private stage using integers only.
#[cfg(debug_assertions)]
pub fn note_closed_native_refusal(stage: u8, detail: u16, ordinal: u16) {
    closed_observation::note(stage, detail, ordinal);
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
