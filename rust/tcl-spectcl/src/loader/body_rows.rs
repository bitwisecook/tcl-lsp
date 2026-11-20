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

//! The rows a command, subcommand and refinement body share: `arg`, `option` with its effects, and `hover` with its examples.

use super::statement_reader::{Stmt, Word, block};
use super::value_parsers::{
    BODY_KINDS, PRESENTATIONS, SCRIPT_TIMINGS, TCL_TYPES, VARIABLE_SCOPES, apply_availability,
    by_name, checked_lifecycle, deprecation_fix_from, enum_by_name, lifecycle_flag, list_words,
    next_text, parse_appended_arity, parse_dialects, parse_integer_domain,
};
use super::{
    ArgRow, ArgRows, ArgSlices, HookSource, Log, PackTables, available, consume_one_word,
    leak_slice, leak_str, leak_strs,
};
use tcl_registry::arg_role::ArgRole;
use tcl_registry::hooks::ArgTypeHint;
use tcl_registry::hover::{
    ArgValue, CallbackTaintInput, HoverSnippet, OptionArg, OptionArity, OptionSpec, OptionValue,
    ScriptTiming, VariableScope,
};
use tcl_registry::option_effect::{
    EffectAxis, FamilyBase, FamilyCombine, OptionEffect, OptionEffectFamily, OptionEffectKind,
};
use tcl_registry::pack_hooks::HookInputs;
use tcl_registry::types::TclType;

// Per-argument rows — six schema keys, one statement

/// The `u8` argument tables cap the index; an index above 255 is dropped with
/// a notice rather than wrapping.
const MAX_ARG_INDEX: usize = 255;

impl ArgRows {
    /// Project the authored rows into the parallel slices the registry stores.
    ///
    /// The **one** place a per-argument row becomes the parallel form the rest
    /// of the registry reads: a column added to [`ArgRow`] and not added here
    /// silently vanishes, which `projection_carries_every_row_column` pins.
    pub(super) fn seal(self) -> ArgSlices {
        let mut out = ArgSlices::default();
        for row in self.rows {
            if let Some(role) = row.role {
                out.roles.push((row.index, role));
            }
            if let Some(hint) = row.type_hint {
                out.types.push((row.index, hint));
            }
            if !row.values.is_empty() {
                out.values.push((row.index, row.values));
            }
            if row.closed {
                out.closed.push(row.index);
            }
            if let Some(presentation) = row.presentation {
                out.presentation.push((row.index, presentation));
            }
            if let Some(appends) = row.appends {
                out.prefixes.push((row.index, appends));
            }
            if let Some(domain) = row.domain {
                out.domains.push((row.index, domain));
            }
        }
        out
    }

