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

//! The one live compile-time value model
//! (`docs/design/compiler/value-evaluation.md` § *The direct route*).
//!
//! [`ConstOps`] implements the runtime seam
//! ([`tcl_syntax::value::ValueOps`]) once for the direct route, so every
//! shipped evaluator *is* a call into the shared cores (`tcl-cmd-core`,
//! `tcl-syntax`) rather than a second implementation beside them. It is
//! byte-exact, bound to the target semantics the analysis context names,
//! charges the evaluation budget, and is poisoned by the first fault so no
//! partial answer escapes: a core that could not decide records the fault
//! and [`ConstOps::take`] declines whatever value it was handed.
//!
//! The release axes a core can read are the admissibility set
//! ([`Needs`]). An evaluator opens the model with [`ConstOps::admit`] and
//! the bits its cores read; the cores' signatures were written for a
//! runtime that has already chosen its release, so the axes they read
//! implicitly are discharged here — an index numeral is pre-resolved under
//! the admitted grammar ([`ConstOps::index`]), a character count runs under
//! the admitted model, and a numeral is parsed under the admitted grammar.
//! A profile that declares a base release evaluates under it, iRules on its
//! 8.4-derived engine included ([`TargetSemantics::of`]). With no release
//! declared, each answer is the one every modelled release gives; where the
//! releases differ the axis is declined with
//! [`DeclineReason::ReleaseAmbiguous`], per operation. Two axes are never
//! answerable on this route — [`Needs::PLATFORM`] and [`Needs::WALL_CLOCK`]
//! — which is how a host- or clock-reading core is kept off it by
//! construction.

use std::rc::Rc;

use tcl_cmd_core::error::CmdError;
use tcl_dialect::{
    ByteStringEncoding, DialectProfile, LexerGrammar, NumberSpelling, NumberSyntax,
    StringCharacterModel, TclVersion,
};
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::number::{
    Number, ParseFlags, format_double, parse_whole_with, spell_double, spell_wide,
};
use tcl_syntax::raw_string::{NativeValueAccessRefusal, UnicodeAccessError};
use tcl_syntax::value::{DictPairs, ValueError, ValueOps, canonical_dict_slots};

use crate::types::TclType;

use super::answers::{ExactValue, ExactValueOrUnavailable, NumericValue, RepresentationEvidence};
use super::context::{AnalysisContext, Budget};
use super::decline::{Axis, DeclineReason};

/// One elementary step of native work. A million units is a few
/// milliseconds on the machines the acceptance suite runs on; every route
/// converts its own work into this unit so one request budget bounds all
/// of them.
pub type WorkUnits = u64;

/// The fixed charge of one admission, so a solver iteration that declines
/// ten thousand times is still bounded.
pub const ADMISSION_CHARGE: WorkUnits = 8;

/// The release axes a core can read: one bit per axis, and the axis's
/// owner in the tree decides each bit (the evaluation page's table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Needs(u16);

impl Needs {
    /// No axis: the core reads nothing release-dependent.
    pub const NONE: Self = Self(0);
    /// How a numeral operand is read (`NumberSyntax`).
    pub const NUMERAL_GRAMMAR: Self = Self(1 << 0);
    /// How an index numeral is read (`index::read_under`).
    pub const INDEX_GRAMMAR: Self = Self(1 << 1);
    /// The character-counting rule (`StringCharacterModel::count_for`).
    pub const CHAR_MODEL: Self = Self(1 << 2);
    /// The addressing unit for a character index.
    pub const CHAR_INDEXING: Self = Self(1 << 3);
    /// Whether an integer operation widens or raises (`ValueOps::int_add`).
    pub const INT_TOWER: Self = Self(1 << 4);
    /// The `binary` field-letter set and the `u` suffix.
    pub const BINARY_FIELDS: Self = Self(1 << 5);
    /// The `format` conversion set.
    pub const FORMAT_VERBS: Self = Self(1 << 6);
    /// The `string is` class set and its bounds.
    pub const STRING_CLASSES: Self = Self(1 << 7);
    /// ARE syntax, flags, and engine limits.
    pub const REGEXP_FEATURES: Self = Self(1 << 8);
    /// The canonical quoting of a list result (`tcl_syntax::list`).
    pub const LIST_RENDERING: Self = Self(1 << 9);
    /// Canonical dict key order and the duplicate rule.
    pub const DICT_ORDER: Self = Self(1 << 10);
    /// Case folding and ordering in a comparison.
    pub const COLLATION: Self = Self(1 << 11);
    /// How a code point above `U+00FF` crosses to bytes.
    pub const BYTE_STRINGS: Self = Self(1 << 12);
    /// How a non-ASCII source literal was decoded.
    pub const SOURCE_ENCODING: Self = Self(1 << 13);
    /// Host facts a platform-backed core reads: never satisfiable here.
    pub const PLATFORM: Self = Self(1 << 14);
    /// A clock, locale, or timezone read: never satisfiable here.
    pub const WALL_CLOCK: Self = Self(1 << 15);

    /// Every axis, in bit order, with the [`Axis`] it declines as.
    const AXES: [(Self, Axis); 16] = [
        (Self::NUMERAL_GRAMMAR, Axis::NumeralGrammar),
        (Self::INDEX_GRAMMAR, Axis::IndexGrammar),
        (Self::CHAR_MODEL, Axis::CharacterModel),
        (Self::CHAR_INDEXING, Axis::CharIndexing),
        (Self::INT_TOWER, Axis::IntTower),
        (Self::BINARY_FIELDS, Axis::BinaryFields),
        (Self::FORMAT_VERBS, Axis::FormatVerbs),
        (Self::STRING_CLASSES, Axis::StringClasses),
        (Self::REGEXP_FEATURES, Axis::RegexpFeatures),
        (Self::LIST_RENDERING, Axis::ListRendering),
        (Self::DICT_ORDER, Axis::DictOrder),
        (Self::COLLATION, Axis::Collation),
        (Self::BYTE_STRINGS, Axis::ByteStrings),
        (Self::SOURCE_ENCODING, Axis::SourceEncoding),
        (Self::PLATFORM, Axis::Platform),
        (Self::WALL_CLOCK, Axis::WallClock),
    ];

    /// The union of two sets, usable in a constant.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Whether every bit of `other` is set.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Whether no bit is set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The axes set, in bit order.
    pub fn axes(self) -> impl Iterator<Item = Axis> {
        Self::AXES
            .into_iter()
            .filter(move |(bit, _)| self.contains(*bit))
            .map(|(_, axis)| axis)
    }
}

