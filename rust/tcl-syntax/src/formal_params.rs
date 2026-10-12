// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Native formal parameter parsing and argument activation plans.
//!
//! A `proc`, method, or `apply` parameter list has two list levels.  The outer
//! list contains parameter specifiers; each specifier is itself a zero-, one-,
//! two-, or many-element Tcl list.  This module owns that shared grammar and
//! the scalar/simple-name validation performed by Tcl when creating a proc.
//! Consumers remain responsible for converting strings into their own value
//! representation and for rendering errors at their byte or object boundary.

use crate::list::{ListError, join_list, split_list};

mod bytes;
pub use bytes::{
    ByteFormalParameter, ByteFormalParameterError, FormalByteArgumentBinding,
    FormalParameterValueError, bind_formal_argument_bytes, formal_parameter_usage_bytes,
    parse_formal_parameter_values,
};

/// Pure native formal count shape, independently of values and activation.
/// The maximum is absent for a variadic signature; this supplies no local
/// cell, argument materialization, body entry or successful execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormalArgumentCountShape {
    /// Least argv count accepted by the selected binding grammar.
    pub minimum: usize,
    /// Greatest accepted argv count, absent for an unbounded rest formal.
    pub maximum: Option<usize>,
}
impl FormalArgumentCountShape {
    /// The count satisfies this descriptive binding shape.
    #[must_use]
    pub fn accepts(self, count: usize) -> bool {
        count >= self.minimum && self.maximum.is_none_or(|maximum| count <= maximum)
    }
}

/// Shared argument-count owner for already decoded formal fields.
/// Each item states whether the native formal storage name is exactly `args`
/// and whether that formal has a default. C uses positional defaults and a
/// final rest formal; Jim reserves required slots and accepts rest anywhere.
#[must_use]
pub fn formal_argument_count_shape(
    parameters: impl DoubleEndedIterator<Item = (bool, bool)> + ExactSizeIterator + Clone,
    grammar: tcl_dialect::ParameterGrammar,
) -> FormalArgumentCountShape {
    if grammar == tcl_dialect::ParameterGrammar::Jim {
        let rest = parameters.clone().any(|(args, _)| args);
        let minimum = parameters
            .clone()
            .filter(|&(args, default)| !args && !default)
            .count();
        let maximum = (!rest).then(|| parameters.filter(|&(args, _)| !args).count());
        FormalArgumentCountShape { minimum, maximum }
    } else {
        let rest = parameters.clone().next_back().is_some_and(|(args, _)| args);
        let fixed = parameters.len() - usize::from(rest);
        let minimum = parameters
            .take(fixed)
            .rposition(|(_, default)| !default)
            .map_or(0, |index| index + 1);
        FormalArgumentCountShape {
            minimum,
            maximum: (!rest).then_some(fixed),
        }
    }
}

/// One decoded formal parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormalParameter {
    /// Native decoded formal name; C Tcl requires scalar simple names.
    pub name: String,
    /// The decoded default value, when the specifier has two fields.
    pub default: Option<String>,
}

/// Where a malformed Tcl list was encountered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterListLevel {
    /// The outer list of parameter specifiers.
    Parameters,
    /// One parameter specifier, which is itself parsed as a Tcl list.
    Specifier,
}

/// Why a strict formal-parameter parse failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormalParameterError {
    /// One of the two Tcl list parses failed.
    InvalidList {
        /// Whether the outer list or an inner specifier failed.
        level: ParameterListLevel,
        /// Text passed to the failing list parse.
        input: String,
        /// The shared Tcl list-parser error.
        error: ListError,
    },
    /// A parameter specifier contained zero fields.
    NoFields,
    /// Jim permits only one variadic args parameter, wherever it appears.
    DuplicateArgs,
    /// A one- or two-field specifier supplied an empty first field.
    EmptyName,
    /// A parameter specifier contained more than two fields.
    TooManyFields {
        /// The decoded outer-list element, as Tcl prints in its diagnostic.
        specifier: String,
    },
    /// The name denotes an array element, which cannot be a formal parameter.
    ArrayElement {
        /// The decoded offending name.
        name: String,
    },
    /// The name contains a namespace separator and is therefore not simple.
    NotSimpleName {
        /// The decoded offending name.
        name: String,
    },
}

