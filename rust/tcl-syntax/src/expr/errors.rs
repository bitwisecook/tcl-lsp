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

//! `expr`'s **operand-type** and **boolean-context** error surface — C's
//! `IllegalExprOperandType` (`tclExecute.c`) and the boolean coercion beside
//! it — written once so both engines emit the same bytes.
//!
//! Two axes live here, and only one of them is release-dependent:
//!
//! * The **message wording is a release axis.** Tcl 9.0 names the offending
//!   value and which side of the operator it sat on
//!   (`cannot use non-numeric string "abc" as left operand of "+"`); Tcl
//!   8.4-8.6 name neither (`can't use non-numeric string as operand of "+"`),
//!   and have no separate "a list" branch at all — a multi-element list is
//!   just a non-numeric string there. Both engines used to emit the 9.0 form
//!   at every `--tcl-version` (issue #1581).
//! * The **`-errorcode`** is `ARITH DOMAIN {<description>}` from 8.5, with
//!   `list` as the description for 9.0's list branch; 8.4 sets none but for
//!   a formal integer it could not hold.
//! * The **description** of a value that is no number is a release axis too:
//!   8.4-8.6 single out an empty string and a numeral with a leading zero and
//!   a digit octal lacks (`TclCheckBadOctal`), and 8.4 a formal integer past
//!   the wide range.
//!
//! Measured against tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0.

use tcl_dialect::TclVersion;

/// How C describes an operand it cannot use. The same word appears in the
/// message and as the detail element of the `ARITH DOMAIN` error code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperandDesc {
    /// A string that is not a number at all.
    NonNumericString,
    /// An empty string, before 9.0.
    EmptyString,
    /// A numeral with a leading zero and a digit octal lacks, before 9.0.
    InvalidOctalNumber,
    /// 8.4 only: a formal integer past the wide range.
    IntegerTooLarge,
    /// A double handed to an integer-only operator (`~`, `&`, `%`, `<<`, …).
    FloatingPointValue,
    /// A NaN — numeric in shape, unusable as an operand.
    NonNumericFloatingPointValue,
    /// Tcl 9.0 only: a well-formed list of more than one element. Tcl 8.6
    /// has no such branch and reports [`Self::NonNumericString`] instead.
    List,
}

impl OperandDesc {
    /// The description as it appears inside the message (and, except for
    /// [`Self::List`], inside the error code).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            OperandDesc::NonNumericString => "non-numeric string",
            OperandDesc::EmptyString => "empty string",
            OperandDesc::InvalidOctalNumber => "invalid octal number",
            OperandDesc::IntegerTooLarge => IOVERFLOW_MESSAGE,
            OperandDesc::FloatingPointValue => "floating-point value",
            OperandDesc::NonNumericFloatingPointValue => "non-numeric floating-point value",
            OperandDesc::List => "a list",
        }
    }

    /// The detail element C puts in `-errorcode ARITH DOMAIN <detail>` — the
    /// description itself, except that the list branch is the bare word
    /// `list` (tclsh 9.0.4: `ARITH DOMAIN list`).
    #[must_use]
    pub fn error_code_detail(self) -> &'static str {
        match self {
            OperandDesc::List => "list",
            other => other.as_str(),
        }
    }

    /// The description this release actually uses: before 9.0 there is no
    /// list branch, so a multi-element list is a non-numeric string.
    #[must_use]
    pub fn for_release(self, release: TclVersion) -> Self {
        if self == OperandDesc::List && release < TclVersion::V9_0 {
            OperandDesc::NonNumericString
        } else {
            self
        }
    }

    /// The description `release` gives `value`, which this describes: before
    /// 9.0 a non-numeric string that is empty or a bad octal numeral is
    /// described as such, and 8.4 describes a formal integer it could not
    /// hold as too large.
    #[must_use]
    pub fn of_value(self, value: &str, release: TclVersion) -> Self {
        let desc = self.for_release(release);
        if desc != OperandDesc::NonNumericString || release >= TclVersion::V9_0 {
            return desc;
        }
        if value.is_empty() {
            return OperandDesc::EmptyString;
        }
        let octal_prefix = release > TclVersion::V8_4;
        if release == TclVersion::V8_4 && !looks_like_integer_84(value) {
            OperandDesc::NonNumericString
        } else if check_bad_octal(value, octal_prefix) {
            OperandDesc::InvalidOctalNumber
        } else if release == TclVersion::V8_4 {
            OperandDesc::IntegerTooLarge
        } else {
            OperandDesc::NonNumericString
        }
    }
}

