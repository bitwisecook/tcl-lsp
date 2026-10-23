// SPDX-License-Identifier: AGPL-3.0-or-later
//! Explicitly supplied native integer string-updater capability.

/// Actual native object constructor whose updater is being requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeIntegerKind {
    /// Tcl's native C long object.
    Long,
    /// Tcl's 64-bit wide integer object.
    Wide,
}

/// Independently checked identity of the loaded native formatter build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeIntegerFormatterBuild {
    /// `Tcl_GetVersion`'s major, minor, patch, and release-type values.
    pub version: [i32; 4],
    /// SHA-256 of the caller-selected native library.
    pub sha256: [u8; 32],
    /// Native C long width of the admitted ABI.
    pub long_bits: u32,
}

/// No admitted native build, unsupported ABI, or unavailable reached operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeIntegerFormatterUnavailable {
    /// No reviewed dynamic-library ABI on this target.
    Target,
    /// File, digest, symbol origin, or version did not match the supplied build.
    Build,
    /// The reached native constructor/updater cannot serve this request.
    Operation,
}

/// Actual loaded-build formatting, independent of numeric grammar or profile.
/// A host must explicitly supply this capability; no default mathematical
/// spelling establishes the native updater's behavior.
pub trait NativeIntegerFormatter {
    /// The verified library and ABI supplying the updater.
    fn build(&self) -> NativeIntegerFormatterBuild;
    /// Invoke the actual selected constructor and string updater.
    fn format(
        &self,
        kind: NativeIntegerKind,
        value: i64,
    ) -> Result<Vec<u8>, NativeIntegerFormatterUnavailable>;
}
