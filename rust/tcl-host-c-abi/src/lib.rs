// SPDX-License-Identifier: AGPL-3.0-or-later
//! Sole C ABI owner for real native thread numeric state. The portable callers
//! receive safe facts; no interpreter-local copy stands in for host errno.

use tcl_platform::{
    DoubleNumericConversion, NativeCIntegerAbi, NumericEnvironment, NumericEnvironmentUnavailable,
    NumericErrorState, SignedNumericConversion, UnsignedNumericConversion,
};

mod integer_formatter;
pub use integer_formatter::LoadedNativeIntegerFormatter;

/// Stateless access to this compiled target's current C numeric environment.
/// WASIp1 uses its own linked guest libc and errno cell, independently of any
/// SDK capture or host-native library. It supplies no Tcl getter recipe.
pub struct NativeNumericEnvironment;

impl NativeNumericEnvironment {
    /// Whether this build supplies an independently supported errno ABI.
    #[must_use]
    pub const fn supported() -> bool {
        cfg!(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "ios",
            target_os = "freebsd",
            all(
                target_arch = "wasm32",
                target_os = "wasi",
                target_env = "p1"
            )
        ))
    }
}

#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    all(target_arch = "wasm32", target_os = "wasi", target_env = "p1")
))]
mod native {
    use super::{
        DoubleNumericConversion, NumericEnvironmentUnavailable, NumericErrorState,
        SignedNumericConversion, UnsignedNumericConversion,
    };
    use std::ffi::CString;

