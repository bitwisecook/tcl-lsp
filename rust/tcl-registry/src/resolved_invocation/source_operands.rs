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

//! Value operands in the authored source: list, pattern, format, expression, variable-name, assignment, append, index and path arguments.

use super::ResolvedInvocation;
use super::argument_count::{InvocationArgumentCount, count_invocation_argument_layout};
use super::subcommand_resolution::SubcommandResolution;
use crate::arg_role::ArgRole;
use crate::arity::Arity;
use crate::traits::Traits;

/// One selected source-declaration local and its optional preceding read slot.
/// These argument positions grant no frame selection, variable link or store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthoredSourceScopeAliasArgument {
    /// Effective post-head argument naming the local declaration.
    pub local: usize,
    /// Adjacent selected `VarRead` or `VarCell` ordinal (an `upvar`'s
    /// `otherVar`), without an alias-target grant.
    pub preceding_read: Option<usize>,
}

/// Lambda-literal source position and guaranteed trailing argument geometry.
/// This is descriptive call syntax, not an entered lambda or parameter binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredSourceLambdaCall {
    /// Effective operand containing the selected lambda-list contract.
    pub lambda_argument: usize,
    /// Guaranteed trailing arguments, with expansion uncertainty retained.
    pub count: InvocationArgumentCount,
}

/// Expression positions in the effective source argv and the selected
/// concatenation grammar. These are readonly source roles, not runtime values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredSourceExpressionArguments {
    /// Effective post-head operand ordinals.
    pub arguments: Vec<usize>,
    /// The selected grammar concatenates its entire post-head argument tail.
    pub concatenates: bool,
}

/// Value ordinals of the selected append source contract. The variable and
/// payloads remain separate from written geometry and runtime cell values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredSourceAppendArguments {
    /// Effective operand that names the target variable.
    pub variable: usize,
    /// Effective trailing payload ordinals with exact source cardinality.
    pub values: std::ops::Range<usize>,
}

impl ResolvedInvocation<'_, '_> {
    /// Conditional list element ordinals of this selected source constructor.
    /// Actual values, result creation, completion and future invocation remain
    /// unknown; expansion or a different result descriptor withdraws the layout.
    #[must_use]
    pub fn authored_source_list_arguments(&self) -> Option<std::ops::Range<usize>> {
        let arguments = self.words.arguments();
        if self.semantics.native_result
            != Some(crate::native_result::NativeResultContract::ListArguments { from: 0 })
            || self.semantics.argument_offset != 0
            || !matches!(self.subcommand, SubcommandResolution::NotApplicable)
        {
            return None;
        }
        Some(0..arguments.exact_argv_len()?)
    }

    /// Original operand templates of the selected list-arguments descriptor.
    /// Each pair is a source-word ordinal and whether that word is expanded;
    /// a splice supplies no evaluated element count or result value. Opaque
    /// source layouts decline, independently of the exact-argv API above.
    #[must_use]
    pub fn authored_source_list_operand_templates(&self) -> Option<Vec<(usize, bool)>> {
        let arguments = self.words.arguments();
        if self.semantics.native_result
            != Some(crate::native_result::NativeResultContract::ListArguments { from: 0 })
            || self.semantics.argument_offset != 0
            || !matches!(self.subcommand, SubcommandResolution::NotApplicable)
        {
            return None;
        }
        (0..arguments.len())
            .map(|ordinal| match arguments.get(ordinal)? {
                crate::InvocationWord::Opaque => None,
                word => Some((ordinal, matches!(word, crate::InvocationWord::Expanded))),
            })
            .collect()
    }

