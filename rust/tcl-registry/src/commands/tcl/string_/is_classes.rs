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

//! `string is`: the class table, the membership tests behind its fold and the type each class proves.

use super::{ArgValue, TclType, TclVersion};
use tcl_syntax::scalar_getter::{NativeScalarGetterKind, NativeScalarGetterProtocol};

/// Literal `string is` without a variable-writing option. An unidentified
/// receiver requires identical outcomes in the supported C cores and Jim.
pub(crate) fn fold_is(args: &[&str], version: Option<TclVersion>) -> Option<String> {
    let answer = if let Some(version) = version {
        fold_is_c(args, version)?
    } else {
        let jim = fold_is_jim(args)?;
        TclVersion::ALL
            .into_iter()
            .all(|release| fold_is_c(args, release) == Some(jim))
            .then_some(jim)?
    };
    Some(u8::from(answer).to_string())
}

/// The ordinary Jim class command's answer: the `tcl-cmd-core` kernel.
fn fold_is_jim(args: &[&str]) -> Option<bool> {
    tcl_cmd_core::string_is::jim_literal_class(args)
}

fn fold_is_c(args: &[&str], version: TclVersion) -> Option<bool> {
    use tcl_cmd_core::string_is::{class_check_in_release, resolve_class};

    // A fail-index target is observed on failure. Other option shapes need
    // the full completion and store transfer before they can be folded.
    let (class, strict, value) = match args {
        [class, value] => (*class, false, *value),
        [class, "-strict", value] => (*class, true, *value),
        _ => return None,
    };
    let class = resolve_class(class, version).ok()?;
    match class {
        "list" | "dict" => {
            let words = tcl_native_model::InvocationDialect::for_version(version).read_list(value);
            Some(words.is_ok_and(|words| class == "list" || words.len().is_multiple_of(2)))
        }
        "ascii" => Some(value.is_ascii() && (!value.is_empty() || !strict)),
        _ if !value.is_ascii() || value.contains('\0') => None,
        "double" if version == TclVersion::V8_4 && !value.is_empty() => {
            tcl_cmd_core::string_is::legacy_double_class(
                value,
                NativeScalarGetterProtocol::for_tcl_version(version),
                version.number_syntax(),
            )
        }
        "wideinteger" if !value.is_empty() => Some(
            NativeScalarGetterProtocol::for_tcl_version(version)
                .fresh_conversion(NativeScalarGetterKind::Wide, value.as_bytes())?
                .outcome()
                .is_ok(),
        ),
        _ => Some(class_check_in_release(class, value, strict, version).0),
    }
}

/// The type every value a `string is` test accepts holds once it has: what
/// the true edge of a branch on `[string is CLASS ?-strict? $x]` proves of
/// `x` (`docs/design/compiler/value-transfers.md` § *Predicate refinement*),
/// a type and never a value. `args` are the words after `is` — the class, as
/// Tcl reads it with its abbreviations, the options, and the value last, as
/// [`fold_is`] reads them.
///
/// The test converts the value it accepts, so the type is the representation
/// the value then holds (`tcl::unsupported::representation`, tclsh 8.6 to
/// 9.1): an integer class (`integer`, `entier`, `wideinteger`) leaves an
/// integer, `double` an integer or a double, so `Numeric`, each only under
/// `-strict`, since without it the empty string, which none converts, passes
/// every class; `dict` leaves a dictionary, the empty one included. `list`
/// leaves the empty string a pure string, and `boolean`, `true` and `false`
/// leave `0` and `1` integers, so neither proves a type, nor does a character
/// class. `-failindex` writes a variable, and an option spelled any other way
/// than `-strict` is one this reading does not decide, so either proves
/// nothing.
#[must_use]
pub fn string_is_member_type(args: &[&str]) -> Option<TclType> {
    let (class, rest) = args.split_first()?;
    let (_value, options) = rest.split_last()?;
    let mut strict = false;
    for option in options {
        match *option {
            "-strict" => strict = true,
            _ => return None,
        }
    }
    let class =
        tcl_cmd_core::string_is::resolve_class(class, tcl_dialect::TclVersion::V9_1).ok()?;
    match class {
        "integer" | "entier" | "wideinteger" if strict => Some(TclType::Int),
        "double" if strict => Some(TclType::Numeric),
        "dict" => Some(TclType::Dict),
        _ => None,
    }
}