impl std::ops::BitOr for Needs {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        self.union(rhs)
    }
}

/// Representation evidence for a computed value: how the route constructed
/// it, never a coercion of its bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Representation {
    /// A string.
    String,
    /// A byte array.
    ByteArray,
    /// A list, rendered canonically.
    List,
    /// A dict, rendered canonically.
    Dict,
    /// An integer, spelled canonically.
    Int,
    /// A double, spelled canonically.
    Double,
}

impl Representation {
    /// The internal representation the evidence names.
    #[must_use]
    pub const fn tcl_type(self) -> TclType {
        match self {
            Self::String => TclType::String,
            Self::ByteArray => TclType::ByteArray,
            Self::List => TclType::List,
            Self::Dict => TclType::Dict,
            Self::Int => TclType::Int,
            Self::Double => TclType::Double,
        }
    }
}

/// The command-core view of an exact value with shareable byte storage.
#[derive(Debug, Clone)]
pub struct ConstValue(ExactValue<Rc<[u8]>>);

impl PartialEq for ConstValue {
    fn eq(&self, other: &Self) -> bool {
        self.0.bytes == other.0.bytes && self.0.representation == other.0.representation
    }
}

impl Eq for ConstValue {}

impl ConstValue {
    /// A string value.
    #[must_use]
    pub fn text(text: &str) -> Self {
        Self::bytes(text.as_bytes(), Representation::String)
    }

    /// An integer value, spelled canonically.
    #[must_use]
    pub fn int(value: i64) -> Self {
        Self::bytes(value.to_string().as_bytes(), Representation::Int)
    }

    /// A value from raw bytes with its representation evidence.
    #[must_use]
    pub fn bytes(bytes: &[u8], repr: Representation) -> Self {
        Self(ExactValue::constructed(Rc::from(bytes), repr.tcl_type()))
    }

    /// The exact value's bytes with the representation its evidence or its
    /// classification names.
    #[must_use]
    pub fn from_exact(value: &ExactValue) -> Self {
        let repr = match value.representation {
            RepresentationEvidence::Constructed(t) => match t {
                TclType::ByteArray => Representation::ByteArray,
                TclType::List => Representation::List,
                TclType::Dict => Representation::Dict,
                TclType::Int | TclType::Boolean => Representation::Int,
                TclType::Double => Representation::Double,
                _ => Representation::String,
            },
            RepresentationEvidence::Unknown => match value.numeric {
                Some(NumericValue::Int(_) | NumericValue::Bool(_)) => Representation::Int,
                Some(NumericValue::Float(_)) => Representation::Double,
                None => Representation::String,
            },
        };
        Self::bytes(&value.bytes, repr)
    }

    /// The bytes as text, when they are UTF-8.
    #[must_use]
    pub fn as_utf8(&self) -> Option<&str> {
        self.0.as_str().ok()
    }

    pub(super) fn as_bytes(&self) -> &[u8] {
        &self.0.bytes
    }

    /// The published exact value, including its numeric reading where known.
    #[must_use]
    pub fn into_exact(self) -> ExactValue {
        self.0
            .classify_constructed()
            .map_bytes(|bytes| bytes.to_vec())
    }
}

/// The target semantics one evaluation runs under. Every field is the
/// declared release's answer, or `None` when the profile declares no release
/// or declares an answer of its own on that axis: the axis then answers per
/// operation with the answer every modelled release gives, and declines
/// where they differ.
#[derive(Debug, Clone, Copy)]
pub struct TargetSemantics {
    /// The release the profile declares: its own for a plain Tcl profile,
    /// its runtime base for a vendor dialect that records one.
    pub release: Option<TclVersion>,
    /// The numeral grammar, when the declared release decides it.
    pub numerals: Option<NumberSyntax>,
    /// The character-counting model, when the declared release decides it.
    pub character_model: Option<StringCharacterModel>,
    /// How a code point above `U+00FF` crosses to bytes, when named.
    pub byte_strings: Option<ByteStringEncoding>,
    /// Whether the analysed text is the text the target reads: from 9.0,
    /// which decodes source as UTF-8 as this front end does by default, or
    /// under any release whose script encoding is declared and was used to
    /// decode the file. A non-ASCII operand is admissible only then.
    pub source_utf8: bool,
    /// Whether a list's first element that starts with `#` is brace-quoted
    /// when the list is rendered, when one release names it: from 8.5
    /// (`[list # a]` is `{#} a`, so the list is never a comment when it is
    /// evaluated as a script), not in 8.4 (`# a`).
    pub quotes_leading_hash: Option<bool>,
    /// How a computed number is spelled and read back, when the declared
    /// release decides it; with none, a double is spelled only where every
    /// release spells it alike. [`ConstOps::admit`] widens it for a module
    /// that may set the precision ([`AnalysisContext::precision_written`]).
    pub spelling: Option<NumberSpelling>,
    /// Whether the module may set the precision variable — the
    /// [`AnalysisContext::precision_written`] the run was admitted under.
    pub precision_written: bool,
    /// The profile, when the request names one.
    pub profile: Option<&'static DialectProfile>,
    /// Actual source grammar retained by the analysis request.
    pub grammar: LexerGrammar,
}

impl TargetSemantics {
    /// The target `profile` names (`docs/design/compiler/value-transfers.md`
    /// § *Rulings* 7 and 8). A profile that declares a base release evaluates
    /// under it: the plain Tcl profiles, and every vendor dialect whose
    /// runtime base the catalogue records — iRules, iApps and tmsh on their
    /// 8.4-derived engine, `expect` on 8.6, the EDA shells on theirs. Each
    /// axis then answers as that release does, except an axis on which the
    /// profile declares an answer of its own that differs from the release's
    /// (the F5 dialects' unmeasured character model): the declared divergence
    /// blocks the release's answer, and the axis answers by unanimity. A
    /// profile that declares no release — the lenient `tcl` sink, `tk`,
    /// `f5-bigip` — answers every axis by unanimity.
    #[must_use]
    pub fn of(profile: Option<&'static DialectProfile>) -> Self {
        let release = profile.and_then(DialectProfile::runtime_version);
        let numerals = release
            .map(TclVersion::number_syntax)
            .filter(|numbers| Some(*numbers) == profile.map(|p| p.grammar.numbers));
        let character_model = release
            .map(TclVersion::string_character_model)
            .filter(|model| Some(*model) == profile.and_then(DialectProfile::character_model));
        Self {
            release,
            numerals,
            character_model,
            byte_strings: release.map(TclVersion::byte_string_encoding),
            source_utf8: release.is_some_and(|v| v >= TclVersion::V9_0),
            quotes_leading_hash: release.map(|v| v >= TclVersion::V8_5),
            spelling: release.map(|_| NumberSpelling::of_profile(profile)),
            precision_written: false,
            profile,
            grammar: profile.map_or_else(LexerGrammar::default, |profile| profile.grammar),
        }
    }

