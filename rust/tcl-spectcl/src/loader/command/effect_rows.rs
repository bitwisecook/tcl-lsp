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

//! The `effects`, `side_effect`, option-relation and `frame_effect` rows, and the knowledge they state together.

use super::super::statement_reader::Stmt;
use super::super::value_parsers::{
    CONNECTION_SIDES, FRAME_LAYOUTS, FRAME_LEVEL_WORDS, SIDE_EFFECT_TARGETS, apply_availability,
    checked_lifecycle, enum_by_name, lifecycle_flag, list_words, next_text, parse_dialects,
};
use super::super::{Log, available, leak_slice, leak_str};
use tcl_dialect::model::{Family, SpecSurface, SpecWindow};
use tcl_registry::frame_effect::FrameEffectSpec;
use tcl_registry::knowledge::Knowledge;
use tcl_registry::side_effects::SideEffect;

/// How completely a scope's `side_effect` rows describe it: the
/// `effects -partial` / `effects -complete {FROM UNTIL}` row (issue #1712).
/// Omitted, a scope's rows are a lower bound, or nothing at all when it has
/// none.
#[derive(Clone, Copy)]
pub(in super::super) enum StatedKnowledge {
    Partial,
    Complete(&'static [SpecSurface]),
}

/// Read an `effects` row.
///
/// `-complete` takes one window per flag: `{FROM UNTIL}` on a family's
/// release ladder, both releases inclusive, or `{package NAME ?FROM UNTIL?}`
/// on a package's own axis. Repeated `-complete` flags add windows.
pub(in super::super) fn effects_row(stmt: &Stmt, log: &mut Log) -> Option<StatedKnowledge> {
    let words = &stmt.words;
    let mut stated: Option<StatedKnowledge> = None;
    let mut surfaces: Vec<SpecSurface> = Vec::new();
    let mut i = 1;
    while i < words.len() {
        match words[i].text.as_str() {
            "-partial" => stated = Some(StatedKnowledge::Partial),
            "-complete" => {
                let text = next_text(words, &mut i);
                surfaces.push(complete_window("effects", &text, stmt.line, log)?);
                stated = Some(StatedKnowledge::Complete(&[]));
            }
            _ => log.unknown_flag("effects", stmt.line, &words[i]),
        }
        i += 1;
    }
    match stated {
        Some(StatedKnowledge::Complete(_)) => Some(StatedKnowledge::Complete(leak_slice(surfaces))),
        Some(StatedKnowledge::Partial) => Some(StatedKnowledge::Partial),
        None => {
            log.say(
                stmt.line,
                "`effects` takes `-partial` or `-complete {FROM UNTIL}`",
            );
            None
        }
    }
}

fn complete_window(row: &str, text: &str, line: u32, log: &mut Log) -> Option<SpecSurface> {
    let window = |from: &str, until: &str| -> Option<(SpecWindow, Option<Family>)> {
        let first = from.parse::<tcl_dialect::model::Release>().ok();
        let last = until.parse::<tcl_dialect::model::Release>().ok();
        match (first, last) {
            (Some(first), Some(last)) if first.family() == last.family() && first <= last => {
                let family = first.family();
                let end = family
                    .next_release(last)
                    .map(tcl_dialect::model::Release::as_str);
                Some(((first.as_str(), end), Some(family)))
            }
            _ => None,
        }
    };
    let parts = list_words(text);
    let parts: Vec<&str> = parts.iter().map(String::as_str).collect();
    let surface = match parts.as_slice() {
        ["package", name] => Some(SpecSurface::package(leak_str(name))),
        [from, until] => window(from, until).and_then(|(window, family)| {
            Some(SpecSurface::core_in(family?, leak_slice(vec![window])))
        }),
        ["package", name, from, until] => Some(SpecSurface::package_in(
            leak_str(name),
            leak_slice(vec![(leak_str(from), Some(leak_str(until)))]),
        )),
        _ => None,
    };
    if surface.is_none() {
        log.say(
            line,
            format!(
                "`{row} -complete` takes `{{FROM UNTIL}}` on one release ladder or \
                 `{{package NAME ?FROM UNTIL?}}`, not `{{{text}}}`"
            ),
        );
    }
    surface
}

/// The knowledge a scope's effect rows and its `effects` row state together.
pub(in super::super) fn effects_knowledge(
    rows: Vec<SideEffect>,
    stated: Option<StatedKnowledge>,
) -> Knowledge<&'static [SideEffect]> {
    let rows = leak_slice(rows);
    match stated {
        None => Knowledge::declared(rows),
        Some(StatedKnowledge::Partial) => Knowledge::Partial(rows),
        Some(StatedKnowledge::Complete(surfaces)) => Knowledge::Complete(rows, surfaces),
    }
}

