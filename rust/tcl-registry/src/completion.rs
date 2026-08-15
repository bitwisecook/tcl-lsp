// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Target-neutral completion semantics declared by command specifications.
//!
//! Every Tcl invocation produces a completion code, result, and return-options
//! dictionary.  The registry owns the static part of that contract so common
//! compiler analyses need not recognise command spellings.  Runtime command
//! binding, traces, and substitutions can still make an invocation more
//! dynamic; an omitted descriptor therefore resolves to the deliberately
//! conservative [`CompletionDescriptor::CONSERVATIVE`].

pub use tcl_core_types::Code as CompletionCode;

/// Parse Tcl's integer completion-code spelling and apply its C-compatible
/// 32-bit conversion. Tcl accepts `INT_MIN..UINT_MAX`: the upper unsigned
/// half wraps into the corresponding signed code (`4294967295` is `-1`).
/// Values outside that range, including integers beyond Tcl's wide parser,
/// are not valid completion-code selectors.
#[must_use]
#[allow(clippy::cast_possible_truncation)] // Intentional Tcl UINT_MAX → signed-code wrap.
pub fn canonical_completion_code(value: &str, numbers: tcl_syntax::number::Numbers) -> Option<i32> {
    let value = numbers.parse_wide(value)?;
    if value < i64::from(i32::MIN) || value > i64::from(u32::MAX) {
        return None;
    }
    Some(value as i32)
}

/// The statically possible Tcl completion codes for an invocation.
///
/// [`Self::Exact`] retains named Tcl codes and arbitrary integer codes alike:
/// use `CompletionCode::Other(n)` for a `return -code n` / `try on n`-style
/// custom completion. [`Self::Any`] is the conservative declaration for a
/// command whose completion cannot be described independently of runtime
/// inputs or callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionCodeDomain {
    /// A finite, exact set of possible completion codes.
    Exact(&'static [CompletionCode]),
    /// Any Tcl integer completion code may result.
    Any,
}

/// The data-flow obligation for one completion payload.
///
/// This deliberately describes provenance rather than a runtime value type.
/// Future executable CFGs must carry both a result and options value on every
/// completion edge; this tells them whether an operation creates the payload,
/// forwards one from a nested evaluation, or requires a conservative runtime
/// representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionPayloadObligation {
    /// The operation creates the payload for its completion.
    Produced,
    /// The operation preserves a nested completion payload.
    Forwarded,
    /// The operation may create or forward a payload; retain it conservatively.
    Unknown,
}

/// Result and return-options obligations for one completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompletionPayloadObligations {
    /// How a future executable CFG must represent the result value.
    pub result: CompletionPayloadObligation,
    /// How a future executable CFG must represent the return-options value.
    pub options: CompletionPayloadObligation,
}

impl CompletionPayloadObligations {
    /// A command that produces its own result and options values.
    pub const PRODUCED: Self = Self {
        result: CompletionPayloadObligation::Produced,
        options: CompletionPayloadObligation::Produced,
    };

    /// A command that preserves a nested completion's result and options.
    pub const FORWARDED: Self = Self {
        result: CompletionPayloadObligation::Forwarded,
        options: CompletionPayloadObligation::Forwarded,
    };

    /// Conservative payload provenance for a generic dynamic invocation.
    pub const UNKNOWN: Self = Self {
        result: CompletionPayloadObligation::Unknown,
        options: CompletionPayloadObligation::Unknown,
    };
}

/// Target-neutral completion contract for a command, subcommand, or form.
///
/// A form descriptor overrides a resolved subcommand descriptor, which in
/// turn overrides its parent command descriptor. The resolver applies
/// [`Self::CONSERVATIVE`] only when none of those registry declarations is
/// present.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompletionDescriptor {
    /// Static completion-code domain.
    pub codes: CompletionCodeDomain,
    /// Result and return-options data-flow obligations.
    pub payloads: CompletionPayloadObligations,
}

impl CompletionDescriptor {
    /// Conservative descriptor for an ordinary generic invocation.
    pub const CONSERVATIVE: Self = Self {
        codes: CompletionCodeDomain::Any,
        payloads: CompletionPayloadObligations::UNKNOWN,
    };

    /// Build an exact completion descriptor with produced result/options.
    #[must_use]
    pub const fn exact(codes: &'static [CompletionCode]) -> Self {
        Self {
            codes: CompletionCodeDomain::Exact(codes),
            payloads: CompletionPayloadObligations::PRODUCED,
        }
    }
}