    /// The authored value model with this request's actual source grammar.
    /// A release's numeral model applies only when it agrees with that
    /// grammar. This selection grants neither executable fold eligibility
    /// nor an original Native value, representation or observer proof.
    #[must_use]
    pub fn for_context(context: &AnalysisContext) -> Self {
        let mut target = Self::of(context.profile);
        target.grammar = context.grammar;
        if let super::ScriptEncoding::Declared(_) = context.script_encoding {
            target.source_utf8 = true;
        }
        target.numerals = target
            .numerals
            .filter(|numbers| *numbers == context.grammar.numbers);
        // A spelling read back under another grammar proves nothing: fall
        // back to the spelling every grammar agrees on.
        target.spelling = target
            .spelling
            .filter(|spelling| spelling.numbers() == context.grammar.numbers);
        target
    }

    /// The text a computed double is written as under this target
    /// ([`spell_double`]), or `None` where it has none that reads back as
    /// `f`. With no release named, the text every grammar's spelling agrees
    /// on.
    #[must_use]
    pub fn spell_double(&self, f: f64) -> Option<String> {
        let under = |spelling: NumberSpelling| {
            spell_double(f, spelling.with_precision_written(self.precision_written))
        };
        match self.spelling {
            Some(spelling) => under(spelling),
            None => NumberSyntax::unanimous(|numbers| under(NumberSpelling::of(numbers, None)))
                .flatten(),
        }
    }

    /// The text a computed wide integer is written as under this target
    /// ([`spell_wide`]): `None` for an `INT64_MIN` the target may print as
    /// `-8`.
    #[must_use]
    pub fn spell_wide(&self, i: i64) -> Option<String> {
        match self.spelling {
            Some(spelling) => spell_wide(i, spelling),
            None => {
                NumberSyntax::unanimous(|numbers| spell_wide(i, NumberSpelling::of(numbers, None)))
                    .flatten()
            }
        }
    }

    /// Render a value through the selected list writer. An unselected target
    /// can propagate a string only when all supported writers agree.
    #[must_use]
    pub fn render_list<S: AsRef<str>>(&self, elements: &[S]) -> Option<String> {
        let words: Vec<_> = elements
            .iter()
            .map(|word| word.as_ref().as_bytes())
            .collect();
        String::from_utf8(self.render_list_bytes(&words)?).ok()
    }

    fn render_list_bytes(&self, words: &[impl AsRef<[u8]>]) -> Option<Vec<u8>> {
        use tcl_syntax::list_result::{
            NativeListResultSerialization as Layout, portable_list_result,
        };
        let layout = self
            .profile
            .and_then(|p| crate::InvocationDialect::of_profile(p).list_result_serialization())
            .or_else(|| {
                self.quotes_leading_hash.map(|quotes| {
                    if quotes {
                        Layout::Tcl85Plus
                    } else {
                        Layout::Tcl84
                    }
                })
            });
        layout.map_or_else(
            || portable_list_result(words),
            |layout| Some(layout.render(words)),
        )
    }
}

/// Fresh literal's native floating-point input for a selected release.
/// Tcl 8.4 reads the spelling with strtod; 8.5 and 8.6 classify integer
/// spellings first. An unknown receiver needs an identical result in Jim and
/// every C Tcl release, including the sign of zero.
pub(crate) fn literal_double(text: &str, version: Option<TclVersion>) -> Option<f64> {
    use tcl_dialect::model::{DialectPoint, Release};
    use tcl_syntax::scalar_getter::{
        NativeScalarGetterKind, NativeScalarGetterProtocol, NativeScalarGetterValue,
    };
    let under = |point| {
        let protocol = NativeScalarGetterProtocol::for_point(point)?;
        let value = protocol.fresh_conversion(NativeScalarGetterKind::Double, text.as_bytes())?;
        match value.outcome().ok()? {
            NativeScalarGetterValue::Double(value) if value.is_finite() => Some(value),
            _ => None,
        }
    };
    if let Some(version) = version {
        return under(DialectPoint::for_tcl_version(version));
    }
    let common = under(DialectPoint::canonical(Release::JIM_0_84))?;
    TclVersion::ALL
        .into_iter()
        .all(|release| {
            under(DialectPoint::for_tcl_version(release)).map(f64::to_bits)
                == Some(common.to_bits())
        })
        .then_some(common)
}

/// The one live compile-time value model. Byte-exact, release-bound,
/// budget-charging, and poisoned by the first fault so no partial answer
/// escapes.
pub struct ConstOps<'ctx> {
    target: TargetSemantics,
    admitted: Needs,
    budget: &'ctx mut Budget,
    fault: Option<DeclineReason>,
    raised: Option<Raised>,
}

/// The program's own error as a completion's payload: the message and the
/// `-errorcode` the command raises, each exact only where the route proves
/// it under every release the target names
/// (`docs/design/compiler/value-transfers.md` § *`catch`, `try`, and
/// completion*).
#[derive(Debug, Clone, PartialEq)]
pub struct Raised {
    /// The error message.
    pub message: ExactValueOrUnavailable,
    /// The `-errorcode`.
    pub error_code: ExactValueOrUnavailable,
}

impl Raised {
    /// An error whose message and `-errorcode` the route does not prove.
    #[must_use]
    pub const fn unproven() -> Self {
        Self {
            message: ExactValueOrUnavailable::unproven_string(),
            error_code: ExactValueOrUnavailable::unproven_string(),
        }
    }

