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

//! Descriptor values a row carries: arity, integer domains, typing and representation effects, deprecation fixes, symbol definitions, semantic operations, handle bindings and object classes.

use super::super::statement_reader::{Stmt, Word, block};
use super::super::subcommand::load_subcommand;
use super::super::{HookDecl, HookSource, Log, PackTables, leak_slice, leak_str, leak_strs};
use super::lifecycles::{checked_lifecycle, lifecycle_flag};
use super::vocabularies::{BYTE_ARRAY_EFFECTS, INTRINSICS, LOWERING_HOOKS, TCL_TYPES};
use super::{by_name, enum_by_name, list_words, next_text};
use crate::catalogue;
use tcl_core_types::DiagCode;
use tcl_registry::abbrev::PrefixMatching;
use tcl_registry::arg_role::AppendedArity;
use tcl_registry::arity::Arity;
use tcl_registry::byte_array_effect::ByteArrayEffect;
use tcl_registry::deprecation::{DeprecationFixHook, DeprecationFixSafety};
use tcl_registry::handle_binding::{
    HandleBindingSpec, HandleClassSource, HandleKeyword, HandleName,
};
use tcl_registry::hover::IntegerDomain;
use tcl_registry::lifecycle::Lifecycle;
use tcl_registry::representation::RepresentationEffect;
use tcl_registry::semantic_operation::SemanticOperationId;
use tcl_registry::spec::{BytePayloadSpec, SubCommand};
use tcl_registry::symbol_def::{DefinedSymbolKind, SymbolDef};
use tcl_registry::taint::SetterConstraint;
use tcl_registry::types::{ReturnElements, VarElementsEffect, VarWriteTyping};

/// `arity 3.. -step 2 -also 2` and its five simpler spellings, optionally
/// gated by the three lifecycle flags (`SpecTcl` 1.2).
///
/// Returns the shape, and the lifecycle when the row carried one. An ungated
/// row is the command's plain arity exactly as before 1.2; a gated one is one
/// *window* of a signature that changed across the owning package's releases,
/// and several may be declared for the same command.
pub(in super::super) fn parse_arity(stmt: &Stmt, log: &mut Log) -> (Arity, Option<Lifecycle>) {
    let range = stmt.word_text(1);
    let mut arity = match range.split_once("..") {
        None => range.parse::<u16>().map_or_else(
            |_| {
                log.say(
                    stmt.line,
                    format!("unreadable arity `{range}`; treated as any"),
                );
                Arity::any()
            },
            Arity::exact,
        ),
        Some((low, high)) => {
            let min = if low.is_empty() {
                0
            } else {
                low.parse().unwrap_or(0)
            };
            let max = if high.is_empty() {
                Arity::UNLIMITED
            } else {
                high.parse().unwrap_or(Arity::UNLIMITED)
            };
            Arity::new(min, max)
        }
    };
    let words = &stmt.words;
    let mut lifecycle = Lifecycle::UNSPECIFIED;
    let mut gated = false;
    let mut i = 2;
    while i < words.len() {
        let rejected_flag_word = words[i].clone();
        let flag = words[i].text.clone();
        if lifecycle_flag(&mut lifecycle, &flag, words, &mut i) {
            log.v12(stmt.line, &flag);
            gated = true;
            i += 1;
            continue;
        }
        match flag.as_str() {
            "-positionals" => {
                arity = arity.with_positionals();
                log.v21(stmt.line, "-positionals");
            }
            "-step" => arity.step = next_text(words, &mut i).parse().unwrap_or(0),
            "-also" => arity.also_exact = next_text(words, &mut i).parse().ok(),
            _ => log.unknown_flag("arity", stmt.line, &rejected_flag_word),
        }
        i += 1;
    }
    if !gated {
        return (arity, None);
    }
    let lifecycle = checked_lifecycle(lifecycle, "arity window", stmt.line, log);
    // A lifecycle rejected as impossibly ordered comes back UNSPECIFIED, which
    // as a window would silently cover every release and shadow the ones that
    // are well formed. The row keeps its shape and loses only the gate, so it
    // becomes the plain arity — the same degradation every other rejected
    // lifecycle takes.
    if lifecycle == Lifecycle::UNSPECIFIED {
        return (arity, None);
    }
    (arity, Some(lifecycle))
}