impl FormalParameterError {
    /// Render definition-time diagnostics under the selected C release. Tcl
    /// 8.4 includes the procedure name in three formal-name errors.
    #[must_use]
    pub fn message_for_definition(
        &self,
        procedure: &str,
        version: Option<tcl_dialect::TclVersion>,
    ) -> String {
        String::from_utf8(self.message_for_definition_bytes(procedure.as_bytes(), version))
            .expect("Unicode procedure and parameter names retain Unicode diagnostics")
    }

    /// Render the same selected definition failure with an exact byte-valued
    /// procedure name. This preserves invalid UTF-8 names without replacement.
    #[must_use]
    pub fn message_for_definition_bytes(
        &self,
        procedure: &[u8],
        version: Option<tcl_dialect::TclVersion>,
    ) -> Vec<u8> {
        if version == Some(tcl_dialect::TclVersion::V8_4) {
            let detail = match self {
                Self::NoFields | Self::EmptyName => " has argument with no name".to_owned(),
                Self::ArrayElement { name } => {
                    format!(" has formal parameter \"{name}\" that is an array element")
                }
                Self::NotSimpleName { name } => {
                    format!(" has formal parameter \"{name}\" that is not a simple name")
                }
                _ => return self.message().into_bytes(),
            };
            let mut message = b"procedure \"".to_vec();
            message.extend_from_slice(procedure);
            message.push(b'"');
            message.extend_from_slice(detail.as_bytes());
            return message;
        }
        self.message().into_bytes()
    }

    /// Render the reference Tcl diagnostic as UTF-8 text.
    ///
    /// Byte-oriented runtimes can convert this result at their API boundary;
    /// keeping rendering here makes the semantic variants useful to compilers
    /// without coupling the parser to an interpreter object representation.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::InvalidList { input, error, .. } => error.full_message(input),
            Self::DuplicateArgs => "'args' specified more than once".to_owned(),
            Self::NoFields | Self::EmptyName => "argument with no name".to_string(),
            Self::TooManyFields { specifier } => {
                format!("too many fields in argument specifier \"{specifier}\"")
            }
            Self::ArrayElement { name } => {
                format!("formal parameter \"{name}\" is an array element")
            }
            Self::NotSimpleName { name } => {
                format!("formal parameter \"{name}\" is not a simple name")
            }
        }
    }
}

impl std::fmt::Display for FormalParameterError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message())
    }
}

impl std::error::Error for FormalParameterError {}

/// Parse a Tcl formal-parameter list strictly.
///
/// The returned strings are Tcl list *values*: braces and quotes are removed,
/// and applicable backslash substitutions are collapsed by the shared list
/// parser.  `args` is not a distinct parameter kind because it is variadic only
/// when it is the final name; use [`has_trailing_args`] on the finished list.
pub fn parse_formal_parameters(source: &str) -> Result<Vec<FormalParameter>, FormalParameterError> {
    parse_formal_parameters_in(source, tcl_dialect::ParameterGrammar::Tcl)
}

/// Parse the selected engine's native formals. Jim's one-field form retains
/// the outer element spelling, and its lenient list grammar owns both levels.
pub fn parse_formal_parameters_in(
    source: &str,
    grammar: tcl_dialect::ParameterGrammar,
) -> Result<Vec<FormalParameter>, FormalParameterError> {
    let split = |input: &str, level| {
        let values = match grammar {
            tcl_dialect::ParameterGrammar::Tcl => split_list(input),
            tcl_dialect::ParameterGrammar::Jim => Ok(crate::list::split_list_jim(input)),
        };
        values
            .map(|values| {
                values
                    .into_iter()
                    .map(std::borrow::Cow::into_owned)
                    .collect::<Vec<_>>()
            })
            .map_err(|error| FormalParameterError::InvalidList {
                level,
                input: input.to_owned(),
                error,
            })
    };
    let specs = split(source, ParameterListLevel::Parameters)?;
    let mut parameters = Vec::with_capacity(specs.len());
    let mut jim_args_seen = false;
    for spec in specs {
        let fields = split(&spec, ParameterListLevel::Specifier)?;
        let (name, default) = match fields.as_slice() {
            [] => return Err(FormalParameterError::NoFields),
            [name] => (
                if grammar == tcl_dialect::ParameterGrammar::Jim {
                    spec.as_str()
                } else {
                    name.as_str()
                },
                None,
            ),
            [name, default] => (name.as_str(), Some(default.as_str())),
            _ => return Err(FormalParameterError::TooManyFields { specifier: spec }),
        };
        if grammar == tcl_dialect::ParameterGrammar::Tcl {
            validate_name(name)?;
        } else if name == "args" {
            if jim_args_seen {
                return Err(FormalParameterError::DuplicateArgs);
            }
            jim_args_seen = true;
        }
        parameters.push(FormalParameter {
            name: name.to_owned(),
            default: default.map(str::to_owned),
        });
    }
    Ok(parameters)
}

