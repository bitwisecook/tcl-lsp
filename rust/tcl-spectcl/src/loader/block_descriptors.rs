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

//! Block-valued descriptors: case lists, clause grammars, definition bodies with their members, the iRules event rows and scoped bodies.

use super::body_rows::hover_block;
use super::command::resolve_block;
use super::statement_reader::{Stmt, block};
use super::subcommand::load_subcommand;
use super::value_parsers::{
    enum_by_name, list_words, next_text, parse_arity, parse_dialects, parse_flag,
};
use super::{HookDecl, Log, PackTables, leak_one, leak_slice, leak_str, leak_strs};
use tcl_registry::arity::Arity;
use tcl_registry::definer::DefinitionBodyGrammar;
use tcl_registry::events::{
    DataCollectionOperation, EventHandlerPriority, EventRequirementForm, EventRequires,
};
use tcl_registry::result_stability::ResultStability;
use tcl_registry::scoped::{ScopedCommand, ScopedCommandEnv};
use tcl_registry::side_effects::SideSwitchTarget;
use tcl_registry::spec::{CaseListSpec, SubCommand};
use tcl_registry::world_effect::WorldStateDomain;

mod clause_grammar;
mod definition_body;

pub use clause_grammar::roles_from_manufacturers;
pub(super) use clause_grammar::{
    check_clause_grammar, clause_grammar_value, record_clause_grammar_derivations,
};
pub(super) use definition_body::{definition_body_block, manufacturer_row};

// Block-valued descriptors

pub(super) fn event_requires_block(stmts: &[Stmt], log: &mut Log) -> EventRequires {
    let mut requires = EventRequires {
        client_side: false,
        server_side: false,
        transport: None,
        profiles: &[],
        also_in: &[],
        flow: false,
    };
    for stmt in stmts {
        let value = stmt.word_text(1).to_owned();
        match stmt.word_text(0) {
            "client_side" => requires.client_side = parse_flag(stmt.tail()),
            "server_side" => requires.server_side = parse_flag(stmt.tail()),
            "flow" => requires.flow = parse_flag(stmt.tail()),
            "transport" => requires.transport = Some(leak_str(&value)),
            "profiles" => requires.profiles = leak_strs(&list_words(&value)),
            "also_in" => requires.also_in = leak_strs(&list_words(&value)),
            _ => log.unknown_property(stmt),
        }
    }
    requires
}

