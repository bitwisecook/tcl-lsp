// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Binary-handler usage tails, separate from the actual invocation prefix.

/// Native public Binary entry protocol, independently of compiler hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeBinaryRootDispatch {
    /// A monolithic C handler selects an abbreviating option table.
    Indexed,
    /// A real C ensemble resolves its configured subcommand map.
    Ensemble,
    /// The Jim bootstrap procedure tailcalls a compound command name.
    Scripted,
}

/// Actual binary handler selected by the runtime's registered ingress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeBinaryArgumentUsage {
    /// Binary object construction.
    Format,
    /// Binary object unpacking and variable outputs.
    Scan,
    /// Hexadecimal encoding.
    EncodeHex,
    /// Wrapped base64 or uuencode encoding.
    EncodeWrapped,
    /// Hexadecimal, base64 or uuencode decoding.
    Decode,
}

impl NativeBinaryArgumentUsage {
    /// Existing generic-handler presentation when no native target is supplied.
    /// This compatibility text supplies no native availability or admission proof.
    #[must_use]
    pub const fn compatibility_usage(self) -> &'static str {
        match self {
            Self::Format => "formatString ?arg ...?",
            Self::Scan => "value formatString ?varName ...?",
            Self::EncodeHex => "data",
            Self::EncodeWrapped => "?-maxlen len? ?-wrapchar char? data",
            Self::Decode => "?options? data",
        }
    }
}

impl crate::InvocationDialect {
    /// Measured public Binary registration protocol. An unknown native point
    /// supplies no protocol; this query grants no handler or compiler identity.
    #[must_use]
    pub fn binary_root_dispatch(self) -> Option<NativeBinaryRootDispatch> {
        if let Some(version) = self.tcl_version {
            return Some(if version < tcl_dialect::TclVersion::V8_6 {
                NativeBinaryRootDispatch::Indexed
            } else {
                NativeBinaryRootDispatch::Ensemble
            });
        }
        self.binary_scripted_ingress()
            .map(|_| NativeBinaryRootDispatch::Scripted)
    }

    /// Whether the indexed public handler attaches `TCL LOOKUP INDEX` to a
    /// member error. C8.4 reports `NONE`; C8.5 attaches the lookup identity.
    /// Other entry protocols or unknown native releases supply no policy.
    #[must_use]
    pub fn binary_root_index_error_code(self) -> Option<bool> {
        match self.tcl_version {
            Some(tcl_dialect::TclVersion::V8_4) => Some(false),
            Some(tcl_dialect::TclVersion::V8_5) => Some(true),
            _ => None,
        }
    }

    /// Measured handler usage, without a command or ensemble-member prefix.
    /// The runtime retains and rewrites that prefix through actual dispatch.
    #[must_use]
    pub fn binary_argument_usage(
        self,
        operation: NativeBinaryArgumentUsage,
    ) -> Option<&'static str> {
        use NativeBinaryArgumentUsage::{Decode, EncodeHex, EncodeWrapped, Format, Scan};
        if let Some(version) = self.tcl_version {
            return match operation {
                Format if version < tcl_dialect::TclVersion::V8_6 => {
                    Some("formatString ?arg arg ...?")
                }
                Scan if version < tcl_dialect::TclVersion::V8_6 => {
                    Some("value formatString ?varName varName ...?")
                }
                Format => Some("formatString ?arg ...?"),
                Scan => Some("value formatString ?varName ...?"),
                EncodeHex if version >= tcl_dialect::TclVersion::V8_6 => Some("data"),
                EncodeWrapped if version >= tcl_dialect::TclVersion::V8_6 => {
                    Some("?-maxlen len? ?-wrapchar char? data")
                }
                Decode if version >= tcl_dialect::TclVersion::V8_6 => Some("?options? data"),
                EncodeHex | EncodeWrapped | Decode => None,
            };
        }
        if self
            .core_point
            .is_some_and(|point| point.release() == tcl_dialect::model::Release::JIM_0_84)
        {
            return match operation {
                Format => Some("formatString ?arg ...?"),
                Scan => Some("value formatString ?varName ...?"),
                EncodeHex | EncodeWrapped | Decode => None,
            };
        }
        None
    }
}
