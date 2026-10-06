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

/// Map a Tcl completion-code *selector* to its code.
///
/// A selector is the word a `return -code` option or a `try … on` clause
/// names: one of the five standard spellings, or any integer completion code
/// in Tcl's accepted range.  Returns `None` for a word that is neither.
#[must_use]
pub fn completion_code_selector(
    value: &str,
    numbers: tcl_syntax::number::Numbers,
) -> Option<CompletionCode> {
    match value {
        "ok" => Some(CompletionCode::Ok),
        "error" => Some(CompletionCode::Error),
        "return" => Some(CompletionCode::Return),
        "break" => Some(CompletionCode::Break),
        "continue" => Some(CompletionCode::Continue),
        value => canonical_completion_code(value, numbers).map(CompletionCode::from_int),
    }
}

/// Native numeric and named completion-selector conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompletionCodePolicy {
    /// C Tcl 8.x accepts signed magnitudes through `UINT_MAX` before conversion.
    Tcl8,
    /// C Tcl 9.x accepts `INT_MIN` through `UINT_MAX` before conversion.
    Tcl9,
    /// Jim accepts signed-wide codes and its extra native code names.
    Jim,
    /// No engine conversion has been established.
    Unknown,
}

impl CompletionCodePolicy {
    /// Protocol corresponding to an explicitly selected native numeral grammar.
    /// A grammar union retains unknown rather than selecting one engine.
    #[must_use]
    pub fn for_numbers(numbers: tcl_syntax::number::Numbers) -> Self {
        match numbers.syntax() {
            Some(tcl_dialect::NumberSyntax::Tcl84 | tcl_dialect::NumberSyntax::Tcl85) => Self::Tcl8,
            Some(tcl_dialect::NumberSyntax::Tcl90) => Self::Tcl9,
            Some(tcl_dialect::NumberSyntax::Jim | tcl_dialect::NumberSyntax::Jim080) => Self::Jim,
            None => Self::Unknown,
        }
    }
}

impl crate::InvocationDialect {
    /// Completion selector protocol of the actual native runtime.
    #[must_use]
    pub fn completion_code_policy(self) -> CompletionCodePolicy {
        if self.family() == Some(tcl_dialect::model::Family::Jim) {
            return CompletionCodePolicy::Jim;
        }
        match self.tcl_version {
            Some(version) if version < tcl_dialect::TclVersion::V9_0 => CompletionCodePolicy::Tcl8,
            Some(_) => CompletionCodePolicy::Tcl9,
            None => CompletionCodePolicy::Unknown,
        }
    }
}

/// Result of a dialect-sensitive completion selector, preserving ambiguity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionCodeSelection {
    /// A proved native code after conversion.
    Exact(CompletionCode),
    /// The selected native grammar rejects this literal.
    Invalid,
    /// Numeric or engine-dependent residue has not been resolved.
    Unknown,
}