/// `{Range 2 max}`, `Any`, `Port` — `max` / `min` are the integer sentinels.
pub(in super::super) fn parse_integer_domain(
    text: &str,
    line: u32,
    log: &mut Log,
) -> Option<IntegerDomain> {
    let parts = list_words(text);
    match parts.split_first() {
        Some((head, rest)) if head == "Range" && rest.len() == 2 => {
            let bound = |word: &str, sentinel: i64| -> i64 {
                match word {
                    "max" => i64::MAX,
                    "min" => i64::MIN,
                    other => other.parse().unwrap_or(sentinel),
                }
            };
            Some(IntegerDomain::Range(
                bound(&rest[0], i64::MIN),
                bound(&rest[1], i64::MAX),
            ))
        }
        Some((head, rest)) if head == "Any" && rest.is_empty() => Some(IntegerDomain::Any),
        Some((head, rest)) if head == "Port" && rest.is_empty() => Some(IntegerDomain::Port),
        _ => {
            log.say(line, format!("unreadable integer domain `{text}` dropped"));
            None
        }
    }
}

/// `{Exactly N}` / `{AtLeast N}` / `Unknown`.
pub(in super::super) fn parse_appended_arity(
    text: &str,
    line: u32,
    log: &mut Log,
) -> Option<AppendedArity> {
    let parts = list_words(text);
    match parts.split_first() {
        Some((head, rest)) if head == "Exactly" && rest.len() == 1 => {
            rest[0].parse().ok().map(AppendedArity::Exactly)
        }
        Some((head, rest)) if head == "AtLeast" && rest.len() == 1 => {
            rest[0].parse().ok().map(AppendedArity::AtLeast)
        }
        Some((head, rest)) if head == "Unknown" && rest.is_empty() => Some(AppendedArity::Unknown),
        _ => {
            log.say(line, format!("unreadable appended arity `{text}` dropped"));
            None
        }
    }
}

/// `ReturnValue` / `Destructured` / `{Fixed T}` / `{ElementsOf N}`.
pub(in super::super) fn parse_var_write_typing(
    text: &str,
    line: u32,
    log: &mut Log,
) -> Option<VarWriteTyping> {
    let parts = list_words(text);
    match parts.split_first() {
        Some((head, rest)) if head == "ReturnValue" && rest.is_empty() => {
            Some(VarWriteTyping::ReturnValue)
        }
        Some((head, rest)) if head == "Destructured" && rest.is_empty() => {
            Some(VarWriteTyping::Destructured)
        }
        Some((head, rest)) if head == "Fixed" && rest.len() == 1 => {
            by_name(TCL_TYPES, &rest[0]).map(VarWriteTyping::Fixed)
        }
        Some((head, rest)) if head == "ElementsOf" && rest.len() == 1 => rest[0]
            .parse()
            .ok()
            .map(|container_arg| VarWriteTyping::ElementsOf { container_arg }),
        _ => {
            log.say(
                line,
                format!("unreadable var_write_typing `{text}` dropped"),
            );
            None
        }
    }
}

/// `{ListOfArgs N}` / `{DictOfPairs N}` / `{ElementOf N}` / `{SubListOf N}`.
///
/// The `{VARIANT payload …}` rule `README.md` gives for `var_write_typing`,
/// applied to the three element-structure facts that follow it in the coverage
/// matrix: a variant word, then its fields in declaration order.
pub(in super::super) fn parse_return_elements(
    text: &str,
    line: u32,
    log: &mut Log,
) -> Option<ReturnElements> {
    let parts = list_words(text);
    let index = |rest: &[String]| rest[0].parse::<u8>().ok();
    match parts.split_first() {
        Some((head, rest)) if head == "ListOfArgs" && rest.len() == 1 => {
            index(rest).map(|from| ReturnElements::ListOfArgs { from })
        }
        Some((head, rest)) if head == "DictOfPairs" && rest.len() == 1 => {
            index(rest).map(|from| ReturnElements::DictOfPairs { from })
        }
        Some((head, rest)) if head == "ElementOf" && rest.len() == 1 => {
            index(rest).map(|container_arg| ReturnElements::ElementOf { container_arg })
        }
        Some((head, rest)) if head == "SubListOf" && rest.len() == 1 => {
            index(rest).map(|container_arg| ReturnElements::SubListOf { container_arg })
        }
        _ => None,
    }
    .or_else(|| {
        log.say(line, format!("unreadable return_elements `{text}` dropped"));
        None
    })
}

