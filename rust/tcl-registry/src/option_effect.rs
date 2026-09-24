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

//! Options with semantic effects — the option-effect descriptor
//! (`docs/design/compiler/registry-consumer-contracts.md` § *Options with
//! semantic effects*).
//!
//! An option row may declare what its presence does to the call: an
//! [`EffectAxis`] value it turns on or off, a role it suppresses, a
//! trailing-operand reservation it changes, or the end of option parsing.
//! Options over one axis form an [`OptionEffectFamily`], which carries where
//! the axis starts and how two of its options combine. The registry derives
//! one answer per call, [`OptionEffects`], by the generic walk
//! [`option_effects`]; every consumer asks it, never an option spelling.
//!
//! Four clients read the descriptor: `subst`'s two switch families (the
//! substitution kinds a call performs, projected by [`substitution_kinds`]),
//! `lsearch`'s match styles (the pattern language of its pattern operand),
//! `regexp`'s `-inline` and `-about` (the layout its argument-role resolver
//! reads), and `switch`'s match modes (the case-list invocation).
//!
//! Three rules hold for every command:
//!
//! - **The option scan boundary is declared.** It ends at the command's
//!   reserved trailing words before the end of the argument list, and a
//!   spelling resolves through the profile-filtered option table, so a
//!   spelling the target release does not have is an invalid call rather
//!   than an invented operand. A literal word that does not begin with `-`
//!   ends the scan normally — the leading-run placement every core command
//!   uses.
//! - **An unreadable call answers every value on.** A computed word where an
//!   option could have been, a spelling the table cannot resolve, a `{*}`
//!   expansion — each makes [`OptionEffects::complete`] false and every axis
//!   value true, because assuming an effect is absent is the answer that
//!   loses a real read.
//! - **Two families over one axis value are alternatives.** A call that uses
//!   options of both cannot be read (`subst -nocommands -variables`); the
//!   error itself is an option relation the analyser reports, not a branch
//!   here.

use tcl_dialect::model::{SpecSurface, SurfaceQuery, surface_admits};

use crate::abbrev::PrefixMatching;
use crate::arg_role::ArgRole;
use crate::hover::OptionSpec;
use crate::invocation_words::{InvocationArguments, InvocationWord};
use crate::patterns::PatternType;
use crate::spec::CaseMatchMode;
use crate::substitution::SubstitutionKinds;

/// What one option does to the call, and the family it belongs to. On
/// [`OptionSpec::effect`], beside `value`, `surface`, and `lifecycle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptionEffect {
    /// What this option does to its axis.
    pub kind: OptionEffectKind,
    /// The family it belongs to — an [`OptionEffectFamily::name`] declared
    /// on the same command or subcommand. Options in one family are mutually
    /// overriding or accumulating by the family's own rule.
    pub family: &'static str,
}

/// The effect an option's presence has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionEffectKind {
    /// Turn this axis value off (`subst -novariables`, and `lsearch -exact`
    /// over the pattern-language axis).
    Disables(EffectAxis),
    /// Turn this axis value on; under a [`FamilyCombine::LastWins`] family
    /// every other value of the family turns off (`subst -variables`,
    /// `lsearch -regexp`, `switch -glob`).
    Selects(EffectAxis),
    /// Suppress a role the command's own layout would otherwise assign:
    /// `regexp -inline` names no match variable, so its trailing words carry
    /// no `VarWrite`.
    SuppressesRole(ArgRole),
    /// Change how many trailing operands the layout reserves after the
    /// options: `regexp -about` needs only the expression, so the two-word
    /// reservation becomes one.
    ReservesTrailingWords(u8),
    /// End option parsing; a following hyphenated word is an operand (`--`,
    /// wherever the command's table has it).
    EndsOptions,
}

impl OptionEffectKind {
    /// The axis value this effect moves, when it moves one.
    #[must_use]
    pub const fn axis(self) -> Option<EffectAxis> {
        match self {
            Self::Disables(axis) | Self::Selects(axis) => Some(axis),
            Self::SuppressesRole(_) | Self::ReservesTrailingWords(_) | Self::EndsOptions => None,
        }
    }