/// C's `TclCheckBadOctal`: whether `value` is a numeral with a leading zero
/// (after any space and sign, and from 8.5 an `0o` prefix) whose digits run
/// to its end, the shape a leading-zero octal with an 8 or 9 takes.
#[must_use]
pub fn check_bad_octal(value: &str, octal_prefix: bool) -> bool {
    let rest = value.trim_start_matches(is_c_space);
    let rest = rest.strip_prefix(['+', '-']).unwrap_or(rest);
    let Some(rest) = rest.strip_prefix('0') else {
        return false;
    };
    let rest = if octal_prefix {
        rest.strip_prefix(['o', 'O']).unwrap_or(rest)
    } else {
        rest
    };
    rest.trim_start_matches(|c: char| c.is_ascii_digit())
        .trim_start_matches(is_c_space)
        .is_empty()
}

/// 8.4's `IllegalExprOperandType` test for a value that looks formally like
/// an integer ([`tcl_dialect::tcl84_formal_integer_end`]): after any space
/// and sign, the integer's text, then only space.
fn looks_like_integer_84(value: &str) -> bool {
    let rest = value.trim_start_matches(is_c_space);
    let rest = rest.strip_prefix(['+', '-']).unwrap_or(rest);
    if rest.is_empty() {
        return false;
    }
    let end = tcl_dialect::tcl84_formal_integer_end(rest.as_bytes(), 0);
    rest[end..].trim_start_matches(is_c_space).is_empty()
}

/// C's `isspace` in the "C" locale.
fn is_c_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\x0b' | '\x0c' | '\r')
}

/// Which operand of the operator was at fault. Tcl 9.0 names it; earlier
/// releases do not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperandSide {
    /// A unary operator's only operand.
    Unary,
    /// A binary operator's left operand.
    Left,
    /// A binary operator's right operand.
    Right,
}

impl OperandSide {
    /// The 9.0 qualifier written before `operand` (`""`, `"left "`,
    /// `"right "`).
    #[must_use]
    pub fn prefix(self) -> &'static str {
        match self {
            OperandSide::Unary => "",
            OperandSide::Left => "left ",
            OperandSide::Right => "right ",
        }
    }
}

/// The wording release of the **ambient** runtime — the release the engine
/// installed with [`crate::number::set_runtime_syntax`], which is the same
/// ambient the numeric grammar already follows.
///
/// `expr`'s operand errors are raised deep inside operator application
/// (`tcl_vm::expr::arith` / `unary`, reached from bytecode opcodes that carry
/// no interpreter), so threading a release through every call site would mean
/// re-plumbing the opcode surface. The numeric grammar solved the same
/// problem the same way; this reuses that single ambient rather than adding a
/// second one. A caller that *does* hold a release passes it explicitly to
/// [`illegal_operand_message`].
#[must_use]
// The Jim arms answer 9.0 for a different reason than the `Tcl90` arm does
// (a backend decision, not a release identity), and each carries its own
// evidence; folding them together would lose that.
#[allow(clippy::match_same_arms)]
pub fn ambient_release() -> TclVersion {
    match crate::number::runtime_syntax() {
        tcl_dialect::NumberSyntax::Tcl90 => TclVersion::V9_0,
        tcl_dialect::NumberSyntax::Tcl85 => TclVersion::V8_6,
        tcl_dialect::NumberSyntax::Tcl84 => TclVersion::V8_4,
        // JimTcl names no C release: it is a `Lineage::Reimplementation`,
        // so `DialectPoint::tcl_version_of_release` answers `None` for it
        // and a projected `jim` profile carries `runtime_base: None`. What
        // it does carry is a `vm_runtime_version`, and the model settles
        // that at Tcl 9.0 — "a Jim unit is read as Jim into codegen and
        // executed as Tcl 9" (`dialect-profile-model.md` §2.5). The wording
        // of a runtime error is the executing engine's, not the source
        // grammar's, so a Jim ambient takes the release its backend runs
        // as. (Unreachable through the VM today, which installs this
        // ambient from `TclVersion::number_syntax` — but
        // `set_runtime_syntax` is public and this match must be total.)
        tcl_dialect::NumberSyntax::Jim | tcl_dialect::NumberSyntax::Jim080 => TclVersion::V9_0,
    }
}