    /// The error a parser's own failure raises — a list that does not split
    /// (`unmatched open brace in list`) or a dict of odd length (`missing
    /// value to go with key`): every release words them alike (measured, 8.4
    /// to 9.1), and the `-errorcode` is `NONE` before 8.6 and the parser's
    /// from it (`TCL VALUE LIST BRACE`, `TCL VALUE DICTIONARY`). A message
    /// that is none of these is unproven.
    #[must_use]
    pub fn parse_failure(target: &TargetSemantics, message: &str) -> Self {
        let code = tcl_syntax::list::ListError::from_message(message)
            .map(tcl_syntax::list::ListError::error_code)
            .or_else(|| {
                (message == "missing value to go with key").then_some("TCL VALUE DICTIONARY")
            });
        let Some(code) = code else {
            return Self::unproven();
        };
        Self::unanimous(target, |release| {
            let code = if release >= TclVersion::V8_6 {
                code
            } else {
                "NONE"
            };
            (message.to_owned(), code.to_owned())
        })
    }

    /// An error with exactly this message and `-errorcode`.
    #[must_use]
    pub fn exact(message: impl Into<String>, error_code: impl Into<String>) -> Self {
        Self {
            message: ExactValueOrUnavailable::exact_text(message),
            error_code: ExactValueOrUnavailable::exact_text(error_code),
        }
    }

    /// The error `per_release` says a release raises, as every release the
    /// target names agrees on it: the named release's own, or — with none
    /// named — the one the whole ladder shares, field by field. A field the
    /// releases disagree on is unproven.
    #[must_use]
    pub fn unanimous(
        target: &TargetSemantics,
        per_release: impl Fn(TclVersion) -> (String, String),
    ) -> Self {
        let releases: Vec<TclVersion> = target
            .release
            .map_or_else(|| TclVersion::ALL.to_vec(), |release| vec![release]);
        let answers: Vec<(String, String)> = releases.into_iter().map(per_release).collect();
        let agreed = |field: fn(&(String, String)) -> &String| {
            let first = field(&answers[0]);
            answers
                .iter()
                .all(|answer| field(answer) == first)
                .then(|| first.clone())
        };
        let exact = |text: Option<String>| {
            text.map_or_else(
                ExactValueOrUnavailable::unproven_string,
                ExactValueOrUnavailable::exact_text,
            )
        };
        Self {
            message: exact(agreed(|answer| &answer.0)),
            error_code: exact(agreed(|answer| &answer.1)),
        }
    }
}

impl<'ctx> ConstOps<'ctx> {
    /// Open a value model for this evaluation under `ctx`'s target,
    /// proving the target answers every axis in `needs`. The `Err` is the
    /// interface contract's decline, so a route's admissibility failure
    /// and a core's runtime failure are one kind of answer.
    ///
    /// # Errors
    ///
    /// An axis no operation can answer on this route (`PLATFORM`,
    /// `WALL_CLOCK`), a cancelled request, or an exhausted budget.
    pub fn admit(
        ctx: &AnalysisContext,
        budget: &'ctx mut Budget,
        needs: Needs,
    ) -> Result<Self, DeclineReason> {
        if budget.is_cancelled() {
            return Err(DeclineReason::Budget(
                super::decline::BudgetLimit::Cancelled,
            ));
        }
        for (bit, axis) in [
            (Needs::PLATFORM, Axis::Platform),
            (Needs::WALL_CLOCK, Axis::WallClock),
        ] {
            if needs.contains(bit) {
                return Err(DeclineReason::ReleaseAmbiguous(axis));
            }
        }
        budget.charge_work(ADMISSION_CHARGE)?;
        Ok(Self {
            target: TargetSemantics {
                precision_written: ctx.precision_written,
                ..TargetSemantics::for_context(ctx)
            },
            admitted: needs,
            budget,
            fault: None,
            raised: None,
        })
    }

    /// The admitted target semantics.
    #[must_use]
    pub const fn target(&self) -> &TargetSemantics {
        &self.target
    }

    /// The first fault recorded, when one was.
    #[must_use]
    pub const fn fault(&self) -> Option<DeclineReason> {
        self.fault
    }

    /// Record a fault. The first one stands; a later one never replaces
    /// it.
    pub fn poison(&mut self, reason: DeclineReason) {
        if self.fault.is_none() {
            self.fault = Some(reason);
        }
    }

    /// A core call outside the admitted set is an evaluator defect, not a
    /// decline: it is recorded as a malformed answer so the run declines
    /// with a notice rather than answering under an axis nobody proved.
    fn require(&mut self, needs: Needs) {
        if !self.admitted.contains(needs) {
            self.poison(DeclineReason::MalformedAnswer);
        }
    }

    /// Charge `units` of work.
    ///
    /// # Errors
    ///
    /// The budget's limit, which is also recorded as the run's fault.
    pub fn charge(&mut self, units: WorkUnits) -> Result<(), DeclineReason> {
        self.budget
            .charge_work(units)
            .inspect_err(|e| self.poison(*e))
    }

    /// Charge `bytes` before the allocation that would produce them.
    ///
    /// # Errors
    ///
    /// The allocation bound, which is also recorded as the run's fault.
    pub fn charge_bytes(&mut self, bytes: u64) -> Result<(), DeclineReason> {
        self.budget
            .charge_allocation(bytes)
            .inspect_err(|e| self.poison(*e))
    }

    /// The work this evaluation can still charge: what a core that meters
    /// its own work (the regexp engine's fuel) may spend before its charge
    /// is made.
    #[must_use]
    pub fn remaining_work(&self) -> WorkUnits {
        self.budget.remaining_work()
    }

    /// The value as text, or the decline for bytes that are not text.
    ///
    /// # Errors
    ///
    /// `NotText`, never a U+FFFD substitution.
    pub fn text_of(&mut self, value: &ConstValue) -> Result<Rc<str>, DeclineReason> {
        if let Some(s) = value.as_utf8() {
            Ok(Rc::from(s))
        } else {
            self.poison(DeclineReason::NotText);
            Err(DeclineReason::NotText)
        }
    }

    /// The value as text a string core may index: text, and — under the
    /// `SOURCE_ENCODING` axis — ASCII unless the target decodes source as
    /// UTF-8, because a non-ASCII literal is a different value under a
    /// release whose script reader follows `encoding system`.
    ///
    /// # Errors
    ///
    /// `NotText`, or `ReleaseAmbiguous(SourceEncoding)`.
    pub fn admissible_text(&mut self, value: &ConstValue) -> Result<Rc<str>, DeclineReason> {
        let text = self.text_of(value)?;
        if !text.is_ascii() && !self.target.source_utf8 {
            self.require(Needs::SOURCE_ENCODING);
            let reason = DeclineReason::ReleaseAmbiguous(Axis::SourceEncoding);
            self.poison(reason);
            return Err(reason);
        }
        Ok(text)
    }

