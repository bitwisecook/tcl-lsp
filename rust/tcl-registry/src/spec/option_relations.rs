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

//! Declared relations between a command's options: the terms, the facts one invocation supplies, the reports and the check counters.

use super::RELATION_STATS;
use crate::relation::{Relation, RelationFactSource, RelationTermKind, TermHolds};

/// One operand of an [`OptionRelation`] — what a term names in an invocation.
///
/// The four shapes are the four things a real library's option table talks
/// about: an option, an option *carrying a particular value*, a positional
/// argument, and a positional argument carrying a particular value. `bibtex`
/// needs the first and the third (`-command` requires `-channel`, and excludes
/// the inline `text` word); `struct::tree walk` needs the second
/// (`-order in` is illegal with `-type bfs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionTerm {
    /// The option is present, whatever value it carries (`-channel`).
    Option(&'static str),
    /// The option is present **and** its first value word is this literal
    /// (`-type bfs`). Canonical option name first, value second.
    OptionValue(&'static str, &'static str),
    /// A positional argument exists at this 0-based index, counted *after*
    /// the leading option run.
    Argument(u8),
    /// The positional argument at this index is this literal.
    ArgumentValue(u8, &'static str),
}

impl RelationTermKind for OptionTerm {
    fn collective_noun() -> &'static str {
        "Options"
    }

    fn describe(self) -> String {
        match self {
            Self::Option(name) => name.to_owned(),
            Self::OptionValue(name, value) => format!("{name} {value}"),
            Self::Argument(index) => format!("argument {}", index + 1),
            Self::ArgumentValue(index, value) => format!("argument {} = {value}", index + 1),
        }
    }

    /// A bare `-name` is the option; a braced `{-name value}` pins its value;
    /// `{arg N}` is the positional at `N`, and `{arg N value}` pins its value.
    fn spelling(self) -> String {
        match self {
            Self::Option(name) => name.to_owned(),
            Self::OptionValue(name, value) => format!("{{{name} {value}}}"),
            Self::Argument(index) => format!("{{arg {index}}}"),
            Self::ArgumentValue(index, value) => format!("{{arg {index} {value}}}"),
        }
    }
}

impl OptionTerm {
    /// The canonical option name this term names, when it names one.
    #[must_use]
    pub const fn option_name(self) -> Option<&'static str> {
        match self {
            Self::Option(name) | Self::OptionValue(name, _) => Some(name),
            Self::Argument(_) | Self::ArgumentValue(_, _) => None,
        }
    }
}

/// A registry-declared relation between an invocation's options and arguments
/// — the option domain's instance of the one relation mechanism (R11).
pub type OptionRelation = Relation<OptionTerm>;

/// One invocation as the relation checker reads it.
///
/// Built once per call site by the analyser from the leading-option walk it
/// already performs, and borrowed by every relation on that command — the
/// scan is not repeated per relation.
#[derive(Debug, Clone, Copy)]
pub struct OptionFacts<'a> {
    /// The canonical name of each declared option the call supplied, in call
    /// order, paired with its first value word when that word is a literal.
    pub options: &'a [(&'static str, Option<&'a str>)],
    /// The positional words after the leading option run, `None` where a word
    /// is not statically known.
    pub positionals: &'a [Option<&'a str>],
    /// Whether the invocation was read to its end with nothing unreadable in
    /// it — no `{*}` expansion, and no substituted word where an option could
    /// have been. **Only** this licenses proving a term *absent*.
    pub complete: bool,
}

/// What a call site says about one option — three cases, not two, because
/// "supplied with a value nobody can read" answers differently from both
/// "absent" and "supplied with this value".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OptionPresence<'a> {
    /// The call did not supply the option here.
    Absent,
    /// Supplied, but its value word is not statically known.
    PresentUnknownValue,
    /// Supplied, carrying this literal value.
    PresentWith(&'a str),
}

impl OptionFacts<'_> {
    /// What the call said about `name`.
    fn option_presence(&self, name: &str) -> OptionPresence<'_> {
        match self.options.iter().find(|(option, _)| *option == name) {
            None => OptionPresence::Absent,
            Some((_, None)) => OptionPresence::PresentUnknownValue,
            Some((_, Some(value))) => OptionPresence::PresentWith(value),
        }
    }

    /// `Yes` when the word is proven to be `wanted`, `No` when proven not to
    /// be, `Unknown` when the word is not statically known — the shared
    /// answer for an option value and a positional value alike.
    fn matches_literal(word: Option<&str>, wanted: &str) -> TermHolds {
        match word {
            Some(actual) if actual == wanted => TermHolds::Yes,
            Some(_) => TermHolds::No,
            None => TermHolds::Unknown,
        }
    }

    /// The verdict for a term the call did not supply: proven absent only
    /// when the invocation was read to its end.
    pub(super) const fn absent(&self) -> TermHolds {
        if self.complete {
            TermHolds::No
        } else {
            TermHolds::Unknown
        }
    }
}

impl RelationFactSource<OptionTerm> for OptionFacts<'_> {
    /// Presence is provable whatever the value is; absence needs `complete`.
    fn holds(&self, term: OptionTerm) -> TermHolds {
        match term {
            OptionTerm::Option(name) => match self.option_presence(name) {
                OptionPresence::Absent => self.absent(),
                OptionPresence::PresentUnknownValue | OptionPresence::PresentWith(_) => {
                    TermHolds::Yes
                }
            },
            OptionTerm::OptionValue(name, wanted) => match self.option_presence(name) {
                OptionPresence::Absent => self.absent(),
                // Present, but its value could be anything at run time —
                // including `wanted`.
                OptionPresence::PresentUnknownValue => TermHolds::Unknown,
                OptionPresence::PresentWith(actual) => Self::matches_literal(Some(actual), wanted),
            },
            OptionTerm::Argument(index) => {
                if usize::from(index) < self.positionals.len() {
                    TermHolds::Yes
                } else {
                    self.absent()
                }
            }
            OptionTerm::ArgumentValue(index, wanted) => {
                match self.positionals.get(usize::from(index)) {
                    Some(word) => Self::matches_literal(*word, wanted),
                    None => self.absent(),
                }
            }
        }
    }
}