/// A single-valued domain row's completeness flags, and the row without
/// them (issue #1712): `ROW VALUE ?-partial? ?-complete WINDOW?…`, as
/// `world_effects none -complete {8.4 9.1}`. The flags follow the value so
/// the value stays the row's first word, where a block body is painted. The
/// value alone states a lower bound; each `-complete` window closes the
/// domain there; no row states nothing. An unreadable window leaves the
/// value a lower bound.
pub(in super::super) fn split_knowledge_flags(
    stmt: &Stmt,
    log: &mut Log,
) -> (Option<StatedKnowledge>, Stmt) {
    let row = stmt.word_text(0).to_owned();
    let mut stated = None;
    let mut surfaces = Vec::new();
    let mut readable = true;
    let mut i = 2;
    while let Some(word) = stmt.words.get(i) {
        match (word.braced, word.text.as_str()) {
            (false, "-partial") => stated = Some(StatedKnowledge::Partial),
            (false, "-complete") => {
                let text = next_text(&stmt.words, &mut i);
                match complete_window(&row, &text, stmt.line, log) {
                    Some(surface) => surfaces.push(surface),
                    None => readable = false,
                }
                stated = Some(StatedKnowledge::Complete(&[]));
            }
            _ => log.unknown_flag(&row, stmt.line, word),
        }
        i += 1;
    }
    let mut value = stmt.clone();
    value.words.truncate(2);
    let stated = match stated {
        Some(StatedKnowledge::Complete(_)) if readable => {
            Some(StatedKnowledge::Complete(leak_slice(surfaces)))
        }
        Some(StatedKnowledge::Complete(_)) => Some(StatedKnowledge::Partial),
        other => other,
    };
    (stated, value)
}

/// The knowledge a single-valued domain's row states: nothing without a
/// value, otherwise a lower bound unless its flags close it.
pub(in super::super) fn stated_knowledge<T: Copy>(
    value: Option<T>,
    stated: Option<StatedKnowledge>,
) -> Knowledge<T> {
    match (value, stated) {
        (None, _) => Knowledge::Unknown,
        (Some(value), None | Some(StatedKnowledge::Partial)) => Knowledge::Partial(value),
        (Some(value), Some(StatedKnowledge::Complete(surfaces))) => {
            Knowledge::Complete(value, surfaces)
        }
    }
}

pub(in super::super) fn side_effect_row(stmt: &Stmt, log: &mut Log) -> Option<SideEffect> {
    let target = enum_by_name(
        SIDE_EFFECT_TARGETS,
        stmt.word_text(1),
        "side-effect target",
        stmt.line,
        log,
    )?;
    let mut effect = SideEffect {
        target,
        ..SideEffect::DEFAULT
    };
    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        let rejected_flag_word = words[i].clone();
        match words[i].text.as_str() {
            "-reads" => effect.reads = true,
            "-writes" => effect.writes = true,
            "-side" => {
                let name = next_text(words, &mut i);
                if let Some(side) =
                    enum_by_name(CONNECTION_SIDES, &name, "connection side", stmt.line, log)
                {
                    effect.connection_side = side;
                }
            }
            "-dialects" => {
                let text = next_text(words, &mut i);
                effect.surface = parse_dialects(&text, stmt.line, log);
            }
            "-available" => {
                log.v20(stmt.line, "-available");
                let text = next_text(words, &mut i);
                let availability = available::from_flag(&text, stmt.line, log);
                apply_availability(
                    &mut effect.surface,
                    availability,
                    "side effect",
                    stmt.line,
                    log,
                );
            }
            other => {
                if lifecycle_flag(&mut effect.lifecycle, other, words, &mut i) {
                    log.v11(stmt.line, other);
                } else {
                    log.unknown_flag("side_effect", stmt.line, &rejected_flag_word);
                }
            }
        }
        i += 1;
    }
    effect.lifecycle = checked_lifecycle(
        effect.lifecycle,
        &format!("side_effect `{}`", stmt.word_text(1)),
        stmt.line,
        log,
    );
    Some(effect)
}

/// One [`tcl_registry::OptionTerm`] as an author spells it.
///
/// A bare `-name` is the option; `{-name value}` pins its value; `{arg N}` is
/// the positional at `N` and `{arg N value}` pins its value. The four shapes
/// are the four things a real library's option table talks about, and each is
/// an ordinary Tcl list word — no mini-language (principle P-E).
fn relation_term(
    spelling: &str,
    statement: &str,
    line: u32,
    log: &mut Log,
) -> Option<tcl_registry::OptionTerm> {
    let words = list_words(spelling);
    match words.as_slice() {
        [name] if name.starts_with('-') => Some(tcl_registry::OptionTerm::Option(leak_str(name))),
        [name, value] if name.starts_with('-') => Some(tcl_registry::OptionTerm::OptionValue(
            leak_str(name),
            leak_str(value),
        )),
        [keyword, index] if keyword == "arg" => {
            index.parse().ok().map(tcl_registry::OptionTerm::Argument)
        }
        [keyword, index, value] if keyword == "arg" => index
            .parse()
            .ok()
            .map(|index| tcl_registry::OptionTerm::ArgumentValue(index, leak_str(value))),
        _ => None,
    }
    .or_else(|| {
        log.say(
            line,
            format!("`{statement}`: unreadable relation term `{spelling}` dropped"),
        );
        None
    })
}

