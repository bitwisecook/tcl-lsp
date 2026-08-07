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

//! Object-handle bindings — which argument of a call names a **variable that
//! ends up holding an object handle**, and which argument says what class the
//! handle is (issue #1185).
//!
//! Two call shapes recur across class systems, and both used to be recognised
//! by matching the command word in the LSP's snit handle scan:
//!
//! ```tcl
//! install axis using ::verticalAxis $win.a   ;# snit component install
//! set axis [::verticalAxis $win.a]           ;# snit bare-word construction
//! ```
//!
//! Describing them as registry *data* is what lets the scan resolve the same
//! layout for every spelling C Tcl resolves to the same command — the bare
//! word, the explicitly global `::set`, and a provable static alias or rename
//! — without the walker naming a command.  Adding another binding shape (a
//! different class system's component installer) is then a
//! [`HandleBindingSpec`] hung off a [`CommandSpec::binds_handle`] or a
//! [`MemberBodyCommand`], never a new arm in a walker.
//!
//! [`CommandSpec::binds_handle`]: crate::spec::CommandSpec::binds_handle
//! [`MemberBodyCommand`]: crate::definer::MemberBodyCommand

/// Which variable a call binds an object handle *to*.
///
/// Almost every installer names it in the call (`install NAME using TYPE …`),
/// but a class system can also bind a variable it provides implicitly: snit's
/// `installhull using TYPE …` installs the widget's **hull** component, whose
/// name appears nowhere in the call (snit(n): "Given this form, `installhull`
/// creates the hull widget"; the hull is reached as `$hull` inside any member
/// body, which is why it is already one of the widget grammar's
/// `implicit_vars`).  That is a different *kind* of binding, not another index,
/// so it is a variant rather than a sentinel value in a `u8`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleName {
    /// The word at this 0-based argument index (after the command name) names
    /// the variable that receives the handle.
    Word(u8),
    /// The call binds a fixed variable the class system provides implicitly in
    /// every member body — snit's `hull`.  Nothing in the call spells it.
    Implicit(&'static str),
}

/// Where the **class** of a bound object handle is written in the call.
///
/// Both variants name a 0-based argument index (after the command name); they
/// differ in whether that word *is* the class name or merely *contains* a
/// construction of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleClassSource {
    /// The word at this index names the class directly — snit's
    /// `install NAME using TYPE ?args…?`, whose `TYPE` word is the type
    /// command itself.
    Word(u8),
    /// The word at this index is a **value** whose command substitution's head
    /// names the class — `set NAME [TYPE inst …]`, snit's bare-word
    /// constructor.  A consumer must parse the substitution out of the word
    /// (and abstain when it is not one) before it has a class name, which is
    /// why this is a distinct variant rather than another index.
    ConstructionValue(u8),
}

impl HandleClassSource {
    /// The argument index this source reads.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Word(i) | Self::ConstructionValue(i) => i as usize,
        }
    }
}

/// A literal keyword word a binding layout requires, at a fixed argument
/// index — snit's `using` in `install NAME using TYPE`.
///
/// Without the gate `install a b` would bind `a` to the class `b`; the keyword
/// is the only thing that distinguishes the component-install form from any
/// other two-word call, so it belongs in the descriptor rather than in a
/// consumer's arity guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandleKeyword {
    /// 0-based argument index (after the command name) the word sits at.
    pub at: u8,
    /// The exact word required there.  Matched literally — these are fixed
    /// syntax words, not abbreviation-matched ensemble subcommands.
    pub word: &'static str,
}

/// How one command binds a variable to an object handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandleBindingSpec {
    /// Which variable receives the handle.
    pub name_from: HandleName,
    /// Where the handle's class is written.
    pub class_from: HandleClassSource,
    /// A literal keyword the layout requires, or `None` when the positional
    /// shape alone identifies it.
    pub keyword: Option<HandleKeyword>,
}

/// One call's resolved handle binding — the two words a consumer needs, sliced
/// out of the argument list by [`HandleBindingSpec::resolve`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundHandle<'a> {
    /// The word naming the variable that receives the handle.
    pub name: &'a str,
    /// The word the class is read from — a bare class name for
    /// [`HandleClassSource::Word`], the whole value word for
    /// [`HandleClassSource::ConstructionValue`] (the consumer parses the
    /// substitution).
    pub class_word: &'a str,
    /// Which of the two shapes `class_word` is, so the consumer knows whether
    /// to read it directly or parse a construction out of it.
    pub class_source: HandleClassSource,
}

impl HandleBindingSpec {
    /// Resolve this layout against one call's argument words (head excluded).
    ///
    /// Returns `None` when the call is too short or the required keyword is
    /// absent — abstention, never a guess: `install $comp using $type` and a
    /// user's own two-word `install` both fall through untouched.
    #[must_use]
    pub fn resolve<'a>(&self, args: &[&'a str]) -> Option<BoundHandle<'a>> {
        if let Some(kw) = self.keyword
            && args.get(kw.at as usize).copied() != Some(kw.word)
        {
            return None;
        }
        let name = match self.name_from {
            HandleName::Word(i) => *args.get(i as usize)?,
            HandleName::Implicit(name) => name,
        };
        let class_word = *args.get(self.class_from.index())?;
        Some(BoundHandle {
            name,
            class_word,
            class_source: self.class_from,
        })
    }
}