    /// The `.tclspec` spelling of this effect's kind — the first word of an
    /// option row's `-effect` value
    /// (`docs/design/compiler/registry-consumer-contracts.md` § *Options
    /// with semantic effects*). The loader reads the rest of the value (the
    /// axis, the role, or the count) against this word directly, the same
    /// way [`crate::definer::MemberEffect::kind_spelling`] does for a member
    /// row's `-effect`.
    #[must_use]
    pub const fn kind_spelling(self) -> &'static str {
        match self {
            Self::Disables(_) => "disables",
            Self::Selects(_) => "selects",
            Self::SuppressesRole(_) => "suppresses-role",
            Self::ReservesTrailingWords(_) => "reserves-trailing-words",
            Self::EndsOptions => "ends-options",
        }
    }
}

/// The closed catalogue of axes an option may move. One variant per axis,
/// never one per command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectAxis {
    /// One kind of substitution a substituting command performs.
    Substitution(SubstitutionKind),
    /// The language a pattern operand is written in.
    PatternLanguage(PatternType),
    /// Case folding: the value is on when the call compares without regard
    /// to case (`switch -nocase`).
    CaseSensitivity,
    /// The comparison mode a case-list command selects its clause by.
    Selection(CaseMatchMode),
}

impl EffectAxis {
    /// The `.tclspec` axis word — `substitution`, `pattern-language`,
    /// `case-sensitivity`, `selection` — never the value.
    #[must_use]
    pub const fn axis_word(self) -> &'static str {
        match self {
            Self::Substitution(_) => "substitution",
            Self::PatternLanguage(_) => "pattern-language",
            Self::CaseSensitivity => "case-sensitivity",
            Self::Selection(_) => "selection",
        }
    }

    /// The `.tclspec` spelling of this axis's value, when it carries one:
    /// `case-sensitivity` is a bare axis word with nothing after it.
    #[must_use]
    pub fn value_word(self) -> Option<&'static str> {
        match self {
            Self::Substitution(kind) => Some(kind.spelling()),
            Self::PatternLanguage(kind) => Some(kind.as_str()),
            Self::CaseSensitivity => None,
            Self::Selection(mode) => Some(mode.spelling()),
        }
    }

    /// The axis `axis` names, with `value` for the axes that need one, or
    /// `None` for an unknown axis word or a value the axis's own vocabulary
    /// does not have.
    #[must_use]
    pub fn from_words(axis: &str, value: Option<&str>) -> Option<Self> {
        match axis {
            "substitution" => Some(Self::Substitution(SubstitutionKind::from_spelling(value?)?)),
            "pattern-language" => Some(Self::PatternLanguage(PatternType::from_str_tag(value?)?)),
            "case-sensitivity" => Some(Self::CaseSensitivity),
            "selection" => Some(Self::Selection(CaseMatchMode::from_spelling(value?)?)),
            _ => None,
        }
    }
}

/// The three kinds of substitution Tcl's substitution phase distinguishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubstitutionKind {
    /// `\n` and friends become their escape values.
    Backslashes,
    /// `[…]` is evaluated as a script.
    Commands,
    /// `$name` is replaced by the variable's value.
    Variables,
}

impl SubstitutionKind {
    /// Every kind, in `.tclspec` vocabulary order.
    pub const ALL: &'static [Self] = &[Self::Backslashes, Self::Commands, Self::Variables];

    /// The `.tclspec` spelling of this kind — the `substitution` axis's
    /// value word.
    #[must_use]
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Backslashes => "backslashes",
            Self::Commands => "commands",
            Self::Variables => "variables",
        }
    }

    /// The kind `word` spells, or `None` for any other word.
    #[must_use]
    pub fn from_spelling(word: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|kind| kind.spelling() == word)
    }
}

/// A family of options over one axis, declared once per command (or
/// subcommand) on [`crate::CommandSpec::option_effect_families`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptionEffectFamily {
    /// The name its options' [`OptionEffect::family`] cite.
    pub name: &'static str,
    /// Where the axis starts before any option in the family is seen.
    pub base: FamilyBase,
    /// How two options of the family combine.
    pub combine: FamilyCombine,
    /// Releases the family exists at; `None` is every release.
    pub surface: Option<&'static [SpecSurface]>,
}

