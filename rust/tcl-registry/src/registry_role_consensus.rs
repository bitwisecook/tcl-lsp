// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Roles agreed by closed operand possibilities, without execution authority.

use super::CommandRegistry;
use crate::{
    ArgRole, InvocationArguments, InvocationWord, RoleOperandAlternatives, RoleOperandValues,
};
use std::collections::BTreeSet;

const MAX_ROLE_CASES: usize = 64;
const MAX_OPERAND_VALUES: usize = 8;
const MAX_OPERAND_BYTES: usize = 8192;

impl CommandRegistry {
    /// Project roles only when all closed, bounded operand cases agree.
    /// Unknown words keep their actual shape; this query cannot establish a
    /// singleton value, native object, handler, body or executable operation.
    #[must_use]
    pub fn arg_role_assignments_consensus(
        &self,
        name: &str,
        arguments: InvocationArguments<'_>,
        alternatives: &[RoleOperandAlternatives<'_>],
        wanted: &[ArgRole],
    ) -> Option<Vec<(usize, ArgRole)>> {
        let count = arguments.exact_argv_len()?;
        if count != arguments.len() {
            return None;
        }
        let mut words = (0..count)
            .map(|index| arguments.get(index))
            .collect::<Option<Vec<_>>>()?;
        let mut selected = BTreeSet::new();
        let mut varying = Vec::new();
        let mut cases = 1usize;
        for operand in alternatives {
            if operand.argument >= count || !selected.insert(operand.argument) {
                return None;
            }
            let RoleOperandValues::Closed(values) = operand.values else {
                return None;
            };
            let values = values.iter().map(String::as_str).collect::<BTreeSet<_>>();
            if values.is_empty()
                || values.len() > MAX_OPERAND_VALUES
                || values
                    .iter()
                    .try_fold(0usize, |sum, value| sum.checked_add(value.len()))?
                    > MAX_OPERAND_BYTES
                || arguments
                    .literal_at(operand.argument)
                    .is_some_and(|literal| values.iter().any(|&value| value != literal))
            {
                return None;
            }
            cases = cases.checked_mul(values.len())?;
            if cases > MAX_ROLE_CASES {
                return None;
            }
            let values = values.into_iter().collect::<Vec<_>>();
            if let [value] = values.as_slice() {
                words[operand.argument] = InvocationWord::Literal(value);
            } else {
                varying.push((operand.argument, values));
            }
        }
        let mut agreed = None;
        visit_cases(
            self,
            name,
            arguments,
            wanted,
            &varying,
            &mut words,
            &mut agreed,
        )?;
        agreed
    }
}

fn visit_cases<'a>(
    registry: &CommandRegistry,
    name: &str,
    original: InvocationArguments<'a>,
    wanted: &[ArgRole],
    varying: &[(usize, Vec<&'a str>)],
    words: &mut [InvocationWord<'a>],
    agreed: &mut Option<Vec<(usize, ArgRole)>>,
) -> Option<()> {
    if let Some(((index, values), remaining)) = varying.split_first() {
        for value in values {
            words[*index] = InvocationWord::Literal(value);
            visit_cases(registry, name, original, wanted, remaining, words, agreed)?;
        }
        return Some(());
    }
    let arguments = InvocationArguments::structured(words);
    let arguments = original
        .dialect()
        .map_or(arguments, |dialect| arguments.with_dialect(dialect));
    let roles = registry.arg_role_assignments_words(name, arguments, wanted)?;
    if let Some(previous) = agreed {
        if *previous != roles {
            return None;
        }
    } else {
        *agreed = Some(roles);
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        InvocationDialect,
        InvocationWord::{Dynamic, Expanded, Literal},
    };

    fn roles(
        environment: &str,
        name: &str,
        words: &[InvocationWord<'_>],
        values: &[String],
    ) -> Option<Vec<(usize, ArgRole)>> {
        let profile = crate::model::ingress::resolve_environment(environment).unit_profile();
        CommandRegistry::build_default().arg_role_assignments_consensus(
            name,
            InvocationArguments::structured(words)
                .with_dialect(InvocationDialect::of_profile(profile)),
            &[RoleOperandAlternatives {
                argument: 0,
                values: RoleOperandValues::Closed(values),
            }],
            &[ArgRole::Pattern, ArgRole::FormatString, ArgRole::VarWrite],
        )
    }

    #[test]
    fn closed_pattern_contents_preserve_layout_with_unknown_other_values() {
        let values = vec!["seed".into(), "left".into()];
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            assert_eq!(
                roles(
                    environment,
                    "regsub",
                    &[Dynamic, Dynamic, Dynamic, Dynamic],
                    &values
                ),
                Some(vec![
                    (0, ArgRole::Pattern),
                    (2, ArgRole::FormatString),
                    (3, ArgRole::VarWrite)
                ]),
                "{environment}"
            );
        }
    }

    #[test]
    fn option_possibilities_cannot_donate_one_operand_layout() {
        for values in [
            vec!["seed".into(), "-command".into()],
            vec!["seed".into(), "-start".into()],
        ] {
            for environment in ["tcl8.6", "tcl9.0", "jim"] {
                assert!(
                    roles(
                        environment,
                        "regsub",
                        &[Dynamic, Dynamic, Dynamic, Dynamic],
                        &values
                    )
                    .is_none(),
                    "{environment}: {values:?}"
                );
            }
        }
        assert!(
            roles(
                "tcl8.6",
                "regexp",
                &[Dynamic, Dynamic, Dynamic],
                &["seed".into(), "-about".into()]
            )
            .is_none()
        );
        assert!(
            roles(
                "tcl9.0",
                "regsub",
                &[Dynamic, Expanded, Dynamic],
                &["seed".into()]
            )
            .is_none()
        );
        assert!(
            roles(
                "tcl9.0",
                "regsub",
                &[Literal("literal"), Dynamic, Dynamic],
                &["different".into()]
            )
            .is_none()
        );
        assert!(roles("tcl9.0", "regsub", &[Dynamic, Dynamic, Dynamic], &[]).is_none());
    }

    #[test]
    fn structured_roles_use_selected_command_option_availability() {
        let registry = CommandRegistry::build_default();
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let dialect = InvocationDialect::of_profile(
                crate::model::ingress::resolve_environment(environment).unit_profile(),
            );
            let words = [Literal("-command"), Literal("pattern"), Dynamic, Dynamic];
            let result = registry.arg_role_assignments_consensus(
                "regsub",
                InvocationArguments::structured(&words).with_dialect(dialect),
                &[],
                &[ArgRole::Pattern, ArgRole::FormatString],
            );
            assert_eq!(
                result,
                matches!(environment, "tcl9.0" | "tcl9.1" | "jim")
                    .then_some(vec![(1, ArgRole::Pattern)]),
                "{environment}"
            );
            let unknown = [Dynamic, Literal("pattern"), Dynamic, Dynamic];
            assert!(
                registry
                    .arg_role_assignments_consensus(
                        "regsub",
                        InvocationArguments::structured(&unknown).with_dialect(dialect),
                        &[],
                        &[ArgRole::Pattern]
                    )
                    .is_none()
            );
        }
    }

    #[test]
    fn unknown_or_overflowed_alternatives_keep_roles_unproved() {
        let words = [Dynamic, Dynamic, Dynamic];
        let registry = CommandRegistry::build_default();
        let arguments = InvocationArguments::structured(&words).with_dialect(
            InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0),
        );
        assert!(
            registry
                .arg_role_assignments_consensus(
                    "regsub",
                    arguments,
                    &[RoleOperandAlternatives {
                        argument: 0,
                        values: RoleOperandValues::Unknown
                    }],
                    &[ArgRole::Pattern]
                )
                .is_none()
        );
        let values = (0..8).map(|n| format!("pattern{n}")).collect::<Vec<_>>();
        let alternatives = (0..3)
            .map(|argument| RoleOperandAlternatives {
                argument,
                values: RoleOperandValues::Closed(&values),
            })
            .collect::<Vec<_>>();
        assert!(
            registry
                .arg_role_assignments_consensus(
                    "regsub",
                    arguments,
                    &alternatives,
                    &[ArgRole::Pattern]
                )
                .is_none()
        );
    }
}
