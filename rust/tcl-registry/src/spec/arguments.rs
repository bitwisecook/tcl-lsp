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

//! Argument positions: roles, literal values with their lifecycle gates, and the synopsis that names them.

use super::CommandSpec;
use super::subcommand::SubCommand;
use crate::arg_role::ArgRole;
use crate::hover::ArgValue;
use crate::lifecycle::{Lifecycle, LifecycleState};
use tcl_dialect::model::{SurfaceQuery, surface_admits};

/// The trailing run of `?placeholder?` words in a synopsis, `?` stripped.
///
/// A synopsis is a whitespace-separated word list whose optional arguments
/// are wrapped in `?…?` (`catch script ?resultVarName? ?optionsVarName?`).
/// Scanning from the right and stopping at the first word that is not so
/// wrapped yields exactly the words a caller may append to a call that
/// supplies every earlier word — the question a splice-a-trailing-argument
/// quick-fix asks.  A variadic tail (`?arg ...?`, `?arg arg ...?`) is not a
/// fixed slot list, so a placeholder containing whitespace or an ellipsis
/// terminates the run rather than being offered as a name.
pub(super) fn optional_trailing_placeholders(synopsis: &'static str) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = synopsis
        .split_whitespace()
        .rev()
        .map_while(|word| {
            let inner = word.strip_prefix('?')?.strip_suffix('?')?;
            (!inner.is_empty() && inner != "..." && !inner.contains("...")).then_some(inner)
        })
        .collect();
    names.reverse();
    names
}

impl CommandSpec {
    /// The command's primary invocation synopsis — the first non-empty
    /// [`Self::forms`] entry *available at `package_version`*, falling back to
    /// the first non-empty hover synopsis line. `None` when the spec declares
    /// neither. Used for the "usage: …" suffix on arity diagnostics
    /// (E002/E003/E005).
    ///
    /// A [`FormSpec`] carries its own [`Lifecycle`], so a form a later release
    /// added — or withdrew — is skipped when `package_version` excludes it: a
    /// document pinned to the older release is told the usage *it* has, not a
    /// synopsis it cannot call. `None` is permissive (every form is eligible),
    /// matching the rest of the `*_for_version` family; the hover fallback is
    /// never gated, because a hover line carries no lifecycle to gate on.
    ///
    /// [`FormSpec`]: crate::hover::FormSpec
    #[must_use]
    pub fn primary_synopsis(&self, package_version: Option<&str>) -> Option<&'static str> {
        self.forms
            .iter()
            .filter(|form| form.available_for_version(package_version))
            .map(|f| f.synopsis)
            .chain(self.hover.iter().flat_map(|h| h.synopsis.iter().copied()))
            .find(|s| !s.is_empty())
    }

    /// The names of the optional trailing words the *dialect-applicable*
    /// synopses document — the run of `?placeholder?` words at the end of a
    /// [`Self::forms`] entry, with the `?` markers stripped.
    ///
    /// This is how a quick-fix that offers to *append* a documented optional
    /// argument learns both how many words it may append and what to call
    /// them, without the consumer knowing the command.  `catch script
    /// ?resultVarName? ?optionsVarName?` yields
    /// `["resultVarName", "optionsVarName"]`; the narrower Tcl 8.4 / iRules
    /// form `catch script ?varName?` yields `["varName"]`, so a consumer
    /// running under those dialects never offers the options-dictionary word
    /// that release has no argument slot for.
    ///
    /// The widest applicable form wins, since a command may declare several
    /// overlapping forms for one dialect and the widest documents every slot
    /// the narrower ones do.  A form whose optional tail is interrupted by a
    /// required word contributes only its trailing run: the words a caller
    /// can append without also having to supply something in between.
    ///
    /// A form is additionally skipped when its [`Lifecycle`] excludes
    /// `package_version` — appending a word that only exists from a later
    /// release is exactly the fix a pinned document must not be offered.
    /// `None` is permissive (every form is eligible).
    ///
    /// Returns an empty vector when the command declares no forms, none apply
    /// to *dialect* and *`package_version`*, or the applicable ones end in a
    /// required word.
    #[must_use]
    pub fn optional_trailing_arg_names(
        &self,
        dialect: Option<SurfaceQuery<'_>>,
        package_version: Option<&str>,
    ) -> Vec<&'static str> {
        self.forms
            .iter()
            .filter(|form| {
                form.surface
                    .or(self.surface)
                    .is_none_or(|allowed| surface_admits(allowed, dialect.as_ref()))
            })
            .filter(|form| form.available_for_version(package_version))
            .map(|form| optional_trailing_placeholders(form.synopsis))
            .max_by_key(Vec::len)
            .unwrap_or_default()
    }

    /// Return static arg role for a given index, if declared.
    #[must_use]
    pub fn arg_role_at(&self, index: u8) -> Option<ArgRole> {
        self.arg_roles
            .iter()
            .find(|(i, _)| *i == index)
            .map(|(_, r)| *r)
    }

    /// Look up enumerable argument values for the 0-based `index`
    /// *after* the command name.  Returns an empty slice when this
    /// argument has no fixed value set.  Mirrors
    /// [`SubCommand::arg_values_at`].
    #[must_use]
    pub fn arg_values_at(&self, index: u8) -> &'static [ArgValue] {
        self.arg_values
            .iter()
            .find(|(i, _)| *i == index)
            .map_or(&[], |(_, vs)| vs)
    }

    /// Whether an enumerable positional argument value exists at
    /// `package_version` — the command-level twin of
    /// [`SubCommand::arg_value_available_for_version`].
    ///
    /// `index` is 0-based *after the command name*. Two gates apply, and a
    /// value must pass both: the value's own [`ArgValue::lifecycle`], and any
    /// [`Self::versioned_arg_values`] entry naming it. A value with neither is
    /// ungated, and an ungated value is always available.
    #[must_use]
    pub fn arg_value_available_for_version(
        &self,
        index: u8,
        value: &str,
        package_version: Option<&str>,
    ) -> bool {
        self.arg_values_at(index)
            .iter()
            .find(|v| v.value == value)
            .is_none_or(|v| v.available_for_version(package_version))
            && self
                .versioned_arg_values
                .iter()
                .find(|gate| gate.index == index && gate.value == value)
                .is_none_or(|gate| gate.available_for_version(package_version))
    }

    /// The enumerable values for the 0-based `index` *after the command name*
    /// that exist at `package_version` — [`Self::arg_values_at`] with both
    /// version gates applied, so completion offers exactly what the target
    /// release accepts.
    #[must_use]
    pub fn available_arg_values_at(
        &self,
        index: u8,
        package_version: Option<&str>,
    ) -> Vec<&'static ArgValue> {
        self.arg_values_at(index)
            .iter()
            .filter(|value| {
                self.arg_value_available_for_version(index, value.value, package_version)
            })
            .collect()
    }
}