/// `{AppendsListElements N}` / `SetsDictValue` /
/// `{ExtendsDictValuesByName N}` / `ListifiesDictValue`.
pub(in super::super) fn parse_var_elements_effect(
    text: &str,
    line: u32,
    log: &mut Log,
) -> Option<VarElementsEffect> {
    let parts = list_words(text);
    match parts.split_first() {
        Some((head, rest)) if head == "SetsArrayElementsFromList" && rest.len() == 1 => rest[0]
            .parse()
            .ok()
            .map(|values_at| VarElementsEffect::SetsArrayElementsFromList { values_at }),
        Some((head, rest)) if head == "AppendsListElements" && rest.len() == 1 => rest[0]
            .parse()
            .ok()
            .map(|values_from| VarElementsEffect::AppendsListElements { values_from }),
        Some((head, rest)) if head == "SetsDictValue" && rest.is_empty() => {
            Some(VarElementsEffect::SetsDictValue)
        }
        Some((head, rest)) if head == "ExtendsDictValuesByName" && rest.len() == 1 => rest[0]
            .parse()
            .ok()
            .map(|values_from| VarElementsEffect::ExtendsDictValuesByName { values_from }),
        Some((head, rest)) if head == "ListifiesDictValue" && rest.is_empty() => {
            Some(VarElementsEffect::ListifiesDictValue)
        }
        _ => None,
    }
    .or_else(|| {
        log.say(
            line,
            format!("unreadable var_elements_effect `{text}` dropped"),
        );
        None
    })
}

/// Representation effect variant plus its authored argument-layout payload.
pub(in super::super) fn parse_representation_effect(
    text: &str,
    line: u32,
    log: &mut Log,
) -> Option<RepresentationEffect> {
    let parts = list_words(text);
    match parts.split_first() {
        Some((head, rest)) if head == "None" && rest.is_empty() => Some(RepresentationEffect::None),
        Some((head, rest)) if head == "CoerceExpressionValues" && rest.len() == 1 => rest[0]
            .parse()
            .ok()
            .map(|arguments_from| RepresentationEffect::CoerceExpressionValues { arguments_from }),
        Some((head, rest)) if head == "CoerceNumericValues" && rest.len() == 1 => rest[0]
            .parse()
            .ok()
            .map(|arguments_from| RepresentationEffect::CoerceNumericValues { arguments_from }),
        Some((head, rest)) if head == "CoerceOrdinaryList" && rest.len() == 1 => rest[0]
            .parse()
            .ok()
            .map(|operand| RepresentationEffect::CoerceOrdinaryList { operand }),
        Some((head, rest)) if head == "CoerceOrdinaryDictionary" && rest.len() == 1 => rest[0]
            .parse()
            .ok()
            .map(|operand| RepresentationEffect::CoerceOrdinaryDictionary { operand }),
        Some((head, rest)) if head == "CoerceOrdinaryListPairs" && rest.len() == 1 => rest[0]
            .parse()
            .ok()
            .map(|variables_from| RepresentationEffect::CoerceOrdinaryListPairs { variables_from }),
        Some((head, rest)) if head == "CoerceOrdinaryListIndices" && rest.len() == 2 => {
            match (rest[0].parse(), rest[1].parse()) {
                (Ok(operand), Ok(indices_from)) => {
                    Some(RepresentationEffect::CoerceOrdinaryListIndices {
                        operand,
                        indices_from,
                    })
                }
                _ => None,
            }
        }
        Some((head, rest)) if head == "CoerceOrdinaryListRange" && rest.len() == 3 => {
            match (rest[0].parse(), rest[1].parse(), rest[2].parse()) {
                (Ok(operand), Ok(first), Ok(last)) => {
                    Some(RepresentationEffect::CoerceOrdinaryListRange {
                        operand,
                        first,
                        last,
                    })
                }
                _ => None,
            }
        }
        Some((head, rest)) if head == "CopyOnWriteContainerMutation" && rest.len() == 2 => {
            match (rest[0].parse(), rest[1].parse()) {
                (Ok(variable_arg), Ok(minimum_arguments)) => {
                    Some(RepresentationEffect::CopyOnWriteContainerMutation {
                        variable_arg,
                        minimum_arguments,
                    })
                }
                _ => None,
            }
        }
        _ => None,
    }
    .or_else(|| {
        log.say(
            line,
            format!("unreadable representation_effect `{text}` dropped"),
        );
        None
    })
}

