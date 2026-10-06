// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Bounded normal variable stores from native matching protocols.

use crate::{ArgRole, InvocationArguments, InvocationFacts};

/// Authored native matcher whose successful stores can be refined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeVariableOutputSpec {
    /// Scan's shared, native-verified complete ASCII conversion subset.
    Scan,
    /// Regexp's ASCII literal/group pattern with optional native case folding.
    Regexp,
}

/// Store obligation at an original post-head argv position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableOutputCommitment {
    /// Successful matching writes this target; physical lookup remains separate.
    Written,
    /// Matching does not store this target.
    Unchanged,
    /// A conditional store remains possible.
    MayWrite,
}

impl InvocationFacts {
    /// Refine conditional output roles only under the authored native matcher.
    /// This proves no handler, observer freedom, address or stored contents.
    /// Unknown operands and unsupported matching syntax preserve `None`.
    #[must_use]
    pub fn successful_variable_output_commitments(
        &self,
        arguments: InvocationArguments<'_>,
    ) -> Option<Vec<(usize, VariableOutputCommitment)>> {
        use crate::native_compilation::SuccessfulHandlerSpec;
        let SuccessfulHandlerSpec::ConditionalVariableOperands(protocol) =
            self.successful_handler?
        else {
            return None;
        };
        let dialect = arguments.dialect()?;
        if dialect.tcl_version.is_none()
            && !dialect.core_point.is_some_and(|point| {
                point.family() == tcl_dialect::model::Family::Jim
                    && point.release() == tcl_dialect::model::Release::JIM_0_84
            })
        {
            return None;
        }
        let count = arguments.exact_argv_len()?;
        if count != arguments.len()
            || !self.arg_roles_complete
            || self.arity_accepts_frozen_arguments() != Some(true)
        {
            return None;
        }
        let indices = |role| {
            self.arg_roles.iter().filter_map(move |&(index, found)| {
                (found == role).then_some(self.argument_offset + usize::from(index))
            })
        };
        let outputs = indices(ArgRole::VarWrite).collect::<Vec<_>>();
        if outputs.is_empty() {
            return Some(Vec::new());
        }
        match protocol {
            NativeVariableOutputSpec::Scan => {
                let format = indices(ArgRole::ScanFormat).next()?;
                let input = arguments.literal_at(format.checked_sub(1)?)?;
                let format = arguments.literal_at(format)?;
                if input.len() > 8192 || format.len() > 1024 {
                    return None;
                }
                let folded = crate::commands::tcl::scan_::fold_scan(&[input, format])?;
                let values = dialect.word_values.split_list(&folded).ok()?;
                (values.len() == outputs.len()).then(|| {
                    outputs
                        .into_iter()
                        .map(|index| (index, VariableOutputCommitment::Written))
                        .collect()
                })
            }
            NativeVariableOutputSpec::Regexp => {
                let pattern_at = indices(ArgRole::Pattern).next()?;
                literal_regexp_outputs(arguments, pattern_at, &outputs)
            }
        }
    }
}