pub(super) fn case_list_block(stmts: &[Stmt], log: &mut Log) -> CaseListSpec {
    let mut spec = CaseListSpec {
        subject_args: 0,
        two_arg_optionless_surface: None,
        fallthrough_body: None,
        value_options_require_regex: &[],
        special_match_options: &[],
        clause_flags: &[],
        clause_regex_flag: None,
        clause_value_flags: &[],
        clause_end_options_flag: None,
        clause_force_inline_flag: None,
        clause_force_list_flag: None,
        clause_force_list_shape: None,
        allow_omitted_final_body: false,
        keyword_patterns: &[],
        keyword_patterns_require_final: false,
        exhaustive_keyword_patterns: &[],
        optional_subject_separator: None,
        warn_unbraced_bodies: false,
        // Unless the block says otherwise, a pack's case list compares
        // exactly where no option row selects a mode, and reads each
        // pattern word as one pattern.
        default_mode: tcl_registry::spec::CaseMatchMode::Exact,
        pattern_words: tcl_registry::spec::PatternWords::Single,
    };
    for stmt in stmts {
        let value = stmt.word_text(1).to_owned();
        match stmt.word_text(0) {
            "subject_args" => spec.subject_args = value.parse().unwrap_or(0),
            "two_arg_optionless_surface" => {
                spec.two_arg_optionless_surface = parse_dialects(&value, stmt.line, log);
            }
            // The command-level switches that pick the match mode, fold case
            // or end the option run are the command's own option rows, each
            // declaring its effect; the descriptor no longer names them.
            retired @ ("exact_option" | "glob_option" | "regex_option" | "nocase_option"
            | "end_options_option") => log.say(
                stmt.line,
                format!(
                    "`case_list` row `{retired}` is retired: a match-mode, case-folding or \
                     terminator switch is the command's own option row, declaring its effect; \
                     dropped"
                ),
            ),
            "fallthrough_body" => spec.fallthrough_body = Some(leak_str(&value)),
            "value_options_require_regex" => {
                spec.value_options_require_regex = leak_strs(&list_words(&value));
            }
            "special_match_options" => {
                spec.special_match_options = leak_strs(&list_words(&value));
            }
            "clause_flags" => spec.clause_flags = leak_strs(&list_words(&value)),
            "clause_regex_flag" => spec.clause_regex_flag = Some(leak_str(&value)),
            "clause_value_flags" => spec.clause_value_flags = leak_strs(&list_words(&value)),
            "clause_end_options_flag" => spec.clause_end_options_flag = Some(leak_str(&value)),
            "clause_force_inline_flag" => spec.clause_force_inline_flag = Some(leak_str(&value)),
            "clause_force_list_flag" => spec.clause_force_list_flag = Some(leak_str(&value)),
            "clause_force_list_shape" => {
                spec.clause_force_list_shape = match value.as_str() {
                    "first_arg_only_remainder" => {
                        Some(tcl_registry::CaseForceListShape::FirstArgOnlyRemainder)
                    }
                    _ => None,
                };
            }
            "allow_omitted_final_body" => spec.allow_omitted_final_body = parse_flag(stmt.tail()),
            "optional_subject_separator" => {
                spec.optional_subject_separator = Some(leak_str(&value));
            }
            "warn_unbraced_bodies" => spec.warn_unbraced_bodies = parse_flag(stmt.tail()),
            "exhaustive_keyword_patterns" => {
                spec.exhaustive_keyword_patterns = leak_strs(&list_words(&value));
            }
            // A specialised comparison is an option row's, never the
            // default a clause makes.
            "default_mode" => match tcl_registry::spec::CaseMatchMode::from_spelling(&value)
                .filter(|mode| *mode != tcl_registry::spec::CaseMatchMode::Other)
            {
                Some(mode) => spec.default_mode = mode,
                None => log.say(
                    stmt.line,
                    format!(
                        "`case_list` row `default_mode` takes `exact`, `glob` or `regexp`, not \
                         `{value}`; kept `exact`"
                    ),
                ),
            },
            "pattern_words" => match tcl_registry::spec::PatternWords::from_spelling(&value) {
                Some(reading) => spec.pattern_words = reading,
                None => log.say(
                    stmt.line,
                    format!(
                        "`case_list` row `pattern_words` takes `single` or `lists`, not \
                         `{value}`; kept `single`"
                    ),
                ),
            },
            "keyword_patterns" => {
                spec.keyword_patterns = leak_strs(&list_words(&value));
                spec.keyword_patterns_require_final =
                    stmt.words.iter().any(|w| w.text == "-final-only");
            }
            _ => log.unknown_property(stmt),
        }
    }
    spec
}

/// The shipped definer grammars a pack may name — `definition_body NAME` —
/// with the name each is written under. The studio seeds a grammar whose data
/// is one of these as its name, so the list is the one both sides read.
pub const SHIPPED_DEFINITION_BODIES: &[(&str, &DefinitionBodyGrammar)] = &[
    ("tcloo", &tcl_registry::definer::TCLOO_GRAMMAR),
    (
        "tcloo-abstract",
        &tcl_registry::definer::TCLOO_ABSTRACT_GRAMMAR,
    ),
    (
        "tcloo-configurable",
        &tcl_registry::definer::TCLOO_CONFIGURABLE_GRAMMAR,
    ),
    ("snit", &tcl_registry::definer::SNIT_GRAMMAR),
    ("snit-widget", &tcl_registry::definer::SNIT_WIDGET_GRAMMAR),
    ("itcl", &tcl_registry::definer::ITCL_GRAMMAR),
    ("clay", &tcl_registry::definer::CLAY_GRAMMAR),
];

/// The shipped definer grammar `name` names.
pub(super) fn shipped_definition_body(name: &str) -> Option<&'static DefinitionBodyGrammar> {
    SHIPPED_DEFINITION_BODIES
        .iter()
        .find(|(shipped, _)| *shipped == name)
        .map(|(_, grammar)| *grammar)
}

/// The shipped case-list descriptors a pack may name.
pub(super) fn shipped_case_list(name: &str) -> Option<&'static CaseListSpec> {
    match name {
        "switch" => Some(&CaseListSpec::SWITCH),
        _ => None,
    }
}

/// The shipped scoped-body environments a pack may name.
fn shipped_body_scope(name: &str) -> Option<&'static ScopedCommandEnv> {
    match name {
        "report-defstyle" => Some(&tcl_registry::scoped::REPORT_DEFSTYLE_ENV),
        "tclpkg-manifest" => Some(&tcl_registry::scoped::TCLPKG_MANIFEST_ENV),
        _ => None,
    }
}

