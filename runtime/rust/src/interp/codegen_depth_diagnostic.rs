// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bounded WORK-only scalar depth observations; no activation authority.

use std::sync::atomic::{AtomicU32, Ordering};

static EVENTS: AtomicU32 = AtomicU32::new(0);

impl super::Interp {
    #[track_caller]
    pub(crate) fn observe_codegen_depth(&self, phase: &'static str, entry: usize) {
        self.observe_codegen_depth_labeled(|| phase, entry);
    }

    #[track_caller]
    pub(crate) fn observe_codegen_depth_labeled(
        &self,
        phase: impl FnOnce() -> &'static str,
        entry: usize,
    ) {
        let eval = self.eval_depth.get();
        if (eval > 6 && eval < 124)
            || std::env::var("TCL_LSP_NATIVE_ADMISSION_DIAGNOSTIC").as_deref()
                != Ok("first-decline-v1")
        {
            return;
        }
        let mut current = EVENTS.load(Ordering::Relaxed);
        let sequence = loop {
            if current >= 512 {
                return;
            }
            let next = current + 1;
            match EVENTS.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed)
            {
                Ok(_) => break current,
                Err(actual) => current = actual,
            }
        };
        let dispatch = self.native_dispatch_depth.get();
        let (frame, owners) = self
            .frames
            .try_borrow()
            .ok()
            .map_or((None, None), |frames| {
                (
                    Some(frames.current_activation()),
                    frames.codegen_depth_owner_ids(),
                )
            });
        // The frame borrow is gone; these are copied scalars only. An unavailable
        // frame is unobserved, not absence, and supplies no owner or admission.
        let caller = std::panic::Location::caller();
        let phase = phase();
        eprintln!(
            "CCG_WASM_DEPTH seq={sequence} phase={phase} eval={eval} dispatch={dispatch} frame={frame:?} entry={entry} caller={caller} owners={owners:?}",
        );
    }
}
