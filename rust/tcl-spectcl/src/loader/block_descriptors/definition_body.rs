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

//! `definition_body` blocks: members with their options, effects and wrapper shifts, construction hints, built-in object methods and manufacturers.

use super::super::statement_reader::Stmt;
use super::super::value_parsers::{
    apply_availability, by_name, enum_by_name, list_words, next_text, parse_dialects,
    parse_handle_binding,
};
use super::super::{Log, available, leak_slice, leak_str, leak_strs};
use tcl_registry::arg_role::ArgRole;
use tcl_registry::definer::{
    BuiltinMethodReceiver, BuiltinObjectMethod, CallableRole, DeclaredMemberVisibility,
    DefinerFamily, DefinitionBodyGrammar, InitTiming, ManufacturerMethod, MemberBodyCommand,
    MemberEffect, MemberKind, MemberOptionValue, MemberReceiver, MemberRefKind, MemberRetraction,
    MemberSpec, MemberVisibility, OptionalMemberArgument, RelationSlot, SlotOp, SlotSpec,
    StateScope, WrapperShift,
};

/// `definition_body { … }` — the inline definer grammar.
pub(in super::super) fn definition_body_block(
    stmts: &[Stmt],
    log: &mut Log,
) -> DefinitionBodyGrammar {
    let mut grammar = DefinitionBodyGrammar {
        family: DefinerFamily::TclOo,
        members: &[],
        implicit_vars: &[],
        implicit_receiver_vars: &[],
        member_body_namespace_path: &[],
        builtin_type_methods: &[],
        builtin_object_methods: &[],
        builtin_terminating_methods: &[],
        member_body_commands: &[],
        bare_word_construction: false,
        bare_word_construction_hint: None,
        bare_word_construction_source_hint: None,
        dynamic_method_dispatch: false,
        manufacturers: &[],
        unknown_dispatch_method: None,
        property_accessor_methods: &[],
    };
    let mut members: Vec<MemberSpec> = Vec::new();
    let mut member_options: Vec<MemberOptionRow> = Vec::new();
    let mut object_methods: Vec<BuiltinObjectMethod> = Vec::new();
    let mut body_commands: Vec<MemberBodyCommand> = Vec::new();
    let mut manufacturers: Vec<ManufacturerMethod> = Vec::new();

    for stmt in stmts {
        let value = stmt.word_text(1).to_owned();
        match stmt.word_text(0) {
            "family" => {
                const FAMILIES: &[DefinerFamily] = &[
                    DefinerFamily::TclOo,
                    DefinerFamily::Snit,
                    DefinerFamily::Itcl,
                    DefinerFamily::Clay,
                    DefinerFamily::JimClass,
                    DefinerFamily::SpecTcl,
                    DefinerFamily::SslicTcl,
                ];
                if let Some(family) =
                    enum_by_name(FAMILIES, &value, "definer family", stmt.line, log)
                {
                    grammar.family = family;
                }
            }
            "member" => members.extend(member_row(stmt, log)),
            "member_option" => member_options.extend(member_option_row(stmt, log)),
            "implicit_vars" => grammar.implicit_vars = leak_strs(&list_words(&value)),
            "implicit_receiver_vars" => {
                grammar.implicit_receiver_vars = leak_strs(&list_words(&value));
            }
            "member_body_namespace_path" => {
                grammar.member_body_namespace_path = leak_strs(&list_words(&value));
            }
            "builtin_type_methods" => {
                grammar.builtin_type_methods = leak_strs(&list_words(&value));
            }
            "builtin_object_method" => object_methods.push(builtin_object_method_row(stmt, log)),
            "builtin_terminating_methods" => {
                grammar.builtin_terminating_methods = leak_strs(&list_words(&value));
            }
            "member_body_command" => body_commands.push(member_body_command_row(stmt, log)),
            "bare_word_construction" => {
                grammar.bare_word_construction = true;
                grammar.bare_word_construction_source_hint = construction_source_hint(stmt, log);
                // Preserve the historical callable projection only for the
                // actual exact Snit descriptor, not an arbitrary bare flag.
                grammar.bare_word_construction_hint = grammar
                    .bare_word_construction_source_hint
                    .filter(|hint| hint.values == ["%AUTO%"] && hint.prefixes == ["."])
                    .map(|_| snit_shaped_hint as fn(&str) -> bool);
            }
            "dynamic_method_dispatch" => grammar.dynamic_method_dispatch = true,
            "manufacturer" => manufacturers.push(manufacturer_row(stmt, log)),
            "unknown_dispatch_method" => {
                grammar.unknown_dispatch_method = Some(leak_str(&value));
            }
            "property_accessor_methods" => {
                grammar.property_accessor_methods = leak_strs(&list_words(&value));
            }
            _ => log.unknown_property(stmt),
        }
    }
    attach_member_options(&mut members, member_options, log);
    grammar.members = leak_slice(members);
    grammar.builtin_object_methods = leak_slice(object_methods);
    grammar.member_body_commands = leak_slice(body_commands);
    grammar.manufacturers = leak_slice(manufacturers);
    grammar
}