/// Conditional C class type under the independently selected invocation
/// family and release. Lexical numeral grammar supplies no class availability,
/// and this projection supplies no physical cache, frame or store effects.
#[must_use]
pub fn string_is_member_type_for_dialect(
    args: &[&str],
    dialect: tcl_native_model::InvocationDialect,
) -> Option<TclType> {
    let point = dialect.execution_point()?;
    if point.family() != tcl_dialect::model::Family::Tcl {
        return None;
    }
    let version = point.tcl_version();
    // The class word abbreviates against the release's own class table.
    let class = tcl_cmd_core::string_is::resolve_class(
        args.first().copied()?,
        version.unwrap_or(tcl_dialect::TclVersion::V9_1),
    )
    .ok()?;
    class_available(class, version).then(|| string_is_member_type(args))?
}

/// Whether `string is class` is a *defined* operation in the target version
/// (it doesn't *raise*).  `wideinteger` requires 8.5+, `entier` 8.6+, and
/// `dict` is **9.0-only** (matching the gating below); an unknown version
/// (`None`) treats all three as unavailable, since one of the dialects it
/// could stand for raises.  Every other class exists in every release.
fn class_available(class: &str, version: Option<TclVersion>) -> bool {
    // Single source of truth: the `IS_CLASSES` arg-value table carries each
    // class's introducing release (`min_tcl`, the §6 argument-DSL rung) —
    // the same data the analyser's version diagnostics read.
    match IS_CLASSES.iter().find(|av| av.value == class) {
        Some(av) => match av.min_tcl {
            Some(min) => version.is_some_and(|v| v >= min),
            None => true,
        },
        None => true,
    }
}

/// Character classes accepted by `string is <class>`.
pub(super) static IS_CLASSES: &[ArgValue] = &[
    ArgValue {
        value: "alnum",
        detail: "Any Unicode alphabet or digit character.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "alpha",
        detail: "Any Unicode alphabet character.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "ascii",
        detail: "Any character with a value less than U+0080 (7-bit ASCII).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "boolean",
        detail: "Any valid boolean value (true/false/yes/no/on/off/0/1).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "control",
        detail: "Any Unicode control character.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "dict",
        detail: "Any proper dict structure, with optional surrounding whitespace.",
        min_tcl: Some(TclVersion::V9_0),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "digit",
        detail: "Any Unicode digit character.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "double",
        detail: "Any valid floating-point number.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "entier",
        detail: "An arbitrary-size integer. On Tcl 8.6, where integer itself was still 32-bit-bounded, entier was the only arbitrary-size class; from Tcl 9.0, where integer itself became arbitrary-size, entier is a plain synonym for it.",
        min_tcl: Some(TclVersion::V8_6),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "false",
        detail: "Any valid boolean false value.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "graph",
        detail: "Any Unicode printing character, except space.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "integer",
        detail: "Any valid integer, with optional surrounding whitespace. Bounded to 32 bits through Tcl 8.6; arbitrary size from Tcl 9.0 onward.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "list",
        detail: "Any proper list structure, with optional surrounding whitespace.",
        // Absent from the Tcl 8.4 manpage's class table (18 classes, no
        // `list`); present from 8.5 (19 classes, `list` added). `string is
        // list ...` raises "bad class" under an 8.4 dialect rather than
        // returning 0/1.
        min_tcl: Some(TclVersion::V8_5),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "lower",
        detail: "Any Unicode lower case alphabet character.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "print",
        detail: "Any Unicode printing character, including space.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "punct",
        detail: "Any Unicode punctuation character.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "space",
        detail: "Any Unicode whitespace character.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "true",
        detail: "Any valid boolean true value.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "upper",
        detail: "Any upper case alphabet character.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "wideinteger",
        detail: "Any valid wide integer, with optional surrounding whitespace. From Tcl 9.0 this is bounded to the signed 64-bit range; Tcl 8.5/8.6 accept a wider positive magnitude (up to 2^64-1).",
        min_tcl: Some(TclVersion::V8_5),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "wordchar",
        detail: "Any Unicode word character (alphanumeric + connector punctuation).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "xdigit",
        detail: "Any hexadecimal digit character (0-9, A-F, a-f).",
        ..ArgValue::DEFAULT
    },
];
