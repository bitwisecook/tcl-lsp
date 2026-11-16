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

//! Case and clause structure: `switch`-style invocations, clause plans and walks, case bodies and clause-shape checks.

use super::{ResolvedInvocation, SelectedDescriptors};
use crate::invocation_words::InvocationWord;
use crate::spec::{CaseInvocation, InlineCaseClause};

/// One case action operand selected for readonly quoting advice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthoredSourceCaseBody {
    /// Effective post-head operand ordinal.
    pub argument: usize,
    /// Whether the actual selected case grammar uses regular expressions.
    pub regexp: bool,
    /// This action is the single clause-list word rather than an inline body.
    pub single_block: bool,
}

impl ResolvedInvocation<'_, '_> {
    /// The call's clause plan: the effective clause grammar walked over the
    /// words' values after the head (and after the subcommand word), reported
    /// in the invocation's post-head coordinates.
    ///
    /// `None` when no grammar applies or it is unavailable at the
    /// invocation's [`Self::dialect`], when a `{*}` expansion makes the word
    /// count unknown, or when a computed word sits where the walk compares a
    /// keyword, a noise word or the fall-through marker — Tcl decides those by
    /// value. A computed word in a positional slot (`if $cond {…}`) is fine.
    #[must_use]
    pub fn clause_plan(&self) -> Option<crate::clause_grammar::ClausePlan> {
        self.clause_walk()?.ok()
    }

    /// The walk behind [`Self::clause_plan`], saying where it abstained: `Err`
    /// names the first computed word standing where the walk compares one,
    /// with the call read with every computed word matching nothing
    /// ([`crate::clause_grammar::ClauseAbstention`]). `None` exactly where
    /// [`Self::clause_plan`] has no grammar to walk or no argv shape.
    #[must_use]
    pub fn clause_walk(
        &self,
    ) -> Option<Result<crate::clause_grammar::ClausePlan, crate::clause_grammar::ClauseAbstention>>
    {
        let arguments = self.words.arguments();
        let offset = self
            .semantics
            .argument_offset
            .min(arguments.exact_argv_len()?);
        Some(
            self.semantics
                .clause_grammar?
                .walk_arguments(
                    arguments.slice_from(offset),
                    self.semantics.repeated_args,
                    self.dialect,
                )?
                .map(|plan| plan.offset_by(offset))
                .map_err(|abstention| crate::clause_grammar::ClauseAbstention {
                    word: abstention.word + offset,
                    inert: abstention.inert.offset_by(offset),
                }),
        )
    }

    /// The call read as a case list — its subject, clause-list or inline
    /// clauses, and match mode — with the inline clauses when the call writes
    /// them inline: `CaseListSpec::invocation` and `inline_clauses` re-keyed
    /// on the resolution, over the options available at its
    /// [`Self::dialect`].
    ///
    /// `None` when the command declares no case list, the words do not read
    /// as one, or the reading depends on a computed word's value: it must
    /// hold whether each computed word is an operand or a dash word — in the
    /// option run always, and at a clause start where the descriptor declares
    /// per-clause flags (without them a clause starts with its pattern).
    #[must_use]
    pub fn case_invocation(&self) -> Option<(CaseInvocation, Vec<InlineCaseClause>)> {
        let SelectedDescriptors { spec, instance, .. } = self.selected;
        if instance {
            return None;
        }
        let case = spec.case_list?;
        let spellings = self.placeholder_spellings()?;
        let options = self.spec_options();
        let arguments = self.words.arguments();
        let values = (0..arguments.exact_argv_len()?)
            .map(|at| arguments.literal_at(at))
            .collect::<Vec<_>>();
        let invocation = case.invocation_values(&values, &options, self.dialect)?;
        let clauses = match invocation.inline_clause_start {
            Some(start) => case.inline_clauses(&spellings, start)?,
            None => Vec::new(),
        };
        let arguments = self.words.arguments();
        let dashed: Vec<&str> = spellings
            .iter()
            .enumerate()
            .map(|(index, spelling)| match arguments.get(index) {
                Some(InvocationWord::Dynamic) => "-",
                _ => spelling,
            })
            .collect();
        if dashed != spellings {
            if case.invocation(&dashed, &options, self.dialect) != Some(invocation) {
                return None;
            }
            if !case.clause_flags.is_empty()
                && let Some(start) = invocation.inline_clause_start
                && case.inline_clauses(&dashed, start).as_ref() != Some(&clauses)
            {
                return None;
            }
        }
        Some((invocation, clauses))
    }

