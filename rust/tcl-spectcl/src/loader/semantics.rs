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

//! The `semantics`, `evaluate` and `facts` statements — vocabulary 2.2
//! (`docs/design/compiler/value-evaluation.md` § *The `semantics`,
//! `evaluate`, and `facts` rows*).
//!
//! Each is legal at `command`, `subcommand` and `refine` scope, the innermost
//! declaring scope winning, and each has a `-native` form, a block form and
//! the explicit abstention:
//!
//! ```text
//! semantics -native ID | semantics { … } | semantics none
//! evaluate -direct ID | evaluate -expression ID | evaluate -native ID
//!        | evaluate -implementation ID -host HOST { … } | evaluate none
//! facts -native ID | facts { … } | facts none
//! ```
//!
//! A scope's `semantics` and `evaluate` statements make one
//! [`DeclaredSemantics`], leaked onto the scope's `semantics` field; `semantics
//! none` is [`SemanticsDeclaration::Declined`]. A declared implementation's
//! body is a hook of the `evaluate` family, bound at command and subcommand
//! scope as every other hook body is. `facts` loads and is checked, and
//! nothing reads it yet.
//!
//! A `-native ID` is `SCOPE::FIELD`. A short id is a load notice naming the
//! full spelling; a full id looked up in the field's own table
//! (`tcl_registry::pack_hooks`'s `SEMANTICS_NATIVE` / `EVALUATE_NATIVE` /
//! `FACTS_NATIVE`) installs what the table holds, and one the table does not
//! hold is a load notice too. `-direct` and `-expression` are different,
//! already-closed catalogues of their own — `NativeEvalId::ALL` and
//! `LanguageProfileId::ALL` (`tcl.expr`, `bpf.expr`) — not `SCOPE::FIELD` ids.

use std::hash::{Hash as _, Hasher as _};

use tcl_registry::hover::OptionSpec;
use tcl_registry::pack_hooks::{
    EVALUATE_NATIVE, FACTS_NATIVE, HookInput, HookInputs, SEMANTICS_NATIVE,
};
use tcl_registry::types::TclType;
use tcl_registry::value_transfer::{
    BindingIdentity, CompletionSupport, ContextDependency, DeclaredEffect, DeclaredEvaluation,
    DeclaredImplementation, DeclaredInput, DeclaredIteration, DeclaredSemantics, DeclaredStores,
    DeclaredStructure, DeclineReason, EvalRoute, EvaluatorCapability, Exactness, HostKind,
    ImplementationBudget, ImplementationIdentity, IterableWord, LanguageProfileId, NativeEvalId,
    Needs, NoRouteReason, OptionEvaluation, OutcomeKind, SemanticType, SemanticsDeclaration,
};

use super::{
    HookDecl, HookFamily, HookOwner, HookSource, Log, Stmt, Word, block, enum_by_name, leak_one,
    leak_slice, leak_str, list_words, next_text,
};

/// The vocabulary the three statements and the two option flags arrived in.
pub(super) const VOCABULARY: &str = "2.2";

/// The hook field an `evaluate -implementation` body fills.
pub(super) const EVALUATE_FIELD: &str = "evaluate";

/// Where a scope's statements are read.
pub(super) struct Scope<'a> {
    /// The id rule's `SCOPE`: the command, `command::subcommand`, or the
    /// form's owner followed by `::FORM`.
    pub(super) path: &'a str,
    /// Whether a hook body binds here: at command and subcommand scope, as
    /// every hook body does, and not on a form.
    pub(super) binds_bodies: bool,
}

/// One scope's `semantics` and `evaluate` statements, as read so far. The
/// later of two statements of one kind replaces the earlier, with a notice.
#[derive(Default)]
pub(super) struct Declarations {
    structure: Option<Structure>,
    evaluation: Option<Evaluation>,
    /// The scope's option rows that switch evaluation off, with the line of
    /// the first, which a scope with no statement of its own reports.
    option_declines: Vec<(&'static str, DeclineReason)>,
    option_line: Option<u32>,
}

/// What the scope's `semantics` statement said.
enum Structure {
    /// `semantics none`.
    Abstain,
    /// `semantics { … }`.
    Block(DeclaredStructure),
}

/// What the scope's `evaluate` statement said.
enum Evaluation {
    /// A route named outright.
    Route(EvalRoute),
    /// A declared implementation and its body.
    Implementation(Implementation),
}

struct Implementation {
    capability: EvaluatorCapability,
    params: Vec<String>,
    body: String,
}

impl Declarations {
    /// Read `stmt` when it is one of the three statements, reporting what it
    /// could not use. `false` for any other statement.
    pub(super) fn read(&mut self, stmt: &Stmt, scope: &Scope<'_>, log: &mut Log) -> bool {
        let keyword = stmt.word_text(0);
        match keyword {
            "semantics" => {
                log.since(stmt.line, keyword, VOCABULARY);
                if let Some(structure) = read_semantics(stmt, scope, log) {
                    if self.structure.is_some() {
                        log.say(stmt.line, "a second `semantics` replaces the first");
                    }
                    self.structure = Some(structure);
                }
                true
            }
            "evaluate" => {
                log.since(stmt.line, keyword, VOCABULARY);
                if let Some(evaluation) = read_evaluate(stmt, scope, log) {
                    if self.evaluation.is_some() {
                        log.say(stmt.line, "a second `evaluate` replaces the first");
                    }
                    self.evaluation = Some(evaluation);
                }
                true
            }
            "facts" => {
                log.since(stmt.line, keyword, VOCABULARY);
                read_facts(stmt, scope, log);
                true
            }
            _ => false,
        }
    }

    /// Whether the scope has said anything a declaration is made of.
    pub(super) fn is_empty(&self) -> bool {
        self.structure.is_none() && self.evaluation.is_none()
    }

