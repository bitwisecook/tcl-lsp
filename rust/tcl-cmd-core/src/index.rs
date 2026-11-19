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

//! Shared Tcl index parsing (`Tcl_GetIntForIndex` / `GetEndOffsetFromObj`).
//!
//! One parser for every indexed command — `string index/range`, `lindex`,
//! `lrange`, `linsert`, `lreplace`, … — so the accepted forms and the error
//! message live once. The grammar is the release's: an integer, `end` with an
//! offset (`end-2`, `end--1` = `end - (-1)`), and from 8.5 `end+N` and the
//! sums `1+1` and `0-1`, each spelt and each integer read as that release
//! reads it (`read`).

use tcl_dialect::TclVersion;
use tcl_syntax::value::{ValueError, ValueOps};

use crate::error::CmdError;

mod compiler_encoding;
mod drill;
mod native_ops;
mod reading;

pub use compiler_encoding::{
    Encoded, IndexCompiler, StringReplaceSettlement, compiled_index_words, compiled_list_bound_in,
    compiled_list_index_in, compiler_encodable_in, compiles_apart, compiles_literals_apart, encode,
    encodes_apart, lreplace_compiles_encoded, settle_string_replace,
    string_replace_compiles_encoded,
};
pub use drill::{
    IndexPathError, IndexedKey, check_index_path, drill, drill_in_release, drill_original_jim,
    drill_prepared_c, encodable, key_argument, key_result, key_value,
};
#[cfg(test)]
use drill::{PathEncoding, failed_step_name};
pub use native_ops::{
    IndexResult, ListNumber, ListNumericKind, NativeIndexOps, NativeSortError,
    checked_operand_bytes, integer_result, native_sort_with_host, original_list_number,
    original_list_stride,
};
pub(crate) use native_ops::{
    run_created_index_result, run_index_metadata, run_index_output, run_index_result,
    run_index_result_using,
};
use reading::{Range, index_int_flags, parse_int_prefix, parse_int_whole, read};

/// Convert Jim's encoded absolute/end-relative index against this container.
#[must_use]
pub fn resolve_jim_encoding(encoded: i32, len: usize) -> i64 {
    if encoded < 0 && encoded != -i32::MAX {
        i64::try_from(len)
            .unwrap_or(i64::MAX)
            .wrapping_add(i64::from(encoded))
    } else {
        i64::from(encoded)
    }
}
use tcl_numeric::number::NumberSyntax;

/// One index word as a release reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexReading {
    /// The index the word names, negative or past the end as it falls.
    At(i64),
    /// No index: the release raises `bad index`.
    Bad,
    /// The index where the host's C `long` is 64 bits and `bad index` where
    /// it is 32 bits — 64-bit Windows and every 32-bit build.
    HostLong(i64),
    /// A word 8.5 reads as [`Self::HostLong`] and 8.6 raises on, read under
    /// the numeral grammar the two releases share with neither named.
    Unsure,
}

impl IndexReading {
    /// The index a runtime on this host reads: C Tcl's on the same host.
    #[must_use]
    pub fn on_this_host(self) -> Option<i64> {
        match self {
            Self::At(index) => Some(index),
            Self::HostLong(index) if tcl_syntax::value::host_long_is_wide() => Some(index),
            Self::HostLong(_) | Self::Bad | Self::Unsure => None,
        }
    }

    fn map(self, f: impl FnOnce(i64) -> i64) -> Self {
        match self {
            Self::At(value) => Self::At(f(value)),
            Self::HostLong(value) => Self::HostLong(f(value)),
            other => other,
        }
    }

    fn zip(self, other: Self, f: impl FnOnce(i64, i64) -> i64) -> Self {
        match (self, other) {
            (Self::Bad, _) | (_, Self::Bad) => Self::Bad,
            (Self::Unsure, _) | (_, Self::Unsure) => Self::Unsure,
            (Self::At(a), Self::At(b)) => Self::At(f(a, b)),
            (Self::At(a) | Self::HostLong(a), Self::At(b) | Self::HostLong(b)) => {
                Self::HostLong(f(a, b))
            }
        }
    }
}