    /// ASCII list value of a selected list-arguments descriptor for readonly
    /// source grammar. Every element must retain an actual known ASCII value;
    /// unknown, expanded or opaque elements and other dialects decline. The
    /// shared Tcl list serializer owns quoting and separators. No native object,
    /// result allocation, effect closure or successful handler entry follows.
    #[must_use]
    pub fn authored_source_ascii_list_result(&self) -> Option<Vec<u8>> {
        // naming.source.original-structured-script-timing
        // docs/design/analysis/name-resolution-proofs/original-structured-script-timing.md
        let arguments = self.words.arguments();
        let dialect = arguments.dialect()?;
        if dialect.family() != Some(tcl_dialect::model::Family::Tcl)
            || dialect.native_string_protocol().is_none()
        {
            return None;
        }
        let ordinals = self.authored_source_list_arguments()?;
        let mut result = Vec::new();
        for ordinal in ordinals {
            let bytes = match arguments.get(ordinal)? {
                crate::InvocationWord::KnownBytes(bytes) => bytes,
                crate::InvocationWord::Literal(value) => value.as_bytes(),
                _ => return None,
            };
            if !bytes.is_ascii() || bytes.contains(&0) {
                return None;
            }
            if ordinal != 0 {
                result.push(b' ');
            }
            tcl_syntax::list::append_list_element(&mut result, bytes, ordinal == 0);
        }
        Some(result)
    }

    /// Pattern languages and effective argument ordinals from the already
    /// selected descriptors and available option grammar. Unknown selectors,
    /// option widths and expanded cardinality decline. Ordinary payloads keep
    /// their unknown values. This readonly source projection grants no handler,
    /// evaluated pattern, normal completion or edit equivalence.
    #[must_use]
    pub fn authored_source_pattern_arguments(&self) -> Option<Vec<crate::patterns::PatternArg>> {
        // naming.core.original-pattern-retained-context
        // docs/design/analysis/name-resolution-proofs/original-pattern-retained-context.md
        if !matches!(
            self.subcommand,
            SubcommandResolution::NotApplicable
                | SubcommandResolution::Exact(_)
                | SubcommandResolution::UniquePrefix(_)
        ) {
            return None;
        }
        let arguments = self.words.arguments();
        let count = arguments.exact_argv_len()?;
        let selected = self.semantics.script_metadata;
        if selected.subcommand.is_none()
            && selected.command.option_selects_pattern_language()
            && selected.command.pattern_arg_resolver.is_none()
        {
            let options = self.semantics.options;
            options.prefix_occurrences(arguments)?;
            let available = options.available().collect::<Vec<_>>();
            let effects = selected.command.option_effects_over(
                &available,
                arguments,
                options.availability.query,
            );
            if !effects.complete {
                return None;
            }
            return Some(crate::patterns::option_selected_pattern_args(
                &effects,
                options.reserved_trailing_words,
                count,
            ));
        }
        if let Some(resolve) = selected.command.pattern_arg_resolver {
            // Paired command resolvers own full post-head ordinals. A selected
            // member cannot borrow that command-level option layout.
            if selected.subcommand.is_some() || self.semantics.argument_offset != 0 {
                return None;
            }
            let options = self.semantics.options;
            let prefix = options.prefix_occurrences(arguments)?;
            let spellings = (0..count)
                .map(|index| arguments.literal_at(index).unwrap_or_default())
                .collect::<Vec<_>>();
            for occurrence in prefix {
                if occurrence.option.is_some_and(|option| {
                    option.value_word_count(&spellings, occurrence.argument_index)
                        != occurrence.values.len()
                }) {
                    return None;
                }
            }
            let available = options.available().collect::<Vec<_>>();
            let mut patterns = resolve(
                &spellings,
                crate::patterns::PatternArgResolverContext {
                    options: &available,
                    reserved_trailing_words: options.reserved_trailing_words,
                },
            );
            patterns.retain(|pattern| usize::from(pattern.index) < count);
            return Some(patterns);
        }
        let Some(kind) = selected
            .subcommand
            .and_then(|sub| sub.pattern_type)
            .or(selected.command.pattern_type)
        else {
            return Some(Vec::new());
        };
        let (roles, complete) = self.authored_source_argument_roles();
        if !complete {
            return None;
        }
        roles
            .into_iter()
            .filter(|(_, role)| *role == ArgRole::Pattern)
            .map(|(index, _)| {
                let index = self
                    .semantics
                    .argument_offset
                    .checked_add(usize::from(index))?;
                (index < count).then_some(crate::patterns::PatternArg {
                    index: u8::try_from(index).ok()?,
                    kind,
                })
            })
            .collect()
    }