/// C's `IllegalExprOperandType` message for `release`.
///
/// * 9.0+: `cannot use non-numeric string "abc" as left operand of "+"`,
///   with the list branch phrased `cannot use a list as left operand of "+"`
///   (no value, no quotes).
/// * 8.4-8.6: `can't use non-numeric string as operand of "+"` — no value and
///   no side, and no list branch.
#[must_use]
pub fn illegal_operand_message(
    desc: OperandDesc,
    value: &str,
    side: OperandSide,
    op: &str,
    release: TclVersion,
) -> String {
    let desc = desc.of_value(value, release);
    if release >= TclVersion::V9_0 {
        if desc == OperandDesc::List {
            format!("cannot use a list as {}operand of \"{op}\"", side.prefix())
        } else {
            format!(
                "cannot use {} \"{value}\" as {}operand of \"{op}\"",
                desc.as_str(),
                side.prefix()
            )
        }
    } else {
        format!("can't use {} as operand of \"{op}\"", desc.as_str())
    }
}

/// The `-errorcode` C stamps on an operand-type error for `value`:
/// `ARITH DOMAIN <detail>` from 8.5, and in 8.4 `NONE` but for a formal
/// integer too large, `ARITH IOVERFLOW`.
#[must_use]
pub fn illegal_operand_error_code(desc: OperandDesc, value: &str, release: TclVersion) -> String {
    let desc = desc.of_value(value, release);
    if desc == OperandDesc::IntegerTooLarge {
        return IOVERFLOW_CODE.to_owned();
    }
    if release == TclVersion::V8_4 {
        return "NONE".to_owned();
    }
    let detail = desc.error_code_detail();
    if detail.contains(' ') {
        format!("ARITH DOMAIN {{{detail}}}")
    } else {
        format!("ARITH DOMAIN {detail}")
    }
}

/// The `-errorcode` for `expected boolean value but got "…"` — a value that
/// is neither a number nor a Tcl boolean word in a boolean context
/// (`&&`, `||`, `!`, `?:`, `bool()`). tclsh 8.6.16/9.0.4: `TCL VALUE NUMBER`.
pub const BOOLEAN_OPERAND_CODE: &str = "TCL VALUE NUMBER";

/// The message and `-errorcode` for a NaN reaching a context that needs a
/// real number — a boolean test, or an integer conversion.
/// tclsh 8.6.16/9.0.4: `floating point value is Not a Number`,
/// `-errorcode TCL VALUE DOUBLE NAN`.
pub const NAN_MESSAGE: &str = "floating point value is Not a Number";
/// See [`NAN_MESSAGE`].
pub const NAN_CODE: &str = "TCL VALUE DOUBLE NAN";

/// The message and `-errorcode` for an infinity reaching an integer
/// conversion (`entier`/`int`/`wide`/`round`/`isqrt`). tclsh 8.6.16/9.0.4:
/// `integer value too large to represent`, `-errorcode ARITH IOVERFLOW
/// {integer value too large to represent}`.
pub const IOVERFLOW_MESSAGE: &str = "integer value too large to represent";
/// See [`IOVERFLOW_MESSAGE`].
pub const IOVERFLOW_CODE: &str = "ARITH IOVERFLOW {integer value too large to represent}";

/// The generic math-function domain error (`sqrt(-1)`, `fmod(x, 0)`,
/// `log(-1)`). tclsh 8.6.16/9.0.4: `domain error: argument not in valid
/// range`, `-errorcode ARITH DOMAIN {domain error: argument not in valid
/// range}`.
pub const DOMAIN_MESSAGE: &str = "domain error: argument not in valid range";
/// See [`DOMAIN_MESSAGE`]. `isqrt` of a negative operand keeps its own
/// message but reuses this code (tclsh verified).
pub const DOMAIN_CODE: &str = "ARITH DOMAIN {domain error: argument not in valid range}";

/// The message and `-errorcode` for a double past the double range, which
/// 8.4 refuses wherever one arises (`TclExprFloatError`).
pub const FLOAT_OVERFLOW_MESSAGE: &str = "floating-point value too large to represent";
/// See [`FLOAT_OVERFLOW_MESSAGE`].
pub const FLOAT_OVERFLOW_CODE: &str =
    "ARITH OVERFLOW {floating-point value too large to represent}";
/// The message and `-errorcode` for a numeral 8.4 reads as a non-zero double
/// that rounds to zero.
pub const FLOAT_UNDERFLOW_MESSAGE: &str = "floating-point value too small to represent";
/// See [`FLOAT_UNDERFLOW_MESSAGE`].
pub const FLOAT_UNDERFLOW_CODE: &str =
    "ARITH UNDERFLOW {floating-point value too small to represent}";

