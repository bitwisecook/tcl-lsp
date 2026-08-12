// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Strict, interpreter-independent parsing of Tcl formal parameter lists.
//!
//! A `proc`, method, or `apply` parameter list has two list levels.  The outer
//! list contains parameter specifiers; each specifier is itself a zero-, one-,
//! two-, or many-element Tcl list.  This module owns that shared grammar and
//! the scalar/simple-name validation performed by Tcl when creating a proc.
//! Consumers remain responsible for converting strings into their own value
//! representation and for rendering errors at their byte or object boundary.

use crate::list::{ListError, join_list, split_list};

/// One decoded formal parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormalParameter {
    /// The scalar, non-namespace-qualified parameter name.
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
    /// Render the reference Tcl diagnostic as UTF-8 text.
    ///
    /// Byte-oriented runtimes can convert this result at their API boundary;
    /// keeping rendering here makes the semantic variants useful to compilers
    /// without coupling the parser to an interpreter object representation.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::InvalidList { input, error, .. } => error.full_message(input),
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
    let specs = split_list(source).map_err(|error| FormalParameterError::InvalidList {
        level: ParameterListLevel::Parameters,
        input: source.to_string(),
        error,
    })?;
    let mut parameters = Vec::with_capacity(specs.len());
    for spec in specs {
        let fields = split_list(&spec).map_err(|error| FormalParameterError::InvalidList {
            level: ParameterListLevel::Specifier,
            input: spec.to_string(),
            error,
        })?;
        let (name, default) = match fields.as_slice() {
            [] => return Err(FormalParameterError::NoFields),
            [name] => (name.as_ref(), None),
            [name, default] => (name.as_ref(), Some(default.as_ref())),
            _ => {
                return Err(FormalParameterError::TooManyFields {
                    specifier: spec.to_string(),
                });
            }
        };
        validate_name(name)?;
        parameters.push(FormalParameter {
            name: name.to_string(),
            default: default.map(str::to_string),
        });
    }
    Ok(parameters)
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
        assert!(parse_formal_parameters("").unwrap().is_empty());

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
}