impl OptionRelation {
    /// Every option name this relation mentions, subject included — what a
    /// consumer needs to decide whether the relation is worth evaluating at
    /// all.
    #[must_use]
    pub fn option_names(&self) -> Vec<&'static str> {
        self.mentioned_terms()
            .into_iter()
            .filter_map(OptionTerm::option_name)
            .collect()
    }
}

/// Where in an invocation a command's declared options may appear — the fact
/// the E-R14 relation checker needs to find them.
///
/// Not a style preference: core Tcl's C option loops almost all `break` on the
/// first word that is not a declared option (`Tcl_GlobObjCmd`, `source`,
/// `lsort`), so a later option-shaped word there is a *positional*, and
/// reading it as an option would invent a conflict the interpreter never
/// raises. A script-level command like `http::geturl`, whose parser is
/// `foreach {flag value} $args` after taking the URL, is the other shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OptionPlacement {
    /// A leading run: option parsing stops at the first non-option word.
    /// The default, and what every core Tcl command does.
    #[default]
    Leading,
    /// Options are recognised anywhere in the argument list, after and
    /// between positional words, up to an explicit `--`.
    Anywhere,
}

impl OptionPlacement {
    /// Every placement, in declaration order — the loader's and the studio's
    /// enumeration.
    pub const ALL: &'static [(&'static str, Self)] =
        &[("Leading", Self::Leading), ("Anywhere", Self::Anywhere)];

    /// The DSL spelling.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Leading => "Leading",
            Self::Anywhere => "Anywhere",
        }
    }
}

/// Where a `constraints` hook's report points.
///
/// The hook's `invalid SLOT MESSAGE` verb names one of these: an option
/// spelling, a 0-based positional index (`arg 2`), or the command word itself
/// when the report is about the invocation as a whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintSlot {
    /// A canonical option name.
    Option(&'static str),
    /// A 0-based positional index, counted after the leading option run.
    Argument(u8),
    /// The command word — for a report about the whole invocation.
    Command,
}

/// One thing a `constraints` hook reported.
///
/// Deliberately smaller than [`RelationViolation`]: a hook says *where* and
/// *what*, and the analyser owns the diagnostic code and the span. `conflict`
/// selects W147 ("these cannot go together") over the default W152 ("this one
/// needs that one"), which is the only structural choice a hook makes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintReport {
    /// The slot the report points at.
    pub slot: ConstraintSlot,
    /// The message, as the hook wrote it.
    pub message: String,
    /// Report as an exclusion (W147) rather than a requirement (W152).
    pub conflict: bool,
}

/// What the option-relation checker did, as a measurement.
///
/// The claim that "every relation the declarative vocabulary can
/// express is checked natively with no VM entry" is only worth as much as
/// the number behind it, so the checker counts rather than asserting. Three
/// thread-local counters, incremented once per call site: cheap enough to
/// leave on (they are `Cell<u64>` adds on a path that already allocates a
/// `Vec` per violation), and the only way a regression that starts entering
/// the VM shows up as anything but a slowdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RelationCheckStats {
    /// Call sites whose walk found at least one declared option word — the
    /// population the check is *about*.
    pub option_bearing_sites: u64,
    /// Of those, the sites whose command declares at least one relation or a
    /// `constraints` hook, and so were actually judged.
    pub judged_sites: u64,
    /// Relations evaluated natively across those call sites — the work that
    /// never enters a VM.
    pub relations_evaluated: u64,
    /// Call sites that reached a `constraints` hook. **This is the number the
    /// design is about**: it must stay 0 for a corpus of shipped commands.
    pub hook_entries: u64,
}

/// Record one call site the option walk read.
///
/// `judged` is false for a call site whose command declares no relation at
/// all — it is still an option-bearing site, and counting it is what makes
/// the ratio meaningful.
pub fn note_relation_check(
    option_bearing: bool,
    judged: bool,
    relations_evaluated: usize,
    entered_hook: bool,
) {
    RELATION_STATS.with(|stats| {
        let mut current = stats.get();
        current.option_bearing_sites += u64::from(option_bearing);
        current.judged_sites += u64::from(judged);
        current.relations_evaluated += relations_evaluated as u64;
        current.hook_entries += u64::from(entered_hook);
        stats.set(current);
    });
}

/// This thread's option-relation counters.
#[must_use]
pub fn relation_check_stats() -> RelationCheckStats {
    RELATION_STATS.with(std::cell::Cell::get)
}

/// Zero this thread's option-relation counters.
pub fn reset_relation_check_stats() {
    RELATION_STATS.with(|stats| stats.set(RelationCheckStats::default()));
}

/// A `constraints` hook: the E-R14 escape hatch.
///
/// **Called only when a spec declares one and the declarative relations found
/// nothing to report** (principle P-B). A shipped spec's hook is an ordinary
/// Rust function; a pack's is a Tcl body on the sandboxed VM, reached through
/// [`crate::pack_hooks::constraints_fn`] — the consumer cannot tell which.
///
/// Silence (an empty vector) means "no report", which is also what a crashed,
/// quarantined, budget-blown or hostless pack hook answers.
pub type ConstraintsHook = fn(&OptionFacts<'_>) -> Vec<ConstraintReport>;