    /// Format operands from the same selected descriptors and source-role
    /// grammar. Complete effective ordinals include captured prefixes and
    /// selected subcommands. This is readonly metadata, not entered dispatch,
    /// template value validation, successful completion or edit permission.
    #[must_use]
    pub fn authored_source_format_arguments(&self) -> Option<Vec<crate::FormatStringArg>> {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        if !matches!(
            self.subcommand,
            SubcommandResolution::NotApplicable
                | SubcommandResolution::Exact(_)
                | SubcommandResolution::UniquePrefix(_)
        ) {
            return None;
        }
        let selected = self.semantics.script_metadata;
        let Some(kind) = selected
            .subcommand
            .and_then(|sub| sub.format_string_type)
            .or(selected.command.format_string_type)
        else {
            return Some(Vec::new());
        };
        let count = self.words.arguments().exact_argv_len()?;
        let (roles, complete) = self.authored_source_argument_roles();
        if !complete {
            return None;
        }
        let mut formats = roles
            .into_iter()
            .filter(|(_, role)| matches!(role, ArgRole::FormatString | ArgRole::ScanFormat))
            .filter_map(|(index, role)| {
                let index = self
                    .semantics
                    .argument_offset
                    .checked_add(usize::from(index))?;
                (index < count).then_some(crate::FormatStringArg {
                    index,
                    kind,
                    scan: role == ArgRole::ScanFormat,
                })
            })
            .collect::<Vec<_>>();
        formats.sort_unstable_by_key(|format| format.index);
        formats.dedup();
        Some(formats)
    }

    /// Selected Apply lambda grammar at its declared `LambdaLiteral` operand.
    /// An unknown/expanded lambda position cannot establish trailing ordinals.
    /// Dynamic values retain their slots, while a trailing expansion remains
    /// an unknown count. This establishes no actual lambda or invocation.
    #[must_use]
    pub fn authored_source_lambda_call(&self) -> Option<AuthoredSourceLambdaCall> {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        if !matches!(
            self.subcommand,
            SubcommandResolution::NotApplicable
                | SubcommandResolution::Exact(_)
                | SubcommandResolution::UniquePrefix(_)
        ) || self.semantics.analyser_hook != Some(crate::hooks::AnalyserHookId::Apply)
        {
            return None;
        }
        let mut lambdas = self
            .semantics
            .arg_roles
            .iter()
            .filter_map(|&(index, role)| {
                (role == ArgRole::LambdaLiteral).then_some(usize::from(index))
            });
        let lambda_argument = self
            .semantics
            .argument_offset
            .checked_add(lambdas.next()?)?;
        if lambdas.next().is_some()
            || !self
                .words
                .arguments()
                .get(lambda_argument)?
                .has_exactly_one_argv_entry()
        {
            return None;
        }
        let count = count_invocation_argument_layout(
            Arity::any(),
            self.words.arguments(),
            lambda_argument.checked_add(1)?,
            self.semantics.options,
        )?;
        Some(AuthoredSourceLambdaCall {
            lambda_argument,
            count,
        })
    }

    /// Variable-name operands of the selected source grammar. Alias-pair
    /// remote operands can be computed names; only their local writes carry
    /// the name/value-confusion purpose. No actual variable cell is selected.
    #[must_use]
    pub fn authored_source_variable_name_arguments(&self) -> Vec<(usize, ArgRole)> {
        let (roles, complete) = self.authored_source_argument_roles();
        if !complete {
            return Vec::new();
        }
        let local_alias = self
            .semantics
            .frame_effect
            .is_some_and(|effect| effect.layout == crate::frame_effect::FrameArgLayout::AliasPairs);
        roles
            .into_iter()
            .filter(|(_, role)| {
                *role == ArgRole::VarWrite || (!local_alias && *role == ArgRole::VarRead)
            })
            .filter_map(|(index, role)| {
                Some((
                    self.semantics
                        .argument_offset
                        .checked_add(usize::from(index))?,
                    role,
                ))
            })
            .collect()
    }

