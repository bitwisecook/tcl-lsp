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

//! The emitter protocol: the verbs a hook body calls, as native host commands.
//!
//! `docs/design/spec-dsl-examples/README.md`, "Outputs: the emitter protocol":
//! **every hook's own return value is ignored**; each family injects one to
//! three verbs, and calling none is an abstention. This module is that table,
//! executable — one [`HostCommand`] per verb, all writing into one
//! [`Sink`], plus the conversion from what the sink collected to the registry's
//! typed answer.
//!
//! A verb that is called wrongly (a bad index, an unknown role name) raises,
//! and raising is an abstention — the same conservative answer an empty sink
//! gives. That is deliberate: a body cannot half-answer.

use std::cell::RefCell;
use std::rc::Rc;

use tcl_engine_api::{EngineError, HostCommand, Value};
use tcl_registry::arg_role::{AppendedArity, ArgRole};
use tcl_registry::clause_shape::ClauseShapeError;
use tcl_registry::literal_validation::{
    LiteralArgumentIssue, LiteralArgumentIssueReason, LiteralArgumentValidation,
    LiteralValidationDecline,
};
use tcl_registry::pack_hooks::{HookAnswer, HookFamily};

use crate::intern::{intern, intern_words};

/// One thing a body emitted, before the family folds them into an answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Emission {
    /// `role IDX ROLE`.
    Role(u8, ArgRole),
    /// `prefix IDX {Exactly N}`.
    Prefix(u8, AppendedArity),
    /// `fold VALUE`.
    Fold(String),
    /// `sink-applies`.
    SinkApplies,
    /// `sink-suppressed`.
    SinkSuppressed,
    /// `reject MESSAGE`.
    Reject(String),
    /// `invalid …`.
    Invalid(LiteralArgumentIssue),
    /// `abstain REASON`.
    Abstain(LiteralValidationDecline),
    /// `missing-expr ?after?`.
    MissingExpr(Option<usize>),
    /// `missing-body after`.
    MissingBody(usize),
    /// `extra-words first`.
    ExtraWords(usize),
    /// `consume N ?-invalid MESSAGE?`.
    Consume {
        /// Words consumed.
        words: usize,
        /// The W141 message, when the value is rejected.
        invalid: Option<String>,
    },
}

/// Where every verb of one invocation writes.
///
/// Shared by `Rc` with each verb command, and drained by the host after the
/// body returns. Interior mutability because a host command sees `&self` — the
/// engine is re-entrant, and a verb must not be able to hold a lock across a
/// nested evaluation.
#[derive(Debug, Default)]
pub struct Sink {
    emissions: RefCell<Vec<Emission>>,
}

impl Sink {
    /// Take everything emitted, leaving the sink empty for the next
    /// invocation.
    pub fn drain(&self) -> Vec<Emission> {
        std::mem::take(&mut self.emissions.borrow_mut())
    }

    /// Discard everything emitted — used when an invocation failed, so a
    /// partial emission from a body that then errored cannot be read as an
    /// answer.
    pub fn clear(&self) {
        self.emissions.borrow_mut().clear();
    }

    fn push(&self, emission: Emission) {
        self.emissions.borrow_mut().push(emission);
    }
}

/// One emitter verb, bound to the sink of the invocation that will read it.
struct Verb {
    verb: &'static str,
    sink: Rc<Sink>,
}

fn text(value: &Value) -> String {
    value.as_str().map_or_else(
        || match value {
            Value::Int(number) => number.to_string(),
            Value::Double(number) => number.to_string(),
            _ => String::new(),
        },
        str::to_owned,
    )
}

fn misuse(verb: &str, detail: &str) -> EngineError {
    EngineError::Script {
        message: format!("{verb}: {detail}"),
        code: None,
    }
}

fn index_of(verb: &str, value: &Value) -> Result<usize, EngineError> {
    text(value)
        .parse::<usize>()
        .map_err(|_| misuse(verb, "expected a word index"))
}

/// Parse an `AppendedArity` the way the DSL spells it: `{Exactly N}`,
/// `{AtLeast N}`, or `Unknown`.
fn appended_arity(verb: &str, value: &Value) -> Result<AppendedArity, EngineError> {
    let words: Vec<String> = value.as_list().map_or_else(
        || text(value).split_whitespace().map(str::to_owned).collect(),
        |items| items.iter().map(text).collect(),
    );
    let count = |raw: Option<&String>| -> Result<u8, EngineError> {
        raw.ok_or_else(|| misuse(verb, "expected an arity count"))?
            .parse::<u8>()
            .map_err(|_| misuse(verb, "expected an arity count"))
    };
    match words.first().map(String::as_str) {
        Some("Exactly") => Ok(AppendedArity::Exactly(count(words.get(1))?)),
        Some("AtLeast") => Ok(AppendedArity::AtLeast(count(words.get(1))?)),
        Some("Unknown") => Ok(AppendedArity::Unknown),
        _ => Err(misuse(verb, "expected Exactly N, AtLeast N, or Unknown")),
    }
}

