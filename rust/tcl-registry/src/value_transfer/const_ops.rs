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
    ByteStringEncoding, DialectProfile, NumberSyntax, StringCharacterModel, TclVersion,
};
use tcl_syntax::number::{Number, ParseFlags, format_double, parse_whole_with};
use tcl_syntax::value::{DictPairs, ValueError, ValueOps, canonical_dict_slots};

use crate::types::TclType;

use super::answers::{ExactValue, NumericValue, RepresentationEvidence};
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
    /// How an index numeral is read (`index::resolve_opt_with`).
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

/// A compile-time value: byte-exact, with the string rep derived on
/// demand and the representation kept as separate evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstValue {
    /// The value's bytes, exactly.
    pub bytes: Rc<[u8]>,
    /// How the value was constructed.
    pub repr: Representation,
}

impl ConstValue {
    /// A string value.
    #[must_use]
    pub fn text(text: &str) -> Self {
        Self {
            bytes: Rc::from(text.as_bytes()),
            repr: Representation::String,
        }
    }

    /// An integer value, spelled canonically.
    #[must_use]
    pub fn int(value: i64) -> Self {
        Self {
            bytes: Rc::from(value.to_string().as_bytes()),
            repr: Representation::Int,
        }
    }

    /// A value from raw bytes with its representation evidence.
    #[must_use]
    pub fn bytes(bytes: &[u8], repr: Representation) -> Self {
        Self {
            bytes: Rc::from(bytes),
            repr,
        }
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
        Self {
            bytes: Rc::from(value.bytes.as_slice()),
            repr,
        }
    }

    /// The bytes as text, when they are UTF-8.
    #[must_use]
    pub fn as_utf8(&self) -> Option<&str> {
        std::str::from_utf8(&self.bytes).ok()
    }

    /// The exact value: the bytes carry over, the representation becomes
    /// evidence, and the numeric classification is derived from the
    /// spelling — an integer that fits a wide, a double, or the literal
    /// rule for a string. A list, dict, or byte array carries no
    /// classification: its spelling is not a numeral's.
    #[must_use]
    pub fn into_exact(self) -> ExactValue {
        let numeric = match self.repr {
            Representation::Int => self
                .as_utf8()
                .and_then(|s| s.parse::<i64>().ok())
                .map(NumericValue::Int),
            Representation::Double => self
                .as_utf8()
                .and_then(|s| s.parse::<f64>().ok())
                .map(NumericValue::Float),
            Representation::String => self
                .as_utf8()
                .and_then(|s| ExactValue::from_literal(s).numeric),
            Representation::List | Representation::Dict | Representation::ByteArray => None,
        };
        ExactValue {
            bytes: self.bytes.to_vec(),
            numeric,
            representation: RepresentationEvidence::Constructed(self.repr.tcl_type()),
        }
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
    /// Whether the target decodes source as UTF-8 — the 9.x answer, and
    /// the one this front end gives every file. A non-ASCII operand is
    /// admissible only when the target agrees.
    pub source_utf8: bool,
    /// Whether a list's first element that starts with `#` is brace-quoted
    /// when the list is rendered, when one release names it: from 8.5
    /// (`[list # a]` is `{#} a`, so the list is never a comment when it is
    /// evaluated as a script), not in 8.4 (`# a`).
    pub quotes_leading_hash: Option<bool>,
    /// The profile, when the request names one.
    pub profile: Option<&'static DialectProfile>,
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
            profile,
        }
    }

    /// `elements` rendered as one canonical list under this target, or
    /// `None` when the rendering depends on a release the target does not
    /// name: a first element that starts with `#` is brace-quoted from 8.5
    /// (`[list # a]` is `{#} a`) and bare in 8.4 (`# a`). The rule every
    /// consumer that renders a list for a target reads, so a folded list is
    /// the one the target prints.
    #[must_use]
    pub fn render_list<S: AsRef<str>>(&self, elements: &[S]) -> Option<String> {
        let first_hash = elements
            .first()
            .is_some_and(|first| first.as_ref().starts_with('#'));
        let quote_hash = match self.quotes_leading_hash {
            Some(quotes) => quotes,
            None if first_hash => return None,
            None => true,
        };
        let rendered = render_list_bytes(
            elements.iter().map(|element| element.as_ref().as_bytes()),
            quote_hash,
        );
        String::from_utf8(rendered).ok()
    }
}