fn literal_regexp_outputs(
    arguments: InvocationArguments<'_>,
    pattern_at: usize,
    outputs: &[usize],
) -> Option<Vec<(usize, VariableOutputCommitment)>> {
    let pattern = arguments.literal_at(pattern_at)?;
    let input = arguments.literal_at(pattern_at.checked_add(1)?)?;
    if !pattern.is_ascii()
        || !input.is_ascii()
        || pattern.contains('\0')
        || input.contains('\0')
        || input.len() > 8192
        || pattern.len() > 1024
        || pattern.bytes().any(|byte| b".^$*+?{}[]\\|".contains(&byte))
    {
        return None;
    }
    let mut nocase = false;
    for index in 0..pattern_at {
        match arguments.literal_at(index)? {
            "-nocase" => nocase = true,
            "--" if index + 1 == pattern_at => {}
            _ => return None,
        }
    }
    // On this independently bounded literal/group grammar, the shared matcher and
    // the pinned C/Jim engines agree. No C regex grammar is donated to Jim.
    let flags = tcl_regex::defs::REG_ADVANCED
        | if nocase {
            tcl_regex::defs::REG_ICASE
        } else {
            0
        };
    let regex = tcl_regex::Regex::compile_str(pattern, flags).ok()?;
    let subject = input.chars().map(u32::from).collect::<Vec<_>>();
    let matched = regex.exec(&subject, 0, 0).is_some();
    Some(
        outputs
            .iter()
            .map(|&index| {
                let commitment = if matched {
                    // Successful regexp writes every supplied output, including
                    // empty values for outputs beyond the capture count.
                    VariableOutputCommitment::Written
                } else {
                    VariableOutputCommitment::Unchanged
                };
                (index, commitment)
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InvocationWord, InvocationWords};

    fn outputs(
        environment: &str,
        command: &str,
        values: &[InvocationWord<'_>],
    ) -> Option<Vec<(usize, VariableOutputCommitment)>> {
        let dialect = crate::InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment(environment).unit_profile(),
        );
        let registry = crate::CommandRegistry::build_default();
        let arguments = InvocationArguments::structured(values).with_dialect(dialect);
        let facts = registry
            .resolve_structured_invocation(
                InvocationWords::from_arguments(InvocationWord::Literal(command), arguments),
                None,
            )
            .resolved()?
            .facts();
        facts.successful_variable_output_commitments(arguments)
    }

    #[test]
    fn literal_capture_groups_preserve_selected_successful_output_stores() {
        use InvocationWord::Literal as L;
        use VariableOutputCommitment::{Unchanged, Written};
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            assert_eq!(
                outputs(
                    environment,
                    "regexp",
                    &[L("-nocase"), L("(x)"), L("X"), L("->"), L("v"), L("extra")]
                ),
                Some(vec![(3, Written), (4, Written), (5, Written)]),
                "{environment}"
            );
            assert_eq!(
                outputs(environment, "regexp", &[L("(x)"), L("Y"), L("->"), L("v")]),
                Some(vec![(2, Unchanged), (3, Unchanged)])
            );
            assert_eq!(
                outputs(environment, "regexp", &[L("x"), L("x\0tail"), L("v")]),
                None
            );
            for pattern in ["(x)?", "(x|y)", "[a-z]", "(x", "x\0tail"] {
                assert_eq!(
                    outputs(environment, "regexp", &[L(pattern), L("X"), L("v")]),
                    None
                );
            }
        }
    }

    #[test]
    fn native_literal_matching_refines_only_proved_successful_stores() {
        use InvocationWord::{Dynamic, Expanded, Literal as L};
        use VariableOutputCommitment::{Unchanged, Written};
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            assert_eq!(
                outputs(environment, "scan", &[L("42"), L("%d"), L("n")]),
                Some(vec![(2, Written)])
            );
            assert_eq!(
                outputs(environment, "scan", &[L("20"), L("%s"), L("n")]),
                Some(vec![(2, Written)])
            );
            assert_eq!(
                outputs(environment, "scan", &[Dynamic, L("%d"), L("n")]),
                None
            );
            assert_eq!(
                outputs(environment, "scan", &[L("42"), L("%5d"), L("n")]),
                None
            );
            assert_eq!(
                outputs(environment, "scan", &[L("42"), L("%d %d"), L("n"), L("m")]),
                None
            );
            assert_eq!(
                outputs(
                    environment,
                    "regexp",
                    &[L("-nocase"), L("x"), L("X"), L("v")]
                ),
                Some(vec![(3, Written)])
            );
            assert_eq!(
                outputs(environment, "regexp", &[L("x"), L("y"), L("v")]),
                Some(vec![(2, Unchanged)])
            );
            assert_eq!(
                outputs(environment, "regexp", &[L("[a-z]"), L("x"), L("v")]),
                None
            );
            assert_eq!(
                outputs(environment, "regexp", &[Dynamic, L("x"), L("v")]),
                None
            );
            assert_eq!(
                outputs(environment, "regexp", &[Expanded, L("x"), L("v")]),
                None
            );
        }
    }
}
