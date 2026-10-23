// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Successful paired-list iteration entry, independent of compiler admission.

use crate::native_compilation::NativeCompilationGrammar;
use crate::{InvocationArguments, InvocationFacts, SemanticOperationId};

/// Whether validated native list inputs require entry into an iteration body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IterationEntry {
    /// All iterator lists are empty; no body runs.
    Empty,
    /// At least one iterator list is nonempty after valid variable-list parsing.
    Required,
    /// A known cardinality or list operand is invalid; no body runs.
    Invalid,
    /// The selected protocol or an input needed for entry remains unknown.
    Unknown,
}

/// Original frozen paired-list inputs for a finite native iterator. Values
/// describe bytes only; object representation and store effects are independent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiniteIteration {
    groups: Vec<FiniteIterationGroup>,
    iterations: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FiniteIterationGroup {
    names: Vec<Vec<u8>>,
    values: Vec<Vec<u8>>,
}

impl FiniteIteration {
    /// Number of actual iterations, including padding of shorter groups.
    #[must_use]
    pub fn iterations(&self) -> usize {
        self.iterations
    }

    /// Native ordered stores for this iteration. Missing elements supply the
    /// empty value; an out-of-range iteration supplies no store plan.
    pub fn bindings(&self, iteration: usize) -> Option<Vec<(&[u8], &[u8])>> {
        if iteration >= self.iterations {
            return None;
        }
        let mut bindings = Vec::new();
        for group in &self.groups {
            let start = iteration.checked_mul(group.names.len())?;
            for (index, name) in group.names.iter().enumerate() {
                let value = group
                    .values
                    .get(start.checked_add(index)?)
                    .map_or(&[][..], Vec::as_slice);
                bindings.push((name.as_slice(), value));
            }
        }
        Some(bindings)
    }
}

impl InvocationFacts {
    /// Exact native paired-list rows from complete frozen literal operands.
    /// This uses the same selected contract as iteration entry, retaining
    /// native list parsing independently from word spelling and compiler hooks.
    #[must_use]
    pub fn finite_iteration(&self, arguments: InvocationArguments<'_>) -> Option<FiniteIteration> {
        if !matches!(
            self.iteration_entry(arguments),
            IterationEntry::Empty | IterationEntry::Required
        ) {
            return None;
        }
        let dialect = arguments.dialect()?;
        let mut groups = Vec::new();
        let mut iterations = 0;
        let decode = |index| {
            tcl_syntax::list::split_list_bytes_in(
                arguments.literal_at(index)?.as_bytes(),
                dialect.word_values.list,
                dialect.lexer_grammar.escapes,
            )
            .ok()
            .map(|items| {
                items
                    .into_iter()
                    .map(std::borrow::Cow::into_owned)
                    .collect::<Vec<_>>()
            })
        };
        for variable in (self.argument_offset..arguments.len().checked_sub(1)?).step_by(2) {
            let names = decode(variable)?;
            let values = decode(variable + 1)?;
            if names.is_empty() {
                return None;
            }
            iterations = iterations.max(values.len().div_ceil(names.len()));
            groups.push(FiniteIterationGroup { names, values });
        }
        Some(FiniteIteration { groups, iterations })
    }