// The ratified words (design §6.2)
//
// Seven words the DSL memo's coverage matrix has always documented and the
// loader had no reader for — the `DraftOpaque`-masks-`LoaderGap` blind spot
// §6.3 names. They are not *new* vocabulary, so they draw no per-site
// version notice: a 1.x pack that spelled one was always meant to load it.

/// Every result-stability spelling with no payload.
const RESULT_STABILITIES: &[ResultStability] = &[
    ResultStability::Unknown,
    ResultStability::ReferentiallyTransparent,
    ResultStability::Volatile,
];

/// The versioned world-state domains a `ReadsVersionedWorld` may list.
const WORLD_STATE_DOMAINS: &[WorldStateDomain] = &[
    WorldStateDomain::InterpreterTopology,
    WorldStateDomain::CommandBindings,
    WorldStateDomain::NamespaceLookup,
    WorldStateDomain::NamespaceUnknown,
    WorldStateDomain::ExecutionTraces,
    WorldStateDomain::VariableTraces,
    WorldStateDomain::CommandTraces,
    WorldStateDomain::OoDispatch,
    WorldStateDomain::InterpreterPolicy,
    WorldStateDomain::InterpreterResult,
    WorldStateDomain::CompletionState,
    WorldStateDomain::PackageState,
    WorldStateDomain::HostCapabilities,
];

/// The sides a `side_switch_target` may select.
pub(super) const SIDE_SWITCH_TARGETS: &[SideSwitchTarget] = &[
    SideSwitchTarget::Client,
    SideSwitchTarget::Server,
    SideSwitchTarget::Peer,
];

pub(super) const IRULES_SOURCE_FLOWS: &[tcl_registry::events::IrulesSourceFlowSpec] = &[
    tcl_registry::events::IrulesSourceFlowSpec::DropsConnection,
    tcl_registry::events::IrulesSourceFlowSpec::ReturnsDnsPacket,
    tcl_registry::events::IrulesSourceFlowSpec::EventControl,
];

/// `result_stability Unknown|ReferentiallyTransparent|Volatile|{ReadsVersionedWorld {D …}}`.
///
/// The payload-carrying variant is one braced word holding the variant name
/// and its domain list, which is how the memo spells every payload variant.
pub(super) fn result_stability_row(stmt: &Stmt, log: &mut Log) -> Option<ResultStability> {
    let value = stmt.word_text(1);
    let words = list_words(value);
    if words.first().map(String::as_str) == Some("ReadsVersionedWorld") {
        let listed = words.get(1).map_or_default(|text| list_words(text));
        if listed.is_empty() {
            log.say(
                stmt.line,
                "`result_stability {ReadsVersionedWorld {D …}}` needs at least one \
                 world-state domain; the row is dropped",
            );
            return None;
        }
        let domains: Vec<WorldStateDomain> = listed
            .iter()
            .filter_map(|name| {
                enum_by_name(
                    WORLD_STATE_DOMAINS,
                    name,
                    "world-state domain",
                    stmt.line,
                    log,
                )
            })
            .collect();
        if domains.len() != listed.len() {
            // An unreadable domain would silently *narrow* the dependency
            // set, which is the direction that makes reuse unsound.
            return None;
        }
        return Some(ResultStability::ReadsVersionedWorld(leak_slice(domains)));
    }
    enum_by_name(
        RESULT_STABILITIES,
        value,
        "result stability",
        stmt.line,
        log,
    )
}

/// `event_handler_priority -default N ?-warn-implicit? …`.
///
/// `-default` is the one required flag; the rest of `EventHandlerPriority`
/// takes the open defaults (`priority`, the whole `u16` range, lower first)
/// so a pack states only what its dialect actually constrains.
pub(super) fn event_handler_priority_row(
    stmt: &Stmt,
    log: &mut Log,
) -> Option<EventHandlerPriority> {
    let mut policy = EventHandlerPriority {
        keyword: "priority",
        default_priority: 0,
        min_priority: 0,
        max_priority: u16::MAX,
        lower_runs_first: true,
        warn_when_implicit: false,
    };
    let mut stated_default = false;
    let words = &stmt.words;
    let mut index = 1;
    while index < words.len() {
        let rejected_flag_word = words[index].clone();
        match words[index].text.as_str() {
            "-keyword" => policy.keyword = leak_str(&next_text(words, &mut index)),
            "-default" => {
                let text = next_text(words, &mut index);
                let Ok(value) = text.parse() else {
                    log.say(
                        stmt.line,
                        format!("`event_handler_priority -default {text}` is not a priority"),
                    );
                    return None;
                };
                policy.default_priority = value;
                stated_default = true;
            }
            "-min" => policy.min_priority = next_text(words, &mut index).parse().unwrap_or(0),
            "-max" => {
                policy.max_priority = next_text(words, &mut index).parse().unwrap_or(u16::MAX);
            }
            "-higher-runs-first" => policy.lower_runs_first = false,
            "-warn-implicit" => policy.warn_when_implicit = true,
            _ => log.unknown_flag("event_handler_priority", stmt.line, &rejected_flag_word),
        }
        index += 1;
    }
    if !stated_default {
        log.say(
            stmt.line,
            "`event_handler_priority` needs `-default N` — the priority the runtime \
             uses when the keyword is omitted; the row is dropped",
        );
        return None;
    }
    if policy.min_priority > policy.max_priority || !policy.accepts(policy.default_priority) {
        log.say(
            stmt.line,
            format!(
                "`event_handler_priority -default {}` is outside `{}..={}`; the row is dropped",
                policy.default_priority, policy.min_priority, policy.max_priority
            ),
        );
        return None;
    }
    Some(policy)
}

