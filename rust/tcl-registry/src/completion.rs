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

use tcl_cmd_core::return_options::ReturnOptionsProtocol;
pub use tcl_core_types::Code as CompletionCode;

/// Parse Tcl's integer completion-code spelling and apply its C-compatible
/// 32-bit conversion. Tcl accepts `INT_MIN..UINT_MAX`: the upper unsigned
/// half wraps into the corresponding signed code (`4294967295` is `-1`).
/// Values outside that range, including integers beyond Tcl's wide parser,
/// are not valid completion-code selectors.
#[must_use]
pub fn canonical_completion_code(value: &str, numbers: tcl_syntax::number::Numbers) -> Option<i32> {
    tcl_cmd_core::return_options::completion_code_integer(value, numbers)
}

/// Map the five standard Tcl completion-code names.
#[must_use]
pub fn standard_completion_code(value: &str) -> Option<CompletionCode> {
    ReturnOptionsProtocol::Tcl86Plus
        .named_completion_code(value.as_bytes())
        .map(CompletionCode::from_int)
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
    standard_completion_code(value)
        .or_else(|| canonical_completion_code(value, numbers).map(CompletionCode::from_int))
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
    let spelling = value.as_bytes();
    if let Some(code) = standard_completion_code(value) {
        return Exact(code);
    }
    if matches!(
        policy,
        CompletionCodePolicy::Jim | CompletionCodePolicy::Unknown
    ) && let Some(code) = ReturnOptionsProtocol::Jim084.named_completion_code(spelling)
    {
        return if policy == CompletionCodePolicy::Jim {
            Exact(CompletionCode::from_int(code))
        } else {
            Unknown
        };
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
    Exact(CompletionCode::from_int(
        tcl_syntax::number::native_int32_low_bits(number),
    ))
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

impl crate::knowledge::Knowledge<CompletionDescriptor> {
    /// The descriptor a consumer acts on: the stated one, or
    /// [`CompletionDescriptor::CONSERVATIVE`] when nothing is known.
    #[must_use]
    pub const fn or_conservative(&self) -> CompletionDescriptor {
        match self.stated() {
            Some(descriptor) => descriptor,
            None => CompletionDescriptor::CONSERVATIVE,
        }
    }
}

/// A completion conversion performed at an evaluated region's boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionConversion {
    /// Catch captures every Tcl code as a normal integer result; process exit escapes.
    CaptureAsResult,
    /// A loop consumes break as normal termination and continue as its next iteration.
    LoopControl,
    /// Selected on/trap handlers may replace a captured code; finally preserves or replaces it.
    SelectedHandlers,
    /// A return level is decremented; at level one the configured code becomes visible.
    ProcedureReturn,
    /// Direct break and continue from a procedure body become errors before return handling.
    /// Codes exposed by `ProcedureReturn` are forwarded to the caller instead.
    EscapingLoopControlIsError,
}

/// Where a command's declared completion boundary applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionBoundarySite {
    /// Bodies evaluated by this invocation.
    EvaluatedBodies,
    /// A callable defined here, when that callable is subsequently invoked.
    DefinedCallable,
}

/// Interception and conversion of nested completion codes, independent of CFG shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompletionBoundary {
    /// Region whose completion crosses this boundary.
    pub site: CompletionBoundarySite,
    /// Codes intercepted rather than propagated unchanged.
    pub intercepts: CompletionCodeDomain,
    /// Ordered semantic conversions; none means unchanged propagation.
    pub conversions: &'static [CompletionConversion],
}

impl CompletionBoundary {
    /// A script evaluator forwards every nested code unchanged.
    pub const FORWARD: Self = Self {
        site: CompletionBoundarySite::EvaluatedBodies,
        intercepts: CompletionCodeDomain::Exact(&[]),
        conversions: &[],
    };
    /// Catch converts every nested Tcl completion into a normal integer result.
    pub const CATCH: Self = Self {
        intercepts: CompletionCodeDomain::Any,
        conversions: &[CompletionConversion::CaptureAsResult],
        ..Self::FORWARD
    };
    /// A loop intercepts break and continue, retaining every other completion.
    pub const LOOP: Self = Self {
        intercepts: CompletionCodeDomain::Exact(&[CompletionCode::Break, CompletionCode::Continue]),
        conversions: &[CompletionConversion::LoopControl],
        ..Self::FORWARD
    };
    /// Try selectors and handlers determine the intercepted subset at invocation resolution.
    pub const TRY: Self = Self {
        intercepts: CompletionCodeDomain::Any,
        conversions: &[CompletionConversion::SelectedHandlers],
        ..Self::FORWARD
    };
    /// An invoked procedure rejects direct loop codes, then consumes return levels.
    pub const PROCEDURE: Self = Self {
        site: CompletionBoundarySite::DefinedCallable,
        intercepts: CompletionCodeDomain::Exact(&[
            CompletionCode::Return,
            CompletionCode::Break,
            CompletionCode::Continue,
        ]),
        conversions: &[
            CompletionConversion::EscapingLoopControlIsError,
            CompletionConversion::ProcedureReturn,
        ],
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
    /// Nested-region boundary; absent means no declared interception contract.
    pub body_boundary: Option<CompletionBoundary>,
}

impl CompletionDescriptor {
    /// Conservative descriptor for an ordinary generic invocation.
    pub const CONSERVATIVE: Self = Self {
        codes: CompletionCodeDomain::Any,
        payloads: CompletionPayloadObligations::UNKNOWN,
        value_semantics: CompletionValueSemantics::None,
        body_boundary: None,
    };

    /// Attach nested completion semantics without changing the invocation's own code domain.
    #[must_use]
    pub const fn with_body_boundary(mut self, boundary: CompletionBoundary) -> Self {
        self.body_boundary = Some(boundary);
        self
    }

    /// Build an exact completion descriptor with produced result/options.
    #[must_use]
    pub const fn exact(codes: &'static [CompletionCode]) -> Self {
        Self {
            codes: CompletionCodeDomain::Exact(codes),
            payloads: CompletionPayloadObligations::PRODUCED,
            value_semantics: CompletionValueSemantics::None,
            body_boundary: None,
        }
    }

    /// Native tail scheduling retains the target's unresolved eventual payload.
    #[must_use]
    pub const fn tailcall() -> Self {
        Self {
            codes: CompletionCodeDomain::Any,
            payloads: CompletionPayloadObligations::UNKNOWN,
            value_semantics: CompletionValueSemantics::Tailcall,
            body_boundary: None,
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
            body_boundary: None,
        }
    }
}