/// Where a family's axis values start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FamilyBase {
    /// Every axis value is on (`subst`'s negated family).
    AllOn,
    /// Every axis value is off (`subst`'s Tcl 9.1 positive family).
    AllOff,
    /// One named value is on (`lsearch`'s default glob).
    Only(EffectAxis),
}

impl FamilyBase {
    /// The `.tclspec` spelling of this base's kind — `all-on`, `all-off`, or
    /// `only` (which the loader then reads an `{AXIS VALUE}` row after).
    #[must_use]
    pub const fn kind_spelling(self) -> &'static str {
        match self {
            Self::AllOn => "all-on",
            Self::AllOff => "all-off",
            Self::Only(_) => "only",
        }
    }
}

/// How two options of one family combine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FamilyCombine {
    /// Each option applies; the effects accumulate (`subst`'s families).
    Accumulate,
    /// The last option Tcl accepts decides (`lsearch`'s match styles): each
    /// option resets the family's values before it applies.
    LastWins,
}

impl FamilyCombine {
    /// Every combine rule, in `.tclspec` vocabulary order.
    pub const ALL: &'static [Self] = &[Self::Accumulate, Self::LastWins];

    /// The `.tclspec` spelling of this combine rule.
    #[must_use]
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Accumulate => "accumulate",
            Self::LastWins => "last-wins",
        }
    }

    /// The combine rule `word` spells, or `None` for any other word.
    #[must_use]
    pub fn from_spelling(word: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|rule| rule.spelling() == word)
    }
}

/// Where one resolved invocation's option effects come from, beside its
/// option table: the families its options cite, the trailing operands the
/// option scan never reaches, the table's prefix policy, and the release gate
/// an option without its own inherits. Selected by the resolution with the
/// option table, so a subcommand's own families answer for a subcommand call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptionEffectScope<'r> {
    /// The families the options cite.
    pub families: &'r [OptionEffectFamily],
    /// The trailing operands the option scan never reaches.
    pub reserved_trailing_words: usize,
    /// Whether the table resolves unique prefixes.
    pub prefix_matching: PrefixMatching,
    /// The release gate an option declaring none of its own inherits.
    pub parent_surface: Option<&'static [SpecSurface]>,
}

/// The option-effect answer for one call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionEffects {
    /// The resolved state of each axis value the command's families cover,
    /// in the order the families declare them.
    pub axes: Vec<(EffectAxis, bool)>,
    /// Operand shifts the call's options applied, in option order: a
    /// [`OptionEffectKind::SuppressesRole`] option contributes `(role, 0)` —
    /// the layout gives the role no operand — and an
    /// [`OptionEffectKind::ReservesTrailingWords`] option contributes
    /// `(ArgRole::Option, n)` — the option run now reserves `n` trailing
    /// operands.
    pub shifts: Vec<(ArgRole, i8)>,
    /// False when the call could not be read to its end — the
    /// `OptionFacts::complete` fact, which is the only thing that licenses
    /// proving an option *absent*.
    pub complete: bool,
    /// The index of the first argument the option scan did not consume:
    /// the first operand after the options, or where an unreadable word
    /// stopped the scan.
    pub option_end: usize,
}

impl OptionEffects {
    /// The state of one axis value, or `None` when no family covers it.
    #[must_use]
    pub fn value(&self, axis: EffectAxis) -> Option<bool> {
        self.axes
            .iter()
            .find(|(covered, _)| *covered == axis)
            .map(|&(_, on)| on)
    }

    /// Whether an option suppressed `role`.
    #[must_use]
    pub fn suppresses(&self, role: ArgRole) -> bool {
        self.shifts.contains(&(role, 0))
    }

    /// The trailing-operand reservation an option changed the layout to, the
    /// last one applied.
    #[must_use]
    pub fn reserved_trailing_words(&self) -> Option<u8> {
        self.shifts
            .iter()
            .rev()
            .find(|(role, _)| *role == ArgRole::Option)
            .and_then(|&(_, words)| u8::try_from(words).ok())
    }