    /// Resolve an index operand under the admitted release, returning the
    /// canonical decimal spelling every release's grammar reads identically.
    /// The cores read an index under the grammar they are handed, but only
    /// this step declines what no single grammar can answer: a release-less
    /// target whose releases disagree (`string range abcdefghijkl 010 end` is
    /// `ijkl` up to 8.6 and `kl` from 9.0), a literal 8.6 compiles apart, and
    /// a reading the host's `long` decides. An index further out than −1 or
    /// `len + 1` is spelt as that one, which every release reads alike and no
    /// command reads apart from it.
    ///
    /// # Errors
    ///
    /// `ReleaseAmbiguous(IndexGrammar)` when the grammars disagree and no
    /// release is named, or where 8.6 compiles the literal apart from the
    /// value ([`tcl_cmd_core::index::compiles_apart`]);
    /// `ReleaseAmbiguous(Platform)` for a reading the host's `long`
    /// decides; `WrongRepresentation` for a malformed index, which the program
    /// raises `bad index` on.
    pub fn index(&mut self, spec: &ConstValue, len: usize) -> Result<ConstValue, DeclineReason> {
        use tcl_cmd_core::index::{IndexReading, compiles_apart, read_under, read_with};
        self.require(Needs::INDEX_GRAMMAR);
        self.charge(1)?;
        let text = self.text_of(spec)?;
        // 8.6 reads such a word apart as a literal and as a value; a
        // target that names no release may be 8.6.
        if compiles_apart(&text, len, self.target.release.unwrap_or(TclVersion::V8_6)) {
            let reason = DeclineReason::ReleaseAmbiguous(Axis::IndexGrammar);
            self.poison(reason);
            return Err(reason);
        }
        let reading = if let (Some(release), Some(_)) = (self.target.release, self.target.numerals)
        {
            read_under(&text, len, release)
        } else {
            let Some(reading) = NumberSyntax::unanimous(|numbers| read_with(&text, len, numbers))
            else {
                let reason = DeclineReason::ReleaseAmbiguous(Axis::IndexGrammar);
                self.poison(reason);
                return Err(reason);
            };
            reading
        };
        if let IndexReading::At(index) = reading {
            let past = i64::try_from(len).unwrap_or(i64::MAX).saturating_add(1);
            Ok(ConstValue::int(index.clamp(-1, past)))
        } else if matches!(reading, IndexReading::HostLong(_) | IndexReading::Unsure) {
            let reason = DeclineReason::ReleaseAmbiguous(Axis::Platform);
            self.poison(reason);
            Err(reason)
        } else {
            // A malformed index is the program's error, worded by the
            // release (measured, 8.4 to 9.1): 8.4 `must be integer or
            // end?-integer?`, 8.5 on `must be integer?[+-]integer? or
            // end?[+-]integer?`, with the `-errorcode` from 8.6.
            let raised = Raised::unanimous(&self.target, |release| {
                let wording = if release < TclVersion::V8_5 {
                    "integer or end?-integer?"
                } else {
                    "integer?[+-]integer? or end?[+-]integer?"
                };
                let code = if release >= TclVersion::V8_6 {
                    "TCL VALUE INDEX"
                } else {
                    "NONE"
                };
                (
                    format!("bad index \"{text}\": must be {wording}"),
                    code.to_owned(),
                )
            });
            Err(self.decline_raising(raised))
        }
    }

    /// The grammar a core reads an index [`Self::index`] returned under: the
    /// target's where it names one, else 9.0's, which reads the canonical
    /// decimal spelling `index` answers as every other release does.
    #[must_use]
    pub fn index_numbers(&self) -> NumberSyntax {
        self.target.numerals.unwrap_or(NumberSyntax::Tcl90)
    }

    /// The decline a failed core call answers with: the run's fault when
    /// one was recorded during the call, else [`DeclineReason::
    /// WrongRepresentation`] — the program's own error, which a route that
    /// models completions takes as the error it is ([`Self::raised`]) and
    /// every other route keeps a decline, an error being no value.
    #[must_use]
    pub fn decline(&mut self, error: &CmdError) -> DeclineReason {
        match error.message() {
            Ok(message) => self.decline_raising(Raised::parse_failure(&self.target, message)),
            Err(refusal) => self.decline_execution(&refusal),
        }
    }

    /// [`Self::decline`] for a seam error.
    #[must_use]
    pub fn decline_value(&mut self, error: &ValueError) -> DeclineReason {
        if let Some(refusal) = error.native_access_refusal() {
            return self.decline_access(refusal);
        }
        // Overflow belongs to the target's integer tower, not a guest error
        // inferred from this fixed-width analysis model.
        if *error == ValueError::IntegerOverflow {
            return self.fault.unwrap_or(DeclineReason::WrongRepresentation);
        }
        let bytes = error.message_bytes();
        let raised = std::str::from_utf8(&bytes).map_or_else(
            |_| Raised::unproven(),
            |message| Raised::parse_failure(&self.target, message),
        );
        self.decline_raising(raised)
    }

    /// Classify an operational decline by its typed owner, without inspecting
    /// its diagnostic. The caller retains the borrowed command's full cause;
    /// an analysis decline never manufactures a guest completion.
    fn decline_execution(&mut self, refusal: &NativeExecutionError) -> DeclineReason {
        match refusal {
            NativeExecutionError::ValueAccessRefusal(refusal) => self.decline_access(*refusal),
            NativeExecutionError::CompilationAdmission(_)
            | NativeExecutionError::CompileServiceRefusal(_)
            | NativeExecutionError::ExpressionRefusal(_)
            | NativeExecutionError::HostCommandRefusal(_) => {
                self.poison(DeclineReason::Unsupported);
                self.fault.expect("recorded operational refusal")
            }
        }
    }

    /// Operational refusal poisons the model and cannot become a guest error.
    fn decline_access(&mut self, refusal: NativeValueAccessRefusal) -> DeclineReason {
        let reason = match refusal {
            NativeValueAccessRefusal::Unicode(_) => DeclineReason::NotText,
            NativeValueAccessRefusal::CharacterModelUnavailable => {
                DeclineReason::ReleaseAmbiguous(Axis::CharacterModel)
            }
            _ => DeclineReason::Unsupported,
        };
        self.poison(reason);
        self.fault.expect("recorded operational refusal")
    }

