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

//! Clause lists of `switch`, `case` and `expect`: matching modes, option scanning and the inline or braced clause layout.

use super::options::resolve_available_option_prefix_with;
use crate::abbrev::PrefixMatching;
use crate::hover::OptionSpec;
use tcl_dialect::model::{SpecSurface, SurfaceQuery, surface_admits};

/// The shape of a `{pattern body …}` clause list (see
/// [`CommandSpec::case_list`]).
///
/// `switch` and Expect's `expect` are the same construct with different
/// spellings: `switch` decides regex-ness once for the whole list (`-regexp`)
/// and takes a subject argument; `expect` decides it per clause (`-re`) and has
/// no subject.  Both have patterns that are keywords rather than match text.
///
/// The command-level options that pick the match mode, fold case, or end the
/// option run are not named here: each is the command's own option row,
/// declaring its [`crate::option_effect::OptionEffect`]
/// (`Selects(Selection(…))`, `Selects(CaseSensitivity)`, `EndsOptions`), so
/// the fact is stated once per option. The descriptor keeps the clause-list
/// *value* shape, which is what makes it a separate field at all.
#[derive(Debug, Clone, Copy)]
pub struct CaseListSpec {
    /// Non-option words between the command's options and the clause list —
    /// `switch`'s subject string is 1; `expect` has none.
    pub subject_args: u8,
    /// Tcl release availability for the two-post-command-word form that
    /// suppresses option scanning. Tcl 8.5 changed `switch` so
    /// `switch -regexp {pattern body}` treats `-regexp` as the subject;
    /// Tcl 8.4 still scans it as an option and rejects the missing subject.
    /// `None` means this case-list descriptor has no such exception.
    pub two_arg_optionless_surface: Option<&'static [SpecSurface]>,
    /// Body word that falls through to the following clause's body.
    pub fallthrough_body: Option<&'static str>,
    /// Value-taking options legal only in regular-expression mode.
    pub value_options_require_regex: &'static [&'static str],
    /// Zero-value command options selecting a specialised comparison mode.
    pub special_match_options: &'static [&'static str],
    /// Flags that may precede a pattern *inside* the list (Expect's `-re`,
    /// `-gl`, `-ex`, `-nocase`, `-timeout`).  Empty means no clause flags.
    pub clause_flags: &'static [&'static str],
    /// The clause flag that makes its pattern a regex (Expect's `-re`).
    pub clause_regex_flag: Option<&'static str>,
    /// Clause flags that consume a following value word (`-timeout 5`).
    pub clause_value_flags: &'static [&'static str],
    /// Clause flag that ends flag parsing, making a following hyphenated word
    /// a pattern rather than an option.
    pub clause_end_options_flag: Option<&'static str>,
    /// Clause flag that makes a following braced word one pattern rather than
    /// a braced clause list (Expect's `-nobrace`).
    pub clause_force_inline_flag: Option<&'static str>,
    /// Exact-only outer-shape flag selecting a braced clause list (`-brace`).
    pub clause_force_list_flag: Option<&'static str>,
    /// Shape required by [`Self::clause_force_list_flag`], when present.
    pub clause_force_list_shape: Option<CaseForceListShape>,
    /// Whether a final pattern may omit its action (Expect permits this).
    pub allow_omitted_final_body: bool,
    /// Patterns that are keywords, not match text (`default`; Expect's
    /// `timeout` / `eof` / `full_buffer`).
    pub keyword_patterns: &'static [&'static str],
    /// Whether a keyword pattern is special only in the final clause.
    pub keyword_patterns_require_final: bool,
    /// Keyword patterns that select a body when ordinary matching selects none.
    /// Event keywords such as Expect timeout do not imply exhaustive selection.
    pub exhaustive_keyword_patterns: &'static [&'static str],
    /// Whether unbraced action bodies carry the `switch` substitution warning.
    /// Expect actions deliberately use a different evaluation model, so its
    /// descriptor leaves this off rather than making a consumer name `switch`.
    pub warn_unbraced_bodies: bool,
    /// A literal word that may sit between the subject and the clause list and
    /// is syntax rather than a pattern — Tcl 8.x's `case string ?in? …`, whose
    /// `Tcl_CaseObjCmd` consumes an `in` at exactly that position
    /// (`generic/tclCmdAH.c`). Recognised only as an exact spelling, because a
    /// substituted word cannot be told apart from a first pattern. `None` for
    /// every descriptor without one.
    pub optional_subject_separator: Option<&'static str>,
    /// The comparison a clause makes when no option selects one: `switch`'s
    /// exact match, and the glob match `case` always makes
    /// (`Tcl_CaseObjCmd` calls `Tcl_StringMatch`) and Expect's patterns
    /// default to.
    pub default_mode: CaseMatchMode,
    /// How the command reads a pattern word: as one pattern, or — `case` —
    /// as a list of patterns when it holds whitespace or a backslash.
    pub pattern_words: PatternWords,
}