/// One native assignment at procedure activation, before executing its body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormalArgumentBinding {
    /// Assign one supplied argument to the formal's native name.
    Value {
        /// Index in the parsed parameter list.
        parameter: usize,
        /// Index in the supplied post-head argv.
        argument: usize,
    },
    /// Assign the formal's literal default; reference spelling is inert here.
    Default {
        /// Index in the parsed parameter list.
        parameter: usize,
    },
    /// Assign a list of surplus argv entries to the selected rest name.
    Rest {
        /// Index of the variadic formal.
        parameter: usize,
        /// Native target name, including Jim's renamed args form.
        name: String,
        /// First surplus argv position.
        start: usize,
        /// Number of surplus argv entries.
        len: usize,
    },
    /// Link the local name to the caller variable named by this argument.
    CallerLink {
        /// Index of the reference formal.
        parameter: usize,
        /// Local alias spelling without the reference marker.
        name: String,
        /// Argument whose value names the existing caller variable.
        argument: usize,
    },
}

/// Argument count cannot satisfy the native parameter activation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormalArityError;

/// Select native bindings from argv positions without evaluating values or
/// performing variable operations. Consumers apply these assignments through
/// their shared frame/cell owner; caller links preserve caller identity.
pub fn bind_formal_arguments(
    parameters: &[FormalParameter],
    argument_count: usize,
    grammar: tcl_dialect::ParameterGrammar,
) -> Result<Vec<FormalArgumentBinding>, FormalArityError> {
    let parameters = parameters
        .iter()
        .map(|parameter| ByteFormalParameter {
            name: parameter.name.as_bytes().to_vec(),
            default: parameter
                .default
                .as_ref()
                .map(|value| value.as_bytes().to_vec()),
        })
        .collect::<Vec<_>>();
    bind_formal_argument_bytes(&parameters, argument_count, grammar).map(|bindings| {
        bindings
            .into_iter()
            .map(|binding| match binding {
                FormalByteArgumentBinding::Value {
                    parameter,
                    argument,
                } => FormalArgumentBinding::Value {
                    parameter,
                    argument,
                },
                FormalByteArgumentBinding::Default { parameter } => {
                    FormalArgumentBinding::Default { parameter }
                }
                FormalByteArgumentBinding::Rest {
                    parameter,
                    name,
                    start,
                    len,
                } => FormalArgumentBinding::Rest {
                    parameter,
                    name: String::from_utf8(name).expect("Unicode formal name"),
                    start,
                    len,
                },
                FormalByteArgumentBinding::CallerLink {
                    parameter,
                    name,
                    argument,
                } => FormalArgumentBinding::CallerLink {
                    parameter,
                    name: String::from_utf8(name).expect("Unicode formal name"),
                    argument,
                },
            })
            .collect()
    })
}

