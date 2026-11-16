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

//! `clause_grammar` blocks: the clause rows with their shapes, flags and slots, and the hook behaviours a grammar derives.

use super::super::statement_reader::{Stmt, block};
use super::super::value_parsers::{apply_availability, by_name, list_words, next_text};
use super::super::{
    HookDecl, HookOwner, HookSource, Log, available, leak_one, leak_slice, leak_str,
};
use tcl_registry::arg_role::ArgRole;
use tcl_registry::clause_grammar::{
    ClauseGrammarSpec, ClauseRow, ClauseRowShape, ClauseSelection, ClauseSlot, ClauseTiming,
    DefaultClause,
};
use tcl_registry::pack_hooks::HookFamily;
use tcl_registry::spec::CommandSpec;

/// Record the two hook behaviours a `clause_grammar` derives, as derivations:
/// the registry's walk answers both, so neither is installed as a hook.
/// `line` is the `clause_grammar` row's.
pub(in super::super) fn record_clause_grammar_derivations(
    hooks: &mut Vec<HookDecl>,
    owner: &HookOwner,
    line: u32,
) {
    for (field, family) in [
        ("arg_role_resolver", HookFamily::ArgRoleResolver),
        ("clause_shape_check", HookFamily::ClauseShapeCheck),
    ] {
        hooks.push(HookDecl {
            owner: owner.clone(),
            field,
            family,
            source: HookSource::Derived {
                keyword: "clause_grammar".to_owned(),
            },
            line,
        });
    }
}

/// The notice a clause grammar that declares no clause at all earns.
///
/// `STRUCTURALLY_CHECKED_ARITY` is neither implied nor required: it is the
/// opt-in that makes the walk's defect the command's arity diagnostic (`if`'s
/// E004), where `try` and the loops keep an ordinary arity range beside their
/// grammar.
pub(in super::super) fn check_clause_grammar(
    grammar: &ClauseGrammarSpec,
    line: u32,
    log: &mut Log,
) {
    if grammar.head.slots().is_empty() && grammar.rows.is_empty() && grammar.tail.is_none() {
        log.say(line, "a `clause_grammar` declares no clause");
    }
}

/// The normative `arg_role_resolver from-manufacturers` rule.
///
/// Look `args[0]` up in **this spec's own `manufacturer` rows**; if it names
/// one and that row has a `-definition-body-at N`, emit `role N Body`,
/// provided `N` is a valid index into the call's arguments; otherwise emit
/// nothing. Three details are load-bearing and none is negotiable:
///
/// - **`args[0]` only** — the manufacturer keyword is the first argument
///   word, never searched for elsewhere in the call.
/// - **`Body` only** — `-names-instance-at` and `-constructor-args-from` are
///   read by other consumers and contribute no role here.
/// - **Bounds-checked** — `oo::class create` with no body word emits nothing
///   rather than a role pointing past the end.
#[must_use]
pub fn roles_from_manufacturers(spec: &CommandSpec, args: &[&str]) -> Vec<(u8, ArgRole)> {
    let Some(body) = args
        .first()
        .and_then(|word| {
            spec.manufacturer_methods
                .iter()
                .find(|method| method.keyword == *word)
        })
        .and_then(|method| method.definition_body_at)
    else {
        return Vec::new();
    };
    if usize::from(body) < args.len() {
        vec![(body, ArgRole::Body)]
    } else {
        Vec::new()
    }
}