    // One flag per `arg` row column, and there are twelve of them; splitting
    // the match would put each column further from the others it constrains.
    #[allow(clippy::too_many_lines)]
    pub(super) fn apply(&mut self, stmt: &Stmt, tables: &PackTables, log: &mut Log) {
        let raw = stmt.word_text(1);
        let Ok(index) = raw.parse::<usize>() else {
            log.say(
                stmt.line,
                format!("unreadable argument index `{raw}` dropped"),
            );
            return;
        };
        if index > MAX_ARG_INDEX {
            log.say(
                stmt.line,
                format!(
                    "argument index {index} is above the {MAX_ARG_INDEX} the tables hold; dropped"
                ),
            );
            return;
        }
        let index = u8::try_from(index).unwrap_or(u8::MAX);

        let mut row = ArgRow {
            index,
            ..ArgRow::default()
        };
        let mut hint = ArgTypeHint {
            expected: None,
            shimmers: false,
            transparent_from: &[],
        };
        let mut saw_type_flag = false;
        let words = &stmt.words;
        let mut i = 2;
        while i < words.len() {
            let rejected_flag_word = words[i].clone();
            let flag = words[i].text.clone();
            match flag.as_str() {
                "-role" => {
                    let name = next_text(words, &mut i);
                    if let Some(role) =
                        enum_by_name(ArgRole::ALL, &name, "argument role", stmt.line, log)
                    {
                        // First declaration wins, matching `arg_role_at`'s
                        // `find` over the parallel slice: a row that named a
                        // role twice always resolved to the first one, and a
                        // `-appends`-implied `CommandPrefix` counts as one.
                        row.role.get_or_insert(role);
                    }
                }
                "-type" => {
                    let name = next_text(words, &mut i);
                    hint.expected = enum_by_name(TCL_TYPES, &name, "argument type", stmt.line, log);
                    saw_type_flag = true;
                }
                "-shimmers" => {
                    hint.shimmers = true;
                    saw_type_flag = true;
                }
                "-transparent" => {
                    let text = next_text(words, &mut i);
                    let types: Vec<TclType> = list_words(&text)
                        .iter()
                        .filter_map(|name| {
                            enum_by_name(TCL_TYPES, name, "argument type", stmt.line, log)
                        })
                        .collect();
                    hint.transparent_from = leak_slice(types);
                    saw_type_flag = true;
                }
                "-values" => {
                    let text = next_text(words, &mut i);
                    let values: Vec<ArgValue> = list_words(&text)
                        .iter()
                        .map(|value| ArgValue {
                            value: leak_str(value),
                            ..ArgValue::DEFAULT
                        })
                        .collect();
                    row.values = leak_slice(values);
                }
                "-values-from" => {
                    let name = next_text(words, &mut i);
                    match tables.values.get(&name) {
                        Some(values) => row.values = leak_slice(values.clone()),
                        None => log.say(
                            stmt.line,
                            format!("no `values {name}` table in this pack; row dropped"),
                        ),
                    }
                }
                "-closed" => row.closed = true,
                "-layout" => {
                    let name = next_text(words, &mut i);
                    if let Some(layout) =
                        enum_by_name(PRESENTATIONS, &name, "argument layout", stmt.line, log)
                    {
                        row.presentation = Some(layout);
                    }
                }
                "-appends" => {
                    let text = next_text(words, &mut i);
                    if let Some(appended) = parse_appended_arity(&text, stmt.line, log) {
                        row.appends = Some(appended);
                        // `-appends` implies the position is a command prefix,
                        // unless this index already has a role — from this row
                        // or an earlier one, since the parallel slice this
                        // projects into is index-keyed across rows.
                        if row.role.is_none()
                            && !self
                                .rows
                                .iter()
                                .any(|other| other.index == index && other.role.is_some())
                        {
                            row.role = Some(ArgRole::CommandPrefix);
                        }
                    }
                }
                "-domain" => {
                    log.since(stmt.line, "-domain", "2.3");
                    let text = next_text(words, &mut i);
                    row.domain = parse_integer_domain(&text, stmt.line, log);
                }
                _ => log.unknown_flag("arg", stmt.line, &rejected_flag_word),
            }
            i += 1;
        }
        if saw_type_flag {
            row.type_hint = Some(hint);
        }
        self.rows.push(row);
    }
}

// Option rows

const fn is_variable_role(role: ArgRole) -> bool {
    matches!(role, ArgRole::VarRead | ArgRole::VarWrite)
}

/// Parse the finite Tk callback-marker vocabulary. A pack cannot invent a
/// marker: accepting arbitrary `%x` text here would silently turn framework
/// metadata into a security source.
fn parse_callback_taint_inputs(
    text: &str,
    line: u32,
    log: &mut Log,
) -> &'static [CallbackTaintInput] {
    let mut out = Vec::new();
    for marker in list_words(text) {
        let input = match marker.as_str() {
            "%P" => CallbackTaintInput::TK_PROPOSED_VALUE,
            "%s" => CallbackTaintInput::TK_CURRENT_VALUE,
            "%S" => CallbackTaintInput::TK_EDIT_TEXT,
            "%A" => CallbackTaintInput::TK_EVENT_CHAR,
            "%K" => CallbackTaintInput::TK_EVENT_KEYSYM,
            _ => {
                log.say(
                    line,
                    format!(
                        "`{marker}` is not a user-controlled Tk callback substitution; \
                         framework metadata must not be declared tainted"
                    ),
                );
                continue;
            }
        };
        if !out.contains(&input) {
            out.push(input);
        }
    }
    leak_slice(out)
}

