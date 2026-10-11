// SPDX-License-Identifier: AGPL-3.0-or-later
//! Explicit numeric host facts for independently authored cache-hit controls.

use std::cell::Cell;
use tcl_platform::{
    DoubleNumericConversion, NativeCIntegerAbi, NumericEnvironment, NumericEnvironmentUnavailable,
    NumericErrorState, UnsignedNumericConversion,
};

/// Native calling-thread conversion owner used by fresh-conversion controls.
pub use tcl_host_c_abi::NativeNumericEnvironment;

/// Complete caller-supplied state receipt; never a default errno baseline.
/// This provider refuses resets and conversions, so a successful cached test
/// also proves that it did not silently execute a fresh numeric primitive.
pub struct RecordedNumericEnvironment {
    state: Result<NumericErrorState, NumericEnvironmentUnavailable>,
    abi: Option<NativeCIntegerAbi>,
    queries: Cell<usize>,
}
impl RecordedNumericEnvironment {
    /// Supply independently recorded state or explicit host unavailability.
    /// The C integer layout remains unavailable without a separate receipt.
    #[must_use]
    pub const fn new(state: Result<NumericErrorState, NumericEnvironmentUnavailable>) -> Self {
        Self {
            state,
            abi: None,
            queries: Cell::new(0),
        }
    }
    /// Retain an independently queried actual C integer-layout receipt.
    /// This supplies no interpreter/object or primitive recipe authority, and
    /// still refuses every fresh conversion and reset.
    #[must_use]
    pub const fn with_c_integer_abi(
        state: Result<NumericErrorState, NumericEnvironmentUnavailable>,
        abi: NativeCIntegerAbi,
    ) -> Self {
        Self {
            state,
            abi: Some(abi),
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
    fn c_integer_abi(&self) -> Result<NativeCIntegerAbi, NumericEnvironmentUnavailable> {
        self.abi.ok_or(NumericEnvironmentUnavailable::Target)
    }
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
