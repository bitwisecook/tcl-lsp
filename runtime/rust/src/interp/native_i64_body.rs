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
