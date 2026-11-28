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

//! Lifecycle flags and their checks: arity and stamp windows, stamp gates, option-effect families and containment in a parent's window.

use super::super::statement_reader::{Stmt, Word};
use super::super::{Log, leak_slice, leak_str};
use super::next_text;
use std::fmt;
use tcl_registry::arity::ArityWindow;
use tcl_registry::hover::OptionSpec;
use tcl_registry::lifecycle::Lifecycle;
use tcl_registry::option_effect::OptionEffectFamily;
use tcl_registry::stamp_window::StampWindow;

/// The three lifecycle flags every gateable row shares — `-introduced`,
/// `-deprecated`, `-retired`, each taking one version word.
///
/// Returns whether `flag` was one of them, so a row's own flag reader keeps
/// its `other => unknown_flag(…)` arm for everything else. The releases are on
/// the entity's own package axis; `-min-tcl` stays the Tcl-core axis and the
/// two are independent (`ArgValue`'s two axes, `hover.rs`).
pub(in super::super) fn lifecycle_flag(
    lifecycle: &mut Lifecycle,
    flag: &str,
    words: &[Word],
    i: &mut usize,
) -> bool {
    match flag {
        "-introduced" => lifecycle.introduced = Some(leak_str(&next_text(words, i))),
        "-deprecated" => lifecycle.deprecated = Some(leak_str(&next_text(words, i))),
        "-retired" => lifecycle.retired = Some(leak_str(&next_text(words, i))),
        _ => return false,
    }
    true
}

/// The arity windows a body declared, with overlapping ones dropped.
///
/// Two windows covering the same release make the selected signature depend on
/// declaration order, which is not something a pack can have meant. The pack
/// keeps the first — the ordinary degradation — and gets a notice naming the
/// one dropped, so an editor can mark the row.
///
/// `registry_sweep` rejects the same shape outright for shipped specs. A pack
/// only ever gets a notice: it is authored outside this repository and must
/// still load.
pub(in super::super) fn checked_arity_windows(
    windows: Vec<ArityWindow>,
    what: &str,
    line: u32,
    log: &mut Log,
) -> &'static [ArityWindow] {
    let mut kept: Vec<ArityWindow> = Vec::with_capacity(windows.len());
    for window in windows {
        if let Some(clash) = kept.iter().find(|other| other.overlaps(window)) {
            log.say(
                line,
                format!(
                    "{what} arity window {:?} overlaps {:?}, which would make the \
                     signature depend on declaration order; the later one is dropped",
                    window.arity, clash.arity
                ),
            );
            continue;
        }
        kept.push(window);
    }
    leak_slice(kept)
}

/// How a stamp statement is gated: not at all, to a window, or not usably.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum StampGate {
    /// No lifecycle flag: the stamp is the command's plain one, for every
    /// release.
    Plain,
    /// `-introduced V ?-deprecated V? ?-retired V?`: one window of the stamp.
    Window(Lifecycle),
    /// A lifecycle that cannot be selected at any release; the row is dropped.
    Dropped,
}

/// The lifecycle flags a stamp statement may end with, read from word `from`
/// on (`SpecTcl` 2.2).
///
/// A gated row is one *window* of a stamp that is not the same at every
/// release, and several may be declared for one command. An impossibly ordered
/// window is dropped and not widened to every release, unlike an `arity`
/// window, which degrades to the plain arity: a stamp applied at a release its
/// author never meant it for specialises there, where a signature only gates
/// less.
pub(in super::super) fn stamp_gate(
    stmt: &Stmt,
    from: usize,
    what: &str,
    log: &mut Log,
) -> StampGate {
    let words = &stmt.words;
    let mut lifecycle = Lifecycle::UNSPECIFIED;
    let mut gated = false;
    let mut i = from;
    while i < words.len() {
        let rejected_flag_word = words[i].clone();
        let flag = words[i].text.clone();
        if lifecycle_flag(&mut lifecycle, &flag, words, &mut i) {
            log.since(stmt.line, &flag, "2.2");
            gated = true;
        } else {
            log.unknown_flag(stmt.word_text(0), stmt.line, &rejected_flag_word);
        }
        i += 1;
    }
    if !gated {
        return StampGate::Plain;
    }
    let lifecycle = checked_lifecycle(lifecycle, &format!("{what} window"), stmt.line, log);
    if lifecycle == Lifecycle::UNSPECIFIED {
        log.say(
            stmt.line,
            format!(
                "{what} window dropped: a stamp widened to every release would specialise \
                 where its author never meant it to"
            ),
        );
        return StampGate::Dropped;
    }
    StampGate::Window(lifecycle)
}