    /// Compute logical string contents through the shared portable owner.
    /// Binary octets outside ASCII need their independent string-value conversion;
    /// their physical encoding is not an authored Unicode string.
    pub(super) fn logical_append(
        &mut self,
        current: Option<ConstValue>,
        values: &[ConstValue],
    ) -> Result<ConstValue, DeclineReason> {
        if !values.is_empty()
            && current.iter().chain(values).any(|value| {
                value.0.representation == RepresentationEvidence::Constructed(TclType::ByteArray)
                    && !value.0.bytes.is_ascii()
            })
        {
            self.poison(DeclineReason::Unsupported);
            return Err(self.fault.expect("recorded binary string-value refusal"));
        }
        tcl_cmd_core::var::logical_append_value(self, current, values)
            .map_err(|error| self.decline(&error))
    }

    /// The decline for a failure that is the program's error, `raised`
    /// being what it raises: the run's fault when one was recorded.
    fn decline_raising(&mut self, raised: Raised) -> DeclineReason {
        if let Some(fault) = self.fault {
            return fault;
        }
        self.raised.get_or_insert(raised);
        DeclineReason::WrongRepresentation
    }

    /// The error the run raised, when its last failure was the program's
    /// own and no fault was recorded: the answer a route that models
    /// completions gives in place of the decline. A fault — an inadmissible
    /// axis, a budget — leaves nothing raised, and stays a decline.
    #[must_use]
    pub fn raised(&mut self) -> Option<Raised> {
        if self.fault.is_some() {
            return None;
        }
        self.raised.take()
    }

    /// Close the evaluation: the value, or the first recorded fault. A
    /// poisoned run declines whatever value the core returned, so a
    /// `native_char_len` that could not decide cannot leak a zero.
    ///
    /// # Errors
    ///
    /// The recorded fault, or the result-bytes bound.
    pub fn take(self, value: ConstValue) -> Result<ExactValue, DeclineReason> {
        if let Some(fault) = self.fault {
            return Err(fault);
        }
        self.budget
            .charge_result(u64::try_from(value.0.bytes.len()).unwrap_or(u64::MAX))?;
        Ok(value.into_exact())
    }

    /// [`Self::take`] for a route that publishes several values — a result
    /// and the values its stores write — each charged as published bytes.
    ///
    /// # Errors
    ///
    /// The recorded fault, or the result-bytes bound.
    pub fn take_all(self, values: Vec<ConstValue>) -> Result<Vec<ExactValue>, DeclineReason> {
        if let Some(fault) = self.fault {
            return Err(fault);
        }
        let budget = self.budget;
        values
            .into_iter()
            .map(|value| {
                budget.charge_result(u64::try_from(value.0.bytes.len()).unwrap_or(u64::MAX))?;
                Ok(value.into_exact())
            })
            .collect()
    }

    /// A numeral read under the admitted grammar, integer-only when
    /// `integer_only`.
    fn parse_number(&mut self, text: &str, integer_only: bool) -> Option<Number> {
        self.require(Needs::NUMERAL_GRAMMAR);
        let parse = |numbers: NumberSyntax| {
            parse_whole_with(
                text,
                ParseFlags {
                    integer_only,
                    ..ParseFlags::for_syntax(numbers)
                },
            )
        };
        if let Some(numbers) = self.target.numerals {
            parse(numbers)
        } else if let Some(answer) = NumberSyntax::unanimous(parse) {
            answer
        } else {
            self.poison(DeclineReason::ReleaseAmbiguous(Axis::NumeralGrammar));
            None
        }
    }

    /// The canonical spelling of an integer past the wide boundary under a
    /// release that widens.
    fn widened(&mut self, sum: i128) -> Result<ConstValue, ValueError> {
        let spelled = sum.to_string();
        if self
            .charge_bytes(u64::try_from(spelled.len()).unwrap_or(u64::MAX))
            .is_err()
        {
            return Err(ValueError::IntegerOverflow);
        }
        Ok(ConstValue::bytes(spelled.as_bytes(), Representation::Int))
    }
}

impl tcl_cmd_core::index::NativeIndexOps for ConstOps<'_> {
    type IndexOperation = ();
    type IndexBinaryReceipt = ();
    type IndexKeyOwner = Self::Value;
    fn index_borrowed_key(
        &mut self,
        value: &Self::Value,
    ) -> Result<Self::Value, tcl_cmd_core::CmdError> {
        Ok(value.clone())
    }
    fn index_created_key(
        &mut self,
        value: Self::Value,
    ) -> Result<Self::Value, tcl_cmd_core::CmdError> {
        Ok(value)
    }
    fn index_key_value(owner: &Self::Value) -> &Self::Value {
        owner
    }
    fn index_hold_key_result(
        &mut self,
        owner: &Self::Value,
    ) -> Result<Self::Value, tcl_cmd_core::CmdError> {
        Ok(owner.clone())
    }
    fn index_finish_held_key_result(
        &mut self,
        held: Self::Value,
    ) -> Result<tcl_cmd_core::index::IndexResult<Self::Value>, tcl_cmd_core::CmdError> {
        Ok(tcl_cmd_core::index::IndexResult::Detached(held))
    }
    fn index_key_argument(
        &mut self,
        owner: &Self::Value,
    ) -> Result<Self::Value, tcl_cmd_core::CmdError> {
        Ok(owner.clone())
    }
    fn index_key_result(
        &mut self,
        owner: &Self::Value,
        created: bool,
        ancestor: bool,
    ) -> Result<tcl_cmd_core::index::IndexResult<Self::Value>, tcl_cmd_core::CmdError> {
        Ok(if created {
            tcl_cmd_core::index::IndexResult::Created(owner.clone())
        } else if ancestor {
            tcl_cmd_core::index::IndexResult::Detached(owner.clone())
        } else {
            tcl_cmd_core::index::IndexResult::Borrowed(owner.clone())
        })
    }
    fn enter_index_operation(&mut self) -> Result<(), tcl_cmd_core::CmdError> {
        Ok(())
    }
    fn check_index_operation(&mut self, (): &()) -> Result<(), tcl_cmd_core::CmdError> {
        Ok(())
    }
    fn retain_index_failure(&mut self, _error: &tcl_cmd_core::CmdError) {
        // Pure neutral adapter has no interpreter first-cause owner.
    }
    fn discard_owned_index_result(&mut self, created: Self::Value) {
        drop(created);
    }
}