/// Read a native completion selector with the shared numeric grammar.
/// Unlike a rejected literal, unresolved numeric grammar retains unknown.
///
/// ```
/// use tcl_registry::completion::{CompletionCodeSelection, CompletionCodePolicy, resolve_completion_code_selector};
/// use tcl_syntax::number::Numbers;
/// assert_eq!(resolve_completion_code_selector("ok", Numbers::Unknown, CompletionCodePolicy::Unknown),
///     CompletionCodeSelection::Exact(tcl_registry::completion::CompletionCode::Ok));
/// ```
#[must_use]
pub fn resolve_completion_code_selector(
    value: &str,
    numbers: tcl_syntax::number::Numbers,
    policy: CompletionCodePolicy,
) -> CompletionCodeSelection {
    use CompletionCodeSelection::{Exact, Invalid, Unknown};
    let named = match value {
        "ok" => Some(CompletionCode::Ok),
        "error" => Some(CompletionCode::Error),
        "return" => Some(CompletionCode::Return),
        "break" => Some(CompletionCode::Break),
        "continue" => Some(CompletionCode::Continue),
        "signal" | "exit" | "eval" if policy == CompletionCodePolicy::Unknown => return Unknown,
        "signal" if policy == CompletionCodePolicy::Jim => Some(CompletionCode::Other(5)),
        "exit" if policy == CompletionCodePolicy::Jim => Some(CompletionCode::Other(6)),
        "eval" if policy == CompletionCodePolicy::Jim => Some(CompletionCode::Other(7)),
        _ => None,
    };
    if let Some(code) = named {
        return Exact(code);
    }
    let Some(number) = numbers.parse_wide(value) else {
        return match numbers.parse_whole(value) {
            Some(
                tcl_syntax::number::Number::Double(_) | tcl_syntax::number::Number::Nan { .. },
            ) => Invalid,
            Some(tcl_syntax::number::Number::Big { .. })
                if matches!(
                    policy,
                    CompletionCodePolicy::Tcl8 | CompletionCodePolicy::Tcl9
                ) =>
            {
                Invalid
            }
            _ if numbers.is_number_in_any_release(value) => Unknown,
            _ => Invalid,
        };
    };
    let lower = match policy {
        CompletionCodePolicy::Tcl8 => -i64::from(u32::MAX),
        CompletionCodePolicy::Tcl9 | CompletionCodePolicy::Unknown => i64::from(i32::MIN),
        CompletionCodePolicy::Jim => i64::MIN,
    };
    if number < lower || (policy != CompletionCodePolicy::Jim && number > i64::from(u32::MAX)) {
        return if policy == CompletionCodePolicy::Unknown {
            Unknown
        } else {
            Invalid
        };
    }
    let bytes = number.to_le_bytes();
    Exact(CompletionCode::from_int(i32::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3],
    ])))
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

/// Value-dependent completion behaviour declared by a command descriptor.
///
/// Most Tcl commands' completion does not depend on parsing one of their
/// ordinary value operands, so [`Self::None`] keeps their descriptor purely
/// code-domain based.  `exit` is the important exception: a valid integer
/// status terminates the process, while an invalid status raises Tcl error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionValueSemantics {
    /// No value-sensitive completion parser is needed.
    None,
    /// Native tail scheduling: C return (2), Jim eval (7), and validation error.
    /// Jim's empty target list is normal in a procedure; runtime root is invalid.
    Tailcall,
    /// Parse an optional Tcl integer process-exit status.
    ///
    /// Omission and a static valid integer terminate the process; a static
    /// invalid value raises `TCL_ERROR`; a dynamic word is the typed union of
    /// those two outcomes.  The registry applies this parser before generic
    /// terminal traits, so invalid `exit` values never become false process
    /// exits in diagram or control-flow consumers.
    ProcessExitStatus,
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
    /// Value-sensitive completion parser, when this command declares one.
    pub value_semantics: CompletionValueSemantics,
}

impl CompletionDescriptor {
    /// Conservative descriptor for an ordinary generic invocation.
    pub const CONSERVATIVE: Self = Self {
        codes: CompletionCodeDomain::Any,
        payloads: CompletionPayloadObligations::UNKNOWN,
        value_semantics: CompletionValueSemantics::None,
    };

    /// Build an exact completion descriptor with produced result/options.
    #[must_use]
    pub const fn exact(codes: &'static [CompletionCode]) -> Self {
        Self {
            codes: CompletionCodeDomain::Exact(codes),
            payloads: CompletionPayloadObligations::PRODUCED,
            value_semantics: CompletionValueSemantics::None,
        }
    }

    /// Native tail scheduling retains the target's unresolved eventual payload.
    #[must_use]
    pub const fn tailcall() -> Self {
        Self {
            codes: CompletionCodeDomain::Any,
            payloads: CompletionPayloadObligations::UNKNOWN,
            value_semantics: CompletionValueSemantics::Tailcall,
        }
    }

    /// Build the value-sensitive descriptor for `exit ?returnCode?`.
    #[must_use]
    pub const fn process_exit_status() -> Self {
        Self {
            // Process termination is deliberately not a Tcl completion code.
            // The value parser above is the authoritative projection.
            codes: CompletionCodeDomain::Exact(&[]),
            payloads: CompletionPayloadObligations::PRODUCED,
            value_semantics: CompletionValueSemantics::ProcessExitStatus,
        }
    }
}