    /// Record an option row's `-evaluate` flags as the decline the driver
    /// records when the option is present. `release_ambiguous` names the
    /// option's own availability axis, so on an option that declares no
    /// availability it is a notice and the row records a bare
    /// `-evaluate none`.
    pub(super) fn decline_option(
        &mut self,
        option: &OptionSpec,
        evaluation: OptionEvaluation,
        line: u32,
        log: &mut Log,
    ) {
        let surface = option
            .surface
            .and_then(|surfaces| surfaces.first().copied());
        let reason = evaluation.decline(surface).unwrap_or_else(|| {
            log.say(
                line,
                format!(
                    "`-evaluate-reason release_ambiguous` names the availability of `{}`, which \
                     declares none; recorded as `-evaluate none`",
                    option.name
                ),
            );
            DeclineReason::NoRoute(NoRouteReason::Declared)
        });
        self.option_declines.push((leak_str(option.name), reason));
        self.option_line.get_or_insert(line);
    }

    /// Report option flags on a scope that declares no route of its own:
    /// a flag switches a route off, so without one here it has nothing to
    /// act on, and it is dropped rather than read as `evaluate none`.
    pub(super) fn report_orphan_option_flags(&self, log: &mut Log) {
        if let Some(line) = self.option_line
            && self.is_empty()
        {
            log.say(
                line,
                "an option's `-evaluate` flag needs a `semantics` or `evaluate` statement at \
                 its scope; dropped",
            );
        }
    }

    /// The scope's declaration, and the implementation body to bind as its
    /// `evaluate` hook when it has one.
    pub(super) fn declaration(
        &self,
        scope: &Scope<'_>,
    ) -> (SemanticsDeclaration, Option<HookSource>) {
        if self.is_empty() {
            return (SemanticsDeclaration::Inherited, None);
        }
        // `semantics none` abstains for the whole scope: the enclosing
        // scope's declaration stops applying, and a route stated beside it
        // has no plan to run under.
        if matches!(self.structure, Some(Structure::Abstain)) {
            return (SemanticsDeclaration::Declined, None);
        }
        let structure = match &self.structure {
            Some(Structure::Block(structure)) => *structure,
            Some(Structure::Abstain) | None => DeclaredStructure::default(),
        };
        let (evaluation, body) = match &self.evaluation {
            None => (tcl_registry::value_transfer::declared::UNAUTHORED, None),
            Some(Evaluation::Route(route)) => (DeclaredEvaluation::Route(*route), None),
            Some(Evaluation::Implementation(implementation)) => (
                DeclaredEvaluation::Implementation(DeclaredImplementation {
                    capability: implementation.capability,
                    slot: None,
                }),
                Some(HookSource::Body {
                    params: implementation.params.clone(),
                    body: implementation.body.clone(),
                    // The body reads its declared inputs, which arrive as
                    // the call's words: content, so the answer is keyed by
                    // them.
                    inputs: HookInputs::declared([HookInput::Words]),
                }),
            ),
        };
        let declared: &'static DeclaredSemantics = leak_one(DeclaredSemantics {
            scope: leak_str(scope.path),
            structure,
            evaluation,
            option_declines: leak_slice(self.option_declines.clone()),
        });
        (SemanticsDeclaration::Declared(declared), body)
    }
}

/// Take an option row's `-evaluate none` and `-evaluate-reason WORD` flags
/// out of `stmt`, returning the row the option reader reads and what the
/// flags state: a bare `-evaluate none` records `NoRoute(Declared)`, and a
/// reason word names the decline.
pub(super) fn option_flags(stmt: &Stmt, log: &mut Log) -> (Stmt, Option<OptionEvaluation>) {
    let mut kept: Vec<Word> = Vec::with_capacity(stmt.words.len());
    let mut evaluate_none = false;
    let mut reason: Option<OptionEvaluation> = None;
    let mut i = 0;
    while i < stmt.words.len() {
        let word = &stmt.words[i];
        match word.text.as_str() {
            "-evaluate" if i >= 2 => {
                log.since(stmt.line, "-evaluate", VOCABULARY);
                let value = next_text(&stmt.words, &mut i);
                if value == "none" {
                    evaluate_none = true;
                } else {
                    log.say(
                        stmt.line,
                        format!("`-evaluate` takes only `none`, not `{value}`"),
                    );
                }
            }
            "-evaluate-reason" if i >= 2 => {
                log.since(stmt.line, "-evaluate-reason", VOCABULARY);
                let value = next_text(&stmt.words, &mut i);
                reason = OptionEvaluation::REASONS
                    .iter()
                    .find(|(word, _)| *word == value)
                    .map(|(_, evaluation)| *evaluation);
                if reason.is_none() {
                    log.say(
                        stmt.line,
                        format!(
                            "unknown `-evaluate-reason` `{value}`: `form_unsupported`, \
                             `callback` or `release_ambiguous`"
                        ),
                    );
                }
            }
            _ => kept.push(word.clone()),
        }
        i += 1;
    }
    if reason.is_some() && !evaluate_none {
        log.say(
            stmt.line,
            "`-evaluate-reason` names the decline of an `-evaluate none`; dropped without one",
        );
    }
    let evaluation = evaluate_none.then(|| reason.unwrap_or(OptionEvaluation::DECLARED));
    (
        Stmt {
            words: kept,
            line: stmt.line,
        },
        evaluation,
    )
}

/// Replace `owner`'s `evaluate` hook in `hooks` with `body`'s, so the hook
/// list always holds the body of the scope's latest `evaluate` statement.
pub(super) fn rebind(hooks: &mut Vec<HookDecl>, owner: &HookOwner, body: Option<HookSource>) {
    hooks.retain(|hook| !(hook.field == EVALUATE_FIELD && hook.owner == *owner));
    if let Some(source) = body {
        hooks.push(HookDecl {
            owner: owner.clone(),
            field: EVALUATE_FIELD,
            family: HookFamily::Evaluate,
            source,
        });
    }
}