/// Re-spell unsigned `digits` in base `from` as base `to`, lower case.
fn convert_radix(digits: &str, from: u32, to: u32) -> String {
    if from == to {
        return digits.to_ascii_lowercase();
    }
    let mut number: Vec<u32> = digits.chars().filter_map(|c| c.to_digit(from)).collect();
    let mut out = Vec::new();
    while number.iter().any(|&digit| digit != 0) {
        let mut remainder = 0;
        for digit in &mut number {
            let current = remainder * from + *digit;
            *digit = current / to;
            remainder = current % to;
        }
        out.push(char::from_digit(remainder, to).unwrap_or('0'));
    }
    if out.is_empty() {
        out.push('0');
    }
    out.iter().rev().collect()
}

impl ValueOps for ConstOps<'_> {
    type Value = ConstValue;

    /// The named C release's character model; Jim keeps checked Unicode access.
    fn string_character_model(&self) -> Option<tcl_dialect::StringCharacterModel> {
        self.target
            .profile
            .and_then(tcl_dialect::DialectProfile::character_model)
            .filter(|model| *model != tcl_dialect::StringCharacterModel::Jim084Utf8)
    }

    /// The named C release's container index policy. A target naming no
    /// release hands the cores only indices every C release reads alike
    /// ([`ConstOps::index`] reads them first; a regex `-start` is proved
    /// release-stable), so 9.0's policy, which its numerals and regex core
    /// already default to, reads each as every release does. Jim abstains.
    fn index_syntax(&self) -> Option<tcl_dialect::IndexSyntax> {
        match self.target.release {
            Some(release) => Some(tcl_dialect::IndexSyntax::for_version(release)),
            None if self
                .target
                .profile
                .and_then(DialectProfile::index_syntax)
                .is_some() =>
            {
                None
            }
            None => Some(tcl_dialect::IndexSyntax::for_version(TclVersion::V9_0)),
        }
    }

    /// No host is named at compile time: a reading the host's `long`
    /// decides declines.
    fn reads_a_wide_long(&mut self) -> bool {
        self.poison(DeclineReason::ReleaseAmbiguous(Axis::Platform));
        false
    }

    fn new_str(&mut self, s: &str) -> ConstValue {
        let _ = self.charge_bytes(u64::try_from(s.len()).unwrap_or(u64::MAX));
        ConstValue::text(s)
    }

    /// A computed integer, spelled as the target spells it; an `INT64_MIN`
    /// the target may print as `-8` poisons the run (`Unspellable`).
    fn new_int(&mut self, n: i64) -> ConstValue {
        let _ = self.charge(1);
        if self.target.spell_wide(n).is_none() {
            self.poison(DeclineReason::Unspellable);
        }
        ConstValue::int(n)
    }

    /// A computed double, spelled as the target spells it; a spelling that
    /// would not read back as `f`, or one the module's precision may change,
    /// poisons the run (`Unspellable`).
    fn new_double(&mut self, f: f64) -> ConstValue {
        let _ = self.charge(1);
        let text = self.target.spell_double(f).unwrap_or_else(|| {
            self.poison(DeclineReason::Unspellable);
            format_double(f)
        });
        ConstValue::bytes(text.as_bytes(), Representation::Double)
    }

    fn new_bool(&mut self, b: bool) -> ConstValue {
        let _ = self.charge(1);
        ConstValue::bytes(if b { b"1" } else { b"0" }, Representation::Int)
    }

    /// Tcl and Jim construct the string representation with their own list writer.
    fn new_list(&mut self, items: Vec<ConstValue>) -> ConstValue {
        self.require(Needs::LIST_RENDERING);
        if items.iter().any(|item| item.as_utf8().is_none()) {
            self.poison(DeclineReason::NotText);
        }
        let words: Vec<_> = items.iter().map(|item| item.0.bytes.as_ref()).collect();
        let rendered = self.target.render_list_bytes(&words).unwrap_or_else(|| {
            self.poison(DeclineReason::ReleaseAmbiguous(Axis::ListRendering));
            Vec::new()
        });
        let _ = self.charge_bytes(u64::try_from(rendered.len()).unwrap_or(u64::MAX));
        ConstValue::bytes(&rendered, Representation::List)
    }

    fn try_as_str(&mut self, v: &ConstValue) -> Result<Rc<str>, UnicodeAccessError> {
        std::str::from_utf8(&v.0.bytes)
            .map(Rc::from)
            .map_err(|error| {
                self.poison(DeclineReason::NotText);
                UnicodeAccessError {
                    valid_up_to: error.valid_up_to(),
                    error_len: error.error_len(),
                }
            })
    }

    fn native_char_len(&mut self, v: &ConstValue) -> Result<usize, ValueError> {
        self.require(Needs::CHAR_MODEL);
        let text = self.try_as_str(v)?;
        if let Some(count) = StringCharacterModel::count_for(self.target.character_model, &text) {
            Ok(count)
        } else {
            self.poison(DeclineReason::ReleaseAmbiguous(Axis::CharacterModel));
            Err(ValueError::CharacterModelUnavailable)
        }
    }

    fn as_int(&mut self, v: &ConstValue) -> Result<i64, ValueError> {
        let text = self.try_as_str(v)?;
        match self.parse_number(&text, true) {
            Some(Number::Int(i)) => Ok(i),
            Some(Number::Big { .. }) => Err(ValueError::IntegerOverflow),
            _ => Err(ValueError::NotInteger(text.to_string())),
        }
    }

    fn string_compare_length(&mut self, value: &ConstValue) -> Result<Option<usize>, ValueError> {
        // A larger magnitude needs the release's native int or size getter
        // and target width. Preserve its original completion in that case.
        let limit = i32::try_from(self.as_int(value)?).map_err(|_| ValueError::IntegerOverflow)?;
        Ok(usize::try_from(limit).ok())
    }

    /// The bignum format path's magnitude: a wide integer or the full
    /// digits of a bignum, never narrowed through `i64`.
    fn integer_magnitude(
        &mut self,
        v: &ConstValue,
        radix: tcl_syntax::number::Radix,
        _syntax: NumberSyntax,
    ) -> Result<tcl_syntax::value::IntegerMagnitude, ValueError> {
        let text = self.try_as_str(v)?;
        let (negative, from, digits) = match self.parse_number(&text, true) {
            Some(Number::Int(i)) => (
                i.is_negative(),
                tcl_syntax::number::Radix::Dec,
                i.unsigned_abs().to_string(),
            ),
            Some(Number::Big {
                negative,
                radix,
                digits,
            }) => (negative, radix, digits),
            _ => return Err(ValueError::NotInteger(text.to_string())),
        };
        Ok(tcl_syntax::value::IntegerMagnitude {
            negative,
            digits: convert_radix(&digits, from as u32, radix as u32),
        })
    }

    fn as_double(&mut self, v: &ConstValue) -> Result<f64, ValueError> {
        self.require(Needs::NUMERAL_GRAMMAR);
        let text = self.try_as_str(v)?;
        // If the ingress selected a different numeral grammar, the release's
        // fresh object getter alone does not establish this conversion.
        let release = self
            .target
            .release
            .filter(|release| self.target.numerals == Some(release.number_syntax()));
        literal_double(&text, release).ok_or_else(|| ValueError::NotDouble(text.to_string()))
    }

    /// The numeric booleans only: the boolean words (`yes`, `off`, …) and
    /// their prefixes are a rung no shipped direct evaluator reads yet, so
    /// a text boolean declines rather than answering under a partial
    /// table.
    fn as_bool(&mut self, v: &ConstValue) -> Result<bool, ValueError> {
        let text = self.try_as_str(v)?;
        match self.parse_number(&text, false) {
            Some(Number::Int(i)) => Ok(i != 0),
            Some(Number::Double(f)) => Ok(f != 0.0),
            _ => {
                self.poison(DeclineReason::Unsupported);
                Err(ValueError::NotBoolean(text.to_string()))
            }
        }
    }

    /// `incr`'s arithmetic under the target's integer tower: 8.5 onward
    /// widens past the wide boundary, 8.4 does not compute the sum the
    /// program would print, and a profile naming no release cannot say
    /// which — so an overflow declines everywhere but under a release that
    /// widens.
    fn int_add(
        &mut self,
        a: Option<&ConstValue>,
        b: &ConstValue,
    ) -> Result<ConstValue, ValueError> {
        self.require(Needs::NUMERAL_GRAMMAR | Needs::INT_TOWER);
        let x = match a {
            Some(v) => self.as_int(v)?,
            None => 0,
        };
        let y = self.as_int(b)?;
        let sum = i128::from(x) + i128::from(y);
        if let Ok(fits) = i64::try_from(sum) {
            return Ok(self.new_int(fits));
        }
        match self.target.release {
            Some(release) if release >= TclVersion::V8_5 => self.widened(sum),
            Some(_) => Err(ValueError::IntegerOverflow),
            None => {
                self.poison(DeclineReason::ReleaseAmbiguous(Axis::IntTower));
                Err(ValueError::IntegerOverflow)
            }
        }
    }

    /// The element count: the list parse alone, which renders nothing, so
    /// it reads no release axis; charged per element.
    fn list_len(&mut self, v: &ConstValue) -> Result<usize, ValueError> {
        let text = self.try_as_str(v)?;
        let elements = tcl_syntax::list::split_list_with_syntax(
            &text,
            self.target.grammar.list_parse,
            self.target.grammar.escapes,
        )
        .map_err(|e| ValueError::BadList(e.message().to_owned()))?;
        let _ = self.charge(u64::try_from(elements.len()).unwrap_or(u64::MAX));
        Ok(elements.len())
    }

    fn list_elements(&mut self, v: &ConstValue) -> Result<Vec<ConstValue>, ValueError> {
        self.require(Needs::LIST_RENDERING);
        let text = self.try_as_str(v)?;
        // Bare and quoted elements retain the request's actual escape grammar.
        let elements = tcl_syntax::list::split_list_with_syntax(
            &text,
            self.target.grammar.list_parse,
            self.target.grammar.escapes,
        )
        .map_err(|e| ValueError::BadList(e.message().to_owned()))?;
        let _ = self.charge(u64::try_from(elements.len()).unwrap_or(u64::MAX));
        Ok(elements
            .iter()
            .map(|element| ConstValue::text(element))
            .collect())
    }

    /// The canonical pairs of a dictionary — first-occurrence order, the
    /// last value winning — through the owner of that rule, charged per
    /// pair; an odd-length list is the program's `missing value to go with
    /// key`.
    fn dict_pairs(&mut self, v: &ConstValue) -> DictPairs<ConstValue> {
        self.require(Needs::DICT_ORDER | Needs::LIST_RENDERING);
        let elements = self.list_elements(v)?;
        if elements.len() % 2 != 0 {
            return Err(ValueError::BadList(
                "missing value to go with key".to_owned(),
            ));
        }
        let keys: Vec<Rc<[u8]>> = elements
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| self.as_bytes(&pair[0]))
            .collect();
        let slots = canonical_dict_slots(keys.iter().map(AsRef::as_ref));
        let _ = self.charge(u64::try_from(slots.len()).unwrap_or(u64::MAX));
        Ok(slots
            .into_iter()
            .map(|(key, value)| (elements[key * 2].clone(), elements[value * 2 + 1].clone()))
            .collect())
    }

    /// A dictionary from canonical pairs, rendered canonically, charged per
    /// pair and by its bytes.
    fn new_dict(&mut self, pairs: Vec<(ConstValue, ConstValue)>) -> ConstValue {
        self.require(Needs::DICT_ORDER | Needs::LIST_RENDERING);
        let _ = self.charge(u64::try_from(pairs.len()).unwrap_or(u64::MAX));
        let mut items = Vec::with_capacity(pairs.len() * 2);
        for (key, value) in pairs {
            items.push(key);
            items.push(value);
        }
        let rendered = self.new_list(items);
        ConstValue(ExactValue::constructed(rendered.0.bytes, TclType::Dict))
    }

    fn as_bytes(&mut self, v: &ConstValue) -> Rc<[u8]> {
        Rc::clone(&v.0.bytes)
    }

    fn new_bytes(&mut self, bytes: &[u8]) -> ConstValue {
        let _ = self.charge_bytes(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
        let repr = if std::str::from_utf8(bytes).is_ok() {
            Representation::String
        } else {
            Representation::ByteArray
        };
        ConstValue::bytes(bytes, repr)
    }
}

#[cfg(test)]
mod tests;
