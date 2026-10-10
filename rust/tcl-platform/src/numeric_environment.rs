// SPDX-License-Identifier: AGPL-3.0-or-later
//! Independently supplied host C numeric conversion and thread range state.

/// Actual C thread error state at a reached observation point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NumericErrorState {
    /// Platform errno value, retained rather than normalized to a baseline.
    pub errno: i32,
    /// Whether that exact state is the selected host's EDOM.
    pub domain_error: bool,
    /// Whether that exact state is the selected host's ERANGE.
    pub range_error: bool,
}

/// A reached unsigned C conversion, including its real thread-state effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnsignedNumericConversion {
    /// Converted unsigned long long value.
    pub value: u64,
    /// End-pointer byte offset in the original counted input.
    pub end: usize,
    /// State before executing the native conversion.
    pub before: NumericErrorState,
    /// State after the conversion; successful libc calls need not clear errno.
    pub after: NumericErrorState,
}

/// A reached double C conversion with an explicit errno-reset purpose.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DoubleNumericConversion {
    /// Exact double returned by the C operation.
    pub value: f64,
    /// End-pointer byte offset in the original counted input.
    pub end: usize,
    /// Actual state before any requested reset.
    pub before: NumericErrorState,
    /// Actual state after parsing, including underflow and overflow.
    pub after: NumericErrorState,
}

/// Unavailable target ABI or invalid conversion geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericEnvironmentUnavailable {
    /// This host cannot independently supply the selected native thread state.
    Target,
    /// The requested offset/base cannot describe a valid C conversion call.
    Conversion,
}

/// Descriptive integer layout of the actual selected host C target.
/// These facts authenticate no interpreter, object header, handler or source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeCIntegerAbi {
    /// Bits per actual C `char`, independently of source Unicode grammar.
    pub char_bits: u8,
    /// Actual C `sizeof(int)` in C characters.
    pub int_bytes: u8,
    /// Actual C `sizeof(long)` in C characters.
    pub long_bytes: u8,
}

/// Host-owned numeric thread state shared by interpreters and host callbacks.
/// There is no interpreter-local errno mirror. Authored hosts can provide a
/// separate implementation; absence does not establish a zero/range-free state.
pub trait NumericEnvironment {
    /// Report this independently supplied host's actual C integer layout.
    /// Default absence supplies neither a width nor a primitive getter recipe.
    fn c_integer_abi(&self) -> Result<NativeCIntegerAbi, NumericEnvironmentUnavailable> {
        Err(NumericEnvironmentUnavailable::Target)
    }

    /// Observe the actual current thread state without resetting it.
    fn state(&self) -> Result<NumericErrorState, NumericEnvironmentUnavailable>;

    /// Execute the explicit reset selected by a reached native conversion.
    /// Cache hits and error-state observations must never call this operation.
    fn reset(&self) -> Result<(), NumericEnvironmentUnavailable> {
        Err(NumericEnvironmentUnavailable::Target)
    }

    /// C8.4 unsigned conversion after its independently selected reset/sign
    /// handling. `long` requires strtoul. A wide request uses strtoul when
    /// the host C long and wide types agree, otherwise strtoull.
    fn unsigned_c84(
        &self,
        input: &[u8],
        offset: usize,
        long: bool,
    ) -> Result<UnsignedNumericConversion, NumericEnvironmentUnavailable> {
        let _ = (input, offset, long);
        Err(NumericEnvironmentUnavailable::Target)
    }

    /// Execute strtoull on the selected original input suffix. `offset` is
    /// measured before the first NUL; base is 0 or 2..=36. Does not reset errno.
    fn unsigned(
        &self,
        input: &[u8],
        offset: usize,
        base: u32,
    ) -> Result<UnsignedNumericConversion, NumericEnvironmentUnavailable>;

    /// Execute strtod on original input. Reset is an explicit caller-selected
    /// native operation, not an inferred property of the current errno.
    fn double(
        &self,
        input: &[u8],
        reset_errno: bool,
    ) -> Result<DoubleNumericConversion, NumericEnvironmentUnavailable>;
}