/// One `member_option KEYWORD POSITION VALUE -role ROLE ?-visibility V?
/// ?-dialects D? ?-available V?` row: one accepted spelling of a member's
/// optional word, keyed by the member and the fixed position it sits at.
struct MemberOptionRow {
    pub(in super::super) keyword: String,
    position: u8,
    pub(in super::super) value: MemberOptionValue,
    pub(in super::super) line: u32,
}

fn member_option_row(stmt: &Stmt, log: &mut Log) -> Option<MemberOptionRow> {
    let Ok(position) = stmt.word_text(2).parse::<u8>() else {
        log.say(
            stmt.line,
            format!(
                "`member_option {}` needs a fixed position; row dropped",
                stmt.word_text(1)
            ),
        );
        return None;
    };
    let mut value = MemberOptionValue {
        value: leak_str(stmt.word_text(3)),
        role: ArgRole::Option,
        surface: None,
        declared_visibility: None,
    };
    let words = &stmt.words;
    let mut i = 4;
    while i < words.len() {
        let rejected_flag_word = words[i].clone();
        match words[i].text.as_str() {
            "-role" => {
                let name = next_text(words, &mut i);
                if let Some(role) = enum_by_name(ArgRole::ALL, &name, "role", stmt.line, log) {
                    value.role = role;
                }
            }
            "-visibility" => {
                let name = next_text(words, &mut i);
                value.declared_visibility = enum_by_name(
                    DeclaredMemberVisibility::ALL,
                    &name,
                    "declared visibility",
                    stmt.line,
                    log,
                );
            }
            "-dialects" => {
                let text = next_text(words, &mut i);
                value.surface = parse_dialects(&text, stmt.line, log);
            }
            "-available" => {
                log.v20(stmt.line, "-available");
                let text = next_text(words, &mut i);
                let availability = available::from_flag(&text, stmt.line, log);
                apply_availability(
                    &mut value.surface,
                    availability,
                    "member option",
                    stmt.line,
                    log,
                );
            }
            _ => log.unknown_flag("member_option", stmt.line, &rejected_flag_word),
        }
        i += 1;
    }
    Some(MemberOptionRow {
        keyword: stmt.word_text(1).to_owned(),
        position,
        value,
        line: stmt.line,
    })
}

/// Hang each `member_option` row off the member it names, in row order: one
/// member has one optional word, so every row for it shares one position.
fn attach_member_options(members: &mut [MemberSpec], rows: Vec<MemberOptionRow>, log: &mut Log) {
    let mut grouped: Vec<(usize, u8, Vec<MemberOptionValue>)> = Vec::new();
    for row in rows {
        let Some(index) = members
            .iter()
            .position(|member| member.keyword == row.keyword)
        else {
            log.say(
                row.line,
                format!(
                    "`member_option` names no member `{}`; row dropped",
                    row.keyword
                ),
            );
            continue;
        };
        match grouped.iter_mut().find(|(member, _, _)| *member == index) {
            Some((_, position, values)) if *position == row.position => values.push(row.value),
            Some(_) => log.say(
                row.line,
                format!(
                    "member `{}` already has its optional word at another position; row dropped",
                    row.keyword
                ),
            ),
            None => grouped.push((index, row.position, vec![row.value])),
        }
    }
    for (index, position, values) in grouped {
        members[index].optional_argument = Some(OptionalMemberArgument {
            position,
            values: leak_slice(values),
        });
    }
}