/// The five fieldless byte-array effects by name, plus the one that carries a
/// payload: `{Rebinarifies N}`.
///
/// Split out from the plain [`enum_by_name`] table because `Rebinarifies` is
/// not one word — it names the value operand whose byte-array representation
/// the operation installs in place, which is the whole content of the S110
/// documented fix.
pub(in super::super) fn parse_byte_array_effect(
    text: &str,
    line: u32,
    log: &mut Log,
) -> Option<ByteArrayEffect> {
    let parts = list_words(text);
    if let Some((head, rest)) = parts.split_first()
        && head == "Rebinarifies"
        && rest.len() == 1
    {
        let Ok(value_arg) = rest[0].parse() else {
            log.say(
                line,
                format!("unreadable byte_array_effect `{text}` dropped"),
            );
            return None;
        };
        return Some(ByteArrayEffect::Rebinarifies { value_arg });
    }
    enum_by_name(BYTE_ARRAY_EFFECTS, text, "byte array effect", line, log)
}

/// `deprecation_fix -replace WORD -description {…} -safety S`.
///
/// Only the declarative variants: `Custom { resolver }` names a registry
/// callback, which `README.md` marks reference-only, and the two positional
/// variants take the index they replace as `-replace-arg N`.
pub(in super::super) fn parse_deprecation_fix(
    stmt: &Stmt,
    log: &mut Log,
) -> Option<DeprecationFixHook> {
    deprecation_fix_from(&stmt.words, 1, stmt.line, log)
}

/// The flag reader both spellings share: the command / subcommand statement
/// (`deprecation_fix …`, whose own name word is skipped) and the option row's
/// `-deprecation-fix {…}` block, whose words are the block's own.
pub(in super::super) fn deprecation_fix_from(
    words: &[Word],
    start: usize,
    line: u32,
    log: &mut Log,
) -> Option<DeprecationFixHook> {
    let mut replacement: Option<&'static str> = None;
    let mut replace_arg: Option<u8> = None;
    let mut whole_invocation = false;
    let mut description: &'static str = "";
    let mut safety = DeprecationFixSafety::RequiresReview;

    let mut i = start;
    while i < words.len() {
        let rejected_flag_word = words[i].clone();
        let flag = words[i].text.clone();
        match flag.as_str() {
            "-replace" => {
                replacement = Some(leak_str(&next_text(words, &mut i)));
            }
            "-replace-arg" => {
                replace_arg = next_text(words, &mut i).parse().ok();
            }
            "-replace-invocation" => whole_invocation = true,
            "-description" => description = leak_str(&next_text(words, &mut i)),
            "-safety" => {
                const SAFETY: &[DeprecationFixSafety] = &[
                    DeprecationFixSafety::SemanticsEquivalent,
                    DeprecationFixSafety::RequiresReview,
                ];
                if let Some(chosen) = enum_by_name(
                    SAFETY,
                    &next_text(words, &mut i),
                    "deprecation fix safety",
                    line,
                    log,
                ) {
                    safety = chosen;
                }
            }
            _ => log.unknown_flag("deprecation_fix", line, &rejected_flag_word),
        }
        i += 1;
    }
    let Some(replacement) = replacement else {
        log.say(line, "`deprecation_fix` needs `-replace WORD`");
        return None;
    };
    Some(match (replace_arg, whole_invocation) {
        (Some(index), _) => DeprecationFixHook::ReplaceArgument {
            index,
            replacement,
            description,
            safety,
        },
        (None, true) => DeprecationFixHook::ReplaceInvocation {
            replacement,
            description,
            safety,
        },
        (None, false) => DeprecationFixHook::ReplaceMatchedWord {
            replacement,
            description,
            safety,
        },
    })
}