/// How a case list reads one pattern word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternWords {
    /// Every pattern word is one pattern: `switch`, Expect.
    Single,
    /// A pattern word holding whitespace or a backslash is a list of
    /// patterns, any of which selects its clause, and one holding neither is
    /// one pattern — `case`'s `Tcl_CaseObjCmd`, which splits such a word with
    /// `Tcl_SplitList` and matches each element.
    Lists,
}

impl PatternWords {
    /// Every reading, in `.tclspec` vocabulary order.
    pub const ALL: &'static [Self] = &[Self::Single, Self::Lists];

    /// The `.tclspec` spelling of this reading.
    #[must_use]
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Lists => "lists",
        }
    }

    /// The reading `word` spells, or `None` for any other word.
    #[must_use]
    pub fn from_spelling(word: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|reading| reading.spelling() == word)
    }
}

/// Registry-owned position/arity shape for an outer clause-list selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseForceListShape {
    /// The selector is the first argument and must have exactly one following
    /// argument containing the clause list.
    FirstArgOnlyRemainder,
}

impl CaseForceListShape {
    fn matches_values(self, args: &[Option<&str>], selector: Option<&str>, index: usize) -> bool {
        match self {
            Self::FirstArgOnlyRemainder => {
                index == 0 && args.len() == 2 && selector == args.first().copied().flatten()
            }
        }
    }
}

/// Registry-owned comparison mode for a case-list invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseMatchMode {
    /// Literal string equality.
    Exact,
    /// Tcl glob matching.
    Glob,
    /// Regular-expression matching; static consumers may abstain.
    Regexp,
    /// A registry-recognised specialised comparison option not represented by
    /// the common string modes (for example Tcl 9.1 `switch -integer`).
    /// Structural consumers may still traverse bodies; evaluators abstain.
    Other,
}

impl CaseMatchMode {
    /// Every mode, in `.tclspec` vocabulary order — the option-effect
    /// descriptor's `selection` axis spelling
    /// (`docs/design/compiler/registry-consumer-contracts.md` § *Options
    /// with semantic effects*).
    pub const ALL: &'static [Self] = &[Self::Exact, Self::Glob, Self::Regexp, Self::Other];

    /// The `.tclspec` spelling of this mode.
    #[must_use]
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Glob => "glob",
            Self::Regexp => "regexp",
            Self::Other => "other",
        }
    }

    /// The mode `word` spells, or `None` for any other word.
    #[must_use]
    pub fn from_spelling(word: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|mode| mode.spelling() == word)
    }
}

/// Validated command-level layout of a case-list invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaseInvocation {
    /// Subject word index, when the command has one.
    pub subject_index: Option<usize>,
    /// Single braced clause-list word, or `None` for inline pairs.
    pub clause_list_index: Option<usize>,
    /// First inline pattern index when not using a clause-list word.
    pub inline_clause_start: Option<usize>,
    /// Comparison mode selected by registry-declared options.
    pub mode: CaseMatchMode,
    /// The canonical spelling of the option that selected
    /// [`CaseMatchMode::Other`] (`-integer`), whether its option row declares
    /// that mode as its effect or it is a
    /// [`CaseListSpec::special_match_options`] entry, else `None`.
    pub special_option: Option<&'static str>,
    /// Whether matching is case-insensitive.
    pub nocase: bool,
}