/// Parse the actual finite hint options of this selected source declaration.
fn construction_source_hint(
    stmt: &Stmt,
    log: &mut Log,
) -> Option<tcl_registry::definer::BareWordConstructionSourceHint> {
    let mut values = None;
    let mut prefixes = None;
    let mut args = stmt.tail().iter();
    while let Some(option) = args.next() {
        let Some(value) = args.next() else {
            log.say(
                stmt.line,
                "bare_word_construction: hint option requires its list value",
            );
            return None;
        };
        let target = match option.text.as_str() {
            "-hint-values" if values.is_none() => &mut values,
            "-hint-prefixes" if prefixes.is_none() => &mut prefixes,
            _ => {
                log.say(
                    stmt.line,
                    "bare_word_construction: unknown or repeated hint option",
                );
                return None;
            }
        };
        let Ok(items) = tcl_syntax::list::split_list(&value.text) else {
            log.say(
                stmt.line,
                "bare_word_construction: hint requires a complete literal list",
            );
            return None;
        };
        *target = Some(
            items
                .into_iter()
                .map(std::borrow::Cow::into_owned)
                .collect::<Vec<_>>(),
        );
    }
    if values.is_none() && prefixes.is_none() {
        return None;
    }
    let values = values.unwrap_or_default();
    let prefixes = prefixes.unwrap_or_default();
    if prefixes.iter().any(String::is_empty) {
        log.say(
            stmt.line,
            "bare_word_construction: an empty source prefix is unavailable",
        );
        return None;
    }
    Some(tcl_registry::definer::BareWordConstructionSourceHint {
        values: leak_strs(&values),
        prefixes: leak_strs(&prefixes),
    })
}

/// Compatibility callable for the exact authored Snit descriptor only.
fn snit_shaped_hint(word: &str) -> bool {
    tcl_registry::definer::BareWordConstructionSourceHint {
        values: &["%AUTO%"],
        prefixes: &["."],
    }
    .recognises(word)
}

/// The `(index, role)` pairs of a member row's `-roles` list.
///
/// Split out of [`member_row`] because the pairing rule — flat list, two
/// words per pair, an unreadable pair dropped with a notice rather than
/// shifting every pair after it — is a self-contained reading of one word.
fn member_arg_roles(text: &str, line: u32, log: &mut Log) -> Vec<(u8, ArgRole)> {
    let mut roles = Vec::new();
    for pair in list_words(text).chunks(2) {
        let [index, role] = pair else { continue };
        match (index.parse::<u8>(), by_name(ArgRole::ALL, role)) {
            (Ok(index), Some(role)) => roles.push((index, role)),
            _ => log.say(
                line,
                format!("unreadable member role pair `{index} {role}` dropped"),
            ),
        }
    }
    roles
}

/// A member row's `-effect` value before the row's `-roles` are all read. A
/// `callable`, `forward` or `init-script` slot the value leaves unwritten is
/// positioned by the first `-roles` index carrying the role it names —
/// `Name`, `ParamList`, `Body`, and `CommandName` / `CommandPrefix` for a
/// forward's target — so the page's `-effect {callable -receiver instance
/// -role method}` reads a method's three slots off its roles.
#[derive(Clone, Copy)]
enum EffectDraft {
    Callable {
        receiver: MemberReceiver,
        role: CallableRole,
        name: Option<u8>,
        params: Option<u8>,
        body: Option<u8>,
    },
    Forward {
        name: Option<u8>,
        prefix: Option<u8>,
    },
    InitScript {
        body: Option<u8>,
        timing: InitTiming,
    },
    Ready(MemberEffect),
}

impl EffectDraft {
    /// The effect, its unwritten slots positioned by `arg_roles`; `None` when
    /// a `forward` or `init-script` slot is neither written nor derivable.
    pub(in super::super) fn resolve(self, arg_roles: &[(u8, ArgRole)]) -> Option<MemberEffect> {
        let first = |role: ArgRole| {
            arg_roles
                .iter()
                .find(|(_, declared)| *declared == role)
                .map(|(index, _)| *index)
        };
        match self {
            Self::Callable {
                receiver,
                role,
                name,
                params,
                body,
            } => Some(MemberEffect::Callable {
                receiver,
                role,
                name_slot: name.or_else(|| first(ArgRole::Name)),
                params_slot: params.or_else(|| first(ArgRole::ParamList)),
                body_slot: body.or_else(|| first(ArgRole::Body)),
            }),
            Self::Forward { name, prefix } => Some(MemberEffect::Forward {
                name_slot: name.or_else(|| first(ArgRole::Name))?,
                prefix_slot: prefix
                    .or_else(|| first(ArgRole::CommandName))
                    .or_else(|| first(ArgRole::CommandPrefix))?,
            }),
            Self::InitScript { body, timing } => Some(MemberEffect::InitScript {
                body_slot: body.or_else(|| first(ArgRole::Body))?,
                timing,
            }),
            Self::Ready(effect) => Some(effect),
        }
    }
}

