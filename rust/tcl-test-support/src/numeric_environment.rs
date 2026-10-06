// SPDX-License-Identifier: AGPL-3.0-or-later
//! Explicit numeric host facts for independently authored cache-hit controls.

use std::cell::Cell;
use tcl_platform::{
    DoubleNumericConversion, NumericEnvironment, NumericEnvironmentUnavailable, NumericErrorState,
    UnsignedNumericConversion,
};

/// Native calling-thread conversion owner used by fresh-conversion controls.
pub use tcl_host_c_abi::NativeNumericEnvironment;

/// Complete caller-supplied state receipt; never a default errno baseline.
/// This provider refuses resets and conversions, so a successful cached test
/// also proves that it did not silently execute a fresh numeric primitive.
pub struct RecordedNumericEnvironment {
    state: Result<NumericErrorState, NumericEnvironmentUnavailable>,
    queries: Cell<usize>,
}
impl RecordedNumericEnvironment {
    /// Supply independently recorded facts or explicit host unavailability.
    #[must_use]
    pub const fn new(state: Result<NumericErrorState, NumericEnvironmentUnavailable>) -> Self {
        Self {
            state,
            queries: Cell::new(0),
        }
    }
    /// Number of reached state observations; this grants no native authority.
    #[must_use]
    pub fn queries(&self) -> usize {
        self.queries.get()
    }
}
impl NumericEnvironment for RecordedNumericEnvironment {
    fn state(&self) -> Result<NumericErrorState, NumericEnvironmentUnavailable> {
        self.queries.set(self.queries.get() + 1);
        self.state
    }
    fn unsigned(
        &self,
        _: &[u8],
        _: usize,
        _: u32,
    ) -> Result<UnsignedNumericConversion, NumericEnvironmentUnavailable> {
        Err(NumericEnvironmentUnavailable::Target)
    }
    fn double(
        &self,
        _: &[u8],
        _: bool,
    ) -> Result<DoubleNumericConversion, NumericEnvironmentUnavailable> {
        Err(NumericEnvironmentUnavailable::Target)
    }
}