#[derive(Clone, Copy)]
enum CaseSubjectProjection {
    Definite,
    AfterSelectedOptions,
    Possible,
}

impl CaseSubjectProjection {
    fn retains_unknown_subject(self, selected_prefix_words: usize) -> bool {
        match self {
            Self::Definite => false,
            Self::AfterSelectedOptions => selected_prefix_words > 0,
            Self::Possible => true,
        }
    }
}

#[derive(Default)]
struct CaseOptionScan {
    pub(super) index: usize,
    pub(super) mode: Option<CaseMatchMode>,
    special_option: Option<&'static str>,
    nocase: bool,
    saw_regex_value_option: bool,
    outer_options_ended: bool,
}

impl CaseOptionScan {
    fn select_mode(&mut self, mode: CaseMatchMode) -> Option<()> {
        if self.mode.is_some() {
            return None;
        }
        self.mode = Some(mode);
        Some(())
    }

    fn consume_option(
        &mut self,
        descriptor: &CaseListSpec,
        arguments: crate::InvocationArguments<'_>,
        option: &OptionSpec,
    ) -> Option<()> {
        use crate::option_effect::{EffectAxis, OptionEffectKind};
        let consumed = option.value_word_count_for_arguments(arguments, self.index)?;
        let name = option.name;
        match option.effect.map(|effect| effect.kind) {
            Some(OptionEffectKind::Selects(EffectAxis::Selection(mode))) => {
                self.select_mode(mode)?;
                if mode == CaseMatchMode::Other {
                    self.special_option = Some(name);
                }
            }
            Some(OptionEffectKind::Selects(EffectAxis::CaseSensitivity)) => {
                self.nocase = true;
            }
            Some(OptionEffectKind::EndsOptions) => {
                self.outer_options_ended = true;
            }
            _ => {
                if consumed == 0 {
                    if !descriptor.special_match_options.contains(&name) {
                        return None;
                    }
                    self.select_mode(CaseMatchMode::Other)?;
                    self.special_option = Some(name);
                } else {
                    self.saw_regex_value_option |=
                        descriptor.value_options_require_regex.contains(&name);
                }
            }
        }
        self.index += 1 + consumed;
        Some(())
    }
}

/// A validated inline pattern/action pair.  Indices are post-command-name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineCaseClause {
    /// Leading clause-flag word indices (values of value-taking flags omitted).
    pub flag_indices: Vec<usize>,
    /// Pattern word index in the invocation.
    pub pattern_index: usize,
    /// Action-body word index in the invocation.
    pub body_index: Option<usize>,
    /// Matching mode selected by this clause's leading flags.
    pub mode: CaseMatchMode,
}

impl CaseListSpec {
    /// The `switch … { pat body … }` shape.
    pub const SWITCH: Self = Self {
        subject_args: 1,
        two_arg_optionless_surface: Some(SpecSurface::TCL85_PLUS),
        fallthrough_body: Some("-"),
        value_options_require_regex: &["-matchvar", "-indexvar"],
        // `-integer`'s mode is its option row's `Selects(Selection(Other))`
        // effect, like the other three modes.
        special_match_options: &[],
        clause_flags: &[],
        clause_regex_flag: None,
        clause_value_flags: &[],
        clause_end_options_flag: None,
        clause_force_inline_flag: None,
        clause_force_list_flag: None,
        clause_force_list_shape: None,
        allow_omitted_final_body: false,
        keyword_patterns: &["default"],
        keyword_patterns_require_final: true,
        exhaustive_keyword_patterns: &["default"],
        optional_subject_separator: None,
        warn_unbraced_bodies: true,
        default_mode: CaseMatchMode::Exact,
        pattern_words: PatternWords::Single,
    };