/// Resolve an index `spec` against a container length `len` (so `end` is
/// `len - 1`) under the numeral grammar `numbers`, as a runtime on this host
/// reads it. The result may be negative or `>= len`; callers clamp per their
/// command's rules. Errors with the canonical message on an unparseable spec.
/// The numeral grammar 8.5 and 8.6 share reads as 8.6 here.
///
/// # Errors
/// `bad index` for a spec `numbers` does not read as an index on this host.
pub fn resolve_with(spec: &str, len: usize, numbers: NumberSyntax) -> Result<i64, CmdError> {
    read_with(spec, len, numbers)
        .on_this_host()
        .ok_or_else(|| bad_index_with(spec.trim(), numbers))
}

/// [`resolve_with`] under the grammar this thread's interpreter installed
/// (`tcl_numeric::number::runtime_syntax`). Runtime-only: a caller under a
/// stated profile — a registry folder, the compiler, a test running two
/// releases in one process — names its grammar through [`resolve_with`] or
/// [`resolve_opt_with`] instead, since the ambient is whatever the thread
/// last installed.
///
/// # Errors
/// As [`resolve_with`].
pub fn resolve(spec: &str, len: usize) -> Result<i64, CmdError> {
    resolve_with(spec, len, tcl_numeric::number::runtime_syntax())
}

/// Legacy C modern-index compatibility API; use [`resolve_opt_in`] for
/// selected native semantics.
///
/// Resolve an index `spec` against `len`, returning `None` (rather than an
/// error) on an unparseable spec — the `lsearch`/`lsort -index` driving needs the
/// raw option to classify "bad index" vs "out of range" itself. Runtime-only,
/// as [`resolve`] is.
#[must_use]
pub fn resolve_opt(spec: &str, len: usize) -> Option<i64> {
    read_with(spec, len, tcl_numeric::number::runtime_syntax()).on_this_host()
}

/// [`resolve_opt`] reading the index under an explicitly named numeral
/// grammar, for a compile-time caller whose target need not be the release
/// this process was built for: `None` also for a spec whose reading turns on
/// the host or on which release the grammar stands for, which
/// [`read_with`] tells apart.
#[must_use]
pub fn resolve_opt_with(spec: &str, len: usize, numbers: NumberSyntax) -> Option<i64> {
    match read_with(spec, len, numbers) {
        IndexReading::At(index) => Some(index),
        IndexReading::Bad | IndexReading::HostLong(_) | IndexReading::Unsure => None,
    }
}

/// One index `spec` against `len` as the numeral grammar `numbers` reads it;
/// the grammar 8.5 and 8.6 share names neither release.
#[must_use]
pub fn read_with(spec: &str, len: usize, numbers: NumberSyntax) -> IndexReading {
    read(spec, len, numbers, Range::of_numbers(numbers))
}

/// One index `spec` against `len` as `release` reads it.
#[must_use]
pub fn read_under(spec: &str, len: usize, release: TclVersion) -> IndexReading {
    read(
        spec,
        len,
        release.number_syntax(),
        Range::of_release(release),
    )
}

/// Resolve an index with the complete selected native policy.
///
/// # Errors
/// A dialect-specific bad-index error when the index is invalid.
pub fn resolve_in(
    spec: &str,
    len: usize,
    syntax: tcl_dialect::IndexSyntax,
) -> Result<i64, CmdError> {
    resolve_opt_in(spec, len, syntax).ok_or_else(|| bad_index_in(spec, syntax))
}