/// The `-FLAG VALUE` pairs of an `-effect` / `-shift` value, or `None` when a
/// flag is not in `known` or has no value.
fn effect_flags<'t>(rest: &'t [String], known: &[&str]) -> Option<Vec<(&'t str, &'t str)>> {
    rest.chunks(2)
        .map(|pair| match pair {
            [flag, value] if known.contains(&flag.as_str()) => {
                Some((flag.as_str(), value.as_str()))
            }
            _ => None,
        })
        .collect()
}

/// `-effect VALUE` on a `member` row: `{callable -receiver R -role K ?-name N?
/// ?-params N? ?-body N?}`, `{forward ?-name N? ?-prefix N?}`,
/// `{state-declaration SCOPE}`, `{relation SLOT}`, `visibility`,
/// `retraction`, `{init-script ?-body N? -timing T}` or `configuration` —
/// the registry's own spellings. `None` (with a notice) for anything else.
fn member_effect(text: &str, line: u32, log: &mut Log) -> Option<EffectDraft> {
    let words = list_words(text);
    let draft = words.split_first().and_then(|(kind, rest)| {
        let slot = |value: &str| value.parse::<u8>().ok();
        match (kind.as_str(), rest) {
            ("callable", _) => {
                let flags =
                    effect_flags(rest, &["-receiver", "-role", "-name", "-params", "-body"])?;
                let mut draft = (None, None, None, None, None);
                for (flag, value) in flags {
                    match flag {
                        "-receiver" => draft.0 = Some(MemberReceiver::from_spelling(value)?),
                        "-role" => draft.1 = Some(CallableRole::from_spelling(value)?),
                        "-name" => draft.2 = Some(slot(value)?),
                        "-params" => draft.3 = Some(slot(value)?),
                        _ => draft.4 = Some(slot(value)?),
                    }
                }
                Some(EffectDraft::Callable {
                    receiver: draft.0?,
                    role: draft.1?,
                    name: draft.2,
                    params: draft.3,
                    body: draft.4,
                })
            }
            ("forward", _) => {
                let (mut name, mut prefix) = (None, None);
                for (flag, value) in effect_flags(rest, &["-name", "-prefix"])? {
                    if flag == "-name" {
                        name = Some(slot(value)?);
                    } else {
                        prefix = Some(slot(value)?);
                    }
                }
                Some(EffectDraft::Forward { name, prefix })
            }
            ("init-script", _) => {
                let (mut body, mut timing) = (None, None);
                for (flag, value) in effect_flags(rest, &["-body", "-timing"])? {
                    if flag == "-body" {
                        body = Some(slot(value)?);
                    } else {
                        timing = Some(InitTiming::from_spelling(value)?);
                    }
                }
                Some(EffectDraft::InitScript {
                    body,
                    timing: timing?,
                })
            }
            ("state-declaration", [scope]) => StateScope::from_spelling(scope)
                .map(|scope| EffectDraft::Ready(MemberEffect::StateDeclaration { scope })),
            ("relation", [slot]) => RelationSlot::from_spelling(slot)
                .map(|slot| EffectDraft::Ready(MemberEffect::Relation { slot })),
            ("visibility", []) => Some(EffectDraft::Ready(MemberEffect::Visibility)),
            ("retraction", []) => Some(EffectDraft::Ready(MemberEffect::Retraction)),
            ("configuration", []) => Some(EffectDraft::Ready(MemberEffect::Configuration)),
            _ => None,
        }
    });
    if draft.is_none() {
        log.say(
            line,
            format!("unreadable member effect `{text}`; row dropped"),
        );
    }
    draft
}

/// `-shift {?-receiver R? ?-visibility V?}` on a wrapper `member` row: the
/// side and visibility the wrapped member takes.
fn wrapper_shift(text: &str, line: u32, log: &mut Log) -> Option<WrapperShift> {
    let words = list_words(text);
    let shift = effect_flags(&words, &["-receiver", "-visibility"]).and_then(|flags| {
        let mut shift = WrapperShift::NONE;
        for (flag, value) in flags {
            if flag == "-receiver" {
                shift.receiver = Some(MemberReceiver::from_spelling(value)?);
            } else {
                shift.visibility = Some(DeclaredMemberVisibility::from_spelling(value)?);
            }
        }
        Some(shift)
    });
    if shift.is_none() {
        log.say(line, format!("unreadable wrapper shift `{text}` dropped"));
    }
    shift
}