/// Join `elements` into one canonical list, the first element's leading
/// `#` brace-quoted when `quote_hash` says the target does.
fn render_list_bytes<'e>(elements: impl Iterator<Item = &'e [u8]>, quote_hash: bool) -> Vec<u8> {
    let mut rendered = Vec::new();
    for (index, element) in elements.enumerate() {
        if index != 0 {
            rendered.push(b' ');
        }
        tcl_syntax::list::append_list_element(&mut rendered, element, index == 0 && quote_hash);
    }
    rendered
}

/// The one live compile-time value model. Byte-exact, release-bound,
/// budget-charging, and poisoned by the first fault so no partial answer
/// escapes.
pub struct ConstOps<'ctx> {
    target: TargetSemantics,
    admitted: Needs,
    budget: &'ctx mut Budget,
    fault: Option<DeclineReason>,
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
            target: TargetSemantics::of(ctx.profile),
            admitted: needs,
            budget,
            fault: None,
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

    /// Resolve an index operand under the admitted grammar, returning the
    /// canonical decimal spelling the cores' own `index::resolve` reads
    /// identically in every release. This is what keeps the `010` answer
    /// honest: `string range abcdefghijkl 010 end` is `ijkl` up to 8.6 and
    /// `kl` from 9.0, and calling the core directly would lose that.
    ///
    /// # Errors
    ///
    /// `ReleaseAmbiguous(IndexGrammar)` when the grammars disagree and no
    /// release is named; `WrongRepresentation` for a malformed index,
    /// which the program raises `bad index` on.
    pub fn index(&mut self, spec: &ConstValue, len: usize) -> Result<ConstValue, DeclineReason> {
        self.require(Needs::INDEX_GRAMMAR);
        self.charge(1)?;
        let text = self.text_of(spec)?;
        let resolve =
            |numbers: NumberSyntax| tcl_cmd_core::index::resolve_opt_with(&text, len, numbers);
        let resolved = if let Some(numbers) = self.target.numerals {
            resolve(numbers)
        } else if let Some(answer) = NumberSyntax::unanimous(resolve) {
            answer
        } else {
            let reason = DeclineReason::ReleaseAmbiguous(Axis::IndexGrammar);
            self.poison(reason);
            return Err(reason);
        };
        if let Some(index) = resolved {
            Ok(ConstValue::int(index))
        } else {
            self.poison(DeclineReason::WrongRepresentation);
            Err(DeclineReason::WrongRepresentation)
        }
    }

    /// The decline a failed core call answers with: the run's fault when
    /// one was recorded during the call, else the program's own error —
    /// an error is never a value, and before the completion slice it is a
    /// decline.
    #[must_use]
    pub fn decline(&mut self, _error: &CmdError) -> DeclineReason {
        self.fault.unwrap_or(DeclineReason::WrongRepresentation)
    }

    /// [`Self::decline`] for a seam error.
    #[must_use]
    pub fn decline_value(&mut self, _error: &ValueError) -> DeclineReason {
        self.fault.unwrap_or(DeclineReason::WrongRepresentation)
    }

    /// Close the evaluation: the value, or the first recorded fault. A
    /// poisoned run declines whatever value the core returned, so a
    /// `char_len` that could not decide cannot leak a zero.
    ///
    /// # Errors
    ///
    /// The recorded fault, or the result-bytes bound.
    pub fn take(self, value: ConstValue) -> Result<ExactValue, DeclineReason> {
        if let Some(fault) = self.fault {
            return Err(fault);
        }
        self.budget
            .charge_result(u64::try_from(value.bytes.len()).unwrap_or(u64::MAX))?;
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
                budget.charge_result(u64::try_from(value.bytes.len()).unwrap_or(u64::MAX))?;
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

    /// Whether the rendered list brace-quotes a leading `#` on `first`: the
    /// named release's rule, or — with no release named and a first element
    /// that starts with `#` — a `ReleaseAmbiguous(ListRendering)` fault,
    /// because 8.4 renders `[list # a]` as `# a` and 8.5 onwards as `{#} a`.
    fn quotes_leading_hash(&mut self, first: Option<&ConstValue>) -> bool {
        if let Some(quotes) = self.target.quotes_leading_hash {
            return quotes;
        }
        if first.is_some_and(|value| value.bytes.first() == Some(&b'#')) {
            self.poison(DeclineReason::ReleaseAmbiguous(Axis::ListRendering));
        }
        true
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

impl ValueOps for ConstOps<'_> {
    type Value = ConstValue;

    fn new_str(&mut self, s: &str) -> ConstValue {
        let _ = self.charge_bytes(u64::try_from(s.len()).unwrap_or(u64::MAX));
        ConstValue::text(s)
    }

    fn new_int(&mut self, n: i64) -> ConstValue {
        let _ = self.charge(1);
        ConstValue::int(n)
    }

    fn new_double(&mut self, f: f64) -> ConstValue {
        let _ = self.charge(1);
        ConstValue::bytes(format_double(f).as_bytes(), Representation::Double)
    }

    fn new_bool(&mut self, b: bool) -> ConstValue {
        let _ = self.charge(1);
        ConstValue::bytes(if b { b"1" } else { b"0" }, Representation::Int)
    }

    /// The canonical rendering under the target's list rules: the element
    /// quoting every release shares, and the one rule they do not — whether
    /// a leading `#` on the first element is brace-quoted, which a profile
    /// naming no release cannot say.
    fn new_list(&mut self, items: Vec<ConstValue>) -> ConstValue {
        self.require(Needs::LIST_RENDERING);
        let quote_hash = self.quotes_leading_hash(items.first());
        if items.iter().any(|item| item.as_utf8().is_none()) {
            self.poison(DeclineReason::NotText);
        }
        let rendered = render_list_bytes(items.iter().map(|item| &*item.bytes), quote_hash);
        let _ = self.charge_bytes(u64::try_from(rendered.len()).unwrap_or(u64::MAX));
        ConstValue::bytes(&rendered, Representation::List)
    }

    fn as_str(&mut self, v: &ConstValue) -> Rc<str> {
        if let Some(s) = v.as_utf8() {
            Rc::from(s)
        } else {
            self.poison(DeclineReason::NotText);
            Rc::from(String::from_utf8_lossy(&v.bytes).as_ref())
        }
    }

    fn char_len(&mut self, v: &ConstValue) -> usize {
        self.require(Needs::CHAR_MODEL);
        let text = self.as_str(v);
        if let Some(count) = StringCharacterModel::count_for(self.target.character_model, &text) {
            count
        } else {
            self.poison(DeclineReason::ReleaseAmbiguous(Axis::CharacterModel));
            text.chars().count()
        }
    }

    fn as_int(&mut self, v: &ConstValue) -> Result<i64, ValueError> {
        let text = self.as_str(v);
        match self.parse_number(&text, true) {
            Some(Number::Int(i)) => Ok(i),
            Some(Number::Big { .. }) => Err(ValueError::IntegerOverflow),
            _ => Err(ValueError::NotInteger(text.to_string())),
        }
    }

    /// An integer reads as the double nearest it: its decimal spelling
    /// parsed as a double is correctly rounded, the value a precision-losing
    /// cast gives.
    fn as_double(&mut self, v: &ConstValue) -> Result<f64, ValueError> {
        let text = self.as_str(v);
        match self.parse_number(&text, false) {
            Some(Number::Double(f)) => Ok(f),
            Some(Number::Int(i)) => i
                .to_string()
                .parse::<f64>()
                .map_err(|_| ValueError::NotDouble(text.to_string())),
            _ => Err(ValueError::NotDouble(text.to_string())),
        }
    }

    /// The numeric booleans only: the boolean words (`yes`, `off`, …) and
    /// their prefixes are a rung no shipped direct evaluator reads yet, so
    /// a text boolean declines rather than answering under a partial
    /// table.
    fn as_bool(&mut self, v: &ConstValue) -> Result<bool, ValueError> {
        let text = self.as_str(v);
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
        let text = self.as_str(v);
        let elements = tcl_syntax::list::split_list(&text)
            .map_err(|e| ValueError::BadList(e.message().to_owned()))?;
        let _ = self.charge(u64::try_from(elements.len()).unwrap_or(u64::MAX));
        Ok(elements.len())
    }

    fn list_elements(&mut self, v: &ConstValue) -> Result<Vec<ConstValue>, ValueError> {
        self.require(Needs::LIST_RENDERING);
        let text = self.as_str(v);
        let elements = tcl_syntax::list::split_list(&text)
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
        let keys: Vec<Rc<str>> = elements
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| self.as_str(&pair[0]))
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
        ConstValue {
            bytes: rendered.bytes,
            repr: Representation::Dict,
        }
    }

    fn as_bytes(&mut self, v: &ConstValue) -> Rc<[u8]> {
        Rc::clone(&v.bytes)
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
mod tests {
    use super::*;

    /// The detached context of a catalogue profile by name; `tcl` is the
    /// lenient sink, which declares no release.
    fn context(dialect: Option<&str>) -> AnalysisContext {
        AnalysisContext::detached(dialect.map(|name| {
            if name == "tcl" {
                DialectProfile::plain_tcl()
            } else {
                DialectProfile::find(name).expect("a catalogue profile")
            }
        }))
    }

    fn admit<'b>(dialect: Option<&str>, budget: &'b mut Budget, needs: Needs) -> ConstOps<'b> {
        ConstOps::admit(&context(dialect), budget, needs).expect("admissible")
    }

    #[test]
    fn needs_compose_and_name_their_axes() {
        let needs = Needs::NUMERAL_GRAMMAR | Needs::INT_TOWER;
        assert!(needs.contains(Needs::INT_TOWER));
        assert!(!needs.contains(Needs::CHAR_MODEL));
        assert_eq!(
            needs.axes().collect::<Vec<_>>(),
            vec![Axis::NumeralGrammar, Axis::IntTower]
        );
        assert!(Needs::NONE.is_empty());
    }

    #[test]
    fn the_two_host_axes_never_admit() {
        let mut budget = Budget::evaluation();
        for needs in [Needs::PLATFORM, Needs::WALL_CLOCK] {
            let declined = ConstOps::admit(&context(None), &mut budget, needs).err();
            assert!(
                matches!(
                    declined,
                    Some(DeclineReason::ReleaseAmbiguous(
                        Axis::Platform | Axis::WallClock
                    ))
                ),
                "{needs:?}: {declined:?}"
            );
        }
    }

    #[test]
    fn a_core_call_outside_the_admitted_set_poisons_the_run() {
        let mut budget = Budget::evaluation();
        let mut ops = admit(None, &mut budget, Needs::NONE);
        let _ = ops.as_int(&ConstValue::text("5"));
        assert_eq!(
            ops.take(ConstValue::int(5)),
            Err(DeclineReason::MalformedAnswer)
        );
    }

    /// A leading zero is octal up to 8.6 and decimal from 9.0: `expr {010 +
    /// 0}` is 8 on tclsh 8.4, 8.5 and 8.6 and 10 on 9.0 and 9.1. A dialect
    /// that declares a base release reads under it (ruling 8): `f5-irules`,
    /// `f5-iapps` and `f5-tmsh` on their 8.4-derived engine read 8, as does
    /// `expect` on 8.6 and the Xilinx shell on 8.5, where before the ruling
    /// they declined; a profile declaring no release (the lenient `tcl`
    /// sink, and no profile at all) still declines.
    #[test]
    fn numerals_read_under_the_named_grammar_or_the_unanimous_one() {
        let text = ConstValue::text("010");
        for (dialect, want) in [
            (Some("tcl8.4"), Some(8)),
            (Some("tcl8.6"), Some(8)),
            (Some("tcl9.0"), Some(10)),
            (Some("tcl9.1"), Some(10)),
            (Some("f5-irules"), Some(8)),
            (Some("f5-iapps"), Some(8)),
            (Some("f5-tmsh"), Some(8)),
            (Some("expect"), Some(8)),
            (Some("xilinx-eda-tcl"), Some(8)),
            (Some("bpf"), Some(10)),
        ] {
            let mut budget = Budget::evaluation();
            let mut ops = admit(dialect, &mut budget, Needs::NUMERAL_GRAMMAR);
            assert_eq!(ops.as_int(&text).ok(), want, "{dialect:?}");
            assert!(ops.take(ConstValue::int(0)).is_ok(), "{dialect:?}");
        }
        for dialect in [None, Some("tcl")] {
            let mut budget = Budget::evaluation();
            let mut ops = admit(dialect, &mut budget, Needs::NUMERAL_GRAMMAR);
            assert!(ops.as_int(&text).is_err(), "{dialect:?}");
            assert_eq!(
                ops.take(ConstValue::int(0)),
                Err(DeclineReason::ReleaseAmbiguous(Axis::NumeralGrammar)),
                "{dialect:?}"
            );
            let mut budget = Budget::evaluation();
            let mut ops = admit(dialect, &mut budget, Needs::NUMERAL_GRAMMAR);
            assert_eq!(
                ops.as_int(&ConstValue::text(" 5")).ok(),
                Some(5),
                "{dialect:?}"
            );
            assert_eq!(
                ops.as_int(&ConstValue::text("-7")).ok(),
                Some(-7),
                "{dialect:?}"
            );
            assert!(ops.as_int(&ConstValue::text("2.5")).is_err(), "{dialect:?}");
            assert!(ops.take(ConstValue::int(0)).is_ok(), "{dialect:?}");
        }
    }

    #[test]
    fn the_integer_tower_widens_from_8_5_and_declines_elsewhere() {
        let max = ConstValue::int(i64::MAX);
        let one = ConstValue::int(1);
        for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut budget = Budget::evaluation();
            let mut ops = admit(
                Some(dialect),
                &mut budget,
                Needs::NUMERAL_GRAMMAR | Needs::INT_TOWER,
            );
            let sum = ops.int_add(Some(&max), &one).expect("widens");
            assert_eq!(sum.as_utf8(), Some("9223372036854775808"), "{dialect}");
            let exact = ops.take(sum).expect("exact");
            assert_eq!(
                exact.numeric, None,
                "{dialect}: a bignum has no wide classification"
            );
        }
        let mut budget = Budget::evaluation();
        let mut ops = admit(
            Some("tcl8.4"),
            &mut budget,
            Needs::NUMERAL_GRAMMAR | Needs::INT_TOWER,
        );
        let err = ops
            .int_add(Some(&max), &one)
            .expect_err("8.4 does not widen");
        assert_eq!(ops.decline_value(&err), DeclineReason::WrongRepresentation);
        let mut budget = Budget::evaluation();
        let mut ops = admit(None, &mut budget, Needs::NUMERAL_GRAMMAR | Needs::INT_TOWER);
        let err = ops.int_add(Some(&max), &one).expect_err("no release");
        assert_eq!(
            ops.decline_value(&err),
            DeclineReason::ReleaseAmbiguous(Axis::IntTower)
        );
        let mut budget = Budget::evaluation();
        let mut ops = admit(None, &mut budget, Needs::NUMERAL_GRAMMAR | Needs::INT_TOWER);
        assert_eq!(
            ops.int_add(None, &ConstValue::text("3"))
                .map(|v| v.as_utf8().map(str::to_owned)),
            Ok(Some("3".to_owned()))
        );
    }

    #[test]
    fn an_integer_reads_as_the_nearest_double() {
        let mut budget = Budget::evaluation();
        let mut ops = admit(None, &mut budget, Needs::NUMERAL_GRAMMAR);
        // 2^53 + 1 has no double: the nearest, ties to even, is 2^53.
        assert_eq!(
            ops.as_double(&ConstValue::text("9007199254740993"))
                .map(f64::to_bits),
            Ok(9_007_199_254_740_992_f64.to_bits())
        );
        assert_eq!(
            ops.as_double(&ConstValue::text("-7")).map(f64::to_bits),
            Ok((-7.0_f64).to_bits())
        );
        assert!(ops.take(ConstValue::int(0)).is_ok());
    }

    #[test]
    fn an_index_is_pre_resolved_under_the_admitted_grammar() {
        let spec = ConstValue::text("010");
        for (dialect, want) in [(Some("tcl8.6"), "8"), (Some("tcl9.0"), "10")] {
            let mut budget = Budget::evaluation();
            let mut ops = admit(dialect, &mut budget, Needs::INDEX_GRAMMAR);
            let resolved = ops.index(&spec, 12).expect("resolves");
            assert_eq!(resolved.as_utf8(), Some(want), "{dialect:?}");
        }
        let mut budget = Budget::evaluation();
        let mut ops = admit(None, &mut budget, Needs::INDEX_GRAMMAR);
        assert_eq!(
            ops.index(&spec, 12),
            Err(DeclineReason::ReleaseAmbiguous(Axis::IndexGrammar))
        );
        let mut budget = Budget::evaluation();
        let mut ops = admit(None, &mut budget, Needs::INDEX_GRAMMAR);
        assert_eq!(
            ops.index(&ConstValue::text("end-1"), 12).unwrap().as_utf8(),
            Some("10")
        );
        assert_eq!(
            ops.index(&ConstValue::text("0x2"), 12).unwrap().as_utf8(),
            Some("2")
        );
        assert_eq!(
            ops.index(&ConstValue::text("bogus"), 12),
            Err(DeclineReason::WrongRepresentation)
        );
    }

    #[test]
    fn character_counts_need_a_release_only_off_the_basic_plane() {
        let plain = ConstValue::text("héllo");
        let astral = ConstValue::text("\u{1F600}");
        let mut budget = Budget::evaluation();
        let mut ops = admit(None, &mut budget, Needs::CHAR_MODEL);
        assert_eq!(ops.char_len(&plain), 5);
        assert!(ops.fault().is_none());
        let _ = ops.char_len(&astral);
        assert_eq!(
            ops.take(ConstValue::int(1)),
            Err(DeclineReason::ReleaseAmbiguous(Axis::CharacterModel))
        );
        for (dialect, want) in [("tcl8.6", 2), ("tcl9.0", 1), ("expect", 2)] {
            let mut budget = Budget::evaluation();
            let mut ops = admit(Some(dialect), &mut budget, Needs::CHAR_MODEL);
            assert_eq!(ops.char_len(&astral), want, "{dialect}");
            assert!(ops.take(ConstValue::int(1)).is_ok());
        }
        // The F5 dialects declare a character model of their own
        // (`DialectProfile::character_model`) that their 8.4 base does not
        // share, so the declared divergence blocks the base's answer and the
        // axis is unanimous: the basic plane counts, an astral character
        // declines.
        for dialect in ["f5-irules", "f5-iapps"] {
            let mut budget = Budget::evaluation();
            let mut ops = admit(Some(dialect), &mut budget, Needs::CHAR_MODEL);
            assert_eq!(ops.char_len(&plain), 5, "{dialect}");
            let _ = ops.char_len(&astral);
            assert_eq!(
                ops.take(ConstValue::int(1)),
                Err(DeclineReason::ReleaseAmbiguous(Axis::CharacterModel)),
                "{dialect}"
            );
        }
    }

    /// Ruling 8 in one table: the release each profile evaluates under, and
    /// the axes a declared divergence leaves unanimous.
    #[test]
    fn a_declared_base_release_is_the_release() {
        for (dialect, release) in [
            ("tcl8.4", Some(TclVersion::V8_4)),
            ("tcl9.1", Some(TclVersion::V9_1)),
            ("f5-irules", Some(TclVersion::V8_4)),
            ("f5-iapps", Some(TclVersion::V8_4)),
            ("expect", Some(TclVersion::V8_6)),
            ("cadence-eda-tcl", Some(TclVersion::V8_4)),
            ("intel-quartus-eda-tcl", Some(TclVersion::V8_5)),
            ("synopsys-eda-tcl", Some(TclVersion::V8_6)),
            ("f5-bigip", None),
        ] {
            let target = TargetSemantics::of(DialectProfile::find(dialect));
            assert_eq!(target.release, release, "{dialect}");
            assert_eq!(
                target.numerals,
                release.map(TclVersion::number_syntax),
                "{dialect}"
            );
            assert_eq!(
                target.quotes_leading_hash,
                release.map(|v| v >= TclVersion::V8_5),
                "{dialect}"
            );
        }
        let lenient = TargetSemantics::of(Some(DialectProfile::plain_tcl()));
        assert_eq!(lenient.release, None);
        assert_eq!(lenient.numerals, None);
        let irules = TargetSemantics::of(DialectProfile::find("f5-irules"));
        assert_eq!(
            irules.character_model, None,
            "the F5 character model is declared, and not 8.4's"
        );
        let expect = TargetSemantics::of(DialectProfile::find("expect"));
        assert_eq!(
            expect.character_model,
            Some(StringCharacterModel::Utf16CodeUnits)
        );
    }

    #[test]
    fn non_ascii_text_is_admissible_only_where_source_is_utf8() {
        let value = ConstValue::text("café");
        let mut budget = Budget::evaluation();
        let mut ops = admit(Some("tcl9.0"), &mut budget, Needs::SOURCE_ENCODING);
        assert_eq!(ops.admissible_text(&value).as_deref(), Ok("café"));
        for dialect in [None, Some("tcl8.6")] {
            let mut budget = Budget::evaluation();
            let mut ops = admit(dialect, &mut budget, Needs::SOURCE_ENCODING);
            assert_eq!(
                ops.admissible_text(&value),
                Err(DeclineReason::ReleaseAmbiguous(Axis::SourceEncoding)),
                "{dialect:?}"
            );
            let mut budget = Budget::evaluation();
            let mut ops = admit(dialect, &mut budget, Needs::SOURCE_ENCODING);
            assert_eq!(
                ops.admissible_text(&ConstValue::text("cafe")).as_deref(),
                Ok("cafe"),
                "{dialect:?}"
            );
        }
    }

    #[test]
    fn lists_split_and_render_through_the_owner() {
        let mut budget = Budget::evaluation();
        let mut ops = admit(None, &mut budget, Needs::LIST_RENDERING);
        let items = ops
            .list_elements(&ConstValue::text("a {b c} d"))
            .expect("a list");
        assert_eq!(
            items
                .iter()
                .map(|i| i.as_utf8().unwrap())
                .collect::<Vec<_>>(),
            ["a", "b c", "d"]
        );
        let rendered = ops.new_list(items);
        assert_eq!(rendered.as_utf8(), Some("a {b c} d"));
        assert_eq!(rendered.repr, Representation::List);
        let bad = ops
            .list_elements(&ConstValue::text("{"))
            .expect_err("not a list");
        assert_eq!(bad.message(), "unmatched open brace in list");
    }

    #[test]
    fn bytes_that_are_not_text_decline_rather_than_substitute() {
        let mut budget = Budget::evaluation();
        let mut ops = admit(None, &mut budget, Needs::NONE);
        let raw = ConstValue::bytes(&[0xff, 0xfe], Representation::ByteArray);
        assert_eq!(&*ops.as_bytes(&raw), &[0xff, 0xfe]);
        let _ = ops.as_str(&raw);
        assert_eq!(ops.take(raw), Err(DeclineReason::NotText));
    }

    #[test]
    fn take_classifies_by_representation() {
        let mut budget = Budget::evaluation();
        let ops = admit(None, &mut budget, Needs::NONE);
        let exact = ops.take(ConstValue::text("42")).expect("exact");
        assert_eq!(exact.numeric, Some(NumericValue::Int(42)));
        assert_eq!(
            exact.representation,
            RepresentationEvidence::Constructed(TclType::String)
        );
        let mut budget = Budget::evaluation();
        let ops = admit(None, &mut budget, Needs::LIST_RENDERING);
        let list = ops
            .take(ConstValue::bytes(b"1", Representation::List))
            .expect("exact");
        assert_eq!(list.numeric, None, "a one-element list is not a numeral");
        assert_eq!(
            list.representation,
            RepresentationEvidence::Constructed(TclType::List)
        );
    }

    #[test]
    fn the_budget_bounds_before_allocation() {
        let mut budget = Budget::evaluation();
        budget.allocation_bytes = 4;
        let mut ops = admit(None, &mut budget, Needs::NONE);
        assert!(ops.charge_bytes(3).is_ok());
        assert_eq!(
            ops.charge_bytes(3),
            Err(DeclineReason::Budget(
                super::super::decline::BudgetLimit::AllocationBytes
            ))
        );
        assert!(ops.take(ConstValue::text("x")).is_err());
    }
}
