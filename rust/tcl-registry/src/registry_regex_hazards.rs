// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Possible regex operands for hazards, independent of guaranteed layouts.

use super::CommandRegistry;
use crate::hooks::ReturnTypeHookId;
use crate::hover::{OptionArity, OptionSpec, OptionValue};
use crate::{InvocationArguments, InvocationWord, InvocationWords};
use std::collections::BTreeSet;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Prefix {
    index: usize,
    about: bool,
    inline: bool,
}

enum OptionConsumption {
    Next(Prefix),
    MissingValue,
}

impl Prefix {
    fn accepts(self, grammar: ReturnTypeHookId, count: usize) -> bool {
        let remaining = count.saturating_sub(self.index);
        match grammar {
            ReturnTypeHookId::Regexp if self.about => remaining >= 1,
            ReturnTypeHookId::Regexp if self.inline => remaining == 2,
            ReturnTypeHookId::Regexp => remaining >= 2,
            ReturnTypeHookId::Regsub => (3..=4).contains(&remaining),
            _ => false,
        }
    }
}

impl CommandRegistry {
    /// Original argv slots that can carry a native regex on a valid layout.
    /// Unknown one-word prefixes keep every admitted switch/positional case.
    /// `None` retains an unbounded layout; an empty vector means no valid case.
    /// This hazard query establishes no guaranteed role, value, result type,
    /// variable store, command handler or executable operation.
    #[must_use]
    pub fn possible_pattern_argument_indices_words(
        &self,
        name: &str,
        arguments: InvocationArguments<'_>,
    ) -> Option<Vec<usize>> {
        let arguments = arguments.with_profile(self.profile());
        let count = arguments.exact_argv_len()?;
        if count != arguments.len() || count > usize::from(u8::MAX) + 1 {
            return None;
        }
        let query = arguments
            .dialect()
            .and_then(crate::InvocationDialect::authoring_query)
            .or_else(|| self.own_surface_query());
        let spec = self.get_for_surface(name, query)?;
        if spec.pattern_type != Some(crate::patterns::PatternType::Regex) {
            return None;
        }
        let grammar = spec.return_type_hook?;
        if !matches!(grammar, ReturnTypeHookId::Regexp | ReturnTypeHookId::Regsub) {
            return None;
        }
        let resolved = self
            .resolve_structured_invocation(
                InvocationWords::from_arguments(InvocationWord::Literal(name), arguments),
                query,
            )
            .resolved()?;
        let options = resolved.semantics.options;
        possible_prefixes(arguments, options, grammar)
    }
}

fn possible_prefixes(
    arguments: InvocationArguments<'_>,
    options: crate::resolved_invocation::InvocationOptions<'_>,
    grammar: ReturnTypeHookId,
) -> Option<Vec<usize>> {
    let count = arguments.exact_argv_len()?;
    let table = options.available().collect::<Vec<_>>();
    let mut pending = BTreeSet::from([Prefix {
        index: options.positional_prefix_words,
        about: false,
        inline: false,
    }]);
    let mut visited = BTreeSet::new();
    let mut patterns = BTreeSet::new();
    while let Some(prefix) = pending.pop_first() {
        if !visited.insert(prefix) || prefix.index >= count {
            continue;
        }
        let word = arguments.get(prefix.index)?;
        if word.proves_non_option()
            || word.literal().is_none()
            || word
                .literal()
                .is_some_and(|word| !word.starts_with('-') || word.len() < 2)
        {
            if prefix.accepts(grammar, count) {
                patterns.insert(prefix.index);
            }
            if word.proves_non_option() || word.literal().is_some() {
                continue;
            }
        }
        let candidates = word.literal().map_or_else(
            || table.clone(),
            |literal| {
                crate::spec::resolve_available_option_prefix_with(
                    &table,
                    literal,
                    options.prefix_matching,
                )
                .into_iter()
                .collect()
            },
        );
        for option in candidates {
            if let OptionConsumption::Next(next) = consume_option(prefix, option, count)? {
                if option.name == "--" {
                    if next.accepts(grammar, count) {
                        patterns.insert(next.index);
                    }
                } else {
                    pending.insert(next);
                }
            }
        }
    }
    Some(patterns.into_iter().collect())
}

fn consume_option(prefix: Prefix, option: &OptionSpec, count: usize) -> Option<OptionConsumption> {
    let width = match option.value {
        OptionValue::Flag => 0,
        OptionValue::Takes(argument) => match argument.arity {
            OptionArity::One => 1,
            OptionArity::Fixed(width) => usize::from(width),
            OptionArity::Hook(_) => return None,
        },
    };
    let index = prefix.index.checked_add(1 + width)?;
    if index > count {
        return Some(OptionConsumption::MissingValue);
    }
    Some(OptionConsumption::Next(Prefix {
        index,
        about: prefix.about || option.name == "-about",
        inline: prefix.inline || option.name == "-inline",
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InvocationWord::{Dynamic, Expanded, Literal};

    fn possible(environment: &str, name: &str, words: &[InvocationWord<'_>]) -> Option<Vec<usize>> {
        let profile = crate::model::ingress::resolve_environment(environment).unit_profile();
        CommandRegistry::build_default().possible_pattern_argument_indices_words(
            name,
            InvocationArguments::structured(words)
                .with_dialect(crate::InvocationDialect::of_profile(profile)),
        )
    }

    #[test]
    fn unknown_prefix_preserves_possible_patterns_without_guaranteed_roles() {
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let words = [Dynamic, Dynamic, Dynamic];
            assert_eq!(
                possible(environment, "regexp", &words),
                Some(if environment == "jim" {
                    vec![0, 1]
                } else {
                    vec![0, 1, 2]
                })
            );
            let registry = CommandRegistry::build_default();
            let arguments = InvocationArguments::structured(&words).with_dialect(
                crate::InvocationDialect::of_profile(
                    crate::model::ingress::resolve_environment(environment).unit_profile(),
                ),
            );
            assert!(
                registry
                    .arg_role_assignments_consensus(
                        "regexp",
                        arguments,
                        &[],
                        &[crate::ArgRole::Pattern]
                    )
                    .is_none()
            );
            assert_eq!(possible(environment, "regsub", &words), Some(vec![0]));
        }
    }

    #[test]
    fn invalid_counts_and_missing_option_values_have_no_possible_pattern() {
        for environment in ["tcl8.6", "tcl9.0", "jim"] {
            assert_eq!(
                possible(environment, "regexp", &[Literal("-start")]),
                Some(vec![])
            );
            assert_eq!(
                possible(environment, "regsub", &[Dynamic, Dynamic]),
                Some(vec![])
            );
            assert_eq!(
                possible(
                    environment,
                    "regexp",
                    &[Literal("-inline"), Literal("pattern"), Dynamic, Dynamic]
                ),
                Some(vec![])
            );
            assert_eq!(
                possible(
                    environment,
                    "regsub",
                    &[Literal("-start"), Dynamic, Dynamic, Dynamic]
                ),
                Some(vec![])
            );
            assert!(possible(environment, "regexp", &[Expanded, Dynamic]).is_none());
        }
    }
}
