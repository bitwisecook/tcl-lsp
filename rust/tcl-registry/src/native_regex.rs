// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original compiled-pattern recipes selected by the actual C string issuer.

impl crate::InvocationDialect {
    /// Select the actual C issuer's compiled-pattern protocol independently of
    /// logical grammar. Missing or Jim issuers supply no C `RegExp` authority.
    #[must_use]
    pub fn native_regex_protocol(self) -> Option<tcl_syntax::native_regex::NativeRegexpRecipe> {
        self.native_string_protocol()?
            .tcl_version()
            .map(tcl_syntax::native_regex::NativeRegexpRecipe::for_version)
    }
}

impl crate::InvocationDialect {
    /// Select only the actual bundled Jim issuer, independently of C ARE.
    #[must_use]
    pub fn native_jim_regex_protocol(self) -> Option<tcl_syntax::native_regex::JimRegexpRecipe> {
        (self.native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084))
        .then(tcl_syntax::native_regex::JimRegexpRecipe::jim084)
    }
}