/// `set NAME VALUE` — the variable at 0 receives whatever `VALUE` evaluates
/// to, so a `[TYPE inst …]` construction there makes `NAME` an object handle.
///
/// Attached to `set`'s own [`CommandSpec::binds_handle`], which is what makes
/// `::set` and a provable `rename set myset` classify identically to the bare
/// spelling (issue #1185).
///
/// [`CommandSpec::binds_handle`]: crate::spec::CommandSpec::binds_handle
pub const SET_BINDS_HANDLE: HandleBindingSpec = HandleBindingSpec {
    name_from: HandleName::Word(0),
    class_from: HandleClassSource::ConstructionValue(1),
    keyword: None,
};

/// snit's `install NAME using TYPE ?args…?` — the component installer
/// available inside every snit member body.
///
/// VERIFIED against tcllib snit(n): `install` creates the component, stores
/// the resulting command in the component variable `NAME`, and takes the type
/// command as the word after the literal `using`.
pub const SNIT_INSTALL_BINDS_HANDLE: HandleBindingSpec = HandleBindingSpec {
    name_from: HandleName::Word(0),
    class_from: HandleClassSource::Word(2),
    keyword: Some(HandleKeyword {
        at: 1,
        word: "using",
    }),
};

/// snit's `installhull using WIDGETTYPE ?args…?` — the hull installer
/// available inside a `snit::widget` / `snit::widgetadaptor` member body.
///
/// VERIFIED against tcllib snit(n), which documents exactly two forms:
///
/// * `installhull using widgetType args...` — "Given this form, `installhull`
///   creates the hull widget, and initializes any options delegated to the
///   hull from the Tk option database."  The class is the word after `using`.
/// * `installhull name` — "the hull widget has already been created; note that
///   its name must be `$win`."  There is no static class word in that form, so
///   the missing `using` keyword makes [`HandleBindingSpec::resolve`] abstain.
///
/// Unlike `install`, the variable it binds is not written in the call at all:
/// it is the widget's implicit `hull` component, which is why this needs
/// [`HandleName::Implicit`] rather than another `name_at` row (issue #1275).
pub const SNIT_INSTALLHULL_BINDS_HANDLE: HandleBindingSpec = HandleBindingSpec {
    name_from: HandleName::Implicit("hull"),
    class_from: HandleClassSource::Word(1),
    keyword: Some(HandleKeyword {
        at: 0,
        word: "using",
    }),
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_needs_its_using_keyword() {
        let spec = SNIT_INSTALL_BINDS_HANDLE;
        assert_eq!(
            spec.resolve(&["axis", "using", "verticalAxis", "$win.a"]),
            Some(BoundHandle {
                name: "axis",
                class_word: "verticalAxis",
                class_source: HandleClassSource::Word(2),
            })
        );
        // No `using` word — a user's own `install a b c` binds nothing.
        assert_eq!(spec.resolve(&["a", "b", "c"]), None);
        // Too short to carry a type word.
        assert_eq!(spec.resolve(&["axis", "using"]), None);
    }

    #[test]
    fn set_binds_its_value_word() {
        let spec = SET_BINDS_HANDLE;
        assert_eq!(
            spec.resolve(&["axis", "[verticalAxis $win.a]"]),
            Some(BoundHandle {
                name: "axis",
                class_word: "[verticalAxis $win.a]",
                class_source: HandleClassSource::ConstructionValue(1),
            })
        );
        // The one-argument read form (`set x`) binds nothing.
        assert_eq!(spec.resolve(&["axis"]), None);
    }

    /// snit's hull installer binds the implicit `hull` component, so the
    /// bound name comes from the descriptor rather than from any argument.
    #[test]
    fn installhull_binds_the_implicit_hull_component() {
        let spec = SNIT_INSTALLHULL_BINDS_HANDLE;
        assert_eq!(
            spec.resolve(&["using", "ttk::frame", "-borderwidth", "2"]),
            Some(BoundHandle {
                name: "hull",
                class_word: "ttk::frame",
                class_source: HandleClassSource::Word(1),
            })
        );
        // `installhull $win` — the already-created form has no static class
        // word, so the missing keyword makes the layout abstain.
        assert_eq!(spec.resolve(&["$win"]), None);
        // A `using` with nothing after it is too short to carry a type word.
        assert_eq!(spec.resolve(&["using"]), None);
    }

    #[test]
    fn class_source_reports_its_index() {
        assert_eq!(HandleClassSource::Word(2).index(), 2);
        assert_eq!(HandleClassSource::ConstructionValue(1).index(), 1);
    }
}