/// `setter_constraint N -prefix P -code CODE -message {…}`.
pub(in super::super) fn parse_setter_constraint(
    stmt: &Stmt,
    log: &mut Log,
) -> Option<SetterConstraint> {
    let Ok(arg_index) = stmt.word_text(1).parse::<u8>() else {
        log.say(
            stmt.line,
            format!(
                "`setter_constraint` needs an argument index, got `{}`",
                stmt.word_text(1)
            ),
        );
        return None;
    };
    let mut required_prefix: &'static str = "";
    let mut code: Option<DiagCode> = None;
    let mut message: &'static str = "";

    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        let rejected_flag_word = words[i].clone();
        let flag = words[i].text.clone();
        match flag.as_str() {
            "-prefix" => required_prefix = leak_str(&next_text(words, &mut i)),
            "-code" => {
                let text = next_text(words, &mut i);
                match text.parse() {
                    Ok(parsed) => code = Some(parsed),
                    Err(_) => log.say(stmt.line, format!("unknown diagnostic code `{text}`")),
                }
            }
            "-message" => message = leak_str(&next_text(words, &mut i)),
            _ => log.unknown_flag("setter_constraint", stmt.line, &rejected_flag_word),
        }
        i += 1;
    }
    let Some(code) = code else {
        log.say(stmt.line, "`setter_constraint` needs `-code CODE`");
        return None;
    };
    Some(SetterConstraint {
        arg_index,
        required_prefix,
        code,
        message,
    })
}

/// `byte_array_payload -replace-data-index N ?-message-flag-shift?`.
pub(in super::super) fn parse_byte_array_payload(
    stmt: &Stmt,
    log: &mut Log,
) -> Option<BytePayloadSpec> {
    let mut replace_data_index: Option<u8> = None;
    let mut message_flag_shift = false;

    let words = &stmt.words;
    let mut i = 1;
    while i < words.len() {
        let rejected_flag_word = words[i].clone();
        let flag = words[i].text.clone();
        match flag.as_str() {
            "-replace-data-index" => replace_data_index = next_text(words, &mut i).parse().ok(),
            "-message-flag-shift" => message_flag_shift = true,
            _ => log.unknown_flag("byte_array_payload", stmt.line, &rejected_flag_word),
        }
        i += 1;
    }
    let Some(replace_data_index) = replace_data_index else {
        log.say(
            stmt.line,
            "`byte_array_payload` needs `-replace-data-index N`",
        );
        return None;
    };
    Some(BytePayloadSpec {
        replace_data_index,
        message_flag_shift,
    })
}

/// `defines_symbol -name-arg N ?-detail-arg N? ?-requires-arg N? -kind KIND`.
pub(in super::super) fn parse_defines_symbol(stmt: &Stmt, log: &mut Log) -> Option<SymbolDef> {
    const KINDS: &[DefinedSymbolKind] = &[
        DefinedSymbolKind::Test,
        DefinedSymbolKind::Constraint,
        DefinedSymbolKind::Matcher,
        DefinedSymbolKind::Event,
    ];
    let mut name_arg: Option<u8> = None;
    let mut detail_arg: Option<u8> = None;
    let mut requires_arg: Option<u8> = None;
    let mut kind: Option<DefinedSymbolKind> = None;

    let words = &stmt.words;
    let mut i = 1;
    while i < words.len() {
        let rejected_flag_word = words[i].clone();
        let flag = words[i].text.clone();
        match flag.as_str() {
            "-name-arg" => name_arg = next_text(words, &mut i).parse().ok(),
            "-detail-arg" => detail_arg = next_text(words, &mut i).parse().ok(),
            "-requires-arg" => requires_arg = next_text(words, &mut i).parse().ok(),
            "-kind" => {
                kind = enum_by_name(
                    KINDS,
                    &next_text(words, &mut i),
                    "defined symbol kind",
                    stmt.line,
                    log,
                );
            }
            _ => log.unknown_flag("defines_symbol", stmt.line, &rejected_flag_word),
        }
        i += 1;
    }
    let (Some(name_arg), Some(kind)) = (name_arg, kind) else {
        log.say(
            stmt.line,
            "`defines_symbol` needs `-name-arg N` and `-kind KIND`",
        );
        return None;
    };
    Some(SymbolDef {
        name_arg,
        detail_arg,
        requires_arg,
        kind,
    })
}