    /// The call's substitution kinds: each kind's axis value, or on when no
    /// family covers it. An unreadable call answers every kind.
    #[must_use]
    pub fn substitution_kinds(&self) -> SubstitutionKinds {
        let on = |kind| {
            !self.complete
                || self
                    .value(EffectAxis::Substitution(kind))
                    .is_none_or(|on| on)
        };
        SubstitutionKinds {
            backslashes: on(SubstitutionKind::Backslashes),
            commands: on(SubstitutionKind::Commands),
            variables: on(SubstitutionKind::Variables),
        }
    }

    /// The one pattern language the call selects, or `None` for an
    /// unreadable call, a call that selects none, or one no family covers.
    #[must_use]
    pub fn pattern_language(&self) -> Option<PatternType> {
        if !self.complete {
            return None;
        }
        let mut on = self.axes.iter().filter_map(|&(axis, on)| match axis {
            EffectAxis::PatternLanguage(kind) if on => Some(kind),
            _ => None,
        });
        let kind = on.next()?;
        on.next().is_none().then_some(kind)
    }

    /// The one selection mode the call picks, or `None` when it cannot be
    /// read or picks none.
    #[must_use]
    pub fn selection(&self) -> Option<CaseMatchMode> {
        if !self.complete {
            return None;
        }
        let mut on = self.axes.iter().filter_map(|&(axis, on)| match axis {
            EffectAxis::Selection(mode) if on => Some(mode),
            _ => None,
        });
        let mode = on.next()?;
        on.next().is_none().then_some(mode)
    }
}

/// The generic walk over a command's own option table: which axis values the
/// call's options leave on, which operand shifts they apply, and whether the
/// call could be read to its end.
///
/// `spec_options` is the command's (or subcommand's) option table; options
/// and families outside `dialect` are unavailable, so a later release's
/// spelling is an invalid call. `reserved_trailing_words` is the command's
/// `CommandSpec::reserved_trailing_words`: the scan never reaches those
/// final operands. Spellings resolve by exact name or unique prefix; a
/// command whose table is exact-only asks
/// [`option_effects_with`] with its [`PrefixMatching`].
#[must_use]
pub fn option_effects(
    spec_options: &[OptionSpec],
    families: &[OptionEffectFamily],
    args: InvocationArguments<'_>,
    reserved_trailing_words: usize,
    dialect: Option<SurfaceQuery<'_>>,
) -> OptionEffects {
    option_effects_with(
        spec_options,
        families,
        args,
        reserved_trailing_words,
        dialect,
        PrefixMatching::Enabled,
    )
}

/// [`option_effects`] with the table's declared prefix policy.
#[must_use]
pub fn option_effects_with(
    spec_options: &[OptionSpec],
    families: &[OptionEffectFamily],
    args: InvocationArguments<'_>,
    reserved_trailing_words: usize,
    dialect: Option<SurfaceQuery<'_>>,
    prefix_matching: PrefixMatching,
) -> OptionEffects {
    let available: Vec<&OptionSpec> = spec_options
        .iter()
        .filter(|option| option.supports_dialect(dialect, None))
        .collect();
    option_effects_over(
        &available,
        families,
        args,
        reserved_trailing_words,
        dialect,
        prefix_matching,
    )
}

