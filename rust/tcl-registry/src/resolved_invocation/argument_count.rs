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

//! Counting the arguments of an invocation and testing the count against the selected arity.

use super::ResolvedInvocation;
use super::invocation_options::InvocationOptions;
use super::subcommand_resolution::SubcommandResolution;
use crate::arity::Arity;
use crate::spec::{CommandSpec, SubCommand};
use crate::traits::Traits;
use tcl_dialect::model::SurfaceQuery;

/// Count under an authored argument axis, retaining effective operand indices.
/// Unknown expansion contributes no guaranteed entries; this source layout
/// establishes neither a substituted argv value nor successful invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvocationArgumentCount {
    /// Guaranteed contributing operands under the selected count axis.
    pub minimum: u16,
    /// At least one source operand has unknown argv cardinality.
    pub indeterminate: bool,
    /// Effective post-head indices of the guaranteed counted operands.
    pub operands: Vec<usize>,
}

/// Selected original-source signature with its package-version windows.
/// This descriptive shape retains no native handler or completion authority.
#[derive(Debug, Clone)]
pub struct AuthoredSourceArity<'r> {
    /// Already selected owning descriptor, for its actual version axis.
    pub command: &'r CommandSpec,
    /// Selected member when the argument offset consumes a selector.
    pub subcommand: Option<&'r SubCommand>,
    /// Effective fallback signature, including form-specific overrides.
    pub arity: Arity,
    /// Declared version-dependent shapes on the owning package axis.
    pub windows: &'static [crate::arity::ArityWindow],
    /// A bare ensemble whose selected signature may require a selector.
    pub missing_subcommand: bool,
    /// Selected descriptive usage string for reporting.
    pub synopsis: Option<&'static str>,
    /// Count and original effective operand positions for the fallback shape.
    pub count: InvocationArgumentCount,
}

/// Count the effective argv axis without inventing an option grammar. Unknown
/// expansion width retains the exact lower bound and uncertainty; checked
/// conversion refuses a count wider than the shared arity representation.
#[must_use]
pub fn count_invocation_argv(
    arguments: crate::InvocationArguments<'_>,
    offset: usize,
) -> Option<InvocationArgumentCount> {
    arguments.len().checked_sub(offset)?;
    let tail = arguments.slice_from(offset);
    let indices = (0..tail.len())
        .filter(|&index| {
            tail.get(index)
                .is_some_and(crate::InvocationWord::has_exactly_one_argv_entry)
        })
        .collect::<Vec<_>>();
    Some(InvocationArgumentCount {
        minimum: u16::try_from(indices.len()).ok()?,
        indeterminate: indices.len() != tail.len(),
        operands: indices
            .into_iter()
            .map(|index| offset.checked_add(index))
            .collect::<Option<Vec<_>>>()?,
    })
}

/// Assess the same authored count axis and selected leading-option grammar
/// while retaining effective operand indices. Mandatory pre-option data keeps
/// its count even when a call stops before the full prefix is supplied.
#[must_use]
pub fn count_invocation_argument_layout(
    arity: Arity,
    arguments: crate::InvocationArguments<'_>,
    offset: usize,
    options: InvocationOptions<'_, '_>,
) -> Option<InvocationArgumentCount> {
    if arity.count == crate::arity::ArityCount::Arguments {
        return count_invocation_argv(arguments, offset);
    }
    arguments.len().checked_sub(offset)?;
    let arguments = arguments.slice_from(offset);
    let count = arguments.len();
    // An expanded option vector can consume following words as values.
    arguments.exact_argv_len()?;
    let prefix = options.positional_prefix_words.min(count);
    let end = if count <= options.positional_prefix_words {
        count
    } else {
        options.leading_word_count(arguments)?.max(prefix)
    };
    let mut indices = (0..prefix).collect::<Vec<_>>();
    // Options recognised anywhere are not operands wherever they sit, up to
    // an explicit `--`.
    let terminated =
        |last: usize| arguments.get(last).and_then(crate::InvocationWord::literal) == Some("--");
    let mut open = options.placement == crate::spec::OptionPlacement::Anywhere
        && !(end > prefix && terminated(end - 1));
    let mut index = end;
    while index < count {
        if open {
            let run = options.option_run_at(arguments, index)?;
            if run > 0 {
                open = !terminated(index + run - 1);
                index += run;
                continue;
            }
        }
        indices.push(index);
        index += 1;
    }
    let indeterminate = false;
    let minimum = u16::try_from(indices.len()).ok()?;
    let operands = indices
        .into_iter()
        .map(|index| offset.checked_add(index))
        .collect::<Option<Vec<_>>>()?;
    Some(InvocationArgumentCount {
        minimum,
        indeterminate,
        operands,
    })
}