    /// Name/value or declaration-tail source positions from the selected
    /// descriptor. Read-modify-write and whole-array forms are excluded;
    /// unknown roles/cardinality decline. No store or cell value is proved.
    #[must_use]
    pub fn authored_source_assignment_arguments(&self) -> Option<Vec<(usize, Option<usize>)>> {
        // naming.navigation.retained-path-source-inventory
        // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
        use crate::{ArgRole, Traits};
        let count = self.words.arguments().exact_argv_len()?;
        let (roles, complete) = self.authored_source_argument_roles();
        if !complete {
            return None;
        }
        if self
            .semantics
            .traits
            .intersects(Traits::READS_BEFORE_WRITE | Traits::WHOLE_ARRAY_ARG)
        {
            return Some(Vec::new());
        }
        let Some(base) = self
            .semantics
            .script_metadata
            .command
            .assigns_variable_at
            .map(usize::from)
        else {
            return Some(Vec::new());
        };
        let writes = roles
            .iter()
            .filter(|(_, role)| *role == ArgRole::VarWrite)
            .map(|(argument, _)| self.semantics.argument_offset + usize::from(*argument))
            .collect::<Vec<_>>();
        let paired_tail = self.semantics.repeated_args.iter().any(|layout| {
            layout.role == ArgRole::VarWrite
                && layout.stride == 2
                && self.semantics.argument_offset + usize::from(layout.start) == base
                && !layout.conditional_binding
        });
        if paired_tail {
            return Some(
                writes
                    .into_iter()
                    .map(|argument| (argument, (argument + 1 < count).then_some(argument + 1)))
                    .collect(),
            );
        }
        Some(
            if self.semantics.repeated_args.is_empty() && writes == [base] && count == base + 2 {
                vec![(base, Some(base + 1))]
            } else {
                Vec::new()
            },
        )
    }

    /// Scope-alias declaration positions from this selected descriptor and
    /// its complete authored roles. Adjacent read positions remain role metadata;
    /// command names, native links and successful frame entry are not inferred.
    #[must_use]
    pub fn authored_source_scope_alias_arguments(
        &self,
    ) -> Option<Vec<AuthoredSourceScopeAliasArgument>> {
        if !matches!(
            self.subcommand,
            SubcommandResolution::NotApplicable
                | SubcommandResolution::Exact(_)
                | SubcommandResolution::UniquePrefix(_)
        ) {
            return None;
        }
        if !self.semantics.traits.contains(Traits::CREATES_SCOPE_ALIAS) {
            return Some(Vec::new());
        }
        let count = self.words.arguments().exact_argv_len()?;
        let (roles, complete) = self.authored_source_argument_roles();
        if !complete {
            return None;
        }
        roles
            .iter()
            .filter(|(_, role)| *role == ArgRole::VarWrite)
            .map(|(index, _)| {
                let local = self
                    .semantics
                    .argument_offset
                    .checked_add(usize::from(*index))?;
                if local >= count {
                    return None;
                }
                let preceding_read = index
                    .checked_sub(1)
                    .filter(|previous| {
                        roles.contains(&(*previous, ArgRole::VarRead))
                            || roles.contains(&(*previous, ArgRole::VarCell))
                    })
                    .and_then(|previous| {
                        self.semantics
                            .argument_offset
                            .checked_add(usize::from(previous))
                    });
                Some(AuthoredSourceScopeAliasArgument {
                    local,
                    preceding_read,
                })
            })
            .collect()
    }

    /// Expression roles and whole-tail grammar from the already selected
    /// descriptor. Unknown selectors, unresolved roles and expanded argv
    /// cardinality withdraw this projection; ordinary unknown values keep
    /// their slots. No native evaluation or brace-rewrite equivalence follows.
    #[must_use]
    pub fn authored_source_expression_arguments(
        &self,
    ) -> Option<AuthoredSourceExpressionArguments> {
        self.source_expression_arguments(self.authored_source_argument_roles())
    }