/// Resolve a role by the spelling the registry's own catalogue uses — a
/// fieldless enum's `Debug` output *is* its catalogue name, which is how the
/// loader spells one too, so the emitter verb and the `-role` flag cannot
/// drift apart. Matched case-insensitively: `VarWrite` and `varwrite` both
/// name the same role, and the difference is not worth an abstention.
fn role_by_name(verb: &str, name: &str) -> Result<ArgRole, EngineError> {
    ArgRole::ALL
        .iter()
        .copied()
        .find(|role| format!("{role:?}").eq_ignore_ascii_case(name))
        .ok_or_else(|| misuse(verb, &format!("unknown argument role \"{name}\"")))
}

impl HostCommand for Verb {
    fn invoke(&self, arguments: &[Value]) -> Result<Value, EngineError> {
        let emission = match self.verb {
            "role" => {
                let [index, role] = arguments else {
                    return Err(misuse(self.verb, "expected IDX ROLE"));
                };
                let index = u8::try_from(index_of(self.verb, index)?)
                    .map_err(|_| misuse(self.verb, "word index out of range"))?;
                Emission::Role(index, role_by_name(self.verb, &text(role))?)
            }
            "prefix" => {
                let [index, arity] = arguments else {
                    return Err(misuse(self.verb, "expected IDX ARITY"));
                };
                let index = u8::try_from(index_of(self.verb, index)?)
                    .map_err(|_| misuse(self.verb, "word index out of range"))?;
                Emission::Prefix(index, appended_arity(self.verb, arity)?)
            }
            "fold" => {
                let [value] = arguments else {
                    return Err(misuse(self.verb, "expected VALUE"));
                };
                Emission::Fold(text(value))
            }
            "sink-applies" => Emission::SinkApplies,
            "sink-suppressed" => Emission::SinkSuppressed,
            "reject" => {
                let [message] = arguments else {
                    return Err(misuse(self.verb, "expected MESSAGE"));
                };
                Emission::Reject(text(message))
            }
            "invalid" => invalid_emission(arguments)?,
            "abstain" => {
                let [reason] = arguments else {
                    return Err(misuse(self.verb, "expected REASON"));
                };
                Emission::Abstain(decline_by_name(&text(reason))?)
            }
            "missing-expr" => match arguments {
                [] => Emission::MissingExpr(None),
                [after] => Emission::MissingExpr(Some(index_of(self.verb, after)?)),
                _ => return Err(misuse(self.verb, "expected ?AFTER?")),
            },
            "missing-body" => {
                let [after] = arguments else {
                    return Err(misuse(self.verb, "expected AFTER"));
                };
                Emission::MissingBody(index_of(self.verb, after)?)
            }
            "extra-words" => {
                let [first] = arguments else {
                    return Err(misuse(self.verb, "expected FIRST"));
                };
                Emission::ExtraWords(index_of(self.verb, first)?)
            }
            "consume" => consume_emission(arguments)?,
            other => return Err(misuse(other, "not an emitter verb of this family")),
        };
        self.sink.push(emission);
        Ok(Value::Empty)
    }
}

fn decline_by_name(reason: &str) -> Result<LiteralValidationDecline, EngineError> {
    match reason {
        "incomplete-invocation" => Ok(LiteralValidationDecline::IncompleteInvocation),
        "non-literal-argument" => Ok(LiteralValidationDecline::NonLiteralArgument),
        "invalid-discriminator" => Ok(LiteralValidationDecline::InvalidDiscriminator),
        "malformed-literal" => Ok(LiteralValidationDecline::MalformedLiteral),
        other => Err(misuse(
            "abstain",
            &format!("unknown abstention reason \"{other}\""),
        )),
    }
}

/// `invalid -index N -subject S -reason … -allowed {…} ?-replacement V?`
fn invalid_emission(arguments: &[Value]) -> Result<Emission, EngineError> {
    let mut index = None;
    let mut subject = None;
    let mut reason = None;
    let mut members: Vec<String> = Vec::new();
    let mut allowed: Vec<String> = Vec::new();
    let mut replacement = None;
    let mut rest = arguments.iter();
    while let Some(flag) = rest.next() {
        let flag = text(flag);
        let value = rest
            .next()
            .ok_or_else(|| misuse("invalid", &format!("{flag} takes a value")))?;
        match flag.as_str() {
            "-index" => index = Some(index_of("invalid", value)?),
            "-subject" => subject = Some(text(value)),
            "-reason" => reason = Some(text(value)),
            "-members" => members = word_list(value),
            "-allowed" => allowed = word_list(value),
            "-replacement" => replacement = Some(text(value)),
            other => {
                return Err(misuse(
                    "invalid",
                    &format!("unknown flag \"{other}\""),
                ));
            }
        }
    }
    let reason = match reason.as_deref() {
        Some("empty") => LiteralArgumentIssueReason::Empty,
        Some("invalid-members") | None => LiteralArgumentIssueReason::InvalidMembers(members),
        Some(other) => {
            return Err(misuse(
                "invalid",
                &format!("unknown reason \"{other}\""),
            ));
        }
    };
    Ok(Emission::Invalid(LiteralArgumentIssue {
        argument_index: index.ok_or_else(|| misuse("invalid", "-index is required"))?,
        subject: intern(&subject.ok_or_else(|| misuse("invalid", "-subject is required"))?),
        reason,
        allowed_values: intern_words(&allowed),
        replacement_value: replacement,
    }))
}