/// Parse an index without discarding its native index grammar.
#[must_use]
pub fn resolve_opt_in(spec: &str, len: usize, syntax: tcl_dialect::IndexSyntax) -> Option<i64> {
    use tcl_dialect::IndexGrammar;
    let len = i64::try_from(len).ok()?;
    if syntax.grammar == IndexGrammar::Jim {
        let (expression, end_relative) = jim_expression(spec)?;
        let grammar = tcl_dialect::LexerGrammar {
            numbers: syntax.numbers,
            expr_comments: tcl_dialect::ExprCommentStyle::None,
            // dialect-drift-ok: Jim's native index-expression grammar, whose
            // numbers come from the selected IndexSyntax, not document text
            ..tcl_dialect::LexerGrammar::default()
        };
        let tree = tcl_syntax::expr::parser::parse_expr_with_grammar(expression, &grammar);
        let value = tcl_syntax::expr::wide::literal_expression(
            &tree,
            syntax.numbers,
            tcl_dialect::NativeArithmetic::JimWide,
        )
        .ok()?;
        return finish_jim_index(value, end_relative, len);
    }
    if let Some(value) = parse_int_whole(spec, syntax.numbers) {
        return narrow_tcl_integer(value, syntax);
    }
    let (base, rest) = if spec == "end"
        || (syntax.end_abbreviations
            && !spec.is_empty()
            && spec.len() < 3
            && "end".starts_with(spec))
    {
        return Some(len - 1);
    } else if let Some(rest) = spec.strip_prefix("end") {
        (len - 1, rest)
    } else {
        if syntax.grammar == IndexGrammar::Tcl84 {
            return None;
        }
        let (base, rest) = parse_int_prefix(spec.trim_start(), syntax.numbers)?;
        (narrow_tcl_integer(base, syntax)?, rest)
    };
    let connector = *rest.as_bytes().first()?;
    if connector != b'-' && (connector != b'+' || syntax.grammar == IndexGrammar::Tcl84) {
        return None;
    }
    let operand_text = rest.get(1..)?;
    if syntax.grammar != IndexGrammar::Tcl84 && operand_text.chars().next()?.is_whitespace() {
        return None;
    }
    let operand = narrow_tcl_integer(parse_int_whole(operand_text, syntax.numbers)?, syntax)?;
    let value = match (syntax.width, connector) {
        (tcl_dialect::IndexIntegerWidth::Tcl64, b'-') => base.saturating_sub(operand),
        (tcl_dialect::IndexIntegerWidth::Tcl64, _) => base.saturating_add(operand),
        (_, b'-') => base.wrapping_sub(operand),
        _ => base.wrapping_add(operand),
    };
    Some(if syntax.width == tcl_dialect::IndexIntegerWidth::Tcl64 {
        value
    } else {
        i64::from(tcl_numeric::number::native_int32_low_bits(value))
    })
}

fn narrow_tcl_integer(value: i64, syntax: tcl_dialect::IndexSyntax) -> Option<i64> {
    if syntax.width == tcl_dialect::IndexIntegerWidth::Tcl64 {
        Some(value)
    } else if (-i64::from(u32::MAX)..=i64::from(u32::MAX)).contains(&value) {
        Some(i64::from(tcl_numeric::number::native_int32_low_bits(value)))
    } else {
        None
    }
}

/// Extract Jim's safe integer expression, retaining `end` relativity.
#[must_use]
pub fn jim_expression(spec: &str) -> Option<(&str, bool)> {
    if let Some(rest) = spec.strip_prefix("end") {
        if rest.is_empty() {
            return Some(("0", true));
        }
        matches!(rest.as_bytes().first(), Some(b'+' | b'-')).then_some((rest, true))
    } else {
        Some((spec, false))
    }
}