/// The walk over an option table a caller has already filtered for the
/// invocation's profile — the registry's own entry point, which filters
/// through the profile's option availability.
#[must_use]
pub(crate) fn option_effects_over(
    options: &[&OptionSpec],
    families: &[OptionEffectFamily],
    args: InvocationArguments<'_>,
    reserved_trailing_words: usize,
    dialect: Option<SurfaceQuery<'_>>,
    prefix_matching: PrefixMatching,
) -> OptionEffects {
    let families: Vec<&OptionEffectFamily> = families
        .iter()
        .filter(|family| {
            family
                .surface
                .is_none_or(|rows| surface_admits(rows, dialect.as_ref()))
        })
        .collect();
    // An option whose family is not available is not available either.
    let options: Vec<&OptionSpec> = options
        .iter()
        .copied()
        .filter(|option| {
            option.effect.is_none_or(|effect| {
                effect.kind.axis().is_none() || families.iter().any(|f| f.name == effect.family)
            })
        })
        .collect();
    let mut walk = Walk::new(&families, &options);
    let spellings: Vec<&str> = (0..args.len())
        .map(|index| args.literal_at(index).unwrap_or_default())
        .collect();
    let exact_len = args.exact_argv_len();
    let mut reserve = reserved_trailing_words;
    let mut pos = 0usize;
    let readable = loop {
        // The boundary depends on the argv length only while the command
        // reserves trailing operands; an unknown length leaves it unknown.
        let limit = match exact_len {
            Some(len) => len.saturating_sub(reserve),
            None if reserve == 0 => usize::MAX,
            None => break false,
        };
        if pos >= limit {
            break true;
        }
        let Some(word) = args.get(pos) else {
            break true;
        };
        let spelling = match word {
            InvocationWord::Literal(spelling) if spelling.starts_with('-') => spelling,
            InvocationWord::Literal(_) | InvocationWord::DynamicNonOption => break true,
            InvocationWord::Dynamic | InvocationWord::Expanded | InvocationWord::Opaque => {
                break false;
            }
        };
        let Some(option) = crate::patterns::resolve_available_option_prefix_with(
            &options,
            spelling,
            prefix_matching,
        ) else {
            break false;
        };
        let consumed = option.value_word_count(&spellings, pos);
        pos += 1 + consumed;
        let Some(effect) = option.effect else {
            continue;
        };
        match effect.kind {
            OptionEffectKind::EndsOptions => break true,
            OptionEffectKind::ReservesTrailingWords(words) => {
                reserve = usize::from(words);
                walk.shifts
                    .push((ArgRole::Option, i8::try_from(words).unwrap_or(i8::MAX)));
            }
            OptionEffectKind::SuppressesRole(role) => walk.shifts.push((role, 0)),
            OptionEffectKind::Selects(axis) => walk.apply(effect.family, axis, true),
            OptionEffectKind::Disables(axis) => walk.apply(effect.family, axis, false),
        }
    };
    walk.finish(readable, pos)
}

/// The substitution kinds a substituting command performs for one call: the
/// [`OptionEffects::substitution_kinds`] projection, with one rule of its own.
/// Every word before the reserved operands must be an option: a word the
/// option run stopped at before them is neither an option nor an operand —
/// an invalid call, or a computed word a spelling-only caller could not tell
/// apart — so the answer is every kind.
#[must_use]
pub fn substitution_kinds(
    spec_options: &[OptionSpec],
    families: &[OptionEffectFamily],
    args: InvocationArguments<'_>,
    reserved_trailing_words: usize,
    dialect: Option<SurfaceQuery<'_>>,
) -> SubstitutionKinds {
    let effects = option_effects(
        spec_options,
        families,
        args,
        reserved_trailing_words,
        dialect,
    );
    let reaches_operands = args
        .exact_argv_len()
        .is_some_and(|len| effects.option_end + reserved_trailing_words >= len);
    if reaches_operands {
        effects.substitution_kinds()
    } else {
        SubstitutionKinds::ALL
    }
}

/// One family's state during the walk.
struct FamilyState<'f> {
    family: &'f OptionEffectFamily,
    /// The axis values the family covers, in declaration order, with their
    /// current state.
    values: Vec<(EffectAxis, bool)>,
    /// Whether one of the family's options appeared.
    used: bool,
}

struct Walk<'f> {
    families: Vec<FamilyState<'f>>,
    shifts: Vec<(ArgRole, i8)>,
}

