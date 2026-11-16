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

//! The intrep type of a literal in `set` and in `expr` context, and the decoded contents of an original literal argument.

use crate::types::{TypeLattice, TypeShape};
use tcl_dialect::NumberSyntax;
use tcl_registry::{CommandRegistry, TclType};
use tcl_syntax::number::{Number, ParseFlags, parse_whole_with};

/// The integer-tower shape of a Tcl integer literal under `numbers` —
/// [`TypeShape::Int`] when the value fits a wide (`i64`), [`TypeShape::Bignum`]
/// beyond it (`9223372036854775808`, `0xFFFFFFFFFFFFFFFF` — promotion is by
/// magnitude, never wrapping; see docs type-tracking.md). `None` when
/// the text is not, in its entirety, an integer numeral of the target release.
///
/// The grammar is the shared one ([`tcl_syntax::number`]), so which spellings
/// exist follows the release: `0x` always, `0o`/`0b` from 8.5, `0d` and `_`
/// separators from 9.0, and a bare leading zero is octal up to 8.6 but decimal
/// from 9.0. A prefix the release lacks is not a prefix (`0o17` is a bareword
/// under 8.4), and radix-invalid digits (`0o8`, `0xZZ`) are never a numeral.
/// `integer_only` mirrors `TCL_PARSE_INTEGER_ONLY`: a fractional part is
/// trailing junk, so `1.5` yields `None` here and is classified as a double by
/// the caller.
#[must_use]
fn int_literal_shape(s: &str, numbers: NumberSyntax) -> Option<TypeShape> {
    let flags = ParseFlags {
        integer_only: true,
        ..ParseFlags::for_syntax(numbers)
    };
    match parse_whole_with(s, flags)? {
        Number::Int(_) => Some(TypeShape::Int),
        Number::Big { .. } => Some(TypeShape::Bignum),
        Number::Double(_) | Number::Nan { .. } => None,
    }
}

/// Whether `s` carries the `0o`/`0O` octal prefix — the one integer spelling
/// the **set-statement** classifier deliberately holds back as `String` while
/// the expr classifier calls it an `Int`.
///
/// The divergence is intentional and pinned by tests (`0o17` is `String` after
/// `set x 0o17`, `Int` inside `expr {0o17}`): a `set` stores a pure string whose
/// canonical stringified intrep (`15`) differs from the source text, whereas the
/// expr lexer tokenises the word as a number outright. Only the *spelling* is
/// held back here; whether that spelling is a numeral at all is still the
/// release's business (under 8.4 there is no `0o` prefix, so such a word is a
/// bareword and would be `String` anyway).
#[must_use]
fn has_octal_prefix(s: &str) -> bool {
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);
    // number-drift-ok: this recognises the `0o` *spelling* to apply the
    // intrep divergence above, not to parse a numeral — the value and its
    // validity still come from `int_literal_shape`'s dialect-aware parse. A
    // release that lacks the prefix classifies such a word `String` either way,
    // so the answer does not depend on the grammar.
    body.starts_with("0o") || body.starts_with("0O")
}

/// Classify a literal string as its Tcl intrep type (set-statement context),
/// reading numerals under `numbers` — the target release's grammar.
pub(super) fn literal_type(text: &str, numbers: NumberSyntax) -> TypeLattice {
    let s = text.trim();
    if !has_octal_prefix(s)
        && let Some(shape) = int_literal_shape(s, numbers)
    {
        return TypeLattice::of_shape(shape);
    }
    // `1.5`, `1e3`, `Inf`, `-Inf`: the shared grammar's floating spellings.
    if matches!(
        parse_whole_with(s, ParseFlags::for_syntax(numbers)),
        Some(Number::Double(_) | Number::Nan { .. })
    ) {
        return TypeLattice::of(TclType::Double);
    }
    // Case-insensitive boolean check (full spellings — the literal
    // classifier's would-commit convention; prefixes commit only at use).
    if tcl_syntax::boolean::is_boolean_full_word(s) {
        return TypeLattice::of(TclType::Boolean);
    }
    TypeLattice::of(TclType::String)
}

/// Decoded contents of one unchanged original literal argument.
/// This carries no object class, header, representation, SSA value or admission.
#[doc(hidden)]
pub struct OriginalLiteralArgumentContents {
    value: String,
    pub(super) numbers: NumberSyntax,
}

impl OriginalLiteralArgumentContents {
    #[doc(hidden)]
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Existing contents-type convention under the actual source's numerals.
    /// A numeric shape is not a physical Native integer representation.
    pub(crate) fn contents_type(&self) -> TypeLattice {
        literal_type(&self.value, self.numbers)
    }
}

/// Decode only a literal written argument under its retained Module and point
/// input. Missing ownership, substitutions, opaque bytes and NUL decline.
#[doc(hidden)]
pub fn original_literal_argument_contents(
    registry: &CommandRegistry,
    module: &crate::ir::Module,
    tokens: &crate::ir::CommandTokens,
    argument: usize,
) -> Option<OriginalLiteralArgumentContents> {
    if tokens.synthetic.is_some() || !tokens.words_align_with_argv_text() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let metadata = binding.original_invocation_metadata_for_module(tokens, module, registry)?;
    let input = metadata.source_analysis_input()?;
    let value = crate::registry_invocation::original_literal_argument_data_for_module(
        registry, module, tokens, argument,
    )?;
    Some(OriginalLiteralArgumentContents {
        value,
        numbers: input.unit_profile().grammar.numbers,
    })
}

/// Classify a literal's type in **expr context**, reading numerals under
/// `numbers`.
///
/// The expr parser tokenises every integer spelling the release has — decimal,
/// hex (`0xff`), octal (`0o15`), binary (`0b1010`), `0d` decimal and
/// `_`-separated forms in 9.0 — as an integer (`Tcl_GetInt` accepts them), so
/// they all map to `Int`. This is the key divergence from the set-statement
/// [`literal_type`], where a `0o…` spelling stays `String` (its canonical
/// stringified intrep differs from the source text). A literal that is not a
/// number of this release degrades to `Numeric` — an `expr` always yields a
/// number — rather than to `String`.
#[cfg(test)]
pub(super) fn expr_literal_type(text: &str, numbers: NumberSyntax) -> TypeLattice {
    let s = text.trim();
    // Boolean first (full spellings — see `literal_type`).
    if tcl_syntax::boolean::is_boolean_full_word(s) {
        return TypeLattice::of(TclType::Boolean);
    }
    // One pass of the shared grammar covers the whole numeric tower: the
    // integer rungs (wide / bignum), doubles, and `Inf`/`NaN`. Anything the
    // release does not read as a number at all — `08` under the 8.x octal
    // rule, `0d99` before 9.0 — is `Numeric`, not a wrongly-narrowed `Double`.
    match parse_whole_with(s, ParseFlags::for_syntax(numbers)) {
        Some(Number::Int(_)) => TypeLattice::of(TclType::Int),
        Some(Number::Big { .. }) => TypeLattice::of_shape(TypeShape::Bignum),
        Some(Number::Double(_) | Number::Nan { .. }) => TypeLattice::of(TclType::Double),
        None => TypeLattice::of(TclType::Numeric),
    }
}
