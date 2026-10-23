// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual Jim switch issuer, separate from logical C option grammars.
pub use tcl_syntax::native_jim_switch::NativeJimSwitchProtocol;
impl crate::InvocationDialect {
    /// Select the pinned Jim core switch's original option and matching recipe.
    #[must_use]
    pub fn native_jim_switch_protocol(self) -> Option<NativeJimSwitchProtocol> {
        self.native_string_protocol()
            .filter(|strings| strings.is_jim084())
            .map(|_| NativeJimSwitchProtocol::jim084())
    }
}