/// The closed vocabularies of a `member` row's enum-valued flags.
const MEMBER_REF_KINDS: &[MemberRefKind] = &[MemberRefKind::Class, MemberRefKind::Method];

const MEMBER_KINDS: &[MemberKind] = &[MemberKind::Flat, MemberKind::Wrapper, MemberKind::FlagKeyed];

const MEMBER_RETRACTIONS: &[MemberRetraction] = &[
    MemberRetraction::EveryArgument,
    MemberRetraction::FirstArgument,
];

const SLOT_OPS: &[SlotOp] = &[
    SlotOp::Set,
    SlotOp::Append,
    SlotOp::AppendIfNew,
    SlotOp::Prepend,
    SlotOp::Remove,
    SlotOp::Clear,
];

const MEMBER_VISIBILITIES: &[MemberVisibility] =
    &[MemberVisibility::Exported, MemberVisibility::Unexported];

fn member_row(stmt: &Stmt, log: &mut Log) -> Option<MemberSpec> {
    let mut member = MemberSpec {
        keyword: leak_str(stmt.word_text(1)),
        arg_roles: &[],
        optional_argument: None,
        all_args_var: false,
        all_args_ref: None,
        kind: MemberKind::Flat,
        wrapper_block_body: false,
        surface: None,
        retraction: None,
        slot: None,
        visibility_effect: None,
        effect: MemberEffect::Configuration,
        wrapper_shift: None,
    };
    let mut slot_op: Option<SlotOp> = None;
    let mut dedup = false;
    let mut effect = EffectFlag::Missing;
    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        let rejected_flag_word = words[i].clone();
        match words[i].text.as_str() {
            "-effect" => {
                let text = next_text(words, &mut i);
                effect = member_effect(&text, stmt.line, log)
                    .map_or(EffectFlag::Unreadable, EffectFlag::Read);
            }
            "-shift" => {
                let text = next_text(words, &mut i);
                member.wrapper_shift = wrapper_shift(&text, stmt.line, log);
            }
            "-roles" => {
                let text = next_text(words, &mut i);
                member.arg_roles = leak_slice(member_arg_roles(&text, stmt.line, log));
            }
            "-all-vars" => member.all_args_var = true,
            flag @ ("-all-refs" | "-kind" | "-retracts" | "-visibility") => {
                let name = next_text(words, &mut i);
                member_named_flag(&mut member, flag, &name, stmt.line, log);
            }
            "-block-body" => member.wrapper_block_body = true,
            "-dialects" => {
                let text = next_text(words, &mut i);
                member.surface = parse_dialects(&text, stmt.line, log);
            }
            "-available" => {
                log.v20(stmt.line, "-available");
                let text = next_text(words, &mut i);
                let availability = available::from_flag(&text, stmt.line, log);
                apply_availability(
                    &mut member.surface,
                    availability,
                    "object-class method",
                    stmt.line,
                    log,
                );
            }
            "-slot" => {
                let name = next_text(words, &mut i);
                slot_op = enum_by_name(SLOT_OPS, &name, "slot operation", stmt.line, log);
            }
            "-dedup" => dedup = true,
            _ => log.unknown_flag("member", stmt.line, &rejected_flag_word),
        }
        i += 1;
    }
    if let Some(default_op) = slot_op {
        member.slot = Some(SlotSpec { default_op, dedup });
    }
    finish_member_row(member, &effect, stmt.line, log)
}

/// A member row's flag whose value is one name from a closed list.
fn member_named_flag(member: &mut MemberSpec, flag: &str, name: &str, line: u32, log: &mut Log) {
    match flag {
        "-all-refs" => {
            member.all_args_ref =
                enum_by_name(MEMBER_REF_KINDS, name, "member reference kind", line, log);
        }
        "-kind" => {
            if let Some(kind) = enum_by_name(MEMBER_KINDS, name, "member kind", line, log) {
                member.kind = kind;
            }
        }
        "-retracts" => {
            member.retraction =
                enum_by_name(MEMBER_RETRACTIONS, name, "member retraction", line, log);
        }
        _ => {
            member.visibility_effect =
                enum_by_name(MEMBER_VISIBILITIES, name, "member visibility", line, log);
        }
    }
}