    /// Native paired-list entry proof from actual frozen operands. This proves
    /// no handler identity, variable-store success, observer freedom or opcode.
    #[must_use]
    pub fn iteration_entry(&self, arguments: InvocationArguments<'_>) -> IterationEntry {
        use crate::hooks::LoweringHookId;
        use IterationEntry::{Empty, Invalid, Required, Unknown};
        if !matches!(self.native_compilation, Some(contract)
            if contract.grammar == NativeCompilationGrammar::Foreach)
            || !matches!(
                self.operation,
                SemanticOperationId::StructuredLowering(
                    LoweringHookId::Foreach | LoweringHookId::Lmap
                )
            )
        {
            return Unknown;
        }
        let Some(dialect) = arguments.dialect() else {
            return Unknown;
        };
        let Some(count) = arguments.exact_argv_len() else {
            return Unknown;
        };
        let Some(count) = count.checked_sub(self.argument_offset) else {
            return Invalid;
        };
        if count < 3 || count % 2 == 0 {
            return Invalid;
        }
        if count + self.argument_offset != arguments.len() {
            return Unknown;
        }
        let mut values_unknown = false;
        let mut required = false;
        let mut variables_unknown = false;
        for variable in (self.argument_offset..arguments.len() - 1).step_by(2) {
            let list_length = |index| {
                arguments.literal_at(index).map(|text| {
                    tcl_syntax::list::split_list_bytes_in(
                        text.as_bytes(),
                        dialect.word_values.list,
                        dialect.lexer_grammar.escapes,
                    )
                    .map(|elements| elements.len())
                })
            };
            match list_length(variable) {
                Some(Ok(0) | Err(_)) => return Invalid,
                Some(Ok(_)) => {}
                None => variables_unknown = true,
            }
            match list_length(variable + 1) {
                Some(Err(_)) => return Invalid,
                Some(Ok(length)) => required |= length > 0,
                None => values_unknown = true,
            }
        }
        if variables_unknown {
            Unknown
        } else if required {
            Required
        } else if values_unknown {
            Unknown
        } else {
            Empty
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InvocationWord, InvocationWords};

    #[test]
    fn finite_native_pairs_keep_order_padding_and_dynamic_withdrawals() {
        use InvocationWord::{Dynamic, Literal as L};
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let profile = crate::model::ingress::resolve_environment(environment).unit_profile();
            let registry = crate::CommandRegistry::build_default();
            let words = [L("x y"), L("a b c"), L("z"), L("1 2 3"), L("")];
            let arguments = InvocationArguments::structured(&words)
                .with_dialect(crate::InvocationDialect::of_profile(profile));
            let selection = registry.resolve_structured_invocation(
                InvocationWords::from_arguments(L("foreach"), arguments),
                None,
            );
            let invocation = selection.resolved().unwrap();
            let facts = invocation.facts();
            let finite = facts.finite_iteration(arguments).unwrap();
            assert_eq!(finite.iterations(), 3);
            assert_eq!(
                finite.bindings(0).unwrap(),
                [
                    (b"x".as_slice(), b"a".as_slice()),
                    (b"y".as_slice(), b"b".as_slice()),
                    (b"z".as_slice(), b"1".as_slice())
                ]
            );
            assert_eq!(
                finite.bindings(1).unwrap(),
                [
                    (b"x".as_slice(), b"c".as_slice()),
                    (b"y".as_slice(), b"".as_slice()),
                    (b"z".as_slice(), b"2".as_slice())
                ]
            );
            assert_eq!(
                finite.bindings(2).unwrap(),
                [
                    (b"x".as_slice(), b"".as_slice()),
                    (b"y".as_slice(), b"".as_slice()),
                    (b"z".as_slice(), b"3".as_slice())
                ]
            );
            assert!(finite.bindings(3).is_none());
            let words = [L("x"), Dynamic, L("")];
            let unknown = InvocationArguments::structured(&words)
                .with_dialect(crate::InvocationDialect::of_profile(profile));
            assert!(facts.finite_iteration(unknown).is_none());
        }
    }

    fn entry(environment: &str, command: &str, values: &[InvocationWord<'_>]) -> IterationEntry {
        let profile = crate::model::ingress::resolve_environment(environment).unit_profile();
        let dialect = crate::InvocationDialect::of_profile(profile);
        let registry = crate::CommandRegistry::build_default();
        let arguments = InvocationArguments::structured(values).with_dialect(dialect);
        let Some(invocation) = registry
            .resolve_structured_invocation(
                InvocationWords::from_arguments(InvocationWord::Literal(command), arguments),
                None,
            )
            .resolved()
        else {
            return IterationEntry::Unknown;
        };
        invocation.facts().iteration_entry(arguments)
    }

    #[test]
    fn native_pairs_distinguish_required_empty_invalid_and_unknown_entry() {
        use InvocationWord::{Dynamic, Expanded, Literal as L};
        use IterationEntry::{Empty, Invalid, Required, Unknown};
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            assert_eq!(
                entry(environment, "foreach", &[L("x"), L("a"), L("")]),
                Required
            );
            assert_eq!(
                entry(environment, "foreach", &[L("x"), L(""), L("")]),
                Empty
            );
            assert_eq!(
                entry(environment, "foreach", &[L("x"), Dynamic, L("")]),
                Unknown
            );
            assert_eq!(
                entry(environment, "foreach", &[Dynamic, L("a"), L("")]),
                Unknown
            );
            assert_eq!(
                entry(environment, "foreach", &[L(""), L("a"), L("")]),
                Invalid
            );
            assert_eq!(
                entry(
                    environment,
                    "foreach",
                    &[L("x"), L("a"), L("y"), Dynamic, L("")]
                ),
                Required
            );
            assert_eq!(
                entry(environment, "foreach", &[Expanded, L("a"), L("")]),
                Unknown
            );
            let malformed = entry(environment, "foreach", &[L("x"), L("{"), L("")]);
            assert_eq!(
                malformed,
                if environment == "jim" {
                    Required
                } else {
                    Invalid
                }
            );
        }
        assert_eq!(entry("tcl8.6", "lmap", &[L("x"), L("a"), L("")]), Required);
        assert_eq!(entry("tcl8.4", "lmap", &[L("x"), L("a"), L("")]), Unknown);
        assert_eq!(
            entry("tcl9.1", "array", &[L("for"), L("x"), L("a"), L("")]),
            Unknown
        );
    }
}
