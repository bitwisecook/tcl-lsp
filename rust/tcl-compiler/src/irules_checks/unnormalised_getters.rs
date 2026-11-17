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

//! IRULE3102: getters read without the normalisation their option vocabulary declares.

use super::{IrulesCheckPayload, IrulesCheckWarning, IrulesSourceQuery, is_irules};
use crate::compilation_unit::CompilationUnit;
#[cfg(test)]
use crate::irules_checks::FunctionUnit;
#[cfg(test)]
use crate::irules_checks::irules_function_source_config;
use crate::sccp::cfg_order;
use tcl_core_types::DiagCode;
use tcl_dialect::model::SurfaceQuery;
use tcl_lexer::Span;
use tcl_registry::CommandRegistry;

/// Whether `cmd` is a registered iRules getter that carries the
/// `-normalized` option in its [`tcl_registry::CommandSpec`].
///
/// Replaces the previous hardcoded `NORMALISED_FLAG_COMMANDS` table:
/// the registry's own option list is the single source of truth, so
/// adding `-normalized` to a new command's spec automatically
/// extends the IRULE3102 surface.
#[cfg(test)]
pub(super) fn supports_normalized_flag(registry: &CommandRegistry, cmd: &str) -> bool {
    registry
        .get(cmd)
        .is_some_and(|spec| spec.options.iter().any(|opt| opt.name == "-normalized"))
}

/// Return `true` when `args` suggests a *getter* invocation — no args,
/// or every arg starts with `-` (all flags, no positional value).
/// Resolving a getter form properly would need a form-kind model on the
/// command registry, which it does not have yet, so this is a
/// conservative approximation. `HTTP::path /foo` (setter) is excluded
/// because `/foo` is a non-flag first arg.
pub(super) fn is_getter_form(args: &[String]) -> bool {
    args.iter().all(|a| a.starts_with('-'))
}

/// Format the IRULE3102 message for `cmd`.
pub(crate) fn format_message(cmd: &str) -> String {
    format!(
        "Use '{cmd} -normalized' for canonicalized request data; \
         non-normalized values may allow URL evasion patterns."
    )
}

/// A selected option vocabulary declares normalization but its exact original
/// flag-form operands do not select that option.
pub(crate) fn is_unnormalised_getter(
    options: &tcl_registry::abbrev::KeywordTable<'_>,
    args: &[String],
) -> bool {
    if !options.names().any(|name| name == "-normalized") || !is_getter_form(args) {
        return false;
    }
    !args.iter().any(|argument| {
        options.resolve(argument) == tcl_registry::abbrev::KeywordMatch::Unique("-normalized")
    })
}

/// Find IRULE3102 warnings across every function in `cu`.
///
/// Gated: returns an empty vector unless the point stands on the iRules
/// surface.
///
/// Each invocation retains its original source binding. Nested commands use
/// the complete argument substitution owner; braced data supplies no calls.
#[must_use]
pub fn find_unnormalised_getter_warnings(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<SurfaceQuery<'_>>,
) -> Vec<IrulesCheckWarning> {
    if !is_irules(dialect) {
        return Vec::new();
    }
    let Some(source) = IrulesSourceQuery::capture(cu, registry) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for function in cu.analysable_functions() {
        let Some(config) = source.function_config(function) else {
            continue;
        };
        for block in cfg_order(&function.cfg) {
            if !function.sccp.executable_blocks.contains(&block) {
                continue;
            }
            let Some(block) = function.cfg.blocks.get(&block) else {
                continue;
            };
            for statement in &block.statements {
                let Some(tokens) =
                    crate::ir::CommandBindingSites::unanimous_statement_source_tokens(
                        function.cfg.command_binding_sites.iter(),
                        statement,
                    )
                    .filter(|tokens| tokens.synthetic.is_none())
                else {
                    continue;
                };
                let Some(warnings) = original_getter_source_warnings(tokens, &source, config)
                else {
                    continue;
                };
                out.extend(warnings.into_iter().map(|mut warning| {
                    warning.span = function.abs_span(warning.span);
                    warning
                }));
            }
        }
    }
    out.sort_by(|a, b| {
        (a.span.start(), a.span.end(), &a.message).cmp(&(b.span.start(), b.span.end(), &b.message))
    });
    out.dedup();
    out
}

fn original_getter_source_warnings(
    parent: &crate::ir::CommandTokens,
    source: &IrulesSourceQuery<'_>,
    config: tcl_lexer::LexerConfig,
) -> Option<Vec<IrulesCheckWarning>> {
    parent
        .source_binding
        .as_ref()?
        .original_lexer_config_for_tokens(parent)?;
    let children = source.argument_calls(parent, config)?;
    let mut warnings = Vec::new();
    for tokens in std::iter::once(parent).chain(
        children
            .iter()
            .map(|child| child.tokens.as_ref())
            .collect::<Option<Vec<_>>>()?,
    ) {
        let assistance =
            crate::registry_invocation::original_registry_invocation_assistance_with_metadata_context(
                source.registry,
                source.metadata(),
                tokens,
            )?;
        for candidate in &assistance.candidates {
            let arguments = candidate
                .source_argument_words(source.registry, source.metadata(), tokens)?
                .iter()
                .map(|argument| {
                    Some(
                        std::str::from_utf8(argument.literal_bytes()?)
                            .ok()?
                            .to_owned(),
                    )
                })
                .collect::<Option<Vec<_>>>();
            let Some(arguments) = arguments else {
                continue;
            };
            if is_unnormalised_getter(&candidate.option_keywords, &arguments) {
                warnings.push(IrulesCheckWarning {
                    span: Span::new(tokens.argv.first()?.start(), tokens.argv.last()?.end()),
                    code: DiagCode::Irule3102,
                    message: format_message(&candidate.command),
                    payload: IrulesCheckPayload::UnnormalisedGetter {
                        command: candidate.command.clone(),
                    },
                    replacement: None,
                    fixes: Vec::new(),
                });
            }
        }
    }
    Some(warnings)
}

#[cfg(test)]
pub(super) fn unnormalised_getter_source_config(
    module: &crate::ir::Module,
    function: &FunctionUnit,
    registry: &CommandRegistry,
    standalone_module: bool,
) -> Option<tcl_lexer::LexerConfig> {
    irules_function_source_config(module, function, registry, standalone_module)
}