    /// Expression positions under an independently retained positive Logical
    /// source model. Native frame grammar and execution remain unknown.
    #[must_use]
    pub fn authored_logical_source_expression_arguments(
        &self,
    ) -> Option<AuthoredSourceExpressionArguments> {
        self.source_expression_arguments(self.authored_logical_source_argument_roles())
    }

    fn source_expression_arguments(
        &self,
        (roles, complete): (Vec<(u8, ArgRole)>, bool),
    ) -> Option<AuthoredSourceExpressionArguments> {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        if !matches!(
            self.subcommand,
            SubcommandResolution::NotApplicable
                | SubcommandResolution::Exact(_)
                | SubcommandResolution::UniquePrefix(_)
        ) {
            return None;
        }
        let count = self.words.arguments().exact_argv_len()?;
        if !complete {
            return None;
        }
        let mut arguments = roles
            .into_iter()
            .filter(|(_, role)| *role == ArgRole::Expr)
            .map(|(index, _)| {
                let index = self
                    .semantics
                    .argument_offset
                    .checked_add(usize::from(index))?;
                (index < count).then_some(index)
            })
            .collect::<Option<Vec<_>>>()?;
        arguments.sort_unstable();
        arguments.dedup();
        Some(AuthoredSourceExpressionArguments {
            arguments,
            concatenates: self
                .semantics
                .traits
                .contains(Traits::EXPR_CONCATENATES_ARGS),
        })
    }

    /// Payload slots from the selected Append codegen descriptor and its complete variable
    /// role. Exact argv cardinality is required; dynamic payload values remain
    /// unknown. This is descriptive syntax, not a variable read or rewrite.
    #[must_use]
    pub fn authored_source_append_arguments(&self) -> Option<AuthoredSourceAppendArguments> {
        self.authored_source_append_layout(crate::hooks::CodegenHookId::Append)
    }

    /// Exact list-append source receiver and value slots from its selected
    /// descriptor. This supplies no native receiver, list conversion or write.
    #[must_use]
    pub fn authored_source_list_append_arguments(&self) -> Option<AuthoredSourceAppendArguments> {
        self.authored_source_append_layout(crate::hooks::CodegenHookId::Lappend)
    }

    fn authored_source_append_layout(
        &self,
        hook: crate::hooks::CodegenHookId,
    ) -> Option<AuthoredSourceAppendArguments> {
        if self.semantics.codegen_hook != Some(hook) {
            return None;
        }
        let count = self.words.arguments().exact_argv_len()?;
        let (roles, complete) = self.authored_source_argument_roles();
        if !complete {
            return None;
        }
        let mut variables = roles
            .into_iter()
            .filter_map(|(index, role)| (role == ArgRole::VarWrite).then_some(usize::from(index)));
        let variable = self
            .semantics
            .argument_offset
            .checked_add(variables.next()?)?;
        if variables.next().is_some() || variable >= count {
            return None;
        }
        Some(AuthoredSourceAppendArguments {
            variable,
            values: variable.checked_add(1)?..count,
        })
    }

    /// A selected Unset source shape whose actual dialect option protocol
    /// consumed every operand. Complete role classification preserves fixed C
    /// versus Jim prefix rules; no consumer scans or repeats option spellings.
    #[must_use]
    pub fn authored_source_unset_option_only_arguments(&self) -> Option<std::ops::Range<usize>> {
        if self.semantics.lowering_hook != Some(crate::hooks::LoweringHookId::Unset) {
            return None;
        }
        let count = self.words.arguments().exact_argv_len()?;
        let (roles, complete) = self.authored_source_argument_roles();
        (complete && count > 0 && !roles.iter().any(|(_, role)| *role == ArgRole::VarWrite))
            .then_some(0..count)
    }