/// Convert Jim's evaluated wide integer to its encoded native index.
#[must_use]
pub fn finish_jim_index(value: i64, end_relative: bool, length: i64) -> Option<i64> {
    if end_relative {
        if value > 0 {
            return Some(i64::from(i32::MAX));
        }
        let encoded = tcl_numeric::number::native_int32_low_bits(value.wrapping_sub(1));
        Some(length.wrapping_add(i64::from(encoded)))
    } else if value < 0 {
        Some(-i64::from(i32::MAX))
    } else {
        (value <= i64::from(i32::MAX)).then_some(value)
    }
}

/// Resolve through a runtime value adapter, including Jim's safe expressions.
///
/// # Errors
/// A bad-index error, or an unselected native policy.
pub fn resolve_for_ops<O: NativeIndexOps>(
    ops: &mut O,
    spec: &str,
    len: usize,
) -> Result<i64, CmdError> {
    resolve_for_ops_with(ops, spec, len, None)
}

/// [`resolve_for_ops`] reading a Tcl (non-Jim) index's integers under
/// `numbers` where the caller names the grammar (a runtime its pinned
/// release's, a folder the target's) rather than the ops' own.
///
/// # Errors
/// As [`resolve_for_ops`].
pub fn resolve_for_ops_with<O: NativeIndexOps>(
    ops: &mut O,
    spec: &str,
    len: usize,
    numbers: Option<NumberSyntax>,
) -> Result<i64, CmdError> {
    run_index_metadata(ops, |ops| {
        let mut syntax = ops
            .index_syntax()
            .ok_or_else(|| CmdError::new("container index dialect is not selected"))?;
        if syntax.grammar != tcl_dialect::IndexGrammar::Jim
            && let Some(numbers) = numbers
        {
            syntax.numbers = numbers;
            // A caller naming 8.4's numerals reads 8.4's index grammar too
            // (`integer or end?-integer?`, its sole bad-index wording).
            if numbers == NumberSyntax::Tcl84 {
                syntax.grammar = tcl_dialect::IndexGrammar::Tcl84;
            }
        }
        if syntax.grammar != tcl_dialect::IndexGrammar::Jim {
            if let Some(value) = resolve_opt_in(spec, len, syntax) {
                return Ok(value);
            }
            if syntax.width == tcl_dialect::IndexIntegerWidth::Tcl64
                && let Some((expression, connector)) = wide_expression(spec, syntax)
            {
                let value = ops.eval_index_expression(expression).map_err(|error| {
                    if error.native_access_refusal().is_some() {
                        CmdError::from(error)
                    } else {
                        bad_index_for_ops(ops, spec, syntax)
                    }
                })?;
                let end = i64::try_from(len).unwrap_or(i64::MAX) - 1;
                return Ok(match connector {
                    Some(b'-') => end.saturating_sub(value),
                    Some(_) => end.saturating_add(value),
                    None => value,
                });
            }
            return Err(bad_index_for_ops(ops, spec, syntax));
        }
        if let Some(encoded) = ops.textual_jim_index(spec.as_bytes())? {
            return Ok(resolve_jim_encoding(encoded, len));
        }
        let (expression, relative) =
            jim_expression(spec).ok_or_else(|| bad_index_for_ops(ops, spec, syntax))?;
        let value = ops.eval_index_expression(expression).map_err(|error| {
            if error.native_access_refusal().is_some() {
                CmdError::from(error)
            } else {
                bad_index_for_ops(ops, spec, syntax)
            }
        })?;
        finish_jim_index(value, relative, i64::try_from(len).unwrap_or(i64::MAX))
            .ok_or_else(|| bad_index_for_ops(ops, spec, syntax))
    })
}