    /// The obsolete Tcl 8.x `case string ?in? { pat body … }` shape.
    ///
    /// `switch`'s ancestor, with none of its options: `Tcl_CaseObjCmd`
    /// (`generic/tclCmdAH.c`) scans no leading flags at all, always matches
    /// with `Tcl_StringMatch` glob semantics, and consumes an exact literal
    /// `in` between the subject and the clauses.  `default` is honoured
    /// wherever it appears rather than only last, and there is no `-`
    /// fall-through body.
    pub const CASE: Self = Self {
        subject_args: 1,
        two_arg_optionless_surface: None,
        fallthrough_body: None,
        value_options_require_regex: &[],
        special_match_options: &[],
        clause_flags: &[],
        clause_regex_flag: None,
        clause_value_flags: &[],
        clause_end_options_flag: None,
        clause_force_inline_flag: None,
        clause_force_list_flag: None,
        clause_force_list_shape: None,
        allow_omitted_final_body: false,
        keyword_patterns: &["default"],
        // `Tcl_CaseObjCmd` records a `default` clause's body as it walks and
        // only falls back to it after every pattern has failed, so a
        // non-final `default` is honoured — unlike `switch`, whose manpage
        // requires it last.
        keyword_patterns_require_final: false,
        exhaustive_keyword_patterns: &["default"],
        optional_subject_separator: Some("in"),
        warn_unbraced_bodies: true,
        default_mode: CaseMatchMode::Glob,
        pattern_words: PatternWords::Lists,
    };

    /// The Expect `expect { ?-flags? pat body … }` shape.
    pub const EXPECT: Self = Self {
        subject_args: 0,
        two_arg_optionless_surface: None,
        fallthrough_body: None,
        // `-timeout` / `-i` are command-level, value-taking options.  In
        // Expect 5.45.4, after either option and its value, one braced word is
        // the final action-less *pattern*, not a clause list: `expect -timeout
        // 5 {ready {action}}` does not execute `action`.  A clause list is
        // only the sole argument (`expect { … }`) or the exact first-argument
        // `-brace { … }` shape. Keep these options out of a speculative
        // list-remainder grammar so every consumer abstains consistently.
        value_options_require_regex: &[],
        special_match_options: &[],
        clause_flags: &[
            "-glob",
            "-regexp",
            "-exact",
            "-notransfer",
            "-nocase",
            "-i",
            "-indices",
            "-iread",
            "-timestamp",
            "-timeout",
            "-nobrace",
            "--",
        ],
        clause_regex_flag: Some("-regexp"),
        clause_value_flags: &["-timeout", "-i"],
        clause_end_options_flag: Some("--"),
        clause_force_inline_flag: Some("-nobrace"),
        clause_force_list_flag: Some("-brace"),
        clause_force_list_shape: Some(CaseForceListShape::FirstArgOnlyRemainder),
        allow_omitted_final_body: true,
        keyword_patterns: &["timeout", "eof", "default", "full_buffer", "null"],
        keyword_patterns_require_final: false,
        exhaustive_keyword_patterns: &[],
        optional_subject_separator: None,
        warn_unbraced_bodies: false,
        default_mode: CaseMatchMode::Glob,
        pattern_words: PatternWords::Single,
    };