/// `clause_grammar { … } ?-available V?` — the clause-grammar descriptor, a
/// [`ClauseGrammarSpec`] the registry walks (`tcl_registry::clause_grammar`).
///
/// One row per statement:
///
/// - `head {SLOTS} ?FLAGS?` — the positional leading clause;
/// - `repeated KEYWORD {SLOTS} ?FLAGS?` — zero or more keyword clauses;
/// - `once ?KEYWORD? {SLOTS} ?FLAGS?` — one clause, keywordless ones entered
///   in declaration order;
/// - `group N ?FLAGS?` — the keywordless groups `repeated_args[N]` lays out;
/// - `tail ?KEYWORD? {SLOTS} ?FLAGS?` — at most one trailing clause;
/// - `fallthrough_body WORD`, `default_clause ROW|tail ?-final-only?`,
///   `selection first-match|all` — the chain-level rules.
///
/// A slot is a role name (`Expr`, `Body`, `LoopVarList`, `Pattern`, `Value`),
/// `?word?` for a noise word, or `{ROLE optional}` for a slot that may be
/// absent. A keyword spelt `?word?` is optional. The row flags are
/// `-timing selected|always|per-iteration|init|next|protected` (default
/// `selected`), `-pattern completion-code|error-code-prefix` (the handler of
/// the row's `Pattern` slot), `-conditional` (its `LoopVarList` slots bind
/// only when a data condition holds), `-optional-keyword`, and
/// `-available V`. A row this build cannot read is dropped with a notice,
/// never guessed.
pub(in super::super) fn clause_grammar_value(
    stmt: &Stmt,
    log: &mut Log,
) -> Option<&'static ClauseGrammarSpec> {
    let body = stmt.arg(1)?;
    let mut grammar = clause_grammar_block(&block(body), log);
    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        let rejected_flag_word = words[i].clone();
        match words[i].text.as_str() {
            "-available" => {
                log.v20(stmt.line, "-available");
                let text = next_text(words, &mut i);
                let availability = available::from_flag(&text, stmt.line, log);
                apply_availability(
                    &mut grammar.surface,
                    availability,
                    "clause grammar",
                    stmt.line,
                    log,
                );
            }
            _ => log.unknown_flag("clause_grammar", stmt.line, &rejected_flag_word),
        }
        i += 1;
    }
    Some(leak_one(grammar))
}

fn clause_grammar_block(stmts: &[Stmt], log: &mut Log) -> ClauseGrammarSpec {
    let mut grammar = ClauseGrammarSpec {
        head: ClauseRow::EMPTY_HEAD,
        rows: &[],
        tail: None,
        fallthrough_body: None,
        default_clause: None,
        selection: ClauseSelection::FirstMatch,
        surface: None,
    };
    let mut rows: Vec<ClauseRow> = Vec::new();
    for stmt in stmts {
        match stmt.word_text(0) {
            "head" => {
                if let Some(row) = clause_row(stmt, ClauseRowKind::Head, log) {
                    grammar.head = row;
                }
            }
            "repeated" => {
                if let Some(row) = clause_row(stmt, ClauseRowKind::Repeated, log) {
                    rows.push(row);
                }
            }
            "once" => {
                if let Some(row) = clause_row(stmt, ClauseRowKind::Once, log) {
                    rows.push(row);
                }
            }
            "group" => {
                if let Some(row) = clause_row(stmt, ClauseRowKind::Group, log) {
                    rows.push(row);
                }
            }
            "tail" => grammar.tail = clause_row(stmt, ClauseRowKind::Tail, log),
            "fallthrough_body" => grammar.fallthrough_body = Some(leak_str(stmt.word_text(1))),
            "default_clause" => {
                let row = match stmt.word_text(1) {
                    "tail" => Some(None),
                    index => index.parse::<u8>().ok().map(Some),
                };
                match row {
                    Some(row) => {
                        grammar.default_clause = Some(DefaultClause {
                            row,
                            final_only: stmt.words.iter().any(|word| word.text == "-final-only"),
                        });
                    }
                    None => log.say(
                        stmt.line,
                        format!(
                            "`default_clause` names a row index or `tail`, not `{}`; dropped",
                            stmt.word_text(1)
                        ),
                    ),
                }
            }
            "selection" => match ClauseSelection::from_spelling(stmt.word_text(1)) {
                Some(selection) => grammar.selection = selection,
                None => log.say(
                    stmt.line,
                    format!(
                        "unknown clause selection `{}` (first-match, all); `first-match` kept",
                        stmt.word_text(1)
                    ),
                ),
            },
            _ => log.unknown_property(stmt),
        }
    }
    grammar.rows = leak_slice(rows);
    grammar
}

