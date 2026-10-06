// SPDX-License-Identifier: AGPL-3.0-or-later
//! Authentic pinned Jim original-object lookup policy.

impl crate::InvocationDialect {
    /// Actual pinned Jim lookup-cache policy; compatibility versions and C
    /// command-name caches cannot supply this original-object protocol.
    #[must_use]
    pub fn native_jim_lookup_protocol(
        self,
    ) -> Option<tcl_syntax::native_jim_lookup::NativeJimLookupProtocol> {
        (self.native_name_protocol()? == tcl_syntax::naming::NativeNameProtocol::Jim084)
            .then_some(tcl_syntax::native_jim_lookup::NativeJimLookupProtocol::jim084())
    }
}