/// A member row's `-effect` as the row's flags left it.
enum EffectFlag {
    /// Not written.
    Missing,
    /// Written but unreadable, and already reported.
    Unreadable,
    /// Written and read, its slots not yet positioned.
    Read(EffectDraft),
}

/// The checks a `member` row needs once every flag is read: a `-shift` only
/// on a wrapper, and an `-effect` — required, since every member states what
/// it declares — whose unwritten slots its `-roles` position.
fn finish_member_row(
    mut member: MemberSpec,
    effect: &EffectFlag,
    line: u32,
    log: &mut Log,
) -> Option<MemberSpec> {
    if member.wrapper_shift.is_some() && member.kind != MemberKind::Wrapper {
        log.say(
            line,
            format!(
                "`-shift` on member `{}`, which is not a wrapper, ignored",
                member.keyword
            ),
        );
        member.wrapper_shift = None;
    }
    let draft = match effect {
        EffectFlag::Read(draft) => *draft,
        EffectFlag::Missing => {
            log.say(
                line,
                format!("member `{}` has no `-effect`; row dropped", member.keyword),
            );
            return None;
        }
        EffectFlag::Unreadable => return None,
    };
    let Some(resolved) = draft.resolve(member.arg_roles) else {
        log.say(
            line,
            format!(
                "member `{}`'s `-effect` names a slot its `-roles` do not position; row dropped",
                member.keyword
            ),
        );
        return None;
    };
    member.effect = resolved;
    Some(member)
}

fn builtin_object_method_row(stmt: &Stmt, log: &mut Log) -> BuiltinObjectMethod {
    let mut method = BuiltinObjectMethod {
        name: leak_str(stmt.word_text(1)),
        visibility: MemberVisibility::Exported,
        receiver: BuiltinMethodReceiver::AnyObject,
        operation: None,
        detail: "",
    };
    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        let rejected_flag_word = words[i].clone();
        match words[i].text.as_str() {
            "-unexported" => method.visibility = MemberVisibility::Unexported,
            "-receiver" => {
                const RECEIVERS: &[BuiltinMethodReceiver] = &[
                    BuiltinMethodReceiver::AnyObject,
                    BuiltinMethodReceiver::ClassObject,
                ];
                let name = next_text(words, &mut i);
                if let Some(receiver) =
                    enum_by_name(RECEIVERS, &name, "builtin method receiver", stmt.line, log)
                {
                    method.receiver = receiver;
                }
            }
            "-detail" => method.detail = leak_str(&next_text(words, &mut i)),
            _ => log.unknown_flag("builtin_object_method", stmt.line, &rejected_flag_word),
        }
        i += 1;
    }
    method
}

fn member_body_command_row(stmt: &Stmt, log: &mut Log) -> MemberBodyCommand {
    let mut command = MemberBodyCommand {
        name: leak_str(stmt.word_text(1)),
        detail: "",
        binds_handle: None,
    };
    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        let rejected_flag_word = words[i].clone();
        match words[i].text.as_str() {
            "-detail" => command.detail = leak_str(&next_text(words, &mut i)),
            "-binds-handle" => {
                let text = next_text(words, &mut i);
                command.binds_handle = parse_handle_binding(&text, stmt.line, log);
            }
            _ => log.unknown_flag("member_body_command", stmt.line, &rejected_flag_word),
        }
        i += 1;
    }
    command
}

pub(in super::super) fn manufacturer_row(stmt: &Stmt, log: &mut Log) -> ManufacturerMethod {
    let mut method = ManufacturerMethod {
        keyword: leak_str(stmt.word_text(1)),
        visibility: MemberVisibility::Exported,
        names_instance_at: None,
        definition_body_at: None,
        constructor_args_from: 0,
    };
    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        let rejected_flag_word = words[i].clone();
        match words[i].text.as_str() {
            "-unexported" => method.visibility = MemberVisibility::Unexported,
            "-names-instance-at" => {
                method.names_instance_at = next_text(words, &mut i).parse().ok();
            }
            "-definition-body-at" => {
                method.definition_body_at = next_text(words, &mut i).parse().ok();
            }
            "-constructor-args-from" => {
                method.constructor_args_from = next_text(words, &mut i).parse().unwrap_or(0);
            }
            _ => log.unknown_flag("manufacturer", stmt.line, &rejected_flag_word),
        }
        i += 1;
    }
    method
}
