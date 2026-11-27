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

//! Replay of a completed evaluation into a `Pack`: the provenance rule, the staged blocks and commands and their registration record.

use super::super::{
    CommandAcc, CommandSpec, LoadError, Log, Notice, Pack, PackTables, Stmt, SubAcc, SubCommand,
    VocabularyClass, apply_command_stmt, apply_pack_stmt, apply_subcommand_stmt,
    check_vocabulary_version, command_from_parts, empty_pack, environment_block,
    finish_newer_words, subcommand_from_parts,
};
use super::capture_handlers::capture_stmt;
use super::{EvalOptions, Node, PackNode, StagedBlock, State};
use crate::discovery::Tier;
use crate::export::{Registration, synth_word};
use tcl_dialect::model::WorkspaceTrust;

// Replay

/// The compiled command surface a workspace pack may not shadow: the
/// permissive all-Tcl view, the same registry the collision policy
/// consults.
fn compiled_command_exists(name: &str) -> bool {
    crate::environment::lenient_store().get(name).is_some()
}

/// The first E-R2 violation in a pack's registration record **as if** the
/// pack were untrusted at `tier` — for the workspace tier, as if the editor
/// had not trusted the workspace: the line it was declared on, and a
/// notice-ready message naming the provenance class, in the conditional
/// ("the pack would not be loaded from …"), since the pack it is asked of
/// has loaded.
///
/// It answers the hypothetical, so `tier` supplies the class the message
/// names, and the one thing it does not assume is a trust state the tier
/// cannot have: the bundled and user tiers are trusted whatever the editor
/// says ([`Tier::trust_under`]), the load never refuses them, and so neither
/// does this — `None` there, by the same [`tcl_registry::model::untrusted`]
/// predicate the load's own gate in [`replay`] asks. That is what the caller
/// wants: reading the **record** rather than the evaluator's own staging lets
/// the verdict be asked of a snapshot that has already loaded, which is how
/// an authoring tool (`spectcl_check`, the Spec Studio's
/// `untrusted_tier_refusal`) tells its user "this loads for you, and would
/// be refused from an untrusted workspace" without evaluating the pack a
/// second time.
#[must_use]
pub fn provenance_violation(pack: &Pack, tier: Tier) -> Option<(u32, String)> {
    let hypothetical = super::super::PackEnvironmentTier::of(tier, WorkspaceTrust::Untrusted);
    if !tcl_registry::model::untrusted(hypothetical.provenance()) {
        return None;
    }
    provenance_violation_in(&pack.registrations, hypothetical, Verdict::Previewed)
}

/// Whether an E-R2 message reports the load's own refusal or an authoring
/// tool's preview of one — the same finding, told in the mood that is true
/// where it is read.
#[derive(Debug, Clone, Copy)]
enum Verdict {
    /// The load refused the pack, so it is not loaded.
    Refused,
    /// The pack loaded; an untrusted install of it would not.
    Previewed,
}

impl Verdict {
    /// One E-R2 message: what the row does, where the pack loads from (the
    /// load's own tier, or the tier the preview imagines), and the rule an
    /// untrusted pack may not break.
    pub(super) fn message(self, fact: &str, connective: &str, class: &str, rule: &str) -> String {
        match self {
            Self::Refused => format!(
                "{fact}{connective} this pack loads from the {class} tier; an untrusted pack \
                 may not {rule}, so the pack is not loaded (design E-R2)"
            ),
            Self::Previewed => format!(
                "{fact}; an untrusted pack may not {rule}, so the pack would not be loaded \
                 from the {class} tier (design E-R2)"
            ),
        }
    }
}

fn provenance_violation_in(
    registrations: &[Registration],
    tier: super::super::PackEnvironmentTier,
    verdict: Verdict,
) -> Option<(u32, String)> {
    let class = tier.label();
    for reg in registrations {
        match reg.word() {
            "command" if reg.has_flag("-override") && compiled_command_exists(reg.arg(1)) => {
                return Some((
                    reg.line(),
                    verdict.message(
                        &format!(
                            "command `{}` declares `-override` for a compiled command name",
                            reg.arg(1)
                        ),
                        ", but",
                        class,
                        "shadow compiled family names",
                    ),
                ));
            }
            "dialect" => {
                return Some((
                    reg.line(),
                    verdict.message(
                        &format!("`dialect {}` declares compiled dialect axes", reg.arg(1)),
                        ", but",
                        class,
                        "alter dialect axes",
                    ),
                ));
            }
            "environment" => {
                if let Some(reserved) = environment_block::reserved_name_for(reg.arg(1), tier) {
                    let verb = if reg.has_flag("-extend") {
                        // §6.4: altering a canonical environment — detection
                        // rows and placements included — needs a trusted
                        // tier, exactly as claiming its name does.
                        "extends"
                    } else {
                        "claims"
                    };
                    return Some((
                        reg.line(),
                        verdict.message(
                            &format!(
                                "`environment {}` {verb} `{reserved}`, a compiled \
                                 environment name",
                                reg.arg(1)
                            ),
                            ", and",
                            class,
                            "touch reserved names",
                        ),
                    ));
                }
            }
            _ => {}
        }
    }
    None
}

