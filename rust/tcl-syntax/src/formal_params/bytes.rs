// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Byte formal names and object-preserving native activation plans.

use tcl_dialect::{ParameterGrammar, TclVersion};

use super::ParameterListLevel;
use crate::naming::NativeNameProtocol;

/// A decoded formal whose default retains the adapter's original value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByteFormalParameter<V = Vec<u8>> {
    /// Actual native formal storage key, distinct from its enumeration spelling.
    pub name: Vec<u8>,
    /// Original decoded default object, when present.
    pub default: Option<V>,
}

/// A structural formal failure retaining exact decoded operand bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ByteFormalParameterError {
    /// An empty parameter specifier.
    NoFields,
    /// An empty decoded formal name.
    EmptyName,
    /// More than two decoded fields.
    TooManyFields(Vec<u8>),
    /// A C formal names an array element.
    ArrayElement(Vec<u8>),
    /// A C formal contains a namespace separator.
    NotSimpleName(Vec<u8>),
    /// Jim permits only one variadic `args` formal.
    DuplicateArgs,
}

impl ByteFormalParameterError {
    /// Render the native definition diagnostic without repairing operand bytes.
    #[must_use]
    pub fn message_for_definition(
        &self,
        protocol: NativeNameProtocol,
        procedure: &[u8],
    ) -> Vec<u8> {
        let old_c = protocol.tcl_version() == Some(TclVersion::V8_4);
        let mut out = Vec::new();
        if old_c
            && matches!(
                self,
                Self::NoFields | Self::EmptyName | Self::ArrayElement(_) | Self::NotSimpleName(_)
            )
        {
            out.extend_from_slice(b"procedure \"");
            out.extend_from_slice(c_string(procedure));
            out.extend_from_slice(b"\" has ");
        }
        match self {
            Self::NoFields | Self::EmptyName => out.extend_from_slice(b"argument with no name"),
            Self::DuplicateArgs => out.extend_from_slice(b"'args' specified more than once"),
            Self::TooManyFields(specifier) => {
                out.extend_from_slice(b"too many fields in argument specifier \"");
                out.extend_from_slice(specifier);
                out.push(b'"');
            }
            Self::ArrayElement(name) | Self::NotSimpleName(name) => {
                out.extend_from_slice(b"formal parameter \"");
                // C's array-element diagnostic uses %s; its simple-name
                // diagnostic appends the counted object on modern releases.
                let name = if old_c || matches!(self, Self::ArrayElement(_)) {
                    c_string(name)
                } else {
                    name
                };
                out.extend_from_slice(name);
                out.extend_from_slice(if matches!(self, Self::ArrayElement(_)) {
                    if old_c {
                        b"\" that is an array element"
                    } else {
                        b"\" is an array element"
                    }
                } else if old_c {
                    b"\" that is not a simple name"
                } else {
                    b"\" is not a simple name"
                });
            }
        }
        out
    }
}

fn c_string(bytes: &[u8]) -> &[u8] {
    &bytes[..bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len())]
}

/// A value conversion failure or a native formal format failure.
#[derive(Debug)]
pub enum FormalParameterValueError<E> {
    /// Preserve the adapter's list/string failure and its actual evaluation point.
    Access(E),
    /// Native name/field validation failed.
    Format(ByteFormalParameterError),
}