pub(crate) fn wide_expression(
    spec: &str,
    syntax: tcl_dialect::IndexSyntax,
) -> Option<(&str, Option<u8>)> {
    let flags = index_int_flags(syntax.numbers);
    let whole_integer = |text: &str| tcl_numeric::number::parse_whole_with(text, flags).is_some();
    if let Some(rest) = spec.strip_prefix("end") {
        let connector = *rest.as_bytes().first()?;
        let operand = rest.get(1..)?;
        if !matches!(connector, b'+' | b'-')
            || operand.chars().next()?.is_whitespace()
            || !whole_integer(operand)
        {
            return None;
        }
        return Some((operand, Some(connector)));
    }
    if whole_integer(spec) {
        return Some((spec, None));
    }
    let trimmed = spec.trim_start();
    let first = tcl_numeric::number::parse(trimmed, flags)?;
    let rest = trimmed.get(first.end..)?;
    let connector = *rest.as_bytes().first()?;
    let operand = rest.get(1..)?;
    (matches!(connector, b'+' | b'-')
        && !operand.chars().next()?.is_whitespace()
        && whole_integer(operand))
    .then_some((spec, None))
}

/// Resolve one runtime value under the selected index policy.
///
/// # Errors
/// The same errors as [`resolve_for_ops`].
pub fn resolve_value_with<O: NativeIndexOps>(
    ops: &mut O,
    value: &O::Value,
    len: usize,
    numbers: NumberSyntax,
) -> Result<i64, CmdError> {
    run_index_metadata(ops, |ops| {
        if ops
            .index_syntax()
            .is_some_and(|syntax| syntax.grammar == tcl_dialect::IndexGrammar::Jim)
            && let Some(encoded) = ops.original_jim_index(value)?
        {
            return Ok(resolve_jim_encoding(encoded, len));
        }
        if ops.index_c_version().is_some()
            && let Some(resolved) = ops.original_c_index(value, len)?
        {
            return Ok(resolved);
        }
        let spelling = ops.index_unicode(value)?;
        resolve_for_ops_with(ops, &spelling, len, Some(numbers))
    })
}

/// [`resolve_value_with`] under the numeral grammar the selected ops carry.
///
/// # Errors
/// As [`resolve_for_ops`].
pub fn resolve_value<O: NativeIndexOps>(
    ops: &mut O,
    value: &O::Value,
    len: usize,
) -> Result<i64, CmdError> {
    run_index_metadata(ops, |ops| {
        if ops
            .index_syntax()
            .is_some_and(|syntax| syntax.grammar == tcl_dialect::IndexGrammar::Jim)
            && let Some(encoded) = ops.original_jim_index(value)?
        {
            return Ok(resolve_jim_encoding(encoded, len));
        }
        if ops.index_c_version().is_some()
            && let Some(resolved) = ops.original_c_index(value, len)?
        {
            return Ok(resolved);
        }
        let spelling = ops.index_unicode(value)?;
        resolve_for_ops(ops, &spelling, len)
    })
}

/// Prepare a complete Jim original index vector before container conversion.
/// `None` describes only neutral adapters. A mixed physical/neutral vector is
/// unavailable rather than evaluating a prefix twice under a new receipt.
///
/// # Errors
/// The first reached original-index failure or an inconsistent adapter.
pub fn prepare_jim_indices<O: NativeIndexOps>(
    ops: &mut O,
    originals: &[&O::Value],
) -> Result<Option<Vec<i32>>, CmdError> {
    run_index_metadata(ops, |ops| {
        if !ops
            .index_syntax()
            .is_some_and(|syntax| syntax.grammar == tcl_dialect::IndexGrammar::Jim)
        {
            return Ok(None);
        }
        let mut indices = Vec::with_capacity(originals.len());
        let mut neutral = false;
        for original in originals {
            match ops.original_jim_index(original)? {
                Some(index) if !neutral => indices.push(index),
                None if indices.is_empty() => neutral = true,
                _ => {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "mixed original Jim Index adapter",
                    )
                    .into());
                }
            }
        }
        Ok((!neutral).then_some(indices))
    })
}