impl<'f> Walk<'f> {
    fn new(families: &[&'f OptionEffectFamily], options: &[&OptionSpec]) -> Self {
        let families = families
            .iter()
            .map(|&family| {
                let mut covered: Vec<EffectAxis> = Vec::new();
                if let FamilyBase::Only(axis) = family.base {
                    covered.push(axis);
                }
                for option in options {
                    if let Some(axis) = option
                        .effect
                        .filter(|effect| effect.family == family.name)
                        .and_then(|effect| effect.kind.axis())
                        && !covered.contains(&axis)
                    {
                        covered.push(axis);
                    }
                }
                let values = covered
                    .into_iter()
                    .map(|axis| {
                        let on = match family.base {
                            FamilyBase::AllOn => true,
                            FamilyBase::AllOff => false,
                            FamilyBase::Only(only) => only == axis,
                        };
                        (axis, on)
                    })
                    .collect();
                FamilyState {
                    family,
                    values,
                    used: false,
                }
            })
            .collect();
        Self {
            families,
            shifts: Vec::new(),
        }
    }

    fn apply(&mut self, family: &str, axis: EffectAxis, on: bool) {
        let Some(state) = self
            .families
            .iter_mut()
            .find(|state| state.family.name == family)
        else {
            return;
        };
        state.used = true;
        if state.family.combine == FamilyCombine::LastWins {
            for value in &mut state.values {
                value.1 = false;
            }
        }
        if let Some(value) = state
            .values
            .iter_mut()
            .find(|(covered, _)| *covered == axis)
        {
            value.1 = on;
        }
    }

    fn finish(self, readable: bool, option_end: usize) -> OptionEffects {
        let mut axes: Vec<(EffectAxis, bool)> = Vec::new();
        let mut mixed = false;
        for state in &self.families {
            for &(axis, _) in &state.values {
                if axes.iter().any(|(seen, _)| *seen == axis) {
                    continue;
                }
                let covering: Vec<&FamilyState<'_>> = self
                    .families
                    .iter()
                    .filter(|other| other.values.iter().any(|(covered, _)| *covered == axis))
                    .collect();
                let mut users = covering.iter().filter(|other| other.used);
                let decider = match (users.next(), users.next()) {
                    (Some(_), Some(_)) => {
                        mixed = true;
                        covering[0]
                    }
                    (Some(user), None) => user,
                    // No option of any covering family: the first declared
                    // family's base decides.
                    (None, _) => covering[0],
                };
                let on = decider
                    .values
                    .iter()
                    .find(|(covered, _)| *covered == axis)
                    .is_some_and(|&(_, on)| on);
                axes.push((axis, on));
            }
        }
        let complete = readable && !mixed;
        if !complete {
            for value in &mut axes {
                value.1 = true;
            }
        }
        OptionEffects {
            axes,
            shifts: self.shifts,
            complete,
            option_end,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::invocation_words::InvocationWord;
    use tcl_dialect::model::{Family, SurfaceQuery};

    /// A shipped spec, through the registry every consumer reads.
    fn shipped(name: &str) -> &'static crate::CommandSpec {
        crate::CommandRegistry::build_default()
            .get(name)
            .unwrap_or_else(|| panic!("`{name}` is a shipped command"))
    }

    fn subst() -> &'static crate::CommandSpec {
        shipped("subst")
    }

    /// The kinds `subst` performs for `args`, under every release.
    fn kinds(args: &[&str]) -> SubstitutionKinds {
        let spec = subst();
        substitution_kinds(
            spec.options,
            spec.option_effect_families,
            InvocationArguments::literals(args),
            spec.reserved_trailing_words,
            None,
        )
    }

    fn effects(spec: &crate::CommandSpec, args: &[&str]) -> OptionEffects {
        option_effects_with(
            spec.options,
            spec.option_effect_families,
            InvocationArguments::literals(args),
            spec.reserved_trailing_words,
            None,
            spec.prefix_matching,
        )
    }

    const SUBSTITUTION_ONLY: fn(SubstitutionKind) -> SubstitutionKinds = |kind| SubstitutionKinds {
        backslashes: kind == SubstitutionKind::Backslashes,
        commands: kind == SubstitutionKind::Commands,
        variables: kind == SubstitutionKind::Variables,
    };

    #[test]
    fn tp_no_switches_runs_every_substitution() {
        assert_eq!(kinds(&["hello $name"]), SubstitutionKinds::ALL);
        let spec = subst();
        let answer = effects(spec, &["hello $name"]);
        assert!(answer.complete);
        assert_eq!(answer.option_end, 0);
    }

    #[test]
    fn tp_a_negated_switch_turns_off_only_its_own_kind() {
        assert_eq!(
            kinds(&["-novariables", "hello $name"]),
            SubstitutionKinds {
                variables: false,
                ..SubstitutionKinds::ALL
            }
        );
        assert_eq!(
            kinds(&["-nocommands", "hello $name"]),
            SubstitutionKinds {
                commands: false,
                ..SubstitutionKinds::ALL
            }
        );
        // A switch may repeat, and the negated family accumulates.
        assert_eq!(
            kinds(&["-nocommands", "-nocommands", "-novariables", "x"]),
            SUBSTITUTION_ONLY(SubstitutionKind::Backslashes)
        );
    }

    /// The Tcl 9.1 positive family names the *only* kinds that run.
    #[test]
    fn tp_a_positive_switch_turns_on_only_its_own_kind() {
        assert_eq!(
            kinds(&["-variables", "hello $name"]),
            SUBSTITUTION_ONLY(SubstitutionKind::Variables)
        );
        assert_eq!(
            kinds(&["-backslashes", "-commands", "x"]),
            SubstitutionKinds {
                variables: false,
                ..SubstitutionKinds::ALL
            }
        );
    }

    /// tclsh 9.1b0: `subst -nocommands -variables x` → `cannot combine
    /// positive and negative options`. The mixed call is unreadable, so every
    /// kind is on; the error is the option relation's finding (W147).
    #[test]
    fn fp_the_two_families_mixed_are_unreadable_and_every_kind_is_on() {
        let spec = subst();
        let answer = effects(spec, &["-novariables", "-commands", "x"]);
        assert!(!answer.complete);
        assert!(answer.axes.iter().all(|&(_, on)| on), "{answer:?}");
        assert_eq!(
            kinds(&["-novariables", "-commands", "x"]),
            SubstitutionKinds::ALL
        );
    }

    /// tclsh 8.4–9.1: `subst -no x` → `ambiguous option "-no"`; `subst -novar
    /// {$x}` prints `$x` — a unique prefix resolves, an ambiguous one cannot.
    #[test]
    fn fp_an_abbreviation_the_table_cannot_resolve_is_unreadable() {
        let spec = subst();
        let ambiguous = effects(spec, &["-no", "x"]);
        assert!(!ambiguous.complete);
        assert_eq!(kinds(&["-no", "x"]), SubstitutionKinds::ALL);
        assert_eq!(
            kinds(&["-novar", "x"]),
            SubstitutionKinds {
                variables: false,
                ..SubstitutionKinds::ALL
            }
        );
        // A spelling the target release does not have is not an option there.
        let tcl90 = option_effects(
            spec.options,
            spec.option_effect_families,
            InvocationArguments::literals(&["-variables", "x"]),
            spec.reserved_trailing_words,
            Some(SurfaceQuery::core(Family::Tcl, "9.0")),
        );
        assert!(!tcl90.complete);
    }

    #[test]
    fn fp_a_computed_switch_word_is_unreadable() {
        let spec = subst();
        let words = [InvocationWord::Dynamic, InvocationWord::Literal("x")];
        let answer = option_effects(
            spec.options,
            spec.option_effect_families,
            InvocationArguments::structured(&words),
            spec.reserved_trailing_words,
            None,
        );
        assert!(!answer.complete);
        assert_eq!(answer.substitution_kinds(), SubstitutionKinds::ALL);
        // A spelling-only caller passes the computed word's source text; it
        // is not an option, and every word before the operand must be one.
        assert_eq!(kinds(&["$opt", "x"]), SubstitutionKinds::ALL);
        assert_eq!(kinds(&["-nocommands", "$opt", "x"]), SubstitutionKinds::ALL);
        // An expansion where an option could have been.
        let expanded = [
            InvocationWord::Literal("-nocommands"),
            InvocationWord::Expanded,
        ];
        assert!(
            !option_effects(
                spec.options,
                spec.option_effect_families,
                InvocationArguments::structured(&expanded),
                spec.reserved_trailing_words,
                None,
            )
            .complete
        );
        // No operand at all: nothing is scanned, so nothing narrows.
        assert_eq!(kinds(&[]), SubstitutionKinds::ALL);
        assert_eq!(kinds(&["-nocommands"]), SubstitutionKinds::ALL);
    }

    fn lsearch() -> &'static crate::CommandSpec {
        shipped("lsearch")
    }