/// Assess one invocation using the authored count axis and shared option walk.
/// This count does not validate option values, effects or native compilation.
#[must_use]
pub fn count_invocation_arguments(
    arity: Arity,
    arguments: crate::InvocationArguments<'_>,
    offset: usize,
    options: InvocationOptions<'_, '_>,
) -> Option<u16> {
    let count = count_invocation_argument_layout(arity, arguments, offset, options)?;
    (!count.indeterminate).then_some(count.minimum)
}

impl<'r> ResolvedInvocation<'r, '_> {
    /// Count the actual frozen argv under its selected signature grammar.
    #[must_use]
    pub fn argument_count_for_arity(&self) -> Option<u16> {
        count_invocation_arguments(
            self.semantics.arity,
            self.words.arguments(),
            self.semantics.argument_offset,
            self.semantics.options,
        )
    }

    /// Whether the selected signature's own forms admitted by `query` accept
    /// this argv, for an invocation whose cardinality varies by release.
    /// Each form recounts the argv under its own arity
    /// ([`Self::authored_source_count_for_arity`]); with no form admitted, the
    /// common arity decides, as in form selection. `None` leaves it undecided:
    /// wherever [`Self::authored_source_arity`] declines (unresolved selector,
    /// structural arity checker), for package arity windows, and for an
    /// indeterminate count.
    #[must_use]
    pub fn arity_accepts_under(&self, query: &SurfaceQuery<'_>) -> Option<bool> {
        let source = self.authored_source_arity()?;
        if !source.windows.is_empty() {
            return None;
        }
        let accepts = |arity: Arity| {
            let count = self.authored_source_count_for_arity(arity)?;
            (!count.indeterminate).then_some(arity.accepts(count.minimum))
        };
        let forms = source
            .subcommand
            .map_or(source.command.command_forms, |sub| sub.subcommand_forms);
        let admitted = forms
            .iter()
            .filter(|form| {
                form.surface
                    .is_none_or(|rows| tcl_dialect::model::surface_admits(rows, Some(query)))
            })
            .collect::<Vec<_>>();
        if admitted.is_empty() {
            return accepts(source.arity);
        }
        let mut decided = Some(false);
        for form in admitted {
            match accepts(form.arity) {
                Some(true) => return Some(true),
                Some(false) => {}
                None => decided = None,
            }
        }
        decided
    }

    /// Selected signature and count axis at authentic effective operands.
    /// Dedicated structural arity checkers and unresolved selectors decline.
    /// Package windows remain explicit for consumers that decide floors later.
    #[must_use]
    pub fn authored_source_arity(&self) -> Option<AuthoredSourceArity<'r>> {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        if !matches!(
            self.subcommand,
            SubcommandResolution::NotApplicable
                | SubcommandResolution::Exact(_)
                | SubcommandResolution::UniquePrefix(_)
        ) || self
            .semantics
            .traits
            .contains(Traits::STRUCTURALLY_CHECKED_ARITY)
        {
            return None;
        }
        let selected = self.semantics.script_metadata;
        let missing_subcommand = !selected.command.subcommands.is_empty()
            && self.words.arguments().is_empty()
            && selected.command.constructor_prefix_words().is_none();
        let windows = selected
            .subcommand
            .map_or(selected.command.arity_windows, |sub| sub.arity_windows);
        let synopsis = selected.subcommand.map_or_else(
            || {
                selected
                    .command
                    .primary_synopsis(self.semantics.options.availability.package_version)
            },
            SubCommand::primary_synopsis,
        );
        let mut arity = self.semantics.arity;
        if selected.subcommand.is_none()
            && let Some((option, alternative)) = selected.command.source_arity_with_option
            && self
                .semantics
                .options
                .prefix_contains(self.words.arguments(), option)?
        {
            arity = alternative;
        }
        Some(AuthoredSourceArity {
            command: selected.command,
            subcommand: selected.subcommand,
            arity,
            windows,
            missing_subcommand,
            synopsis,
            count: self.authored_source_count_for_arity(arity)?,
        })
    }

    /// Recount this same retained invocation under a selected signature window.
    /// The count axis belongs to that window; a cached fallback count cannot
    /// be reused when a version selects a different axis or option layout.
    #[must_use]
    pub fn authored_source_count_for_arity(&self, arity: Arity) -> Option<InvocationArgumentCount> {
        count_invocation_argument_layout(
            arity,
            self.words.arguments(),
            self.semantics.argument_offset,
            self.semantics.options,
        )
    }
}
