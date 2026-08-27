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

//! F6: the tmsh command-syntax version is its **own** axis, with a
//! temporal transition inside one script.
//!
//! Since BIG-IP 11.5.0 tmsh commands are versioned, and a `cli script` may
//! select which command grammar is active. F5's own multi-version example
//! calls `tmsh::modify cli version active 11.5.0`, runs a command, changes
//! the active version to 11.6.0, and runs another **in the same
//! `script::run`** — so the setting is not document-constant, and an
//! environment-level version selected once for the file cannot express it.
//!
//! Three facts have to stay apart:
//!
//! ```text
//! BIG-IP software/build
//!   × embedded Tcl runtime evidence
//!   × selected tmsh command-syntax version
//! ```
//!
//! The first two are [`crate::f5::evidence`]'s business. The third is
//! this module's, and its axis is
//! [`tcl_dialect::model::VersionAxisId::tmsh_syntax`] — a distinct axis, so
//! the version-set algebra makes any attempt to intersect it with a Tcl
//! core release or a package version a typed error rather than a
//! coincidence of dotted numbers (invariant I2).
//!
//! The transition itself reuses the registry's existing shape for state
//! whose identity affects analysis ([`crate::state_transition`]): a
//! **literal** argument moves the state to a known version, and anything
//! non-literal widens it to [`TmshSyntaxState::Unknown`] rather than
//! guessing. What is deliberately *not* decided here is the realm scope —
//! whether the setting is local to the script, the tmsh process, the user
//! session, or the system. F6 says to probe that before assigning a scope,
//! and the probe has not been run
//! (`docs/design/bigip-irule-parser-measurements.md` §12), so
//! [`TmshSyntaxTransition::scope_is_measured`] is `false` and consumers
//! must treat the state as script-local *and* say they are assuming it.

use tcl_dialect::model::VersionAxisId;

use crate::invocation_words::InvocationArguments;
use crate::state_transition::TransitionSubject;

/// The BIG-IP release that introduced versioned tmsh commands and the
/// `cli version active` selector.
pub const TMSH_SYNTAX_VERSIONED_SINCE: &str = "11.5.0";

/// The command whose invocation changes the active tmsh syntax version.
const TRANSITION_COMMAND: &str = "tmsh::modify";

/// The literal argument words that select the transition form
/// (`tmsh::modify cli version active <V>`).
const TRANSITION_PREFIX: [&str; 3] = ["cli", "version", "active"];

/// The active tmsh command-syntax version at a point in a script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TmshSyntaxState {
    /// No selection has been made: the appliance's own default, which is
    /// the installed BIG-IP release's syntax.
    ApplianceDefault,
    /// A constant selection is in force.
    Selected(String),
    /// A dynamic or unsupported selection widened the state. Every
    /// subsequent tmsh command resolves against an unknown grammar, and
    /// version-sensitive analysis must abstain.
    Unknown,
}

impl TmshSyntaxState {
    /// The selected version, when one is statically known.
    #[must_use]
    pub fn version(&self) -> Option<&str> {
        match self {
            Self::Selected(version) => Some(version),
            Self::ApplianceDefault | Self::Unknown => None,
        }
    }

    /// Whether tmsh-command resolution can proceed against a known
    /// grammar.
    #[must_use]
    pub const fn is_resolvable(&self) -> bool {
        !matches!(self, Self::Unknown)
    }

    /// The axis this state's versions live on — never a Tcl axis (F6).
    #[must_use]
    pub fn axis() -> VersionAxisId {
        VersionAxisId::tmsh_syntax()
    }
}

/// What one `tmsh::modify cli version active` invocation does to the
/// state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TmshSyntaxSelection {
    /// A literal version word: subsequent tmsh commands resolve against
    /// this grammar.
    Constant(String),
    /// A computed, expanded, or otherwise opaque word: the state widens.
    Dynamic(TransitionSubject),
}

/// The registry-declared transition for the selector command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmshSyntaxTransition {
    /// What the invocation selected.
    pub selection: TmshSyntaxSelection,
    /// Whether the realm scope of the setting has been established by
    /// measurement. Always `false` today — F6 requires probing whether the
    /// state is script-, process-, session-, or system-scoped before a
    /// scope is assigned, and §12 lists that probe as outstanding.
    pub scope_is_measured: bool,
}

impl TmshSyntaxTransition {
    /// The state in force after this transition.
    ///
    /// It takes no incoming state on purpose: the selector *replaces* the
    /// active version outright, so the state after a selection never
    /// depends on the state before it. A dynamic operand widens to
    /// [`TmshSyntaxState::Unknown`] even when a known version was in force
    /// — the script has just asked for a version the analyser cannot name.
    #[must_use]
    pub fn resulting_state(&self) -> TmshSyntaxState {
        match &self.selection {
            TmshSyntaxSelection::Constant(version) => TmshSyntaxState::Selected(version.clone()),
            TmshSyntaxSelection::Dynamic(_) => TmshSyntaxState::Unknown,
        }
    }
}