/// Validate whether a selected native index can be encoded for a search path.
///
/// # Errors
/// Bad index or the actual first native operation/getter failure.
pub fn encodable_for_ops<O: NativeIndexOps>(ops: &mut O, spec: &str) -> Result<bool, CmdError> {
    run_index_metadata(ops, |ops| {
        let syntax = ops.index_syntax().ok_or_else(|| {
            CmdError::from(ValueError::CommandProtocolUnavailable(
                "container index dialect",
            ))
        })?;
        let value = resolve_for_ops(ops, spec, 1)?;
        let end_relative = match syntax.grammar {
            tcl_dialect::IndexGrammar::Jim => spec.starts_with("end"),
            _ => spec.starts_with("end") || (!spec.is_empty() && "end".starts_with(spec)),
        };
        Ok(if end_relative { value <= 0 } else { value >= 0 })
    })
}

/// Native bad-index message, preserving the authored spelling.
#[must_use]
pub fn bad_index_in(spec: impl AsRef<[u8]>, syntax: tcl_dialect::IndexSyntax) -> CmdError {
    let grammar = match syntax.grammar {
        tcl_dialect::IndexGrammar::Tcl84 => "integer or end?-integer?",
        tcl_dialect::IndexGrammar::TclModern => "integer?[+-]integer? or end?[+-]integer?",
        tcl_dialect::IndexGrammar::Jim => "intexpr or end?[+-]intexpr?",
    };
    let mut message = b"bad index \"".to_vec();
    message.extend_from_slice(spec.as_ref());
    message.extend_from_slice(b"\": must be ");
    message.extend_from_slice(grammar.as_bytes());
    CmdError::new_bytes(message)
}

/// Publish the reached index diagnostic with its independently selected
/// physical String producer. Unavailable physical issuers remain host refusals.
#[must_use]
pub fn bad_index_for_ops<O: ValueOps>(
    ops: &O,
    spec: impl AsRef<[u8]>,
    syntax: tcl_dialect::IndexSyntax,
) -> CmdError {
    let diagnostic = if syntax.grammar == tcl_dialect::IndexGrammar::Jim {
        spec.as_ref()
    } else {
        // C Tcl's native bad-index diagnostic consumes Tcl_GetString through
        // its original %s field; counted parser extent remains independent.
        // naming.string.original-counted-replacement-and-index-stages
        // docs/design/analysis/name-resolution-proofs/string-original-counted-replacement-and-index-stages.md
        tcl_core_types::c_string_extent(spec.as_ref())
    };
    let error = bad_index_in(diagnostic, syntax);
    match ops.index_error_string_protocol() {
        Ok(Some(protocol)) => {
            let mut details = error.into_byte_details();
            let mut string_result = true;
            if let Some(version) = protocol.tcl_version() {
                if tcl_numeric::number::native_index_octal_hint(spec.as_ref(), version) {
                    details
                        .message
                        .extend_from_slice(b" (looks like invalid octal number)");
                    // C84 Tcl_AppendResult reaches the interpreter CString
                    // result, later materialised as a fresh NULL-primary object.
                    string_result = version != TclVersion::V8_4;
                }
                if version >= TclVersion::V8_6 {
                    details.error_code =
                        crate::error::CmdErrorCodeUpdate::Set(b"TCL VALUE INDEX".to_vec());
                }
            }
            let error = CmdError::from_byte_details(details);
            if string_result {
                error.with_native_string_result(protocol)
            } else {
                error
            }
        }
        Ok(None) => error,
        Err(refusal) => refusal.into(),
    }
}

/// The canonical `bad index "<spec>": …` error, as Tcl 8.6 and later raise it
/// ([`bad_index_under`]).
#[must_use]
pub fn bad_index(spec: &str) -> CmdError {
    bad_index_under(spec, TclVersion::V8_6)
}

/// The `bad index` error for `spec` a caller reading it under `numbers`
/// raises: 8.4's wording under its grammar, 8.6's otherwise
/// ([`bad_index_under`]). The grammar does not tell 8.5 from 8.6, so 8.6's
/// `TCL VALUE INDEX` error code is left for an 8.5 runtime to drop.
#[must_use]
pub fn bad_index_with(spec: &str, numbers: NumberSyntax) -> CmdError {
    if numbers == NumberSyntax::Tcl84 {
        bad_index_under(spec, TclVersion::V8_4)
    } else {
        bad_index(spec)
    }
}