/// The message and `-errorcode` for a division by zero.
pub const DIVZERO_MESSAGE: &str = "divide by zero";
/// See [`DIVZERO_MESSAGE`].
pub const DIVZERO_CODE: &str = "ARITH DIVZERO {divide by zero}";

/// The error `release` raises for `value`, a double an operator, a math
/// function or an operand's conversion produced: 8.4 refuses any that is not
/// finite (`TclExprFloatError`), a NaN as a domain error and an infinity as
/// too large, where later releases keep it. `None` where it is kept.
#[must_use]
pub fn float_result_error(value: f64, release: TclVersion) -> Option<(&'static str, &'static str)> {
    if !checks_float_range(release) || value.is_finite() {
        None
    } else if value.is_nan() {
        Some((DOMAIN_MESSAGE, DOMAIN_CODE))
    } else {
        Some(float_range_error(value))
    }
}

/// Whether `release` checks each double an operator, a math function or a
/// numeral produces (`TclExprFloatError`): 8.4 does, later releases keep an
/// infinity, a NaN or an underflow.
#[must_use]
pub fn checks_float_range(release: TclVersion) -> bool {
    release == TclVersion::V8_4
}

/// The message and `-errorcode` 8.4 gives a double C reported with `ERANGE`
/// (`TclExprFloatError`): too small where it came back zero, too large
/// otherwise, a subnormal included (tclsh 8.4.20: `expr {1e-320}` is
/// `floating-point value too large to represent`).
#[must_use]
pub fn float_range_error(value: f64) -> (&'static str, &'static str) {
    if value == 0.0 {
        (FLOAT_UNDERFLOW_MESSAGE, FLOAT_UNDERFLOW_CODE)
    } else {
        (FLOAT_OVERFLOW_MESSAGE, FLOAT_OVERFLOW_CODE)
    }
}