/// The shared data-collection descriptors, by the id `-native` names.
///
/// Reference-only by design: the descriptor is paired with protocol
/// machinery outside the registry, so a pack names one rather than
/// spelling it.
fn data_collection_by_id(id: &str) -> Option<DataCollectionOperation> {
    use tcl_registry::events as ev;
    let found = match id {
        "HTTP_COLLECT" => ev::HTTP_COLLECT,
        "HTTP_RELEASE" => ev::HTTP_RELEASE,
        "HTTP_PAYLOAD" => ev::HTTP_PAYLOAD,
        "TCP_COLLECT" => ev::TCP_COLLECT,
        "TCP_RELEASE" => ev::TCP_RELEASE,
        "TCP_PAYLOAD" => ev::TCP_PAYLOAD,
        "SSL_COLLECT" => ev::SSL_COLLECT,
        "SSL_RELEASE" => ev::SSL_RELEASE,
        "SSL_PAYLOAD" => ev::SSL_PAYLOAD,
        "UDP_PAYLOAD" => ev::UDP_PAYLOAD,
        "ASM_PAYLOAD" => ev::ASM_PAYLOAD,
        "MQTT_COLLECT" => ev::MQTT_COLLECT,
        "MQTT_RELEASE" => ev::MQTT_RELEASE,
        "MQTT_PAYLOAD" => ev::MQTT_PAYLOAD,
        "MR_COLLECT" => ev::MR_COLLECT,
        "MR_RELEASE" => ev::MR_RELEASE,
        "MR_PAYLOAD" => ev::MR_PAYLOAD,
        "RTSP_COLLECT" => ev::RTSP_COLLECT,
        "RTSP_RELEASE" => ev::RTSP_RELEASE,
        "RTSP_PAYLOAD" => ev::RTSP_PAYLOAD,
        "SCTP_COLLECT" => ev::SCTP_COLLECT,
        "SCTP_RELEASE" => ev::SCTP_RELEASE,
        "SCTP_PAYLOAD" => ev::SCTP_PAYLOAD,
        "WS_COLLECT" => ev::WS_COLLECT,
        "WS_RELEASE" => ev::WS_RELEASE,
        "WS_PAYLOAD" => ev::WS_PAYLOAD,
        "CACHE_PAYLOAD" => ev::CACHE_PAYLOAD,
        "DIAMETER_PAYLOAD" => ev::DIAMETER_PAYLOAD,
        "GTP_PAYLOAD" => ev::GTP_PAYLOAD,
        "REWRITE_PAYLOAD" => ev::REWRITE_PAYLOAD,
        "SIP_PAYLOAD" => ev::SIP_PAYLOAD,
        "XML_PAYLOAD" => ev::XML_PAYLOAD,
        _ => return None,
    };
    Some(found)
}

/// `data_collection -native ID`.
pub(super) fn data_collection_row(stmt: &Stmt, log: &mut Log) -> Option<DataCollectionOperation> {
    if stmt.word_text(1) != "-native" {
        log.say(stmt.line, "`data_collection` takes `-native ID`");
        return None;
    }
    let id = stmt.word_text(2);
    let found = data_collection_by_id(id);
    if found.is_none() {
        log.say(
            stmt.line,
            format!("unknown data-collection descriptor `{id}` dropped"),
        );
    }
    found
}