/// Parse `callback_taint_inputs {{ARG {%P %S}} …}` for positional callback
/// bodies. Each row names the 0-based callback argument and its finite set of
/// externally controlled substitutions.
pub(super) fn parse_callback_taint_input_table(
    text: &str,
    line: u32,
    log: &mut Log,
) -> Vec<(u8, &'static [CallbackTaintInput])> {
    let mut out = Vec::new();
    for row in list_words(text) {
        let fields = list_words(&row);
        let Some(index) = fields.first() else {
            continue;
        };
        if fields.len() != 2 {
            log.say(
                line,
                "`callback_taint_inputs` rows need `{argument-index {%P ...}}`",
            );
            continue;
        }
        let Ok(index) = index.parse::<u8>() else {
            log.say(
                line,
                "`callback_taint_inputs` needs an integer argument index",
            );
            continue;
        };
        let inputs = parse_callback_taint_inputs(&fields[1], line, log);
        if inputs.is_empty() {
            continue;
        }
        out.push((index, inputs));
    }
    out
}

/// Reject positional taint metadata that can never describe a deferred
/// executable argument. Dynamic role/timing resolvers count as an exact fact:
/// the registry rechecks their concrete answer for each invocation before it
/// exposes the inputs, so a resolver's abstaining shapes remain harmless.
pub(super) fn validated_callback_taint_input_table(
    entries: Vec<(u8, &'static [CallbackTaintInput])>,
    roles: &[(u8, ArgRole)],
    has_role_resolver: bool,
    can_defer: bool,
    owner: &str,
    line: u32,
    log: &mut Log,
) -> Vec<(u8, &'static [CallbackTaintInput])> {
    entries
        .into_iter()
        .filter(|(index, _)| {
            let executable = roles
                .iter()
                .any(|(candidate, role)| candidate == index && role.has_script_timing())
                || has_role_resolver;
            if !executable || !can_defer {
                log.say(
                    line,
                    format!(
                        "{owner} declares callback taint inputs for argument {index}, which is not a deferred executable position; dropped"
                    ),
                );
                false
            } else {
                true
            }
        })
        .collect()
}

/// `AXIS` or `AXIS VALUE` — the operand(s) of a `disables` / `selects` /
/// `only` word, read off an already-listed value.
fn effect_axis_words(words: &[String], line: u32, log: &mut Log) -> Option<EffectAxis> {
    let axis = match words {
        [axis] => EffectAxis::from_words(axis, None),
        [axis, value] => EffectAxis::from_words(axis, Some(value)),
        _ => None,
    };
    if axis.is_none() {
        log.say(
            line,
            format!("unknown option-effect axis `{}`", words.join(" ")),
        );
    }
    axis
}

/// `-effect VALUE` on an `option` row: `{disables AXIS VALUE}`,
/// `{selects AXIS VALUE}`, `{suppresses-role ROLE}`,
/// `{reserves-trailing-words N}`, or the bare `ends-options` — the
/// option-effect descriptor's own spellings
/// (`docs/design/compiler/registry-consumer-contracts.md` § *Options with
/// semantic effects*). `None` (with a notice) for anything else.
fn option_effect_kind(text: &str, line: u32, log: &mut Log) -> Option<OptionEffectKind> {
    let words = list_words(text);
    let kind = words
        .split_first()
        .and_then(|(kind, rest)| match (kind.as_str(), rest) {
            ("disables", axis) => {
                effect_axis_words(axis, line, log).map(OptionEffectKind::Disables)
            }
            ("selects", axis) => effect_axis_words(axis, line, log).map(OptionEffectKind::Selects),
            ("suppresses-role", [role]) => {
                by_name(ArgRole::ALL, role).map(OptionEffectKind::SuppressesRole)
            }
            ("reserves-trailing-words", [n]) => {
                n.parse().ok().map(OptionEffectKind::ReservesTrailingWords)
            }
            ("ends-options", []) => Some(OptionEffectKind::EndsOptions),
            _ => None,
        });
    if kind.is_none() {
        log.say(line, format!("unreadable option effect `{text}`; dropped"));
    }
    kind
}

/// `option_effect_family NAME { base all-on|all-off|{only AXIS VALUE} \
/// combine accumulate|last-wins ?-introduced V? }` at command or subcommand
/// scope — the families an option row's `-effect` cites by name.
pub(super) fn option_effect_family_row(stmt: &Stmt, log: &mut Log) -> Option<OptionEffectFamily> {
    let name = leak_str(stmt.word_text(1));
    let Some(block_word) = stmt.arg(2) else {
        log.say(
            stmt.line,
            format!("`option_effect_family {name}` needs a `{{ … }}` block; dropped"),
        );
        return None;
    };
    let words: Vec<Word> = block(block_word)
        .into_iter()
        .flat_map(|row| row.words)
        .collect();
    let mut base: Option<FamilyBase> = None;
    let mut combine: Option<FamilyCombine> = None;
    let mut surface = None;
    let mut i = 0;
    while i < words.len() {
        let rejected_flag_word = words[i].clone();
        match words[i].text.as_str() {
            "base" => {
                let text = next_text(&words, &mut i);
                let parts = list_words(&text);
                base = match parts.split_first() {
                    Some((head, [])) if head == "all-on" => Some(FamilyBase::AllOn),
                    Some((head, [])) if head == "all-off" => Some(FamilyBase::AllOff),
                    Some((head, axis)) if head == "only" => {
                        effect_axis_words(axis, stmt.line, log).map(FamilyBase::Only)
                    }
                    _ => None,
                };
                if base.is_none() {
                    log.say(
                        stmt.line,
                        format!("unreadable `option_effect_family` base `{text}`"),
                    );
                }
            }
            "combine" => {
                let text = next_text(&words, &mut i);
                combine = FamilyCombine::from_spelling(&text);
                if combine.is_none() {
                    log.say(
                        stmt.line,
                        format!("unknown option-effect family combine rule `{text}` dropped"),
                    );
                }
            }
            "-introduced" => {
                let text = next_text(&words, &mut i);
                surface =
                    available::from_texts(&["tcl".to_owned(), format!("{text}-")], stmt.line, log)
                        .surface();
            }
            _ => log.unknown_flag("option_effect_family", stmt.line, &rejected_flag_word),
        }
        i += 1;
    }
    if let (Some(base), Some(combine)) = (base, combine) {
        Some(OptionEffectFamily {
            name,
            base,
            combine,
            surface,
        })
    } else {
        log.say(
            stmt.line,
            format!("`option_effect_family {name}` needs both `base` and `combine`; dropped"),
        );
        None
    }
}

/// Parse one `option NAME …` row, returning the spec and any `-arity-hook`.
#[allow(clippy::too_many_lines)]
pub(super) fn option_row(
    stmt: &Stmt,
    tables: &PackTables,
    log: &mut Log,
) -> (OptionSpec, Option<(HookSource, String)>) {
    let name = leak_str(stmt.word_text(1));
    let mut option = OptionSpec {
        name,
        ..OptionSpec::DEFAULT
    };
    let mut arg: Option<OptionArg> = None;
    let mut hook: Option<(HookSource, String)> = None;
    // Defaults are intentionally valid for every value role.  Remember which
    // semantic flags the author actually wrote so validation can reject only
    // contradictory declarations, not an untouched default.
    let mut wrote_script_timing = false;
    let mut wrote_callback_taint_inputs = false;
    let mut wrote_variable_scope = false;
    let mut wrote_taints_var_write = false;
    let mut effect_kind: Option<OptionEffectKind> = None;
    let mut effect_family: Option<&'static str> = None;
    let mut wrote_effect = false;
    let mut wrote_family = false;

    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        let rejected_flag_word = words[i].clone();
        let flag = words[i].text.clone();
        match flag.as_str() {
            "-takes" => {
                let hint = next_text(words, &mut i);
                let entry = arg.get_or_insert(OptionArg::DEFAULT);
                entry.hint = leak_str(&hint);
            }
            "-detail" => option.detail = leak_str(&next_text(words, &mut i)),
            "-aliases" => {
                let text = next_text(words, &mut i);
                option.aliases = leak_strs(&list_words(&text));
            }
            "-dialects" => {
                let text = next_text(words, &mut i);
                option.surface = parse_dialects(&text, stmt.line, log);
            }
            "-available" => {
                log.v20(stmt.line, "-available");
                let text = next_text(words, &mut i);
                let availability = available::from_flag(&text, stmt.line, log);
                apply_availability(&mut option.surface, availability, "option", stmt.line, log);
            }
            "-min-abbrev" => option.min_abbrev = next_text(words, &mut i).parse().ok(),
            "-effect" => {
                wrote_effect = true;
                let text = next_text(words, &mut i);
                effect_kind = option_effect_kind(&text, stmt.line, log);
            }
            "-family" => {
                wrote_family = true;
                effect_family = Some(leak_str(&next_text(words, &mut i)));
            }
            // The data form only: `{-replace WORD ?-replace-arg N? …}`, read by
            // the same flag reader the command-level `deprecation_fix`
            // statement uses. The contextual-callback variant stays
            // reference-only.
            "-deprecation-fix" => {
                log.v11(stmt.line, "-deprecation-fix");
                i += 1;
                match words.get(i) {
                    Some(value) => {
                        let block: Vec<Word> = block(value)
                            .into_iter()
                            .flat_map(|stmt| stmt.words)
                            .collect();
                        option.lifecycle.deprecation_fix =
                            deprecation_fix_from(&block, 0, stmt.line, log);
                    }
                    None => log.say(stmt.line, "`-deprecation-fix` needs a `{…}` block"),
                }
            }
            "-arity" => {
                let text = next_text(words, &mut i);
                let parts = list_words(&text);
                let entry = arg.get_or_insert(OptionArg::DEFAULT);
                match parts.split_first() {
                    Some((head, rest)) if head == "Fixed" && rest.len() == 1 => {
                        match rest[0].parse() {
                            Ok(n) => entry.arity = OptionArity::Fixed(n),
                            Err(_) => log.say(stmt.line, format!("unreadable `-arity {text}`")),
                        }
                    }
                    Some((head, rest)) if head == "One" && rest.is_empty() => {
                        entry.arity = OptionArity::One;
                    }
                    Some((head, rest)) if head == "OptionalOneOf" && !rest.is_empty() => {
                        entry.arity = OptionArity::OptionalOneOf(leak_strs(rest));
                    }
                    Some((head, rest)) if head == "OptionalNotOption" && rest.is_empty() => {
                        entry.arity = OptionArity::OptionalNotOption;
                    }
                    _ => log.say(
                        stmt.line,
                        format!("`-arity {text}` is not a static option arity; use -arity-hook"),
                    ),
                }
            }
            "-arity-hook" => {
                let entry = arg.get_or_insert(OptionArg::DEFAULT);
                entry.arity = OptionArity::Hook(consume_one_word);
                let first = next_text(words, &mut i);
                if first == "-native" {
                    let id = next_text(words, &mut i);
                    hook = Some((HookSource::Native { id }, option.name.to_owned()));
                } else {
                    let body = next_text(words, &mut i);
                    hook = Some((
                        HookSource::Body {
                            params: list_words(&first),
                            body,
                            // An option row's `-arity-hook` has no `-inputs`
                            // flag of its own: the row's own flag parser owns
                            // this word position, and the family reads the
                            // option keys, which are not part of the shape
                            // key — so it is uncacheable by construction.
                            inputs: HookInputs::unrestricted(),
                        },
                        option.name.to_owned(),
                    ));
                }
            }
            "-role" => {
                let name = next_text(words, &mut i);
                let entry = arg.get_or_insert(OptionArg::DEFAULT);
                if let Some(role) =
                    enum_by_name(ArgRole::ALL, &name, "argument role", stmt.line, log)
                {
                    entry.role = role;
                }
            }
            "-also-role" => {
                let name = next_text(words, &mut i);
                let entry = arg.get_or_insert(OptionArg::DEFAULT);
                entry.also_role =
                    enum_by_name(ArgRole::ALL, &name, "argument role", stmt.line, log);
            }
            "-body-kind" => {
                let name = next_text(words, &mut i);
                let entry = arg.get_or_insert(OptionArg::DEFAULT);
                if let Some(kind) = enum_by_name(BODY_KINDS, &name, "body kind", stmt.line, log) {
                    entry.body_kind = kind;
                }
            }
            "-script-timing" => {
                log.v12(stmt.line, "-script-timing");
                wrote_script_timing = true;
                let name = next_text(words, &mut i);
                let entry = arg.get_or_insert(OptionArg::DEFAULT);
                if let Some(timing) =
                    enum_by_name(SCRIPT_TIMINGS, &name, "script timing", stmt.line, log)
                {
                    entry.script_timing = timing;
                }
            }
            "-callback-taint-inputs" => {
                log.v12(stmt.line, "-callback-taint-inputs");
                wrote_callback_taint_inputs = true;
                let text = next_text(words, &mut i);
                arg.get_or_insert(OptionArg::DEFAULT).callback_taint_inputs =
                    parse_callback_taint_inputs(&text, stmt.line, log);
            }
            "-variable-scope" => {
                log.v12(stmt.line, "-variable-scope");
                wrote_variable_scope = true;
                let name = next_text(words, &mut i);
                let entry = arg.get_or_insert(OptionArg::DEFAULT);
                if let Some(scope) =
                    enum_by_name(VARIABLE_SCOPES, &name, "variable scope", stmt.line, log)
                {
                    entry.variable_scope = scope;
                }
            }
            "-values" => {
                let text = next_text(words, &mut i);
                let values: Vec<ArgValue> = list_words(&text)
                    .iter()
                    .map(|value| ArgValue {
                        value: leak_str(value),
                        ..ArgValue::DEFAULT
                    })
                    .collect();
                arg.get_or_insert(OptionArg::DEFAULT).values = leak_slice(values);
            }
            "-values-from" => {
                let name = next_text(words, &mut i);
                match tables.values.get(&name) {
                    Some(values) => {
                        arg.get_or_insert(OptionArg::DEFAULT).values = leak_slice(values.clone());
                    }
                    None => log.say(
                        stmt.line,
                        format!("no `values {name}` table in this pack; -values-from dropped"),
                    ),
                }
            }
            "-closed" => arg.get_or_insert(OptionArg::DEFAULT).closed = true,
            "-taints-var-write" => {
                log.v12(stmt.line, "-taints-var-write");
                wrote_taints_var_write = true;
                arg.get_or_insert(OptionArg::DEFAULT).taints_var_write = true;
            }
            "-integer" => {
                let text = next_text(words, &mut i);
                arg.get_or_insert(OptionArg::DEFAULT).integer =
                    parse_integer_domain(&text, stmt.line, log);
            }
            "-appends" => {
                let text = next_text(words, &mut i);
                if let Some(appended) = parse_appended_arity(&text, stmt.line, log) {
                    arg.get_or_insert(OptionArg::DEFAULT).appended_arity = appended;
                }
            }
            other => {
                if !lifecycle_flag(&mut option.lifecycle, other, words, &mut i) {
                    log.unknown_flag("option", stmt.line, &rejected_flag_word);
                }
            }
        }
        i += 1;
    }

    if let Some(mut arg) = arg {
        let roles = [Some(arg.role), arg.also_role];
        let executable = roles.into_iter().flatten().any(ArgRole::has_script_timing);
        let variable = roles.into_iter().flatten().any(is_variable_role);
        let variable_write = roles
            .into_iter()
            .flatten()
            .any(|role| role == ArgRole::VarWrite);

        if wrote_script_timing && !executable {
            log.say(
                stmt.line,
                format!(
                    "option `{}` declares `-script-timing` on a non-executable value; dropped",
                    option.name
                ),
            );
            arg.script_timing = ScriptTiming::SameInvocation;
        }
        if wrote_callback_taint_inputs
            && (!executable || arg.script_timing != ScriptTiming::Deferred)
        {
            log.say(
                stmt.line,
                format!(
                    "option `{}` declares callback taint inputs for a value that is not a deferred executable; dropped",
                    option.name
                ),
            );
            arg.callback_taint_inputs = &[];
        }
        if wrote_variable_scope && !variable {
            log.say(
                stmt.line,
                format!(
                    "option `{}` declares `-variable-scope` on a non-variable value; dropped",
                    option.name
                ),
            );
            arg.variable_scope = VariableScope::CurrentFrame;
        }
        if wrote_taints_var_write && !variable_write {
            log.say(
                stmt.line,
                format!(
                    "option `{}` declares `-taints-var-write` without a VarWrite role; dropped",
                    option.name
                ),
            );
            arg.taints_var_write = false;
        }
        option.value = OptionValue::Takes(arg);
    }
    option.effect = if let (Some(kind), Some(family)) = (effect_kind, effect_family) {
        Some(OptionEffect { kind, family })
    } else {
        if wrote_effect && !wrote_family {
            log.say(
                stmt.line,
                format!(
                    "option `{}` declares `-effect` without `-family`; dropped",
                    option.name
                ),
            );
        } else if wrote_family && !wrote_effect {
            log.say(
                stmt.line,
                format!(
                    "option `{}` declares `-family` without `-effect`; dropped",
                    option.name
                ),
            );
        }
        None
    };
    option.lifecycle = checked_lifecycle(
        option.lifecycle,
        &format!("option `{}`", option.name),
        stmt.line,
        log,
    );
    (option, hook)
}

