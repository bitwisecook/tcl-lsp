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

//! The option table selected for one invocation: prefix occurrences, value roles and the leading option run.

use super::InvocationAvailability;
use super::source_options::{
    AuthoredSourceOption, AuthoredSourceOptionBoundary, AuthoredSourceOptionScan,
};
use crate::arg_role::ArgRole;
use crate::forms::CommandForm;
use crate::hover::OptionSpec;
use crate::invocation_words::InvocationWords;
use crate::spec::{CommandSpec, SubCommand};
use tcl_dialect::model::SpecSurface;

/// Option descriptors applicable to a resolved invocation.
///
/// Command and subcommand options remain separate from form-local options so
/// the resolver remains allocation-free.  Consumers must consult both slices;
/// this mirrors the registry keyword-table construction.
#[derive(Debug, Clone, Copy)]
pub struct InvocationOptions<'r, 'q> {
    /// Ingress-selected option availability.
    pub availability: InvocationAvailability<'q>,
    /// Surface inherited by shared command/subcommand options.
    pub parent_surface: Option<&'static [SpecSurface]>,
    /// Surface inherited by form-local options.
    pub form_surface: Option<&'static [SpecSurface]>,
    /// The selected option table's actual prefix policy.
    pub prefix_matching: crate::abbrev::PrefixMatching,
    /// Where the selected descriptor's options may appear.
    pub placement: crate::spec::OptionPlacement,
    /// Authored constructor or lifecycle operands preceding the option prefix.
    pub positional_prefix_words: usize,
    /// Native operands excluded from the leading option scan.
    pub reserved_trailing_words: usize,
    /// Authored case grammar's conditional optionless layout.
    pub case_list: Option<&'r crate::spec::CaseListSpec>,
    /// Options declared by the command or resolved subcommand.
    pub base: &'r [OptionSpec],
    /// Options declared only by the matched form.
    pub form: &'r [OptionSpec],
}

/// One option or terminator selected in a proved invocation prefix.
#[derive(Debug, Clone)]
pub struct InvocationOptionOccurrence<'r> {
    /// Post-head ordinal within the argument slice supplied to the owner.
    pub argument_index: usize,
    /// Selected descriptor; `None` is the declared `--` terminator.
    pub option: Option<&'r OptionSpec>,
    /// Exact value positions, empty for flags and the terminator.
    pub values: std::ops::Range<usize>,
}

