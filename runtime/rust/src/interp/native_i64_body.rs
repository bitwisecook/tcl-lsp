// SPDX-License-Identifier: AGPL-3.0-or-later
//! Inert future body candidates and entered original-body execution.

use super::*;

pub(crate) struct FutureNativeI64AddPlan {
    parameters: Box<[u8]>,
    body: Box<[u8]>,
}

impl FutureNativeI64AddPlan {
    pub(super) fn parameters(&self) -> &[u8] {
        &self.parameters
    }
    pub(super) fn body(&self) -> &[u8] {
        &self.body
    }
    pub(crate) fn new(parameters: &[u8], body: &[u8]) -> Self {
        observe("candidate-request", None);
        Self {
            parameters: parameters.into(),
            body: body.into(),
        }
    }
}

pub(crate) struct NativeProcedureBodyCandidate {
    pub(crate) native: Option<NativeProcEntry>,
    pub(crate) i64_add: Option<FutureNativeI64AddPlan>,
}

/// Bounded WORK diagnostic over already reached stages and scalar outcomes.
/// No Native getter, owner, callback, lookup or admission is introduced.
pub(crate) fn observe(stage: &'static str, value: Option<u64>) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static ROWS: AtomicU64 = AtomicU64::new(0);
    if std::env::var("TCL_LSP_NATIVE_ADMISSION_DIAGNOSTIC").as_deref() != Ok("first-decline-v1") {
        return;
    }
    let mut current = ROWS.load(Ordering::Relaxed);
    loop {
        if current >= 128 {
            return;
        }
        let next = current + 1;
        match ROWS.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(actual) => current = actual,
        }
    }
    eprintln!("CCG_NATIVE_I64_BODY stage={stage} value={value:?}");
}
