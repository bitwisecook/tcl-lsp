// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Actual object conversion performed by native character-length access.

/// Representation changes made by the selected native length accessor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeStringLengthRepresentation {
    /// C8.4/8.5 convert the object to its string internal representation.
    String,
    /// C8.6 preserves any byte-array internal representation and counts bytes.
    AnyByteArray,
    /// C9 preserves only a pure, proper byte-array representation.
    PureProperByteArray,
    /// Jim converts to its string internal representation and caches the count.
    JimCachedString,
}

impl NativeStringLengthRepresentation {
    /// Modern C returns an existing zero/one-byte string length without shimmer.
    #[must_use]
    pub fn preserves_short_string(self) -> bool {
        matches!(self, Self::AnyByteArray | Self::PureProperByteArray)
    }

    /// Whether an actual byte-array payload supplies the count directly.
    #[must_use]
    pub fn counts_byte_array(self, pure: bool, proper: bool) -> bool {
        match self {
            Self::AnyByteArray => true,
            Self::PureProperByteArray => pure && proper,
            Self::String | Self::JimCachedString => false,
        }
    }
}

impl crate::InvocationDialect {
    /// The selected interpreter's length conversion, independent of its compiler.
    /// Unknown engine points do not acquire an object-conversion protocol.
    #[must_use]
    pub fn string_length_representation(self) -> Option<NativeStringLengthRepresentation> {
        self.native_string_protocol()
            .map(native_string_length_representation)
    }

    /// Request the independently authored logical string recipe explicitly.
    #[must_use]
    pub fn string_length_representation_with_provider(
        self,
        provider: Option<crate::native_string_materialization::LogicalStringProvider>,
    ) -> Option<NativeStringLengthRepresentation> {
        self.native_string_materialization(provider)
            .map(|recipe| native_string_length_representation(recipe.protocol()))
    }
}

fn native_string_length_representation(
    protocol: tcl_syntax::native_string::NativeStringProtocol,
) -> NativeStringLengthRepresentation {
    use NativeStringLengthRepresentation as Representation;
    use tcl_dialect::TclVersion;
    if let Some(version) = protocol.tcl_version() {
        return match version {
            TclVersion::V8_4 | TclVersion::V8_5 => Representation::String,
            TclVersion::V8_6 => Representation::AnyByteArray,
            TclVersion::V9_0 | TclVersion::V9_1 => Representation::PureProperByteArray,
        };
    }
    Representation::JimCachedString
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actual_length_conversion_retains_release_specific_bytearray_rules() {
        use NativeStringLengthRepresentation as Representation;
        use tcl_dialect::TclVersion;
        for (version, expected) in [
            (TclVersion::V8_4, Representation::String),
            (TclVersion::V8_5, Representation::String),
            (TclVersion::V8_6, Representation::AnyByteArray),
            (TclVersion::V9_0, Representation::PureProperByteArray),
            (TclVersion::V9_1, Representation::PureProperByteArray),
        ] {
            assert_eq!(
                crate::InvocationDialect::for_version(version).string_length_representation(),
                Some(expected)
            );
        }
        assert!(Representation::AnyByteArray.counts_byte_array(false, false));
        assert!(!Representation::PureProperByteArray.counts_byte_array(false, true));
        assert!(!Representation::PureProperByteArray.counts_byte_array(true, false));
        assert!(Representation::PureProperByteArray.counts_byte_array(true, true));
        let jim = crate::model::ingress::resolve_environment("jim").analyser_profile();
        assert_eq!(
            crate::InvocationDialect::of_profile(jim).string_length_representation(),
            Some(Representation::JimCachedString)
        );
        assert_eq!(
            crate::InvocationDialect::of_profile(jim)
                .string_length_representation_with_provider(Some(
                crate::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation
            ),),
            Some(Representation::JimCachedString)
        );
    }
}
