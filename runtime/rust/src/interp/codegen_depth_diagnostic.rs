// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bounded WORK-only scalar depth observations; no activation authority.

use std::sync::atomic::{AtomicU32, Ordering};

static EVENTS: AtomicU32 = AtomicU32::new(0);

impl super::Interp {
    pub(crate) fn observe_codegen_depth(&self, phase: &'static str, entry: usize) {
        let eval = self.eval_depth.get();
        if (eval > 6 && eval < 124)
            || std::env::var("TCL_LSP_NATIVE_ADMISSION_DIAGNOSTIC").as_deref()
                != Ok("first-decline-v1")
        {
            return;
        }
        let Ok(sequence) = EVENTS.try_update(Ordering::Relaxed, Ordering::Relaxed, |old| {
            (old < 512).then_some(old + 1)
        }) else {
            return;
        };
        let dispatch = self.native_dispatch_depth.get();
        let frame = self
            .frames
            .try_borrow()
            .ok()
            .map(|frames| frames.current_activation());
        // The frame borrow is gone; these are copied scalars only. An unavailable
        // frame is unobserved, not absence, and supplies no owner or admission.
        eprintln!(
            "CCG_WASM_DEPTH seq={sequence} phase={phase} eval={eval} dispatch={dispatch} frame={frame:?} entry={entry}",
        );
    }
}