    /// Literal arguments matching the selected source descriptors' closed
    /// value sets at the actual package floor. Keyword roles take precedence;
    /// option values use the shared prefix/width/reserved-data scan. Unknown
    /// values and excluded options are not enumerable source values.
    #[must_use]
    pub fn authored_source_enum_arguments(&self) -> Vec<usize> {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let selected = self.authored_source_descriptors();
        let arguments = self.words.arguments();
        let version = self.semantics.options.availability.package_version;
        let (roles, complete) = self.authored_source_argument_roles();
        if !complete {
            return Vec::new();
        }
        let mut result = Vec::new();
        let mut mark = |argument: usize, values: Vec<&crate::hover::ArgValue>| {
            if roles.iter().any(|&(index, role)| {
                self.semantics
                    .argument_offset
                    .checked_add(usize::from(index))
                    == Some(argument)
                    && role == ArgRole::Keyword
            }) {
                return;
            }
            let matches = arguments.get(argument).is_some_and(|word| match word {
                crate::InvocationWord::Literal(literal) => {
                    values.iter().any(|value| value.value == literal)
                }
                crate::InvocationWord::KnownBytes(bytes) => {
                    values.iter().any(|value| value.value.as_bytes() == bytes)
                }
                _ => false,
            });
            if matches {
                result.push(argument);
            }
        };
        for &(index, _) in selected.command.arg_values {
            mark(
                usize::from(index),
                selected.command.available_arg_values_at(index, version),
            );
        }
        if let Some(subcommand) = selected.subcommand {
            for &(index, _) in subcommand.arg_values {
                if let Some(argument) = self
                    .semantics
                    .argument_offset
                    .checked_add(usize::from(index))
                {
                    mark(argument, subcommand.available_arg_values_at(index, version));
                }
            }
        }
        if let Some(scan) = self.authored_source_diagnostic_options() {
            for option in scan.options.into_iter().filter(|option| option.available) {
                if let Some(values) = option.values {
                    for argument in values {
                        mark(
                            argument,
                            option
                                .option
                                .value_values()
                                .iter()
                                .filter(|value| value.available_for_version(version))
                                .collect(),
                        );
                    }
                }
            }
        }
        result.sort_unstable();
        result.dedup();
        result
    }
}

impl ResolvedInvocation<'_, '_> {
    /// Original effective operands of a selected conditional index schema.
    /// Unknown selectors, expansions and incompatible shapes supply no advice.
    /// Selection does not establish a native handler or substituted value.
    #[must_use]
    pub fn authored_source_index_bounds(&self) -> Option<crate::SourceIndexBoundsInvocation> {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let selected = self.semantics.script_metadata;
        let declared = selected
            .subcommand
            .map_or(selected.command.source_index_bounds, |sub| {
                sub.source_index_bounds
            });
        let operation = declared
            .or_else(|| crate::SourceIndexBounds::from_operation(self.semantics.operation))?;
        let count = self.words.arguments().exact_argv_len()?;
        let first = self.semantics.argument_offset;
        let local_count = count.checked_sub(first)?;
        let arity = self.authored_source_arity()?;
        if arity.count.indeterminate
            || !arity.arity.accepts(arity.count.minimum)
            || !operation.accepts_argument_count(local_count)
        {
            return None;
        }
        Some(crate::SourceIndexBoundsInvocation {
            operation,
            arguments: first..count,
            dialect: self.words.arguments().dialect(),
        })
    }

    /// Select the actual authored source path operation and complete effective
    /// post-head argument range. Bound prefix values retain their ordinals;
    /// unknown expansion/selectors and unmatched forms supply no algebra.
    /// This is conditional source advice, not a native handler or value.
    #[must_use]
    pub fn authored_source_path_operation(
        &self,
    ) -> Option<crate::source_path::SourcePathInvocation> {
        // naming.navigation.retained-path-source-inventory
        // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
        if self.form.is_some() {
            return None;
        }
        let operation = if let Some(member) = self.semantics.script_metadata.subcommand {
            member.source_path_operation?
        } else if self.selected.spec.inline_codegen_hook
            == Some(crate::hooks::InlineCodegenHookId::List)
        {
            crate::source_path::SourcePathOperation::SingletonList
        } else {
            return None;
        };
        let count = self.words.arguments().exact_argv_len()?;
        let first = self.semantics.argument_offset;
        let values = count.checked_sub(first)?;
        operation.accepts_argument_count(values).then_some(
            crate::source_path::SourcePathInvocation {
                operation,
                arguments: first..count,
            },
        )
    }
}