/// Decode already-selected outer specifier objects using the native field
/// splitter. Defaults are returned unchanged; this owner never flattens them.
/// The caller selects C8.4/C8.5 `CString` list parsing or modern object parsing
/// before invoking this structural owner.
///
/// # Errors
/// Returns original adapter failures or exact native formal format failures.
pub fn parse_formal_parameter_values<V: Clone, E>(
    specifiers: &[V],
    protocol: NativeNameProtocol,
    mut split: impl FnMut(&V, ParameterListLevel) -> Result<Vec<V>, E>,
    mut bytes: impl FnMut(&V) -> Result<Vec<u8>, E>,
) -> Result<Vec<ByteFormalParameter<V>>, FormalParameterValueError<E>> {
    let mut parsed = Vec::with_capacity(specifiers.len());
    let mut args_seen = false;
    for specifier in specifiers {
        let fields = split(specifier, ParameterListLevel::Specifier)
            .map_err(FormalParameterValueError::Access)?;
        let (name, default) = match fields.as_slice() {
            [] => {
                return Err(FormalParameterValueError::Format(
                    ByteFormalParameterError::NoFields,
                ));
            }
            [name] => (
                if protocol.is_jim084() {
                    specifier
                } else {
                    name
                },
                None,
            ),
            [name, default] => (name, Some(default.clone())),
            _ => {
                return Err(FormalParameterValueError::Format(
                    ByteFormalParameterError::TooManyFields(
                        bytes(specifier).map_err(FormalParameterValueError::Access)?,
                    ),
                ));
            }
        };
        let written = bytes(name).map_err(FormalParameterValueError::Access)?;
        let name = protocol
            .formal_storage_name_input(&written)
            .selected()
            .to_vec();
        if protocol.is_jim084() {
            if name == b"args" {
                if args_seen {
                    return Err(FormalParameterValueError::Format(
                        ByteFormalParameterError::DuplicateArgs,
                    ));
                }
                args_seen = true;
            }
        } else {
            validate_name(&name).map_err(FormalParameterValueError::Format)?;
        }
        parsed.push(ByteFormalParameter { name, default });
    }
    Ok(parsed)
}

fn validate_name(name: &[u8]) -> Result<(), ByteFormalParameterError> {
    if name.is_empty() {
        return Err(ByteFormalParameterError::EmptyName);
    }
    for index in 0..name.len().saturating_sub(1) {
        if name[index] == b'(' && name.last() == Some(&b')') {
            return Err(ByteFormalParameterError::ArrayElement(name.to_vec()));
        }
        if name[index..].starts_with(b"::") {
            return Err(ByteFormalParameterError::NotSimpleName(name.to_vec()));
        }
    }
    Ok(())
}

/// Native formal activation with byte-valued local names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormalByteArgumentBinding {
    /// Bind supplied argv to this formal.
    Value {
        /// Parsed formal index.
        parameter: usize,
        /// Post-head argv index.
        argument: usize,
    },
    /// Bind the original default object.
    Default {
        /// Parsed formal index.
        parameter: usize,
    },
    /// Bind surplus arguments to the native rest name.
    Rest {
        /// Parsed formal index.
        parameter: usize,
        /// Actual local name, including Jim renamed args.
        name: Vec<u8>,
        /// First surplus argv index.
        start: usize,
        /// Surplus argv count.
        len: usize,
    },
    /// Link to the existing caller variable named by this argv value.
    CallerLink {
        /// Parsed formal index.
        parameter: usize,
        /// Local name without Jim's reference marker.
        name: Vec<u8>,
        /// Post-head argv index.
        argument: usize,
    },
}

/// Plan native formal activation without reading or reconstructing values.
/// Defaults in this signature are native string bytes only because Jim can
/// use the `args` default as a local name; ordinary default values stay with
/// the adapter and are identified by their parameter index.
///
/// # Errors
/// Returns an arity error when the native formal layout cannot accept argv.
pub fn bind_formal_argument_bytes(
    parameters: &[ByteFormalParameter],
    count: usize,
    grammar: ParameterGrammar,
) -> Result<Vec<FormalByteArgumentBinding>, super::FormalArityError> {
    use FormalByteArgumentBinding as Binding;
    let jim = grammar == ParameterGrammar::Jim;
    let required = parameters
        .iter()
        .filter(|p| p.name != b"args" && p.default.is_none())
        .count();
    let optional = parameters
        .iter()
        .filter(|p| p.name != b"args" && p.default.is_some())
        .count();
    if jim
        && (count < required
            || (parameters.iter().all(|p| p.name != b"args") && count > required + optional))
    {
        return Err(super::FormalArityError);
    }
    let mut optional_left = count.saturating_sub(required);
    let mut argument = 0;
    let mut seen = std::collections::HashSet::new();
    let mut bindings = Vec::new();
    for (parameter, formal) in parameters.iter().enumerate() {
        let rest = formal.name == b"args" && (jim || parameter + 1 == parameters.len());
        let binding = if rest {
            let len = if jim {
                count.saturating_sub(required + optional)
            } else {
                count.saturating_sub(argument)
            };
            let name = if jim {
                formal.default.clone().unwrap_or_else(|| b"args".to_vec())
            } else {
                formal.name.clone()
            };
            let binding = Binding::Rest {
                parameter,
                name,
                start: argument,
                len,
            };
            argument += len;
            binding
        } else if argument < count && (!jim || formal.default.is_none() || optional_left > 0) {
            if jim && formal.default.is_some() {
                optional_left -= 1;
            }
            let binding = if jim && formal.name.starts_with(b"&") {
                Binding::CallerLink {
                    parameter,
                    name: formal.name[1..].to_vec(),
                    argument,
                }
            } else {
                Binding::Value {
                    parameter,
                    argument,
                }
            };
            argument += 1;
            binding
        } else if formal.default.is_some() {
            Binding::Default { parameter }
        } else {
            return Err(super::FormalArityError);
        };
        if jim || seen.insert(formal.name.as_slice()) {
            bindings.push(binding);
        }
    }
    if argument != count {
        return Err(super::FormalArityError);
    }
    Ok(bindings)
}