impl<'r> InvocationOptions<'r, '_> {
    /// Keyword vocabulary of the same selected options and prefix policy.
    /// Actual surface, package floor and form inheritance remain on this
    /// metadata; option positions and value widths require `prefix_occurrences`.
    /// This supplies no handler, successful dispatch or rewrite permission.
    #[must_use]
    pub fn keyword_table(self) -> crate::abbrev::KeywordTable<'static> {
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        crate::spec::keyword_table_for_available_options(self.available(), self.prefix_matching)
    }

    /// Project the selected leading option grammar without exposing computed
    /// values as literals. Availability, prefix ambiguity, reserved operands
    /// and value widths share the same owners as argument-role resolution.
    #[must_use]
    pub fn prefix_occurrences(
        self,
        arguments: crate::InvocationArguments<'_>,
    ) -> Option<Vec<InvocationOptionOccurrence<'r>>> {
        let end = self.leading_word_count(arguments)?;
        let options = self.available().collect::<Vec<_>>();
        let mut index = self.positional_prefix_words;
        let mut occurrences = Vec::new();
        while index < end {
            let word = arguments.literal_at(index)?;
            if word == "--" {
                occurrences.push(InvocationOptionOccurrence {
                    argument_index: index,
                    option: None,
                    values: index + 1..index + 1,
                });
                break;
            }
            let option = crate::spec::resolve_available_option_prefix_with(
                &options,
                word,
                self.prefix_matching,
            )?;
            let first = index.checked_add(1)?;
            let next =
                first.checked_add(option.value_word_count_for_arguments(arguments, index)?)?;
            if next > end {
                return None;
            }
            occurrences.push(InvocationOptionOccurrence {
                argument_index: index,
                option: Some(option),
                values: first..next,
            });
            index = next;
        }
        Some(occurrences)
    }

    /// Selected option-value roles, relative to this argument slice. Fixed
    /// value widths preserve positions even when the value bytes are unknown;
    /// unknown option selection or width withdraws the complete projection.
    pub(crate) fn value_roles(
        self,
        arguments: crate::InvocationArguments<'_>,
    ) -> Option<Vec<(usize, ArgRole)>> {
        let options = self.available().collect::<Vec<_>>();
        if !options
            .iter()
            .any(|option| option.value_role().is_some() || option.value_also_role().is_some())
        {
            return Some(Vec::new());
        }
        let end = self.leading_word_count(arguments)?;
        let mut index = self.positional_prefix_words;
        let mut roles = Vec::new();
        while index < end {
            let word = arguments.literal_at(index)?;
            if word == "--" {
                break;
            }
            let option = crate::spec::resolve_available_option_prefix_with(
                &options,
                word,
                self.prefix_matching,
            )?;
            let count = option.value_word_count_for_arguments(arguments, index)?;
            for role in [option.value_role(), option.value_also_role()]
                .into_iter()
                .flatten()
            {
                roles.extend((index + 1..index + 1 + count).map(|argument| (argument, role)));
            }
            index += 1 + count;
        }
        Some(roles)
    }

    fn diagnostic_option(
        self,
        word: &str,
    ) -> Result<&'r OptionSpec, crate::abbrev::KeywordMatch<'static>> {
        if let Some(exact) = self
            .base
            .iter()
            .chain(self.form)
            .find(|option| option.matches(word))
        {
            return Ok(exact);
        }
        let available = self.available().collect::<Vec<_>>();
        let table = crate::spec::keyword_table_for_available_options(
            available.iter().copied(),
            self.prefix_matching,
        );
        match table.resolve(word) {
            crate::abbrev::KeywordMatch::Unique(name) => available
                .into_iter()
                .find(|option| option.name == name)
                .ok_or(crate::abbrev::KeywordMatch::Unknown),
            other => Err(other),
        }
    }

    pub(super) fn diagnostic_scan(
        self,
        arguments: crate::InvocationArguments<'_>,
        offset: usize,
        placement: crate::OptionPlacement,
    ) -> Option<AuthoredSourceOptionScan<'r>> {
        let count = arguments.exact_argv_len()?;
        let reserved = self.reserved_word_count(arguments)?;
        let mut scan = AuthoredSourceOptionScan {
            options: Vec::new(),
            accepts_terminator: self.available().any(|option| option.name == "--"),
            subcommands: Vec::new(),
            boundary: AuthoredSourceOptionBoundary::End,
        };
        let end = count.saturating_sub(reserved);
        let mut index = self.positional_prefix_words;
        while index < end {
            let argument = offset.checked_add(index)?;
            let Some(word) = arguments.literal_at(index) else {
                // A literal prefix other than `-` proves the value cannot be an
                // option, whatever the substitution after it produces.
                if matches!(
                    arguments.get(index),
                    Some(crate::InvocationWord::DynamicNonOption)
                ) {
                    if placement == crate::OptionPlacement::Anywhere {
                        index += 1;
                        continue;
                    }
                    scan.boundary = AuthoredSourceOptionBoundary::Positional(argument);
                    break;
                }
                scan.boundary = AuthoredSourceOptionBoundary::Dynamic(argument);
                break;
            };
            if word == "--" && scan.accepts_terminator {
                scan.boundary = AuthoredSourceOptionBoundary::Terminator(argument);
                break;
            }
            if !word.starts_with('-') || word == "-" {
                if placement == crate::OptionPlacement::Anywhere {
                    index += 1;
                    continue;
                }
                scan.boundary = AuthoredSourceOptionBoundary::Positional(argument);
                break;
            }
            let option = match self.diagnostic_option(word) {
                Ok(option) => option,
                Err(crate::abbrev::KeywordMatch::Ambiguous(candidates)) => {
                    scan.boundary = AuthoredSourceOptionBoundary::Ambiguous {
                        argument,
                        candidates,
                    };
                    break;
                }
                Err(_) => {
                    scan.boundary = AuthoredSourceOptionBoundary::Unknown(argument);
                    break;
                }
            };
            let surface = if self
                .form
                .iter()
                .any(|candidate| std::ptr::eq(candidate, option))
            {
                self.form_surface
            } else {
                self.parent_surface
            };
            let next = option
                .value_word_count_for_arguments(arguments, index)
                .and_then(|width| index.checked_add(1)?.checked_add(width))
                .filter(|next| *next <= end);
            scan.options.push(AuthoredSourceOption {
                argument,
                option,
                available: option.supports_dialect(self.availability.query, surface)
                    && option.available_for_version(self.availability.package_version),
                surface: option.surface.or(surface),
                values: next
                    .and_then(|next| Some(argument.checked_add(1)?..offset.checked_add(next)?)),
            });
            let Some(next) = next else {
                scan.boundary = AuthoredSourceOptionBoundary::Indeterminate;
                break;
            };
            index = next;
        }
        Some(scan)
    }

    fn reserved_word_count(self, arguments: crate::InvocationArguments<'_>) -> Option<usize> {
        match self.case_list {
            Some(case) => case.option_scan_reserved_for_arguments(
                arguments,
                self.availability.query,
                self.reserved_trailing_words,
            ),
            None => Some(self.reserved_trailing_words),
        }
    }

    /// Possible option words under the selected grammar. Unknown operands
    /// branch over admitted option widths; they do not supply values or prove
    /// that any branch reaches a handler. Ordinals are relative to this slice.
    pub(super) fn possible_option_arguments(
        self,
        arguments: crate::InvocationArguments<'_>,
        placement: crate::OptionPlacement,
    ) -> Option<Vec<usize>> {
        let count = arguments.exact_argv_len()?;
        let end = count.saturating_sub(self.reserved_word_count(arguments)?);
        let options = self.available().collect::<Vec<_>>();
        let mut reachable = vec![false; end.checked_add(1)?];
        *reachable.get_mut(self.positional_prefix_words)? = true;
        let mut possible = Vec::new();
        for index in self.positional_prefix_words..end {
            if !reachable[index] {
                continue;
            }
            match arguments.literal_at(index) {
                Some("--") => {}
                Some(word) if !word.starts_with('-') || word == "-" => {
                    if placement == crate::OptionPlacement::Anywhere {
                        reachable[index + 1] = true;
                    }
                }
                Some(word) => {
                    possible.push(index);
                    if let Some(option) = crate::spec::resolve_available_option_prefix_with(
                        &options,
                        word,
                        self.prefix_matching,
                    ) {
                        Self::retain_possible_option_successors(
                            &mut reachable,
                            arguments,
                            index,
                            option,
                        );
                    }
                }
                None => {
                    possible.push(index);
                    for option in options.iter().copied().filter(|option| option.name != "--") {
                        Self::retain_possible_option_successors(
                            &mut reachable,
                            arguments,
                            index,
                            option,
                        );
                    }
                }
            }
        }
        Some(possible)
    }

    fn retain_possible_option_successors(
        reachable: &mut [bool],
        arguments: crate::InvocationArguments<'_>,
        index: usize,
        option: &OptionSpec,
    ) {
        let first = index + 1;
        if let Some(width) = option.value_word_count_for_arguments(arguments, index) {
            if let Some(next) = first
                .checked_add(width)
                .and_then(|next| reachable.get_mut(next))
            {
                *next = true;
            }
        } else {
            // An unresolved authored width can consume any later slot. Keep
            // every remaining position possible rather than guessing a width.
            reachable[first..].fill(true);
        }
    }

    /// Options admitted by the retained ingress surface and package floor.
    /// Shared and form-local rows keep their own inherited surfaces.
    pub fn available(self) -> impl Iterator<Item = &'r OptionSpec> {
        let admits = move |option: &&OptionSpec, parent| {
            option.supports_dialect(self.availability.query, parent)
                && option.available_for_version(self.availability.package_version)
        };
        self.base
            .iter()
            .filter(move |option| admits(option, self.parent_surface))
            .chain(
                self.form
                    .iter()
                    .filter(move |option| admits(option, self.form_surface)),
            )
    }

    /// Whether an option occurs in the proved leading prefix. Unknown prefix
    /// words decline; unknown values of a declared fixed-width option remain
    /// ordinary values and do not change this selection.
    #[must_use]
    pub fn prefix_contains(
        self,
        arguments: crate::InvocationArguments<'_>,
        name: &str,
    ) -> Option<bool> {
        let end = self.leading_word_count(arguments)?;
        let options = self.available().collect::<Vec<_>>();
        let mut found = false;
        let mut index = self.positional_prefix_words;
        while index < end {
            let option = crate::spec::resolve_available_option_prefix_with(
                &options,
                arguments.literal_at(index)?,
                self.prefix_matching,
            )?;
            found |= option.name == name;
            index += 1 + option.value_word_count_for_arguments(arguments, index)?;
        }
        Some(found)
    }

    /// Prove the end of the option prefix after any authored constructor words.
    /// Unknown option words retain uncertainty; fixed option values keep slots.
    #[must_use]
    pub fn leading_word_count(self, arguments: crate::InvocationArguments<'_>) -> Option<usize> {
        let table = self.available().collect::<Vec<_>>();
        if table.is_empty() {
            return Some(0);
        }
        if arguments.exact_argv_len()? < self.positional_prefix_words {
            return None;
        }
        let consumed = crate::spec::leading_option_word_count_for_arguments(
            &table,
            arguments.slice_from(self.positional_prefix_words),
            self.prefix_matching,
            self.reserved_word_count(arguments)?,
        )?;
        self.positional_prefix_words.checked_add(consumed)
    }

    /// Words the option run starting at `index` consumes; zero where that
    /// word is an operand.
    pub(super) fn option_run_at(
        self,
        arguments: crate::InvocationArguments<'_>,
        index: usize,
    ) -> Option<usize> {
        let table = self.available().collect::<Vec<_>>();
        crate::spec::leading_option_word_count_for_arguments(
            &table,
            arguments.slice_from(index),
            self.prefix_matching,
            0,
        )
    }
}