/// The tmsh-syntax transition an invocation establishes, or `None` when
/// the invocation is not the selector.
///
/// `command` is the resolved command head and `arguments` are the words
/// after it, exactly as the registry's other transition descriptors see
/// them — so a computed version word arrives as a typed unknown rather
/// than as source spelling.
#[must_use]
pub fn tmsh_syntax_transition_for(
    command: &str,
    arguments: InvocationArguments<'_>,
) -> Option<TmshSyntaxTransition> {
    if command != TRANSITION_COMMAND {
        return None;
    }
    for (index, expected) in TRANSITION_PREFIX.iter().enumerate() {
        if arguments.get(index)?.literal()? != *expected {
            return None;
        }
    }
    let version_index = TRANSITION_PREFIX.len();
    let subject = TransitionSubject::from_argument(arguments, version_index)?;
    let selection = match subject.literal() {
        Some(version) => TmshSyntaxSelection::Constant(version.to_owned()),
        None => TmshSyntaxSelection::Dynamic(subject),
    };
    Some(TmshSyntaxTransition {
        selection,
        scope_is_measured: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::invocation_words::InvocationWord;
    use tcl_dialect::model::{Version, VersionSet};

    const fn words<'a>(values: &'a [InvocationWord<'a>]) -> InvocationArguments<'a> {
        InvocationArguments::structured(values)
    }

    const fn literal(value: &str) -> InvocationWord<'_> {
        InvocationWord::Literal(value)
    }

    /// F5's own multi-version example: two selections in one
    /// `script::run`, and the statements between them resolve against
    /// different grammars. An environment-level version chosen once for
    /// the file cannot express this.
    #[test]
    fn two_selections_in_one_script_are_two_states() {
        let mut state = TmshSyntaxState::ApplianceDefault;
        assert_eq!(state.version(), None);
        assert!(state.is_resolvable());

        for expected in ["11.5.0", "11.6.0"] {
            let args = [
                literal("cli"),
                literal("version"),
                literal("active"),
                literal(expected),
            ];
            let transition = tmsh_syntax_transition_for("tmsh::modify", words(&args))
                .expect("the selector form");
            assert_eq!(
                transition.selection,
                TmshSyntaxSelection::Constant(expected.to_owned())
            );
            assert!(
                !transition.scope_is_measured,
                "the realm scope is unprobed (§12)"
            );
            state = transition.resulting_state();
            assert_eq!(state.version(), Some(expected));
        }
    }

    /// A dynamic argument widens to `Unknown` instead of guessing.
    #[test]
    fn a_dynamic_version_widens_the_state() {
        let args = [
            literal("cli"),
            literal("version"),
            literal("active"),
            InvocationWord::Dynamic,
        ];
        let transition =
            tmsh_syntax_transition_for("tmsh::modify", words(&args)).expect("the selector form");
        assert!(matches!(
            transition.selection,
            TmshSyntaxSelection::Dynamic(_)
        ));
        let state = transition.resulting_state();
        assert_eq!(state, TmshSyntaxState::Unknown);
        assert!(!state.is_resolvable());
        assert_eq!(state.version(), None);
    }

    /// Only the exact `cli version active` form transitions; every other
    /// `tmsh::modify` is an ordinary configuration write.
    #[test]
    fn only_the_selector_form_transitions() {
        let other = [literal("/ltm/pool/app"), literal("members")];
        assert_eq!(
            tmsh_syntax_transition_for("tmsh::modify", words(&other)),
            None
        );
        let partial = [literal("cli"), literal("version")];
        assert_eq!(
            tmsh_syntax_transition_for("tmsh::modify", words(&partial)),
            None
        );
        let wrong_command = [
            literal("cli"),
            literal("version"),
            literal("active"),
            literal("11.6.0"),
        ];
        assert_eq!(
            tmsh_syntax_transition_for("tmsh::create", words(&wrong_command)),
            None
        );
    }

    /// The axis is typed: a tmsh syntax version can never be compared with
    /// a Tcl core release or a BIG-IP build, even though all three are
    /// dotted numbers.
    #[test]
    fn the_axis_is_not_comparable_with_tcl_or_bigip() {
        let versioned = VersionSet::from_requirements(
            TmshSyntaxState::axis(),
            &[&format!("{TMSH_SYNTAX_VERSIONED_SINCE}-")],
        )
        .expect("tmsh requirement");
        assert!(versioned.contains(&Version::parse("11.6.0").expect("version")));
        assert!(!versioned.contains(&Version::parse("11.4.0").expect("version")));

        let bigip = VersionSet::from_requirements(VersionAxisId::big_ip(), &["21.1.0.1-"])
            .expect("bigip requirement");
        assert!(versioned.intersect(&bigip).is_err(), "F6: distinct axes");
        assert!(
            versioned
                .intersect(
                    &VersionSet::from_requirements(
                        VersionAxisId::core(tcl_dialect::model::family::Family::F5Tcl),
                        &["0-"]
                    )
                    .expect("core requirement")
                )
                .is_err()
        );
    }
}