// Hover

/// The hover text, plus the structured worked examples among its `example`
/// rows. An `example` whose braced word starts with a `code` row is a
/// [`DocumentationExample`]; any other `example` is hover text, as before.
pub(super) fn hover_block(
    stmts: &[Stmt],
    log: &mut Log,
) -> (
    HoverSnippet,
    Vec<tcl_registry::documentation::DocumentationExample>,
) {
    let mut structured = Vec::new();
    let mut summary = String::new();
    let mut synopsis: Vec<String> = Vec::new();
    let mut snippet = String::new();
    let mut source = String::new();
    let mut examples: Vec<String> = Vec::new();
    let mut return_value = String::new();
    for stmt in stmts {
        let value = stmt.word_text(1).to_owned();
        match stmt.word_text(0) {
            "summary" => summary = value,
            "synopsis" => synopsis.push(value),
            // Three words are renamed from their Rust field names; nothing
            // else in the DSL renames a key.
            "description" => snippet = value,
            "source" => source = value,
            "example" => match stmt.arg(1).map(block) {
                Some(rows) if rows.first().is_some_and(|row| row.word_text(0) == "code") => {
                    if let Some(example) = documentation_example(&rows, log) {
                        structured.push(example);
                    }
                }
                _ => examples.push(value),
            },
            "returns" => return_value = value,
            _ => log.unknown_property(stmt),
        }
    }
    let hover = HoverSnippet {
        summary: leak_str(&summary),
        synopsis: leak_strs(&synopsis),
        snippet: leak_str(&snippet),
        source: leak_str(&source),
        // Repeated `example` rows join with a single newline.
        examples: leak_str(&examples.join("\n")),
        return_value: leak_str(&return_value),
    };
    (hover, structured)
}