    #[test]
    fn lsearch_last_style_wins() {
        let spec = lsearch();
        let language = |args: &[&str]| effects(spec, args).pattern_language();
        assert_eq!(language(&["{a b}", "a*"]), Some(PatternType::Glob));
        assert_eq!(
            language(&["-regexp", "{a b}", "a+"]),
            Some(PatternType::Regex)
        );
        assert_eq!(language(&["-exact", "{a b}", "a+"]), None);
        assert_eq!(language(&["-sorted", "{a b}", "a+"]), None);
        assert_eq!(
            language(&["-regexp", "-glob", "{a b}", "a*"]),
            Some(PatternType::Glob)
        );
        assert_eq!(language(&["-regexp", "-exact", "{a b}", "a+"]), None);
        assert_eq!(
            language(&["-exact", "-regexp", "{a b}", "a+"]),
            Some(PatternType::Regex)
        );
        // A value-taking option consumes its value word, abbreviated or not.
        let answer = effects(spec, &["-sta", "2", "{a b c}", "b*"]);
        assert_eq!(answer.option_end, 2);
        assert_eq!(answer.pattern_language(), Some(PatternType::Glob));
    }

    /// `lsearch -regexp -glob` searches the list `-regexp` for the glob
    /// pattern `-glob`: Tcl scans options only while both operands remain.
    #[test]
    fn lsearch_regexp_glob_scans_no_option_past_the_reserved_words() {
        let spec = lsearch();
        let answer = effects(spec, &["-regexp", "-glob"]);
        assert!(answer.complete);
        assert_eq!(answer.option_end, 0);
        assert_eq!(answer.pattern_language(), Some(PatternType::Glob));
        // The unsupported `--` and an unknown switch are unreadable.
        assert!(!effects(spec, &["--", "{a b}", "a+"]).complete);
        assert!(!effects(spec, &["-unknown", "{a b}", "a+"]).complete);
    }