/// Which statement a clause row came from — what its positional words mean.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClauseRowKind {
    Head,
    Repeated,
    Once,
    Group,
    Tail,
}

/// The row flags a clause row may carry; any other word is positional.
const CLAUSE_ROW_FLAGS: &[&str] = &[
    "-timing",
    "-pattern",
    "-conditional",
    "-optional-keyword",
    "-available",
];

/// One `head` / `repeated` / `once` / `group` / `tail` row, or `None` (with a
/// notice) when a word of it cannot be read.
fn clause_row(stmt: &Stmt, kind: ClauseRowKind, log: &mut Log) -> Option<ClauseRow> {
    let words = &stmt.words;
    let mut positional: Vec<&str> = Vec::new();
    let mut i = 1;
    while i < words.len() && !CLAUSE_ROW_FLAGS.contains(&words[i].text.as_str()) {
        positional.push(words[i].text.as_str());
        i += 1;
    }
    let row = clause_row_shape(stmt, kind, &positional, log)?;
    clause_row_flags(row, stmt, i, log)
}

/// The row a statement's positional words spell, before its flags.
fn clause_row_shape(
    stmt: &Stmt,
    kind: ClauseRowKind,
    positional: &[&str],
    log: &mut Log,
) -> Option<ClauseRow> {
    let keyworded = |keyword: &str, slots: &str, log: &mut Log, repeated: bool| {
        let (keyword, required) = clause_keyword(keyword);
        let slots = clause_slots(slots, stmt.line, log);
        let row = if repeated {
            ClauseRow::repeated(keyword, slots, ClauseTiming::Selected)
        } else {
            ClauseRow::once(Some(keyword), slots, ClauseTiming::Selected)
        };
        if required {
            row
        } else {
            row.optional_keyword()
        }
    };
    Some(match (kind, positional) {
        (ClauseRowKind::Group, [layout]) => {
            let Ok(layout) = layout.parse::<u8>() else {
                log.say(
                    stmt.line,
                    format!(
                        "`group` cites a `repeat` layout by index, not `{layout}`; the row is \
                         dropped"
                    ),
                );
                return None;
            };
            ClauseRow::group(layout, ClauseTiming::Selected)
        }
        (ClauseRowKind::Head, [slots]) => {
            ClauseRow::head(clause_slots(slots, stmt.line, log), ClauseTiming::Selected)
        }
        (ClauseRowKind::Repeated, [keyword, slots]) => keyworded(keyword, slots, log, true),
        (ClauseRowKind::Once | ClauseRowKind::Tail, [slots]) => ClauseRow::once(
            None,
            clause_slots(slots, stmt.line, log),
            ClauseTiming::Selected,
        ),
        (ClauseRowKind::Once | ClauseRowKind::Tail, [keyword, slots]) => {
            keyworded(keyword, slots, log, false)
        }
        _ => {
            log.say(
                stmt.line,
                format!(
                    "`{}` row has the wrong number of words; the row is dropped",
                    stmt.word_text(0)
                ),
            );
            return None;
        }
    })
}