/// Whether `release` refuses a double divided by zero as it does an integer
/// one: 8.4 does, where later releases give an infinity or a NaN.
#[must_use]
pub fn refuses_double_division_by_zero(release: TclVersion) -> bool {
    release == TclVersion::V8_4
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every numeral grammar answers a wording release, and each answers
    /// the release whose engine actually raises the error: the C grammars
    /// their own, `JimTcl` the Tcl 9.0 backend the model executes it as.
    #[test]
    fn every_numeral_grammar_names_its_wording_release() {
        use tcl_dialect::NumberSyntax;
        let restore = crate::number::runtime_syntax();
        for (syntax, expected) in [
            (NumberSyntax::Tcl84, TclVersion::V8_4),
            (NumberSyntax::Tcl85, TclVersion::V8_6),
            (NumberSyntax::Tcl90, TclVersion::V9_0),
            (NumberSyntax::Jim, TclVersion::V9_0),
            (NumberSyntax::Jim080, TclVersion::V9_0),
        ] {
            crate::number::set_runtime_syntax(syntax);
            assert_eq!(ambient_release(), expected, "{syntax:?}");
        }
        // `NumberSyntax::ALL` is the list a new grammar joins; this fails
        // the moment one lands without a row above.
        assert_eq!(NumberSyntax::ALL.len(), 5);
        crate::number::set_runtime_syntax(restore);
    }

    /// Every row measured on tclsh 9.0.4 and tclsh 8.6.16 with
    /// `catch {expr {...}} m o; list $m [dict get $o -errorcode]`.
    #[test]
    fn the_wording_axis_matches_both_releases() {
        use OperandDesc::{
            FloatingPointValue, List, NonNumericFloatingPointValue, NonNumericString,
        };
        use OperandSide::{Left, Right, Unary};
        let v90 = TclVersion::V9_0;
        let v86 = TclVersion::V8_6;

        // expr {!"abc"}
        assert_eq!(
            illegal_operand_message(NonNumericString, "abc", Unary, "!", v90),
            "cannot use non-numeric string \"abc\" as operand of \"!\""
        );
        assert_eq!(
            illegal_operand_message(NonNumericString, "abc", Unary, "!", v86),
            "can't use non-numeric string as operand of \"!\""
        );
        // expr {"abc" + 1} / expr {1 + "abc"}
        assert_eq!(
            illegal_operand_message(NonNumericString, "abc", Left, "+", v90),
            "cannot use non-numeric string \"abc\" as left operand of \"+\""
        );
        assert_eq!(
            illegal_operand_message(NonNumericString, "abc", Right, "+", v90),
            "cannot use non-numeric string \"abc\" as right operand of \"+\""
        );
        // 8.6 names neither the value nor the side, so both sides read alike.
        assert_eq!(
            illegal_operand_message(NonNumericString, "abc", Right, "+", v86),
            "can't use non-numeric string as operand of \"+\""
        );
        // expr {~1.5}
        assert_eq!(
            illegal_operand_message(FloatingPointValue, "1.5", Unary, "~", v90),
            "cannot use floating-point value \"1.5\" as operand of \"~\""
        );
        assert_eq!(
            illegal_operand_message(FloatingPointValue, "1.5", Unary, "~", v86),
            "can't use floating-point value as operand of \"~\""
        );
        // expr {NaN + 1}
        assert_eq!(
            illegal_operand_message(NonNumericFloatingPointValue, "NaN", Left, "+", v90),
            "cannot use non-numeric floating-point value \"NaN\" as left operand of \"+\""
        );
        assert_eq!(
            illegal_operand_message(NonNumericFloatingPointValue, "NaN", Left, "+", v86),
            "can't use non-numeric floating-point value as operand of \"+\""
        );
        // expr {"a b" + 1}: 9.0 has a list branch, 8.6 does not.
        assert_eq!(
            illegal_operand_message(List, "a b", Left, "+", v90),
            "cannot use a list as left operand of \"+\""
        );
        assert_eq!(
            illegal_operand_message(List, "a b", Left, "+", v86),
            "can't use non-numeric string as operand of \"+\""
        );
    }

    /// The `-errorcode` from 8.5 is the description's, but for the list
    /// branch 8.6 lacks.
    #[test]
    fn the_error_code_follows_the_description_from_eight_five() {
        use OperandDesc::{FloatingPointValue, List, NonNumericString};
        for release in [TclVersion::V8_6, TclVersion::V9_0] {
            assert_eq!(
                illegal_operand_error_code(NonNumericString, "abc", release),
                "ARITH DOMAIN {non-numeric string}"
            );
            assert_eq!(
                illegal_operand_error_code(FloatingPointValue, "1.5", release),
                "ARITH DOMAIN {floating-point value}"
            );
        }
        assert_eq!(
            illegal_operand_error_code(List, "a b", TclVersion::V9_0),
            "ARITH DOMAIN list"
        );
        assert_eq!(
            illegal_operand_error_code(List, "a b", TclVersion::V8_6),
            "ARITH DOMAIN {non-numeric string}"
        );
    }

    /// tclsh 8.4.20, 8.6.18 and 9.0.4: `catch {expr {$v + 1}} m o` for each
    /// value.
    #[test]
    fn a_value_that_is_no_number_is_described_as_each_release_describes_it() {
        use OperandDesc::NonNumericString;
        use OperandSide::Left;
        let v84 = TclVersion::V8_4;
        let v86 = TclVersion::V8_6;
        let v90 = TclVersion::V9_0;
        let message = |value, release| {
            (
                illegal_operand_message(NonNumericString, value, Left, "+", release),
                illegal_operand_error_code(NonNumericString, value, release),
            )
        };
        let pair = |m: &str, c: &str| (m.to_owned(), c.to_owned());
        assert_eq!(
            message("", v84),
            pair("can't use empty string as operand of \"+\"", "NONE")
        );
        assert_eq!(
            message("", v86),
            pair(
                "can't use empty string as operand of \"+\"",
                "ARITH DOMAIN {empty string}"
            )
        );
        assert_eq!(
            message("", v90),
            pair(
                "cannot use non-numeric string \"\" as left operand of \"+\"",
                "ARITH DOMAIN {non-numeric string}"
            )
        );
        assert_eq!(
            message(" -08 ", v84),
            pair("can't use invalid octal number as operand of \"+\"", "NONE")
        );
        assert_eq!(
            message("0o8", v84),
            pair("can't use non-numeric string as operand of \"+\"", "NONE")
        );
        assert_eq!(
            message("0o8", v86),
            pair(
                "can't use invalid octal number as operand of \"+\"",
                "ARITH DOMAIN {invalid octal number}"
            )
        );
        assert_eq!(
            message("0x", v84),
            pair(
                "can't use integer value too large to represent as operand of \"+\"",
                IOVERFLOW_CODE
            )
        );
        assert_eq!(
            message("abc", v84),
            pair("can't use non-numeric string as operand of \"+\"", "NONE")
        );
    }
}