    fn errno_pointer() -> *mut libc::c_int {
        // SAFETY: these libc accessors return the calling thread's errno cell.
        // No pointer is exported, retained, or used by a different thread.
        unsafe {
            #[cfg(any(
                target_os = "linux",
                all(target_arch = "wasm32", target_os = "wasi", target_env = "p1")
            ))]
            {
                libc::__errno_location()
            }
            #[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
            {
                libc::__error()
            }
        }
    }

    pub(super) fn state() -> NumericErrorState {
        // SAFETY: libc guarantees a live errno cell for the calling thread.
        let errno = unsafe { *errno_pointer() };
        NumericErrorState {
            errno,
            domain_error: errno == libc::EDOM,
            range_error: errno == libc::ERANGE,
        }
    }

    fn input(bytes: &[u8]) -> CString {
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        CString::new(&bytes[..end]).expect("prefix excludes all NUL bytes")
    }

    pub(super) fn reset() {
        // SAFETY: this is the calling thread's live errno cell. The caller
        // selects the reset only at an authentic native conversion frontier.
        unsafe {
            *errno_pointer() = 0;
        }
    }

    pub(super) fn unsigned_c84(
        bytes: &[u8],
        offset: usize,
        long: bool,
    ) -> Result<UnsignedNumericConversion, NumericEnvironmentUnavailable> {
        let original = input(bytes);
        if offset > original.as_bytes().len() {
            return Err(NumericEnvironmentUnavailable::Conversion);
        }
        let before = state();
        let mut end = std::ptr::null_mut();
        // SAFETY: admitted offset lies in the owned NUL-terminated allocation;
        // both libc calls receive a writable local end pointer and radix zero.
        let (value, consumed) = unsafe {
            // C8.4 aliases its wide parser to the long parser when both
            // native C types have the same width (TCL_WIDE_INT_IS_LONG).
            let value = if long
                || core::mem::size_of::<libc::c_long>() == core::mem::size_of::<libc::c_longlong>()
            {
                libc::strtoul(original.as_ptr().add(offset), &raw mut end, 0) as u64
            } else {
                libc::strtoull(original.as_ptr().add(offset), &raw mut end, 0) as u64
            };
            (value, end.offset_from(original.as_ptr()))
        };
        let end =
            usize::try_from(consumed).map_err(|_| NumericEnvironmentUnavailable::Conversion)?;
        if end > original.as_bytes().len() {
            return Err(NumericEnvironmentUnavailable::Conversion);
        }
        Ok(UnsignedNumericConversion {
            value,
            end,
            before,
            after: state(),
        })
    }

    pub(super) fn unsigned(
        bytes: &[u8],
        offset: usize,
        base: u32,
    ) -> Result<UnsignedNumericConversion, NumericEnvironmentUnavailable> {
        let original = input(bytes);
        if offset > original.as_bytes().len()
            || !(base == 0 || (2..=36).contains(&base))
            || std::mem::size_of::<libc::c_ulonglong>() != 8
        {
            return Err(NumericEnvironmentUnavailable::Conversion);
        }
        let before = state();
        let mut end = std::ptr::null_mut();
        // SAFETY: offset lies in the owned NUL-terminated allocation, base is
        // admitted by strtoull, and end points to a writable local pointer.
        let (value, consumed) = unsafe {
            let value = libc::strtoull(
                original.as_ptr().add(offset),
                &raw mut end,
                libc::c_int::try_from(base).expect("admitted strtoull radix fits c_int"),
            );
            (value, end.offset_from(original.as_ptr()))
        };
        let after = state();
        let end =
            usize::try_from(consumed).map_err(|_| NumericEnvironmentUnavailable::Conversion)?;
        if end > original.as_bytes().len() {
            return Err(NumericEnvironmentUnavailable::Conversion);
        }
        Ok(UnsignedNumericConversion {
            value,
            end,
            before,
            after,
        })
    }

    pub(super) fn signed_long(
        bytes: &[u8],
        base: u32,
    ) -> Result<SignedNumericConversion, NumericEnvironmentUnavailable> {
        let original = input(bytes);
        if !(base == 0 || (2..=36).contains(&base))
            || !matches!(core::mem::size_of::<libc::c_long>(), 4 | 8)
        {
            return Err(NumericEnvironmentUnavailable::Conversion);
        }
        let before = state();
        let mut end = std::ptr::null_mut();
        let radix =
            libc::c_int::try_from(base).map_err(|_| NumericEnvironmentUnavailable::Conversion)?;
        // SAFETY: original owns a NUL-terminated allocation through the C call
        // and end-pointer observation. The admitted radix is valid for strtol.
        let (value, consumed) = unsafe {
            let value = libc::strtol(original.as_ptr(), &raw mut end, radix);
            (value, end.offset_from(original.as_ptr()))
        };
        let after = state();
        let value = i64::try_from(value).map_err(|_| NumericEnvironmentUnavailable::Conversion)?;
        let end =
            usize::try_from(consumed).map_err(|_| NumericEnvironmentUnavailable::Conversion)?;
        if end > original.as_bytes().len() {
            return Err(NumericEnvironmentUnavailable::Conversion);
        }
        Ok(SignedNumericConversion {
            value,
            end,
            before,
            after,
        })
    }

    pub(super) fn double(
        bytes: &[u8],
        reset_errno: bool,
    ) -> Result<DoubleNumericConversion, NumericEnvironmentUnavailable> {
        let original = input(bytes);
        let before = state();
        let mut end = std::ptr::null_mut();
        // SAFETY: the errno cell belongs to this thread and original remains
        // NUL-terminated/alive through strtod and end-pointer inspection.
        let (value, consumed) = unsafe {
            if reset_errno {
                *errno_pointer() = 0;
            }
            let value = libc::strtod(original.as_ptr(), &raw mut end);
            (value, end.offset_from(original.as_ptr()))
        };
        let after = state();
        let end =
            usize::try_from(consumed).map_err(|_| NumericEnvironmentUnavailable::Conversion)?;
        if end > original.as_bytes().len() {
            return Err(NumericEnvironmentUnavailable::Conversion);
        }
        Ok(DoubleNumericConversion {
            value,
            end,
            before,
            after,
        })
    }
}

impl NumericEnvironment for NativeNumericEnvironment {
    fn c_integer_abi(&self) -> Result<NativeCIntegerAbi, NumericEnvironmentUnavailable> {
        if !Self::supported() {
            return Err(NumericEnvironmentUnavailable::Target);
        }
        // Reviewed libc target bindings describe this host's integer layout.
        // They grant no foreign Tcl build, object or selected-handler authority.
        Ok(NativeCIntegerAbi {
            char_bits: u8::try_from(u8::BITS).map_err(|_| NumericEnvironmentUnavailable::Target)?,
            int_bytes: u8::try_from(core::mem::size_of::<libc::c_int>())
                .map_err(|_| NumericEnvironmentUnavailable::Target)?,
            long_bytes: u8::try_from(core::mem::size_of::<libc::c_long>())
                .map_err(|_| NumericEnvironmentUnavailable::Target)?,
        })
    }