/// Every semantic operation a pack can write, in the vocabulary's order:
/// `Invoke`, then each intrinsic, then each structured lowering — the closed
/// set [`parse_semantic_operation`] reads.
pub fn semantic_operations() -> impl Iterator<Item = SemanticOperationId> {
    std::iter::once(SemanticOperationId::Invoke)
        .chain(
            INTRINSICS
                .iter()
                .copied()
                .map(SemanticOperationId::Intrinsic),
        )
        .chain(
            LOWERING_HOOKS
                .iter()
                .copied()
                .map(SemanticOperationId::StructuredLowering),
        )
}

/// The `.tclspec` value of a `semantic_operation` row — `Invoke`,
/// `Intrinsic ID` or `StructuredLowering ID`, the list
/// [`parse_semantic_operation`] reads back.
#[must_use]
pub fn semantic_operation_spelling(operation: SemanticOperationId) -> String {
    match operation {
        SemanticOperationId::Invoke => "Invoke".to_owned(),
        SemanticOperationId::Intrinsic(intrinsic) => {
            format!("Intrinsic {}", catalogue::variant_name(&intrinsic))
        }
        SemanticOperationId::StructuredLowering(lowering) => {
            format!("StructuredLowering {}", catalogue::variant_name(&lowering))
        }
    }
}

/// `Invoke` / `{Intrinsic ID}` / `{StructuredLowering ID}`.
pub(in super::super) fn parse_semantic_operation(
    text: &str,
    line: u32,
    log: &mut Log,
) -> Option<SemanticOperationId> {
    let parts = list_words(text);
    match parts.split_first() {
        Some((head, rest)) if head == "Invoke" && rest.is_empty() => {
            Some(SemanticOperationId::Invoke)
        }
        Some((head, rest)) if head == "Intrinsic" && rest.len() == 1 => {
            by_name(INTRINSICS, &rest[0]).map(SemanticOperationId::Intrinsic)
        }
        Some((head, rest)) if head == "StructuredLowering" && rest.len() == 1 => {
            by_name(LOWERING_HOOKS, &rest[0]).map(SemanticOperationId::StructuredLowering)
        }
        _ => {
            log.say(
                line,
                format!("unreadable semantic_operation `{text}` dropped"),
            );
            None
        }
    }
}

/// `-binds-handle {-name-from {Word N} -class-from {Word N} ?-keyword {N W}?}`.
pub(in super::super) fn parse_handle_binding(
    text: &str,
    line: u32,
    log: &mut Log,
) -> Option<HandleBindingSpec> {
    let parts = list_words(text);
    let mut name_from = None;
    let mut class_from = None;
    let mut keyword = None;
    let mut i = 0;
    while i < parts.len() {
        let flag = parts[i].clone();
        let value = parts.get(i + 1).cloned().unwrap_or_default();
        match flag.as_str() {
            "-name-from" => {
                let inner = list_words(&value);
                name_from = match inner.split_first() {
                    Some((head, rest)) if head == "Word" && rest.len() == 1 => {
                        rest[0].parse().ok().map(HandleName::Word)
                    }
                    Some((head, rest)) if head == "Implicit" && rest.len() == 1 => {
                        Some(HandleName::Implicit(leak_str(&rest[0])))
                    }
                    _ => None,
                };
            }
            "-class-from" => {
                let inner = list_words(&value);
                class_from = match inner.split_first() {
                    Some((head, rest)) if head == "Word" && rest.len() == 1 => {
                        rest[0].parse().ok().map(HandleClassSource::Word)
                    }
                    Some((head, rest)) if head == "ConstructionValue" && rest.len() == 1 => rest[0]
                        .parse()
                        .ok()
                        .map(HandleClassSource::ConstructionValue),
                    _ => None,
                };
            }
            "-keyword" => {
                let inner = list_words(&value);
                if inner.len() == 2 {
                    keyword = inner[0].parse().ok().map(|at| HandleKeyword {
                        at,
                        word: leak_str(&inner[1]),
                    });
                }
            }
            other => log.unknown_flag_value("binds_handle", line, other),
        }
        i += 2;
    }
    let (Some(name_from), Some(class_from)) = (name_from, class_from) else {
        log.say(line, "binds_handle needs -name-from and -class-from");
        return None;
    };
    Some(HandleBindingSpec {
        name_from,
        class_from,
        keyword,
    })
}