/// Render the actual byte-valued formal usage suffix.
#[must_use]
pub fn formal_parameter_usage_bytes(
    parameters: &[ByteFormalParameter],
    grammar: ParameterGrammar,
) -> Vec<u8> {
    let mut out = Vec::new();
    for (index, parameter) in parameters.iter().enumerate() {
        if index != 0 {
            out.push(b' ');
        }
        if parameter.name == b"args"
            && (grammar == ParameterGrammar::Jim || index + 1 == parameters.len())
        {
            out.push(b'?');
            out.extend_from_slice(if grammar == ParameterGrammar::Jim {
                parameter.default.as_deref().unwrap_or(b"arg")
            } else {
                b"arg"
            });
            out.extend_from_slice(b" ...?");
        } else {
            let name = if grammar == ParameterGrammar::Jim && parameter.default.is_none() {
                parameter.name.strip_prefix(b"&").unwrap_or(&parameter.name)
            } else {
                &parameter.name
            };
            let mut word = Vec::new();
            if parameter.default.is_some() {
                word.push(b'?');
            }
            word.extend_from_slice(name);
            if parameter.default.is_some() {
                word.push(b'?');
            }
            if grammar == ParameterGrammar::Tcl {
                crate::list::append_list_element(&mut out, &word, index == 0);
            } else {
                out.extend_from_slice(&word);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    #[test]
    fn parsed_storage_is_distinct_from_enumeration_and_keeps_default_object() {
        for version in TclVersion::ALL {
            let protocol = NativeNameProtocol::for_tcl_version(version);
            let name = Rc::new(b"k\0z".to_vec());
            let default = Rc::new(vec![0xff, 0, b'x']);
            let specifier = Rc::new(b"specifier".to_vec());
            let parsed = parse_formal_parameter_values(
                &[Rc::clone(&specifier)],
                protocol,
                |_, _| Ok::<_, ()>(vec![Rc::clone(&name), Rc::clone(&default)]),
                |value| Ok(value.as_ref().clone()),
            )
            .unwrap();
            let expected: &[u8] = if version <= TclVersion::V8_5 {
                b"k"
            } else {
                b"k\0z"
            };
            assert_eq!(parsed[0].name, expected);
            assert!(Rc::ptr_eq(parsed[0].default.as_ref().unwrap(), &default));
            assert_eq!(
                protocol
                    .formal_enumeration_name_input(&parsed[0].name)
                    .selected(),
                b"k"
            );
        }
    }

    #[test]
    fn byte_activation_keeps_distinct_keys_and_jim_reference_names() {
        let parameters = vec![
            ByteFormalParameter {
                name: b"k\0z".to_vec(),
                default: None,
            },
            ByteFormalParameter {
                name: b"k".to_vec(),
                default: None,
            },
        ];
        assert_eq!(
            bind_formal_argument_bytes(&parameters, 2, ParameterGrammar::Tcl)
                .unwrap()
                .len(),
            2
        );
        let reference = vec![ByteFormalParameter {
            name: vec![b'&', 0xff],
            default: None,
        }];
        assert_eq!(
            bind_formal_argument_bytes(&reference, 1, ParameterGrammar::Jim).unwrap(),
            vec![FormalByteArgumentBinding::CallerLink {
                parameter: 0,
                name: vec![0xff],
                argument: 0
            }]
        );
        assert_eq!(
            formal_parameter_usage_bytes(&parameters, ParameterGrammar::Tcl),
            b"k\0z k"
        );
    }
}