/// `example { code SCRIPT; carrier LINE NEEDLE; note LINE NEEDLE LABEL … }`.
///
/// Lines are zero-based lines of `code`, and every needle must occur on its
/// line: an annotation pointing at text the code does not contain is
/// refused with the whole example rather than drawn somewhere else.
fn documentation_example(
    rows: &[Stmt],
    log: &mut Log,
) -> Option<tcl_registry::documentation::DocumentationExample> {
    use tcl_registry::documentation::{
        DocumentationAnnotation, DocumentationCarrier, DocumentationExample,
    };
    let code = rows.first()?.word_text(1);
    let lines: Vec<&str> = code.lines().collect();
    let located = |row: &Stmt, log: &mut Log| -> Option<(usize, &'static str)> {
        let line = row.word_text(1).parse::<usize>().ok();
        let needle = row.word_text(2);
        match line {
            Some(line)
                if lines.get(line).is_some_and(|text| text.contains(needle))
                    && !needle.is_empty() =>
            {
                Some((line, leak_str(needle)))
            }
            _ => {
                log.say(
                    row.line,
                    format!(
                        "`{}` needs a 0-based line of `code` and text on that line; `{} {}` is not",
                        row.word_text(0),
                        row.word_text(1),
                        needle
                    ),
                );
                None
            }
        }
    };
    let mut carrier = None;
    let mut notes = Vec::new();
    for row in &rows[1..] {
        match row.word_text(0) {
            "carrier" => carrier = Some(located(row, log)?),
            "note" => {
                let (line, needle) = located(row, log)?;
                notes.push(DocumentationAnnotation::new(
                    line,
                    needle,
                    leak_str(row.word_text(3)),
                ));
            }
            _ => {
                log.unknown_property(row);
                return None;
            }
        }
    }
    let code = leak_str(code);
    let notes: &'static [DocumentationAnnotation] = Box::leak(notes.into_boxed_slice());
    Some(match carrier {
        Some((line, needle)) => {
            DocumentationExample::with_carrier(code, DocumentationCarrier::new(line, needle), notes)
        }
        None => DocumentationExample::new(code, notes),
    })
}