/// Retain the selected option tables and their availability context.
pub(crate) fn invocation_options<'r, 'w>(
    spec: &'r CommandSpec,
    sub: Option<&'r SubCommand>,
    form: Option<&'r CommandForm>,
    availability: InvocationAvailability<'w>,
) -> InvocationOptions<'r, 'w> {
    let parent_surface = sub.and_then(|sub| sub.surface).or(spec.surface);
    InvocationOptions {
        availability,
        parent_surface,
        form_surface: form.and_then(|form| form.surface).or(parent_surface),
        prefix_matching: sub.map_or(spec.prefix_matching, |sub| sub.prefix_matching),
        placement: sub.map_or(spec.option_placement, |sub| sub.option_placement),
        positional_prefix_words: if let Some(sub) = sub {
            sub.option_prefix_words
        } else if spec.option_prefix_words > 0 {
            spec.option_prefix_words
        } else {
            spec.constructor_prefix_words()
                .unwrap_or(match spec.body_execution {
                    Some(crate::body_execution::BodyExecutionSpec::CapturedLifecycle(
                        lifecycle,
                    )) => lifecycle.leading_arguments,
                    _ => 0,
                })
        },
        reserved_trailing_words: sub.map_or(spec.reserved_trailing_words, |sub| {
            sub.reserved_trailing_words
        }),
        case_list: if sub.is_none() { spec.case_list } else { None },
        base: sub.map_or(spec.options, |sub| sub.options),
        form: form.map_or(&[], |form| form.options),
    }
}

pub(super) fn selected_nested_options<'r, 'w>(
    mut options: InvocationOptions<'r, 'w>,
    spec: &'r CommandSpec,
    sub: Option<&'r SubCommand>,
    words: InvocationWords<'_>,
) -> InvocationOptions<'r, 'w> {
    let Some(sub) = sub.filter(|sub| !sub.sub_subcommands.is_empty()) else {
        return options;
    };
    let Some(word) = words.arguments().literal_at(1) else {
        return options;
    };
    let scope = sub.option_scope(
        Some(word),
        options.availability.query,
        options.availability.package_version,
        spec.surface,
    );
    if scope.sub_subcommand.is_some() {
        options.base = scope.options;
        options.parent_surface = scope.surface;
        options.positional_prefix_words = scope.option_prefix_words;
    }
    options
}