/// `consume N ?-invalid MESSAGE?`
fn consume_emission(arguments: &[Value]) -> Result<Emission, EngineError> {
    let [count, rest @ ..] = arguments else {
        return Err(misuse("consume", "expected N ?-invalid MESSAGE?"));
    };
    let words = index_of("consume", count)?;
    let invalid = match rest {
        [] => None,
        [flag, message] if text(flag) == "-invalid" => Some(text(message)),
        _ => return Err(misuse("consume", "expected N ?-invalid MESSAGE?")),
    };
    Ok(Emission::Consume { words, invalid })
}

fn word_list(value: &Value) -> Vec<String> {
    value.as_list().map_or_else(
        || {
            text(value)
                .split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        },
        |items| items.iter().map(text).collect(),
    )
}

/// Build the verb commands a family injects, all sharing `sink`.
pub fn verbs_for(family: HookFamily, sink: &Rc<Sink>) -> Vec<(&'static str, Rc<dyn HostCommand>)> {
    family
        .verbs()
        .iter()
        .map(|verb| {
            let command: Rc<dyn HostCommand> = Rc::new(Verb {
                verb,
                sink: Rc::clone(sink),
            });
            (*verb, command)
        })
        .collect()
}

/// Fold what a body emitted into the family's answer, applying that family's
/// silence to an empty sink.
#[must_use]
pub fn answer_of(family: HookFamily, emissions: Vec<Emission>) -> HookAnswer {
    match family {
        HookFamily::ArgRoleResolver => {
            let roles: Vec<(u8, ArgRole)> = emissions
                .into_iter()
                .filter_map(|emission| match emission {
                    Emission::Role(index, role) => Some((index, role)),
                    _ => None,
                })
                .collect();
            if roles.is_empty() {
                HookAnswer::Abstain
            } else {
                HookAnswer::Roles(roles)
            }
        }
        HookFamily::CommandPrefixResolver => {
            let prefixes: Vec<(u8, AppendedArity)> = emissions
                .into_iter()
                .filter_map(|emission| match emission {
                    Emission::Prefix(index, arity) => Some((index, arity)),
                    _ => None,
                })
                .collect();
            if prefixes.is_empty() {
                HookAnswer::Abstain
            } else {
                HookAnswer::Prefixes(prefixes)
            }
        }
        HookFamily::ConstFold | HookFamily::ConstFoldVersioned => emissions
            .into_iter()
            .find_map(|emission| match emission {
                Emission::Fold(value) => Some(HookAnswer::Fold(value)),
                _ => None,
            })
            .unwrap_or(HookAnswer::Abstain),
        // Silence means the sink applies, and so does an explicit
        // `sink-applies`; only `sink-suppressed` turns it off.
        HookFamily::TaintSinkGate => emissions
            .into_iter()
            .find_map(|emission| match emission {
                Emission::SinkSuppressed => Some(HookAnswer::SinkSuppressed),
                _ => None,
            })
            .unwrap_or(HookAnswer::Abstain),
        HookFamily::ContextGate => emissions
            .into_iter()
            .find_map(|emission| match emission {
                Emission::Reject(message) => Some(HookAnswer::Reject(message)),
                _ => None,
            })
            .unwrap_or(HookAnswer::Abstain),
        HookFamily::LiteralArgumentValidator => emissions
            .into_iter()
            .find_map(|emission| match emission {
                Emission::Invalid(issue) => Some(HookAnswer::Literal(
                    LiteralArgumentValidation::Invalid(issue),
                )),
                Emission::Abstain(decline) => Some(HookAnswer::Literal(
                    LiteralArgumentValidation::Abstain(decline),
                )),
                _ => None,
            })
            .unwrap_or(HookAnswer::Abstain),
        HookFamily::ClauseShapeCheck => emissions
            .into_iter()
            .find_map(|emission| match emission {
                Emission::MissingExpr(after) => Some(HookAnswer::ClauseShape(
                    ClauseShapeError::MissingExpr { after },
                )),
                Emission::MissingBody(after) => Some(HookAnswer::ClauseShape(
                    ClauseShapeError::MissingBody { after },
                )),
                Emission::ExtraWords(first_extra) => Some(HookAnswer::ClauseShape(
                    ClauseShapeError::ExtraWords { first_extra },
                )),
                _ => None,
            })
            .unwrap_or(HookAnswer::Abstain),
        HookFamily::OptionArity => emissions
            .into_iter()
            .find_map(|emission| match emission {
                Emission::Consume { words, invalid } => {
                    Some(HookAnswer::Consume { words, invalid })
                }
                _ => None,
            })
            .unwrap_or(HookAnswer::Abstain),
    }
}