    /// Case action positions for original-word quoting advice. The selected
    /// outer grammar retains unknown subject/actions; only option and clause
    /// flag selection need values. This does not validate an action value,
    /// enter a script or establish successful matching/completion.
    #[must_use]
    pub fn authored_source_case_body_arguments(&self) -> Option<Vec<AuthoredSourceCaseBody>> {
        if self.semantics.argument_offset != 0 {
            return None;
        }
        let case = *self.semantics.options.case_list?;
        if !case.warn_unbraced_bodies {
            return Some(Vec::new());
        }
        let arguments = self.words.arguments();
        let count = arguments.exact_argv_len()?;
        let values = (0..count)
            .map(|index| arguments.literal_at(index))
            .collect::<Vec<_>>();
        let options = self.semantics.options.available().collect::<Vec<_>>();
        let invocation = case.possible_source_invocation_values(
            &values,
            &options,
            self.semantics.options.availability.query,
        )?;
        let regexp = invocation.mode == crate::spec::CaseMatchMode::Regexp;
        if let Some(argument) = invocation.clause_list_index {
            return Some(vec![AuthoredSourceCaseBody {
                argument,
                regexp,
                single_block: true,
            }]);
        }
        let start = invocation.inline_clause_start?;
        Some(
            case.inline_clause_positions(&values, start)?
                .into_iter()
                .filter_map(|clause| {
                    let argument = clause.body_index?;
                    (case.fallthrough_body != values.get(argument).copied().flatten()).then_some(
                        AuthoredSourceCaseBody {
                            argument,
                            regexp: regexp || clause.mode == crate::spec::CaseMatchMode::Regexp,
                            single_block: false,
                        },
                    )
                })
                .collect(),
        )
    }

    /// Possible case-list layout from this already selected source descriptor.
    /// Dynamic subjects retain their unknown value. Exact cardinality, option
    /// availability and clause grammar use the same descriptor as execution;
    /// this grants no matching, entered body or normal-completion authority.
    #[must_use]
    pub fn authored_source_case_invocation(
        &self,
    ) -> Option<(crate::CaseListSpec, crate::spec::CaseInvocation)> {
        // Implementation contract: naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md

        if self.semantics.argument_offset != 0 {
            return None;
        }
        let case = *self.semantics.options.case_list?;
        let arguments = self.words.arguments();
        let count = arguments.exact_argv_len()?;
        let values = (0..count)
            .map(|index| {
                arguments.literal_at(index).or_else(|| {
                    let bytes = arguments.native_bytes_at(index)?;
                    if bytes.contains(&0) {
                        return None;
                    }
                    std::str::from_utf8(bytes).ok()
                })
            })
            .collect::<Vec<_>>();
        let options = self.semantics.options.available().collect::<Vec<_>>();
        case.possible_invocation_values(
            &values,
            &options,
            self.semantics.options.availability.query,
        )
        .map(|layout| (case, layout))
    }

    /// Conditional clause bodies from the selected case descriptor and the
    /// exact source argv. Counted source bytes are borrowed only when lossless;
    /// option ambiguity remains explicit. Executable roles remain unchanged.
    #[must_use]
    pub fn authored_source_case_clause_projection(
        &self,
        source_values: tcl_syntax::word_rules::WordValueRules,
    ) -> Option<crate::case_bodies::CaseBodyOperands> {
        if self.semantics.argument_offset != 0 {
            return None;
        }
        let case = *self.semantics.options.case_list?;
        let arguments = self.words.arguments();
        let count = arguments.exact_argv_len()?;
        let source = (0..count)
            .map(|argument| match arguments.get(argument)? {
                InvocationWord::KnownBytes(bytes) => {
                    (!bytes.contains(&0)).then_some(())?;
                    Some(InvocationWord::Literal(std::str::from_utf8(bytes).ok()?))
                }
                word => Some(word),
            })
            .collect::<Option<Vec<_>>>()?;
        let source = crate::InvocationArguments::structured(&source);
        let options = self.semantics.options.available().collect::<Vec<_>>();
        case.possible_source_body_operands(
            source,
            &options,
            source_values,
            self.semantics.options.availability.query,
        )
    }