/// Every term in a braced list, dropping the unreadable ones with a notice.
fn relation_terms(
    text: &str,
    statement: &str,
    line: u32,
    log: &mut Log,
) -> &'static [tcl_registry::OptionTerm] {
    let terms: Vec<tcl_registry::OptionTerm> = list_words(text)
        .iter()
        .filter_map(|spelling| relation_term(spelling, statement, line, log))
        .collect();
    leak_slice(terms)
}

/// The shared row parser behind all four option-relation statements.
///
/// `option_conflict {-a -b}` keeps its 1.x spelling exactly — the terms are
/// word 1 and there is no subject. The three E-R14 statements take the subject
/// as word 1 and the terms as word 2, with an empty subject (`{}`) making the
/// relation unconditional. Every flag (`-dialects`, `-available`, the
/// lifecycle trio) is shared, so an author learns one row and four verbs.
pub(in super::super) fn option_relation_row(
    stmt: &Stmt,
    kind: tcl_registry::RelationKind,
    log: &mut Log,
) -> tcl_registry::spec::OptionRelation {
    let statement = kind.statement_word();
    let (subject, terms_word, first_flag) = if kind == tcl_registry::RelationKind::MutuallyExclusive
    {
        (None, stmt.word_text(1).to_owned(), 2)
    } else {
        let spelling = stmt.word_text(1).to_owned();
        let subject = if spelling.trim().is_empty() {
            None
        } else {
            relation_term(&spelling, statement, stmt.line, log)
        };
        (subject, stmt.word_text(2).to_owned(), 3)
    };
    let mut relation = tcl_registry::spec::OptionRelation {
        kind,
        subject,
        terms: relation_terms(&terms_word, statement, stmt.line, log),
        ..tcl_registry::spec::OptionRelation::DEFAULT
    };
    let words = &stmt.words;
    let mut i = first_flag;
    while i < words.len() {
        let rejected_flag_word = words[i].clone();
        match words[i].text.as_str() {
            "-dialects" => {
                let text = next_text(words, &mut i);
                relation.surface = parse_dialects(&text, stmt.line, log);
            }
            "-available" => {
                log.v20(stmt.line, "-available");
                let text = next_text(words, &mut i);
                let availability = available::from_flag(&text, stmt.line, log);
                apply_availability(
                    &mut relation.surface,
                    availability,
                    "option relation",
                    stmt.line,
                    log,
                );
            }
            // The library's own error text, quoted instead of generated.
            "-message" => {
                let text = next_text(words, &mut i);
                relation.message = Some(leak_str(&text));
            }
            other => {
                if lifecycle_flag(&mut relation.lifecycle, other, words, &mut i) {
                    log.v11(stmt.line, other);
                } else {
                    log.unknown_flag(statement, stmt.line, &rejected_flag_word);
                }
            }
        }
        i += 1;
    }
    relation.lifecycle =
        checked_lifecycle(relation.lifecycle, &relation.describe(), stmt.line, log);
    relation
}

pub(super) fn frame_effect_row(stmt: &Stmt, log: &mut Log) -> Option<FrameEffectSpec> {
    let mut level_word = None;
    let mut layout = None;
    let words = &stmt.words;
    let mut i = 1;
    while i < words.len() {
        let rejected_flag_word = words[i].clone();
        match words[i].text.as_str() {
            "-level-word" => {
                let name = next_text(words, &mut i);
                level_word =
                    enum_by_name(FRAME_LEVEL_WORDS, &name, "frame level word", stmt.line, log);
            }
            "-layout" => {
                let name = next_text(words, &mut i);
                layout = enum_by_name(
                    FRAME_LAYOUTS,
                    &name,
                    "frame argument layout",
                    stmt.line,
                    log,
                );
            }
            _ => log.unknown_flag("frame_effect", stmt.line, &rejected_flag_word),
        }
        i += 1;
    }
    let (Some(level_word), Some(layout)) = (level_word, layout) else {
        log.say(stmt.line, "`frame_effect` needs -level-word and -layout");
        return None;
    };
    Some(FrameEffectSpec { level_word, layout })
}