    /// Parse and validate this case-list command's option/subject/clause
    /// layout. The two-argument switch exception, `--`, option values, and
    /// pair arity are all registry-owned here.
    #[must_use]
    pub fn invocation(
        self,
        args: &[&str],
        options: &[&crate::hover::OptionSpec],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<CaseInvocation> {
        self.invocation_values(
            &args.iter().copied().map(Some).collect::<Vec<_>>(),
            options,
            dialect,
        )
    }

    /// Readonly operand layout with an unknown subject retained as unknown.
    /// Options and clauses use this descriptor's selected source grammar.
    /// This predicts possible source positions, not matching or execution.
    #[must_use]
    pub fn invocation_values(
        self,
        args: &[Option<&str>],
        options: &[&crate::hover::OptionSpec],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<CaseInvocation> {
        self.invocation_values_with_clause_validation(args, options, dialect, true, false)
    }

    /// Readonly outer source shape with unknown clause-list values retained.
    /// Option selection and exact inline parity remain checked; a sole action
    /// word is not decoded as a value or admitted as an executable script.
    #[must_use]
    pub fn source_invocation_values(
        self,
        args: &[Option<&str>],
        options: &[&crate::hover::OptionSpec],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<CaseInvocation> {
        self.invocation_values_with_clause_validation(args, options, dialect, false, false)
    }

    /// One possible positional subject branch, with the original unknown value
    /// retained. This does not resolve an unreadable option or its width; the
    /// caller must retain a separate unknown selection/error continuation.
    #[must_use]
    pub fn possible_invocation_values(
        self,
        args: &[Option<&str>],
        options: &[&crate::hover::OptionSpec],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<CaseInvocation> {
        self.invocation_values_with_clause_validation(args, options, dialect, true, true)
    }

    /// Possible outer source positions without admitting an action/list value.
    /// This carries no definite option selection or executable body authority.
    #[must_use]
    pub fn possible_source_invocation_values(
        self,
        args: &[Option<&str>],
        options: &[&crate::hover::OptionSpec],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<CaseInvocation> {
        self.invocation_values_with_clause_validation(args, options, dialect, false, true)
    }

    /// Skip a written clause-shape selector at `at`, returning the next word
    /// index and whether a list or an inline selector was written.
    fn skip_clause_selector(
        self,
        args: &[Option<&str>],
        shape: &tcl_syntax::case_list::CaseListShape,
        at: usize,
        outer_options_ended: bool,
    ) -> (usize, bool, bool) {
        // Selectors are descriptor syntax, not ordinary command options.
        // They are considered only while the outer scan is active: after an
        // outer `--`, an option-shaped word is positional data instead.
        let force_list = !outer_options_ended
            && self
                .clause_force_list_shape
                .is_some_and(|shape| shape.matches_values(args, self.clause_force_list_flag, at));
        let force_inline = !outer_options_ended
            && self
                .clause_force_inline_flag
                .is_some_and(|flag| args.get(at).copied().flatten() == Some(flag));
        let skipped = force_list
            || (force_inline
                && args
                    .get(at)
                    .copied()
                    .flatten()
                    .and_then(|word| shape.resolve_flag(word))
                    .is_none());
        // When the inline selector is itself a clause flag (Expect's
        // `-nobrace`), leave it to `inline_clauses` so consumers retain
        // its decorator location. A selector outside that vocabulary is
        // shape-only and is not part of the first clause.
        (at + usize::from(skipped), force_list, force_inline)
    }

    fn invocation_values_with_clause_validation(
        self,
        args: &[Option<&str>],
        options: &[&crate::hover::OptionSpec],
        dialect: Option<SurfaceQuery<'_>>,
        validate_clause_list: bool,
        possible_subject: bool,
    ) -> Option<CaseInvocation> {
        let shape = tcl_syntax::case_list::CaseListShape {
            clause_flags: self.clause_flags,
            clause_value_flags: self.clause_value_flags,
        };
        let per_clause_flags = self.subject_args == 0 && !self.clause_flags.is_empty();
        let sole_clause_list = per_clause_flags && args.len() == 1;
        let scan = self.scan_invocation_options(
            args,
            options,
            dialect,
            &shape,
            sole_clause_list,
            if possible_subject {
                CaseSubjectProjection::Possible
            } else if validate_clause_list {
                CaseSubjectProjection::Definite
            } else {
                CaseSubjectProjection::AfterSelectedOptions
            },
        )?;
        let mode = scan.mode.unwrap_or(self.default_mode);
        let special_option = scan.special_option;
        let nocase = scan.nocase;
        let (mut i, force_list, force_inline) =
            self.skip_clause_selector(args, &shape, scan.index, scan.outer_options_ended);
        let subject_index = (self.subject_args == 1).then_some(i);
        i += usize::from(self.subject_args);
        // A descriptor-declared literal separator between the subject and the
        // clauses (Tcl 8.x `case string ?in? …`).  Matched exactly, mirroring
        // `Tcl_CaseObjCmd`'s own `strcmp(arg, "in")`: a substituted word at
        // this position is a first pattern, not the separator.
        if self
            .optional_subject_separator
            .is_some_and(|sep| args.get(i).copied().flatten() == Some(sep))
        {
            i += 1;
        }
        if i >= args.len() {
            return None;
        }
        let remaining = args.len() - i;
        if force_list {
            if remaining != 1 || validate_clause_list && !self.valid_clause_list(args[i]?) {
                return None;
            }
            Some(CaseInvocation {
                subject_index: None,
                clause_list_index: Some(i),
                inline_clause_start: None,
                mode,
                special_option,
                nocase,
            })
        } else if remaining == 1 && !force_inline && (sole_clause_list || self.subject_args == 1) {
            // Validate the list grammar first, then its registry-declared
            // clause grammar.  Strict pattern/body parity is sufficient for
            // `switch`, but not for Expect: a clause may start with `-re` or
            // a value-taking `-timeout 5`, so counting raw list elements
            // rejects valid clauses and leaves generic consumers unable to
            // recurse their bodies.
            if validate_clause_list && !self.valid_clause_list(args[i]?) {
                return None;
            }
            Some(CaseInvocation {
                subject_index,
                clause_list_index: Some(i),
                inline_clause_start: None,
                mode,
                special_option,
                nocase,
            })
        } else if per_clause_flags {
            // Expect's flags belong to the following pattern, not to the
            // command as a whole.  Parse them from this descriptor before
            // pairing bodies, including unique flag abbreviations.
            self.inline_clauses(&args[i..].iter().copied().collect::<Option<Vec<_>>>()?, 0)?;
            Some(CaseInvocation {
                subject_index,
                clause_list_index: None,
                inline_clause_start: Some(i),
                mode,
                special_option,
                nocase,
            })
        } else if remaining >= 2
            && remaining.is_multiple_of(2)
            && self.fallthrough_body != args.last().copied().flatten()
        {
            Some(CaseInvocation {
                subject_index,
                clause_list_index: None,
                inline_clause_start: Some(i),
                mode,
                special_option,
                nocase,
            })
        } else {
            None
        }
    }

    fn scan_invocation_options(
        self,
        args: &[Option<&str>],
        options: &[&crate::hover::OptionSpec],
        dialect: Option<SurfaceQuery<'_>>,
        shape: &tcl_syntax::case_list::CaseListShape,
        sole_clause_list: bool,
        subject_projection: CaseSubjectProjection,
    ) -> Option<CaseOptionScan> {
        let mut scan = CaseOptionScan::default();
        // A sole subject-less clause-list word owns its own clause flags.
        // The selected two-word switch exception keeps option-like subjects.
        let values = args
            .iter()
            .map(|word| {
                word.map_or(
                    crate::InvocationWord::Dynamic,
                    crate::InvocationWord::Literal,
                )
            })
            .collect::<Vec<_>>();
        let two_arg_optionless = self.option_scan_reserved_for_arguments(
            crate::InvocationArguments::structured(&values),
            dialect,
            2,
        ) == Some(2);
        if !(sole_clause_list || self.subject_args == 1 && args.len() == 2 && two_arg_optionless) {
            while let Some(word) = args.get(scan.index).copied().flatten() {
                if !word.starts_with('-') {
                    break;
                }
                let option = Self::resolve_option(options, word);
                // Clause flags and exact outer-shape selectors retain the
                // first inline clause; value-taking command options stay outer.
                let clause_flag = shape.resolve_flag(word);
                let force_selector = self.clause_force_inline_flag == Some(word)
                    || self.clause_force_list_shape.is_some_and(|shape| {
                        shape.matches_values(args, self.clause_force_list_flag, scan.index)
                    });
                if self.subject_args == 0
                    && (force_selector
                        || clause_flag.is_some_and(|flag| {
                            !shape.flag_takes_value(flag)
                                || !option.is_some_and(OptionSpec::takes_value)
                        }))
                {
                    break;
                }
                scan.consume_option(
                    &self,
                    crate::InvocationArguments::structured(&values),
                    option?,
                )?;
                if scan.outer_options_ended {
                    break;
                }
            }
            let readable_boundary = scan.outer_options_ended
                || options.is_empty()
                || args.get(scan.index) != Some(&None)
                || (self.subject_args == 1
                    && subject_projection.retains_unknown_subject(scan.index));
            if !readable_boundary {
                // An unreadable active candidate can still select an option;
                // it does not establish the positional subject boundary.
                // Source shape after a selected prefix and possible-body
                // queries may retain a positional branch, without reading
                // this value as an option or assigning it an option width.
                return None;
            }
        }
        if scan.saw_regex_value_option
            && scan.mode.unwrap_or(self.default_mode) != CaseMatchMode::Regexp
            || scan.mode.unwrap_or(self.default_mode) == CaseMatchMode::Other && scan.nocase
        {
            return None;
        }
        Some(scan)
    }

    /// Whether the descriptor's two-word optionless form may assign a body
    /// role for `dialect`. A non-option subject remains valid in every
    /// release; the gate only applies when Tcl 8.4 would scan an option-like
    /// first word and consume it.
    #[must_use]
    pub(crate) fn two_arg_body_roles_allowed(
        self,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> bool {
        self.two_arg_optionless_form_is_available(args, dialect)
    }

    fn two_arg_optionless_form_is_available(
        self,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> bool {
        args.len() != 2
            || self.subject_args != 1
            || args.first().is_none_or(|word| !word.starts_with('-'))
            || self
                .two_arg_optionless_surface
                .is_none_or(|available| surface_admits(available, dialect.as_ref()))
    }

    /// Adjust a command's static trailing-word reservation for the
    /// optionless two-word form. The regular switch shape reserves its
    /// subject and clause-list words from option scanning, but Tcl 8.4 still
    /// scans both words when that shape begins with an option-like subject.
    #[must_use]
    pub(crate) fn option_scan_reserved_trailing_words(
        self,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
        default: usize,
    ) -> usize {
        if self.two_arg_body_roles_allowed(args, dialect) {
            default
        } else {
            0
        }
    }

    /// Retain the native optionless case layout for frozen typed argv.
    /// A dynamic subject in a release that scans it remains unresolved.
    #[must_use]
    pub(crate) fn option_scan_reserved_for_arguments(
        self,
        arguments: crate::InvocationArguments<'_>,
        dialect: Option<SurfaceQuery<'_>>,
        default: usize,
    ) -> Option<usize> {
        if arguments.exact_argv_len()? != 2 || self.subject_args != 1 {
            return Some(default);
        }
        if arguments
            .literal_at(0)
            .is_some_and(|subject| !subject.starts_with('-'))
        {
            return Some(default);
        }
        let Some(available) = self.two_arg_optionless_surface else {
            return Some(default);
        };
        let query = dialect?;
        if surface_admits(available, Some(&query)) {
            return Some(default);
        }
        arguments.literal_at(0).map(|_| 0)
    }

    /// Resolve one outer option exactly or by its unique declared prefix.
    ///
    /// The result is an `OptionSpec`, rather than just its spelling, because
    /// aliases and canonical names are one option for ambiguity purposes.
    /// That makes descriptor consumers agree with the registry's usual
    /// `KeywordTable` option semantics without treating two aliases of one
    /// option as two ambiguous candidates.
    fn resolve_option<'a>(options: &'a [&'a OptionSpec], word: &str) -> Option<&'a OptionSpec> {
        resolve_available_option_prefix_with(options, word, PrefixMatching::Enabled)
    }

    fn valid_clause_list(self, text: &str) -> bool {
        let Ok(elements) = tcl_syntax::list::split_list(text) else {
            return false;
        };
        // Tcl's case-list commands require at least one pattern clause.  An
        // empty value is a syntactically valid Tcl list, but not a valid
        // `switch`/Expect case list (Tcl reports wrong # args rather than
        // evaluating a zero-clause dispatch).  Keep that distinction in the
        // shared descriptor boundary so every semantic consumer abstains;
        // formatter recovery can still present an incomplete list locally.
        if elements.is_empty() {
            return false;
        }
        let shape = tcl_syntax::case_list::CaseListShape {
            clause_flags: self.clause_flags,
            clause_value_flags: self.clause_value_flags,
        };
        let clauses = tcl_syntax::case_list::split_case_list(text, &shape);
        !clauses.iter().enumerate().any(|(index, clause)| {
            !clause.valid
                || clause.pattern.is_none()
                || (clause.body.is_none()
                    && !(self.allow_omitted_final_body && index + 1 == clauses.len()))
        }) && !elements.last().is_some_and(|body| {
            self.fallthrough_body
                .is_some_and(|marker| body.as_ref() == marker)
        })
    }

    /// Presentation position for a complete flag-free list with a dangling
    /// pattern. Exact outer options/subject grammar is shared with invocation
    /// selection, but no successful match, body or executable roles follow.
    #[must_use]
    pub fn presentation_clause_list_argument(
        self,
        args: &[Option<&str>],
        options: &[&crate::hover::OptionSpec],
        dialect: Option<SurfaceQuery<'_>>,
        rules: tcl_syntax::word_rules::WordValueRules,
    ) -> Option<usize> {
        // Implementation contract: naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md

        if !self.clause_flags.is_empty() || !self.clause_value_flags.is_empty() {
            return None;
        }
        let layout =
            self.invocation_values_with_clause_validation(args, options, dialect, false, false)?;
        let argument = layout.clause_list_index?;
        let elements = rules
            .split_list(args.get(argument).copied().flatten()?)
            .ok()?;
        (!elements.is_empty() && !elements.len().is_multiple_of(2)).then_some(argument)
    }

    /// Parse descriptor-declared inline pattern/action clauses.  A future
    /// case-list command gets the same body locations by changing only its
    /// descriptor; consumers never infer flag or value arity themselves.
    #[must_use]
    pub fn inline_clauses(self, args: &[&str], start: usize) -> Option<Vec<InlineCaseClause>> {
        self.inline_clause_positions(&args.iter().copied().map(Some).collect::<Vec<_>>(), start)
    }

    /// Inline clause ordinals with unknown payload values retained. Only words
    /// that can choose a clause flag require a value; unknown flag selection
    /// declines instead of guessing where its pattern or action begins.
    #[must_use]
    pub fn inline_clause_positions(
        self,
        args: &[Option<&str>],
        start: usize,
    ) -> Option<Vec<InlineCaseClause>> {
        let shape = tcl_syntax::case_list::CaseListShape {
            clause_flags: self.clause_flags,
            clause_value_flags: self.clause_value_flags,
        };
        let mut clauses = Vec::new();
        let mut i = start;
        while i < args.len() {
            let mut mode = CaseMatchMode::Exact;
            let mut options_ended = false;
            let mut flag_indices = Vec::new();
            while !self.clause_flags.is_empty() && !options_ended && i < args.len() {
                let word = args[i]?;
                if !word.starts_with('-') {
                    break;
                }
                let flag = shape.resolve_flag(word)?;
                flag_indices.push(i);
                i += 1;
                if self.clause_end_options_flag == Some(flag) {
                    options_ended = true;
                    continue;
                }
                if shape.flag_takes_value(flag) {
                    args.get(i)?;
                    i += 1;
                }
                if self.clause_regex_flag == Some(flag) {
                    mode = CaseMatchMode::Regexp;
                }
            }
            let pattern_index = i;
            args.get(pattern_index)?;
            let body_index = pattern_index.checked_add(1)?;
            let body_index = args.get(body_index).is_some().then_some(body_index);
            if body_index.is_none() && !self.allow_omitted_final_body {
                return None;
            }
            clauses.push(InlineCaseClause {
                flag_indices,
                pattern_index,
                body_index,
                mode,
            });
            let Some(body_index) = body_index else { break };
            i = body_index.checked_add(1)?;
        }
        (!clauses.is_empty()).then_some(clauses)
    }

    /// Whether `pattern` is a special keyword at this clause position.
    #[must_use]
    pub fn is_keyword_pattern(self, pattern: &str, index: usize, total: usize) -> bool {
        self.keyword_patterns.contains(&pattern)
            && (!self.keyword_patterns_require_final || index + 1 == total)
    }

    /// Whether the command reads the pattern word whose value is `pattern`
    /// as a list of patterns rather than as one: only under a descriptor
    /// whose pattern words may be lists ([`PatternWords::Lists`]), by the
    /// shared `case` core's rule (`tcl_cmd_core::case::splits_as_list`).
    #[must_use]
    pub fn pattern_is_list(self, pattern: &str) -> bool {
        self.pattern_words == PatternWords::Lists && tcl_cmd_core::case::splits_as_list(pattern)
    }
}