/// Turn a completed evaluation's staging into a [`Pack`] through the
/// vocabulary's row readers.
pub(super) fn replay(state: State, options: EvalOptions) -> Pack {
    let mut log = Log {
        context: "pack".to_owned(),
        source: Some(std::sync::Arc::from(state.verbatim.source.as_ref())),
        ..Log::default()
    };
    let mut pack = empty_pack();
    pack.target_dependent = state.target_dependent;

    let Some(speclib) = &state.speclib else {
        if state.refused_braced_name {
            pack.notices.extend(state.eval_notices);
            return pack;
        }
        log.say(1, "no `speclib` declaration; nothing loaded");
        pack.notices = log.notices;
        pack.notices.extend(state.eval_notices);
        return pack;
    };

    pack.name.clone_from(&speclib.name);
    pack.dsl_version.clone_from(&speclib.version);
    log.forward_vocabulary = super::super::declared_major(&pack.dsl_version).is_some()
        && !super::super::KNOWN_VOCABULARY_VERSIONS.contains(&pack.dsl_version.as_str())
        && tcl_registry::version::compare(
            &pack.dsl_version,
            super::super::NEWEST_VOCABULARY_VERSION,
        )
        .is_gt();
    if check_vocabulary_version(&pack.dsl_version, speclib.line, &mut log) {
        pack.load_error = Some(LoadError::UnsupportedMajor(pack.dsl_version.clone()));
        pack.notices = log.notices;
        return pack;
    }
    if !speclib.had_body && state.pack_nodes.is_empty() {
        log.say(speclib.line, "`speclib` has no body block");
        pack.notices = log.notices;
        pack.notices.extend(state.eval_notices);
        return pack;
    }

    // The canonical record (design E-R11): what the program registered, in
    // the order it registered it. For a straight-line pack this is the file's
    // own statements — which is what makes `export` a byte-stable round trip
    // there and the *expansion* for a templated pack. Built before the E-R2
    // gate, which reads it, and published on the pack only if the gate
    // passes: a discarded pack registered nothing.
    let registrations = record_nodes(&state.pack_nodes);

    // E-R2: provenance gates what the registrations may touch, and a
    // violation is transactional — the whole pack is discarded. The one
    // `untrusted` predicate (`tcl_registry::model::untrusted`) is the tree's
    // single door onto `Provenance::is_untrusted`; this loader no longer
    // keeps its own copy of the same match (#2139).
    let tier = super::super::PackEnvironmentTier::of(options.tier, options.trust);
    if tcl_registry::model::untrusted(tier.provenance())
        && let Some((line, message)) =
            provenance_violation_in(&registrations, tier, Verdict::Refused)
    {
        let error = LoadError::Provenance(message.clone());
        pack.notices.push(Notice {
            subject: None,
            newer_vocabulary: None,
            context: "pack".to_owned(),
            line,
            class: VocabularyClass::Semantic,
            message,
        });
        pack.load_error = Some(error);
        return pack;
    }

    // Pass 1: pack-level rows, in evaluation order — tables first, since
    // commands are staged separately and built in pass 2.
    let mut tables = PackTables::default();
    for node in &state.pack_nodes {
        match node {
            PackNode::Row(stmt) => {
                let is_command = apply_pack_stmt(&mut pack, &mut tables, stmt, &mut log);
                debug_assert!(!is_command, "a raw `command` row cannot reach pack scope");
            }
            PackNode::Block(staged) => apply_block(&mut pack, staged, &mut log),
            PackNode::Command(_) => {}
        }
    }

    replay_commands(&state.pack_nodes, &tables, &mut pack, &mut log);

    pack.registrations = registrations;

    // Sites the capture layer consumed itself (`include`): logged here so
    // they get the same per-site vocabulary-consistency notices a row the
    // readers saw would.
    for (line, word) in &state.newer_word_sites {
        log.since(*line, word, "2.0");
    }
    super::super::finish_pack_cores(&mut pack, &mut log);
    finish_newer_words(&pack, &mut log);
    pack.notices = log.notices;
    pack.notices.extend(state.eval_notices);
    pack
}