    fn state(&self) -> Result<NumericErrorState, NumericEnvironmentUnavailable> {
        #[cfg(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "ios",
            target_os = "freebsd",
            all(target_arch = "wasm32", target_os = "wasi", target_env = "p1")
        ))]
        {
            Ok(native::state())
        }
        #[cfg(not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "ios",
            target_os = "freebsd",
            all(target_arch = "wasm32", target_os = "wasi", target_env = "p1")
        )))]
        {
            Err(NumericEnvironmentUnavailable::Target)
        }
    }

    fn reset(&self) -> Result<(), NumericEnvironmentUnavailable> {
        #[cfg(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "ios",
            target_os = "freebsd",
            all(target_arch = "wasm32", target_os = "wasi", target_env = "p1")
        ))]
        {
            native::reset();
            Ok(())
        }
        #[cfg(not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "ios",
            target_os = "freebsd",
            all(target_arch = "wasm32", target_os = "wasi", target_env = "p1")
        )))]
        {
            Err(NumericEnvironmentUnavailable::Target)
        }
    }
    fn unsigned_c84(
        &self,
        input: &[u8],
        offset: usize,
        long: bool,
    ) -> Result<UnsignedNumericConversion, NumericEnvironmentUnavailable> {
        #[cfg(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "ios",
            target_os = "freebsd",
            all(target_arch = "wasm32", target_os = "wasi", target_env = "p1")
        ))]
        {
            native::unsigned_c84(input, offset, long)
        }
        #[cfg(not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "ios",
            target_os = "freebsd",
            all(target_arch = "wasm32", target_os = "wasi", target_env = "p1")
        )))]
        {
            let _ = (input, offset, long);
            Err(NumericEnvironmentUnavailable::Target)
        }
    }

    fn unsigned(
        &self,
        input: &[u8],
        offset: usize,
        base: u32,
    ) -> Result<UnsignedNumericConversion, NumericEnvironmentUnavailable> {
        #[cfg(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "ios",
            target_os = "freebsd",
            all(target_arch = "wasm32", target_os = "wasi", target_env = "p1")
        ))]
        {
            native::unsigned(input, offset, base)
        }
        #[cfg(not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "ios",
            target_os = "freebsd",
            all(target_arch = "wasm32", target_os = "wasi", target_env = "p1")
        )))]
        {
            let _ = (input, offset, base);
            Err(NumericEnvironmentUnavailable::Target)
        }
    }

    fn signed_long(
        &self,
        input: &[u8],
        base: u32,
    ) -> Result<SignedNumericConversion, NumericEnvironmentUnavailable> {
        #[cfg(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "ios",
            target_os = "freebsd",
            all(target_arch = "wasm32", target_os = "wasi", target_env = "p1")
        ))]
        {
            native::signed_long(input, base)
        }
        #[cfg(not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "ios",
            target_os = "freebsd",
            all(target_arch = "wasm32", target_os = "wasi", target_env = "p1")
        )))]
        {
            let _ = (input, base);
            Err(NumericEnvironmentUnavailable::Target)
        }
    }

    fn double(
        &self,
        input: &[u8],
        reset_errno: bool,
    ) -> Result<DoubleNumericConversion, NumericEnvironmentUnavailable> {
        #[cfg(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "ios",
            target_os = "freebsd",
            all(target_arch = "wasm32", target_os = "wasi", target_env = "p1")
        ))]
        {
            native::double(input, reset_errno)
        }
        #[cfg(not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "ios",
            target_os = "freebsd",
            all(target_arch = "wasm32", target_os = "wasi", target_env = "p1")
        )))]
        {
            let _ = (input, reset_errno);
            Err(NumericEnvironmentUnavailable::Target)
        }
    }
}