/// `FIELD -native ID`: the id rule. A short id — one that does not spell
/// `SCOPE::FIELD` — is a notice naming the full spelling; a full id `table`
/// does not hold is a notice too, and either way nothing installs beyond
/// what `table` itself names.
fn native_id<T: Copy>(
    stmt: &Stmt,
    field: &str,
    flag: &str,
    scope: &Scope<'_>,
    table: &[(&'static str, T)],
    log: &mut Log,
) -> Option<T> {
    let id = stmt.word_text(2);
    let full = format!("{}::{field}", scope.path);
    if id.is_empty() {
        log.say(stmt.line, format!("`{field} {flag}` needs an id: `{full}`"));
        return None;
    }
    if id != full {
        log.say(
            stmt.line,
            format!(
                "`{field} {flag} {id}` is not this scope's id; spell it `{full}` — \
                 the statement installs nothing"
            ),
        );
        return None;
    }
    let found = table
        .iter()
        .find(|(key, _)| *key == full)
        .map(|(_, value)| *value);
    if found.is_none() {
        log.say(
            stmt.line,
            format!(
                "`{field} {flag} {id}` names nothing this build ships; the statement installs \
                 nothing"
            ),
        );
    }
    found
}

fn read_semantics(stmt: &Stmt, scope: &Scope<'_>, log: &mut Log) -> Option<Structure> {
    match (stmt.words.len(), stmt.word_text(1)) {
        (2, "none") if !stmt.words[1].braced => Some(Structure::Abstain),
        (3, "-native") => native_id(stmt, "semantics", "-native", scope, SEMANTICS_NATIVE, log)
            .map(Structure::Block),
        (2, _) if stmt.words[1].braced => {
            let structure = structure_block(&stmt.words[1], log)?;
            Some(Structure::Block(structure))
        }
        _ => {
            log.say(
                stmt.line,
                "`semantics` takes `-native ID`, a `{ … }` block, or `none`; dropped",
            );
            None
        }
    }
}

/// The rows of a `semantics { … }` block. A declaration whose rows
/// contradict each other is dropped whole, with the reason: validation
/// catches contradictory metadata, and the generic behaviour stays.
fn structure_block(word: &Word, log: &mut Log) -> Option<DeclaredStructure> {
    let mut structure = DeclaredStructure::default();
    let mut effects: Vec<DeclaredEffect> = Vec::new();
    for row in block(word) {
        match row.word_text(0) {
            "effects" => {
                for name in list_words(row.word_text(1)) {
                    match DeclaredEffect::ALL.iter().find(|e| e.as_str() == name) {
                        Some(effect) if !effects.contains(effect) => effects.push(*effect),
                        Some(_) => {}
                        None => log.say(row.line, format!("unknown effect `{name}` dropped")),
                    }
                }
            }
            "result" => match (row.word_text(1), row.words.len()) {
                ("-semantic", 3) => {
                    structure.result = semantic_type(row.word_text(2), row.line, log);
                }
                _ => log.say(row.line, "`result` takes `-semantic TYPE`; dropped"),
            },
            "stores" => structure.stores = stores_row(&row, log),
            "iterate" => match row.words.get(1) {
                Some(body) if row.words.len() == 2 => {
                    structure.iterate = iterate_block(body, log);
                }
                _ => log.say(row.line, "`iterate` takes a `{ … }` block; dropped"),
            },
            _ => log.unknown_property(&row),
        }
    }
    if effects.contains(&DeclaredEffect::NoStoreWrites) && structure.stores.is_some() {
        log.say(
            word.line,
            "a declared `no_store_writes` effect cannot coexist with a `stores` row; \
             the `semantics` block is dropped",
        );
        return None;
    }
    structure.effects = leak_slice(effects);
    Some(structure)
}

/// `-semantic T`: a Tcl type in lower case, or `vendor.NAME`.
fn semantic_type(name: &str, line: u32, log: &mut Log) -> Option<SemanticType> {
    if let Some(rest) = name.strip_prefix("vendor.")
        && !rest.is_empty()
    {
        return Some(SemanticType::Vendor(leak_str(name)));
    }
    let found = TclType::ALL
        .iter()
        .find(|ty| format!("{ty:?}").eq_ignore_ascii_case(name))
        .copied();
    if found.is_none() {
        log.say(line, format!("unknown semantic type `{name}` dropped"));
    }
    found.map(SemanticType::Tcl)
}

/// `stores -targets {N …} -outcome O`.
fn stores_row(row: &Stmt, log: &mut Log) -> Option<DeclaredStores> {
    let mut targets: Option<Vec<usize>> = None;
    let mut outcome: Option<OutcomeKind> = None;
    let mut i = 1;
    while i < row.words.len() {
        match row.words[i].text.as_str() {
            "-targets" => {
                let words = list_words(&next_text(&row.words, &mut i));
                let parsed: Option<Vec<usize>> = words
                    .iter()
                    .map(|word| word.parse::<usize>().ok())
                    .collect();
                match parsed {
                    Some(indices) if !indices.is_empty() => targets = Some(indices),
                    _ => log.say(
                        row.line,
                        "`-targets` takes a list of argument indices; the row is dropped",
                    ),
                }
            }
            "-outcome" => {
                let word = next_text(&row.words, &mut i);
                outcome = OutcomeKind::ALL
                    .iter()
                    .find(|o| o.as_str() == word)
                    .copied();
                if outcome.is_none() {
                    log.say(
                        row.line,
                        format!("unknown outcome `{word}`; the row is dropped"),
                    );
                }
            }
            other => log.unknown_flag("stores", row.line, other),
        }
        i += 1;
    }
    let (Some(targets), Some(outcome)) = (targets, outcome) else {
        log.say(
            row.line,
            "`stores` needs `-targets {…}` and `-outcome O`; dropped",
        );
        return None;
    };
    Some(DeclaredStores {
        targets: leak_slice(targets),
        outcome,
    })
}

/// An `iterate { … }` block: `binder`, `iterable` and `body` rows name
/// argument indices, and `yield`, `cardinality`, `completion` and
/// `zero_iterations` refine the plan.
fn iterate_block(word: &Word, log: &mut Log) -> Option<DeclaredIteration> {
    let mut binder = None;
    let mut iterable = None;
    let mut kind = IterableWord::List;
    let mut body = None;
    let mut yields = None;
    let mut cardinality = None;
    let mut zero_iterations_bind = false;
    for row in block(word) {
        let key = row.word_text(0).to_owned();
        let mut i = 1;
        while i < row.words.len() {
            let flag = row.words[i].text.clone();
            match (key.as_str(), flag.as_str()) {
                ("binder" | "iterable" | "body" | "cardinality", "-arg") => {
                    let text = next_text(&row.words, &mut i);
                    let Ok(index) = text.parse::<usize>() else {
                        log.say(
                            row.line,
                            format!("`{key} -arg` takes an index, not `{text}`"),
                        );
                        i += 1;
                        continue;
                    };
                    match key.as_str() {
                        "binder" => binder = Some(index),
                        "iterable" => iterable = Some(index),
                        "body" => body = Some(index),
                        _ => cardinality = Some(index),
                    }
                }
                ("binder", "-grammar")
                | ("body", "-scope")
                | ("cardinality", "-from")
                | ("completion", "-contract") => {
                    // Named for the reader: one binder, the enclosing frame,
                    // a summary or contract the analyser does not read yet.
                    let _ = next_text(&row.words, &mut i);
                }
                ("iterable", "-kind") => {
                    let text = next_text(&row.words, &mut i);
                    kind = match text.as_str() {
                        "list" => IterableWord::List,
                        "dict" => IterableWord::Dict,
                        vendor if vendor.starts_with("vendor.") => {
                            IterableWord::Vendor(leak_str(vendor))
                        }
                        other => {
                            log.say(row.line, format!("unknown iterable kind `{other}` dropped"));
                            kind
                        }
                    };
                }
                ("yield", "-semantic") => {
                    let text = next_text(&row.words, &mut i);
                    yields = semantic_type(&text, row.line, log);
                }
                ("zero_iterations", "-bindings") => match next_text(&row.words, &mut i).as_str() {
                    "preserve" => zero_iterations_bind = false,
                    "bind" => zero_iterations_bind = true,
                    other => log.say(
                        row.line,
                        format!(
                            "`zero_iterations -bindings` takes `preserve` or `bind`, not `{other}`"
                        ),
                    ),
                },
                _ => log.unknown_flag(&key, row.line, &flag),
            }
            i += 1;
        }
        if !matches!(
            key.as_str(),
            "binder"
                | "iterable"
                | "body"
                | "yield"
                | "cardinality"
                | "completion"
                | "zero_iterations"
        ) {
            log.unknown_property(&row);
        }
    }
    let (Some(binder), Some(iterable)) = (binder, iterable) else {
        log.say(
            word.line,
            "`iterate` needs a `binder -arg N` and an `iterable -arg N`; dropped",
        );
        return None;
    };
    Some(DeclaredIteration {
        binder,
        iterable,
        kind,
        body,
        yields,
        cardinality,
        zero_iterations_bind,
    })
}

fn read_evaluate(stmt: &Stmt, scope: &Scope<'_>, log: &mut Log) -> Option<Evaluation> {
    match (stmt.words.len(), stmt.word_text(1)) {
        (2, "none") => Some(Evaluation::Route(EvalRoute::None {
            reason: NoRouteReason::Declared,
        })),
        (3, "-native") => native_id(stmt, "evaluate", "-native", scope, EVALUATE_NATIVE, log)
            .map(Evaluation::Route),
        (3, "-direct") => {
            let id = stmt.word_text(2);
            enum_by_name(NativeEvalId::ALL, id, "direct evaluator", stmt.line, log)
                .map(|id| Evaluation::Route(EvalRoute::Direct { id }))
        }
        (3, "-expression") => {
            let id = stmt.word_text(2);
            let language = LanguageProfileId::ALL
                .iter()
                .find(|language| language.as_str() == id)
                .copied();
            if language.is_none() {
                log.say(
                    stmt.line,
                    format!("`evaluate -expression {id}` names no language profile; dropped"),
                );
            }
            language.map(|language| Evaluation::Route(EvalRoute::Expression { language }))
        }
        // A form's hook bodies are not bound, so its implementation cannot
        // run; the form then declares no route rather than inherit an
        // evaluator written for another shape.
        (6, "-implementation") if !scope.binds_bodies => {
            log.say(
                stmt.line,
                "a form's declared implementation is dropped: implementation bodies bind at \
                 command and subcommand scope, as every hook body does; the form declares no \
                 route",
            );
            Some(Evaluation::Route(EvalRoute::None {
                reason: NoRouteReason::Unauthored,
            }))
        }
        (6, "-implementation") => read_implementation(stmt, log).map(Evaluation::Implementation),
        _ => {
            log.say(
                stmt.line,
                "`evaluate` takes `none`, `-direct ID`, `-expression ID`, `-native ID`, or \
                 `-implementation ID -host HOST { … }`; dropped",
            );
            None
        }
    }
}

/// `evaluate -implementation ID -host HOST { inputs … depends … budget … body
/// {params} {…} }`, whose four rows are the only ones legal inside.
fn read_implementation(stmt: &Stmt, log: &mut Log) -> Option<Implementation> {
    let id = stmt.word_text(2);
    if stmt.word_text(3) != "-host" || id.is_empty() {
        log.say(
            stmt.line,
            "`evaluate -implementation` takes `ID -host HOST { … }`; dropped",
        );
        return None;
    }
    let host_word = stmt.word_text(4);
    let Some(host) = HostKind::ALL
        .iter()
        .find(|host| host.as_str() == host_word)
        .copied()
    else {
        log.say(
            stmt.line,
            format!("unknown host `{host_word}`: `bounded_tcl` is the only host; dropped"),
        );
        return None;
    };
    let mut inputs: Vec<DeclaredInput> = Vec::new();
    let mut depends: Vec<ContextDependency> = Vec::new();
    let mut budget = ImplementationBudget::default();
    let mut body: Option<(Vec<String>, String)> = None;
    for row in block(&stmt.words[5]) {
        match row.word_text(0) {
            "inputs" => inputs = inputs_row(&row, log),
            "depends" => depends = depends_row(&row, log),
            "budget" => budget = budget_row(&row, log),
            "body" if row.words.len() == 3 => {
                body = Some((list_words(row.word_text(1)), row.word_text(2).to_owned()));
            }
            "body" => log.say(row.line, "`body` takes `{params} { … }`; dropped"),
            _ => log.unknown_property(&row),
        }
    }
    let Some((params, text)) = body else {
        log.say(
            stmt.line,
            format!("`evaluate -implementation {id}` has no `body`; it installs nothing"),
        );
        return None;
    };
    if params.len() != inputs.len() {
        log.say(
            stmt.line,
            format!(
                "the body takes {} parameter(s) for {} declared input(s); every call would \
                 fail, so it installs nothing",
                params.len(),
                inputs.len()
            ),
        );
        return None;
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    params.hash(&mut hasher);
    text.hash(&mut hasher);
    Some(Implementation {
        capability: EvaluatorCapability {
            identity: ImplementationIdentity {
                // The pack names itself when the host plan binds the body.
                pack: "",
                id: leak_str(id),
                content_hash: hasher.finish(),
            },
            host,
            // The body runs pinned to the call's release, so no axis needs
            // admitting here; a profile naming no release is declined by
            // the host.
            target: Needs::NONE,
            inputs: leak_slice(inputs),
            depends: leak_slice(depends),
            budget,
            completion: CompletionSupport::NormalOnly,
        },
        params,
        body: text,
    })
}

/// `inputs {arg N exact target N incoming option -NAME exact …}`, read as
/// three-word entries.
fn inputs_row(row: &Stmt, log: &mut Log) -> Vec<DeclaredInput> {
    let words = list_words(row.word_text(1));
    let mut inputs = Vec::new();
    for entry in words.chunks(3) {
        let parsed = match entry {
            [kind, index, mode] if kind == "arg" && mode == Exactness::Exact.as_str() => {
                index.parse().ok().map(|index| DeclaredInput::Operand {
                    index,
                    exactness: Exactness::Exact,
                })
            }
            [kind, index, mode] if kind == "target" && mode == "incoming" => index
                .parse()
                .ok()
                .map(|index| DeclaredInput::IncomingTarget { index }),
            [kind, name, mode] if kind == "option" && mode == "exact" && name.starts_with('-') => {
                Some(DeclaredInput::OptionValue {
                    name: leak_str(name),
                })
            }
            _ => None,
        };
        match parsed {
            Some(input) => inputs.push(input),
            None => log.say(
                row.line,
                format!(
                    "`inputs` entry `{}` is not `arg N exact`, `target N incoming` or \
                     `option -NAME exact`; dropped",
                    entry.join(" ")
                ),
            ),
        }
    }
    inputs
}

/// `depends {tcl_profile implementation_identity registry_generation
/// evaluator_generation binding NAME …}`, in declaration order.
fn depends_row(row: &Stmt, log: &mut Log) -> Vec<ContextDependency> {
    let words = list_words(row.word_text(1));
    let mut depends = Vec::new();
    let mut words = words.iter();
    while let Some(word) = words.next() {
        if word == "binding" {
            match words.next() {
                Some(name) => depends.push(ContextDependency::Binding(BindingIdentity {
                    resolution_namespace: String::new(),
                    name: name.clone(),
                    identity: name.trim_start_matches("::").to_owned(),
                })),
                None => log.say(row.line, "`binding` needs a command name; dropped"),
            }
            continue;
        }
        match ContextDependency::WORDS.iter().find(|d| d.as_str() == word) {
            Some(dependency) if !depends.contains(dependency) => depends.push(dependency.clone()),
            Some(_) => {}
            None => log.say(row.line, format!("unknown dependency `{word}` dropped")),
        }
    }
    depends
}

/// `budget {-commands N -wall-clock MS -value-bytes N}`. A value above the
/// host's is a notice, and the host's stands: a declaration narrows the
/// host's budget, never widens it.
fn budget_row(row: &Stmt, log: &mut Log) -> ImplementationBudget {
    let host = tcl_spec_hooks::HostConfig::default().budget;
    let host_wall_clock = host
        .wall_clock
        .map(|limit| u64::try_from(limit.as_millis()).unwrap_or(u64::MAX));
    let words = list_words(row.word_text(1));
    let mut budget = ImplementationBudget::default();
    for pair in words.chunks(2) {
        let [flag, value] = pair else {
            log.say(
                row.line,
                format!("`budget` flag `{}` has no value; dropped", pair[0]),
            );
            continue;
        };
        let Ok(value) = value.parse::<u64>() else {
            log.say(
                row.line,
                format!("`budget {flag}` takes a count, not `{value}`"),
            );
            continue;
        };
        let (slot, ceiling) = match flag.as_str() {
            "-commands" => (&mut budget.commands, host.commands),
            "-wall-clock" => (&mut budget.wall_clock_ms, host_wall_clock),
            "-value-bytes" => (&mut budget.value_bytes, host.max_value_bytes),
            other => {
                log.unknown_flag("budget", row.line, other);
                continue;
            }
        };
        if ceiling.is_some_and(|ceiling| value > ceiling) {
            log.say(
                row.line,
                format!(
                    "`budget {flag} {value}` is above the host's {}; the host's stands",
                    ceiling.unwrap_or_default()
                ),
            );
            continue;
        }
        *slot = Some(value);
    }
    budget
}

/// `facts -native ID | facts { … } | facts none`, checked and carried by the
/// registration record; no solver reads a pack's facts yet.
fn read_facts(stmt: &Stmt, scope: &Scope<'_>, log: &mut Log) {
    match (stmt.words.len(), stmt.word_text(1)) {
        (2, "none") if !stmt.words[1].braced => {}
        (3, "-native") => {
            let _ = native_id(stmt, "facts", "-native", scope, FACTS_NATIVE, log);
        }
        (2, _) if stmt.words[1].braced => {
            for row in block(&stmt.words[1]) {
                let known = matches!(
                    (row.word_text(0), row.word_text(1)),
                    ("result", "-representation" | "-string_segments")
                        | ("taint", "-result_from")
                        | ("range", "-integer_add")
                );
                if !known {
                    log.unknown_property(&row);
                }
            }
        }
        _ => log.say(
            stmt.line,
            "`facts` takes `-native ID`, a `{ … }` block, or `none`; dropped",
        ),
    }
}

#[cfg(test)]
mod tests {
    use tcl_dialect::model::{Family, SpecProvider};
    use tcl_registry::value_transfer::{
        Axis, CommandSemantics, ContextDependency, DeclaredEffect, DeclaredEvaluation,
        DeclaredInput, DeclaredSemantics, DeclineReason, EvalRoute, Exactness, HostKind,
        ImplementationBudget, IterableWord, LanguageProfileId, NoRouteReason, OutcomeKind,
        SemanticType, SemanticsDeclaration,
    };

    use super::super::{HookOwner, HookSource, Pack, evaluate_pack};
    use super::{EVALUATE_FIELD, HookFamily, Log, Scope, Stmt, Word, native_id};

    const TENANT: &str = r#"
speclib probe 2.2 {
    command tenant::label {
        arity 1
        semantics {
            effects {no_store_writes no_external_io}
            result -semantic string
        }
        evaluate -implementation tenant.label.v1 -host bounded_tcl {
            inputs {arg 0 exact}
            depends {tcl_profile implementation_identity}
            budget {-commands 2000 -wall-clock 20 -value-bytes 65536}
            body {name} { fold [string cat "tenant:" $name] }
        }
        facts {
            result -string_segments {{constant "tenant:"} {operand 0}}
            taint  -result_from {arg 0}
        }
    }
    command tenant {
        arity 1..
        subcommand label {
            arity 1
            evaluate -implementation tenant.label.v1 -host bounded_tcl {
                inputs {arg 0 exact}
                depends {tcl_profile}
                body {name} { fold [string cat "tenant:" $name] }
            }
        }
        subcommand quiet {
            evaluate -implementation tenant.quiet.v1 -host bounded_tcl {
                inputs {arg 0 exact}
                body {name} { fold $name }
            }
            semantics none
        }
        subcommand plain {
            evaluate none
        }
        refine lookup {
            selector {lookup}
            evaluate -expression tcl.expr
        }
    }
    command kv::split3 {
        arity 4
        semantics {
            stores -targets {1 2 3} -outcome write_or_preserve
            result -semantic int
        }
        option -strict -evaluate none -evaluate-reason form_unsupported
        option -legacy -available {tcl 8.4-8.6} -evaluate none -evaluate-reason release_ambiguous
    }
    command collection::each {
        arity 3
        semantics {
            iterate {
                binder -arg 0 -grammar vendor.single_variable
                iterable -arg 1 -kind vendor.collection
                body -arg 2 -scope enclosing
                yield -semantic vendor.object_handle
                cardinality -from vendor.collection_summary
                completion -contract vendor.collection_loop_completion
                zero_iterations -bindings preserve
            }
        }
        evaluate none
    }
}
"#;

    fn declared(declaration: SemanticsDeclaration) -> &'static DeclaredSemantics {
        let SemanticsDeclaration::Declared(semantics) = declaration else {
            panic!("a declaration: {declaration:?}");
        };
        semantics.as_declared().expect("a pack declaration")
    }

    fn evaluate_hooks<'p>(pack: &'p Pack, command: &str) -> Vec<(&'p HookOwner, &'p HookSource)> {
        pack.command(command)
            .expect("the command loads")
            .hooks
            .iter()
            .filter(|hook| hook.field == EVALUATE_FIELD)
            .inspect(|hook| assert_eq!(hook.family, HookFamily::Evaluate))
            .map(|hook| (&hook.owner, &hook.source))
            .collect()
    }

    /// The three statements load at command, subcommand and form scope, the
    /// innermost declaring scope winning: a command's structure, its
    /// declared implementation's capability and its body; a subcommand's own
    /// implementation, its abstention (`semantics none` beside an
    /// implementation declines, and its body is not bound) and `evaluate
    /// none`; and a form's expression route. `facts` loads and installs
    /// nothing.
    #[test]
    fn semantics_statements_load_at_every_scope() {
        let pack = evaluate_pack(TENANT);
        assert!(pack.notices.is_empty(), "{:#?}", pack.notices);

        let label = declared(pack.command("tenant::label").unwrap().spec.semantics);
        assert_eq!(label.identity(), "tenant::label");
        assert_eq!(
            label.structure.effects,
            [DeclaredEffect::NoStoreWrites, DeclaredEffect::NoExternalIo]
        );
        assert_eq!(
            label.structure.result,
            Some(SemanticType::Tcl(tcl_registry::types::TclType::String))
        );
        let DeclaredEvaluation::Implementation(implementation) = label.evaluation else {
            panic!("an implementation: {:?}", label.evaluation);
        };
        let capability = implementation.capability;
        assert_eq!(capability.identity.id, "tenant.label.v1");
        assert_ne!(capability.identity.content_hash, 0);
        assert_eq!(capability.host, HostKind::BoundedTcl);
        assert_eq!(
            capability.inputs,
            [DeclaredInput::Operand {
                index: 0,
                exactness: Exactness::Exact
            }]
        );
        assert_eq!(
            capability.depends,
            [
                ContextDependency::TclProfile,
                ContextDependency::ImplementationIdentity
            ]
        );
        assert_eq!(
            capability.budget,
            ImplementationBudget {
                commands: Some(2000),
                wall_clock_ms: Some(20),
                value_bytes: Some(65536),
            }
        );
        assert_eq!(implementation.slot, None, "bound by the host plan");
        assert_eq!(label.route(), EvalRoute::Implementation(capability));
        let [(owner, HookSource::Body { params, body, .. })] =
            evaluate_hooks(&pack, "tenant::label")[..]
        else {
            panic!("one evaluate body");
        };
        assert_eq!(*owner, HookOwner::Command);
        assert_eq!(params, &["name"]);
        assert!(body.contains("string cat"), "{body}");

        let tenant = pack.command("tenant").unwrap().spec;
        assert!(matches!(tenant.semantics, SemanticsDeclaration::Inherited));
        let sub = |name: &str| {
            tenant
                .subcommands
                .iter()
                .find(|sub| sub.name == name)
                .expect("the subcommand loads")
        };
        let sub_label = declared(sub("label").semantics);
        assert_eq!(sub_label.identity(), "tenant::label");
        assert!(matches!(
            sub_label.evaluation,
            DeclaredEvaluation::Implementation(_)
        ));
        assert!(matches!(
            sub("quiet").semantics,
            SemanticsDeclaration::Declined
        ));
        assert_eq!(
            declared(sub("plain").semantics).route(),
            EvalRoute::None {
                reason: NoRouteReason::Declared
            }
        );
        let owners: Vec<&HookOwner> = evaluate_hooks(&pack, "tenant")
            .into_iter()
            .map(|(owner, _)| owner)
            .collect();
        assert_eq!(
            owners,
            [&HookOwner::Subcommand("label".to_owned())],
            "`quiet` abstains, so its body binds nowhere"
        );
        let lookup = tenant
            .command_forms
            .iter()
            .find(|form| form.name == "lookup")
            .expect("the form loads");
        let form = declared(lookup.semantics);
        assert_eq!(form.identity(), "tenant::lookup");
        assert_eq!(
            form.route(),
            EvalRoute::Expression {
                language: LanguageProfileId::TclExpr
            }
        );
    }

    /// A `stores` row loads with its targets and outcome beside the option
    /// flags that switch the route off — `release_ambiguous` on the option's
    /// own availability axis — and an `iterate` block loads as a vendor
    /// loop.
    #[test]
    fn a_stores_row_an_iterate_block_and_the_option_flags_load() {
        let pack = evaluate_pack(TENANT);
        assert!(pack.notices.is_empty(), "{:#?}", pack.notices);

        let split = declared(pack.command("kv::split3").unwrap().spec.semantics);
        let stores = split.structure.stores.expect("a stores row");
        assert_eq!(stores.targets, [1, 2, 3]);
        assert_eq!(stores.outcome, OutcomeKind::WriteOrPreserve);
        let [strict, legacy] = split.option_declines else {
            panic!("two option declines: {:?}", split.option_declines);
        };
        assert_eq!(
            *strict,
            (
                "-strict",
                DeclineReason::NoRoute(NoRouteReason::FormUnsupported)
            )
        );
        assert_eq!(legacy.0, "-legacy");
        assert!(
            matches!(
                legacy.1,
                DeclineReason::ReleaseAmbiguous(Axis::Availability(surface))
                    if surface.provider == SpecProvider::Core(Family::Tcl)
            ),
            "the option's own availability axis: {:?}",
            legacy.1
        );
        assert_eq!(
            split.route(),
            EvalRoute::None {
                reason: NoRouteReason::Unauthored
            }
        );

        let each = declared(pack.command("collection::each").unwrap().spec.semantics);
        let iteration = each.structure.iterate.expect("an iterate block");
        assert_eq!((iteration.binder, iteration.iterable), (0, 1));
        assert_eq!(iteration.body, Some(2));
        assert_eq!(iteration.kind, IterableWord::Vendor("vendor.collection"));
        assert_eq!(
            iteration.yields,
            Some(SemanticType::Vendor("vendor.object_handle"))
        );
        assert!(!iteration.zero_iterations_bind);
    }

    /// The statements are vocabulary 2.2: a pack declaring 2.1 that uses
    /// one draws the per-site notice, and still loads it.
    #[test]
    fn the_statements_are_vocabulary_2_2() {
        let pack = evaluate_pack(&TENANT.replacen("probe 2.2", "probe 2.1", 1));
        assert!(
            pack.notices.iter().any(|notice| notice
                .message
                .contains("`evaluate` is SpecTcl 2.2 vocabulary")),
            "{:#?}",
            pack.notices
        );
        assert!(matches!(
            pack.command("tenant::label").unwrap().spec.semantics,
            SemanticsDeclaration::Declared(_)
        ));
    }

    /// `-native ID` is `SCOPE::FIELD`: a short id is a load notice naming
    /// the full spelling, a full id no table holds is a notice too, and
    /// neither installs anything, so the scope inherits. `-direct` is a
    /// different, already-closed catalogue — `NativeEvalId`'s own Rust
    /// spelling, not `SCOPE::FIELD` — and a name it does not hold is dropped
    /// the same way.
    #[test]
    fn a_short_native_id_is_a_load_notice() {
        let pack = evaluate_pack(
            "speclib probe 2.2 {\n\
             command demo {\n\
             \x20   arity 1\n\
             \x20   evaluate -native evaluate\n\
             \x20   evaluate -direct nonexistent\n\
             \x20   semantics -native demo::semantics\n\
             \x20   facts -native facts\n\
             \x20   subcommand go {\n\
             \x20       evaluate -native go::evaluate\n\
             \x20   }\n\
             }\n\
             }",
        );
        let messages: Vec<&str> = pack
            .notices
            .iter()
            .map(|notice| notice.message.as_str())
            .collect();
        for expected in [
            "`evaluate -native evaluate` is not this scope's id; spell it `demo::evaluate`",
            "unknown direct evaluator `nonexistent` dropped",
            "`semantics -native demo::semantics` names nothing this build ships",
            "`facts -native facts` is not this scope's id; spell it `demo::facts`",
            "`evaluate -native go::evaluate` is not this scope's id; spell it `demo::go::evaluate`",
        ] {
            assert!(
                messages.iter().any(|message| message.contains(expected)),
                "{expected}: {messages:#?}"
            );
        }
        let demo = pack.command("demo").unwrap().spec;
        assert!(matches!(demo.semantics, SemanticsDeclaration::Inherited));
        assert!(matches!(
            demo.subcommands[0].semantics,
            SemanticsDeclaration::Inherited
        ));
    }

    /// A `-native ID` that a table *does* hold installs the table's value —
    /// `native_id` itself, proven directly against a synthetic table, since
    /// nothing shipped is reachable this way yet (every real table above is
    /// empty).
    #[test]
    fn a_native_id_a_table_holds_installs_its_value() {
        fn word(text: &str) -> Word {
            Word {
                text: text.to_owned(),
                braced: false,
                line: 1,
            }
        }
        let mut log = Log::default();
        let scope = Scope {
            path: "demo",
            binds_bodies: true,
        };
        let stmt = Stmt {
            words: vec![word("semantics"), word("-native"), word("demo::semantics")],
            line: 1,
        };
        let table: &[(&'static str, u8)] = &[("demo::semantics", 42)];
        assert_eq!(
            native_id(&stmt, "semantics", "-native", &scope, table, &mut log),
            Some(42)
        );
        assert!(log.notices.is_empty(), "{:?}", log.notices);
    }

    /// What cannot be used is reported and dropped, never half-installed:
    /// a form's implementation (bodies bind at command and subcommand
    /// scope), a body whose parameters do not match its inputs, a budget
    /// above the host's, `no_store_writes` beside a `stores` row, and an
    /// option flag on a scope that declares no route.
    #[test]
    fn what_cannot_be_used_is_reported_and_dropped() {
        let pack = evaluate_pack(
            "speclib probe 2.2 {\n\
             command demo {\n\
             \x20   arity 1..\n\
             \x20   refine f {\n\
             \x20       selector {f}\n\
             \x20       evaluate -implementation demo.f -host bounded_tcl { inputs {arg 0 exact}; body {x} { fold $x } }\n\
             \x20   }\n\
             \x20   subcommand pair {\n\
             \x20       evaluate -implementation demo.pair -host bounded_tcl { inputs {arg 0 exact}; body {x y} { fold $x } }\n\
             \x20   }\n\
             \x20   subcommand wide {\n\
             \x20       evaluate -implementation demo.wide -host bounded_tcl { inputs {arg 0 exact}; budget {-commands 900000}; body {x} { fold $x } }\n\
             \x20   }\n\
             \x20   subcommand clash {\n\
             \x20       semantics { effects {no_store_writes}; stores -targets {0} -outcome write }\n\
             \x20   }\n\
             \x20   subcommand flag {\n\
             \x20       option -x -evaluate none\n\
             \x20   }\n\
             \x20   subcommand axis {\n\
             \x20       evaluate none\n\
             \x20       option -y -evaluate none -evaluate-reason release_ambiguous\n\
             \x20   }\n\
             }\n\
             }",
        );
        let messages: Vec<&str> = pack
            .notices
            .iter()
            .map(|notice| notice.message.as_str())
            .collect();
        for expected in [
            "a form's declared implementation is dropped",
            "the body takes 2 parameter(s) for 1 declared input(s)",
            "`budget -commands 900000` is above the host's 100000; the host's stands",
            "cannot coexist with a `stores` row",
            "an option's `-evaluate` flag needs a `semantics` or `evaluate` statement",
            "`-evaluate-reason release_ambiguous` names the availability of `-y`, which declares \
             none; recorded as `-evaluate none`",
        ] {
            assert!(
                messages.iter().any(|message| message.contains(expected)),
                "{expected}: {messages:#?}"
            );
        }
        let demo = pack.command("demo").unwrap().spec;
        let sub = |name: &str| {
            demo.subcommands
                .iter()
                .find(|sub| sub.name == name)
                .expect("the subcommand loads")
                .semantics
        };
        assert!(matches!(sub("pair"), SemanticsDeclaration::Inherited));
        let wide = declared(sub("wide"));
        let DeclaredEvaluation::Implementation(implementation) = wide.evaluation else {
            panic!("the implementation loads with the host's budget");
        };
        assert_eq!(implementation.capability.budget.commands, None);
        assert!(matches!(sub("clash"), SemanticsDeclaration::Inherited));
        assert!(matches!(sub("flag"), SemanticsDeclaration::Inherited));
        assert_eq!(
            declared(sub("axis")).option_declines,
            [("-y", DeclineReason::NoRoute(NoRouteReason::Declared))]
        );
        assert_eq!(
            declared(demo.command_forms[0].semantics).route(),
            EvalRoute::None {
                reason: NoRouteReason::Unauthored
            },
            "the form declines rather than inherit"
        );
    }
}