    /// Original case-pattern layout from this already selected descriptor.
    /// Unknown ordinary subjects/actions remain unknown. Exact cardinality,
    /// options and their availability stay shared with the case grammar. The
    /// caller must retain original literal fields before issuing child spans;
    /// this supplies no matching, body entry or successful completion.
    #[must_use]
    pub fn authored_source_case_pattern_layout(
        &self,
    ) -> Option<(crate::CaseListSpec, crate::spec::CaseInvocation)> {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        if self.semantics.argument_offset != 0 {
            return None;
        }
        let case = *self.semantics.options.case_list?;
        let arguments = self.words.arguments();
        let count = arguments.exact_argv_len()?;
        let values = (0..count)
            .map(|argument| arguments.literal_at(argument))
            .collect::<Vec<_>>();
        let options = self.semantics.options.available().collect::<Vec<_>>();
        case.source_invocation_values(&values, &options, self.semantics.options.availability.query)
            .map(|layout| (case, layout))
    }

    /// Incomplete clause-list style from this actual source descriptor and
    /// exact unchanged argv. A complete flag-free odd list may be presented
    /// despite unavailable executable roles; no matching/body/Normal follows.
    #[must_use]
    pub fn authored_source_case_presentation(&self) -> Option<(crate::CaseListSpec, usize)> {
        // Implementation contract: naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md

        if self.semantics.argument_offset != 0 {
            return None;
        }
        let case = *self.semantics.options.case_list?;
        let arguments = self.words.arguments();
        let count = arguments.exact_argv_len()?;
        let values = (0..count)
            .map(|index| {
                arguments.literal_at(index).or_else(|| {
                    let bytes = arguments.native_bytes_at(index)?;
                    (!bytes.contains(&0)).then_some(())?;
                    std::str::from_utf8(bytes).ok()
                })
            })
            .collect::<Vec<_>>();
        let options = self.semantics.options.available().collect::<Vec<_>>();
        case.presentation_clause_list_argument(
            &values,
            &options,
            self.semantics.options.availability.query,
            arguments.dialect()?.word_values,
        )
        .map(|argument| (case, argument))
    }

    /// Clause shape from the selected original grammar. Unknown payloads keep
    /// their positions; unknown selectors and expanded counts do not select a
    /// shape. This is source advice, not native execution admission.
    #[must_use]
    pub fn authored_source_clause_shape(&self) -> Option<crate::ClauseShapeError> {
        self.authored_source_clause_issue()
            .map(crate::ClauseShapeIssue::error)
    }

    /// Clause defect and optional repair anchor from this selected grammar.
    /// A diagnostic-only pack error supplies no borrowed stock edit proposal.
    #[must_use]
    pub fn authored_source_clause_issue(&self) -> Option<crate::ClauseShapeIssue> {
        (self.semantics.script_metadata.command.clause_shape_check?)(self.words.arguments())
    }

    /// Inline case clauses at their original effective ordinals. The selected
    /// case owner validates clauses after the shared option/subject layout;
    /// a computed subject need not be materialised to select static clauses.
    /// Missing literal clause values decline without supplying script entry.
    #[must_use]
    pub fn authored_source_inline_case_clauses(
        &self,
    ) -> Option<Vec<crate::spec::InlineCaseClause>> {
        // naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        let (case, invocation) = self.authored_source_case_invocation()?;
        let start = invocation.inline_clause_start?;
        let arguments = self.words.arguments();
        let count = arguments.exact_argv_len()?;
        let values = (start..count)
            .map(|argument| arguments.literal_at(argument))
            .collect::<Option<Vec<_>>>()?;
        let mut clauses = case.inline_clauses(&values, 0)?;
        for clause in &mut clauses {
            clause.pattern_index = clause.pattern_index.checked_add(start)?;
            clause.body_index = match clause.body_index {
                Some(index) => Some(index.checked_add(start)?),
                None => None,
            };
            for argument in &mut clause.flag_indices {
                *argument = argument.checked_add(start)?;
            }
        }
        Some(clauses)
    }
}