/// [`bad_index_with`] under the grammar this thread's interpreter installed,
/// as a message and the code it carries, for an option word read under it
/// (`lsearch -start`, an `-index` step).
#[must_use]
pub fn bad_index_parts(spec: &[u8]) -> (Vec<u8>, Option<&'static [u8]>) {
    let details = bad_index_with(
        &String::from_utf8_lossy(spec),
        tcl_numeric::number::runtime_syntax(),
    )
    .into_byte_details();
    let coded = matches!(details.error_code, crate::error::CmdErrorCodeUpdate::Set(_));
    (details.message, coded.then_some(&b"TCL VALUE INDEX"[..]))
}

/// The `bad index` error `release` raises for `spec`: 8.4 words the grammar
/// it reads (`integer or end?-integer?`), 8.5 on the sums too, and from 8.6
/// the error carries `TCL VALUE INDEX` (measured, 8.4.20 to 9.1.0).
#[must_use]
pub fn bad_index_under(spec: &str, release: TclVersion) -> CmdError {
    if release < TclVersion::V8_5 {
        return CmdError::new(format!(
            "bad index \"{spec}\": must be integer or end?-integer?"
        ));
    }
    let message = format!("bad index \"{spec}\": must be integer?[+-]integer? or end?[+-]integer?");
    if release < TclVersion::V8_6 {
        CmdError::new(message)
    } else {
        CmdError::with_error_code(message, "TCL VALUE INDEX")
    }
}

#[cfg(test)]
mod tests;
#[test]
fn compiler_encoding_preserves_wide_target_uncertainty() {
    use tcl_dialect::{IndexSyntax, TclVersion};
    for version in TclVersion::ALL {
        let syntax = IndexSyntax::for_version(version);
        for index in ["0", "end", "end-2", "0x10", "-1"] {
            assert_eq!(compiler_encodable_in(index, syntax), Some(true));
        }
        if version >= TclVersion::V9_0 {
            assert_eq!(compiler_encodable_in("2147483648", syntax), None);
            assert_eq!(compiler_encodable_in("end-2147483648", syntax), None);
        }
    }
}

/// Measured with `tcl::unsupported::disassemble` on tclsh 9.0.4 and 9.1.0.
#[test]
fn c9_index_encoding_follows_the_64_bit_host() {
    use tcl_dialect::TclVersion;
    let bound = |spec, before, after| {
        compiled_list_bound_in(spec, TclVersion::V9_0, before, after)
            .unwrap()
            .map(tcl_syntax::native_compiled_index::NativeCompiledListIndex::encoded)
    };
    // `string range` last-index coordinates: before NONE, after END.
    for (spec, encoded) in [
        ("end", Some(-2)),
        ("end-0", Some(-2)),
        ("end+5", Some(-2)),
        ("end-2147483646", Some(-2_147_483_648)),
        ("end-1073741825", Some(-1_073_741_827)),
        ("end-2147483647", None),
        ("end-2147483649", None),
        ("end-9999999999999999999999", None),
        ("end-2000000000000000000", Some(-1)),
        ("end+1", None),
        ("99999999999999999999999", Some(-2)),
        ("9223372036854775806", Some(-2)),
        ("9223372036854775805", None),
        ("2147483648", None),
        ("2147483647", Some(2_147_483_647)),
        ("-5", Some(-1)),
        ("0x10", Some(16)),
        ("5-1", Some(4)),
        ("1-2", None),
        ("1-3", Some(-1)),
        ("bad", None),
    ] {
        assert_eq!(bound(spec, -1, -2), encoded, "{spec}");
    }
}