/// `object_class NAME ?-superclass {…}? ?-allow-unknown?`
/// `?-method-prefix-matching Enabled|Strict? ?{ method … }?`.
///
/// The ratified spelling (`docs/design/spec-dsl-examples/README.md`, "Rulings
/// on the census drafts"): the class's scalar fields ride on the statement
/// rather than in the block.
/// The `NAME` word is [`ObjectClassSpec::class_name`] and is **not** always the
/// command name — a factory command may manufacture a differently-named class.
/// The optional trailing block holds `method NAME { … }` rows, which reuse the
/// `subcommand` body grammar unchanged because
/// [`ObjectClassSpec::instance_methods`] really is `&[SubCommand]`.
pub(in super::super) fn object_class_row(
    stmt: &Stmt,
    tables: &PackTables,
    log: &mut Log,
) -> Option<tcl_registry::spec::ObjectClassSpec> {
    const MATCHING: &[PrefixMatching] = &[PrefixMatching::Enabled, PrefixMatching::Strict];

    let class_name = stmt.word_text(1).to_owned();
    if class_name.is_empty() {
        log.say(stmt.line, "`object_class` with no class name dropped");
        return None;
    }
    let mut superclasses: Vec<String> = Vec::new();
    let mut allow_unknown_methods = false;
    let mut method_prefix_matching = PrefixMatching::Strict;
    let mut body: Option<&Word> = None;
    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        let rejected_flag_word = words[i].clone();
        match words[i].text.as_str() {
            "-superclass" => superclasses = list_words(&next_text(words, &mut i)),
            "-allow-unknown" => allow_unknown_methods = true,
            // The §6.2 `dynamic_surface` / `unknown_members` fact in its
            // object-class spelling: the instance-method surface is open.
            "-dynamic-surface" | "-unknown-members" => {
                log.v20(stmt.line, &format!("object_class {}", words[i].text));
                allow_unknown_methods = true;
            }
            "-method-prefix-matching" => {
                let value = next_text(words, &mut i);
                if let Some(mode) = enum_by_name(
                    MATCHING,
                    &value,
                    "object method prefix matching",
                    stmt.line,
                    log,
                ) {
                    method_prefix_matching = mode;
                    log.v12(stmt.line, "object_class -method-prefix-matching");
                }
            }
            // The one unflagged word left is the member block, and it is
            // always last.
            _ if words[i].braced && i + 1 == words.len() => body = Some(&words[i]),
            _ => log.unknown_flag("object_class", stmt.line, &rejected_flag_word),
        }
        i += 1;
    }

    let outer = log.context.clone();
    let methods = log.scoped(format!("{outer} / object_class {class_name}"), |log| {
        let mut methods: Vec<SubCommand> = Vec::new();
        let Some(body) = body else {
            return methods;
        };
        for stmt in block(body) {
            if stmt.word_text(0) != "method" {
                log.unknown_property(&stmt);
                continue;
            }
            // A member row's own hooks have no owner the pack-hook binder can
            // name today — `HookOwner::Subcommand` resolves against
            // `spec.subcommands`, which an instance method is not in — so the
            // declarations are collected here and reported rather than bound
            // to the wrong table.
            let mut hooks: Vec<HookDecl> = Vec::new();
            let Some(method) = load_subcommand(&stmt, tables, &mut hooks, "method", log) else {
                continue;
            };
            for hook in &hooks {
                if matches!(hook.source, HookSource::Body { .. }) {
                    log.say(
                        stmt.line,
                        format!(
                            "`{}` hook body on `method {}` is not yet bindable; the field \
                             abstains",
                            hook.field, method.name
                        ),
                    );
                }
            }
            methods.push(method);
        }
        methods
    });

    Some(tcl_registry::spec::ObjectClassSpec {
        class_name: leak_str(&class_name),
        instance_methods: leak_slice(methods),
        superclasses: leak_strs(&superclasses),
        allow_unknown_methods,
        method_prefix_matching,
    })
}