/// Native usage suffix for a parsed formal list, without the command prefix.
/// Diagnostic rendering shares variadic position/name selection with activation.
#[must_use]
pub fn formal_parameter_usage(
    parameters: &[FormalParameter],
    grammar: tcl_dialect::ParameterGrammar,
) -> String {
    parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| {
            let rest = parameter.name == "args"
                && (grammar == tcl_dialect::ParameterGrammar::Jim || index + 1 == parameters.len());
            if rest {
                let name = if grammar == tcl_dialect::ParameterGrammar::Jim {
                    parameter.default.as_deref().unwrap_or("arg")
                } else {
                    "arg"
                };
                format!("?{name} ...?")
            } else if parameter.default.is_some() {
                let name = format!("?{}?", parameter.name);
                if grammar == tcl_dialect::ParameterGrammar::Tcl {
                    join_list([name])
                } else {
                    name
                }
            } else if grammar == tcl_dialect::ParameterGrammar::Jim {
                parameter
                    .name
                    .strip_prefix('&')
                    .unwrap_or(&parameter.name)
                    .to_owned()
            } else {
                join_list([&parameter.name])
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Repair an accidentally grouped, overlong parameter specifier by splitting
/// its fields into separate formal parameters.
///
/// This is deliberately the only mechanical repair exposed by the strict
/// parser. An array-element or qualified name has several plausible intended
/// replacements, while an empty or malformed list may be incomplete source.
/// A [`FormalParameterError::TooManyFields`] already identifies the first
/// overlong specifier Tcl rejected; replacing that one element with its
/// decoded fields produces a canonical, valid outer Tcl list when possible.
#[must_use]
pub fn split_overlong_parameter_specifier(
    source: &str,
    error: &FormalParameterError,
) -> Option<String> {
    let FormalParameterError::TooManyFields { specifier } = error else {
        return None;
    };
    let mut specs = split_list(source).ok()?;
    let position = specs.iter().position(|candidate| candidate == specifier)?;
    let fields = split_list(specifier).ok()?;
    if fields.len() <= 2 {
        return None;
    }
    specs.splice(position..=position, fields);
    let repaired = join_list(specs);
    parse_formal_parameters(&repaired).ok()?;
    Some(repaired)
}

/// Whether the final formal parameter is Tcl's variadic `args` parameter.
#[must_use]
pub fn has_trailing_args(parameters: &[FormalParameter]) -> bool {
    parameters
        .last()
        .is_some_and(|parameter| parameter.name == "args")
}

fn validate_name(name: &str) -> Result<(), FormalParameterError> {
    if name.is_empty() {
        return Err(FormalParameterError::EmptyName);
    }
    let bytes = name.as_bytes();
    let final_byte = bytes.len() - 1;
    let mut index = 0;
    while index < final_byte {
        if bytes[index] == b'(' && bytes[final_byte] == b')' {
            return Err(FormalParameterError::ArrayElement {
                name: name.to_string(),
            });
        }
        if bytes[index] == b':' && bytes[index + 1] == b':' {
            return Err(FormalParameterError::NotSimpleName {
                name: name.to_string(),
            });
        }
        index += 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definition_error_preserves_the_actual_byte_name_and_release() {
        let error = FormalParameterError::ArrayElement {
            name: "arr(k)".to_owned(),
        };
        assert_eq!(
            error.message_for_definition_bytes(b"bad\xff", Some(tcl_dialect::TclVersion::V8_4)),
            b"procedure \"bad\xff\" has formal parameter \"arr(k)\" that is an array element"
        );
        assert_eq!(
            error.message_for_definition_bytes(b"bad\xff", Some(tcl_dialect::TclVersion::V8_5)),
            b"formal parameter \"arr(k)\" is an array element"
        );
    }

    #[test]
    fn native_activation_plan_reserves_jim_required_arguments() {
        use tcl_dialect::ParameterGrammar::{Jim, Tcl};
        let parameters = parse_formal_parameters_in("a {b B} c", Jim).unwrap();
        assert_eq!(
            bind_formal_arguments(&parameters, 2, Jim).unwrap(),
            vec![
                FormalArgumentBinding::Value {
                    parameter: 0,
                    argument: 0
                },
                FormalArgumentBinding::Default { parameter: 1 },
                FormalArgumentBinding::Value {
                    parameter: 2,
                    argument: 1
                },
            ]
        );
        assert_eq!(
            bind_formal_arguments(&parameters, 2, Tcl),
            Err(FormalArityError)
        );
        let parameters = parse_formal_parameters_in("a args b", Jim).unwrap();
        assert_eq!(
            bind_formal_arguments(&parameters, 4, Jim).unwrap(),
            vec![
                FormalArgumentBinding::Value {
                    parameter: 0,
                    argument: 0
                },
                FormalArgumentBinding::Rest {
                    parameter: 1,
                    name: "args".into(),
                    start: 1,
                    len: 2
                },
                FormalArgumentBinding::Value {
                    parameter: 2,
                    argument: 3
                },
            ]
        );
    }

    #[test]
    fn native_activation_plan_keeps_reference_and_default_distinct() {
        use tcl_dialect::ParameterGrammar::{Jim, Tcl};
        let parameters = parse_formal_parameters_in("{&x DEFAULT}", Jim).unwrap();
        assert_eq!(
            bind_formal_arguments(&parameters, 0, Jim).unwrap(),
            vec![FormalArgumentBinding::Default { parameter: 0 }]
        );
        assert_eq!(
            bind_formal_arguments(&parameters, 1, Jim).unwrap(),
            vec![FormalArgumentBinding::CallerLink {
                parameter: 0,
                name: "x".into(),
                argument: 0
            }]
        );
        let parameters = parse_formal_parameters_in("{args rest}", Jim).unwrap();
        assert_eq!(
            bind_formal_arguments(&parameters, 2, Jim).unwrap(),
            vec![FormalArgumentBinding::Rest {
                parameter: 0,
                name: "rest".into(),
                start: 0,
                len: 2
            }]
        );
        assert_eq!(
            bind_formal_arguments(&parameters, 2, Tcl).unwrap(),
            vec![FormalArgumentBinding::Rest {
                parameter: 0,
                name: "args".into(),
                start: 0,
                len: 2
            }]
        );
    }

    #[test]
    fn native_parameter_grammar_and_usage_preserve_engine_differences() {
        use tcl_dialect::ParameterGrammar::{Jim, Tcl};
        assert_eq!(
            parse_formal_parameters_in("args args", Jim),
            Err(FormalParameterError::DuplicateArgs)
        );
        assert!(parse_formal_parameters_in("n::x a(k)", Jim).is_ok());
        assert!(parse_formal_parameters_in("n::x a(k)", Tcl).is_err());
        assert_eq!(
            parse_formal_parameters_in("{\"x\"}", Jim).unwrap()[0].name,
            "\"x\""
        );
        let parameters =
            parse_formal_parameters_in("{\"a b\"} {\"c d\" DEFAULT} args", Tcl).unwrap();
        assert_eq!(
            formal_parameter_usage(&parameters, Tcl),
            "{a b} {?c d?} ?arg ...?"
        );
        let parameters = parse_formal_parameters_in("&v {args rest} required", Jim).unwrap();
        assert_eq!(
            formal_parameter_usage(&parameters, Jim),
            "v ?rest ...? required"
        );
    }

    #[test]
    fn parses_names_defaults_and_trailing_args() {
        let parameters = parse_formal_parameters("a {b {hello world}} args").unwrap();
        assert_eq!(
            parameters,
            vec![
                FormalParameter {
                    name: "a".to_string(),
                    default: None,
                },
                FormalParameter {
                    name: "b".to_string(),
                    default: Some("hello world".to_string()),
                },
                FormalParameter {
                    name: "args".to_string(),
                    default: None,
                },
            ]
        );
        assert!(has_trailing_args(&parameters));
        assert!(!has_trailing_args(&parameters[..2]));

        let defaulted_args = parse_formal_parameters("{args fallback}").unwrap();
        assert!(has_trailing_args(&defaulted_args));
        let non_trailing_args = parse_formal_parameters("args value").unwrap();
        assert!(!has_trailing_args(&non_trailing_args));
    }

    #[test]
    fn distinguishes_zero_empty_and_too_many_fields() {
        assert_eq!(
            parse_formal_parameters("").unwrap(),
            [] as [crate::formal_params::FormalParameter; 0]
        );

        let no_fields = parse_formal_parameters("{}").unwrap_err();
        assert_eq!(no_fields, FormalParameterError::NoFields);
        assert_eq!(no_fields.message(), "argument with no name");

        let empty_name = parse_formal_parameters("{{} default}").unwrap_err();
        assert_eq!(empty_name, FormalParameterError::EmptyName);
        assert_eq!(empty_name.message(), "argument with no name");

        assert_eq!(
            parse_formal_parameters("{a b c}").unwrap_err(),
            FormalParameterError::TooManyFields {
                specifier: "a b c".to_string(),
            }
        );
    }

    #[test]
    fn overlong_specifier_repair_splits_only_the_rejected_group() {
        let source = "first {second default} {a b c} args";
        let error = parse_formal_parameters(source).expect_err("overlong specifier");
        let repaired = split_overlong_parameter_specifier(source, &error)
            .expect("the structural repair is available");

        assert_eq!(repaired, "first {second default} a b c args");
        assert!(parse_formal_parameters(&repaired).is_ok());
        assert!(
            split_overlong_parameter_specifier(
                "a(x)",
                &FormalParameterError::ArrayElement {
                    name: "a(x)".to_owned()
                }
            )
            .is_none()
        );
    }

    #[test]
    fn distinguishes_invalid_scalar_names() {
        assert_eq!(
            parse_formal_parameters("{a(1)}").unwrap_err(),
            FormalParameterError::ArrayElement {
                name: "a(1)".to_string(),
            }
        );
        assert_eq!(
            parse_formal_parameters("{a::b}").unwrap_err(),
            FormalParameterError::NotSimpleName {
                name: "a::b".to_string(),
            }
        );
        // Parentheses that do not form a trailing array-element spelling are
        // ordinary scalar-name bytes, matching TclCreateProc's scan.
        assert!(parse_formal_parameters("{a(}").is_ok());
    }

    #[test]
    fn retains_list_error_level_and_input() {
        let error = parse_formal_parameters("{a").unwrap_err();
        assert!(matches!(
            error,
            FormalParameterError::InvalidList {
                level: ParameterListLevel::Parameters,
                error: ListError::UnmatchedBrace,
                ..
            }
        ));

        let error = parse_formal_parameters("{{a b}x}").unwrap_err();
        assert!(matches!(
            error,
            FormalParameterError::InvalidList {
                level: ParameterListLevel::Specifier,
                error: ListError::BraceFollowedByJunk,
                ..
            }
        ));
    }
    #[test]
    fn formal_count_shapes_preserve_c_and_jim_default_and_rest_grammars() {
        // naming.procedure.original-formal-count-shape
        // docs/design/analysis/name-resolution-proofs/procedure-original-formal-count-shape.md
        use tcl_dialect::ParameterGrammar::{Jim, Tcl};
        for (fields, c, jim) in [
            (
                vec![(false, false), (false, true), (false, false)],
                (3, Some(3)),
                (2, Some(3)),
            ),
            (
                vec![(false, false), (true, false), (false, false)],
                (3, Some(3)),
                (2, None),
            ),
            (
                vec![(false, false), (true, true), (false, false)],
                (3, Some(3)),
                (2, None),
            ),
            (vec![(false, false), (true, false)], (1, None), (1, None)),
            (
                vec![(false, true), (false, true)],
                (0, Some(2)),
                (0, Some(2)),
            ),
        ] {
            for (grammar, (minimum, maximum)) in [(Tcl, c), (Jim, jim)] {
                assert_eq!(
                    formal_argument_count_shape(fields.iter().copied(), grammar),
                    FormalArgumentCountShape { minimum, maximum }
                );
            }
        }
    }
    #[test]
    fn native_formal_acceptance_matrix_keeps_provider_count_answers() {
        // naming.procedure.original-formal-count-shape
        // docs/design/analysis/name-resolution-proofs/procedure-original-formal-count-shape.md
        // These are the caught definition/call answers for the original ASCII
        // probe. Counts above five, values and runtime frames are not observed.
        use tcl_dialect::ParameterGrammar::{Jim, Tcl};
        for (source, c, jim) in [
            (
                "",
                [true, false, false, false, false, false],
                [true, false, false, false, false, false],
            ),
            (
                "a {b 2} c",
                [false, false, false, true, false, false],
                [false, false, true, true, false, false],
            ),
            (
                "a args b",
                [false, false, false, true, false, false],
                [false, false, true, true, true, true],
            ),
            (
                "a {args tail} b",
                [false, false, false, true, false, false],
                [false, false, true, true, true, true],
            ),
            (
                "a args",
                [false, true, true, true, true, true],
                [false, true, true, true, true, true],
            ),
            (
                "a {b 2} args {c 3}",
                [false, false, false, true, true, false],
                [false, true, true, true, true, true],
            ),
            (
                "{a 1} {b 2}",
                [true, true, true, false, false, false],
                [true, true, true, false, false, false],
            ),
        ] {
            for (grammar, expected) in [(Tcl, c), (Jim, jim)] {
                let parameters = parse_formal_parameters_in(source, grammar)
                    .expect("original definition accepted");
                for (count, accepted) in expected.into_iter().enumerate() {
                    assert_eq!(
                        bind_formal_arguments(&parameters, count, grammar).is_ok(),
                        accepted,
                        "original provider count answer: {source:?}, {grammar:?}, argv {count}"
                    );
                }
            }
        }
        for grammar in [Tcl, Jim] {
            assert!(
                matches!(
                    parse_formal_parameters_in("{a b c}", grammar),
                    Err(FormalParameterError::TooManyFields { .. })
                ),
                "original definition refused"
            );
        }
    }
}
