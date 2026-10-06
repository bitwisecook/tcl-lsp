// SPDX-License-Identifier: AGPL-3.0-or-later
//! Jim switch's original two-byte option scan, without object/cache authority.

/// Pure pinned Jim switch recipe; live engines independently select its issuer.
#[derive(Clone, Copy, Debug)]
pub struct NativeJimSwitchProtocol(());
/// One reached original switch option, not an Enum lookup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeJimSwitchOption {
    /// This original argument starts the subject/case vector.
    Subject,
    /// End option scanning after this argument.
    End,
    /// Select counted exact object equality.
    Exact,
    /// Select the original Jim glob matcher.
    Glob,
    /// Select a fresh original regexp command and retain the -- flag.
    Regexp,
    /// Consume the following original command-name object.
    Command,
    /// Original diagnostic input for an unknown option.
    Invalid,
}
impl NativeJimSwitchProtocol {
    /// Pinned pure recipe, without interpreter authority.
    #[must_use]
    pub const fn jim084() -> Self {
        Self(())
    }
    /// Reach Jim's strncmp(option, literal, 2) scan on original counted bytes.
    /// The scanner installs no Enum or `ComparedString` primary.
    #[must_use]
    pub fn option(self, original: &[u8]) -> NativeJimSwitchOption {
        use NativeJimSwitchOption as O;
        if original.first() != Some(&b'-') {
            return O::Subject;
        }
        match original.get(1).copied().unwrap_or(0) {
            b'-' => O::End,
            b'e' => O::Exact,
            b'g' => O::Glob,
            b'r' => O::Regexp,
            b'c' => O::Command,
            _ => O::Invalid,
        }
    }
}