/// A lifecycle gate on one literal positional argument value of a
/// subcommand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionedArgValue {
    /// Argument index after the subcommand word.
    pub index: u8,
    /// Literal value whose availability is gated.
    pub value: &'static str,
    /// Introduction / deprecation / retirement releases of this literal value
    /// on the owning package's version axis.
    pub lifecycle: Lifecycle,
}

impl VersionedArgValue {
    /// Whether this value exists at `package_version`.
    #[must_use]
    pub fn available_for_version(&self, package_version: Option<&str>) -> bool {
        self.lifecycle.available_at(package_version)
    }

    /// This value's lifecycle state at `package_version`.
    #[must_use]
    pub fn lifecycle_state(&self, package_version: Option<&str>) -> LifecycleState {
        self.lifecycle.state_at(package_version)
    }
}

impl SubCommand {
    /// The subcommand's primary invocation synopsis — its own
    /// [`Self::synopsis`] when non-empty, falling back to the first
    /// non-empty hover synopsis line. `None` when neither is declared.
    /// The subcommand counterpart of [`CommandSpec::primary_synopsis`].
    #[must_use]
    pub fn primary_synopsis(&self) -> Option<&'static str> {
        std::iter::once(self.synopsis)
            .chain(self.hover.iter().flat_map(|h| h.synopsis.iter().copied()))
            .find(|s| !s.is_empty())
    }

    /// Whether an enumerable positional argument value exists at
    /// `package_version`.
    ///
    /// Both gates apply: the value's own [`ArgValue::lifecycle`] and any
    /// [`Self::versioned_arg_values`] entry naming it.
    #[must_use]
    pub fn arg_value_available_for_version(
        &self,
        index: u8,
        value: &str,
        package_version: Option<&str>,
    ) -> bool {
        self.arg_values_at(index)
            .iter()
            .find(|v| v.value == value)
            .is_none_or(|v| v.available_for_version(package_version))
            && self
                .versioned_arg_values
                .iter()
                .find(|gate| gate.index == index && gate.value == value)
                .is_none_or(|gate| gate.available_for_version(package_version))
    }

    /// The enumerable values for the 0-based `index` *after the subcommand
    /// word* that exist at `package_version` — [`Self::arg_values_at`] with
    /// both version gates applied.
    #[must_use]
    pub fn available_arg_values_at(
        &self,
        index: u8,
        package_version: Option<&str>,
    ) -> Vec<&'static ArgValue> {
        self.arg_values_at(index)
            .iter()
            .filter(|value| {
                self.arg_value_available_for_version(index, value.value, package_version)
            })
            .collect()
    }

    /// Look up a static arg role by index.
    #[must_use]
    pub fn arg_role_at(&self, index: u8) -> Option<ArgRole> {
        self.arg_roles
            .iter()
            .find(|(i, _)| *i == index)
            .map(|(_, r)| *r)
    }

    /// Look up enumerable argument values for the 0-based
    /// `index` *after* the subcommand word.  Returns an empty
    /// slice when this argument has no fixed value set.
    #[must_use]
    pub fn arg_values_at(&self, index: u8) -> &'static [ArgValue] {
        self.arg_values
            .iter()
            .find(|(i, _)| *i == index)
            .map_or(&[], |(_, vs)| vs)
    }
}