    fn switch() -> &'static crate::CommandSpec {
        shipped("switch")
    }

    #[test]
    fn switch_dash_dash_ends_options() {
        let spec = switch();
        let answer = effects(spec, &["-glob", "--", "-x", "{a b}"]);
        assert!(answer.complete);
        assert_eq!(answer.option_end, 2);
        assert_eq!(answer.selection(), Some(CaseMatchMode::Glob));
        assert_eq!(answer.value(EffectAxis::CaseSensitivity), Some(false));
        // Without `--` the scan stops at the first non-option word.
        let plain = effects(spec, &["-regexp", "-nocase", "subject", "{a b}"]);
        assert_eq!(plain.option_end, 2);
        assert_eq!(plain.selection(), Some(CaseMatchMode::Regexp));
        assert_eq!(plain.value(EffectAxis::CaseSensitivity), Some(true));
        // With no mode option the family's base decides.
        let default = effects(spec, &["subject", "{a b}"]);
        assert_eq!(default.selection(), Some(CaseMatchMode::Exact));
    }

    #[test]
    fn regexp_layout_effects_are_reported_as_shifts() {
        let spec = shipped("regexp");
        let inline = effects(spec, &["-inline", "a(b)", "ab"]);
        assert!(inline.suppresses(ArgRole::VarWrite));
        assert_eq!(inline.reserved_trailing_words(), None);
        let about = effects(spec, &["-about", "a(b)"]);
        assert!(!about.suppresses(ArgRole::VarWrite));
        assert_eq!(about.reserved_trailing_words(), Some(1));
        // After `--`, `-inline` is the pattern.
        let ended = effects(spec, &["--", "-inline", "s", "v"]);
        assert!(ended.shifts.is_empty());
        assert_eq!(ended.option_end, 1);
    }
}