/// `event_requirement_form {word …} ?-only-in {E …}? ?{ … }?`.
///
/// The literal selector is the row's own first word; the trailing block, when
/// there is one, is a nested `event_requires` read by the same reader the
/// standalone row uses.
pub(super) fn event_requirement_form_row(stmt: &Stmt, log: &mut Log) -> EventRequirementForm {
    let prefix = list_words(stmt.word_text(1));
    let mut only_in: Vec<String> = Vec::new();
    let mut requires = None;
    let words = &stmt.words;
    let mut index = 2;
    while index < words.len() {
        let rejected_flag_word = words[index].clone();
        match words[index].text.as_str() {
            "-only-in" => only_in = list_words(&next_text(words, &mut index)),
            _ if words[index].braced => {
                requires = Some(event_requires_block(&block(&words[index]), log));
            }
            _ => log.unknown_flag("event_requirement_form", stmt.line, &rejected_flag_word),
        }
        index += 1;
    }
    EventRequirementForm {
        argument_prefix: leak_strs(&prefix),
        requires,
        only_in: leak_strs(&only_in),
    }
}

/// `body_scope NAME | { … }` — a shipped environment by name, a pack
/// `descriptor body_scope NAME { … }`, or an inline block.
pub(super) fn body_scope_value(
    stmt: &Stmt,
    tables: &PackTables,
    hooks: &mut Vec<HookDecl>,
    log: &mut Log,
) -> Option<&'static ScopedCommandEnv> {
    let word = stmt.arg(1)?;
    if !word.braced
        && let Some(shipped) = shipped_body_scope(&word.text)
    {
        return Some(shipped);
    }
    resolve_block(stmt, "body_scope", tables, log)
        .map(|stmts| leak_one(body_scope_block(&stmts, tables, hooks, log)))
}

/// The rows of a `body_scope { … }` block: `ScopedCommandEnv`'s own fields.
fn body_scope_block(
    stmts: &[Stmt],
    tables: &PackTables,
    hooks: &mut Vec<HookDecl>,
    log: &mut Log,
) -> ScopedCommandEnv {
    let mut env = ScopedCommandEnv {
        name: "",
        commands: &[],
        include_sibling_definitions: false,
        allow_unknown_commands: false,
    };
    let mut commands: Vec<ScopedCommand> = Vec::new();
    for stmt in stmts {
        match stmt.word_text(0) {
            "name" => env.name = leak_str(stmt.word_text(1)),
            "include_sibling_definitions" => {
                env.include_sibling_definitions = parse_flag(stmt.tail());
            }
            "allow_unknown_commands" => env.allow_unknown_commands = parse_flag(stmt.tail()),
            "command" => {
                if let Some(command) = scoped_command_row(stmt, tables, hooks, log) {
                    commands.push(command);
                }
            }
            _ => log.unknown_property(stmt),
        }
    }
    env.commands = leak_slice(commands);
    env
}

/// One `command NAME { … }` of a `body_scope` block.
fn scoped_command_row(
    stmt: &Stmt,
    tables: &PackTables,
    hooks: &mut Vec<HookDecl>,
    log: &mut Log,
) -> Option<ScopedCommand> {
    let name = stmt.word_text(1);
    if name.is_empty() {
        log.say(stmt.line, "a `body_scope` `command` needs a name");
        return None;
    }
    let Some(body) = stmt.arg(2) else {
        log.say(
            stmt.line,
            format!("`command {name}` in a `body_scope` has no `{{ … }}` block"),
        );
        return None;
    };
    let mut command = ScopedCommand {
        name: leak_str(name),
        arity: Arity::any(),
        subcommands: &[],
        allow_unknown_subcommands: false,
        detail: "",
        hover: None,
    };
    let mut subcommands: Vec<SubCommand> = Vec::new();
    log.scoped(format!("body_scope command {name}"), |log| {
        for row in block(body) {
            match row.word_text(0) {
                "arity" => command.arity = parse_arity(&row, log).0,
                "detail" => command.detail = leak_str(row.word_text(1)),
                "allow_unknown_subcommands" => {
                    command.allow_unknown_subcommands = parse_flag(row.tail());
                }
                "hover" => {
                    if let Some(word) = row.arg(1) {
                        let (hover, examples) = hover_block(&block(word), log);
                        if !examples.is_empty() {
                            log.say(
                                row.line,
                                "a body-scope command has no structured examples; they stay hover text",
                            );
                        }
                        command.hover = Some(hover);
                    }
                }
                "subcommand" => {
                    if let Some(sub) =
                        load_subcommand(&row, tables, hooks, "body_scope subcommand", log)
                    {
                        subcommands.push(sub);
                    }
                }
                _ => log.unknown_property(&row),
            }
        }
    });
    command.subcommands = leak_slice(subcommands);
    Some(command)
}
