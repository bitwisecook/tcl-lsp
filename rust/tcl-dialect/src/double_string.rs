// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Native lazy double-string conversion and interpreter precision policy.

/// Native ownership of double-to-string precision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DoubleStringPolicy {
    /// Tcl 8.4: shared mutable precision, initially twelve significant digits.
    Tcl84Precision,
    /// Tcl 8.5/8.6: shared mutable precision, initially shortest round trip.
    TclPrecision,
    /// Tcl 9: shortest round trip, unaffected by an ordinary `tcl_precision` variable.
    Shortest,
    /// Current Jim: twelve significant digits, unaffected by Tcl's variable.
    JimTwelve,
}

/// A fully selected conversion at the first string materialisation of an object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DoubleFormat {
    /// Tcl shortest round-trip digits with its fixed decimal notation range.
    Shortest,
    /// Significant digits with the precision-dependent C `%g` notation range.
    General(u8),
    /// Significant digits with Tcl 8.5/8.6's fixed decimal notation range.
    TclSignificant(u8),
}

impl DoubleStringPolicy {
    /// Policy of a known C Tcl release.
    #[must_use]
    pub const fn for_tcl_version(version: crate::TclVersion) -> Self {
        match version {
            crate::TclVersion::V8_4 => Self::Tcl84Precision,
            crate::TclVersion::V8_5 | crate::TclVersion::V8_6 => Self::TclPrecision,
            crate::TclVersion::V9_0 | crate::TclVersion::V9_1 => Self::Shortest,
        }
    }

    /// Whether Tcl's linked precision variable controls string conversion.
    #[must_use]
    pub const fn has_precision_variable(self) -> bool {
        matches!(self, Self::Tcl84Precision | Self::TclPrecision)
    }

    /// Global variable whose native trace owns shared precision, when present.
    #[must_use]
    pub const fn precision_variable(self) -> Option<&'static str> {
        if self.has_precision_variable() {
            Some("::tcl_precision")
        } else {
            None
        }
    }

    /// Initial shared precision for a fresh execution thread.
    #[must_use]
    pub const fn default_precision(self) -> u8 {
        match self {
            Self::Tcl84Precision | Self::JimTwelve => 12,
            Self::TclPrecision | Self::Shortest => 0,
        }
    }

    /// Conversion selected by a proved current precision. Fixed policies ignore it.
    #[must_use]
    pub const fn format(self, precision: u8) -> Option<DoubleFormat> {
        match self {
            Self::Shortest => Some(DoubleFormat::Shortest),
            Self::JimTwelve => Some(DoubleFormat::General(12)),
            Self::Tcl84Precision if precision > 0 && precision <= 17 => {
                Some(DoubleFormat::General(precision))
            }
            Self::TclPrecision if precision == 0 => Some(DoubleFormat::Shortest),
            Self::TclPrecision if precision <= 17 => Some(DoubleFormat::TclSignificant(precision)),
            Self::Tcl84Precision | Self::TclPrecision => None,
        }
    }

    /// A conversion valid without proof of mutable interpreter precision.
    #[must_use]
    pub const fn constant_format(self) -> Option<DoubleFormat> {
        match self {
            Self::Shortest => Some(DoubleFormat::Shortest),
            Self::JimTwelve => Some(DoubleFormat::General(12)),
            Self::Tcl84Precision | Self::TclPrecision => None,
        }
    }
}