fn replay_commands(nodes: &[PackNode], tables: &PackTables, pack: &mut Pack, log: &mut Log) {
    // Pass 2: the staged commands, through the shared builder.
    for node in nodes {
        let PackNode::Command(staged) = node else {
            continue;
        };
        if staged.name.is_empty() {
            log.say(staged.line, "`command` with no name dropped");
            continue;
        }
        let Some(body) = &staged.body else {
            log.say(
                staged.line,
                format!(
                    "`command {}` has no `{{ … }}` body block; dropped \
                     (an opening brace must be on the same line as the command name)",
                    staged.name
                ),
            );
            continue;
        };
        let built = command_from_parts(
            &staged.name,
            staged.overrides,
            staged.line,
            tables,
            log,
            |spec, acc, log| fill_command_nodes(body, tables, spec, acc, log),
        );
        if let Some(command) = built {
            pack.commands.push(command);
        }
    }
}

/// Replay one evaluated `environment` / `dialect` declaration through its
/// own block reader.
///
/// The reader owns every validation and notice; all this does is hand it
/// the header and the rows the body registered, so a block written with a
/// variable or a loop is read by exactly the code a longhand one is.
fn apply_block(pack: &mut Pack, staged: &StagedBlock, log: &mut Log) {
    let head = Stmt {
        words: staged.head.clone(),
        line: staged.line,
    };
    let rows = staged.body.as_deref().map(row_stmts);
    if staged.word == "environment" {
        super::super::read_environment_rows(pack, &head, rows.as_deref(), log);
    } else {
        super::super::read_dialect_rows(pack, &head, rows.as_deref(), log);
    }
}

/// A block body's staged nodes as row statements.
///
/// Every node is a row: `subcommand` only opens a scope inside a `command`
/// body, so nothing nested can reach here, and a `command` written inside
/// a block is captured as an ordinary row for the block reader to reject.
fn row_stmts(nodes: &[Node]) -> Vec<Stmt> {
    nodes
        .iter()
        .map(|node| match node {
            Node::Row(stmt) => stmt.clone(),
            Node::Sub(sub) => capture_stmt("subcommand", std::slice::from_ref(&sub.name), sub.line),
        })
        .collect()
}

/// The staged top-level nodes as canonical registrations.
fn record_nodes(nodes: &[PackNode]) -> Vec<Registration> {
    nodes
        .iter()
        .map(|node| match node {
            PackNode::Row(stmt) => Registration::row(stmt),
            PackNode::Command(staged) => {
                let mut head = vec![
                    synth_word("command", staged.line),
                    synth_word(&staged.name, staged.line),
                ];
                if staged.overrides {
                    head.push(synth_word("-override", staged.line));
                }
                match &staged.body {
                    // A `command` whose body word never arrived stays a row,
                    // so the reload reports the same missing-block notice.
                    None => Registration::row_words(head, staged.line),
                    Some(body) => Registration::block(head, record_body(body), staged.line),
                }
            }
            PackNode::Block(staged) => match &staged.body {
                // Same rule as `command`: a declaration that never opened a
                // block exports as the row it was, so the reload reports the
                // identical missing-block notice.
                None => Registration::row_words(staged.head.clone(), staged.line),
                Some(body) => {
                    Registration::block(staged.head.clone(), record_body(body), staged.line)
                }
            },
        })
        .collect()
}

/// One command or subcommand body's staged nodes as registrations.
fn record_body(nodes: &[Node]) -> Vec<Registration> {
    nodes
        .iter()
        .map(|node| match node {
            Node::Row(stmt) => Registration::row(stmt),
            Node::Sub(sub) => Registration::block(
                vec![
                    synth_word("subcommand", sub.line),
                    synth_word(&sub.name, sub.line),
                ],
                record_body(&sub.body),
                sub.line,
            ),
        })
        .collect()
}

/// Replay one command body's captured nodes into the shared accumulator.
fn fill_command_nodes(
    nodes: &[Node],
    tables: &PackTables,
    spec: &mut CommandSpec,
    acc: &mut CommandAcc,
    log: &mut Log,
) {
    for node in nodes {
        match node {
            Node::Row(stmt) => apply_command_stmt(spec, acc, stmt, tables, log),
            Node::Sub(sub) => {
                let hooks = &mut acc.hooks;
                let built = subcommand_from_parts(
                    &sub.name,
                    "subcommand",
                    sub.line,
                    log,
                    |built: &mut SubCommand, sacc: &mut SubAcc, log: &mut Log| {
                        for inner in &sub.body {
                            if let Node::Row(stmt) = inner {
                                apply_subcommand_stmt(
                                    built, sacc, stmt, tables, hooks, &sub.name, log,
                                );
                            }
                        }
                    },
                );
                if let Some(subcommand) = built {
                    acc.subcommands.push(subcommand);
                }
            }
        }
    }
}