/// A row's flags, from word `i` on; `None` (with a notice) when a flag's value
/// cannot be read — a row is dropped rather than read as some other row.
fn clause_row_flags(
    mut row: ClauseRow,
    stmt: &Stmt,
    mut i: usize,
    log: &mut Log,
) -> Option<ClauseRow> {
    let words = &stmt.words;
    let what = stmt.word_text(0);
    while i < words.len() {
        let rejected_flag_word = words[i].clone();
        match words[i].text.as_str() {
            "-timing" => {
                let name = next_text(words, &mut i);
                let Some(timing) = ClauseTiming::from_spelling(&name) else {
                    log.say(
                        stmt.line,
                        format!(
                            "unknown clause timing `{name}` (selected, always, per-iteration, \
                             init, next, protected); the `{what}` row is dropped"
                        ),
                    );
                    return None;
                };
                row.timing = timing;
            }
            "-pattern" => {
                let name = next_text(words, &mut i);
                let Some(handler) = tcl_registry::clause_grammar::handler_from_spelling(&name)
                else {
                    log.say(
                        stmt.line,
                        format!(
                            "unknown handler pattern `{name}` (completion-code, \
                             error-code-prefix); the `{what}` row is dropped"
                        ),
                    );
                    return None;
                };
                row = with_slots(row, stmt.line, log, "-pattern", |slot| {
                    (slot.role == ArgRole::Pattern).then(|| slot.selecting(handler))
                })?;
            }
            "-conditional" => {
                row = with_slots(row, stmt.line, log, "-conditional", |slot| {
                    (slot.role == ArgRole::LoopVarList).then(|| slot.conditional())
                })?;
            }
            "-optional-keyword" => row = row.optional_keyword(),
            "-available" => {
                log.v20(stmt.line, "-available");
                let text = next_text(words, &mut i);
                let availability = available::from_flag(&text, stmt.line, log);
                apply_availability(&mut row.surface, availability, "clause row", stmt.line, log);
            }
            _ => log.unknown_flag(what, stmt.line, &rejected_flag_word),
        }
        i += 1;
    }
    Some(row)
}

/// A clause keyword word: `?else?` is an optional keyword.
fn clause_keyword(word: &str) -> (&'static str, bool) {
    match word
        .strip_prefix('?')
        .and_then(|inner| inner.strip_suffix('?'))
        .filter(|inner| !inner.is_empty())
    {
        Some(inner) => (leak_str(inner), false),
        None => (leak_str(word), true),
    }
}

/// A row with the slots `change` rewrites, or `None` (with a notice) when the
/// flag names a slot the row does not have.
fn with_slots(
    mut row: ClauseRow,
    line: u32,
    log: &mut Log,
    flag: &str,
    change: impl Fn(ClauseSlot) -> Option<ClauseSlot>,
) -> Option<ClauseRow> {
    let mut changed = false;
    let slots: Vec<ClauseSlot> = row
        .slots()
        .iter()
        .map(|slot| {
            change(*slot).map_or(*slot, |new| {
                changed = true;
                new
            })
        })
        .collect();
    if !changed {
        log.say(
            line,
            format!("`{flag}` names a slot the row does not have; the row is dropped"),
        );
        return None;
    }
    row.shape = match row.shape {
        ClauseRowShape::Repeated { .. } => ClauseRowShape::Repeated {
            slots: leak_slice(slots),
        },
        ClauseRowShape::Once { .. } => ClauseRowShape::Once {
            slots: leak_slice(slots),
        },
        group @ ClauseRowShape::Group { .. } => group,
    };
    Some(row)
}

/// A braced slot list: role names, `?word?` noise words, `{ROLE optional}`.
fn clause_slots(text: &str, line: u32, log: &mut Log) -> &'static [ClauseSlot] {
    let slots: Vec<ClauseSlot> = list_words(text)
        .iter()
        .map(|word| {
            if let Some(noise) = word
                .strip_prefix('?')
                .and_then(|inner| inner.strip_suffix('?'))
                .filter(|inner| !inner.is_empty())
            {
                return ClauseSlot::noise(leak_str(noise));
            }
            let parts = list_words(word);
            let (name, flags) = parts
                .split_first()
                .map_or(("", &[][..]), |(name, flags)| (name.as_str(), flags));
            let role = by_name(ArgRole::ALL, name).unwrap_or_else(|| {
                log.say(
                    line,
                    format!("unknown clause slot role `{name}`; read as `Value`"),
                );
                ArgRole::Value
            });
            let mut slot = ClauseSlot::of(role);
            for flag in flags {
                match flag.as_str() {
                    "optional" => slot = slot.optional(),
                    other => log.say(line, format!("unknown clause slot flag `{other}` ignored")),
                }
            }
            slot
        })
        .collect();
    leak_slice(slots)
}