/// Whether a form's stamp statement is the plain, ungated row a form can carry.
///
/// A form is already a shape picked out by the call's arguments, and it has no
/// window list to put a gated row in. A row that asks for one is dropped with a
/// notice and not read as an ungated stamp, which would apply it at every
/// release.
pub(in super::super) fn form_stamp_is_plain(
    stmt: &Stmt,
    from: usize,
    what: &str,
    log: &mut Log,
) -> bool {
    if stamp_gate(stmt, from, what, log) == StampGate::Plain {
        return true;
    }
    log.say(
        stmt.line,
        format!(
            "{what} on a form takes no lifecycle window; the row is dropped — declare the \
             windows on the command or the subcommand"
        ),
    );
    false
}

/// Put one stamp statement's value where its gate says: the unversioned field
/// for a plain row, the window list for a gated one, nowhere for a dropped one.
pub(in super::super) fn place_stamp<T: Copy>(
    value: Option<T>,
    gate: StampGate,
    unversioned: &mut Option<T>,
    windows: &mut Vec<StampWindow<T>>,
) {
    match gate {
        StampGate::Plain => *unversioned = value,
        StampGate::Window(lifecycle) => {
            if let Some(value) = value {
                windows.push(StampWindow { lifecycle, value });
            }
        }
        StampGate::Dropped => {}
    }
}

/// The stamp windows a body declared, with overlapping ones dropped.
///
/// The twin of [`checked_arity_windows`]: two windows covering one release make
/// the stamp depend on declaration order, which a pack cannot have meant. The
/// pack keeps the first and gets a notice naming the one dropped.
pub(in super::super) fn checked_stamp_windows<T: Copy + fmt::Debug>(
    windows: Vec<StampWindow<T>>,
    what: &str,
    field: &str,
    line: u32,
    log: &mut Log,
) -> &'static [StampWindow<T>] {
    let mut kept: Vec<StampWindow<T>> = Vec::with_capacity(windows.len());
    for window in windows {
        if let Some(clash) = kept.iter().find(|other| other.overlaps(&window)) {
            log.say(
                line,
                format!(
                    "{what} {field} window {:?} overlaps {:?}, which would make the stamp \
                     depend on declaration order; the later one is dropped",
                    window.value, clash.value
                ),
            );
            continue;
        }
        kept.push(window);
    }
    leak_slice(kept)
}

/// Drop an option's `-effect` when its `-family` names no
/// `option_effect_family` this command or subcommand declares — the
/// generic walk resolves a family by name against exactly this table, so
/// an unresolvable name would otherwise silently narrow every axis the
/// family covers rather than reading as absent.
pub(in super::super) fn checked_option_effect_families(
    mut options: Vec<OptionSpec>,
    families: &[OptionEffectFamily],
    what: &str,
    line: u32,
    log: &mut Log,
) -> &'static [OptionSpec] {
    for option in &mut options {
        if let Some(effect) = option.effect
            && !families.iter().any(|family| family.name == effect.family)
        {
            log.say(
                line,
                format!(
                    "{what} option `{}` declares `-effect` naming family `{}`, which no \
                     `option_effect_family` here declares; the effect is dropped",
                    option.name, effect.family
                ),
            );
            option.effect = None;
        }
    }
    leak_slice(options)
}

/// A parsed lifecycle, or nothing when its releases are impossibly ordered.
///
/// An entity whose lifecycle is rejected still loads: the declaration is a
/// degradation like any other, so the pack keeps the option / form / value and
/// loses only the gate it could not have meant.
pub(in super::super) fn checked_lifecycle(
    lifecycle: Lifecycle,
    what: &str,
    line: u32,
    log: &mut Log,
) -> Lifecycle {
    match lifecycle.validate() {
        Ok(()) => lifecycle,
        Err(error) => {
            log.say(line, format!("{what}: {error}; lifecycle dropped"));
            Lifecycle::UNSPECIFIED
        }
    }
}

/// Report a child lifecycle that reaches outside its parent's window.
///
/// Notice-only, and the declaration is kept: the hard gate is the shipped-spec
/// sweep (`tcl-spectcl-it/tests/it_5/registry_sweep.rs`), which asserts the same
/// containment. Only releases the child declares **for itself** are compared —
/// one it leaves open is the inherited one — and the narrowed window must
/// still be a legal ordering, since a child introduced after its parent
/// retired is unreachable.
pub(in super::super) fn check_contained(
    what: &str,
    child: Lifecycle,
    parent: Lifecycle,
    line: u32,
    log: &mut Log,
) {
    if child.is_unspecified() || parent.is_unspecified() {
        return;
    }
    let merged = child.intersect(parent);
    for (release, own, narrowed) in [
        ("introduced", child.introduced, merged.introduced),
        ("deprecated", child.deprecated, merged.deprecated),
        ("retired", child.retired, merged.retired),
    ] {
        if let Some(own) = own
            && Some(own) != narrowed
        {
            log.say(
                line,
                format!(
                    "{what}: {release} {own} reaches outside the enclosing declaration's \
                     window; kept as declared"
                ),
            );
        }
    }
    if merged.validate().is_err() {
        log.say(
            line,
            format!("{what}: narrowed by the enclosing declaration it is never available"),
        );
    }
}
